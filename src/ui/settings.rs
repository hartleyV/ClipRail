//! 设置窗口：仅保留快捷键与边缘收纳。

use eframe::egui::{self, Rounding, Stroke};

use crate::app::App;
use crate::ui::{theme, toggle_switch, wide_button};

pub fn show(app: &mut App, ctx: &egui::Context) {
    if !app.show_settings {
        return;
    }

    let mut open = true;
    let mut save_clicked = false;
    let mut cancel_clicked = false;

    egui::Window::new("设置")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .default_width(300.0)
        .show(ctx, |ui| {
            ui.set_width(ui.available_width().min(300.0));

            // ---------------- 快捷键
            ui.label(
                egui::RichText::new("显示 / 隐藏快捷键")
                    .size(theme::FONT_BODY)
                    .strong()
                    .color(theme::TEXT),
            );
            ui.add_space(6.0);
            let edit = egui::TextEdit::singleline(&mut app.draft_hotkey)
                .hint_text("alt+v")
                .desired_width(f32::INFINITY)
                .margin(egui::Margin::symmetric(8.0, 6.0));
            ui.add(edit);
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new("修饰键：ctrl / alt / shift / super；主键：A–Z 或 F1–F12")
                    .size(theme::FONT_SMALL)
                    .color(theme::MUTED),
            );
            ui.label(
                egui::RichText::new("例：alt+shift+v、ctrl+alt+c、super+shift+v")
                    .size(theme::FONT_SMALL)
                    .color(theme::MUTED),
            );

            ui.add_space(14.0);
            ui.separator();
            ui.add_space(12.0);

            // ---------------- 边缘收纳
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    ui.label(
                        egui::RichText::new("边缘收纳")
                            .size(theme::FONT_BODY)
                            .strong()
                            .color(theme::TEXT),
                    );
                    ui.add_space(2.0);
                    ui.label(
                        egui::RichText::new("鼠标离开后贴边保留窄条，移回即可展开")
                            .size(theme::FONT_SMALL)
                            .color(theme::MUTED),
                    );
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    toggle_switch(ui, &mut app.draft_edge_hide);
                });
            });

            // ---------------- 错误提示
            if let Some(err) = app.settings_error.clone() {
                ui.add_space(12.0);
                egui::Frame::none()
                    .fill(theme::RED_SOFT)
                    .stroke(Stroke::new(1.0, theme::RED))
                    .rounding(Rounding::same(theme::RADIUS_CTRL))
                    .inner_margin(egui::Margin::symmetric(10.0, 8.0))
                    .show(ui, |ui| {
                        ui.set_width(ui.available_width());
                        ui.label(egui::RichText::new(err).size(theme::FONT_SMALL).color(theme::RED));
                    });
            }

            ui.add_space(18.0);
            ui.horizontal(|ui| {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if wide_button(ui, "保存", true).clicked() {
                        save_clicked = true;
                    }
                    if wide_button(ui, "取消", false).clicked() {
                        cancel_clicked = true;
                    }
                });
            });
        });

    if save_clicked {
        app.save_settings_draft();
    }
    if cancel_clicked || !open {
        app.close_settings();
    }
}
