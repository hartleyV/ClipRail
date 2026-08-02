use crate::model::{ClipItem, ClipKind, Settings};
use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::{collections::HashSet, fs, path::{Path, PathBuf}};

#[derive(Clone)]
pub struct Store { pub root: PathBuf }

impl Store {
    pub fn portable() -> Self {
        let base = std::env::current_exe().ok().and_then(|p| p.parent().map(Path::to_path_buf)).unwrap_or_else(|| PathBuf::from("."));
        Self { root: base.join("data") }
    }
    pub fn ensure(&self) -> Result<()> {
        fs::create_dir_all(self.root.join("images"))?;
        fs::create_dir_all(self.root.join("archives"))?;
        Ok(())
    }
    pub fn load_clips(&self) -> Vec<ClipItem> {
        let bytes = match fs::read(self.root.join("clips.json")) { Ok(v) => v, Err(_) => return vec![] };
        let raw: Vec<serde_json::Value> = serde_json::from_slice(&bytes).unwrap_or_default();
        let mut seen = HashSet::new();
        raw.into_iter().filter_map(|v| serde_json::from_value::<ClipItem>(v).ok()).filter(|c| {
            let valid_image = c.kind != ClipKind::Image || c.image_path.exists();
            valid_image && seen.insert(c.hash.clone())
        }).collect()
    }
    pub fn load_settings(&self) -> Settings {
        fs::read(self.root.join("settings.json")).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
    }
    pub fn save_clips(&self, clips: &[ClipItem]) -> Result<()> {
        self.atomic_json(&self.root.join("clips.json"), clips)?;
        self.write_archives(clips)
    }
    pub fn save_settings(&self, settings: &Settings) -> Result<()> { self.atomic_json(&self.root.join("settings.json"), settings) }
    fn atomic_json<T: serde::Serialize + ?Sized>(&self, path: &Path, value: &T) -> Result<()> {
        self.ensure()?;
        let temp = path.with_extension("tmp");
        fs::write(&temp, serde_json::to_vec_pretty(value)?).with_context(|| format!("写入 {}", temp.display()))?;
        if path.exists() { let _ = fs::remove_file(path); }
        fs::rename(temp, path)?;
        Ok(())
    }
    fn write_archives(&self, clips: &[ClipItem]) -> Result<()> {
        let mut dates = std::collections::BTreeMap::<String, Vec<&ClipItem>>::new();
        for clip in clips { dates.entry(clip.date_key()).or_default().push(clip); }
        for (date, rows) in dates { self.atomic_json(&self.root.join("archives").join(format!("{date}.json")), &rows)?; }
        Ok(())
    }
    pub fn save_image(&self, rgba: &[u8], width: usize, height: usize, created: i64, hash: &str) -> Result<PathBuf> {
        self.ensure()?;
        let path = self.root.join("images").join(format!("{}-{}.png", created, &hash[..12]));
        image::save_buffer(&path, rgba, width as u32, height as u32, image::ColorType::Rgba8)?;
        Ok(path)
    }
    pub fn remove_asset(&self, clip: &ClipItem) { if clip.kind == ClipKind::Image { let _ = fs::remove_file(&clip.image_path); } }
}

pub fn sha256(bytes: &[u8]) -> String { format!("{:x}", Sha256::digest(bytes)) }
