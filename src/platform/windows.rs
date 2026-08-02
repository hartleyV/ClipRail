//! Windows 平台细节。
//! 全局快捷键、置顶、无边框窗口均由 winit / global-hotkey 处理，
//! 这里只保留平台特定的辅助函数。

#[allow(dead_code)]
pub fn is_wayland() -> bool {
    false
}
