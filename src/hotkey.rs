//! 全局快捷键：解析字符串、注册 / 重新注册。

use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::GlobalHotKeyManager;

/// 解析形如 `alt+shift+v` / `ctrl+alt+c` / `super+shift+f2` 的快捷键
pub fn parse(input: &str) -> Result<HotKey, String> {
    let text = input.trim().to_lowercase();
    if text.is_empty() {
        return Err("快捷键不能为空".to_string());
    }

    let mut modifiers = Modifiers::empty();
    let mut main_key: Option<Code> = None;

    for part in text.split('+') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        match part {
            "ctrl" | "control" => modifiers |= Modifiers::CONTROL,
            "alt" | "option" => modifiers |= Modifiers::ALT,
            "shift" => modifiers |= Modifiers::SHIFT,
            "super" | "win" | "meta" | "cmd" => modifiers |= Modifiers::SUPER,
            other => {
                if main_key.is_some() {
                    return Err("只能设置一个主键".to_string());
                }
                main_key = Some(parse_code(other)?);
            }
        }
    }

    let code = main_key.ok_or_else(|| "缺少主键（A–Z 或 F1–F12）".to_string())?;
    if modifiers.is_empty() {
        return Err("至少需要一个修饰键（Ctrl / Alt / Shift / Super）".to_string());
    }
    Ok(HotKey::new(Some(modifiers), code))
}

fn parse_code(key: &str) -> Result<Code, String> {
    let code = match key {
        "a" => Code::KeyA,
        "b" => Code::KeyB,
        "c" => Code::KeyC,
        "d" => Code::KeyD,
        "e" => Code::KeyE,
        "f" => Code::KeyF,
        "g" => Code::KeyG,
        "h" => Code::KeyH,
        "i" => Code::KeyI,
        "j" => Code::KeyJ,
        "k" => Code::KeyK,
        "l" => Code::KeyL,
        "m" => Code::KeyM,
        "n" => Code::KeyN,
        "o" => Code::KeyO,
        "p" => Code::KeyP,
        "q" => Code::KeyQ,
        "r" => Code::KeyR,
        "s" => Code::KeyS,
        "t" => Code::KeyT,
        "u" => Code::KeyU,
        "v" => Code::KeyV,
        "w" => Code::KeyW,
        "x" => Code::KeyX,
        "y" => Code::KeyY,
        "z" => Code::KeyZ,
        "f1" => Code::F1,
        "f2" => Code::F2,
        "f3" => Code::F3,
        "f4" => Code::F4,
        "f5" => Code::F5,
        "f6" => Code::F6,
        "f7" => Code::F7,
        "f8" => Code::F8,
        "f9" => Code::F9,
        "f10" => Code::F10,
        "f11" => Code::F11,
        "f12" => Code::F12,
        _ => return Err(format!("不支持的按键：{}", key)),
    };
    Ok(code)
}

/// 保存当前注册状态，便于修改设置时替换
pub struct HotkeyService {
    manager: Option<GlobalHotKeyManager>,
    current: Option<HotKey>,
}

impl HotkeyService {
    pub fn new() -> Self {
        Self {
            manager: GlobalHotKeyManager::new().ok(),
            current: None,
        }
    }

    pub fn available(&self) -> bool {
        self.manager.is_some()
    }

    /// 注册新的快捷键；失败时返回可直接展示的错误文本
    pub fn register(&mut self, spec: &str) -> Result<u32, String> {
        let hotkey = parse(spec)?;
        let manager = self
            .manager
            .as_ref()
            .ok_or_else(|| "当前系统不支持全局快捷键，可使用 ClipRail --toggle".to_string())?;

        if let Some(old) = self.current.take() {
            let _ = manager.unregister(old);
        }
        manager
            .register(hotkey)
            .map_err(|e| format!("快捷键注册失败（可能与其他程序冲突）：{}", e))?;
        self.current = Some(hotkey);
        Ok(hotkey.id())
    }

    pub fn current_id(&self) -> Option<u32> {
        self.current.map(|h| h.id())
    }
}
