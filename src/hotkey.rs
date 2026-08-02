use crate::model::ClipboardEvent;
use crossbeam_channel::Sender;
use global_hotkey::{hotkey::{Code, HotKey, Modifiers}, GlobalHotKeyEvent, GlobalHotKeyManager};
use std::{thread, time::Duration};

pub fn parse(value: &str) -> Result<HotKey, String> {
    let lower = value.to_lowercase();
    let parts: Vec<_> = lower.split('+').map(str::trim).filter(|s| !s.is_empty()).collect();
    if parts.len() < 2 { return Err("请至少包含一个修饰键和一个主键".into()); }
    let mut mods = Modifiers::empty();
    let mut code = None;
    for part in parts {
        match part {
            "ctrl" | "control" => mods |= Modifiers::CONTROL,
            "alt" => mods |= Modifiers::ALT,
            "shift" => mods |= Modifiers::SHIFT,
            "super" | "win" | "meta" => mods |= Modifiers::SUPER,
            key => code = Some(parse_code(key)?),
        }
    }
    if mods.is_empty() { return Err("请至少使用一个修饰键".into()); }
    Ok(HotKey::new(Some(mods), code.ok_or("缺少主键")?))
}

fn parse_code(key: &str) -> Result<Code, String> {
    let code = match key {
        "a"=>Code::KeyA,"b"=>Code::KeyB,"c"=>Code::KeyC,"d"=>Code::KeyD,"e"=>Code::KeyE,"f"=>Code::KeyF,
        "g"=>Code::KeyG,"h"=>Code::KeyH,"i"=>Code::KeyI,"j"=>Code::KeyJ,"k"=>Code::KeyK,"l"=>Code::KeyL,
        "m"=>Code::KeyM,"n"=>Code::KeyN,"o"=>Code::KeyO,"p"=>Code::KeyP,"q"=>Code::KeyQ,"r"=>Code::KeyR,
        "s"=>Code::KeyS,"t"=>Code::KeyT,"u"=>Code::KeyU,"v"=>Code::KeyV,"w"=>Code::KeyW,"x"=>Code::KeyX,
        "y"=>Code::KeyY,"z"=>Code::KeyZ,"f1"=>Code::F1,"f2"=>Code::F2,"f3"=>Code::F3,"f4"=>Code::F4,
        "f5"=>Code::F5,"f6"=>Code::F6,"f7"=>Code::F7,"f8"=>Code::F8,"f9"=>Code::F9,"f10"=>Code::F10,
        "f11"=>Code::F11,"f12"=>Code::F12,
        _ => return Err(format!("不支持的按键：{key}")),
    };
    Ok(code)
}

pub fn spawn(value: String, tx: Sender<ClipboardEvent>) {
    thread::spawn(move || {
        let hotkey = match parse(&value) { Ok(v) => v, Err(_) => return };
        let manager = match GlobalHotKeyManager::new() { Ok(v) => v, Err(_) => return };
        if manager.register(hotkey).is_err() { return; }
        loop {
            if let Ok(event) = GlobalHotKeyEvent::receiver().try_recv() {
                if event.id == hotkey.id() { let _ = tx.send(ClipboardEvent::ToggleWindow); }
            }
            thread::sleep(Duration::from_millis(40));
        }
    });
}
