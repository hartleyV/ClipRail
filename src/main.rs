#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

use std::{
    borrow::Cow,
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    thread,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use arboard::{Clipboard, ImageData};
use chrono::{DateTime, Local, Utc};
use crossbeam_channel::{unbounded, Receiver, Sender};
use eframe::egui::{self, Color32, Context, Id, Pos2, Rect, RichText, Sense, Stroke, TextureHandle, Vec2};
use global_hotkey::{
    hotkey::{Code, HotKey, Modifiers},
    GlobalHotKeyEvent, GlobalHotKeyManager,
};
use image::{ImageBuffer, Rgba};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

#[derive(Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
enum ClipKind {
    Text,
    Image,
}

#[derive(Clone, Serialize, Deserialize)]
struct ClipItem {
    id: String,
    kind: ClipKind,
    #[serde(default)]
    text: String,
    #[serde(default)]
    image_path: String,
    hash: String,
    created: i64,
    #[serde(default)]
    pinned: bool,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
struct Settings {
    hotkey: String,
    edge_hide: bool,
    panel_pinned: bool,
    width: f32,
    x: f32,
    y: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: "alt+shift+v".into(),
            edge_hide: true,
            panel_pinned: false,
            width: 390.0,
            x: 1200.0,
            y: 20.0,
        }
    }
}

struct Store {
    root: PathBuf,
    items: Vec<ClipItem>,
    settings: Settings,
}

impl Store {
    fn load() -> Self {
        let exe_dir = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from("."));
        let root = exe_dir.join("data");
        let _ = fs::create_dir_all(root.join("images"));
        let _ = fs::create_dir_all(root.join("archives"));
        let settings = read_json(root.join("settings.json")).unwrap_or_default();
        let mut items: Vec<ClipItem> = read_json(root.join("clips.json")).unwrap_or_default();

        let mut seen = HashSet::new();
        items.retain(|item| {
            let valid = item.kind == ClipKind::Text || Path::new(&item.image_path).exists();
            valid && seen.insert(item.hash.clone())
        });
        items.sort_by_key(|item| !item.pinned);
        Self { root, items, settings }
    }

    fn save(&self) {
        write_json(self.root.join("clips.json"), &self.items);
        write_json(self.root.join("settings.json"), &self.settings);
        let mut by_day: HashMap<String, Vec<&ClipItem>> = HashMap::new();
        for item in &self.items {
            by_day.entry(day_label(item.created)).or_default().push(item);
        }
        for (day, items) in by_day {
            write_json(self.root.join("archives").join(format!("{day}.json")), &items);
        }
    }

    fn contains_hash(&self, hash: &str) -> bool {
        self.items.iter().any(|item| item.hash == hash)
    }

    fn add(&mut self, item: ClipItem) {
        if self.contains_hash(&item.hash) { return; }
        let index = self.items.iter().take_while(|x| x.pinned).count();
        self.items.insert(index, item);
        self.save();
    }

    fn toggle_pin(&mut self, id: &str) {
        if let Some(item) = self.items.iter_mut().find(|x| x.id == id) {
            item.pinned = !item.pinned;
        }
        self.items.sort_by_key(|item| !item.pinned);
        self.save();
    }

    fn delete_ids(&mut self, ids: &HashSet<String>) {
        for item in self.items.iter().filter(|x| ids.contains(&x.id)) {
            if item.kind == ClipKind::Image { let _ = fs::remove_file(&item.image_path); }
        }
        self.items.retain(|x| !ids.contains(&x.id));
        self.save();
    }

    fn reorder(&mut self, id: &str, before: Option<&str>) {
        let Some(index) = self.items.iter().position(|x| x.id == id) else { return; };
        let item = self.items.remove(index);
        let target = before.and_then(|b| self.items.iter().position(|x| x.id == b)).unwrap_or(self.items.len());
        self.items.insert(target, item);
        self.items.sort_by_key(|item| !item.pinned);
        self.save();
    }
}

fn read_json<T: for<'de> Deserialize<'de>>(path: PathBuf) -> Option<T> {
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

fn write_json<T: Serialize + ?Sized>(path: PathBuf, value: &T) {
    if let Ok(bytes) = serde_json::to_vec(value) {
        let tmp = path.with_extension("tmp");
        if fs::write(&tmp, bytes).is_ok() { let _ = fs::rename(tmp, path); }
    }
}

fn now_ts() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64
}

fn day_label(ts: i64) -> String {
    DateTime::<Utc>::from_timestamp(ts, 0)
        .map(|x| x.with_timezone(&Local).format("%Y-%m-%d").to_string())
        .unwrap_or_else(|| "unknown".into())
}

fn time_label(ts: i64) -> String {
    DateTime::<Utc>::from_timestamp(ts, 0)
        .map(|x| x.with_timezone(&Local).format("%m-%d %H:%M").to_string())
        .unwrap_or_default()
}

fn hash_bytes(bytes: &[u8]) -> String { format!("{:x}", Sha256::digest(bytes)) }

#[derive(Clone)]
enum ClipboardEvent {
    Text(String),
    Image { width: usize, height: usize, rgba: Vec<u8> },
}

enum ClipboardCommand {
    SetText(String),
    SetImage { width: usize, height: usize, rgba: Vec<u8> },
}

fn spawn_clipboard_worker() -> (Receiver<ClipboardEvent>, Sender<ClipboardCommand>) {
    let (event_tx, event_rx) = unbounded();
    let (cmd_tx, cmd_rx) = unbounded();
    thread::spawn(move || {
        let Ok(mut clipboard) = Clipboard::new() else { return; };
        let mut last_seen = String::new();
        loop {
            while let Ok(cmd) = cmd_rx.try_recv() {
                match cmd {
                    ClipboardCommand::SetText(text) => { let _ = clipboard.set_text(text); }
                    ClipboardCommand::SetImage { width, height, rgba } => {
                        let _ = clipboard.set_image(ImageData { width, height, bytes: Cow::Owned(rgba) });
                    }
                }
            }
            if let Ok(image) = clipboard.get_image() {
                let hash = hash_bytes(&image.bytes);
                if hash != last_seen {
                    last_seen = hash;
                    let _ = event_tx.send(ClipboardEvent::Image {
                        width: image.width,
                        height: image.height,
                        rgba: image.bytes.into_owned(),
                    });
                }
            } else if let Ok(text) = clipboard.get_text() {
                let hash = hash_bytes(text.as_bytes());
                if !text.trim().is_empty() && hash != last_seen {
                    last_seen = hash;
                    let _ = event_tx.send(ClipboardEvent::Text(text));
                }
            }
            thread::sleep(Duration::from_millis(320));
        }
    });
    (event_rx, cmd_tx)
}

struct ClipRailApp {
    store: Store,
    clipboard_rx: Receiver<ClipboardEvent>,
    clipboard_tx: Sender<ClipboardCommand>,
    hotkey_rx: Receiver<u32>,
    hotkey_manager: Option<GlobalHotKeyManager>,
    hotkey: Option<HotKey>,
    textures: HashMap<String, TextureHandle>,
    selected: HashSet<String>,
    copied_until: HashMap<String, Instant>,
    current_day: String,
    settings_open: bool,
    hotkey_edit: String,
    edge_hide_edit: bool,
    dragged: Option<String>,
    card_rects: Vec<(String, Rect)>,
    last_pointer_inside: Instant,
    collapsed: bool,
    visible: bool,
}

impl ClipRailApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        configure_style(&cc.egui_ctx);
        let store = Store::load();
        let (clipboard_rx, clipboard_tx) = spawn_clipboard_worker();
        let (hotkey_tx, hotkey_rx) = unbounded();
        let ctx = cc.egui_ctx.clone();
        thread::spawn(move || loop {
            if let Ok(event) = GlobalHotKeyEvent::receiver().recv() {
                let _ = hotkey_tx.send(event.id);
                ctx.request_repaint();
            }
        });
        let mut app = Self {
            hotkey_edit: store.settings.hotkey.clone(),
            edge_hide_edit: store.settings.edge_hide,
            store,
            clipboard_rx,
            clipboard_tx,
            hotkey_rx,
            hotkey_manager: GlobalHotKeyManager::new().ok(),
            hotkey: None,
            textures: HashMap::new(),
            selected: HashSet::new(),
            copied_until: HashMap::new(),
            current_day: "all".into(),
            settings_open: false,
            dragged: None,
            card_rects: vec![],
            last_pointer_inside: Instant::now(),
            collapsed: false,
            visible: true,
        };
        app.register_hotkey();
        app
    }

    fn register_hotkey(&mut self) {
        if let (Some(manager), Some(old)) = (&self.hotkey_manager, &self.hotkey) { let _ = manager.unregister(*old); }
        self.hotkey = parse_hotkey(&self.store.settings.hotkey);
        if let (Some(manager), Some(hotkey)) = (&self.hotkey_manager, self.hotkey) { let _ = manager.register(hotkey); }
    }

    fn capture_events(&mut self) {
        while let Ok(event) = self.clipboard_rx.try_recv() {
            match event {
                ClipboardEvent::Text(text) => {
                    let hash = hash_bytes(text.as_bytes());
                    if !self.store.contains_hash(&hash) {
                        self.store.add(ClipItem { id: format!("{}-{hash}", now_ts()), kind: ClipKind::Text, text, image_path: String::new(), hash, created: now_ts(), pinned: false });
                    }
                }
                ClipboardEvent::Image { width, height, rgba } => {
                    let hash = hash_bytes(&rgba);
                    if self.store.contains_hash(&hash) { continue; }
                    let path = self.store.root.join("images").join(format!("{}-{}.png", now_ts(), &hash[..10]));
                    if let Some(image) = ImageBuffer::<Rgba<u8>, _>::from_raw(width as u32, height as u32, rgba) {
                        if image.save(&path).is_ok() {
                            self.store.add(ClipItem { id: format!("{}-{hash}", now_ts()), kind: ClipKind::Image, text: String::new(), image_path: path.to_string_lossy().into(), hash, created: now_ts(), pinned: false });
                        }
                    }
                }
            }
        }
    }

    fn handle_hotkey(&mut self, ctx: &Context) {
        while let Ok(id) = self.hotkey_rx.try_recv() {
            if self.hotkey.map(|x| x.id()) == Some(id) {
                self.visible = !self.visible;
                ctx.send_viewport_cmd(egui::ViewportCommand::Visible(self.visible));
                if self.visible { ctx.send_viewport_cmd(egui::ViewportCommand::Focus); }
            }
        }
    }

    fn texture_for(&mut self, ctx: &Context, item: &ClipItem) -> Option<TextureHandle> {
        if let Some(texture) = self.textures.get(&item.image_path) { return Some(texture.clone()); }
        let image = image::open(&item.image_path).ok()?.to_rgba8();
        let size = [image.width() as usize, image.height() as usize];
        let color = egui::ColorImage::from_rgba_unmultiplied(size, image.as_raw());
        let texture = ctx.load_texture(&item.image_path, color, egui::TextureOptions::LINEAR);
        self.textures.insert(item.image_path.clone(), texture.clone());
        Some(texture)
    }

    fn copy_item(&mut self, item: &ClipItem) {
        match item.kind {
            ClipKind::Text => { let _ = self.clipboard_tx.send(ClipboardCommand::SetText(item.text.clone())); }
            ClipKind::Image => {
                if let Ok(image) = image::open(&item.image_path) {
                    let rgba = image.to_rgba8();
                    let _ = self.clipboard_tx.send(ClipboardCommand::SetImage { width: rgba.width() as usize, height: rgba.height() as usize, rgba: rgba.into_raw() });
                }
            }
        }
        self.copied_until.insert(item.id.clone(), Instant::now() + Duration::from_millis(1000));
    }

    fn draw_card(&mut self, ui: &mut egui::Ui, ctx: &Context, item: &ClipItem) {
        let is_dragged = self.dragged.as_deref() == Some(&item.id);
        let t = ctx.animate_bool(Id::new(("drag", &item.id)), is_dragged);
        let fill = if item.pinned { Color32::from_rgb(237, 246, 253) } else { Color32::WHITE };
        let border = if item.pinned { Color32::from_rgb(116, 177, 224) } else { Color32::from_rgb(216, 216, 212) };
        let fill = Color32::from_rgba_unmultiplied(fill.r(), fill.g(), fill.b(), (255.0 - 105.0 * t) as u8);
        let frame = egui::Frame::none()
            .fill(fill)
            .stroke(Stroke::new(1.5_f32, border))
            .rounding(egui::Rounding::same(11.0))
            .inner_margin(egui::Margin::same(12.0));
        let mut suppress_copy = false;
        let inner = frame.show(ui, |ui| {
            ui.horizontal(|ui| {
                let mut checked = self.selected.contains(&item.id);
                if ui.checkbox(&mut checked, "").changed() {
                    suppress_copy = true;
                    if checked { self.selected.insert(item.id.clone()); } else { self.selected.remove(&item.id); }
                }
                if self.copied_until.get(&item.id).is_some_and(|x| *x > Instant::now()) {
                    egui::Frame::none().fill(Color32::from_rgb(47, 143, 97)).rounding(6.0).inner_margin(egui::Margin::symmetric(8.0, 4.0)).show(ui, |ui| {
                        ui.label(RichText::new("✓ 已复制").color(Color32::WHITE).strong().size(12.0));
                    });
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let label = if item.pinned { "已置顶" } else { "置顶" };
                    if ui.button(label).clicked() { suppress_copy = true; self.store.toggle_pin(&item.id); }
                });
            });
            ui.add_space(6.0);
            match item.kind {
                ClipKind::Text => { ui.label(RichText::new(&item.text).size(14.0).color(Color32::from_rgb(41, 41, 39))); }
                ClipKind::Image => {
                    if let Some(texture) = self.texture_for(ctx, item) {
                        let available = ui.available_width();
                        let aspect = texture.size()[0] as f32 / texture.size()[1].max(1) as f32;
                        let size = Vec2::new(available, (available / aspect).min(230.0));
                        ui.add(egui::Image::new((texture.id(), size)));
                    }
                }
            }
            ui.add_space(5.0);
            ui.label(RichText::new(time_label(item.created)).size(11.0).color(Color32::from_rgb(119, 117, 112)));
        });
        let response = inner.response.interact(Sense::click_and_drag());
        self.card_rects.push((item.id.clone(), response.rect));
        if response.clicked() && !suppress_copy { self.copy_item(item); }
        if response.drag_started() { self.dragged = Some(item.id.clone()); }
        if response.drag_stopped() {
            let pointer = ctx.input(|i| i.pointer.latest_pos());
            let id = item.id.clone();
            self.dragged = None;
            match pointer {
                None => { let ids = HashSet::from([id]); self.store.delete_ids(&ids); }
                Some(pos) => {
                    let before = self.card_rects.iter().find(|(other, rect)| other != &id && pos.y < rect.center().y).map(|x| x.0.clone());
                    self.store.reorder(&id, before.as_deref());
                }
            }
        }
        ui.add_space(12.0);
    }

    fn edge_behavior(&mut self, ctx: &Context) {
        let hover = ctx.input(|i| i.pointer.hover_pos());
        if hover.is_some() {
            self.last_pointer_inside = Instant::now();
            if self.collapsed {
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(Pos2::new(self.store.settings.x, self.store.settings.y)));
                self.collapsed = false;
            }
        } else if !self.store.settings.panel_pinned && self.store.settings.edge_hide && self.last_pointer_inside.elapsed() > Duration::from_millis(600) && !self.collapsed {
            if let Some(rect) = ctx.input(|i| i.viewport().outer_rect) {
                self.store.settings.x = rect.min.x;
                self.store.settings.y = rect.min.y;
                self.store.save();
                let monitor = ctx.input(|i| i.viewport().monitor_size).unwrap_or(Vec2::new(1920.0, 1080.0));
                let to_left = rect.center().x < monitor.x / 2.0;
                let x = if to_left { -rect.width() + 6.0 } else { monitor.x - 6.0 };
                ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(Pos2::new(x, rect.min.y)));
                self.collapsed = true;
            }
        }
    }
}

impl eframe::App for ClipRailApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(Duration::from_millis(120));
        self.capture_events();
        self.handle_hotkey(ctx);
        self.edge_behavior(ctx);
        self.copied_until.retain(|_, until| *until > Instant::now());
        self.card_rects.clear();

        egui::CentralPanel::default().frame(egui::Frame::none().fill(Color32::from_rgb(247, 247, 245)).inner_margin(12.0)).show(ctx, |ui| {
            let header = ui.horizontal(|ui| {
                ui.label(RichText::new("ClipRail").size(18.0).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(if self.store.settings.panel_pinned { "已置顶" } else { "自动隐藏" }).clicked() {
                        self.store.settings.panel_pinned = !self.store.settings.panel_pinned;
                        self.store.save();
                        let level = if self.store.settings.panel_pinned { egui::WindowLevel::AlwaysOnTop } else { egui::WindowLevel::Normal };
                        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(level));
                    }
                    if ui.button("设置").clicked() {
                        self.hotkey_edit = self.store.settings.hotkey.clone();
                        self.edge_hide_edit = self.store.settings.edge_hide;
                        self.settings_open = true;
                    }
                });
            }).response.interact(Sense::drag());
            if header.drag_started() { ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag); }

            ui.add_space(8.0);
            ui.horizontal(|ui| {
                egui::ComboBox::from_id_salt("day_filter")
                    .selected_text(if self.current_day == "all" { "全部记录" } else { &self.current_day })
                    .width(ui.available_width() - 125.0)
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.current_day, "all".into(), "全部记录");
                        let mut days: Vec<_> = self.store.items.iter().map(|x| day_label(x.created)).collect();
                        days.sort(); days.dedup(); days.reverse();
                        for day in days { ui.selectable_value(&mut self.current_day, day.clone(), day); }
                    });
                if ui.button("全选").clicked() {
                    let visible: HashSet<_> = self.store.items.iter().filter(|x| self.current_day == "all" || day_label(x.created) == self.current_day).map(|x| x.id.clone()).collect();
                    if visible.iter().all(|id| self.selected.contains(id)) { for id in visible { self.selected.remove(&id); } } else { self.selected.extend(visible); }
                }
                if ui.add_enabled(!self.selected.is_empty(), egui::Button::new(format!("删除 {}", self.selected.len()))).clicked() {
                    self.store.delete_ids(&self.selected); self.selected.clear();
                }
            });

            let resize_rect = Rect::from_min_max(ui.min_rect().left_top(), Pos2::new(ui.min_rect().left() + 6.0, ui.max_rect().bottom()));
            if ui.interact(resize_rect, Id::new("resize_left"), Sense::drag()).drag_started() {
                ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(egui::ResizeDirection::West));
            }

            ui.add_space(10.0);
            egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                let visible: Vec<_> = self.store.items.iter().filter(|x| self.current_day == "all" || day_label(x.created) == self.current_day).cloned().collect();
                for item in visible { self.draw_card(ui, ctx, &item); }
            });
        });

        if self.settings_open {
            egui::Window::new("偏好设置")
                .collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
                .show(ctx, |ui| {
                    ui.label("显示 / 隐藏快捷键");
                    ui.text_edit_singleline(&mut self.hotkey_edit);
                    ui.add_space(8.0);
                    ui.checkbox(&mut self.edge_hide_edit, "离开竖栏后收缩到屏幕边缘");
                    ui.add_space(12.0);
                    ui.horizontal(|ui| {
                        if ui.button("取消").clicked() { self.settings_open = false; }
                        if ui.button("保存").clicked() {
                            self.store.settings.hotkey = self.hotkey_edit.trim().to_lowercase();
                            self.store.settings.edge_hide = self.edge_hide_edit;
                            self.store.save(); self.register_hotkey(); self.settings_open = false;
                        }
                    });
                });
        }
    }
}

fn parse_hotkey(value: &str) -> Option<HotKey> {
    let parts: Vec<_> = value.to_lowercase().split('+').map(str::trim).map(str::to_string).collect();
    let mut modifiers = Modifiers::empty();
    let mut code = None;
    for part in parts {
        match part.as_str() {
            "alt" => modifiers |= Modifiers::ALT,
            "shift" => modifiers |= Modifiers::SHIFT,
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "super" | "win" | "meta" => modifiers |= Modifiers::SUPER,
            "a" => code = Some(Code::KeyA), "b" => code = Some(Code::KeyB), "c" => code = Some(Code::KeyC),
            "d" => code = Some(Code::KeyD), "e" => code = Some(Code::KeyE), "f" => code = Some(Code::KeyF),
            "g" => code = Some(Code::KeyG), "h" => code = Some(Code::KeyH), "i" => code = Some(Code::KeyI),
            "j" => code = Some(Code::KeyJ), "k" => code = Some(Code::KeyK), "l" => code = Some(Code::KeyL),
            "m" => code = Some(Code::KeyM), "n" => code = Some(Code::KeyN), "o" => code = Some(Code::KeyO),
            "p" => code = Some(Code::KeyP), "q" => code = Some(Code::KeyQ), "r" => code = Some(Code::KeyR),
            "s" => code = Some(Code::KeyS), "t" => code = Some(Code::KeyT), "u" => code = Some(Code::KeyU),
            "v" => code = Some(Code::KeyV), "w" => code = Some(Code::KeyW), "x" => code = Some(Code::KeyX),
            "y" => code = Some(Code::KeyY), "z" => code = Some(Code::KeyZ),
            "f1" => code = Some(Code::F1), "f2" => code = Some(Code::F2), "f3" => code = Some(Code::F3),
            "f4" => code = Some(Code::F4), "f5" => code = Some(Code::F5), "f6" => code = Some(Code::F6),
            "f7" => code = Some(Code::F7), "f8" => code = Some(Code::F8), "f9" => code = Some(Code::F9),
            "f10" => code = Some(Code::F10), "f11" => code = Some(Code::F11), "f12" => code = Some(Code::F12),
            _ => {}
        }
    }
    code.map(|code| HotKey::new(Some(modifiers), code))
}

fn configure_style(ctx: &Context) {
    let mut style = (*ctx.style()).clone();
    style.spacing.item_spacing = Vec2::new(8.0, 8.0);
    style.visuals.widgets.inactive.rounding = egui::Rounding::same(7.0);
    style.visuals.widgets.hovered.rounding = egui::Rounding::same(7.0);
    style.visuals.widgets.active.rounding = egui::Rounding::same(7.0);
    style.visuals.selection.bg_fill = Color32::from_rgb(39, 131, 222);
    ctx.set_style(style);
}

fn main() -> eframe::Result<()> {
    let store = Store::load();
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("ClipRail")
        .with_inner_size([store.settings.width, 780.0])
        .with_position([store.settings.x, store.settings.y])
        .with_decorations(false)
        .with_resizable(true);
    if store.settings.panel_pinned { viewport = viewport.with_always_on_top(); }
    let options = eframe::NativeOptions { viewport, ..Default::default() };
    eframe::run_native("ClipRail", options, Box::new(|cc| Ok(Box::new(ClipRailApp::new(cc)))))
}
