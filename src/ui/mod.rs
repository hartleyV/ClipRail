pub mod item;
pub mod settings;
pub mod sidebar;
pub mod theme;

use eframe::egui::{self, Color32, Rect, Rounding, Sense, Stroke, Vec2};

/// 统一风格的小按钮（可选主色调）
pub fn chip_button(
    ui: &mut egui::Ui,
    label: &str,
    active: bool,
    danger: bool,
) -> egui::Response {
    let (fill, stroke_color, text_color) = if danger {
        (theme::RED_SOFT, theme::RED, theme::RED)
    } else if active {
        (theme::ACCENT, theme::ACCENT, Color32::WHITE)
    } else {
        (theme::CARD, theme::BORDER, theme::TEXT)
    };

    let text = egui::RichText::new(label).size(12.0).color(text_color);
    let button = egui::Button::new(text)
        .fill(fill)
        .stroke(Stroke::new(1.0, stroke_color))
        .rounding(Rounding::same(theme::RADIUS_CTRL))
        .min_size(Vec2::new(0.0, 26.0));
    ui.add(button)
}

/// 自绘复选框：未选为白底灰框，选中为蓝底白对勾
pub fn paint_checkbox(ui: &egui::Ui, rect: Rect, checked: bool, hovered: bool) {
    let painter = ui.painter();
    let rounding = Rounding::same(5.0);
    if checked {
        painter.rect_filled(rect, rounding, theme::ACCENT);
        painter.rect_stroke(rect, rounding, Stroke::new(1.0, theme::ACCENT));
        // 白色对勾
        let c = rect.center();
        let s = rect.width();
        let p1 = egui::pos2(c.x - s * 0.24, c.y + s * 0.02);
        let p2 = egui::pos2(c.x - s * 0.06, c.y + s * 0.20);
        let p3 = egui::pos2(c.x + s * 0.26, c.y - s * 0.20);
        let stroke = Stroke::new(2.0, Color32::WHITE);
        painter.line_segment([p1, p2], stroke);
        painter.line_segment([p2, p3], stroke);
    } else {
        painter.rect_filled(rect, rounding, Color32::WHITE);
        let border = if hovered { theme::ACCENT } else { theme::BORDER_STRONG };
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
