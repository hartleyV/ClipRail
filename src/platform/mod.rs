//! 平台相关的小差异。

#[cfg(target_os = "windows")]
pub mod windows;

#[cfg(not(target_os = "windows"))]
pub mod linux;

/// 当前会话是否为 Wayland（Wayland 下全局快捷键可能被限制）
#[allow(dead_code)]
pub fn is_wayland() -> bool {
    #[cfg(not(target_os = "windows"))]
    {
        linux::is_wayland()
    }
    #[cfg(target_os = "windows")]
    {
        false
    }
}

/// 快捷键不可用时给用户的提示
#[allow(dead_code)]
pub fn hotkey_fallback_hint() -> &'static str {
    if is_wayland() {
        "Wayland 限制下可将系统快捷键绑定到：ClipRail --toggle"
    } else {
        "可在设置中更换快捷键，或使用：ClipRail --toggle"
    }
}
