//! # 应用路径管理
//!
//! 统一计算所有应用数据文件的路径。
//! 应用数据目录为 `~/.local/share/lwd/`，日志子目录为 `logs/`。
//!
//! 首次使用时自动创建目录，并检测旧版 `generation.runtime.json`
//! （可执行文件同级目录下），如存在则自动迁移到新位置。

use std::path::PathBuf;
use std::sync::OnceLock;

/// 全局单例：应用数据根目录（`~/.local/share/lwd/`）
static DATA_DIR: OnceLock<PathBuf> = OnceLock::new();

/// 获取应用数据根目录，首次调用时初始化（创建目录 + 迁移旧文件）。
pub fn data_dir() -> &'static PathBuf {
    DATA_DIR.get_or_init(|| {
        let dir = resolve_data_dir();
        // 确保目录存在
        let _ = std::fs::create_dir_all(&dir);
        let _ = std::fs::create_dir_all(dir.join("logs"));
        // 迁移旧文件
        migrate_legacy(&dir);
        dir
    })
}

/// runtime.json 的完整路径
pub fn runtime_json_path() -> PathBuf {
    data_dir().join("runtime.json")
}

/// 性能日志目录
pub fn logs_dir() -> PathBuf {
    data_dir().join("logs")
}

// ── 内部实现 ────────────────────────────────────────────────

/// 推算数据根目录
fn resolve_data_dir() -> PathBuf {
    // 1) 优先使用 $XDG_DATA_HOME/lwd
    if let Ok(xdg) = std::env::var("XDG_DATA_HOME") {
        let p = PathBuf::from(xdg).join("lwd");
        if p.parent().map(|d| d.exists()).unwrap_or(false) {
            return p;
        }
    }
    // 2) 回退 ~/.local/share/lwd
    if let Some(home) = home_dir() {
        return home.join(".local").join("share").join("lwd");
    }
    // 3) 极端 fallback：可执行文件旁边
    exe_dir().unwrap_or_else(|| PathBuf::from("."))
}

/// 获取 $HOME
fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

/// 获取可执行文件所在目录
fn exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
}

/// 迁移旧版 `generation.runtime.json`。
///
/// 检查可执行文件同级目录和当前工作目录下是否存在旧文件，
/// 如存在则复制到新位置，再删除旧文件。
fn migrate_legacy(new_dir: &PathBuf) {
    let new_path = new_dir.join("runtime.json");
    if new_path.exists() {
        return; // 新文件已存在，无需迁移
    }

    let candidates = [
        exe_dir().map(|d| d.join("generation.runtime.json")),
        Some(PathBuf::from("generation.runtime.json")),
    ];

    for candidate in candidates.into_iter().flatten() {
        if candidate.exists() {
            if let Ok(content) = std::fs::read_to_string(&candidate) {
                if std::fs::write(&new_path, &content).is_ok() {
                    let _ = std::fs::remove_file(&candidate);
                    eprintln!(
                        "[storage] 已迁移旧配置: {} → {}",
                        candidate.display(),
                        new_path.display()
                    );
                }
            }
            return;
        }
    }
}
