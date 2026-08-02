pub mod item;
pub mod settings;
pub mod sidebar;
pub mod theme;

use eframe::egui::{self, Color32, Rect, Rounding, Sense, Stroke, Vec2};

/// 测量一段文字的宽度
pub fn text_width(ui: &egui::Ui, text: &str, size: f32) -> f32 {
    let font = egui::FontId::proportional(size);
    ui.fonts(|f| {
        f.layout_no_wrap(text.to_owned(), font, Color32::BLACK)
            .size()
            .x
    })
}

/// 统一风格的小按钮。
/// 文字使用 CENTER_CENTER 直接绘制，不受字体基线影响，水平与垂直都严格居中。
pub fn chip_button(
    ui: &mut egui::Ui,
    label: &str,
    active: bool,
    danger: bool,
) -> egui::Response {
    let size = theme::FONT_BUTTON;
    let width = text_width(ui, label, size) + 22.0;
    let (rect, response) =
        ui.allocate_exact_size(Vec2::new(width.max(46.0), 26.0), Sense::click());
    let hovered = response.hovered();

    let (fill, stroke_color, text_color) = if danger {
        if hovered {
            (theme::RED, theme::RED, Color32::WHITE)
        } else {
            (theme::RED_SOFT, theme::RED, theme::RED)
        }
    } else if active {
        (theme::ACCENT, theme::ACCENT, Color32::WHITE)
    } else if hovered {
        (theme::ACCENT_SOFT, theme::ACCENT, theme::ACCENT)
    } else {
        (theme::CARD, theme::BORDER_STRONG, theme::TEXT)
    };

    if hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }

    let painter = ui.painter();
    let rounding = Rounding::same(theme::RADIUS_CTRL);
    painter.rect_filled(rect, rounding, fill);
    painter.rect_stroke(rect, rounding, Stroke::new(1.0, stroke_color));
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(size),
        text_color,
    );
    response
}

/// 设置窗口的大按钮（主按钮 / 次按钮），文字同样严格居中
pub fn wide_button(ui: &mut egui::Ui, label: &str, primary: bool) -> egui::Response {
    let size = theme::FONT_BODY;
    let width = (text_width(ui, label, size) + 32.0).max(76.0);
    let (rect, response) = ui.allocate_exact_size(Vec2::new(width, 30.0), Sense::click());
    let hovered = response.hovered();
    if hovered {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let (fill, stroke_color, text_color) = if primary {
        let fill = if hovered {
            Color32::from_rgb(0x1B, 0x69, 0xB8)
        } else {
            theme::ACCENT
        };
        (fill, fill, Color32::WHITE)
    } else if hovered {
        (theme::ACCENT_SOFT, theme::ACCENT, theme::ACCENT)
    } else {
        (theme::CARD, theme::BORDER_STRONG, theme::TEXT)
    };

    let painter = ui.painter();
    let rounding = Rounding::same(theme::RADIUS_CTRL);
    painter.rect_filled(rect, rounding, fill);
    painter.rect_stroke(rect, rounding, Stroke::new(1.0, stroke_color));
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(size),
        text_color,
    );
    response
}

/// 绘制对勾（矢量绘制，不依赖字体中是否包含 ✓ 字形，避免出现方框）
pub fn paint_check_mark(
    painter: &egui::Painter,
    center: egui::Pos2,
    size: f32,
    color: Color32,
    thickness: f32,
) {
    let p1 = egui::pos2(center.x - size * 0.42, center.y + size * 0.04);
    let p2 = egui::pos2(center.x - size * 0.12, center.y + size * 0.34);
    let p3 = egui::pos2(center.x + size * 0.44, center.y - size * 0.32);
    let stroke = Stroke::new(thickness, color);
    painter.line_segment([p1, p2], stroke);
    painter.line_segment([p2, p3], stroke);
    // 转角处补一个小圆点，避免锚齿
    painter.circle_filled(p2, thickness * 0.5, color);
}

/// 自绘复选框：未选为白底灰框，选中为蓝底白对勾
pub fn paint_checkbox(ui: &egui::Ui, rect: Rect, checked: bool, hovered: bool) {
    let painter = ui.painter();
    let rounding = Rounding::same(5.0);
    if checked {
        painter.rect_filled(rect, rounding, theme::ACCENT);
        painter.rect_stroke(rect, rounding, Stroke::new(1.0, theme::ACCENT));
        paint_check_mark(painter, rect.center(), rect.width() * 0.62, Color32::WHITE, 2.0);
    } else {
        painter.rect_filled(rect, rounding, Color32::WHITE);
        let border = if hovered {
            theme::ACCENT
        } else {
            theme::BORDER_STRONG
        };
        painter.rect_stroke(rect, rounding, Stroke::new(1.0, border));
    }
}

/// 自绘开关（设置窗口使用）
pub fn toggle_switch(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let desired = Vec2::new(42.0, 24.0);
    let (rect, mut response) = ui.allocate_exact_size(desired, Sense::click());
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    let t = ui.ctx().animate_bool(response.id, *on);
    let painter = ui.painter();
    let radius = rect.height() / 2.0;
    let bg = if *on { theme::ACCENT } else { theme::SURFACE };
    painter.rect_filled(rect, Rounding::same(radius), bg);
    painter.rect_stroke(
        rect,
        Rounding::same(radius),
        Stroke::new(1.0, if *on { theme::ACCENT } else { theme::BORDER_STRONG }),
    );
    let cx = egui::lerp((rect.left() + radius)..=(rect.right() - radius), t);
    painter.circle_filled(egui::pos2(cx, rect.center().y), radius - 3.0, Color32::WHITE);
    response
}

/// 空状态提示
pub fn empty_state(ui: &mut egui::Ui, title: &str, hint: &str) {
    ui.add_space(56.0);
    ui.vertical_centered(|ui| {
        ui.label(
            egui::RichText::new(title)
                .size(14.0)
                .color(theme::TEXT)
                .strong(),
        );
        ui.add_space(6.0);
        ui.label(egui::RichText::new(hint).size(12.0).color(theme::MUTED));
    });
}
