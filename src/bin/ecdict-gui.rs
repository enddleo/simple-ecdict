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
            selection: selection::SelectionWatcher::start(cc.egui_ctx.clone()),
        }
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        egui::Color32::from_rgb(248, 248, 248).to_normalized_gamma_f32()
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
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
                let resp = ui.add(
                    egui::TextEdit::singleline(&mut self.input)
                        .id(search_id)
                        .desired_width(ui.available_width()),
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
                ui.label(
                    egui::RichText::new(format!("/{}/", ph))
                        .size(14.0)
                        .color(egui::Color32::from_rgb(130, 130, 130)),
                );
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

/// 加载 Windows 系统中文字体，保证中文释义正常显示。
fn install_cjk_font(ctx: &egui::Context) {
    const CANDIDATES: &[&str] = &[
        "C:\\Windows\\Fonts\\simhei.ttf",  // 黑体 (TTF)
        "C:\\Windows\\Fonts\\Deng.ttf",    // 等线 (TTF)
        "C:\\Windows\\Fonts\\simfang.ttf", // 仿宋 (TTF)
        "C:\\Windows\\Fonts\\simkai.ttf",  // 楷体 (TTF)
        "C:\\Windows\\Fonts\\simsunb.ttf", // 宋体粗体 (TTF)
        "C:\\Windows\\Fonts\\msyh.ttc",    // 微软雅黑 (TTC)
        "C:\\Windows\\Fonts\\simsun.ttc",  // 宋体 (TTC)
        "C:\\Windows\\Fonts\\msjh.ttc",    // JhengHei (TTC)
    ];
    for path in CANDIDATES {
        if let Ok(bytes) = std::fs::read(path) {
            let mut fonts = egui::FontDefinitions::default();
            fonts
                .font_data
                .insert("cjk".to_owned(), egui::FontData::from_owned(bytes).into());
            for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
                fonts
                    .families
                    .entry(family)
                    .or_default()
                    .insert(0, "cjk".to_owned());
            }
            ctx.set_fonts(fonts);
            return;
        }
    }
    eprintln!("警告: 未找到中文字体，中文可能显示为方框");
}

/// 全选指定 TextEdit 中的文本。
fn select_all(ctx: &egui::Context, id: egui::Id, text: &str) {
    if let Some(mut state) = egui::TextEdit::load_state(ctx, id) {
        state.cursor.set_char_range(Some(egui::text::CCursorRange::select_all_str(text)));
        state.store(ctx, id);
    }
}
