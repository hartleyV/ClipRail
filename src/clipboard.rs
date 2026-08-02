//! 剪贴板线程：轮询系统剪贴板变化，并执行写回命令。

use std::borrow::Cow;
use std::time::Duration;

use crossbeam_channel::{Receiver, Sender};

use crate::store;

/// 剪贴板线程 -> 主线程
pub enum ClipEvent {
    Text { text: String, hash: String },
    Image { width: u32, height: u32, rgba: Vec<u8>, hash: String },
}

/// 主线程 -> 剪贴板线程
pub enum ClipCommand {
    SetText(String),
    SetImage { width: u32, height: u32, rgba: Vec<u8> },
}

const POLL_MS: u64 = 300;

pub fn spawn(tx: Sender<ClipEvent>, rx: Receiver<ClipCommand>) {
    std::thread::spawn(move || {
        // Clipboard 实例需要长期存活：Linux/X11 下由它持续提供剪贴板内容
        let mut clipboard = match arboard::Clipboard::new() {
            Ok(c) => c,
            Err(_) => return,
        };

        let mut last_text_hash = String::new();
        let mut last_image_hash = String::new();

        loop {
            // 1. 处理写回命令
            while let Ok(cmd) = rx.try_recv() {
                match cmd {
                    ClipCommand::SetText(text) => {
                        last_text_hash = store::hash_bytes(text.as_bytes());
                        let _ = clipboard.set_text(text);
                    }
                    ClipCommand::SetImage { width, height, rgba } => {
                        last_image_hash = store::hash_bytes(&rgba);
                        let data = arboard::ImageData {
                            width: width as usize,
                            height: height as usize,
                            bytes: Cow::Owned(rgba),
                        };
                        let _ = clipboard.set_image(data);
                    }
                }
            }

            // 2. 读取文本
            let mut handled = false;
            if let Ok(text) = clipboard.get_text() {
                if !text.trim().is_empty() {
                    let hash = store::hash_bytes(text.as_bytes());
                    if hash != last_text_hash {
                        last_text_hash = hash.clone();
                        handled = true;
                        if tx.send(ClipEvent::Text { text, hash }).is_err() {
                            return;
                        }
                    } else {
                        handled = true;
                    }
                }
            }

            // 3. 读取图片
            if !handled {
                if let Ok(img) = clipboard.get_image() {
                    let rgba = img.bytes.into_owned();
                    let hash = store::hash_bytes(&rgba);
                    if hash != last_image_hash {
                        last_image_hash = hash.clone();
                        let event = ClipEvent::Image {
                            width: img.width as u32,
                            height: img.height as u32,
                            rgba,
                            hash,
                        };
                        if tx.send(event).is_err() {
                            return;
                        }
                    }
                }
            }

            std::thread::sleep(Duration::from_millis(POLL_MS));
        }
    });
}
