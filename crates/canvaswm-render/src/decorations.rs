//! Window decoration shaders for SSD borders, shadows, and corner rounding.
//!
//! These are rendered as PixelShaderElement quads placed around/behind each window,
//! using the canvas viewport zoom to scale correctly.

use smithay::backend::renderer::gles::{
    GlesPixelProgram, GlesRenderer, Uniform, UniformName, UniformType,
};

/// Shader for SSD title bar — solid colored bar above each window.
pub const SSD_TITLE_BAR_SHADER: &str = r#"
precision mediump float;
varying vec2 v_coords;
uniform float alpha;
uniform vec4 u_color;
uniform float u_radius;

void main() {
    vec2 uv = v_coords;
    // Only round top corners
    float r = u_radius;
    vec2 size_px = vec2(1.0); // normalized, actual size via dst rect
    vec2 pos = uv;
    float d = 1.0;
    // Top-left corner
    if (pos.x < r && pos.y < r) {
        d = 1.0 - smoothstep(r - 1.0, r, length(pos - vec2(r, r)));
    }
    // Top-right corner
    if (pos.x > 1.0 - r && pos.y < r) {
        d = 1.0 - smoothstep(r - 1.0, r, length(pos - vec2(1.0 - r, r)));
    }
    gl_FragColor = u_color * alpha * d;
}
"#;

/// Shader for window shadow — Gaussian-ish drop shadow.
pub const SHADOW_SHADER: &str = r#"
precision mediump float;
varying vec2 v_coords;
uniform float alpha;
uniform vec4 u_shadow_color;
uniform float u_radius;
uniform vec2 u_window_size;
uniform float u_spread;

// Approximate box shadow with rounded corners
float roundedBoxSDF(vec2 p, vec2 b, float r) {
    vec2 d = abs(p) - b + vec2(r);
    return length(max(d, 0.0)) - r;
}

void main() {
    vec2 uv = v_coords;
    // Map UV to centered coordinates
    vec2 size = u_window_size + u_spread * 2.0;
    vec2 p = (uv - 0.5) * size;
    vec2 halfWin = u_window_size * 0.5;
    
    float dist = roundedBoxSDF(p, halfWin, u_radius);
    
    // Shadow falloff — Gaussian-like
    float shadow = 1.0 - smoothstep(0.0, u_spread, dist);
    shadow = shadow * shadow; // softer falloff
    
    gl_FragColor = vec4(u_shadow_color.rgb, u_shadow_color.a * shadow * alpha);
}
"#;

/// Shader for rounded corner mask — clips corners of windows.
/// This draws a rounded rectangle filled with a border color.
pub const BORDER_SHADER: &str = r#"
precision mediump float;
varying vec2 v_coords;
uniform float alpha;
uniform vec4 u_color;
uniform float u_radius;
uniform float u_border_width;
uniform vec2 u_size;

float roundedBoxSDF(vec2 p, vec2 b, float r) {
    vec2 d = abs(p) - b + vec2(r);
    return length(max(d, 0.0)) - r;
}

void main() {
    vec2 uv = v_coords;
    vec2 p = (uv - 0.5) * u_size;
    vec2 halfSize = u_size * 0.5;
    
    float outer = roundedBoxSDF(p, halfSize, u_radius);
    float inner = roundedBoxSDF(p, halfSize - vec2(u_border_width), max(u_radius - u_border_width, 0.0));
    
    // Border region: outside inner, inside outer
    float border = smoothstep(0.5, -0.5, outer) * smoothstep(-0.5, 0.5, inner);
    
    // Corner rounding mask for the outer edge
    float mask = smoothstep(0.5, -0.5, outer);
    
    gl_FragColor = vec4(u_color.rgb, u_color.a * border * alpha);
}
"#;

/// Shader for corner clipping — drawn on top of window content to round corners.
/// Outputs background color at the corners (outside rounded rect) and is transparent inside.
pub const CORNER_CLIP_SHADER: &str = r#"
precision mediump float;
varying vec2 v_coords;
uniform float alpha;
uniform float u_radius;
uniform vec2 u_size;
uniform vec4 u_bg_color;

float roundedBoxSDF(vec2 p, vec2 b, float r) {
    vec2 d = abs(p) - b + vec2(r);
    return length(max(d, 0.0)) - r;
}

void main() {
    vec2 p = (v_coords - 0.5) * u_size;
    vec2 halfSize = u_size * 0.5;
    float dist = roundedBoxSDF(p, halfSize, u_radius);
    // Inside rounded rect: transparent (window shows through)
    // Outside rounded rect (corners): background color covers sharp edges
    float outside = smoothstep(-0.5, 0.5, dist);
    gl_FragColor = vec4(u_bg_color.rgb, u_bg_color.a * outside * alpha);
}
"#;

/// Compiled decoration shader programs.
pub struct DecorationShaders {
    pub shadow: GlesPixelProgram,
    pub border: GlesPixelProgram,
    pub title_bar: GlesPixelProgram,
    pub corner_clip: GlesPixelProgram,
}

impl DecorationShaders {
    /// Compile all decoration shaders. Call once at startup.
    pub fn compile(renderer: &mut GlesRenderer) -> Result<Self, String> {
        let shadow = renderer
            .compile_custom_pixel_shader(
                SHADOW_SHADER,
                &[
                    UniformName::new("u_shadow_color", UniformType::_4f),
                    UniformName::new("u_radius", UniformType::_1f),
                    UniformName::new("u_window_size", UniformType::_2f),
                    UniformName::new("u_spread", UniformType::_1f),
                ],
            )
            .map_err(|e| format!("Shadow shader: {e:?}"))?;

        let border = renderer
            .compile_custom_pixel_shader(
                BORDER_SHADER,
                &[
                    UniformName::new("u_color", UniformType::_4f),
                    UniformName::new("u_radius", UniformType::_1f),
                    UniformName::new("u_border_width", UniformType::_1f),
                    UniformName::new("u_size", UniformType::_2f),
                ],
            )
            .map_err(|e| format!("Border shader: {e:?}"))?;

        let title_bar = renderer
            .compile_custom_pixel_shader(
                SSD_TITLE_BAR_SHADER,
                &[
                    UniformName::new("u_color", UniformType::_4f),
                    UniformName::new("u_radius", UniformType::_1f),
                ],
            )
            .map_err(|e| format!("Title bar shader: {e:?}"))?;

        let corner_clip = renderer
            .compile_custom_pixel_shader(
                CORNER_CLIP_SHADER,
                &[
                    UniformName::new("u_radius", UniformType::_1f),
                    UniformName::new("u_size", UniformType::_2f),
                    UniformName::new("u_bg_color", UniformType::_4f),
                ],
            )
            .map_err(|e| format!("Corner clip shader: {e:?}"))?;

        Ok(Self {
            shadow,
            border,
            title_bar,
            corner_clip,
        })
    }

    /// Build shadow uniforms for a window.
    pub fn shadow_uniforms(
        color: [f32; 4],
        radius: f32,
        window_size: (f32, f32),
        spread: f32,
    ) -> Vec<Uniform<'static>> {
        vec![
            Uniform::new("u_shadow_color", color),
            Uniform::new("u_radius", radius),
            Uniform::new("u_window_size", [window_size.0, window_size.1]),
            Uniform::new("u_spread", spread),
        ]
    }

    /// Build border/rounding uniforms for a window.
    pub fn border_uniforms(
        color: [f32; 4],
        radius: f32,
        border_width: f32,
        size: (f32, f32),
    ) -> Vec<Uniform<'static>> {
        vec![
            Uniform::new("u_color", color),
            Uniform::new("u_radius", radius),
            Uniform::new("u_border_width", border_width),
            Uniform::new("u_size", [size.0, size.1]),
        ]
    }

    /// Build title bar uniforms.
    pub fn title_bar_uniforms(color: [f32; 4], radius: f32) -> Vec<Uniform<'static>> {
        vec![
            Uniform::new("u_color", color),
            Uniform::new("u_radius", radius),
        ]
    }

    /// Build corner clip uniforms.
    pub fn corner_clip_uniforms(
        radius: f32,
        size: (f32, f32),
        bg_color: [f32; 4],
    ) -> Vec<Uniform<'static>> {
        vec![
            Uniform::new("u_radius", radius),
            Uniform::new("u_size", [size.0, size.1]),
            Uniform::new("u_bg_color", bg_color),
        ]
    }
}
