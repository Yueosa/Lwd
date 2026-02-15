use std::collections::HashMap;

use eframe::egui;
use egui::{Color32, FontData, FontDefinitions, FontFamily, TextureHandle};

use crate::config::blocks::load_blocks_config;
use crate::config::world::{load_world_config, WorldConfig};
use crate::core::block::{build_block_definitions, BlockDefinition};
use crate::core::world::{World, WorldProfile};
use crate::generation::{build_default_pipeline, GenerationPipeline};
use crate::rendering::canvas::{build_color_lut, build_color_map, world_to_color_image};
use crate::rendering::viewport::ViewportState;
use crate::ui::canvas_view::show_canvas;
use crate::ui::control_panel::{show_control_panel, ControlAction, WorldSizeSelection};
use crate::ui::status_bar::show_status_bar;

const CJK_FONT_BYTES: &[u8] = include_bytes!("../assets/fonts/NotoSansCJK-Regular.ttc");

pub struct LianWorldApp {
    // ── config (loaded once) ──
    world_cfg: WorldConfig,
    blocks: Vec<BlockDefinition>,
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

    // ── UI ──
    last_status: String,
}

impl LianWorldApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_chinese_font(&cc.egui_ctx);

        let blocks_cfg = load_blocks_config().expect("blocks.json 加载失败");
        let world_cfg = load_world_config().expect("world.json 加载失败");

        let blocks = build_block_definitions(&blocks_cfg);
        let color_lut = build_color_lut(&build_color_map(&blocks));
        let block_names: HashMap<u8, String> =
            blocks.iter().map(|b| (b.id, b.name.clone())).collect();

        let world_profile =
            WorldProfile::from_config(&world_cfg, "small", None).expect("world.json 配置非法");
        let world = world_profile.create_world();

        let seed = rand::random::<u64>();
        let pipeline = build_default_pipeline(seed);

        let image = world_to_color_image(&world, &color_lut);
        let texture = Some(cc.egui_ctx.load_texture(
            "world_texture",
            image,
            egui::TextureOptions::NEAREST,
        ));

        Self {
            world_cfg,
            blocks,
            color_lut,
            block_names,
            world_size: WorldSizeSelection::Small,
            world,
            world_profile,
            pipeline,
            viewport: ViewportState::default(),
            texture,
            texture_dirty: false,
            last_status: "世界初始化完成".to_string(),
        }
    }

    // ── world size change ───────────────────────────────────

    fn apply_world_size_change(&mut self) {
        let target = match self.world_size {
            WorldSizeSelection::Small => "small",
            WorldSizeSelection::Medium => "medium",
            WorldSizeSelection::Large => "large",
        };

        if self.world_profile.size.key == target {
            return;
        }

        self.world_profile = WorldProfile::from_config(&self.world_cfg, target, None)
            .expect("world size 配置非法");
        self.world = self.world_profile.create_world();
        self.pipeline.reset_all(&mut self.world);
        self.viewport.reset();
        self.texture_dirty = true;
        self.last_status = format!(
            "已切换: {} ({}×{})",
            self.world_profile.size.description, self.world.width, self.world.height
        );
    }

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

        if action.step_forward {
            match self.pipeline.step_forward(
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

        if action.step_backward {
            match self.pipeline.step_backward(
                &mut self.world,
                &self.world_profile,
                &self.blocks,
            ) {
                Ok(true) => {
                    self.texture_dirty = true;
                    self.last_status = format!(
                        "已回退至步骤 {}/{}",
                        self.pipeline.executed_count(),
                        self.pipeline.total_steps()
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
                        self.pipeline.executed_count(),
                        self.pipeline.total_steps()
                    );
                }
                Err(e) => {
                    self.texture_dirty = true;
                    self.last_status = format!("生成失败: {e}");
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

impl eframe::App for LianWorldApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.apply_world_size_change();
        self.refresh_texture_if_dirty(ctx);

        // ── left panel ──
        let step_info = self.pipeline.step_info_list();
        let executed = self.pipeline.executed_count();
        let total = self.pipeline.total_steps();
        let mut action = ControlAction::none();

        egui::SidePanel::left("control_panel")
            .resizable(true)
            .default_width(260.0)
            .show(ctx, |ui| {
                action =
                    show_control_panel(ui, &mut self.world_size, &step_info, executed, total);
                ui.separator();
                ui.label(format!("缩放: {:.0}%", self.viewport.zoom * 100.0));
                ui.label(format!("方块数: {}", self.blocks.len()));
                ui.label(format!(
                    "尺寸: {} × {}",
                    self.world.width, self.world.height
                ));
            });

        // ── dispatch actions ──
        self.handle_action(&action);
        self.refresh_texture_if_dirty(ctx);

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
                if let Some(hover) = show_canvas(
                    ui,
                    texture,
                    self.world.width,
                    self.world.height,
                    &mut self.viewport,
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
