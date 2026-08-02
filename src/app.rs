//! 应用状态与主循环。

use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender};
use eframe::egui::{self, Pos2};
use global_hotkey::GlobalHotKeyEvent;

use crate::archive;
use crate::clipboard::{ClipCommand, ClipEvent};
use crate::hotkey::HotkeyService;
use crate::model::{Item, Kind, Settings};
use crate::store;
use crate::ui::{settings as settings_ui, sidebar, theme};

const EDGE_STRIP: f32 = 7.0;
const HIDE_DELAY: Duration = Duration::from_millis(650);
const COPIED_DURATION: Duration = Duration::from_millis(1000);
const PERSIST_INTERVAL: Duration = Duration::from_millis(700);
const TEXTURES_PER_FRAME: usize = 2;

pub fn today_string() -> String {
    store::format_ts(store::now_ts(), "%Y-%m-%d")
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DateFilter {
    All,
    Today,
    Day(String),
}

pub struct DragState {
    pub id: String,
}

/// 拖动左边缘调宽时的基准：右边缘保持不动
/// 拖动过程中只记录 preview（不变动窗口），松开鼠标后一次性应用
pub struct ResizeState {
    pub right: f32,
    pub preview: f32,
}

pub enum Action {
    Copy(String),
    TogglePin(String),
    ToggleSelect(String),
    SelectAllVisible,
    ClearSelection,
    DeleteSelected,
    SetFilter(DateFilter),
    OpenSettings,
    TogglePanelPin,
    DragStart(String),
    DragStop(Option<Pos2>),
}

pub struct App {
    pub items: Vec<Item>,
    pub settings: Settings,

    clip_tx: Sender<ClipCommand>,
    clip_rx: Receiver<ClipEvent>,
    hotkeys: HotkeyService,
    hotkey_id: Option<u32>,
    last_hotkey: Instant,

    pub textures: HashMap<String, Option<egui::TextureHandle>>,
    pub visible_ids: HashSet<String>,
    pub selected: HashSet<String>,
    pub copied: Option<(String, Instant)>,
    pub filter: DateFilter,

    pub show_settings: bool,
    pub draft_hotkey: String,
    pub draft_edge_hide: bool,
    pub settings_error: Option<String>,
    pub hotkey_error: Option<String>,

    pub drag: Option<DragState>,
    pub rows: Vec<(String, egui::Rect)>,

    resize: Option<ResizeState>,
    geometry_hold: Instant,
    last_shown: Instant,

    last_active: Instant,
    collapsed: bool,
    hidden: bool,
    placed: bool,
    dirty: bool,
    last_persist: Instant,
    last_toggle_check: Instant,
}

impl App {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        settings: Settings,
        clip_tx: Sender<ClipCommand>,
        clip_rx: Receiver<ClipEvent>,
    ) -> Self {
        theme::install(&cc.egui_ctx);

        let mut hotkeys = HotkeyService::new();
        let mut hotkey_error = None;
        let hotkey_id = match hotkeys.register(&settings.hotkey) {
            Ok(id) => Some(id),
            Err(err) => {
                hotkey_error = Some(format!("{}（可在设置中更换，或使用 ClipRail --toggle）", err));
                None
            }
        };

        let items = store::load_items();
        archive::rebuild(&items);

        Self {
            draft_hotkey: settings.hotkey.clone(),
            draft_edge_hide: settings.edge_hide,
            items,
            settings,
            clip_tx,
            clip_rx,
            hotkeys,
            hotkey_id,
            last_hotkey: Instant::now() - Duration::from_secs(5),
            textures: HashMap::new(),
            visible_ids: HashSet::new(),
            selected: HashSet::new(),
            copied: None,
            filter: DateFilter::All,
            show_settings: false,
            settings_error: None,
            hotkey_error,
            drag: None,
            rows: Vec::new(),
            resize: None,
            geometry_hold: Instant::now(),
            last_shown: Instant::now(),
            last_active: Instant::now(),
            collapsed: false,
            hidden: false,
            placed: false,
            dirty: false,
            last_persist: Instant::now(),
            last_toggle_check: Instant::now(),
        }
    }

    // ------------------------------------------------------------ 查询辅助

    pub fn visible_indices(&self) -> Vec<usize> {
        let today = today_string();
        self.items
            .iter()
            .enumerate()
            .filter(|(_, item)| match &self.filter {
                DateFilter::All => true,
                DateFilter::Today => item.local_date() == today,
                DateFilter::Day(d) => &item.local_date() == d,
            })
            .map(|(i, _)| i)
            .collect()
    }

    pub fn filter_label(&self) -> String {
        match &self.filter {
            DateFilter::All => format!("全部记录（{}）", self.items.len()),
            DateFilter::Today => format!("今天（{}）", self.count_for_date(&today_string())),
            DateFilter::Day(d) => format!("{}（{}）", d, self.count_for_date(d)),
        }
    }

    pub fn count_for_date(&self, date: &str) -> usize {
        self.items
            .iter()
            .filter(|i| i.local_date() == date)
            .count()
    }

    /// 按日期倒序返回 (日期, 数量)
    pub fn date_options(&self) -> Vec<(String, usize)> {
        let mut map: BTreeMap<String, usize> = BTreeMap::new();
        for item in &self.items {
            *map.entry(item.local_date()).or_insert(0) += 1;
        }
        let mut out: Vec<(String, usize)> = map.into_iter().collect();
        out.reverse();
        out
    }

    // ------------------------------------------------------------ 动作处理

    pub fn apply(&mut self, ctx: &egui::Context, action: Action) {
        match action {
            Action::Copy(id) => self.copy_to_clipboard(&id),
            Action::TogglePin(id) => {
                if let Some(item) = self.items.iter_mut().find(|i| i.id == id) {
                    item.pinned = !item.pinned;
                }
                store::sort_pinned_first(&mut self.items);
                self.mark_dirty();
            }
            Action::ToggleSelect(id) => {
                if !self.selected.remove(&id) {
                    self.selected.insert(id);
                }
            }
            Action::SelectAllVisible => {
                for index in self.visible_indices() {
                    self.selected.insert(self.items[index].id.clone());
                }
            }
            Action::ClearSelection => self.selected.clear(),
            Action::DeleteSelected => {
                let ids: Vec<String> = self.selected.iter().cloned().collect();
                self.delete_ids(&ids);
            }
            Action::SetFilter(filter) => self.filter = filter,
            Action::OpenSettings => {
                self.show_settings = true;
                self.draft_hotkey = self.settings.hotkey.clone();
                self.draft_edge_hide = self.settings.edge_hide;
                self.settings_error = None;
            }
            Action::TogglePanelPin => {
                self.settings.panel_pinned = !self.settings.panel_pinned;
                self.apply_window_level(ctx);
                self.mark_dirty();
            }
            Action::DragStart(id) => self.drag = Some(DragState { id }),
            Action::DragStop(pointer) => {
                if let Some(drag) = self.drag.take() {
                    if let Some(pos) = pointer {
                        if !ctx.screen_rect().contains(pos) {
                            self.delete_ids(&[drag.id]);
                        } else {
                            self.reorder(&drag.id, pos);
                        }
                    }
                }
            }
        }
    }

    fn reorder(&mut self, id: &str, pointer: Pos2) {
        let target_id = self
            .rows
            .iter()
            .find(|(row_id, rect)| row_id != id && pointer.y < rect.center().y)
            .map(|(row_id, _)| row_id.clone());

        let from = match self.items.iter().position(|i| i.id == id) {
            Some(index) => index,
            None => return,
        };
        let item = self.items.remove(from);
        let to = match target_id {
            Some(tid) => self
                .items
                .iter()
                .position(|i| i.id == tid)
                .unwrap_or(self.items.len()),
            None => self.items.len(),
        };
        self.items.insert(to.min(self.items.len()), item);
        store::sort_pinned_first(&mut self.items);
        self.mark_dirty();
    }

    fn delete_ids(&mut self, ids: &[String]) {
        if ids.is_empty() {
            return;
        }
        let set: HashSet<&String> = ids.iter().collect();
        let mut removed_images: Vec<String> = Vec::new();
        self.items.retain(|item| {
            if set.contains(&item.id) {
                if item.kind == Kind::Image {
                    removed_images.push(item.image_path.clone());
                }
                false
            } else {
                true
            }
        });
        for path in removed_images {
            store::delete_image(&path);
        }
        for id in ids {
            self.selected.remove(id);
            self.textures.remove(id);
        }
        self.mark_dirty();
    }

    fn copy_to_clipboard(&mut self, id: &str) {
        let item = match self.items.iter().find(|i| i.id == id) {
            Some(item) => item.clone(),
            None => return,
        };
        match item.kind {
            Kind::Text => {
                let _ = self.clip_tx.send(ClipCommand::SetText(item.text.clone()));
            }
            Kind::Image => {
                if let Some((w, h, rgba)) = store::load_full_image(&item.image_path) {
                    let _ = self.clip_tx.send(ClipCommand::SetImage {
                        width: w,
                        height: h,
                        rgba,
                    });
                }
            }
        }
        self.copied = Some((item.id, Instant::now()));
    }

    fn add_clip(&mut self, event: ClipEvent) {
        match event {
            ClipEvent::Text { text, hash } => {
                if self.items.iter().any(|i| i.hash == hash) {
                    return;
                }
                let item = Item {
                    id: store::new_id(&hash),
                    kind: Kind::Text,
                    text,
                    image_path: String::new(),
                    hash,
                    created: store::now_ts(),
                    pinned: false,
                    width: 0,
                    height: 0,
                };
                self.insert_new(item);
            }
            ClipEvent::Image {
                width,
                height,
                rgba,
                hash,
            } => {
                if self.items.iter().any(|i| i.hash == hash) {
                    return;
                }
                let id = store::new_id(&hash);
                // 图片保存失败时跳过该条，不影响程序运行
                let path = match store::save_image(&id, width, height, &rgba) {
                    Some(path) => path,
                    None => return,
                };
                let item = Item {
                    id,
                    kind: Kind::Image,
                    text: String::new(),
                    image_path: path,
                    hash,
                    created: store::now_ts(),
                    pinned: false,
                    width,
                    height,
                };
                self.insert_new(item);
            }
        }
    }

    fn insert_new(&mut self, item: Item) {
        let position = self.items.iter().filter(|i| i.pinned).count();
        self.items.insert(position, item);
        self.mark_dirty();
    }

    fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    // ------------------------------------------------------------ 设置窗口

    pub fn save_settings_draft(&mut self) {
        let draft = self.draft_hotkey.trim().to_lowercase();
        match self.hotkeys.register(&draft) {
            Ok(id) => {
                self.hotkey_id = Some(id);
                self.hotkey_error = None;
                self.settings.hotkey = draft;
                self.settings.edge_hide = self.draft_edge_hide;
                self.settings_error = None;
                self.show_settings = false;
                store::save_settings(&self.settings);
            }
            Err(err) => {
                self.settings_error = Some(err);
            }
        }
    }

    pub fn close_settings(&mut self) {
        self.show_settings = false;
        self.settings_error = None;
    }

    // ------------------------------------------------------------ 纹理懒加载

    pub fn preload_textures(&mut self, ctx: &egui::Context) {
        let mut budget = TEXTURES_PER_FRAME;
        let candidates: Vec<(String, String)> = self
            .items
            .iter()
            .filter(|item| item.kind == Kind::Image && !self.textures.contains_key(&item.id))
            .filter(|item| self.visible_ids.is_empty() || self.visible_ids.contains(&item.id))
            .take(TEXTURES_PER_FRAME)
            .map(|item| (item.id.clone(), item.image_path.clone()))
            .collect();

        for (id, path) in candidates {
            if budget == 0 {
                break;
            }
            budget -= 1;
            let handle = store::load_thumbnail(&path, 720).map(|(w, h, rgba)| {
                let image = egui::ColorImage::from_rgba_unmultiplied([w as usize, h as usize], &rgba);
                ctx.load_texture(format!("clip_{}", id), image, egui::TextureOptions::LINEAR)
            });
            self.textures.insert(id, handle);
        }

        // 缓存上限，避免大量图片占用内存
        if self.textures.len() > 60 && !self.visible_ids.is_empty() {
            let visible = self.visible_ids.clone();
            self.textures.retain(|id, _| visible.contains(id));
        }
    }

    // ------------------------------------------------------------ 窗口行为

    fn apply_window_level(&self, ctx: &egui::Context) {
        let level = if self.settings.panel_pinned {
            egui::WindowLevel::AlwaysOnTop
        } else {
            egui::WindowLevel::Normal
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(level));
    }

    fn monitor_size(&self, ctx: &egui::Context) -> egui::Vec2 {
        ctx.input(|i| i.viewport().monitor_size)
            .unwrap_or(egui::vec2(1920.0, 1080.0))
    }

    fn place_initial(&mut self, ctx: &egui::Context) {
        let monitor = self.monitor_size(ctx);
        let width = self.settings.clamped_width();
        if self.settings.x < 0.0 {
            self.settings.x = (monitor.x - width - 16.0).max(0.0);
        }
        if self.settings.height <= 0.0 {
            self.settings.height = (monitor.y - 120.0).max(320.0);
        }
        self.settings.width = width;
        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
            self.settings.x,
            self.settings.y,
        )));
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
            width,
            self.settings.height,
        )));
        self.apply_window_level(ctx);
        self.placed = true;
    }

    pub fn toggle_panel(&mut self, ctx: &egui::Context) {
        if self.hidden || self.collapsed {
            self.show_panel(ctx);
        } else {
            self.hide_panel(ctx);
        }
    }

    /// 一次性恢复位置 / 尺寸 / 可见性，不分多帧逐步展开，避免闪烁与拖影
    fn show_panel(&mut self, ctx: &egui::Context) {
        if !self.hidden && !self.collapsed {
            self.last_active = Instant::now();
            return;
        }
        self.hidden = false;
        self.collapsed = false;
        self.last_active = Instant::now();
        self.last_shown = Instant::now();
        self.geometry_hold = Instant::now();

        ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
            self.settings.x,
            self.settings.y,
        )));
        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
            self.settings.clamped_width(),
            self.settings.height,
        )));
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        self.apply_window_level(ctx);
        ctx.request_repaint();
    }

    fn hide_panel(&mut self, ctx: &egui::Context) {
        if self.hidden || self.collapsed {
            return;
        }
        self.geometry_hold = Instant::now();
        if self.settings.edge_hide {
            self.collapsed = true;
            let monitor = self.monitor_size(ctx);
            let width = self.settings.clamped_width();
            let center = self.settings.x + width / 2.0;
            // 收纳到最近的屏幕边缘
            let x = if center > monitor.x / 2.0 {
                monitor.x - EDGE_STRIP
            } else {
                0.0
            };
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(
                EDGE_STRIP,
                self.settings.height,
            )));
            ctx.send_viewport_cmd(egui::ViewportCommand::OuterPosition(egui::pos2(
                x,
                self.settings.y,
            )));
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(
                egui::WindowLevel::AlwaysOnTop,
            ));
        } else {
            self.hidden = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        }
    }

    /// 只有鼠标离开整个竖栏、且没有任何交互进行中时，才会隐藏
    fn update_autohide(&mut self, ctx: &egui::Context) {
        if self.hidden {
            return;
        }
        let pointer_inside = ctx.input(|i| i.pointer.hover_pos().is_some());
        let busy = self.drag.is_some()
            || self.resize.is_some()
            || self.show_settings
            || ctx.memory(|m| m.any_popup_open())
            || ctx.input(|i| i.pointer.any_down())
            || ctx.wants_keyboard_input();

        if pointer_inside || busy {
            self.last_active = Instant::now();
            // 贴边状态下鼠标碰到感知区：立即、一次性完整展开
            if self.collapsed && pointer_inside {
                self.show_panel(ctx);
            }
            return;
        }

        if self.settings.panel_pinned || self.collapsed {
            return;
        }
        // 刚展开的短时间内不允许隐藏，避免“弹出即收回”的闪烁
        if self.last_shown.elapsed() < Duration::from_millis(400) {
            return;
        }
        if self.last_active.elapsed() > HIDE_DELAY {
            self.hide_panel(ctx);
        }
    }

    /// 左侧边缘拖动调整宽度（300–800 px）。
    /// 拖动过程中**不**变动窗口（避免每帧重建窗口导致的卡顿），
    /// 只画一条轻量预览线；松开鼠标时一次性计算并立即应用新宽度。
    /// 采用“右边缘固定 + 绝对位置计算”，不累加增量，因此不会抖动；
    /// 平时不绘制任何竖线，只在鼠标移到边缘时切换为左右拉伸光标。
    fn width_resizer(&mut self, ctx: &egui::Context) {
        let screen = ctx.screen_rect();
        egui::Area::new(egui::Id::new("cliprail_resizer"))
            .order(egui::Order::Foreground)
            .fixed_pos(screen.left_top())
            .show(ctx, |ui| {
                let (_rect, response) = ui.allocate_exact_size(
                    egui::vec2(6.0, screen.height()),
                    egui::Sense::drag(),
                );
                if response.hovered() || response.dragged() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeHorizontal);
                }

                if response.drag_started() {
                    let width = self.settings.clamped_width();
                    self.resize = Some(ResizeState {
                        right: self.settings.x + width,
                        preview: width,
                    });
                }

                if response.dragged() {
                    // 只更新预览值，不发送任何窗口命令
                    let pointer = ui.ctx().input(|i| i.pointer.latest_pos());
                    let base_x = self.settings.x;
                    if let (Some(state), Some(p)) = (self.resize.as_mut(), pointer) {
                        // 鼠标在屏幕上的位置 = 窗口左边 + 窗口内坐标
                        let global_x = base_x + p.x;
                        state.preview = (state.right - global_x).clamp(300.0, 800.0).round();
                    }
                    // 预览线：标记松开后的左边缘位置
                    if let Some(state) = self.resize.as_ref() {
                        let current = self.settings.clamped_width();
                        let guide_x = screen.left() + (current - state.preview).max(0.0);
                        ui.painter().rect_filled(
                            egui::Rect::from_min_size(
                                egui::pos2(guide_x, screen.top()),
                                egui::vec2(2.0, screen.height()),
                            ),
                            0.0,
                            crate::ui::theme::ACCENT,
                        );
                    }
                }

                if response.drag_stopped() {
                    // 松开鼠标：一次性计算并应用
                    if let Some(state) = self.resize.take() {
                        let new_width = state.preview.clamp(300.0, 800.0).round();
                        let new_x = (state.right - new_width).round();
                        if (new_width - self.settings.width).abs() >= 1.0 {
                            self.settings.width = new_width;
                            self.settings.x = new_x;
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::OuterPosition(
                                egui::pos2(new_x, self.settings.y),
                            ));
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::InnerSize(
                                egui::vec2(new_width, self.settings.height),
                            ));
                            self.mark_dirty();
                        }
                    }
                    self.geometry_hold = Instant::now();
                    ui.ctx().request_repaint();
                }
            });
    }

    /// 记录用户手动移动 / 缩放后的位置，下次启动恢复
    fn sync_geometry(&mut self, ctx: &egui::Context) {
        if self.collapsed || self.hidden {
            return;
        }
        // 调宽 / 显示隐藏刚发生时，系统上报的窗口矩形还是旧值，
        // 这时回写会与我们自己的目标值互相拉扯（表现为抖动）。
        if self.resize.is_some() || self.geometry_hold.elapsed() < Duration::from_millis(400) {
            return;
        }
        let outer = ctx.input(|i| i.viewport().outer_rect);
        if let Some(rect) = outer {
            let (x, y, w, h) = (rect.left(), rect.top(), rect.width(), rect.height());
            if w >= 200.0
                && ((x - self.settings.x).abs() > 2.0
                    || (y - self.settings.y).abs() > 2.0
                    || (w - self.settings.width).abs() > 2.0
                    || (h - self.settings.height).abs() > 2.0)
            {
                self.settings.x = x;
                self.settings.y = y;
                self.settings.width = w;
                self.settings.height = h;
                self.mark_dirty();
            }
        }
    }

    // ------------------------------------------------------------ 轮询

    fn poll_clipboard(&mut self) {
        while let Ok(event) = self.clip_rx.try_recv() {
            self.add_clip(event);
        }
    }

    /// 窗口内部的快捷键：直接从输入事件中“消费”掉该组合键，
    /// 因此它的优先级高于输入框等任何控件，不会被吸走。
    fn poll_local_hotkey(&mut self, ctx: &egui::Context) {
        let combo = match crate::hotkey::parse_egui(&self.settings.hotkey) {
            Some(combo) => combo,
            None => return,
        };
        let pressed = ctx.input_mut(|i| i.consume_key(combo.0, combo.1));
        if pressed && self.last_hotkey.elapsed() > Duration::from_millis(200) {
            self.last_hotkey = Instant::now();
            self.toggle_panel(ctx);
        }
    }

    fn poll_hotkey(&mut self, ctx: &egui::Context) {
        while let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
            if let Some(id) = self.hotkey_id {
                if event.id != id {
                    continue;
                }
            }
            // 按下 / 松开会各发一次事件，用时间窗去抱
            if self.last_hotkey.elapsed() > Duration::from_millis(250) {
                self.toggle_panel(ctx);
            }
            self.last_hotkey = Instant::now();
        }
    }

    /// `ClipRail --toggle`（Wayland 方案）通过标记文件通知本实例
    fn poll_toggle_file(&mut self, ctx: &egui::Context) {
        if self.last_toggle_check.elapsed() < Duration::from_millis(250) {
            return;
        }
        self.last_toggle_check = Instant::now();
        let path = store::toggle_file();
        if path.exists() {
            let _ = std::fs::remove_file(&path);
            self.toggle_panel(ctx);
        }
    }

    fn tick_copied(&mut self) {
        if let Some((_, at)) = &self.copied {
            if at.elapsed() > COPIED_DURATION {
                self.copied = None;
            }
        }
    }

    fn persist(&mut self, force: bool) {
        if !self.dirty {
            return;
        }
        if !force && self.last_persist.elapsed() < PERSIST_INTERVAL {
            return;
        }
        store::save_items(&self.items);
        store::save_settings(&self.settings);
        archive::rebuild(&self.items);
        self.dirty = false;
        self.last_persist = Instant::now();
    }
}

impl eframe::App for App {
    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        [0.976, 0.973, 0.969, 1.0]
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.placed {
            self.place_initial(ctx);
        }

        self.poll_clipboard();
        // 先处理窗口内的快捷键（优先级高于任何控件），
        // 保证鼠标在竖栏范围内 / 窗口获得焦点时按键也能显示隐藏
        self.poll_local_hotkey(ctx);
        self.poll_hotkey(ctx);
        self.poll_toggle_file(ctx);
        self.tick_copied();

        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            // Esc 取消拖拽，不删除、不改变顺序
            self.drag = None;
            if self.show_settings {
                self.close_settings();
            }
        }

        if self.collapsed {
            sidebar::show_collapsed(ctx);
        } else {
            sidebar::show(self, ctx);
            settings_ui::show(self, ctx);
            self.width_resizer(ctx);
            self.sync_geometry(ctx);
        }

        self.update_autohide(ctx);
        self.persist(false);

        ctx.request_repaint_after(Duration::from_millis(120));
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.dirty = true;
        self.persist(true);
    }
}
