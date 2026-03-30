//! GLSL fragment shader background for the infinite canvas.
//!
//! Uses smithay's built-in `PixelShaderElement` to render a custom fragment shader
//! across the full screen, passing viewport uniforms so shaders can react to
//! camera position, zoom, and time.
//!
//! ## Shader contract
//!
//! The shader runs through `GlesRenderer::compile_custom_pixel_shader`, which means:
//! - No `#version` directive (smithay prepends `#version 100`)
//! - Built-in uniforms: `size` (vec2), `alpha` (float)
//! - Built-in varying: `v_coords` (vec2, 0–1 in quad)
//! - Custom uniforms we define: `u_time`, `u_camera`, `u_zoom`, `u_resolution`
//!
//! Example user shader (saved to `~/.config/canvaswm/background.frag`):
//! ```glsl
//! precision mediump float;
//! varying vec2 v_coords;
//! uniform float u_time;
//! uniform vec2 u_camera;
//! uniform float u_zoom;
//! uniform vec2 u_resolution;
//!
//! void main() {
//!     vec2 uv = v_coords;
//!     vec2 canvas = u_camera + uv * u_resolution / u_zoom;
//!     float grid = step(0.98, fract(canvas.x / 60.0)) + step(0.98, fract(canvas.y / 60.0));
//!     vec3 col = vec3(0.12, 0.12, 0.18) + grid * vec3(0.08);
//!     gl_FragColor = vec4(col, 1.0);
//! }
//! ```

use smithay::backend::renderer::gles::{
    GlesPixelProgram, GlesRenderer, Uniform, UniformName, UniformType,
};
use std::path::Path;

/// Default fragment shader: subtle animated gradient + grid.
pub const DEFAULT_SHADER: &str = r#"
precision mediump float;
varying vec2 v_coords;
uniform float u_time;
uniform vec2 u_camera;
uniform float u_zoom;
uniform vec2 u_resolution;
uniform float alpha;

void main() {
    vec2 uv = v_coords;
    // Canvas-space coordinate
    vec2 canvas = u_camera + uv * u_resolution / u_zoom;

    // Subtle grid (60px spacing in canvas space)
    float spacing = 60.0;
    vec2 grid_uv = fract(canvas / spacing);
    float line = smoothstep(0.96, 1.0, grid_uv.x) + smoothstep(0.96, 1.0, grid_uv.y);
    line = clamp(line, 0.0, 1.0);

    // Dot at intersections
    vec2 dot_uv = abs(grid_uv - 0.5) * 2.0;
    float dot = 1.0 - smoothstep(0.0, 0.06, length(dot_uv - 1.0));

    // Base color: dark background with very subtle gradient
    vec3 base = vec3(0.118, 0.118, 0.180);
    float gradient = sin(uv.x * 3.14159 + u_time * 0.05) * 0.01;
    base += gradient;

    // Combine
    vec3 col = base + line * vec3(0.04) + dot * vec3(0.15, 0.15, 0.25);

    // Fade grid when zoomed out
    float fade = smoothstep(0.05, 0.2, u_zoom);
    col = mix(base, col, fade);

    gl_FragColor = vec4(col, 1.0) * alpha;
}
"#;

/// Uniforms we pass to every background shader.
pub const UNIFORM_NAMES: &[(&str, UniformType)] = &[
    ("u_time", UniformType::_1f),
    ("u_camera", UniformType::_2f),
    ("u_zoom", UniformType::_1f),
    ("u_resolution", UniformType::_2f),
];

/// Compile a background shader program. If `shader_path` is Some, load from file;
/// otherwise use the built-in default shader.
pub fn compile_background_shader(
    renderer: &mut GlesRenderer,
    shader_path: Option<&str>,
) -> Result<GlesPixelProgram, String> {
    let source = match shader_path {
        Some(path) => {
            let p = Path::new(path);
            // Try relative to config dir first, then absolute
            let resolved = if p.is_absolute() {
                p.to_path_buf()
            } else if let Some(config_dir) = dirs::config_dir() {
                config_dir.join("canvaswm").join(p)
            } else {
                p.to_path_buf()
            };
            std::fs::read_to_string(&resolved)
                .map_err(|e| format!("Failed to read shader '{}': {e}", resolved.display()))?
        }
        None => DEFAULT_SHADER.to_string(),
    };

    let uniform_names: Vec<UniformName<'_>> = UNIFORM_NAMES
        .iter()
        .map(|(name, ty)| UniformName::new(*name, *ty))
        .collect();

    renderer
        .compile_custom_pixel_shader(&source, &uniform_names)
        .map_err(|e| format!("Shader compilation failed: {e:?}"))
}

/// Build the uniform values for a given frame.
pub fn build_uniforms(
    time: f32,
    camera: (f32, f32),
    zoom: f32,
    resolution: (f32, f32),
) -> Vec<Uniform<'static>> {
    vec![
        Uniform::new("u_time", time),
        Uniform::new("u_camera", [camera.0, camera.1]),
        Uniform::new("u_zoom", zoom),
        Uniform::new("u_resolution", [resolution.0, resolution.1]),
    ]
}
