use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ClipKind { Text, Image }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClipItem {
    pub id: String,
    pub kind: ClipKind,
    #[serde(default)] pub text: String,
    #[serde(default)] pub image_path: PathBuf,
    pub hash: String,
    pub created: i64,
    #[serde(default)] pub pinned: bool,
}

impl ClipItem {
    pub fn date_key(&self) -> String {
        chrono::DateTime::from_timestamp(self.created, 0)
            .map(|d| d.with_timezone(&chrono::Local).format("%Y-%m-%d").to_string())
            .unwrap_or_else(|| "未知日期".into())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub hotkey: String,
    pub edge_hide: bool,
    pub panel_pinned: bool,
    pub width: f32,
    pub x: f32,
    pub y: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self { hotkey: "alt+v".into(), edge_hide: true, panel_pinned: false, width: 390.0, x: 1200.0, y: 20.0 }
    }
}

#[derive(Debug)]
pub enum ClipboardEvent {
    NewText { text: String, hash: String, created: i64 },
    NewImage { rgba: Vec<u8>, width: usize, height: usize, hash: String, created: i64 },
    ToggleWindow,
}
