//! 本地存储：data/ 目录、clips.json、settings.json、images/
//! 所有写入都通过临时文件 + 原子替换完成。

use std::collections::HashSet;
use std::io::Write;
use std::path::{Path, PathBuf};

use chrono::{Local, TimeZone};
use sha2::{Digest, Sha256};

use crate::model::{Item, Kind, Settings};

pub fn base_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn data_dir() -> PathBuf {
    base_dir().join("data")
}

pub fn images_dir() -> PathBuf {
    data_dir().join("images")
}

pub fn archives_dir() -> PathBuf {
    data_dir().join("archives")
}

pub fn clips_file() -> PathBuf {
    data_dir().join("clips.json")
}

pub fn settings_file() -> PathBuf {
    data_dir().join("settings.json")
}

/// `--toggle` 命令通过该文件通知正在运行的实例
pub fn toggle_file() -> PathBuf {
    data_dir().join(".toggle")
}

pub fn ensure_dirs() {
    let _ = std::fs::create_dir_all(images_dir());
    let _ = std::fs::create_dir_all(archives_dir());
}

// ---------------------------------------------------------------- 工具函数

pub fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub fn now_ts() -> i64 {
    Local::now().timestamp()
}

pub fn format_ts(ts: i64, fmt: &str) -> String {
    match Local.timestamp_opt(ts, 0).single() {
        Some(dt) => dt.format(fmt).to_string(),
        None => String::from("-"),
    }
}

pub fn new_id(hash: &str) -> String {
    let short: String = hash.chars().take(8).collect();
    format!("{}-{}", now_ts(), short)
}

/// 写临时文件后原子替换，避免中途崩溃导致 JSON 损坏
pub fn write_atomic(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("tmp");
    {
        let mut f = std::fs::File::create(&tmp)?;
        f.write_all(bytes)?;
        f.sync_all()?;
    }
    // Windows 上 rename 覆盖已存在文件会失败，先删除
    if path.exists() {
        let _ = std::fs::remove_file(path);
    }
    std::fs::rename(&tmp, path)
}

// ---------------------------------------------------------------- 记录读写

/// 读取记录：
/// - 跳过重复哈希
/// - 跳过图片文件已丢失的记录
/// - 单条记录损坏不影响整体启动
pub fn load_items() -> Vec<Item> {
    let raw = match std::fs::read_to_string(clips_file()) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };

    let values: Vec<serde_json::Value> = match serde_json::from_str(&raw) {
        Ok(v) => v,
        Err(_) => {
            // 索引损坏：备份后重新开始，不阻塞启动
            let backup = data_dir().join("clips.corrupt.json");
            let _ = std::fs::write(backup, raw.as_bytes());
            return Vec::new();
        }
    };

    let mut seen: HashSet<String> = HashSet::new();
    let mut items: Vec<Item> = Vec::new();
    for v in values {
        let item: Item = match serde_json::from_value(v) {
            Ok(i) => i,
            Err(_) => continue,
        };
        if item.hash.is_empty() || !seen.insert(item.hash.clone()) {
            continue;
        }
        if item.kind == Kind::Image && !base_dir().join(&item.image_path).exists() {
            continue;
        }
        items.push(item);
    }
    sort_pinned_first(&mut items);
    items
}

pub fn save_items(items: &[Item]) {
    if let Ok(json) = serde_json::to_vec_pretty(items) {
        let _ = write_atomic(&clips_file(), &json);
    }
}

/// 置顶 Item 永远排在普通 Item 之前（稳定排序，保留手动顺序）
pub fn sort_pinned_first(items: &mut [Item]) {
    items.sort_by_key(|i| !i.pinned);
}

// ---------------------------------------------------------------- 设置读写

pub fn load_settings() -> Settings {
    std::fs::read_to_string(settings_file())
        .ok()
        .and_then(|s| serde_json::from_str::<Settings>(&s).ok())
        .unwrap_or_default()
}

pub fn save_settings(settings: &Settings) {
    if let Ok(json) = serde_json::to_vec_pretty(settings) {
        let _ = write_atomic(&settings_file(), &json);
    }
}

// ---------------------------------------------------------------- 图片存取

/// 保存 RGBA 图片为 PNG，返回相对路径（失败返回 None，不影响程序运行）
pub fn save_image(id: &str, width: u32, height: u32, rgba: &[u8]) -> Option<String> {
    if width == 0 || height == 0 {
        return None;
    }
    let buffer = image::RgbaImage::from_raw(width, height, rgba.to_vec())?;
    let rel = format!("data/images/{}.png", id);
    let abs = base_dir().join(&rel);
    if let Some(parent) = abs.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    match buffer.save_with_format(&abs, image::ImageFormat::Png) {
        Ok(_) => Some(rel),
        Err(_) => None,
    }
}

pub fn delete_image(rel_path: &str) {
    if rel_path.is_empty() {
        return;
    }
    let _ = std::fs::remove_file(base_dir().join(rel_path));
}

/// 读取图片并生成缩略图（等比例缩放），返回 (宽, 高, RGBA)
pub fn load_thumbnail(rel_path: &str, max_width: u32) -> Option<(u32, u32, Vec<u8>)> {
    let bytes = std::fs::read(base_dir().join(rel_path)).ok()?;
    let img = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png).ok()?;
    let rgba = if img.width() > max_width {
        let ratio = max_width as f32 / img.width() as f32;
        let h = ((img.height() as f32 * ratio).round() as u32).max(1);
        img.resize(max_width, h, image::imageops::FilterType::Triangle)
            .to_rgba8()
    } else {
        img.to_rgba8()
    };
    Some((rgba.width(), rgba.height(), rgba.into_raw()))
}

/// 读取原图（复制回剪贴板时使用）
pub fn load_full_image(rel_path: &str) -> Option<(u32, u32, Vec<u8>)> {
    let bytes = std::fs::read(base_dir().join(rel_path)).ok()?;
    let img = image::load_from_memory_with_format(&bytes, image::ImageFormat::Png).ok()?;
    let rgba = img.to_rgba8();
    Some((rgba.width(), rgba.height(), rgba.into_raw()))
}
