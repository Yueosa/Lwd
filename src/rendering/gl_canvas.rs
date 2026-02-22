//! GPU-accelerated canvas renderer using glow (OpenGL).
//!
//! Renders checkerboard background + world texture + biome overlay in a
//! single [`egui::PaintCallback`], bypassing egui's tessellation pipeline.
//!
//! This eliminates the per-frame CPU cost of hundreds of `painter.rect_filled()`
//! calls (checkerboard) and large-texture `painter.image()` passes that go
//! through egui's Shape → tessellation → vertex upload path.

use std::sync::{Arc, Mutex};

use egui::Color32;
use glow::HasContext as _;

// ─── Shader sources ─────────────────────────────────────────────────────

const VERT_SRC: &str = r#"#version 140

in vec2 a_pos;
out vec2 v_uv;

void main() {
    // Map [-1, 1] NDC to [0, 1] UV with y=0 at screen top
    v_uv = vec2(a_pos.x * 0.5 + 0.5, 0.5 - a_pos.y * 0.5);
    gl_Position = vec4(a_pos, 0.0, 1.0);
}
"#;

const FRAG_SRC: &str = r#"#version 140

in vec2 v_uv;
out vec4 frag_color;

uniform vec2 u_viewport_size;     // viewport in physical pixels
uniform vec4 u_world_rect;        // [left, top, right, bottom] normalised [0,1]
uniform float u_checker_tile;     // checkerboard tile in physical pixels
uniform float u_has_world;        // 1.0 = world texture ready
uniform float u_has_biome;        // 1.0 = biome overlay ready
uniform sampler2D u_world_tex;    // texture unit 0
uniform sampler2D u_biome_tex;    // texture unit 1

void main() {
    // ── Checkerboard background ──
    vec2 px = v_uv * u_viewport_size;
    float checker = mod(
        floor(px.x / u_checker_tile) + floor(px.y / u_checker_tile),
        2.0
    );
    // gray(28) = 0.10980, gray(35) = 0.13725
    vec3 bg = mix(vec3(0.10980), vec3(0.13725), checker);
    frag_color = vec4(bg, 1.0);

    // ── World texture (alpha-blend over checkerboard) ──
    if (u_has_world > 0.5) {
        vec2 span = u_world_rect.zw - u_world_rect.xy;
        vec2 wuv = (v_uv - u_world_rect.xy) / max(span, vec2(0.0001));
        if (wuv.x >= 0.0 && wuv.x <= 1.0 && wuv.y >= 0.0 && wuv.y <= 1.0) {
            vec4 world = texture(u_world_tex, wuv);
            frag_color = vec4(mix(frag_color.rgb, world.rgb, world.a), 1.0);
        }
    }

    // ── Biome overlay (alpha blend) ──
    if (u_has_biome > 0.5) {
        vec2 span = u_world_rect.zw - u_world_rect.xy;
        vec2 wuv = (v_uv - u_world_rect.xy) / max(span, vec2(0.0001));
        if (wuv.x >= 0.0 && wuv.x <= 1.0 && wuv.y >= 0.0 && wuv.y <= 1.0) {
            vec4 ov = texture(u_biome_tex, wuv);
            frag_color = vec4(mix(frag_color.rgb, ov.rgb, ov.a), 1.0);
        }
    }
}
"#;

// ─── GL resource bundle ─────────────────────────────────────────────────

struct GlResources {
    program: glow::Program,
    vao: glow::VertexArray,
    vbo: glow::Buffer,
    world_tex: glow::Texture,
    biome_tex: glow::Texture,
    // uniform locations
    loc_viewport_size: Option<glow::UniformLocation>,
    loc_world_rect: Option<glow::UniformLocation>,
    loc_checker_tile: Option<glow::UniformLocation>,
    loc_has_world: Option<glow::UniformLocation>,
    loc_has_biome: Option<glow::UniformLocation>,
    loc_world_tex: Option<glow::UniformLocation>,
    loc_biome_tex: Option<glow::UniformLocation>,
}

struct PendingTexture {
    rgba: Vec<u8>,
    width: u32,
    height: u32,
}

// ─── Public shared state ────────────────────────────────────────────────

/// Shared state for the GPU canvas renderer.
///
/// Wrap in `Arc<Mutex<GlCanvasState>>` and share between the app logic
/// (which pushes pixel data) and the [`egui::PaintCallback`] (which renders).
pub struct GlCanvasState {
    resources: Option<GlResources>,
    world_data: Option<PendingTexture>,
    biome_data: Option<PendingTexture>,
    world_dirty: bool,
    biome_dirty: bool,
    biome_valid: bool,
    has_world: bool,
    has_biome: bool,
}

impl GlCanvasState {
    pub fn new() -> Self {
        Self {
            resources: None,
            world_data: None,
            biome_data: None,
            world_dirty: false,
            biome_dirty: false,
            biome_valid: false,
            has_world: false,
            has_biome: false,
        }
    }

    /// Store new world texture pixel data (RGBA `u8`).
    /// Invalidates the biome overlay as well.
    pub fn set_world_pixels(&mut self, rgba: Vec<u8>, width: u32, height: u32) {
        self.world_data = Some(PendingTexture { rgba, width, height });
        self.world_dirty = true;
        self.has_world = true;
        // World changed → biome overlay is stale
        self.biome_valid = false;
        self.has_biome = false;
    }

    /// Store new biome overlay pixel data (RGBA `u8`).
    pub fn set_biome_pixels(&mut self, rgba: Vec<u8>, width: u32, height: u32) {
        self.biome_data = Some(PendingTexture { rgba, width, height });
        self.biome_dirty = true;
        self.biome_valid = true;
        self.has_biome = true;
    }

    /// Whether the biome overlay data needs regeneration from the biome map.
    pub fn needs_biome_regen(&self) -> bool {
        !self.biome_valid
    }

    /// Whether valid biome overlay data is ready for rendering.
    pub fn has_biome_ready(&self) -> bool {
        self.has_biome
    }

    /// Explicitly invalidate biome overlay (e.g. when biome map changes).
    #[allow(dead_code)]
    pub fn invalidate_biome(&mut self) {
        self.biome_valid = false;
        self.has_biome = false;
    }

    /// Release GL resources.  Must be called with a current GL context.
    #[allow(dead_code)]
    pub fn destroy(&mut self, gl: &glow::Context) {
        if let Some(res) = self.resources.take() {
            unsafe {
                gl.delete_program(res.program);
                gl.delete_vertex_array(res.vao);
                gl.delete_buffer(res.vbo);
                gl.delete_texture(res.world_tex);
                gl.delete_texture(res.biome_tex);
            }
        }
    }
}

// ─── GL helpers ─────────────────────────────────────────────────────────

fn compile_shader(gl: &glow::Context, kind: u32, source: &str) -> glow::Shader {
    unsafe {
        let shader = gl.create_shader(kind).expect("GL: 创建着色器失败");
        gl.shader_source(shader, source);
        gl.compile_shader(shader);
        if !gl.get_shader_compile_status(shader) {
            let log = gl.get_shader_info_log(shader);
            panic!("GL: 着色器编译失败:\n{log}");
        }
        shader
    }
}

fn init_resources(gl: &glow::Context) -> GlResources {
    unsafe {
        // ── compile & link ──
        let vert = compile_shader(gl, glow::VERTEX_SHADER, VERT_SRC);
        let frag = compile_shader(gl, glow::FRAGMENT_SHADER, FRAG_SRC);

        let program = gl.create_program().expect("GL: 创建程序失败");
        gl.attach_shader(program, vert);
        gl.attach_shader(program, frag);
        gl.link_program(program);
        if !gl.get_program_link_status(program) {
            let log = gl.get_program_info_log(program);
            panic!("GL: 着色器链接失败:\n{log}");
        }
        gl.detach_shader(program, vert);
        gl.detach_shader(program, frag);
        gl.delete_shader(vert);
        gl.delete_shader(frag);

        // ── fullscreen quad (triangle strip) ──
        let vertices: [f32; 8] = [
            -1.0, -1.0,
             1.0, -1.0,
            -1.0,  1.0,
             1.0,  1.0,
        ];
        let vbo = gl.create_buffer().expect("GL: 创建 VBO 失败");
        gl.bind_buffer(glow::ARRAY_BUFFER, Some(vbo));
        let vertex_bytes: &[u8] = core::slice::from_raw_parts(
            vertices.as_ptr() as *const u8,
            core::mem::size_of_val(&vertices),
        );
        gl.buffer_data_u8_slice(glow::ARRAY_BUFFER, vertex_bytes, glow::STATIC_DRAW);

        let vao = gl.create_vertex_array().expect("GL: 创建 VAO 失败");
        gl.bind_vertex_array(Some(vao));

        let a_pos = gl
            .get_attrib_location(program, "a_pos")
            .expect("GL: 找不到 a_pos 属性");
        gl.enable_vertex_attrib_array(a_pos);
        gl.vertex_attrib_pointer_f32(a_pos, 2, glow::FLOAT, false, 8, 0);

        gl.bind_vertex_array(None);
        gl.bind_buffer(glow::ARRAY_BUFFER, None);

        // ── placeholder textures ──
        let world_tex = create_empty_texture(gl);
        let biome_tex = create_empty_texture(gl);

        // ── uniform locations ──
        let loc = |name: &str| gl.get_uniform_location(program, name);

        GlResources {
            program,
            vao,
            vbo,
            world_tex,
            biome_tex,
            loc_viewport_size: loc("u_viewport_size"),
            loc_world_rect: loc("u_world_rect"),
            loc_checker_tile: loc("u_checker_tile"),
            loc_has_world: loc("u_has_world"),
            loc_has_biome: loc("u_has_biome"),
            loc_world_tex: loc("u_world_tex"),
            loc_biome_tex: loc("u_biome_tex"),
        }
    }
}

fn create_empty_texture(gl: &glow::Context) -> glow::Texture {
    unsafe {
        let tex = gl.create_texture().expect("GL: 创建纹理失败");
        gl.bind_texture(glow::TEXTURE_2D, Some(tex));
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MIN_FILTER, glow::NEAREST as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_MAG_FILTER, glow::NEAREST as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_S, glow::CLAMP_TO_EDGE as i32);
        gl.tex_parameter_i32(glow::TEXTURE_2D, glow::TEXTURE_WRAP_T, glow::CLAMP_TO_EDGE as i32);
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA as i32,
            1,
            1,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            Some(&[0u8; 4]),
        );
        gl.bind_texture(glow::TEXTURE_2D, None);
        tex
    }
}

fn upload_texture(gl: &glow::Context, tex: glow::Texture, data: &PendingTexture) {
    unsafe {
        gl.bind_texture(glow::TEXTURE_2D, Some(tex));
        gl.tex_image_2d(
            glow::TEXTURE_2D,
            0,
            glow::RGBA as i32,
            data.width as i32,
            data.height as i32,
            0,
            glow::RGBA,
            glow::UNSIGNED_BYTE,
            Some(&data.rgba),
        );
        gl.bind_texture(glow::TEXTURE_2D, None);
    }
}

// ─── Public API ─────────────────────────────────────────────────────────

/// Per-frame parameters describing world-image placement within the canvas.
pub struct GlCanvasParams {
    /// The egui rect allocated for the whole canvas area.
    pub canvas_rect: egui::Rect,
    /// Normalised `[left, top, right, bottom]` of the world image within the
    /// canvas viewport, in `[0, 1]` coordinates (0,0 = top-left).
    pub world_rect_norm: [f32; 4],
    /// Whether to sample the world texture.
    pub has_world: bool,
    /// Whether to sample the biome overlay texture.
    pub has_biome: bool,
}

/// Build the [`egui::PaintCallback`] that renders the canvas via raw OpenGL.
pub fn make_canvas_callback(
    state: Arc<Mutex<GlCanvasState>>,
    params: GlCanvasParams,
) -> egui::PaintCallback {
    let world_rect_norm = params.world_rect_norm;
    let has_world = params.has_world;
    let has_biome = params.has_biome;

    let cb = egui_glow::CallbackFn::new(move |info, painter| {
        let gl = painter.gl();
        let mut st = state.lock().unwrap();

        // ── lazy init ──
        if st.resources.is_none() {
            st.resources = Some(init_resources(gl));
        }
        // Copy GL handles out so we can release the immutable borrow before mutating.
        let res = st.resources.as_ref().unwrap();
        let program = res.program;
        let vao = res.vao;
        let world_tex = res.world_tex;
        let biome_tex = res.biome_tex;
        let loc_viewport_size = res.loc_viewport_size.clone();
        let loc_world_rect = res.loc_world_rect.clone();
        let loc_checker_tile = res.loc_checker_tile.clone();
        let loc_has_world = res.loc_has_world.clone();
        let loc_has_biome = res.loc_has_biome.clone();
        let loc_world_tex_u = res.loc_world_tex.clone();
        let loc_biome_tex_u = res.loc_biome_tex.clone();
        let _ = res; // release immutable borrow on `st`

        // ── upload dirty textures ──
        if st.world_dirty {
            if let Some(data) = &st.world_data {
                upload_texture(gl, world_tex, data);
            }
            st.world_dirty = false;
        }
        if st.biome_dirty {
            if let Some(data) = &st.biome_data {
                upload_texture(gl, biome_tex, data);
            }
            st.biome_dirty = false;
        }

        // ── draw ──
        let vp = info.viewport_in_pixels();
        unsafe {
            gl.disable(glow::SCISSOR_TEST);
            gl.disable(glow::BLEND);

            gl.use_program(Some(program));

            // viewport size (physical pixels)
            gl.uniform_2_f32(
                loc_viewport_size.as_ref(),
                vp.width_px as f32,
                vp.height_px as f32,
            );

            // world image rect (normalised)
            gl.uniform_4_f32(
                loc_world_rect.as_ref(),
                world_rect_norm[0],
                world_rect_norm[1],
                world_rect_norm[2],
                world_rect_norm[3],
            );

            // checkerboard tile (48 logical px → physical px)
            gl.uniform_1_f32(
                loc_checker_tile.as_ref(),
                48.0 * info.pixels_per_point,
            );

            // feature flags
            gl.uniform_1_f32(
                loc_has_world.as_ref(),
                if has_world && st.has_world { 1.0 } else { 0.0 },
            );
            gl.uniform_1_f32(
                loc_has_biome.as_ref(),
                if has_biome && st.has_biome { 1.0 } else { 0.0 },
            );

            // bind textures
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, Some(world_tex));
            gl.uniform_1_i32(loc_world_tex_u.as_ref(), 0);

            gl.active_texture(glow::TEXTURE1);
            gl.bind_texture(glow::TEXTURE_2D, Some(biome_tex));
            gl.uniform_1_i32(loc_biome_tex_u.as_ref(), 1);

            // draw fullscreen quad
            gl.bind_vertex_array(Some(vao));
            gl.draw_arrays(glow::TRIANGLE_STRIP, 0, 4);
            gl.bind_vertex_array(None);

            // unbind
            gl.active_texture(glow::TEXTURE1);
            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.active_texture(glow::TEXTURE0);
            gl.bind_texture(glow::TEXTURE_2D, None);
            gl.use_program(None);
        }
    });

    egui::PaintCallback {
        rect: params.canvas_rect,
        callback: Arc::new(cb),
    }
}

/// Convert a `&[Color32]` pixel buffer to packed RGBA `Vec<u8>`.
///
/// This is a zero-copy reinterpret when possible; falls back to a
/// per-pixel copy.
pub fn pixels_to_rgba(pixels: &[Color32]) -> Vec<u8> {
    // Color32 is #[repr(C)] [u8; 4] in RGBA order, safe to reinterpret.
    unsafe {
        core::slice::from_raw_parts(pixels.as_ptr() as *const u8, pixels.len() * 4).to_vec()
    }
}
