use crate::{model::ClipboardEvent, store::sha256};
use crossbeam_channel::{Receiver, Sender};
use std::{borrow::Cow, thread, time::Duration};

pub fn spawn(tx: Sender<ClipboardEvent>, write_rx: Receiver<ClipWrite>) {
    thread::spawn(move || {
        let mut board = match arboard::Clipboard::new() { Ok(v) => v, Err(_) => return };
        let mut last = String::new();
        loop {
            while let Ok(cmd) = write_rx.try_recv() {
                match cmd {
                    ClipWrite::Text(v) => { let _ = board.set_text(v); }
                    ClipWrite::Image { rgba, width, height } => {
                        let _ = board.set_image(arboard::ImageData { width, height, bytes: Cow::Owned(rgba) });
                    }
                }
            }
            if let Ok(text) = board.get_text() {
                let hash = sha256(text.as_bytes());
                if !text.trim().is_empty() && hash != last {
                    last = hash.clone();
                    let _ = tx.send(ClipboardEvent::NewText { text, hash, created: chrono::Local::now().timestamp() });
                }
            } else if let Ok(img) = board.get_image() {
                let rgba = img.bytes.into_owned();
                let hash = sha256(&rgba);
                if hash != last {
                    last = hash.clone();
                    let _ = tx.send(ClipboardEvent::NewImage { rgba, width: img.width, height: img.height, hash, created: chrono::Local::now().timestamp() });
                }
            }
            thread::sleep(Duration::from_millis(300));
        }
    });
}

#[derive(Debug)]
pub enum ClipWrite { Text(String), Image { rgba: Vec<u8>, width: usize, height: usize } }
