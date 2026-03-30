//! Multi-pass Kawase blur for window transparency effects.
//!
//! Kawase blur uses iterative down/up-sampling with offset texture reads.
//! Each pass reads 4 texels at increasing offsets, producing a wide soft blur
//! much cheaper than Gaussian.
//!
//! This module provides the shader source and uniform helpers.
//! The actual multi-pass rendering must be done by the compositor using
//! GlesRenderer's render-to-texture capabilities.

use smithay::backend::renderer::gles::{
    GlesPixelProgram, GlesRenderer, Uniform, UniformName, UniformType,
};

/// Kawase blur downsample pass shader.
/// Reads 5 samples (center + 4 diagonal offsets) and averages.
pub const KAWASE_DOWN_SHADER: &str = r#"
precision mediump float;
varying vec2 v_coords;
uniform float alpha;
uniform vec2 u_half_pixel;
uniform sampler2D u_texture;

void main() {
    vec2 uv = v_coords;
    
    vec4 sum = texture2D(u_texture, uv) * 4.0;
    sum += texture2D(u_texture, uv - u_half_pixel);
    sum += texture2D(u_texture, uv + u_half_pixel);
    sum += texture2D(u_texture, uv + vec2(u_half_pixel.x, -u_half_pixel.y));
    sum += texture2D(u_texture, uv + vec2(-u_half_pixel.x, u_half_pixel.y));
    
    gl_FragColor = sum / 8.0 * alpha;
}
"#;

/// Kawase blur upsample pass shader.
/// Reads 8 samples in a diamond pattern and averages.
pub const KAWASE_UP_SHADER: &str = r#"
precision mediump float;
varying vec2 v_coords;
uniform float alpha;
uniform vec2 u_half_pixel;
uniform sampler2D u_texture;

void main() {
    vec2 uv = v_coords;
    
    vec4 sum = vec4(0.0);
    sum += texture2D(u_texture, uv + vec2(-u_half_pixel.x * 2.0, 0.0));
    sum += texture2D(u_texture, uv + vec2(-u_half_pixel.x, u_half_pixel.y)) * 2.0;
    sum += texture2D(u_texture, uv + vec2(0.0, u_half_pixel.y * 2.0));
    sum += texture2D(u_texture, uv + vec2(u_half_pixel.x, u_half_pixel.y)) * 2.0;
    sum += texture2D(u_texture, uv + vec2(u_half_pixel.x * 2.0, 0.0));
    sum += texture2D(u_texture, uv + vec2(u_half_pixel.x, -u_half_pixel.y)) * 2.0;
    sum += texture2D(u_texture, uv + vec2(0.0, -u_half_pixel.y * 2.0));
    sum += texture2D(u_texture, uv + vec2(-u_half_pixel.x, -u_half_pixel.y)) * 2.0;
    
    gl_FragColor = sum / 12.0 * alpha;
}
"#;

/// Compiled Kawase blur programs.
pub struct KawaseBlurShaders {
    pub downsample: GlesPixelProgram,
    pub upsample: GlesPixelProgram,
}

impl KawaseBlurShaders {
    /// Compile blur shaders. Call once at startup.
    pub fn compile(renderer: &mut GlesRenderer) -> Result<Self, String> {
        let downsample = renderer
            .compile_custom_pixel_shader(
                KAWASE_DOWN_SHADER,
                &[
                    UniformName::new("u_half_pixel", UniformType::_2f),
                    UniformName::new("u_texture", UniformType::_1i),
                ],
            )
            .map_err(|e| format!("Kawase downsample: {e:?}"))?;

        let upsample = renderer
            .compile_custom_pixel_shader(
                KAWASE_UP_SHADER,
                &[
                    UniformName::new("u_half_pixel", UniformType::_2f),
                    UniformName::new("u_texture", UniformType::_1i),
                ],
            )
            .map_err(|e| format!("Kawase upsample: {e:?}"))?;

        Ok(Self { downsample, upsample })
    }

    /// Build uniforms for a blur pass at a given resolution.
    pub fn pass_uniforms(width: f32, height: f32, strength: f32) -> Vec<Uniform<'static>> {
        vec![
            Uniform::new("u_half_pixel", [strength / width, strength / height]),
            Uniform::new("u_texture", 0i32),
        ]
    }
}
