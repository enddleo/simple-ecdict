//! 剪贴板监控：复制英文单词后自动翻译（默认启用，无需开关）。

use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::time::Duration;

use eframe::egui;

pub struct SelectionWatcher {
    rx: mpsc::Receiver<String>,
}

impl SelectionWatcher {
    pub fn start(ctx: egui::Context) -> Self {
        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || clipboard_loop(tx, ctx));
        Self { rx }
    }

    pub fn try_recv(&self) -> Option<String> {
        self.rx.try_recv().ok()
    }
}

fn clipboard_loop(tx: mpsc::Sender<String>, ctx: egui::Context) {
    win::time_begin_period(1);
    log("clipboard monitor started");
    let mut last_seq = win::clipboard_sequence();
    loop {
        std::thread::sleep(Duration::from_millis(30));
        let seq = win::clipboard_sequence();
        if seq != last_seq {
            last_seq = seq;
            if let Some(word) = read_clipboard_word() {
                log(&format!("word: {}", word));
                let _ = tx.send(word);
                ctx.request_repaint();
            }
        }
    }
}

fn read_clipboard_word() -> Option<String> {
    let text = win::read_clipboard()?;
    clean_selection(&text)
}

/// 从复制的文本中提取首个英文单词（忽略前后空白/标点，多词取第一个）。
fn clean_selection(text: &str) -> Option<String> {
    let t = text.trim();
    if t.is_empty() {
        return None;
    }
    let word: String = t
        .chars()
        .take_while(|c| c.is_ascii_alphabetic() || *c == '\'' || *c == '-')
        .collect();
    if word.is_empty() || word.len() > 40 {
        None
    } else {
        Some(word)
    }
}

// ---------------------------------------------------------------------------
// 日志
// ---------------------------------------------------------------------------

static LOG_ENABLED: AtomicBool = AtomicBool::new(false);

pub fn set_logging(on: bool) {
    LOG_ENABLED.store(on, Ordering::Relaxed);
}

pub fn log(msg: &str) {
    if !LOG_ENABLED.load(Ordering::Relaxed) {
        return;
    }
    // 日志写入 exe 所在目录（仅在 --log 时启用）
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let Some(dir) = exe.parent() else {
        return;
    };
    let path = dir.join("ecdict.log");
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let _ = writeln!(f, "[{}] {}", timestamp(), msg);
    }
}

fn timestamp() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Windows FFI（仅剪贴板）
// ---------------------------------------------------------------------------

#[cfg(windows)]
mod win {
    use std::ffi::c_void;

    type BOOL = i32;
    type HANDLE = *mut c_void;

    #[link(name = "user32")]
    extern "system" {
        fn GetClipboardSequenceNumber() -> u32;
        fn OpenClipboard(h: *mut c_void) -> BOOL;
        fn CloseClipboard() -> BOOL;
        fn GetClipboardData(u_format: u32) -> HANDLE;
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn GlobalLock(h_mem: HANDLE) -> *mut c_void;
        fn GlobalUnlock(h_mem: HANDLE) -> BOOL;
    }

    #[link(name = "winmm")]
    extern "system" {
        fn timeBeginPeriod(u_period: u32) -> u32;
    }

    const CF_UNICODETEXT: u32 = 13;

    pub fn time_begin_period(period_ms: u32) {
        unsafe {
            timeBeginPeriod(period_ms);
        }
    }

    pub fn clipboard_sequence() -> u32 {
        unsafe { GetClipboardSequenceNumber() }
    }

    pub fn read_clipboard() -> Option<String> {
        unsafe {
            if OpenClipboard(std::ptr::null_mut()) == 0 {
                return None;
            }
            let h = GetClipboardData(CF_UNICODETEXT);
            if h.is_null() {
                CloseClipboard();
                return None;
            }
            let ptr = GlobalLock(h) as *const u16;
            if ptr.is_null() {
                CloseClipboard();
                return None;
            }
            let mut len = 0usize;
            while *ptr.add(len) != 0 {
                len += 1;
            }
            let slice = std::slice::from_raw_parts(ptr, len);
            let s = String::from_utf16_lossy(slice);
            GlobalUnlock(h);
            CloseClipboard();
            Some(s)
        }
    }
}

#[cfg(not(windows))]
mod win {
    pub fn time_begin_period(_: u32) {}
    pub fn clipboard_sequence() -> u32 {
        0
    }
    pub fn read_clipboard() -> Option<String> {
        None
    }
}
