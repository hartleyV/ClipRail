//! 单条 Item 的绘制。
//! 内部按钮（复选框、置顶）采用“自绘 + 区域命中”方式，
//! 避免与整体单击复制、拖拽手势互相抢占事件。

use eframe::egui::{self, Color32, Rect, Rounding, Sense, Stroke, Vec2};

use crate::model::{Item, Kind};
use crate::ui::theme;

pub struct ItemView<'a> {
    pub item: &'a Item,
    pub selected: bool,
    pub copied: bool,
    pub dragging: bool,
    pub texture: Option<&'a egui::TextureHandle>,
}

pub struct ItemOutput {
    pub rect: Rect,
    pub checkbox_rect: Rect,
    pub pin_rect: Rect,
}

pub fn show(ui: &mut egui::Ui, view: ItemView) -> ItemOutput {
    let item = view.item;
    let pinned = item.pinned;

    let fill = if view.selected {
        theme::ACCENT_SOFT
    } else if pinned {
        theme::ACCENT_SOFT
    } else {
        theme::CARD
    };
    let base_stroke = if pinned || view.selected {
        Stroke::new(1.0, theme::ACCENT)
    } else {
        Stroke::new(1.0, theme::BORDER)
    };

    let mut checkbox_rect = Rect::NOTHING;
    let mut pin_rect = Rect::NOTHING;

    let frame = egui::Frame::none()
        .fill(fill)
        .stroke(base_stroke)
        .rounding(Rounding::same(theme::RADIUS_CARD))
        .inner_margin(egui::Margin::symmetric(10.0, 10.0));

    let inner = frame.show(ui, |ui| {
        ui.set_width(ui.available_width());

        // ---------------- 顶部行：复选框 / 时间 / 置顶按钮
        ui.horizontal(|ui| {
            let (cb_rect, _) = ui.allocate_exact_size(Vec2::splat(18.0), Sense::hover());
            checkbox_rect = cb_rect.expand(3.0);
            let cb_hover = ui.rect_contains_pointer(checkbox_rect);
            crate::ui::paint_checkbox(ui, cb_rect, view.selected, cb_hover);

            ui.add_space(8.0);
            // 显示“已复制”提示时让出时间位置，避免两者重叠
            if view.copied {
                ui.allocate_exact_size(Vec2::new(80.0, 18.0), Sense::hover());
            } else {
                ui.label(
                    egui::RichText::new(item.local_time())
                        .size(11.0)
                        .color(theme::MUTED),
                );
            }

            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                let label = if pinned { "已置顶" } else { "置顶" };
                let size = Vec2::new(if pinned { 54.0 } else { 42.0 }, 22.0);
                let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
                pin_rect = rect;
                let hovered = ui.rect_contains_pointer(rect);
                let (bg, border, fg) = if pinned {
                    (theme::ACCENT, theme::ACCENT, Color32::WHITE)
                } else if hovered {
                    (theme::ACCENT_SOFT, theme::ACCENT, theme::ACCENT)
                } else {
                    (theme::SURFACE, theme::BORDER, theme::MUTED)
                };
                let painter = ui.painter();
                painter.rect_filled(rect, Rounding::same(6.0), bg);
                painter.rect_stroke(rect, Rounding::same(6.0), Stroke::new(1.0, border));
                painter.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    label,
                    egui::FontId::proportional(11.0),
                    fg,
                );
            });
        });

        ui.add_space(8.0);

        // ---------------- 内容区
        match item.kind {
            Kind::Text => {
                ui.label(
                    egui::RichText::new(item.preview())
                        .size(13.0)
                        .color(theme::TEXT),
                );
            }
            Kind::Image => match view.texture {
                Some(texture) => {
                    let avail = ui.available_width();
                    let source = egui::load::SizedTexture::from_handle(texture);
                    ui.add(
                        egui::Image::new(source)
                            .max_width(avail)
                            .max_height(240.0)
                            .maintain_aspect_ratio(true),
                    );
                }
                None => {
                    let avail = ui.available_width();
                    let (rect, _) =
                        ui.allocate_exact_size(Vec2::new(avail, 84.0), Sense::hover());
                    let painter = ui.painter();
                    painter.rect_filled(rect, Rounding::same(6.0), theme::SURFACE);
                    painter.text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "图片加载中…",
                        egui::FontId::proportional(12.0),
                        theme::MUTED,
                    );
                }
            },
        }

        // ---------------- 底部信息
        ui.add_space(6.0);
        let meta = match item.kind {
            Kind::Text => format!("文本 · {} 字", item.char_count()),
            Kind::Image => {
                if item.width > 0 {
                    format!("图片 · {}×{}", item.width, item.height)
                } else {
                    "图片".to_string()
                }
            }
        };
        ui.label(egui::RichText::new(meta).size(10.5).color(theme::MUTED));
    });

    let rect = inner.response.rect;

    // 悬停时边框轻微变化（在已知矩形上重绘，不影响布局）
    if ui.rect_contains_pointer(rect) && !view.dragging {
        let color = if pinned || view.selected {
            theme::ACCENT
        } else {
            theme::BORDER_STRONG
        };
        ui.painter().rect_stroke(
            rect,
            Rounding::same(theme::RADIUS_CARD),
            Stroke::new(1.4, color),
        );
    }

    // 拖拽中：淡出至约 50%
    if view.dragging {
        ui.painter().rect_filled(
            rect,
            Rounding::same(theme::RADIUS_CARD),
            Color32::from_rgba_unmultiplied(249, 248, 247, 130),
        );
        ui.painter().rect_stroke(
            rect,
            Rounding::same(theme::RADIUS_CARD),
            Stroke::new(1.4, theme::ACCENT),
        );
    }

    // “✓ 已复制”提示：左上区域，不遮挡复选框与置顶按钮
    if view.copied {
        let badge = Rect::from_min_size(
            egui::pos2(rect.left() + 38.0, rect.top() + 9.0),
            Vec2::new(78.0, 21.0),
        );
        let painter = ui.painter();
        painter.rect_filled(badge, Rounding::same(6.0), theme::GREEN);
        painter.text(
            badge.center(),
            egui::Align2::CENTER_CENTER,
            "✓ 已复制",
            egui::FontId::proportional(12.5),
            Color32::WHITE,
        );
    }

    ItemOutput {
        rect,
        checkbox_rect,
        pin_rect,
    }
}

/// 拖拽时跟随鼠标的缩略图
pub fn paint_drag_ghost(ctx: &egui::Context, pos: egui::Pos2, label: &str) {
    let layer = egui::LayerId::new(egui::Order::Tooltip, egui::Id::new("cliprail_drag_ghost"));
    let painter = ctx.layer_painter(layer);
    let size = Vec2::new(180.0, 34.0);
    let rect = Rect::from_min_size(pos + Vec2::new(12.0, 12.0), size);
    painter.rect_filled(rect, Rounding::same(8.0), Color32::from_rgb(0x2C, 0x2C, 0x2B));
    painter.text(
        rect.left_center() + Vec2::new(12.0, 0.0),
        egui::Align2::LEFT_CENTER,
        label,
        egui::FontId::proportional(12.0),
        Color32::WHITE,
    );
}
