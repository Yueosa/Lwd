use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::Instant;

use eframe::egui;
use egui::{Color32, FontData, FontDefinitions, FontFamily, TextureHandle};

use crate::config::biome::load_biomes_config;
use crate::config::blocks::load_blocks_config;
use crate::config::world::{load_world_config, WorldConfig};
use crate::core::biome::{build_biome_definitions, get_biome_context, BiomeDefinition};
use crate::core::block::{build_block_definitions, BlockDefinition};
use crate::core::world::{World, WorldProfile};
use crate::generation::{build_pipeline, GenerationPipeline, WorldSnapshot, export_png,
    AdaptiveBatchSize, TextureUpdateThrottle};
use crate::rendering::canvas::{build_color_lut, build_color_map, world_to_color_image, world_to_color_image_downsampled};
use crate::rendering::gl_canvas::GlCanvasState;
use crate::rendering::viewport::ViewportState;
use crate::storage::engine_config::EngineConfig;
use crate::storage::runtime as app_runtime;
use crate::ui::algo_config::show_algo_config_window;
use crate::ui::canvas_view::show_canvas;
use crate::ui::control_panel::{show_control_panel, ControlAction, WorldSizeSelection};
use crate::ui::geo_preview::{show_geo_preview_window, GeoPreviewState};
use crate::ui::layer_config::show_layer_config_window;
use crate::ui::overlay_config::{show_overlay_config_window, OverlaySettings};
use crate::ui::perf_panel::show_perf_panel_window;
use crate::ui::shape_sandbox::{show_shape_sandbox_window, ShapeSandboxState};
use crate::ui::splash::show_splash;
use crate::ui::status_bar::show_status_bar;
use crate::ui::theme;

const CJK_FONT_BYTES: &[u8] = include_bytes!("../assets/fonts/NotoSansCJKsc-Regular.otf");
const SYMBOLS_FONT_BYTES: &[u8] = include_bytes!("../assets/fonts/NotoSansSymbols2-Regular.ttf");

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
    /// 是否正在后台逐帧执行（替代同步 run_all 阻塞 UI）
    running_to_end: bool,
    /// 自适应批量大小控制器（替代硬编码 STEPS_PER_FRAME）
    adaptive_batch: AdaptiveBatchSize,
    /// 智能纹理更新节流器
    texture_throttle: Option<TextureUpdateThrottle>,
    /// 引擎调优配置
    engine_config: EngineConfig,

    // ── rendering ──
    viewport: ViewportState,
    texture: Option<TextureHandle>,
    texture_dirty: bool,
    /// true = 当前纹理是降采样预览版，需要在生成结束后重建全分辨率
    texture_is_preview: bool,
    /// 当前降采样倍率（1 = 全分辨率）
    preview_factor: u32,
    /// GPU 画布渲染器共享状态
    gl_canvas: Arc<Mutex<GlCanvasState>>,

    // ── UI ──
    last_status: String,
    hover_status: String,
    overlay: OverlaySettings,
    show_overlay_config: bool,
    show_layer_config: bool,
    show_algo_config: bool,
    /// 是否显示几何预览窗口
    show_geo_preview: bool,
    /// 几何预览窗口状态
    geo_preview_state: GeoPreviewState,
    /// 图形 API 沙箱实例列表（支持多开）
    shape_sandboxes: Vec<ShapeSandboxState>,
    /// 沙箱 ID 计数器
    next_sandbox_id: usize,
    /// 是否已经开始过生成（用于控制 splash 显示）
    has_started_generation: bool,
    /// 手动种子输入框的文本内容
    seed_input: String,
    /// 是否显示性能面板
    show_perf_panel: bool,
}

impl LianWorldApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_chinese_font(&cc.egui_ctx);
        theme::apply_theme(&cc.egui_ctx);

        // 加载引擎配置（首次运行自动校准）
        let mut engine_config = EngineConfig::load();
        engine_config.ensure_calibrated();
        // 将校准后的阈值应用到全局原子变量
        crate::core::geometry::set_parallel_threshold(engine_config.parallel_pixel_threshold);

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

        let gl_canvas = Arc::new(Mutex::new(GlCanvasState::new()));

        // 从 runtime.json 恢复 UI 状态
        let (saved_size, saved_overlay) = load_runtime_ui_state();

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
            running_to_end: false,
            adaptive_batch: AdaptiveBatchSize::from_config(&engine_config),
            texture_throttle: None,
            engine_config,
            viewport: ViewportState::default(),
            texture,
            texture_dirty: false,
            texture_is_preview: false,
            preview_factor: 1,
            gl_canvas,
            last_status: "世界初始化完成".to_string(),
            hover_status: String::new(),
            overlay: saved_overlay,
            show_overlay_config: false,
            show_layer_config: false,
            show_algo_config: false,
            show_geo_preview: false,
            geo_preview_state: GeoPreviewState::default(),
            shape_sandboxes: Vec::new(),
            next_sandbox_id: 0,
            has_started_generation: false,
            seed_input: String::new(),
            show_perf_panel: false,
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
        save_runtime_ui_state(self.world_size, &self.overlay);    }

    // ── texture management ──────────────────────────────────

    /// 根据世界大小计算预览降采样倍率
    fn downsample_factor(&self) -> u32 {
        let pixels = self.world.width as u64 * self.world.height as u64;
        if pixels > 15_000_000 {
            4 // 大世界 8400×2400 = 20M → 预览 2100×600
        } else if pixels > 8_000_000 {
            2 // 中世界 6400×1800 = 11.5M → 预览 3200×900
        } else {
            1 // 小世界全分辨率
        }
    }

    fn refresh_texture_if_dirty(&mut self, ctx: &egui::Context) {
        if !self.texture_dirty {
            return;
        }

        // Bump version → GL region will re-render on next show_canvas
        self.gl_canvas.lock().unwrap().mark_world_changed();

        // Minimap texture: always use downsampled for efficiency
        let minimap_factor = self.downsample_factor().max(2); // at least 2× for minimap
        let image = world_to_color_image_downsampled(
            &self.world, &self.color_lut, minimap_factor,
        );

        if self.running_to_end && !self.pipeline.is_complete() {
            self.preview_factor = minimap_factor;
            self.texture_is_preview = true;
        } else {
            self.preview_factor = minimap_factor;
            self.texture_is_preview = false;
        }

        // Minimap texture (egui TextureHandle)
        self.texture = Some(ctx.load_texture(
            "world_texture",
            image,
            egui::TextureOptions::NEAREST,
        ));
        self.texture_dirty = false;
    }

    /// 生成完成后，如果当前纹理是预览版，强制重建全分辨率
    fn ensure_full_resolution_texture(&mut self, ctx: &egui::Context) {
        if self.texture_is_preview {
            self.texture_dirty = true;
            self.refresh_texture_if_dirty(ctx);
        }
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
            self.seed_input = format!("{new_seed:016X}");
            self.last_status = format!("已重置到第0步 (seed: {new_seed})");
        }

        // ── 手动设置种子
        if action.apply_seed {
            if let Some(new_seed) = parse_seed_input(&self.seed_input) {
                self.pipeline.set_seed(new_seed);
                self.pipeline.reset_all(&mut self.world);
                self.texture_dirty = true;
                self.last_status = format!("已应用种子: 0x{new_seed:016X}");
            } else {
                self.last_status = "种子格式无效（请输入十六进制或十进制数字）".to_string();
            }
        }

        // ── "一键生成" or "执行到底": start incremental run
        if action.run_all {
            self.running_to_end = true;
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
                        
                        // 4) 增量重新执行全部步骤
                        self.pipeline.reset_all(&mut self.world);
                        self.running_to_end = true;
                        self.texture_dirty = true;
                        self.viewport.reset();
                        self.last_status = format!(
                            "正在从存档恢复 (seed: {})…",
                            snapshot.seed
                        );
                        
                        save_runtime_ui_state(
                            self.world_size,
                            &self.overlay,
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

    // 符号字体 — 覆盖 Geometric Shapes / Dingbats / Technical / Arrows 等
    fonts.font_data.insert(
        "symbols".to_owned(),
        FontData::from_static(SYMBOLS_FONT_BYTES),
    );
    // CJK 字体 — 覆盖中文 + 日韩
    fonts.font_data.insert(
        "cjk".to_owned(),
        FontData::from_static(CJK_FONT_BYTES),
    );

    // fallback 顺序: egui 默认字体 → symbols → CJK
    // 这样 Latin/符号先从默认字体查找，找不到再试 symbols，最后是 CJK
    if let Some(family) = fonts.families.get_mut(&FontFamily::Proportional) {
        family.push("symbols".to_owned());
        family.push("cjk".to_owned());
    }
    if let Some(family) = fonts.families.get_mut(&FontFamily::Monospace) {
        family.push("symbols".to_owned());
        family.push("cjk".to_owned());
    }

    ctx.set_fonts(fonts);
}

/// 解析用户输入的种子值
/// 
/// 支持格式：
/// - 十六进制（带 0x 前缀或纯 hex 字符串）
/// - 十进制整数
fn parse_seed_input(input: &str) -> Option<u64> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return None;
    }
    // 尝试十六进制（带 0x 或 0X 前缀）
    if let Some(hex) = trimmed.strip_prefix("0x").or_else(|| trimmed.strip_prefix("0X")) {
        return u64::from_str_radix(hex, 16).ok();
    }
    // 尝试十进制
    if let Ok(v) = trimmed.parse::<u64>() {
        return Some(v);
    }
    // 尝试纯十六进制（无前缀，但包含 a-f 字符）
    u64::from_str_radix(trimmed, 16).ok()
}

/// 从 runtime.json 加载层级配置（如果存在）
fn load_runtime_layers(layers: &mut [crate::core::layer::LayerDefinition]) {
    if let Some(layers_val) = app_runtime::load_field("layers") {
        if let Some(layers_obj) = layers_val.as_object() {
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

/// 从 runtime.json 加载 UI 状态 (world_size, overlay 开关)
fn load_runtime_ui_state() -> (WorldSizeSelection, OverlaySettings) {
    let mut size = WorldSizeSelection::Small;
    let mut overlay = OverlaySettings::default();
    
    if let Some(ui_val) = app_runtime::load_field("ui") {
        if let Some(ui) = ui_val.as_object() {
            if let Some(s) = ui.get("world_size").and_then(|v| v.as_str()) {
                size = match s {
                    "medium" => WorldSizeSelection::Medium,
                    "large" => WorldSizeSelection::Large,
                    _ => WorldSizeSelection::Small,
                };
            }
            if let Some(b) = ui.get("show_biome_color").and_then(|v| v.as_bool()) {
                overlay.show_biome_color = b;
            }
            if let Some(b) = ui.get("show_biome_labels").and_then(|v| v.as_bool()) {
                overlay.show_biome_labels = b;
            }
            if let Some(b) = ui.get("show_layer_lines").and_then(|v| v.as_bool()) {
                overlay.show_layer_lines = b;
            }
            if let Some(b) = ui.get("show_layer_labels").and_then(|v| v.as_bool()) {
                overlay.show_layer_labels = b;
            }
            // 兼容旧配置
            if let Some(b) = ui.get("show_biome_overlay").and_then(|v| v.as_bool()) {
                if !ui.contains_key("show_biome_color") {
                    overlay.show_biome_color = b;
                    overlay.show_biome_labels = b;
                }
            }
            if let Some(b) = ui.get("show_layer_overlay").and_then(|v| v.as_bool()) {
                if !ui.contains_key("show_layer_lines") {
                    overlay.show_layer_lines = b;
                    overlay.show_layer_labels = b;
                }
            }
        }
    }
    
    (size, overlay)
}

/// 保存 UI 状态 (world_size, overlay 开关) 到 runtime.json
fn save_runtime_ui_state(
    world_size: WorldSizeSelection,
    overlay: &OverlaySettings,
) {
    use serde_json::json;
    
    let size_str = match world_size {
        WorldSizeSelection::Small => "small",
        WorldSizeSelection::Medium => "medium",
        WorldSizeSelection::Large => "large",
    };
    
    let ui_state = json!({
        "world_size": size_str,
        "show_biome_color": overlay.show_biome_color,
        "show_biome_labels": overlay.show_biome_labels,
        "show_layer_lines": overlay.show_layer_lines,
        "show_layer_labels": overlay.show_layer_labels,
    });
    
    let _ = app_runtime::merge_field("ui", ui_state);
}

/// 将性能分析数据持久化到日志文件
fn save_perf_log(pipeline: &GenerationPipeline, world: &World) {
    use crate::storage::perf_log::{PerfEntry, StepEntry};

    let profiler = pipeline.profiler();
    let steps: Vec<StepEntry> = profiler.all_steps_sorted().iter().map(|(idx, sp)| {
        StepEntry {
            index: *idx,
            name: sp.name.clone(),
            avg_ms: sp.avg_duration().as_secs_f64() * 1000.0,
            min_ms: sp.min_duration.as_secs_f64() * 1000.0,
            max_ms: sp.max_duration.as_secs_f64() * 1000.0,
        }
    }).collect();

    let now = chrono_timestamp();
    let entry = PerfEntry {
        timestamp: now,
        seed: format!("{:016X}", pipeline.seed()),
        world_size: format!("{}x{}", world.width, world.height),
        total_ms: profiler.total_generation_time().as_secs_f64() * 1000.0,
        steps,
    };

    // 使用 EngineConfig 中的 max_files 设置
    let cfg = EngineConfig::load();
    crate::storage::perf_log::save_entry(&entry, cfg.perf_log_max_files);
}

/// 生成 YYYYMMDD_HHMMSS 格式的时间戳
fn chrono_timestamp() -> String {
    use std::time::SystemTime;
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // 简单的 UTC 时间格式化
    let secs_per_day = 86400u64;
    let secs_per_hour = 3600u64;
    let secs_per_min = 60u64;

    let days = now / secs_per_day;
    let time_of_day = now % secs_per_day;
    let h = time_of_day / secs_per_hour;
    let m = (time_of_day % secs_per_hour) / secs_per_min;
    let s = time_of_day % secs_per_min;

    // 从天数计算年月日（简化算法）
    let (y, mo, d) = days_to_ymd(days);
    format!("{y:04}{mo:02}{d:02}_{h:02}{m:02}{s:02}")
}

fn days_to_ymd(mut days: u64) -> (u64, u64, u64) {
    // 从 1970-01-01 算起
    let mut y = 1970u64;
    loop {
        let year_days = if is_leap(y) { 366 } else { 365 };
        if days < year_days { break; }
        days -= year_days;
        y += 1;
    }
    let month_days: [u64; 12] = if is_leap(y) {
        [31,29,31,30,31,30,31,31,30,31,30,31]
    } else {
        [31,28,31,30,31,30,31,31,30,31,30,31]
    };
    let mut mo = 1u64;
    for &md in &month_days {
        if days < md { break; }
        days -= md;
        mo += 1;
    }
    (y, mo, days + 1)
}

fn is_leap(y: u64) -> bool {
    (y % 4 == 0 && y % 100 != 0) || y % 400 == 0
}

impl eframe::App for LianWorldApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // 检测当前帧是否有缩放/滚动输入——如果在生成中缩放，跳过纹理更新
        let is_zooming = ctx.input(|i| i.raw_scroll_delta.y.abs() > 0.5);

        self.apply_world_size_change();

        // 生成进行中 + 正在缩放 → 延迟纹理更新（避免帧率锯齿）
        if !(self.running_to_end && is_zooming) {
            self.refresh_texture_if_dirty(ctx);
        }

        // ── left panel ──
        // 使用 pipeline 的缓存 phase_info（仅步骤变化时重建）
        let phase_info = self.pipeline.phase_info_list().to_vec();
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
                    &mut self.seed_input,
                    &phase_info,
                    executed,
                    total,
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

        // ── geo preview ──
        if action.open_geo_preview {
            self.show_geo_preview = true;
        }

        // ── shape sandbox ──
        if action.open_shape_sandbox {
            let id = self.next_sandbox_id;
            self.next_sandbox_id += 1;
            self.shape_sandboxes.push(ShapeSandboxState::new(id));
        }

        // ── overlay config window ──
        if action.open_overlay_config {
            self.show_overlay_config = true;
        }

        if self.show_overlay_config {
            let changed = show_overlay_config_window(
                ctx,
                &mut self.show_overlay_config,
                &mut self.overlay,
            );
            if changed {
                // 切换 biome 覆盖色时重建 GL overlay 缓存
                self.gl_canvas.lock().unwrap().invalidate_biome();
                save_runtime_ui_state(self.world_size, &self.overlay);
            }
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

        // ── geo preview window ──
        if self.show_geo_preview {
            let executed = self.pipeline.executed_sub_steps();
            let step_label = if executed > 0 {
                self.pipeline.last_executed_name().unwrap_or_default()
            } else {
                "未执行".to_string()
            };
            let shapes = if executed > 0 {
                self.pipeline.shape_log(executed - 1).unwrap_or(&[])
            } else {
                &[]
            };
            show_geo_preview_window(
                ctx,
                &mut self.show_geo_preview,
                &step_label,
                shapes,
                &mut self.geo_preview_state,
                (self.world.width, self.world.height),
            );
        }

        // ── shape sandbox windows (多实例) ──
        let ws = (self.world.width, self.world.height);
        for sandbox in &mut self.shape_sandboxes {
            if sandbox.open {
                show_shape_sandbox_window(ctx, sandbox, ws);
            }
        }
        // 清理已关闭的沙箱
        self.shape_sandboxes.retain(|s| s.open);
        
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

        // ── perf panel window ──
        if action.open_perf_panel {
            self.show_perf_panel = true;
        }
        if self.show_perf_panel {
            let cfg_changed = show_perf_panel_window(
                ctx,
                &mut self.show_perf_panel,
                &mut self.engine_config,
                self.pipeline.profiler(),
            );
            if cfg_changed {
                // 应用配置到运行中的组件
                self.adaptive_batch.apply_config(&self.engine_config);
                crate::core::geometry::set_parallel_threshold(
                    self.engine_config.parallel_pixel_threshold,
                );
                self.engine_config.save();
            }
        }

        // ── dispatch actions ──
        self.handle_action(&action);

        // ── incremental execution tick ──
        // 使用自适应批量大小控制器，自动调整每帧步骤数
        if self.running_to_end && !self.pipeline.is_complete() {
            // 确保纹理节流器已初始化
            if self.texture_throttle.is_none() {
                self.texture_throttle = Some(TextureUpdateThrottle::from_config(
                    &self.engine_config, self.world.width, self.world.height,
                ));
            }

            let frame_start = Instant::now();
            let batch = self.adaptive_batch.batch_size();
            for _ in 0..batch {
                if self.pipeline.is_complete() {
                    break;
                }
                match self.pipeline.step_forward_sub(
                    &mut self.world,
                    &self.world_profile,
                    &self.blocks,
                ) {
                    Ok(_) => {}
                    Err(e) => {
                        self.running_to_end = false;
                        self.last_status = format!("生成失败: {e}");
                        break;
                    }
                }
            }
            let frame_elapsed = frame_start.elapsed();
            self.adaptive_batch.report_frame(frame_elapsed);

            // 智能纹理更新：根据世界大小和帧率自动调节
            let is_final = self.pipeline.is_complete();
            if let Some(throttle) = &mut self.texture_throttle {
                throttle.adjust_interval(self.adaptive_batch.ema_frame_ms());
                if throttle.tick(is_final) {
                    self.texture_dirty = true;
                }
            }

            let batch_ms = frame_elapsed.as_secs_f64() * 1000.0;
            self.last_status = format!(
                "正在生成… {}/{} (batch={}, {:.1}ms/帧)",
                self.pipeline.executed_sub_steps(),
                self.pipeline.total_sub_steps(),
                batch,
                batch_ms,
            );
            if self.pipeline.is_complete() {
                self.running_to_end = false;
                self.adaptive_batch.reset();
                if let Some(throttle) = &mut self.texture_throttle {
                    throttle.reset();
                }

                // 降采样预览 → 全分辨率
                self.ensure_full_resolution_texture(ctx);

                let report = self.pipeline.performance_report();
                eprintln!("{report}");

                // 持久化性能日志
                save_perf_log(&self.pipeline, &self.world);

                self.last_status = format!(
                    "全部步骤已完成 ({}/{}) — 总耗时 {:.1}ms",
                    self.pipeline.executed_sub_steps(),
                    self.pipeline.total_sub_steps(),
                    self.pipeline.profiler().total_generation_time().as_secs_f64() * 1000.0,
                );
            }
            ctx.request_repaint(); // 确保下一帧继续处理
        }

        // 缩放时跳过纹理刷新（generation complete 除外，那个强制走 ensure_full_resolution）
        if !(self.running_to_end && is_zooming) {
            self.refresh_texture_if_dirty(ctx);
        }

        // 如果 overlay 开关变化，保存 UI 状态
        if action.open_overlay_config {
            save_runtime_ui_state(self.world_size, &self.overlay);
        }

        // ── bottom bar ──
        let seed = self.pipeline.seed();
        let step_progress = match self.pipeline.current_step_display_id() {
            Some(id) => format!("Step {} ({}/{})", id, self.pipeline.executed_sub_steps(), self.pipeline.total_sub_steps()),
            None if self.pipeline.is_complete() => format!("已完成 ({0}/{0})", self.pipeline.total_sub_steps()),
            None => format!("0/{}", self.pipeline.total_sub_steps()),
        };
        let world_size_label = format!("{}×{}", self.world.width, self.world.height);
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
                show_status_bar(
                    ui, fps, mem_mb,
                    &self.last_status, &self.hover_status,
                    seed, &step_progress, &world_size_label,
                );
            });

        // ── central canvas ──
        egui::CentralPanel::default().show(ctx, |ui| {
            // 检查是否有生成操作发生（任何步进/重置/run_all 都算）
            if action.step_forward_sub || action.step_forward_phase
                || action.step_backward_sub || action.step_backward_phase
                || action.reset_and_step || action.run_all
                || action.import_lwd
            {
                self.has_started_generation = true;
            }

            if !self.has_started_generation {
                // 显示 splash 字符画
                show_splash(ui);
            } else if let Some(texture) = &self.texture {
                let biome_map = self.pipeline.biome_map();
                if let Some(hover) = show_canvas(
                    ui,
                    texture,
                    &self.world,
                    &self.color_lut,
                    &mut self.viewport,
                    biome_map,
                    &self.biomes,
                    &self.world_profile.layers,
                    self.overlay.show_biome_color,
                    self.overlay.show_biome_labels,
                    self.overlay.show_layer_lines,
                    self.overlay.show_layer_labels,
                    &self.gl_canvas,
                ) {
                    let idx = (hover.y * self.world.width + hover.x) as usize;
                    let block_id = self.world.tiles.get(idx).copied().unwrap_or(0);
                    let name = self
                        .block_names
                        .get(&block_id)
                        .map(|s| s.as_str())
                        .unwrap_or("未知");

                    // 环境 + 地层信息
                    let biome_layer = if let Some(bm) = biome_map {
                        let ctx = get_biome_context(
                            hover.x, hover.y, bm,
                            &self.world_profile.layers, self.world.height,
                        );
                        let biome_name = ctx.horizontal
                            .and_then(|id| self.biomes.iter().find(|b| b.id == id))
                            .map(|b| b.name.as_str())
                            .unwrap_or("未分配");
                        let layer_name = ctx.vertical.as_deref().unwrap_or("?");
                        format!(" | {biome_name}·{layer_name}")
                    } else {
                        String::new()
                    };

                    self.hover_status =
                        format!("{name}(ID:{block_id}) @ ({}, {}){biome_layer}", hover.x, hover.y);
                } else {
                    self.hover_status.clear();
                }
            } else {
                ui.label("画布纹理尚未初始化");
            }
        });

    }
}
