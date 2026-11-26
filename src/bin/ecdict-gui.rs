//! ecdict GUI — 离线英汉词典（Windows 图形界面，窗口自适应内容）
#![windows_subsystem = "windows"]

use eframe::egui;

use ecdict::{parse_exchange, parse_translation, Index};

mod selection;

const WIN_WIDTH: f32 = 250.0; // 固定窗口宽度（尽可能小）
const PAD: f32 = 10.0;        // 内边距

fn main() -> eframe::Result {
    // 仅在传入 --log 时启用日志（默认不写日志文件）
    if std::env::args().any(|a| a == "--log") {
        selection::set_logging(true);
    }

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([WIN_WIDTH, 56.0])
            .with_min_inner_size([230.0, 44.0])
            .with_title("simple ecdict")
            .with_icon(app_icon())
            .with_maximize_button(false)
            .with_always_on_top(),
        ..Default::default()
    };
    eframe::run_native(
        "simple_ecdict",
        options,
        Box::new(|cc| Ok(Box::new(App::new(cc)))),
    )
}

struct App {
    index: Index,
    input: String,
    result: Option<String>,
    focus_on_start: bool,
    needs_resize: bool,
    dark: bool,
    selection: selection::SelectionWatcher,
}

impl App {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        install_cjk_font(&cc.egui_ctx);
        cc.egui_ctx.set_visuals(egui::Visuals::light());
        App {
            index: Index::new(),
            input: String::new(),
            result: None,
            focus_on_start: true,
            needs_resize: true,
            dark: false,
            selection: selection::SelectionWatcher::start(cc.egui_ctx.clone()),
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        if self.dark {
            egui::Color32::from_rgb(27, 27, 27).to_normalized_gamma_f32()
        } else {
            egui::Color32::from_rgb(248, 248, 248).to_normalized_gamma_f32()
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        // 启动时把记住的主题同步到标题栏
        if self.focus_on_start {
            set_title_bar_dark(self.dark);
        }

        // 剪贴板监控：处理复制到的英文单词
        while let Some(word) = self.selection.try_recv() {
            self.result = Some(word.clone());
            self.input = word;
            self.needs_resize = true;
        }

        let mut content_h = 0.0;
        egui::Frame::NONE.inner_margin(10).show(ui, |ui| {
            let search_id = egui::Id::unique("search");

            ui.horizontal(|ui| {
                // 输入框（右侧留出亮暗切换按钮空间）
                let button_w = 36.0;
                let input_w = (ui.available_width() - button_w).max(100.0);

                let (bg, border) = if self.dark {
                    (
                        egui::Color32::from_rgb(40, 40, 40),
                        egui::Color32::from_rgb(75, 75, 75),
                    )
                } else {
                    (
                        egui::Color32::WHITE,
                        egui::Color32::from_rgb(175, 175, 175),
                    )
                };
                let frame = egui::Frame::new()
                    .fill(bg)
                    .stroke(egui::Stroke::new(1.0, border))
                    .corner_radius(egui::CornerRadius::same(4))
                    .inner_margin(egui::Margin::symmetric(8, 3));

                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.input)
                        .id(search_id)
                        .frame(frame)
                        .desired_width(input_w),
                );

                if self.focus_on_start {
                    resp.request_focus();
                    self.focus_on_start = false;
                }

                // 鼠标点击/激活输入框时，全选已有内容，便于直接输入新词覆盖
                if resp.gained_focus() {
                    select_all(ui.ctx(), search_id, &self.input);
                }

                // 回车：搜索 + 全选已输入内容 + 保持聚焦
                let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                if enter && (resp.lost_focus() || resp.has_focus()) {
                    let word = self.input.trim().to_string();
                    self.result = if word.is_empty() { None } else { Some(word) };
                    select_all(ui.ctx(), search_id, &self.input);
                    resp.request_focus();
                    self.needs_resize = true;
                }

                // 亮暗切换按钮（居中在输入框右缘与界面右缘之间）
                ui.add_space(7.0);
                let (icon_rect, icon_resp) = ui.allocate_exact_size(
                    egui::vec2(22.0, 22.0),
                    egui::Sense::click(),
                );
                draw_theme_icon(ui, icon_rect, self.dark);
                if icon_resp.clicked() {
                    self.dark = !self.dark;
                    ui.ctx().set_visuals(if self.dark {
                        egui::Visuals::dark()
                    } else {
                        egui::Visuals::light()
                    });
                    set_title_bar_dark(self.dark);
                }
            });

            ui.add_space(8.0);

            if let Some(q) = self.result.clone() {
                match self.index.lookup(&q) {
                    Some(k) => self.render_entry(ui, k),
                    None => self.render_not_found(ui, &q),
                }
            }

            content_h = ui.min_rect().height();
        });

        // 按实测内容高度调整窗口高度（宽度固定，顶部锚定不移动）
        if self.needs_resize {
            let desired_h = (content_h + 2.0 * PAD).max(56.0);
            ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(
                egui::vec2(WIN_WIDTH, desired_h),
            ));
            self.needs_resize = false;
        }
    }
}

impl App {
    fn render_entry(&self, ui: &mut egui::Ui, k: usize) {
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.label(egui::RichText::new(self.index.word(k)).size(24.0).strong());
            let ph = self.index.phonetic(k);
            if !ph.is_empty() {
                // 音标灰色：暗模式下更亮，提升对比度
                let ph_color = if self.dark {
                    egui::Color32::from_rgb(185, 185, 185)
                } else {
                    egui::Color32::from_rgb(130, 130, 130)
                };
                // 顶部留白微调，让音标与单词视觉居中
                egui::Frame::new()
                    .inner_margin(egui::Margin {
                        left: 4,
                        right: 0,
                        top: 2,
                        bottom: 0,
                    })
                    .show(ui, |ui| {
                        ui.label(
                            egui::RichText::new(format!("/{}/", ph))
                                .size(14.0)
                                .color(ph_color),
                        );
                    });
            }
        });
        ui.separator();

        for g in parse_translation(self.index.translation(k)) {
            ui.add_space(4.0);
            let header = if g.pos.is_empty() {
                format!("〔{}〕", g.label)
            } else {
                g.pos.clone()
            };
            ui.label(
                egui::RichText::new(header)
                    .strong()
                    .color(egui::Color32::from_rgb(0xC0, 0x70, 0x20)),
            );
            for m in &g.meanings {
                ui.label(egui::RichText::new(format!("  {}", m)));
            }
        }

        let forms = parse_exchange(self.index.exchange(k));
        if !forms.is_empty() {
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("词态变化")
                    .strong()
                    .color(egui::Color32::from_rgb(0x20, 0x70, 0xC0)),
            );
            ui.add_space(2.0);
            egui::Grid::new("exchange")
                .striped(true)
                .min_col_width(100.0)
                .spacing([14.0, 3.0])
                .show(ui, |ui| {
                    for f in &forms {
                        ui.label(&f.label);
                        ui.label(&f.value);
                        ui.end_row();
                    }
                });
        }
        ui.add_space(6.0);
    }

    fn render_not_found(&mut self, ui: &mut egui::Ui, q: &str) {
        ui.add_space(6.0);
        ui.label(egui::RichText::new(format!("未找到单词：{}", q)).size(16.0));
        let sug = self.index.suggest(q);
        if !sug.is_empty() {
            ui.add_space(6.0);
            ui.label("你是否要找：");
            ui.add_space(2.0);
            ui.horizontal_wrapped(|ui| {
                for w in sug {
                    if ui.link(w).clicked() {
                        self.input = w.to_string();
                        self.result = Some(w.to_string());
                        self.needs_resize = true;
                    }
                }
            });
        }
        ui.add_space(6.0);
    }
}

/// 加载 Windows 系统中文字体 + 含 IPA 音标的拉丁字体，保证中文释义与音标都正常显示。
fn install_cjk_font(ctx: &egui::Context) {
    const CJK_CANDIDATES: &[&str] = &[
        "C:\\Windows\\Fonts\\simhei.ttf",  // 黑体 (TTF)
        "C:\\Windows\\Fonts\\Deng.ttf",    // 等线 (TTF)
        "C:\\Windows\\Fonts\\simfang.ttf", // 仿宋 (TTF)
        "C:\\Windows\\Fonts\\simkai.ttf",  // 楷体 (TTF)
        "C:\\Windows\\Fonts\\simsunb.ttf", // 宋体粗体 (TTF)
        "C:\\Windows\\Fonts\\msyh.ttc",    // 微软雅黑 (TTC)
        "C:\\Windows\\Fonts\\simsun.ttc",  // 宋体 (TTC)
        "C:\\Windows\\Fonts\\msjh.ttc",    // JhengHei (TTC)
    ];
    // 含 IPA 音标字符（ə ʃ ʊ ɪ 等）的拉丁字体，作为音标回退
    const LATIN_CANDIDATES: &[&str] = &[
        "C:\\Windows\\Fonts\\arial.ttf",   // Arial（含 IPA 扩展）
        "C:\\Windows\\Fonts\\segoeui.ttf", // Segoe UI（含 IPA）
        "C:\\Windows\\Fonts\\times.ttf",   // Times New Roman（含 IPA）
    ];

    let mut fonts = egui::FontDefinitions::default();

    let mut cjk_loaded = false;
    for path in CJK_CANDIDATES {
        if let Ok(bytes) = std::fs::read(path) {
            fonts
                .font_data
                .insert("cjk".to_owned(), egui::FontData::from_owned(bytes).into());
            cjk_loaded = true;
            break;
        }
    }

    let mut latin_loaded = false;
    for path in LATIN_CANDIDATES {
        if let Ok(bytes) = std::fs::read(path) {
            fonts
                .font_data
                .insert("latin".to_owned(), egui::FontData::from_owned(bytes).into());
            latin_loaded = true;
            break;
        }
    }

    if cjk_loaded || latin_loaded {
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
            let fam = fonts.families.entry(family).or_default();
            if cjk_loaded {
                fam.insert(0, "cjk".to_owned());
            }
            if latin_loaded {
                fam.push("latin".to_owned());
            }
        }
        ctx.set_fonts(fonts);
    } else {
        eprintln!("警告: 未找到中文字体，中文可能显示为方框");
    }
}

/// 全选指定 TextEdit 中的文本。
fn select_all(ctx: &egui::Context, id: egui::Id, text: &str) {
    if let Some(mut state) = egui::TextEdit::load_state(ctx, id) {
        state.cursor.set_char_range(Some(egui::text::CCursorRange::select_all_str(text)));
        state.store(ctx, id);
    }
}

/// 自绘亮暗切换图标：暗色显示太阳，亮色显示月亮（月牙）。
fn draw_theme_icon(ui: &egui::Ui, rect: egui::Rect, dark: bool) {
    let center = rect.center();
    let color = ui.visuals().text_color();
    if dark {
        // 太阳：实心圆 + 八条光芒
        ui.painter().circle_filled(center, 6.0, color);
        for i in 0..8 {
            let a = i as f32 * std::f32::consts::TAU / 8.0;
            let d = egui::vec2(a.cos(), a.sin());
            ui.painter().line_segment(
                [center + d * 8.0, center + d * 10.5],
                egui::Stroke::new(1.5, color),
            );
        }
    } else {
        // 月亮：实心圆减去一个偏移圆，形成月牙（减去用亮色背景）
        ui.painter().circle_filled(center, 8.0, color);
        ui.painter()
            .circle_filled(center + egui::vec2(4.0, -2.0), 6.5, egui::Color32::from_rgb(248, 248, 248));
    }
}

/// 从嵌入的 PNG 加载窗口图标。
fn app_icon() -> egui::IconData {
    const PNG: &[u8] = include_bytes!("../../assets/icon.png");
    eframe::icon_data::from_png_bytes(PNG).unwrap_or_default()
}

/// 让 Windows 原生标题栏跟随亮/暗模式（沉浸式深色标题栏）。
#[cfg(windows)]
fn set_title_bar_dark(dark: bool) {
    use std::ffi::c_void;

    #[link(name = "dwmapi")]
    extern "system" {
        fn DwmSetWindowAttribute(
            hwnd: *mut c_void,
            attr: u32,
            value: *const c_void,
            size: u32,
        ) -> i32;
    }

    #[link(name = "user32")]
    extern "system" {
        fn FindWindowW(class: *const u16, window: *const u16) -> *mut c_void;
    }

    unsafe {
        let title: Vec<u16> = "simple ecdict"
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect();
        let hwnd = FindWindowW(std::ptr::null(), title.as_ptr());
        if !hwnd.is_null() {
            let v: i32 = if dark { 1 } else { 0 };
            // DWMWA_USE_IMMERSIVE_DARK_MODE = 20
            let _ = DwmSetWindowAttribute(
                hwnd,
                20,
                &v as *const i32 as *const c_void,
                4,
            );
        }
    }
}

#[cfg(not(windows))]
fn set_title_bar_dark(_dark: bool) {}
