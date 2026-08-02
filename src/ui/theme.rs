//! 配色、字体与全局样式。整体风格：浅色、干净、低干扰。

use eframe::egui::{self, Color32, FontFamily, FontId, Rounding, Stroke, TextStyle};

pub const TEXT: Color32 = Color32::from_rgb(0x2C, 0x2C, 0x2B);
pub const MUTED: Color32 = Color32::from_rgb(0x7D, 0x7A, 0x75);
pub const CANVAS: Color32 = Color32::from_rgb(0xF9, 0xF8, 0xF7);
pub const CARD: Color32 = Color32::from_rgb(0xFF, 0xFF, 0xFF);
pub const SURFACE: Color32 = Color32::from_rgb(0xF0, 0xEF, 0xED);
pub const BORDER: Color32 = Color32::from_rgb(0xE6, 0xE5, 0xE3);
pub const BORDER_STRONG: Color32 = Color32::from_rgb(0xCF, 0xCD, 0xC9);
pub const ACCENT: Color32 = Color32::from_rgb(0x27, 0x83, 0xDE);
pub const ACCENT_SOFT: Color32 = Color32::from_rgb(0xE5, 0xF2, 0xFC);
pub const GREEN: Color32 = Color32::from_rgb(0x46, 0xA1, 0x71);
pub const RED: Color32 = Color32::from_rgb(0xE5, 0x64, 0x58);
pub const RED_SOFT: Color32 = Color32::from_rgb(0xFC, 0xE9, 0xE7);

pub const RADIUS_CARD: f32 = 10.0;
pub const RADIUS_CTRL: f32 = 8.0;

/// 字号统一使用整数，避免半像素导致的发虚
pub const FONT_TITLE: f32 = 15.0;
pub const FONT_BODY: f32 = 13.0;
pub const FONT_BUTTON: f32 = 12.0;
pub const FONT_SMALL: f32 = 11.0;

/// 安装中文字体（系统自带，不增加体积）与全局样式
pub fn install(ctx: &egui::Context) {
    install_fonts(ctx);
    install_style(ctx);
}

/// 优先选择屏幕显示清晰、带完整 hinting 的无衬线字体
fn cjk_font_candidates() -> Vec<&'static str> {
    vec![
        // Windows：微软雅黑 UI / 微软雅黑
        "C:/Windows/Fonts/msyh.ttc",
        "C:/Windows/Fonts/msyh.ttf",
        "C:/Windows/Fonts/msyhl.ttc",
        "C:/Windows/Fonts/Deng.ttf",
        "C:/Windows/Fonts/simhei.ttf",
        "C:/Windows/Fonts/simsun.ttc",
        // Linux：Noto Sans CJK / 文泉驿微米黑
        "/usr/share/fonts/opentype/noto/NotoSansCJKsc-Regular.otf",
        "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/google-noto-cjk/NotoSansCJK-Regular.ttc",
        "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        "/usr/share/fonts/wenquanyi/wqy-microhei/wqy-microhei.ttc",
        "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    ]
}

fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    for path in cjk_font_candidates() {
        if let Ok(bytes) = std::fs::read(path) {
            fonts
                .font_data
                .insert("ui_cjk".to_owned(), egui::FontData::from_owned(bytes));
            // 放在第一位：中文与英文使用同一套字形，粗细与基线一致，观感更清晰
            fonts
                .families
                .entry(FontFamily::Proportional)
                .or_default()
                .insert(0, "ui_cjk".to_owned());
            fonts
                .families
                .entry(FontFamily::Monospace)
                .or_default()
                .insert(1, "ui_cjk".to_owned());
            break;
        }
    }
    ctx.set_fonts(fonts);
}

fn install_style(ctx: &egui::Context) {
    let mut style = (*ctx.style()).clone();
    style.visuals = egui::Visuals::light();

    style.text_styles = [
        (
            TextStyle::Heading,
            FontId::new(FONT_TITLE, FontFamily::Proportional),
        ),
        (
            TextStyle::Body,
            FontId::new(FONT_BODY, FontFamily::Proportional),
        ),
        (
            TextStyle::Button,
            FontId::new(FONT_BUTTON, FontFamily::Proportional),
        ),
        (
            TextStyle::Small,
            FontId::new(FONT_SMALL, FontFamily::Proportional),
        ),
        (
            TextStyle::Monospace,
            FontId::new(FONT_BODY, FontFamily::Monospace),
        ),
    ]
    .into();

    let v = &mut style.visuals;
    v.panel_fill = CANVAS;
    v.window_fill = CARD;
    v.extreme_bg_color = CARD;
    v.faint_bg_color = SURFACE;
    v.window_stroke = Stroke::new(1.0, BORDER);
    v.window_rounding = Rounding::same(12.0);
    v.menu_rounding = Rounding::same(10.0);
    v.window_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 4.0),
        blur: 16.0,
        spread: 0.0,
        color: Color32::from_black_alpha(18),
    };
    v.popup_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 3.0),
        blur: 12.0,
        spread: 0.0,
        color: Color32::from_black_alpha(20),
    };
    v.selection.bg_fill = ACCENT_SOFT;
    v.selection.stroke = Stroke::new(1.0, ACCENT);

    v.widgets.noninteractive.bg_fill = CANVAS;
    v.widgets.noninteractive.weak_bg_fill = CANVAS;
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0, BORDER);
    v.widgets.noninteractive.fg_stroke = Stroke::new(1.0, TEXT);
    v.widgets.noninteractive.rounding = Rounding::same(RADIUS_CTRL);

    v.widgets.inactive.bg_fill = CARD;
    v.widgets.inactive.weak_bg_fill = CARD;
    v.widgets.inactive.bg_stroke = Stroke::new(1.0, BORDER);
    v.widgets.inactive.fg_stroke = Stroke::new(1.0, TEXT);
    v.widgets.inactive.rounding = Rounding::same(RADIUS_CTRL);
    v.widgets.inactive.expansion = 0.0;

    v.widgets.hovered.bg_fill = ACCENT_SOFT;
    v.widgets.hovered.weak_bg_fill = ACCENT_SOFT;
    v.widgets.hovered.bg_stroke = Stroke::new(1.0, ACCENT);
    v.widgets.hovered.fg_stroke = Stroke::new(1.0, TEXT);
    v.widgets.hovered.rounding = Rounding::same(RADIUS_CTRL);
    v.widgets.hovered.expansion = 0.0;

    v.widgets.active.bg_fill = ACCENT_SOFT;
    v.widgets.active.weak_bg_fill = ACCENT_SOFT;
    v.widgets.active.bg_stroke = Stroke::new(1.0, ACCENT);
    v.widgets.active.fg_stroke = Stroke::new(1.0, TEXT);
    v.widgets.active.rounding = Rounding::same(RADIUS_CTRL);
    v.widgets.active.expansion = 0.0;

    v.widgets.open.bg_fill = CARD;
    v.widgets.open.weak_bg_fill = CARD;
    v.widgets.open.bg_stroke = Stroke::new(1.0, ACCENT);
    v.widgets.open.fg_stroke = Stroke::new(1.0, TEXT);
    v.widgets.open.rounding = Rounding::same(RADIUS_CTRL);

    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    style.spacing.window_margin = egui::Margin::same(18.0);
    style.spacing.menu_margin = egui::Margin::same(6.0);
    style.spacing.scroll.bar_width = 8.0;
    style.spacing.scroll.floating = true;
    style.spacing.interact_size = egui::vec2(28.0, 26.0);
    style.spacing.combo_height = 360.0;
    style.spacing.icon_width = 16.0;
    style.spacing.icon_width_inner = 10.0;

    ctx.set_style(style);
}
