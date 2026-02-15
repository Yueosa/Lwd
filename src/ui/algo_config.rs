//! 算法参数配置窗口
//!
//! 根据算法模块的 `PhaseMeta.params` 自动生成 UI 控件。

use egui::{Context, Ui};

use crate::generation::algorithm::{ParamDef, ParamType, PhaseAlgorithm, PhaseMeta};

/// 显示算法参数配置窗口。
///
/// 返回 `true` 表示参数有变更（调用方应标记 texture_dirty 等）。
pub fn show_algo_config_window(
    ctx: &Context,
    open: &mut bool,
    algorithm: &mut Box<dyn PhaseAlgorithm>,
) -> bool {
    let meta = algorithm.meta();
    let mut params = algorithm.get_params();
    let mut changed = false;

    egui::Window::new(format!("⚙ {} — 参数配置", meta.name))
        .open(open)
        .resizable(true)
        .default_width(340.0)
        .show(ctx, |ui| {
            if meta.params.is_empty() {
                ui.label("此算法模块没有可调参数。");
                return;
            }

            ui.label(&meta.description);
            ui.separator();

            for param_def in &meta.params {
                changed |= render_param(ui, param_def, &mut params);
            }

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("重置为默认值").clicked() {
                    // 用 param_def.default 还原每个参数
                    for param_def in &meta.params {
                        if let Some(obj) = params.as_object_mut() {
                            obj.insert(param_def.key.clone(), param_def.default.clone());
                        }
                    }
                    changed = true;
                }
            });
        });

    if changed {
        algorithm.set_params(&params);
    }

    changed
}

/// 根据 ParamDef 的类型渲染对应的 UI 控件，返回是否发生了修改。
fn render_param(ui: &mut Ui, def: &ParamDef, params: &mut serde_json::Value) -> bool {
    let mut changed = false;

    ui.horizontal(|ui| {
        ui.label(&def.name);
        if !def.description.is_empty() {
            ui.label("ℹ").on_hover_text(&def.description);
        }
    });

    let obj = match params.as_object_mut() {
        Some(o) => o,
        None => return false,
    };

    match &def.param_type {
        ParamType::Float { min, max } => {
            let current = obj
                .get(&def.key)
                .and_then(|v| v.as_f64())
                .unwrap_or_else(|| def.default.as_f64().unwrap_or(0.0));
            let mut val = current;
            let resp = ui.add(
                egui::Slider::new(&mut val, *min..=*max)
                    .text(&def.key)
                    .clamp_to_range(true),
            );
            if resp.changed() {
                obj.insert(def.key.clone(), serde_json::json!(val));
                changed = true;
            }
        }
        ParamType::Int { min, max } => {
            let current = obj
                .get(&def.key)
                .and_then(|v| v.as_i64())
                .unwrap_or_else(|| def.default.as_i64().unwrap_or(0));
            let mut val = current;
            let resp = ui.add(
                egui::Slider::new(&mut val, (*min)..=(*max))
                    .text(&def.key)
                    .clamp_to_range(true),
            );
            if resp.changed() {
                obj.insert(def.key.clone(), serde_json::json!(val));
                changed = true;
            }
        }
        ParamType::Bool => {
            let current = obj
                .get(&def.key)
                .and_then(|v| v.as_bool())
                .unwrap_or_else(|| def.default.as_bool().unwrap_or(false));
            let mut val = current;
            if ui.checkbox(&mut val, "").changed() {
                obj.insert(def.key.clone(), serde_json::json!(val));
                changed = true;
            }
        }
        ParamType::Text => {
            let current = obj
                .get(&def.key)
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| def.default.as_str().unwrap_or(""))
                .to_string();
            let mut val = current;
            if ui.text_edit_singleline(&mut val).changed() {
                obj.insert(def.key.clone(), serde_json::json!(val));
                changed = true;
            }
        }
        ParamType::Enum { options } => {
            let current = obj
                .get(&def.key)
                .and_then(|v| v.as_str())
                .unwrap_or_else(|| def.default.as_str().unwrap_or(""))
                .to_string();
            let mut selected = current;
            egui::ComboBox::from_label(&def.key)
                .selected_text(&selected)
                .show_ui(ui, |ui| {
                    for opt in options {
                        if ui.selectable_value(&mut selected, opt.clone(), opt).changed() {
                            obj.insert(def.key.clone(), serde_json::json!(selected.clone()));
                            changed = true;
                        }
                    }
                });
        }
    }

    ui.add_space(4.0);
    changed
}
