use egui::Window;

use crate::core::layer::LayerDefinition;

/// 配置模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConfigMode {
    Percentage,
    AbsoluteRows,
}

/// 层级配置窗口
pub fn show_layer_config_window(
    ctx: &egui::Context,
    is_open: &mut bool,
    layers: &mut [LayerDefinition],
    world_height: u32,
) -> bool {
    let mut changed = false;
    let mut should_close = false;
    
    // 配置模式状态（默认百分比模式）
    let mode_id = egui::Id::new("layer_config_mode");
    let mut mode = ctx.data_mut(|data| {
        data.get_persisted::<ConfigMode>(mode_id)
            .unwrap_or(ConfigMode::Percentage)
    });
    
    Window::new("🗺 层级配置")
        .open(is_open)
        .resizable(true)
        .default_width(500.0)
        .show(ctx, |ui| {
            ui.heading("地层分布设置");
            ui.label("调整每个层级的垂直范围（百分比或具体高度）");
            ui.separator();
            
            // 配置模式选择
            ui.horizontal(|ui| {
                ui.label("配置模式：");
                if ui.radio_value(&mut mode, ConfigMode::Percentage, "百分比").clicked() {
                    ctx.data_mut(|data| data.insert_persisted(mode_id, mode));
                }
                if ui.radio_value(&mut mode, ConfigMode::AbsoluteRows, "具体高度（行数）").clicked() {
                    ctx.data_mut(|data| data.insert_persisted(mode_id, mode));
                }
            });
            ui.separator();
            
            // 先按 start_percent 排序以确保正确的显示顺序
            let mut sorted_indices: Vec<usize> = (0..layers.len()).collect();
            sorted_indices.sort_by_key(|&i| layers[i].start_percent);
            
            // 显示每个层级的配置
            egui::Grid::new("layer_config_grid")
                .num_columns(4)
                .spacing([10.0, 8.0])
                .striped(true)
                .show(ui, |ui| {
                    // 表头（根据模式显示不同列名）
                    ui.label("层级名称");
                    match mode {
                        ConfigMode::Percentage => {
                            ui.label("起始 (%)");
                            ui.label("结束 (%)");
                            ui.label("高度（行）");
                        }
                        ConfigMode::AbsoluteRows => {
                            ui.label("起始（行）");
                            ui.label("结束（行）");
                            ui.label("百分比");
                        }
                    }
                    ui.end_row();
                    
                    // 记录需要调整的下一层级
                    let mut next_layer_adjustment: Option<(usize, u8)> = None;
                    
                    // 按顺序显示每个层级
                    for (display_index, &actual_index) in sorted_indices.iter().enumerate() {
                        ui.label(&layers[actual_index].key);
                        
                        let current_start = layers[actual_index].start_percent;
                        let current_end = layers[actual_index].end_percent;
                        let (current_start_row, current_end_row) = layers[actual_index].bounds_for_height(world_height);
                        
                        match mode {
                            ConfigMode::Percentage => {
                                // 起始百分比
                                let mut start = current_start as i32;
                                if ui.add(egui::DragValue::new(&mut start).speed(1).clamp_range(0..=100).suffix("%")).changed() {
                                    if start >= 0 && start <= 100 && start < current_end as i32 {
                                        layers[actual_index].start_percent = start as u8;
                                        changed = true;
                                    }
                                }
                                
                                // 结束百分比
                                let mut end = current_end as i32;
                                if ui.add(egui::DragValue::new(&mut end).speed(1).clamp_range(0..=100).suffix("%")).changed() {
                                    if end >= 0 && end <= 100 && end > current_start as i32 {
                                        layers[actual_index].end_percent = end as u8;
                                        
                                        if display_index + 1 < sorted_indices.len() {
                                            let next_index = sorted_indices[display_index + 1];
                                            if layers[next_index].start_percent == current_end {
                                                next_layer_adjustment = Some((next_index, end as u8));
                                            }
                                        }
                                        
                                        changed = true;
                                    }
                                }
                                
                                // 只读：行数范围
                                ui.label(format!("{} - {}", current_start_row, current_end_row));
                            }
                            ConfigMode::AbsoluteRows => {
                                let max_row = world_height as i32;
                                
                                // 起始行
                                let mut start_row = current_start_row as i32;
                                if ui.add(egui::DragValue::new(&mut start_row).speed(1).clamp_range(0..=max_row)).changed() {
                                    let new_pct = ((start_row as f64 / world_height as f64) * 100.0).round() as u8;
                                    if new_pct < current_end {
                                        layers[actual_index].start_percent = new_pct;
                                        changed = true;
                                    }
                                }
                                
                                // 结束行
                                let mut end_row = current_end_row as i32;
                                if ui.add(egui::DragValue::new(&mut end_row).speed(1).clamp_range(0..=max_row)).changed() {
                                    let new_pct = ((end_row as f64 / world_height as f64) * 100.0).round() as u8;
                                    if new_pct > current_start {
                                        layers[actual_index].end_percent = new_pct;
                                        
                                        if display_index + 1 < sorted_indices.len() {
                                            let next_index = sorted_indices[display_index + 1];
                                            if layers[next_index].start_percent == current_end {
                                                next_layer_adjustment = Some((next_index, new_pct));
                                            }
                                        }
                                        
                                        changed = true;
                                    }
                                }
                                
                                // 只读：百分比范围
                                ui.label(format!("{}% - {}%", current_start, current_end));
                            }
                        }
                        
                        ui.end_row();
                    }
                    
                    // 在遍历完成后应用下一层级的调整
                    if let Some((next_index, new_start)) = next_layer_adjustment {
                        layers[next_index].start_percent = new_start;
                    }
                });
            
            ui.separator();
            
            // 提示信息
            ui.label("💡 提示：");
            ui.label("• 层级顺序从上到下：太空 → 地表 → 地下 → 洞穴 → 地狱");
            ui.label("• 调整结束值会自动调整下一层级的起始值（智能对齐）");
            ui.label("• 修改会立即应用到可视化");
            
            ui.separator();
            
            // 底部按钮
            ui.horizontal(|ui| {
                if ui.button("🔄 恢复默认").clicked() {
                    reset_to_default(layers);
                    changed = true;
                }
                
                if ui.button("💾 保存配置").clicked() {
                    if let Err(e) = save_to_runtime(layers) {
                        eprintln!("保存失败: {}", e);
                    } else {
                        ui.ctx().debug_painter().text(
                            ui.ctx().screen_rect().center_top() + egui::vec2(0.0, 50.0),
                            egui::Align2::CENTER_TOP,
                            "✓ 已保存到 generation.runtime.json",
                            egui::FontId::proportional(16.0),
                            egui::Color32::GREEN,
                        );
                    }
                }
                
                if ui.button("✖ 关闭").clicked() {
                    should_close = true;
                }
            });
        });
    
    if should_close {
        *is_open = false;
    }
    
    changed
}

/// 恢复为默认层级配置
fn reset_to_default(layers: &mut [LayerDefinition]) {
    let defaults = &[
        ("space", 0, 5),
        ("surface", 5, 25),
        ("underground", 25, 35),
        ("cavern", 35, 80),
        ("hell", 80, 100),
    ];
    
    for layer in layers.iter_mut() {
        for &(key, start, end) in defaults.iter() {
            if layer.key == key {
                layer.start_percent = start;
                layer.end_percent = end;
                break;
            }
        }
    }
}

/// 保存层级配置到 generation.runtime.json
fn save_to_runtime(layers: &[LayerDefinition]) -> Result<(), Box<dyn std::error::Error>> {
    use std::collections::HashMap;
    use serde_json::json;
    
    // 构建层级配置 JSON
    let mut layers_config = HashMap::new();
    for layer in layers {
        layers_config.insert(
            layer.key.clone(),
            json!({
                "start_percent": layer.start_percent,
                "end_percent": layer.end_percent,
            })
        );
    }
    
    // 读取现有的 runtime.json 并合并 layers 字段
    let config = merge_runtime_field("layers", json!(layers_config))?;
    
    // 写入文件（格式化输出）
    let content = serde_json::to_string_pretty(&config)?;
    std::fs::write("generation.runtime.json", content)?;
    
    Ok(())
}

/// 读取 generation.runtime.json，合并一个字段，返回完整的 JSON Value
pub fn merge_runtime_field(key: &str, value: serde_json::Value) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    use std::fs;
    use serde_json::json;
    
    let runtime_path = "generation.runtime.json";
    let mut config = if let Ok(content) = fs::read_to_string(runtime_path) {
        serde_json::from_str::<serde_json::Value>(&content).unwrap_or(json!({}))
    } else {
        json!({})
    };
    
    if let Some(obj) = config.as_object_mut() {
        obj.insert(key.to_string(), value);
    }
    
    Ok(config)
}
