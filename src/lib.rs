//! ecdict — 离线英汉词典核心库（CLI 与 GUI 共用）。
//! 词库数据由 tools/build_data.py 生成，静态嵌入 word_data.rs。

use std::collections::HashMap;

mod word_data;

/// 词库索引：支持精确/大小写不敏感查询与拼写建议。
pub struct Index {
    exact: HashMap<&'static str, usize>,
    lower: HashMap<String, usize>,
    sorted: Vec<&'static str>,
}

impl Index {
    pub fn new() -> Self {
        let n = word_data::WORDS.len();
        let mut exact = HashMap::with_capacity(n);
        let mut lower = HashMap::with_capacity(n);
        let mut sorted = Vec::with_capacity(n);
        for (i, e) in word_data::WORDS.iter().enumerate() {
            let w = e.0;
            exact.entry(w).or_insert(i);
            lower.entry(w.to_lowercase()).or_insert(i);
            sorted.push(w);
        }
        sorted.sort_unstable();
        Index { exact, lower, sorted }
    }

    /// 查询单词，返回词条下标；找不到返回 None。
    pub fn lookup(&self, q: &str) -> Option<usize> {
        if let Some(&i) = self.exact.get(q) {
            return Some(i);
        }
        let lq = q.to_lowercase();
        if let Some(&i) = self.lower.get(&lq) {
            return Some(i);
        }
        None
    }

    pub fn word(&self, i: usize) -> &'static str {
        word_data::WORDS[i].0
    }
    pub fn phonetic(&self, i: usize) -> &'static str {
        word_data::WORDS[i].1
    }
    pub fn translation(&self, i: usize) -> &'static str {
        word_data::WORDS[i].2
    }
    pub fn exchange(&self, i: usize) -> &'static str {
        word_data::WORDS[i].3
    }
    pub fn len(&self) -> usize {
        word_data::WORDS.len()
    }

    /// 拼写建议：前缀匹配优先，其次编辑距离 <= 2，最多 5 个。
    pub fn suggest(&self, q: &str) -> Vec<&'static str> {
        let lq = q.to_lowercase();
        let mut best: Vec<(usize, &'static str)> = Vec::new();
        for &w in &self.sorted {
            let wl = w.to_lowercase();
            if wl.starts_with(&lq) {
                best.push((0, w));
            } else {
                let d = levenshtein(&lq, &wl);
                if d <= 2 {
                    best.push((d + 1, w));
                }
            }
            if best.len() > 200 {
                break;
            }
        }
        best.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.cmp(b.1)));
        best.truncate(5);
        best.into_iter().map(|(_, w)| w).collect()
    }
}

impl Default for Index {
    fn default() -> Self {
        Self::new()
    }
}

// ---------------------------------------------------------------------------
// 翻译解析：按词性分组
// ---------------------------------------------------------------------------

pub struct TransGroup {
    pub pos: String,   // 规范化词性缩写，如 "n." / "adj." / "[医]" / ""
    pub label: String, // 仅空词性时用作分组标题（"其他"）
    pub meanings: Vec<String>,
}

/// 规范化词性缩写：ECDICT 的 "a."/"ad." 统一为标准 "adj."/"adv."。
pub fn normalize_pos(pos: &str) -> String {
    match pos {
        "a." | "adj." => "adj.".to_string(),
        "ad." | "adv." => "adv.".to_string(),
        _ => pos.to_string(),
    }
}

fn parse_line(line: &str) -> (String, String) {
    if let Some(rest) = line.strip_prefix('[') {
        if let Some(pos) = rest.find(']') {
            let tag = &rest[..pos];
            let meaning = rest[pos + 1..].trim_start_matches(' ').trim();
            if !meaning.is_empty() {
                return (format!("[{}]", tag), meaning.to_string());
            }
        }
    }
    if let Some(idx) = line.find(' ') {
        let head = &line[..idx];
        if head.ends_with('.') && head.chars().all(|c| c.is_ascii_lowercase() || c == '.') {
            let meaning = line[idx + 1..].trim_start().trim();
            return (head.to_string(), meaning.to_string());
        }
    }
    ("".to_string(), line.to_string())
}

pub fn parse_translation(t: &str) -> Vec<TransGroup> {
    let mut groups: Vec<TransGroup> = Vec::new();
    for raw in t.split('\n') {
        let line = raw.trim_end_matches('\r').trim();
        if line.is_empty() {
            continue;
        }
        let (pos, meaning) = parse_line(line);
        if meaning.is_empty() {
            continue;
        }
        let pos = normalize_pos(&pos);
        match groups.iter_mut().find(|g| g.pos == pos) {
            Some(g) => {
                if !g.meanings.contains(&meaning) {
                    g.meanings.push(meaning);
                }
            }
            None => groups.push(TransGroup {
                label: if pos.is_empty() { "其他".to_string() } else { String::new() },
                pos,
                meanings: vec![meaning],
            }),
        }
    }
    groups
}

// ---------------------------------------------------------------------------
// 词态变化解析
// ---------------------------------------------------------------------------

pub struct Form {
    pub code: char,
    pub label: String,
    pub value: String,
}

pub fn exchange_label(code: char) -> String {
    let s = match code {
        's' => "复数",
        'p' => "过去式",
        'd' => "过去分词",
        'i' => "现在分词",
        '3' => "第三人称单数",
        'r' => "比较级",
        't' => "最高级",
        _ => return code.to_string(),
    };
    s.to_string()
}

pub fn parse_exchange(x: &str) -> Vec<Form> {
    const ORDER: &str = "spdi3rt";
    let mut map: HashMap<char, String> = HashMap::new();
    for part in x.split('/') {
        if part.is_empty() {
            continue;
        }
        if let Some(idx) = part.find(':') {
            let code = part[..idx].chars().next().unwrap_or('?');
            let val = part[idx + 1..].trim();
            if !val.is_empty() {
                map.insert(code, val.to_string());
            }
        }
    }
    let mut out = Vec::new();
    for c in ORDER.chars() {
        if let Some(v) = map.remove(&c) {
            out.push(Form {
                code: c,
                label: exchange_label(c),
                value: v,
            });
        }
    }
    out
}

// ---------------------------------------------------------------------------
// 工具函数
// ---------------------------------------------------------------------------

fn levenshtein(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0usize; b.len() + 1];
    for i in 1..=a.len() {
        cur[0] = i;
        for j in 1..=b.len() {
            let cost = if a[i - 1] == b[j - 1] { 0 } else { 1 };
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

pub fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 8);
    for ch in s.chars() {
        match ch {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}
