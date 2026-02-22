//! # 性能日志持久化
//!
//! 将每次生成的性能分析数据写入 `~/.local/share/lwd/logs/` 下的
//! 带时间戳 JSON 文件，并按配置的最大数量自动清理旧日志。

use serde::{Deserialize, Serialize};

use super::paths;

/// 持久化的单步性能记录
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepEntry {
    pub index: usize,
    pub name: String,
    pub avg_ms: f64,
    pub min_ms: f64,
    pub max_ms: f64,
}

/// 一次完整生成的性能摘要
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfEntry {
    pub timestamp: String,
    pub seed: String,
    pub world_size: String,
    pub total_ms: f64,
    pub steps: Vec<StepEntry>,
}

/// 性能日志文件的元信息（供列表展示）
#[derive(Debug, Clone)]
pub struct LogFileInfo {
    pub filename: String,
    pub timestamp: String,
    pub total_ms: f64,
    pub world_size: String,
}

/// 保存一条性能日志。
///
/// 文件名格式：`perf_YYYYMMDD_HHMMSS.json`
pub fn save_entry(entry: &PerfEntry, max_files: usize) {
    let dir = paths::logs_dir();
    let _ = std::fs::create_dir_all(&dir);

    let filename = format!("perf_{}.json", entry.timestamp.replace([':', '-', ' '], ""));
    let path = dir.join(&filename);

    if let Ok(content) = serde_json::to_string_pretty(entry) {
        let _ = std::fs::write(path, content);
    }

    // 清理超出限制的旧日志
    cleanup_old_logs(&dir, max_files);
}

/// 列出所有性能日志文件（按时间倒序），并解析基本信息。
pub fn list_entries() -> Vec<LogFileInfo> {
    let dir = paths::logs_dir();
    let mut entries = Vec::new();

    let Ok(read) = std::fs::read_dir(&dir) else {
        return entries;
    };

    for item in read.flatten() {
        let filename = item.file_name().to_string_lossy().to_string();
        if !filename.starts_with("perf_") || !filename.ends_with(".json") {
            continue;
        }

        // 快速解析摘要信息（只读文件头部字段，避免反序列化所有 steps）
        if let Ok(content) = std::fs::read_to_string(item.path()) {
            if let Ok(entry) = serde_json::from_str::<PerfEntry>(&content) {
                entries.push(LogFileInfo {
                    filename,
                    timestamp: entry.timestamp.clone(),
                    total_ms: entry.total_ms,
                    world_size: entry.world_size,
                });
            }
        }
    }

    // 按文件名倒序（即时间倒序）
    entries.sort_by(|a, b| b.filename.cmp(&a.filename));
    entries
}

/// 读取指定日志文件的完整内容。
pub fn load_entry(filename: &str) -> Option<PerfEntry> {
    let path = paths::logs_dir().join(filename);
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

/// 清理超出 max_files 数量的最旧日志文件。
fn cleanup_old_logs(dir: &std::path::Path, max_files: usize) {
    if max_files == 0 {
        return;
    }

    let mut files: Vec<String> = std::fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            if name.starts_with("perf_") && name.ends_with(".json") {
                Some(name)
            } else {
                None
            }
        })
        .collect();

    if files.len() <= max_files {
        return;
    }

    files.sort(); // 时间戳升序，最旧在前
    let to_remove = files.len() - max_files;
    for name in files.into_iter().take(to_remove) {
        let _ = std::fs::remove_file(dir.join(name));
    }
}
