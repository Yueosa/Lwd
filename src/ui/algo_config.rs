//! 算法参数配置窗口
//!
//! 根据算法模块的 `PhaseMeta.params` 自动生成 UI 控件。
//! 参数按 `ParamDef.group` 分组，以可折叠面板呈现。

use egui::{Context, Ui};

use crate::generation::algorithm::{ParamDef, ParamType, PhaseAlgorithm, PhaseMeta};
use crate::ui::theme;

/// 算法配置窗口的返回值
pub struct AlgoConfigResult {
    /// 参数是否有变更
    pub changed: bool,
    /// 用户是否请求重新执行当前步骤
    pub replay_requested: bool,
}

/// 显示算法参数配置窗口。
pub fn show_algo_config_window(
    ctx: &Context,
    open: &mut bool,
    algorithm: &mut Box<dyn PhaseAlgorithm>,
) -> AlgoConfigResult {
    let meta = algorithm.meta();
    let mut params = algorithm.get_params();
    let mut changed = false;
    let mut replay = false;

    egui::Window::new(format!("⚙ {} — 参数配置", meta.name))
        .open(open)
        .resizable(true)
        .default_width(360.0)
        .show(ctx, |ui| {
            if meta.params.is_empty() {
                ui.label("此算法模块没有可调参数。");
                return;
            }

            ui.label(&meta.description);
            ui.separator();

            egui::ScrollArea::vertical()
                .max_height(ui.available_height() - 40.0)
                .show(ui, |ui| {
                    changed |= render_grouped_params(ui, &meta, &mut params);
                });

            ui.separator();

            ui.horizontal(|ui| {
                if ui.button("🔄 重新执行当前步骤")
                    .on_hover_text("应用修改后的参数，从当前阶段开头重新执行")
                    .clicked()
                {
                    replay = true;
                }
                if ui.button("重置为默认值").clicked() {
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

    AlgoConfigResult {
        changed,
        replay_requested: replay,
    }
}

/// 将参数按 group 分组渲染，有 group 的用 CollapsingHeader，无 group 的直接渲染。
fn render_grouped_params(
    ui: &mut Ui,
    meta: &PhaseMeta,
    params: &mut serde_json::Value,
) -> bool {
    let mut changed = false;

    // 收集分组顺序（保持首次出现顺序）
    let mut group_order: Vec<Option<String>> = Vec::new();
    for p in &meta.params {
        let g = p.group.clone();
        if !group_order.contains(&g) {
            group_order.push(g);
        }
    }

    for group in &group_order {
        let group_params: Vec<&ParamDef> = meta
            .params
            .iter()
            .filter(|p| &p.group == group)
            .collect();

        match group {
            None => {
                // 无分组的参数直接渲染
                for param_def in &group_params {
                    changed |= render_param(ui, param_def, params);
                }
            }
            Some(group_name) => {
                // 有分组的参数用可折叠面板
                let id = ui.make_persistent_id(group_name);
                egui::collapsing_header::CollapsingState::load_with_default_open(
                    ui.ctx(),
                    id,
                    false,
                )
                .show_header(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.strong(group_name);
                        ui.colored_label(
                            theme::TEXT_MUTED,
                            format!("({} 个参数)", group_params.len()),
                        );
                    });
                })
                .body(|ui| {
                    ui.indent(group_name, |ui| {
                        for param_def in &group_params {
                            changed |= render_param(ui, param_def, params);
                        }
                    });
                });
            }
        }
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
