use crate::{clipboard::{self, ClipWrite}, hotkey, model::{ClipItem, ClipKind, ClipboardEvent, Settings}, store::Store};
use crossbeam_channel::{unbounded, Receiver, Sender};
use eframe::egui::{self, Color32, CornerRadius, FontId, Frame, Margin, RichText, Sense, Stroke, Vec2};
use std::{collections::{BTreeMap, HashSet}, time::{Duration, Instant}};

const BLUE: Color32 = Color32::from_rgb(39, 131, 222);
const TEXT: Color32 = Color32::from_rgb(44, 44, 43);
const MUTED: Color32 = Color32::from_rgb(112, 109, 104);
const BORDER: Color32 = Color32::from_rgb(230, 229, 227);
const SOFT: Color32 = Color32::from_rgb(249, 248, 247);

pub struct ClipRailApp {
    store: Store,
    clips: Vec<ClipItem>,
    settings: Settings,
    events: Receiver<ClipboardEvent>,
    write_tx: Sender<ClipWrite>,
    selected: HashSet<String>,
    date_filter: String,
    copied: Option<(String, Instant)>,
    show_settings: bool,
    draft_hotkey: String,
    draft_edge_hide: bool,
    settings_error: Option<String>,
    status: Option<String>,
}

impl ClipRailApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_style(&cc.egui_ctx);
        let store = Store::portable(); let _ = store.ensure();
        let settings = store.load_settings();
        let (tx, events) = unbounded();
        let (write_tx, write_rx) = unbounded();
        clipboard::spawn(tx.clone(), write_rx);
        hotkey::spawn(settings.hotkey.clone(), tx);
        Self { clips: store.load_clips(), store, draft_hotkey: settings.hotkey.clone(), draft_edge_hide: settings.edge_hide, settings, events, write_tx, selected: HashSet::new(), date_filter: "全部记录".into(), copied: None, show_settings: false, settings_error: None, status: None }
    }

    fn handle_events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.events.try_recv() {
            match event {
                ClipboardEvent::ToggleWindow => ctx.send_viewport_cmd(egui::ViewportCommand::Visible(!ctx.input(|i| i.viewport().focused.unwrap_or(true)))),
                ClipboardEvent::NewText { text, hash, created } => {
                    if !self.clips.iter().any(|c| c.hash == hash) {
                        self.clips.insert(0, ClipItem { id: format!("{created}-{}", &hash[..8]), kind: ClipKind::Text, text, image_path: Default::default(), hash, created, pinned: false });
                        self.persist();
                    }
                }
                ClipboardEvent::NewImage { rgba, width, height, hash, created } => {
                    if !self.clips.iter().any(|c| c.hash == hash) {
                        match self.store.save_image(&rgba, width, height, created, &hash) {
                            Ok(path) => { self.clips.insert(0, ClipItem { id: format!("{created}-{}", &hash[..8]), kind: ClipKind::Image, text: String::new(), image_path: path, hash, created, pinned: false }); self.persist(); }
                            Err(e) => self.status = Some(format!("图片保存失败：{e}")),
                        }
                    }
                }
            }
        }
        if self.copied.as_ref().is_some_and(|(_, t)| t.elapsed() > Duration::from_secs(1)) { self.copied = None; }
    }

    fn persist(&mut self) { if let Err(e) = self.store.save_clips(&self.clips) { self.status = Some(format!("保存失败：{e}")); } }
    fn ordered_indices(&self) -> Vec<usize> {
        let mut out: Vec<_> = self.clips.iter().enumerate().filter(|(_, c)| self.matches_date(c)).map(|(i, _)| i).collect();
        out.sort_by_key(|&i| (!self.clips[i].pinned, i)); out
    }
    fn matches_date(&self, clip: &ClipItem) -> bool {
        self.date_filter == "全部记录" || (self.date_filter == "今天" && clip.date_key() == chrono::Local::now().format("%Y-%m-%d").to_string()) || self.date_filter == clip.date_key()
    }
    fn copy_clip(&mut self, index: usize) {
        let clip = &self.clips[index];
        let sent = match clip.kind {
            ClipKind::Text => self.write_tx.send(ClipWrite::Text(clip.text.clone())).is_ok(),
            ClipKind::Image => match image::open(&clip.image_path) {
                Ok(image) => {
                    let image = image.to_rgba8();
                    self.write_tx.send(ClipWrite::Image {
                        width: image.width() as usize,
                        height: image.height() as usize,
                        rgba: image.into_raw(),
                    }).is_ok()
                }
                Err(_) => false,
            },
        };
        if sent { self.copied = Some((clip.id.clone(), Instant::now())); } else { self.status = Some("无法写入系统剪贴板".into()); }
    }
    fn delete_selected(&mut self) {
        let ids = self.selected.clone();
        self.clips.retain(|c| { if ids.contains(&c.id) { self.store.remove_asset(c); false } else { true } });
        self.selected.clear(); self.persist();
    }
}

impl eframe::App for ClipRailApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_events(ctx);
        ctx.request_repaint_after(Duration::from_millis(120));
        egui::CentralPanel::default().frame(Frame::new().fill(SOFT).inner_margin(Margin::symmetric(14, 12))).show(ctx, |ui| {
            self.header(ui, ctx);
            ui.add_space(10.0);
            self.toolbar(ui);
            ui.add_space(10.0);
            let indices = self.ordered_indices();
            if indices.is_empty() { empty_state(ui); }
            else { egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| { for index in indices { self.clip_card(ui, index); ui.add_space(10.0); } }); }
        });
        if self.show_settings { self.settings_modal(ctx); }
    }
}

impl ClipRailApp {
    fn header(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let r = ui.horizontal(|ui| {
            ui.label(RichText::new("ClipRail").font(FontId::proportional(20.0)).strong().color(TEXT));
            ui.label(RichText::new("本地剪贴板").size(13.0).color(MUTED));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(icon_button("⚙", "设置")).clicked() { self.draft_hotkey = self.settings.hotkey.clone(); self.draft_edge_hide = self.settings.edge_hide; self.settings_error = None; self.show_settings = true; }
                let label = if self.settings.panel_pinned { "📌" } else { "◒" };
                if ui.add(icon_button(label, if self.settings.panel_pinned { "取消置顶" } else { "自动隐藏" })).clicked() { self.settings.panel_pinned = !self.settings.panel_pinned; let _ = self.store.save_settings(&self.settings); }
            });
        }).response;
        if r.dragged() { ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag); }
    }
    fn toolbar(&mut self, ui: &mut egui::Ui) {
        let mut dates = BTreeMap::<String, usize>::new(); for c in &self.clips { *dates.entry(c.date_key()).or_default() += 1; }
        Frame::new().fill(Color32::WHITE).stroke(Stroke::new(1.0, BORDER)).corner_radius(8.0).inner_margin(Margin::symmetric(10, 7)).show(ui, |ui| {
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("date_filter").selected_text(format!("{} ({})", self.date_filter, self.ordered_indices().len())).width(132.0).show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.date_filter, "全部记录".into(), format!("全部记录 ({})", self.clips.len()));
                    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
                    ui.selectable_value(&mut self.date_filter, "今天".into(), format!("今天 ({})", dates.get(&today).copied().unwrap_or(0)));
                    for (date, count) in dates.iter().rev() { ui.selectable_value(&mut self.date_filter, date.clone(), format!("{date} ({count})")); }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let n = self.selected.len();
                    if n > 0 && ui.add(egui::Button::new(RichText::new(format!("删除 {n}")).color(Color32::WHITE).strong()).fill(Color32::from_rgb(229, 100, 88)).stroke(Stroke::NONE).corner_radius(6.0)).clicked() { self.delete_selected(); }
                    if ui.link(if n == self.clips.len() && n > 0 { "取消全选" } else { "全选" }).clicked() { if n == self.clips.len() { self.selected.clear(); } else { self.selected = self.clips.iter().filter(|c| self.matches_date(c)).map(|c| c.id.clone()).collect(); } }
                });
            });
        });
    }
    fn clip_card(&mut self, ui: &mut egui::Ui, index: usize) {
        let pinned = self.clips[index].pinned; let id = self.clips[index].id.clone();
        let copied = self.copied.as_ref().is_some_and(|(x, _)| x == &id);
        let fill = if pinned { Color32::from_rgb(237, 247, 255) } else { Color32::WHITE };
        let stroke = if pinned { Stroke::new(1.2, BLUE) } else { Stroke::new(1.0, BORDER) };
        let inner = Frame::new().fill(fill).stroke(stroke).corner_radius(10.0).inner_margin(Margin::same(12)).show(ui, |ui| {
            ui.horizontal(|ui| {
                let mut checked = self.selected.contains(&id);
                if ui.checkbox(&mut checked, "").changed() { if checked { self.selected.insert(id.clone()); } else { self.selected.remove(&id); } }
                if copied { ui.label(RichText::new("✓ 已复制").size(12.5).strong().color(Color32::WHITE).background_color(Color32::from_rgb(70, 161, 113))); }
                else { ui.label(RichText::new(if pinned { "已置顶" } else { time_label(self.clips[index].created).as_str() }).size(12.5).color(if pinned { BLUE } else { MUTED })); }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button(if pinned { "取消置顶" } else { "置顶" }).clicked() { self.clips[index].pinned = !pinned; self.persist(); }
                });
            });
            ui.add_space(8.0);
            match self.clips[index].kind {
                ClipKind::Text => { let mut text = self.clips[index].text.clone(); ui.add(egui::TextEdit::multiline(&mut text).desired_width(f32::INFINITY).desired_rows(text.lines().count().clamp(2, 8)).interactive(false).frame(false).text_color(TEXT)); }
                ClipKind::Image => {
                    if let Ok(img) = image::open(&self.clips[index].image_path) {
                        let rgba = img.to_rgba8(); let size = [rgba.width() as usize, rgba.height() as usize];
                        let tex = ui.ctx().load_texture(format!("clip-{id}"), egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw()), egui::TextureOptions::LINEAR);
                        let available = ui.available_width(); let ratio = (available / size[0] as f32).min(1.0); ui.add(egui::Image::new(&tex).fit_to_exact_size(Vec2::new(size[0] as f32 * ratio, size[1] as f32 * ratio).min(Vec2::new(available, 280.0))).corner_radius(6.0));
                    } else { ui.label(RichText::new("图片���件不可用").color(Color32::from_rgb(229, 100, 88))); }
                }
            }
            ui.add_space(6.0); ui.label(RichText::new("单击复制  ·  拖动调整顺序").size(12.0).color(MUTED));
        });
        if inner.response.interact(Sense::click_and_drag()).clicked() { self.copy_clip(index); }
    }
    fn settings_modal(&mut self, ctx: &egui::Context) {
        egui::Window::new("设置").collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).fixed_size([340.0, 240.0]).show(ctx, |ui| {
            ui.label(RichText::new("显示 / 隐藏快捷键").strong().color(TEXT)); ui.add_space(6.0);
            ui.add(egui::TextEdit::singleline(&mut self.draft_hotkey).hint_text("例如 alt+shift+v").desired_width(f32::INFINITY));
            ui.label(RichText::new("支持 Ctrl、Alt、Shift、Super + A–Z / F1–F12").size(12.0).color(MUTED));
            ui.add_space(16.0); ui.checkbox(&mut self.draft_edge_hide, "鼠标离开后收纳到屏幕边缘");
            if let Some(e) = &self.settings_error { ui.add_space(8.0); ui.label(RichText::new(e).color(Color32::from_rgb(229, 100, 88))); }
            ui.add_space(18.0); ui.horizontal(|ui| {
                if ui.button("取消").clicked() { self.show_settings = false; }
                if ui.add(egui::Button::new(RichText::new("保存").color(Color32::WHITE).strong()).fill(BLUE).stroke(Stroke::NONE).corner_radius(6.0).min_size([72.0, 36.0].into())).clicked() {
                    match hotkey::parse(&self.draft_hotkey) {
                        Ok(_) => { self.settings.hotkey = self.draft_hotkey.trim().to_lowercase(); self.settings.edge_hide = self.draft_edge_hide; if let Err(e) = self.store.save_settings(&self.settings) { self.settings_error = Some(format!("保存失败：{e}")); } else { self.show_settings = false; self.status = Some("设置已保存，快捷键将在下次启动后生效".into()); } }
                        Err(e) => self.settings_error = Some(e),
                    }
                }
            });
        });
    }
}

fn icon_button<'a>(text: &'a str, tip: &'a str) -> egui::Button<'a> { egui::Button::new(RichText::new(text).size(16.0)).frame(false).min_size([36.0, 36.0].into()).sense(Sense::click()).wrap().shortcut_text(tip) }
fn time_label(ts: i64) -> String { chrono::DateTime::from_timestamp(ts, 0).map(|d| d.with_timezone(&chrono::Local).format("%m月%d日  %H:%M").to_string()).unwrap_or_default() }
fn empty_state(ui: &mut egui::Ui) { ui.vertical_centered(|ui| { ui.add_space(80.0); ui.label(RichText::new("⌘").size(36.0).color(BLUE)); ui.add_space(8.0); ui.label(RichText::new("复制一点内容开始吧").size(17.0).strong().color(TEXT)); ui.label(RichText::new("文本和图片会自动、安全地保存在本机").size(13.0).color(MUTED)); }); }
fn configure_style(ctx: &egui::Context) { let mut style = (*ctx.style()).clone(); style.spacing.item_spacing = Vec2::new(8.0, 8.0); style.spacing.interact_size.y = 34.0; style.visuals = egui::Visuals::light(); style.visuals.panel_fill = SOFT; style.visuals.widgets.inactive.corner_radius = CornerRadius::same(6); style.visuals.widgets.hovered.corner_radius = CornerRadius::same(6); style.visuals.selection.bg_fill = BLUE; ctx.set_style(style); }
