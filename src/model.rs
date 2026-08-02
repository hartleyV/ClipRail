//! 数据模型：Item 与 Settings

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    Text,
    Image,
}

/// 一条剪贴板记录
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub kind: Kind,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub image_path: String,
    pub hash: String,
    pub created: i64,
    #[serde(default)]
    pub pinned: bool,
    /// 图片像素尺寸（仅用于展示，缺失不影响功能）
    #[serde(default)]
    pub width: u32,
    #[serde(default)]
    pub height: u32,
}

impl Item {
    /// 列表中显示的文本预览（限制长度，避免超长文本拖慢渲染）
    pub fn preview(&self) -> String {
        const MAX_CHARS: usize = 420;
        let trimmed = self.text.trim();
        let mut out = String::new();
        for (i, ch) in trimmed.chars().enumerate() {
            if i >= MAX_CHARS {
                out.push('…');
                break;
            }
            out.push(ch);
        }
        out
    }

    pub fn char_count(&self) -> usize {
        self.text.chars().count()
    }

    pub fn local_date(&self) -> String {
        crate::store::format_ts(self.created, "%Y-%m-%d")
    }

    pub fn local_time(&self) -> String {
        crate::store::format_ts(self.created, "%H:%M")
    }
}

/// 设置（保存在 data/settings.json）
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Settings {
    pub hotkey: String,
    pub edge_hide: bool,
    pub panel_pinned: bool,
    pub width: f32,
    pub height: f32,
    pub x: f32,
    pub y: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            hotkey: "alt+v".to_string(),
            edge_hide: true,
            panel_pinned: false,
            width: 390.0,
            height: 760.0,
            x: -1.0,
            y: 40.0,
        }
    }
}

impl Settings {
    pub fn clamped_width(&self) -> f32 {
        self.width.clamp(300.0, 800.0)
    }
}
