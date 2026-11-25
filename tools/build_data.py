#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
从 ECDICT 的 SQLite 数据库 (stardict.db) 生成 Rust 静态词表数据文件。

用法:
    python build_data.py [stardict.db路径] [输出.rs路径] [词数]

默认:
    输入: <本脚本同级>/../../ecdict-data/stardict.db
    输出: <项目>/src/word_data.rs
    词数: 42231

数据来源: ECDICT (https://github.com/skywind3000/ECDICT) — MIT License
"""
import os
import re
import sqlite3
import sys

# 允许单词/带连字符/带撇号的纯字母词
ALPHA = re.compile(r"^[A-Za-z][A-Za-z'\-]*$")


def rust_str(s: str) -> str:
    """将 Python 字符串转为合法的 Rust 字符串字面量（含双引号）。"""
    out = []
    for ch in s:
        o = ord(ch)
        if ch == "\\":
            out.append("\\\\")
        elif ch == '"':
            out.append('\\"')
        elif ch == "\n":
            out.append("\\n")
        elif ch == "\r":
            out.append("\\r")
        elif ch == "\t":
            out.append("\\t")
        elif o < 0x20 or o == 0x7F:
            out.append("\\u{%x}" % o)
        else:
            out.append(ch)
    return '"' + "".join(out) + '"'


def normalize_phonetic(p: str) -> str:
    """规范化 ECDICT 音标：首引号 -> 重音符号；西里尔 schwa -> IPA schwa。"""
    if not p:
        return ""
    p = p.replace("\u04d9", "\u0259")  # ә -> ə
    p = p.replace("\u04d8", "\u018f")  # Ә -> Ə (大写)
    if p.startswith("'"):
        p = "\u02c8" + p[1:]  # ' -> ˈ
    return p


def main():
    here = os.path.dirname(os.path.abspath(__file__))
    root = os.path.dirname(here)

    db_path = sys.argv[1] if len(sys.argv) > 1 else os.path.join(
        root, "..", "ecdict-data", "stardict.db")
    out_path = sys.argv[2] if len(sys.argv) > 2 else os.path.join(
        root, "src", "word_data.rs")
    target = int(sys.argv[3]) if len(sys.argv) > 3 else 42231

    db_path = os.path.abspath(db_path)
    out_path = os.path.abspath(out_path)

    if not os.path.exists(db_path):
        print("找不到数据库:", db_path, file=sys.stderr)
        sys.exit(1)

    conn = sqlite3.connect(db_path)
    cur = conn.cursor()

    # 按词频 frq 升序（越小越常用）取所有有词频的条目
    rows = cur.execute(
        "SELECT word, phonetic, translation, exchange "
        "FROM stardict WHERE frq > 0 ORDER BY frq ASC"
    ).fetchall()

    records = []
    seen = set()
    skipped_non_alpha = 0
    skipped_no_trans = 0
    skipped_dup = 0
    for word, phonetic, translation, exchange in rows:
        if not translation:
            skipped_no_trans += 1
            continue
        if not ALPHA.match(word):
            skipped_non_alpha += 1
            continue
        key = word.lower()
        if key in seen:
            skipped_dup += 1
            continue
        seen.add(key)
        records.append((
            word,
            normalize_phonetic(phonetic or ""),
            translation,
            exchange or "",
        ))
        if len(records) >= target:
            break

    os.makedirs(os.path.dirname(out_path), exist_ok=True)
    with open(out_path, "w", encoding="utf-8") as f:
        f.write("// 本文件由 tools/build_data.py 自动生成，请勿手动编辑。\n")
        f.write("// 数据来源: ECDICT (https://github.com/skywind3000/ECDICT) — MIT License\n")
        f.write("// 字段顺序: (单词, 音标, 中文释义(按词性), 词态变化)\n\n")
        f.write("pub static WORDS: &[(&str, &str, &str, &str)] = &[\n")
        for w, p, t, x in records:
            f.write("    (%s, %s, %s, %s),\n"
                    % (rust_str(w), rust_str(p), rust_str(t), rust_str(x)))
        f.write("];\n")

    conn.close()

    print("总词条(有词频):", len(rows))
    print("  - 跳过无中文释义:", skipped_no_trans)
    print("  - 跳过非字母词:", skipped_non_alpha)
    print("  - 跳过大小写重复:", skipped_dup)
    print("生成词条数:", len(records))
    print("输出文件:", out_path)
    if records:
        print("最常用:", records[0][0], "| 第%d个:" % len(records), records[-1][0])


if __name__ == "__main__":
    main()
