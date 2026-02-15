use serde::Deserialize;

use crate::config::ConfigError;

const STEPS_JSON: &str = include_str!("../assets/steps.json");

/// 子步骤元数据（从 steps.json 读取）
#[derive(Debug, Clone, Deserialize)]
pub struct SubStepMeta {
    pub id: String,
    pub name: String,
    pub description: String,
    pub doc_url: Option<String>,
}

/// Phase 元数据（从 steps.json 读取）
#[derive(Debug, Clone, Deserialize)]
pub struct PhaseMeta {
    pub id: u32,
    pub name: String,
    pub description: String,
    pub sub_steps: Vec<SubStepMeta>,
}

/// steps.json 顶层结构
#[derive(Debug, Clone, Deserialize)]
pub struct StepsConfig {
    pub phases: Vec<PhaseMeta>,
}

pub fn load_steps_config() -> Result<StepsConfig, ConfigError> {
    let config: StepsConfig = serde_json::from_str(STEPS_JSON)?;
    Ok(config)
}
