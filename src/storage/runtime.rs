//! # runtime.json 通用读写
//!
//! 提供对 `~/.local/share/lwd/runtime.json` 的原子读/写/合并操作。
//! 所有模块（层级配置、UI 状态、EngineConfig）统一通过此接口访问，
//! 避免多处各自读写文件导致的竞争和不一致。

use serde_json::Value;

use super::paths;

/// 读取 runtime.json 的完整内容，文件不存在 / 解析失败返回空对象。
pub fn load() -> Value {
    let path = paths::runtime_json_path();
    match std::fs::read_to_string(&path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or(Value::Object(Default::default())),
        Err(_) => Value::Object(Default::default()),
    }
}

/// 读取 runtime.json 中指定 key 的值，不存在返回 None。
pub fn load_field(key: &str) -> Option<Value> {
    let root = load();
    root.get(key).cloned()
}

/// 将整个 Value 写入 runtime.json（格式化）。
pub fn save(value: &Value) -> Result<(), std::io::Error> {
    let path = paths::runtime_json_path();
    let content = serde_json::to_string_pretty(value).unwrap_or_default();
    std::fs::write(path, content)
}

/// 合并一个字段到 runtime.json 并写入。
///
/// 读取 → 插入/替换 key → 写回。
pub fn merge_field(key: &str, value: Value) -> Result<(), std::io::Error> {
    let mut root = load();
    if let Some(obj) = root.as_object_mut() {
        obj.insert(key.to_string(), value);
    }
    save(&root)
}
