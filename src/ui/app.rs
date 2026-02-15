use std::collections::HashMap;

use eframe::egui;
use egui::{Color32, FontData, FontDefinitions, FontFamily, TextureHandle};

use crate::config::biome::load_biomes_config;
use crate::config::blocks::load_blocks_config;
use crate::config::world::{load_world_config, WorldConfig};
use crate::core::biome::{build_biome_definitions, BiomeDefinition};
use crate::core::block::{build_block_definitions, BlockDefinition};
use crate::core::world::{World, WorldProfile};
use crate::generation::{build_pipeline, GenerationPipeline, WorldSnapshot, export_png};
use crate::rendering::canvas::{build_color_lut, build_color_map, world_to_color_image};
use crate::rendering::viewport::ViewportState;
use crate::ui::algo_config::show_algo_config_window;
use crate::ui::canvas_view::show_canvas;
use crate::ui::control_panel::{show_control_panel, ControlAction, WorldSizeSelection};
use crate::ui::layer_config::{show_layer_config_window, merge_runtime_field};
use crate::ui::status_bar::show_status_bar;

const CJK_FONT_BYTES: &[u8] = include_bytes!("../assets/fonts/NotoSansCJK-Regular.ttc");

pub struct LianWorldApp {
    // ── config (loaded once) ──
    world_cfg: WorldConfig,
    blocks: Vec<BlockDefinition>,
    biomes: Vec<BiomeDefinition>,
    color_lut: [Color32; 256],
    block_names: HashMap<u8, String>,

    // ── world state ──
    world_size: WorldSizeSelection,
    world: World,
    world_profile: WorldProfile,

    // ── generation ──
    pipeline: GenerationPipeline,

    // ── rendering ──
    viewport: ViewportState,
    texture: Option<TextureHandle>,
    texture_dirty: bool,
    biome_overlay_texture: Option<TextureHandle>,

    // ── UI ──
    last_status: String,
    show_biome_overlay: bool,
    show_layer_overlay: bool,
    show_layer_config: bool,
    show_algo_config: bool,
}

impl LianWorldApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_chinese_font(&cc.egui_ctx);

        let blocks_cfg = load_blocks_config().expect("blocks.json 加载失败");
        let biomes_cfg = load_biomes_config().expect("biome.json 加载失败");
        let world_cfg = load_world_config().expect("world.json 加载失败");

        let blocks = build_block_definitions(&blocks_cfg);
        let biomes = build_biome_definitions(&biomes_cfg);
        let color_lut = build_color_lut(&build_color_map(&blocks));
        let block_names: HashMap<u8, String> =
            blocks.iter().map(|b| (b.id, b.name.clone())).collect();

        let mut world_profile =
            WorldProfile::from_config(&world_cfg, "small", None).expect("world.json 配置非法");
        
        // 尝试从 runtime.json 加载层级配置
        load_runtime_layers(&mut world_profile.layers);
        
        let world = world_profile.create_world();

        let seed = rand::random::<u64>();
        let pipeline = build_pipeline(seed, biomes.clone());

        let image = world_to_color_image(&world, &color_lut);
        let texture = Some(cc.egui_ctx.load_texture(
            "world_texture",
            image,
            egui::TextureOptions::NEAREST,
        ));

        // 从 runtime.json 恢复 UI 状态
        let (saved_size, saved_biome_ov, saved_layer_ov) = load_runtime_ui_state();

        let mut app = Self {
            world_cfg,
            blocks,
            biomes,
            color_lut,
            block_names,
            world_size: saved_size,
            world,
            world_profile,
            pipeline,
            viewport: ViewportState::default(),
            texture,
            texture_dirty: false,
            biome_overlay_texture: None,
            last_status: "世界初始化完成".to_string(),
            show_biome_overlay: saved_biome_ov,
            show_layer_overlay: saved_layer_ov,
            show_layer_config: false,
            show_algo_config: false,
        };

        // 根据恢复的 world_size 切换
        app.apply_world_size_change();
        app.refresh_texture_if_dirty(&cc.egui_ctx);

        app
    }

    // ── world size change ───────────────────────────────────

    fn world_size_key(&self) -> &'static str {
        match self.world_size {
            WorldSizeSelection::Small => "small",
            WorldSizeSelection::Medium => "medium",
            WorldSizeSelection::Large => "large",
        }
    }

    fn apply_world_size_change(&mut self) {
        let target = self.world_size_key();

        if self.world_profile.size.key == target {
            return;
        }

        self.world_profile = WorldProfile::from_config(&self.world_cfg, target, None)
            .expect("world size 配置非法");
        // 重新加载 runtime.json 中的层级配置，避免切换尺寸后丢失
        load_runtime_layers(&mut self.world_profile.layers);
        self.world = self.world_profile.create_world();
        self.pipeline.reset_all(&mut self.world);
        self.viewport.reset();
        self.texture_dirty = true;
        self.last_status = format!(
            "已切换: {} ({}×{})",
            self.world_profile.size.description, self.world.width, self.world.height
        );        // 保存 UI 状态
        save_runtime_ui_state(self.world_size, self.show_biome_overlay, self.show_layer_overlay);    }

    // ── texture management ──────────────────────────────────

    fn refresh_texture_if_dirty(&mut self, ctx: &egui::Context) {
        if !self.texture_dirty {
            return;
        }
        let image = world_to_color_image(&self.world, &self.color_lut);
        self.texture = Some(ctx.load_texture(
            "world_texture",
            image,
            egui::TextureOptions::NEAREST,
        ));
        // 同时失效 biome overlay（每次步骤执行后重建）
        self.biome_overlay_texture = None;
        self.texture_dirty = false;
    }

    // ── action dispatch ─────────────────────────────────────

    fn handle_action(&mut self, action: &ControlAction) {
        if action.zoom_in {
            self.viewport.zoom_in();
        }
        if action.zoom_out {
            self.viewport.zoom_out();
        }
        if action.zoom_reset {
            self.viewport.reset();
        }

        if action.step_forward_sub {
            match self.pipeline.step_forward_sub(
                &mut self.world,
                &self.world_profile,
                &self.blocks,
            ) {
                Ok(true) => {
                    self.texture_dirty = true;
                    if let Some(name) = self.pipeline.last_executed_name() {
                        self.last_status = format!("已执行: {name}");
                    }
                }
                Ok(false) => {
                    self.last_status = "所有步骤已完成".to_string();
                }
                Err(e) => {
                    self.last_status = format!("步骤失败: {e}");
                }
            }
        }

        if action.step_forward_phase {
            match self.pipeline.step_forward_phase(
                &mut self.world,
                &self.world_profile,
                &self.blocks,
            ) {
                Ok(true) => {
                    self.texture_dirty = true;
                    if let Some(name) = self.pipeline.last_executed_name() {
                        self.last_status = format!("阶段完成: {name}");
                    }
                }
                Ok(false) => {
                    self.last_status = "所有步骤已完成".to_string();
                }
                Err(e) => {
                    self.last_status = format!("步骤失败: {e}");
                }
            }
        }

        if action.step_backward_sub {
            match self.pipeline.step_backward_sub(
                &mut self.world,
                &self.world_profile,
                &self.blocks,
            ) {
                Ok(true) => {
                    self.texture_dirty = true;
                    self.last_status = format!(
                        "已回退至子步骤 {}/{}",
                        self.pipeline.executed_sub_steps(),
                        self.pipeline.total_sub_steps()
                    );
                }
                Ok(false) => {
                    self.last_status = "已在起始状态".to_string();
                }
                Err(e) => {
                    self.last_status = format!("回退失败: {e}");
                }
            }
        }

        if action.step_backward_phase {
            match self.pipeline.step_backward_phase(
                &mut self.world,
                &self.world_profile,
                &self.blocks,
            ) {
                Ok(true) => {
                    self.texture_dirty = true;
                    self.last_status = format!(
                        "已回退至子步骤 {}/{}",
                        self.pipeline.executed_sub_steps(),
                        self.pipeline.total_sub_steps()
                    );
                }
                Ok(false) => {
                    self.last_status = "已在起始状态".to_string();
                }
                Err(e) => {
                    self.last_status = format!("回退失败: {e}");
                }
            }
        }

        // ── "重新初始化" = new seed + reset to step 0
        if action.reset_and_step {
            let new_seed = rand::random::<u64>();
            self.pipeline.set_seed(new_seed);
            self.pipeline.reset_all(&mut self.world);
            self.texture_dirty = true;
            self.last_status = format!("已重置到第0步 (seed: {new_seed})");
        }

        // ── "一键生成" or "执行到底": run remaining steps
        if action.run_all {
            match self.pipeline.run_all(
                &mut self.world,
                &self.world_profile,
                &self.blocks,
            ) {
                Ok(()) => {
                    self.texture_dirty = true;
                    self.last_status = format!(
                        "全部步骤已完成 ({}/{})",
                        self.pipeline.executed_sub_steps(),
                        self.pipeline.total_sub_steps()
                    );
                }
                Err(e) => {
                    self.texture_dirty = true;
                    self.last_status = format!("生成失败: {e}");
                }
            }
        }

        // ── 导出 PNG
        if action.export_png {
            let dialog = rfd::FileDialog::new()
                .set_title("导出 PNG")
                .set_file_name("world_export.png")
                .add_filter("PNG 图片", &["png"]);
            if let Some(path) = dialog.save_file() {
                match export_png(&self.world, &self.color_lut, &path) {
                    Ok(()) => {
                        self.last_status = format!("PNG 已导出: {}", path.display());
                    }
                    Err(e) => {
                        self.last_status = format!("PNG 导出失败: {e}");
                    }
                }
            }
        }

        // ── 导出 .lwd
        if action.export_lwd {
            let snapshot = self.pipeline.collect_snapshot(
                self.world_size_key(),
                &self.world_profile.layers,
            );
            let dialog = rfd::FileDialog::new()
                .set_title("导出世界存档")
                .set_file_name("world_export.lwd")
                .add_filter("Lian World 存档", &["lwd"]);
            if let Some(path) = dialog.save_file() {
                match snapshot.save_lwd(&path) {
                    Ok(()) => {
                        self.last_status = format!("存档已导出: {}", path.display());
                    }
                    Err(e) => {
                        self.last_status = format!("存档导出失败: {e}");
                    }
                }
            }
        }

        // ── 导入 .lwd
        if action.import_lwd {
            let dialog = rfd::FileDialog::new()
                .set_title("导入世界存档")
                .add_filter("Lian World 存档", &["lwd"]);
            if let Some(path) = dialog.pick_file() {
                match WorldSnapshot::load_lwd(&path) {
                    Ok(snapshot) => {
                        // 1) 恢复世界尺寸
                        self.world_size = match snapshot.world_size.as_str() {
                            "medium" => WorldSizeSelection::Medium,
                            "large" => WorldSizeSelection::Large,
                            _ => WorldSizeSelection::Small,
                        };
                        self.world_profile = WorldProfile::from_config(
                            &self.world_cfg,
                            &snapshot.world_size,
                            None,
                        )
                        .expect("world size 配置非法");
                        
                        // 2) 恢复层级配置
                        for layer in &mut self.world_profile.layers {
                            if let Some(ov) = snapshot.layers.get(&layer.key) {
                                layer.start_percent = ov.start_percent;
                                layer.end_percent = ov.end_percent;
                            }
                        }
                        
                        self.world = self.world_profile.create_world();
                        
                        // 3) 恢复种子 + 算法参数
                        self.pipeline.set_seed(snapshot.seed);
                        self.pipeline.restore_from_snapshot(&snapshot);
                        
                        // 4) 重新执行全部步骤
                        self.pipeline.reset_all(&mut self.world);
                        match self.pipeline.run_all(
                            &mut self.world,
                            &self.world_profile,
                            &self.blocks,
                        ) {
                            Ok(()) => {
                                self.texture_dirty = true;
                                self.viewport.reset();
                                self.last_status = format!(
                                    "已从存档恢复 (seed: {})",
                                    snapshot.seed
                                );
                            }
                            Err(e) => {
                                self.texture_dirty = true;
                                self.last_status = format!("存档恢复执行失败: {e}");
                            }
                        }
                        
                        save_runtime_ui_state(
                            self.world_size,
                            self.show_biome_overlay,
                            self.show_layer_overlay,
                        );
                    }
                    Err(e) => {
                        self.last_status = format!("存档导入失败: {e}");
                    }
                }
            }
        }
    }
}

fn setup_chinese_font(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    fonts
        .font_data
        .insert("cjk".to_owned(), FontData::from_static(CJK_FONT_BYTES));

    if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
        family.insert(0, "cjk".to_owned());
    }
    if let Some(family) = fonts.families.get_mut(&FontFamily::Monospace) {
        family.insert(0, "cjk".to_owned());
    }

    ctx.set_fonts(fonts);
}

/// 从 generation.runtime.json 加载层级配置（如果存在）
fn load_runtime_layers(layers: &mut [crate::core::layer::LayerDefinition]) {
    use std::fs;
    
    let runtime_path = "generation.runtime.json";
    if let Ok(content) = fs::read_to_string(runtime_path) {
        if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(layers_obj) = config.get("layers").and_then(|v| v.as_object()) {
                for layer in layers.iter_mut() {
                    if let Some(layer_config) = layers_obj.get(&layer.key).and_then(|v| v.as_object()) {
                        if let Some(start) = layer_config.get("start_percent").and_then(|v| v.as_u64()) {
                            layer.start_percent = start as u8;
                        }
                        if let Some(end) = layer_config.get("end_percent").and_then(|v| v.as_u64()) {
                            layer.end_percent = end as u8;
                        }
                    }
                }
            }
        }
    }
}

/// 从 runtime.json 加载 UI 状态 (world_size, overlay 开关)
fn load_runtime_ui_state() -> (WorldSizeSelection, bool, bool) {
    use std::fs;
    
    let mut size = WorldSizeSelection::Small;
    let mut biome_ov = false;
    let mut layer_ov = true;
    
    let runtime_path = "generation.runtime.json";
    if let Ok(content) = fs::read_to_string(runtime_path) {
        if let Ok(config) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(ui) = config.get("ui").and_then(|v| v.as_object()) {
                if let Some(s) = ui.get("world_size").and_then(|v| v.as_str()) {
                    size = match s {
                        "medium" => WorldSizeSelection::Medium,
                        "large" => WorldSizeSelection::Large,
                        _ => WorldSizeSelection::Small,
                    };
                }
                if let Some(b) = ui.get("show_biome_overlay").and_then(|v| v.as_bool()) {
                    biome_ov = b;
                }
                if let Some(l) = ui.get("show_layer_overlay").and_then(|v| v.as_bool()) {
                    layer_ov = l;
                }
            }
        }
    }
    
    (size, biome_ov, layer_ov)
}

/// 保存 UI 状态 (world_size, overlay 开关) 到 runtime.json
fn save_runtime_ui_state(
    world_size: WorldSizeSelection,
    show_biome_overlay: bool,
    show_layer_overlay: bool,
) {
    use serde_json::json;
    
    let size_str = match world_size {
        WorldSizeSelection::Small => "small",
        WorldSizeSelection::Medium => "medium",
        WorldSizeSelection::Large => "large",
    };
    
    let ui_state = json!({
        "world_size": size_str,
        "show_biome_overlay": show_biome_overlay,
        "show_layer_overlay": show_layer_overlay,
    });
    
    if let Ok(config) = merge_runtime_field("ui", ui_state) {
        let content = serde_json::to_string_pretty(&config).unwrap_or_default();
        let _ = std::fs::write("generation.runtime.json", content);
    }
}

impl eframe::App for LianWorldApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.apply_world_size_change();
        self.refresh_texture_if_dirty(ctx);

        // ── left panel ──
        let phase_info = self.pipeline.phase_info_list();
        let executed = self.pipeline.executed_sub_steps();
        let total = self.pipeline.total_sub_steps();
        let mut action = ControlAction::none();

        egui::SidePanel::left("control_panel")
            .resizable(true)
            .default_width(260.0)
            .show(ctx, |ui| {
                action = show_control_panel(
                    ui,
                    &mut self.world_size,
                    &phase_info,
                    executed,
                    total,
                    &mut self.show_biome_overlay,
                    &mut self.show_layer_overlay,
                );
                ui.separator();
                ui.label(format!("缩放: {:.0}%", self.viewport.zoom * 100.0));
                ui.label(format!("方块数: {}", self.blocks.len()));
                ui.label(format!(
                    "尺寸: {} × {}",
                    self.world.width, self.world.height
                ));
            });

        // ── algo config window ──
        if action.open_step_config {
            self.show_algo_config = true;
        }

        if self.show_algo_config {
            if let Some((_idx, algo)) = self.pipeline.current_algorithm_mut() {
                let result = show_algo_config_window(
                    ctx,
                    &mut self.show_algo_config,
                    algo,
                );
                if result.replay_requested {
                    // 回退到当前阶段开头，然后重新执行到当前位置
                    let target = self.pipeline.executed_sub_steps();
                    if target > 0 {
                        match self.pipeline.step_backward_phase(
                            &mut self.world,
                            &self.world_profile,
                            &self.blocks,
                        ) {
                            Ok(true) => {
                                // 从阶段开头重新执行到之前的位置
                                let current = self.pipeline.executed_sub_steps();
                                for _ in current..target {
                                    if let Err(e) = self.pipeline.step_forward_sub(
                                        &mut self.world,
                                        &self.world_profile,
                                        &self.blocks,
                                    ) {
                                        self.last_status = format!("重新执行失败: {e}");
                                        break;
                                    }
                                }
                                self.texture_dirty = true;
                                self.last_status = "已用新参数重新执行当前阶段".to_string();
                            }
                            Ok(false) => {}
                            Err(e) => {
                                self.last_status = format!("回退失败: {e}");
                            }
                        }
                    }
                }
            }
        }

        // ── layer config window ──
        if action.open_layer_config {
            self.show_layer_config = true;
        }
        
        if self.show_layer_config {
            let changed = show_layer_config_window(
                ctx,
                &mut self.show_layer_config,
                &mut self.world_profile.layers,
                self.world.height,
            );
            
            // 如果层级配置改变，刷新纹理（虽然现在只影响可视化，但保持一致性）
            if changed {
                // 可以在这里触发重新生成或只是更新状态
                self.last_status = "层级配置已更新".to_string();
            }
        }

        // ── dispatch actions ──
        self.handle_action(&action);
        self.refresh_texture_if_dirty(ctx);

        // 如果 overlay 开关变化，保存 UI 状态
        if action.biome_overlay_toggled || action.layer_overlay_toggled {
            save_runtime_ui_state(self.world_size, self.show_biome_overlay, self.show_layer_overlay);
        }

        // ── bottom bar ──
        egui::TopBottomPanel::bottom("status_bar")
            .resizable(false)
            .min_height(28.0)
            .show(ctx, |ui| {
                let fps = ctx.input(|i| {
                    if i.stable_dt > 0.0 {
                        1.0 / i.stable_dt
                    } else {
                        0.0
                    }
                });
                let mem_mb = ((self.world.width as usize * self.world.height as usize * 4)
                    / (1024 * 1024))
                    .max(1);
                show_status_bar(ui, fps, mem_mb, &self.last_status);
            });

        // ── central canvas ──
        egui::CentralPanel::default().show(ctx, |ui| {
            if let Some(texture) = &self.texture {
                let biome_map = self.pipeline.biome_map();
                if let Some(hover) = show_canvas(
                    ui,
                    texture,
                    self.world.width,
                    self.world.height,
                    &mut self.viewport,
                    biome_map,
                    &self.biomes,
                    &self.world_profile.layers,
                    self.show_biome_overlay,
                    self.show_layer_overlay,
                    &mut self.biome_overlay_texture,
                ) {
                    let idx = (hover.y * self.world.width + hover.x) as usize;
                    let block_id = self.world.tiles.get(idx).copied().unwrap_or(0);
                    let name = self
                        .block_names
                        .get(&block_id)
                        .map(|s| s.as_str())
                        .unwrap_or("未知");
                    self.last_status =
                        format!("{name}(ID:{block_id}) @ ({}, {})", hover.x, hover.y);
                }
            } else {
                ui.label("画布纹理尚未初始化");
            }
        });
    }
}
