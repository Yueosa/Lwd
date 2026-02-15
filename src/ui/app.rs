use std::collections::HashMap;

use eframe::egui;
use egui::{Color32, TextureHandle};

use crate::config::blocks::load_blocks_config;
use crate::config::world::load_world_config;
use crate::core::block::{build_block_definitions, BlockDefinition};
use crate::core::world::{World, WorldProfile};
use crate::rendering::canvas::{build_color_map, world_to_color_image};
use crate::rendering::viewport::ViewportState;
use crate::ui::canvas_view::show_canvas;
use crate::ui::control_panel::{show_control_panel, WorldSizeSelection};
use crate::ui::status_bar::show_status_bar;

pub struct LianWorldApp {
    world_size: WorldSizeSelection,
    blocks: Vec<BlockDefinition>,
    world: World,
    world_profile: WorldProfile,
    viewport: ViewportState,
    texture: Option<TextureHandle>,
    colors: HashMap<u8, Color32>,
    last_status: String,
}

impl LianWorldApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let blocks_cfg = load_blocks_config().expect("blocks.json 加载失败");
        let world_cfg = load_world_config().expect("world.json 加载失败");

        let blocks = build_block_definitions(&blocks_cfg);
        let world_profile = WorldProfile::from_config(&world_cfg, "small", None)
            .expect("world.json 配置非法");
        let world = world_profile.create_world();

        let colors = build_color_map(&blocks);
        let image = world_to_color_image(&world, &colors);
        let texture = Some(cc.egui_ctx.load_texture(
            "world_texture",
            image,
            egui::TextureOptions::NEAREST,
        ));

        Self {
            world_size: WorldSizeSelection::Small,
            blocks,
            world,
            world_profile,
            viewport: ViewportState::default(),
            texture,
            colors,
            last_status: "世界初始化完成（全空气）".to_string(),
        }
    }

    fn regenerate_world_if_needed(&mut self, ctx: &egui::Context) {
        let target = match self.world_size {
            WorldSizeSelection::Small => "small",
            WorldSizeSelection::Medium => "medium",
            WorldSizeSelection::Large => "large",
        };

        if self.world_profile.size.key == target {
            return;
        }

        let world_cfg = load_world_config().expect("world.json 加载失败");
        self.world_profile = WorldProfile::from_config(&world_cfg, target, None)
            .expect("world size 配置非法");
        self.world = self.world_profile.create_world();
        self.viewport.reset();

        let image = world_to_color_image(&self.world, &self.colors);
        self.texture = Some(ctx.load_texture("world_texture", image, egui::TextureOptions::NEAREST));
        self.last_status = format!(
            "已切换世界: {} ({}x{})",
            self.world_profile.size.description, self.world.width, self.world.height
        );
    }
}

impl eframe::App for LianWorldApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.regenerate_world_if_needed(ctx);

        egui::SidePanel::left("control_panel")
            .resizable(true)
            .default_width(260.0)
            .show(ctx, |ui| {
                let action = show_control_panel(ui, &mut self.world_size, 1, 60);
                if action.zoom_in {
                    self.viewport.zoom_in();
                }
                if action.zoom_out {
                    self.viewport.zoom_out();
                }
                if action.zoom_reset {
                    self.viewport.reset();
                }
                ui.separator();
                ui.label(format!("缩放: {:.0}%", self.viewport.zoom * 100.0));
                ui.label(format!("方块数: {}", self.blocks.len()));
                ui.label(format!("尺寸: {} x {}", self.world.width, self.world.height));
            });

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
                let memory_hint_mb = ((self.world.width as usize * self.world.height as usize * 4)
                    / (1024 * 1024))
                    .max(1);
                show_status_bar(ui, fps, memory_hint_mb, &self.last_status);
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("世界画布");
            ui.label("拖拽平移，滚轮缩放；当前画布为全空气初始化结果");
            ui.separator();

            if let Some(texture) = &self.texture {
                if let Some(hover) = show_canvas(
                    ui,
                    texture,
                    self.world.width,
                    self.world.height,
                    &mut self.viewport,
                ) {
                    self.last_status = format!("悬停: 空气(ID:1) @ ({}, {})", hover.x, hover.y);
                }
            } else {
                ui.label("画布纹理尚未初始化");
            }
        });
    }
}
