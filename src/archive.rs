//! 每日归档：data/archives/YYYY-MM-DD.json

use std::collections::{BTreeMap, HashSet};

use crate::model::Item;
use crate::store;

/// 按本地日期重建归档文件；同时清理已经没有记录的归档。
pub fn rebuild(items: &[Item]) {
    let dir = store::archives_dir();
    let _ = std::fs::create_dir_all(&dir);

    let mut grouped: BTreeMap<String, Vec<&Item>> = BTreeMap::new();
    for item in items {
        grouped.entry(item.local_date()).or_default().push(item);
    }

    let mut written: HashSet<String> = HashSet::new();
    for (date, list) in &grouped {
        let payload = serde_json::json!({
            "date": date,
            "count": list.len(),
            "items": list,
        });
        if let Ok(bytes) = serde_json::to_vec_pretty(&payload) {
            let path = dir.join(format!("{}.json", date));
            let _ = store::write_atomic(&path, &bytes);
            written.insert(format!("{}.json", date));
        }
    }

    // 删除已无记录的旧归档文件
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if name.ends_with(".json") && !written.contains(&name) {
                let _ = std::fs::remove_file(entry.path());
            }
        }
    }
}
