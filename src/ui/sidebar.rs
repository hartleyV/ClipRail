//! 竖栏主界面：顶部工具栏 + 列表 + 底部提示。

use std::collections::HashSet;

use eframe::egui::{self, Color32, Rect, Rounding, Sense, Stroke, Vec2};

use crate::app::{Action, App, DateFilter};
use crate::model::Kind;
use crate::ui::{chip_button, empty_state, item as item_ui, theme};

pub fn show(app: &mut App, ctx: &egui::Context) {
    app.preload_textures(ctx);

    let mut actions: Vec<Action> = Vec::new();
    let mut rows: Vec<(String, Rect)> = Vec::new();
    let mut visible_now: HashSet<String> = HashSet::new();

    let visible = app.visible_indices();
    let selected_count = app.selected.len();
    let drag_id = app.drag.as_ref().map(|d| d.id.clone());

    // ------------------------------------------------------------ 顶部工具栏
    egui::TopBottomPanel::top("cliprail_top")
        .frame(
            egui::Frame::none()
                .fill(theme::CARD)
                .inner_margin(egui::Margin::symmetric(12.0, 10.0))
                .stroke(Stroke::NONE),
        )
        .show(ctx, |ui| {
            // 第一行：标题（可拖动窗口）+ 固定 / 设置
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new("ClipRail")
                        .size(15.0)
                        .strong()
                        .color(theme::TEXT),
                );
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(format!("{} 条", app.items.len()))
                        .size(11.0)
                        .color(theme::MUTED),
                );

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if chip_button(ui, "设置", false, false)
                        .on_hover_text("快捷键与边缘收纳")
                        .clicked()
                    {
                        actions.push(Action::OpenSettings);
                    }
                    let pinned = app.settings.panel_pinned;
                    let label = if pinned { "已置顶" } else { "自动隐藏" };
                    let hint = if pinned {
                        "竖栏保持可见并显示在普通窗口上方，点击取消"
                    } else {
                        "鼠标离开后自动隐藏，点击保持常驻"
                    };
                    if chip_button(ui, label, pinned, false)
                        .on_hover_text(hint)
                        .clicked()
                    {
                        actions.push(Action::TogglePanelPin);
                    }

                    // 剩余区域作为窗口拖动把手
                    let remaining = ui.available_size();
                    if remaining.x > 4.0 {
                        let (rect, resp) =
                            ui.allocate_exact_size(remaining, Sense::click_and_drag());
                        let _ = rect;
                        if resp.drag_started() || resp.is_pointer_button_down_on() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                        }
                        if resp.hovered() {
                            ctx.set_cursor_icon(egui::CursorIcon::Grab);
                        }
                    }
                });
            });

            ui.add_space(8.0);

            // 第二行：日期筛选 + 批量操作
            ui.horizontal(|ui| {
                let current_label = app.filter_label();
                egui::ComboBox::from_id_source("cliprail_date")
                    .selected_text(egui::RichText::new(current_label).size(12.0))
                    .width(150.0)
                    .show_ui(ui, |ui| {
                        let total = app.items.len();
                        if ui
                            .selectable_label(
                                matches!(app.filter, DateFilter::All),
                                format!("全部记录（{}）", total),
                            )
                            .clicked()
                        {
                            actions.push(Action::SetFilter(DateFilter::All));
                        }
                        let today_count = app.count_for_date(&crate::app::today_string());
                        if ui
                            .selectable_label(
                                matches!(app.filter, DateFilter::Today),
                                format!("今天（{}）", today_count),
                            )
                            .clicked()
                        {
                            actions.push(Action::SetFilter(DateFilter::Today));
                        }
                        ui.separator();
                        for (date, count) in app.date_options() {
                            let selected = match &app.filter {
                                DateFilter::Day(d) => d == &date,
                                _ => false,
                            };
                            if ui
                                .selectable_label(selected, format!("{}（{}）", date, count))
                                .clicked()
                            {
                                actions.push(Action::SetFilter(DateFilter::Day(date.clone())));
                            }
                        }
                    });

                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if selected_count > 0 {
                        if chip_button(ui, &format!("删除 {}", selected_count), false, true)
                            .clicked()
                        {
                            actions.push(Action::DeleteSelected);
                        }
                        if chip_button(ui, "取消", false, false).clicked() {
                            actions.push(Action::ClearSelection);
                        }
                    } else if !visible.is_empty() {
                        if chip_button(ui, "全选", false, false).clicked() {
                            actions.push(Action::SelectAllVisible);
                        }
                    }
                });
            });
        });

    // ------------------------------------------------------------ 底部提示
    egui::TopBottomPanel::bottom("cliprail_bottom")
        .frame(
            egui::Frame::none()
                .fill(theme::CARD)
                .inner_margin(egui::Margin::symmetric(12.0, 8.0)),
        )
        .show(ctx, |ui| {
            let tip = format!(
                "单击复制 · 上下拖动排序 · 拖出窗口删除 · {} 显示/隐藏",
                app.settings.hotkey.to_uppercase()
            );
            ui.label(egui::RichText::new(tip).size(10.5).color(theme::MUTED));
        });

    // ------------------------------------------------------------ 列表
    egui::CentralPanel::default()
        .frame(
            egui::Frame::none()
                .fill(theme::CANVAS)
                .inner_margin(egui::Margin::symmetric(12.0, 10.0)),
        )
        .show(ctx, |ui| {
            if let Some(err) = &app.hotkey_error {
                banner(ui, err);
                ui.add_space(8.0);
            }

            if visible.is_empty() {
                if app.items.is_empty() {
                    empty_state(ui, "还没有记录", "复制任意文本或图片，ClipRail 会自动保存到这里");
                } else {
                    empty_state(ui, "该日期没有记录", "切换到“全部记录”查看其他内容");
                }
                return;
            }

            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 11.0;
                    let clip = ui.clip_rect();
                    let mut pinned_section_done = false;

                    for (pos, &index) in visible.iter().enumerate() {
                        let item = &app.items[index];

                        // 置顶区与普通区之间的分隔标题
                        if !item.pinned && !pinned_section_done && pos > 0 {
                            pinned_section_done = true;
                            ui.add_space(2.0);
                            ui.label(
                                egui::RichText::new("最近")
                                    .size(10.5)
                                    .color(theme::MUTED),
                            );
                        }
                        if item.pinned && pos == 0 {
                            ui.label(
                                egui::RichText::new("已置顶")
                                    .size(10.5)
                                    .color(theme::ACCENT),
                            );
                        }

                        let dragging = drag_id.as_deref() == Some(item.id.as_str());
                        let copied = app
                            .copied
                            .as_ref()
                            .map(|(id, _)| id == &item.id)
                            .unwrap_or(false);
                        let texture = app.textures.get(&item.id).and_then(|t| t.as_ref());

                        let out = item_ui::show(
                            ui,
                            item_ui::ItemView {
                                item,
                                selected: app.selected.contains(&item.id),
                                copied,
                                dragging,
                                texture,
                            },
                        );

                        if clip.intersects(out.rect) {
                            visible_now.insert(item.id.clone());
                        }
                        rows.push((item.id.clone(), out.rect));

                        let response = ui.interact(
                            out.rect,
                            egui::Id::new(("cliprail_item", &item.id)),
                            Sense::click_and_drag(),
                        );

                        if response.hovered() && app.drag.is_none() {
                            ctx.set_cursor_icon(egui::CursorIcon::PointingHand);
                        }

                        if response.clicked() {
                            if let Some(pointer) = response.interact_pointer_pos() {
                                if out.checkbox_rect.contains(pointer) {
                                    actions.push(Action::ToggleSelect(item.id.clone()));
                                } else if out.pin_rect.contains(pointer) {
                                    actions.push(Action::TogglePin(item.id.clone()));
                                } else {
                                    actions.push(Action::Copy(item.id.clone()));
                                }
                            }
                        }

                        if response.drag_started() {
                            if let Some(pointer) = response.interact_pointer_pos() {
                                if !out.checkbox_rect.contains(pointer)
                                    && !out.pin_rect.contains(pointer)
                                {
                                    actions.push(Action::DragStart(item.id.clone()));
                                }
                            }
                        }

                        if response.drag_stopped() && dragging {
                            let pointer = response
                                .interact_pointer_pos()
                                .or_else(|| ctx.input(|i| i.pointer.latest_pos()));
                            actions.push(Action::DragStop(pointer));
                        }
                    }
                });
        });

    // ------------------------------------------------------------ 拖拽反馈
    if let Some(drag) = &app.drag {
        let pointer = ctx.input(|i| i.pointer.latest_pos());
        if let Some(pos) = pointer {
            let outside = !ctx.screen_rect().contains(pos);
            let label = if outside {
                "松开删除该记录".to_string()
            } else {
                match app.items.iter().find(|i| i.id == drag.id) {
                    Some(item) if item.kind == Kind::Image => "移动图片记录".to_string(),
                    Some(item) => {
                        let mut s: String = item.preview().chars().take(18).collect();
                        s = s.replace('\n', " ");
                        format!("移动：{}", s)
                    }
                    None => "移动记录".to_string(),
                }
            };
            item_ui::paint_drag_ghost(ctx, pos, &label);
            ctx.set_cursor_icon(egui::CursorIcon::Grabbing);

            if outside {
                // 窗口外提示：边框变红
                let painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Foreground,
                    egui::Id::new("cliprail_delete_hint"),
                ));
                painter.rect_stroke(
                    ctx.screen_rect().shrink(1.0),
                    Rounding::same(12.0),
                    Stroke::new(2.0, theme::RED),
                );
            } else if let Some(target) = insert_indicator(&rows, pos) {
                let painter = ctx.layer_painter(egui::LayerId::new(
                    egui::Order::Foreground,
                    egui::Id::new("cliprail_drop_line"),
                ));
                painter.rect_filled(
                    Rect::from_min_size(
                        egui::pos2(target.left(), target.top() - 1.5),
                        Vec2::new(target.width(), 3.0),
                    ),
                    Rounding::same(2.0),
                    theme::ACCENT,
                );
            }
        }
    }

    app.rows = rows;
    app.visible_ids = visible_now;
    for action in actions {
        app.apply(ctx, action);
    }
}

/// 计算插入位置指示线的矩形
fn insert_indicator(rows: &[(String, Rect)], pointer: egui::Pos2) -> Option<Rect> {
    for (_, rect) in rows {
        if pointer.y < rect.center().y {
            return Some(*rect);
        }
    }
    rows.last().map(|(_, rect)| {
        Rect::from_min_size(
            egui::pos2(rect.left(), rect.bottom() + 6.0),
            Vec2::new(rect.width(), 3.0),
        )
    })
}

fn banner(ui: &mut egui::Ui, message: &str) {
    egui::Frame::none()
        .fill(theme::RED_SOFT)
        .stroke(Stroke::new(1.0, theme::RED))
        .rounding(Rounding::same(theme::RADIUS_CTRL))
        .inner_margin(egui::Margin::symmetric(10.0, 8.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(
                egui::RichText::new(message)
                    .size(11.5)
                    .color(Color32::from_rgb(0x9E, 0x33, 0x28)),
            );
        });
}

/// 边缘收纳状态下的窄条触发区
pub fn show_collapsed(ctx: &egui::Context) {
    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(theme::ACCENT))
        .show(ctx, |ui| {
            let rect = ui.max_rect();
            ui.painter()
                .rect_filled(rect, Rounding::same(3.0), theme::ACCENT);
        });
}
