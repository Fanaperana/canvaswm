//! Unified background renderer for the infinite canvas.
//!
//! Supports four modes selected by configuration:
//! - `"shader"` — animated GLSL fragment shader (custom or built-in).
//! - `"image"`  — still wallpaper image (PNG / JPEG / WebP).
//! - `"dots"`   — dot-grid that scrolls with the canvas.
//! - `"solid"`  — plain colour (handled by smithay clear colour; no elements).

use smithay::{
    backend::renderer::{
        element::{memory::MemoryRenderBuffer, Kind},
        gles::{element::PixelShaderElement, GlesPixelProgram, GlesRenderer},
    },
    utils::{Logical, Point, Rectangle, Size, Transform},
};

use canvaswm_canvas::Viewport;

use crate::element::CanvasRenderElement;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Shader element alpha.
const ELEMENT_ALPHA: f32 = 1.0;

// ---------------------------------------------------------------------------
// Background state
// ---------------------------------------------------------------------------

/// Holds the pre-compiled / pre-loaded resources for whichever background
/// mode is active. Created once at compositor startup.
pub enum Background {
    Shader(GlesPixelProgram),
    Image(MemoryRenderBuffer),
    Dots,
    Solid,
}

impl Background {
    /// Initialise the background from config values.
    ///
    /// `mode` is one of `"shader"`, `"image"`, `"dots"`, `"solid"`.
    pub fn init(
        renderer: &mut GlesRenderer,
        mode: &str,
        shader_path: Option<&str>,
        image_path: Option<&str>,
    ) -> Self {
        match mode {
            "shader" => match crate::shader_bg::compile_background_shader(renderer, shader_path) {
                Ok(prog) => {
                    tracing::info!("Background shader compiled successfully");
                    Self::Shader(prog)
                }
                Err(e) => {
                    tracing::error!("Background shader error: {e}. Falling back to dots.");
                    Self::Dots
                }
            },
            "image" => match image_path {
                Some(path) => match crate::image_bg::load_image(path) {
                    Ok(img) => {
                        let buf = MemoryRenderBuffer::from_slice(
                            &img.data,
                            smithay::backend::allocator::Fourcc::Abgr8888,
                            (img.width as i32, img.height as i32),
                            1,
                            Transform::Normal,
                            None,
                        );
                        tracing::info!(
                            "Background image loaded: {}x{}",
                            img.width,
                            img.height,
                        );
                        Self::Image(buf)
                    }
                    Err(e) => {
                        tracing::error!("Background image error: {e}. Falling back to solid.");
                        Self::Solid
                    }
                },
                None => {
                    tracing::warn!("Background mode is 'image' but no image_path set.");
                    Self::Solid
                }
            },
            "dots" => Self::Dots,
            _ => Self::Solid,
        }
    }

    /// Produce the render elements for the current frame.
    ///
    /// `renderer` is only needed for the `Image` variant (texture upload).
    #[allow(clippy::too_many_arguments)]
    pub fn render_elements(
        &self,
        renderer: &mut GlesRenderer,
        viewport: &Viewport,
        screen_size: (i32, i32),
        elapsed_secs: f32,
        dot_color: [f32; 4],
        grid_spacing: f64,
        dot_size: f64,
    ) -> Vec<CanvasRenderElement> {
        match self {
            Self::Shader(prog) => {
                let uniforms = crate::shader_bg::build_uniforms(
                    elapsed_secs,
                    (viewport.camera_x as f32, viewport.camera_y as f32),
                    viewport.zoom as f32,
                    (screen_size.0 as f32, screen_size.1 as f32),
                );
                let area = Rectangle::from_size(Size::<i32, Logical>::from(screen_size));
                let element = PixelShaderElement::new(
                    prog.clone(),
                    area,
                    None,
                    ELEMENT_ALPHA,
                    uniforms,
                    Kind::Unspecified,
                );
                vec![CanvasRenderElement::Shader(element)]
            }
            Self::Image(buf) => {
                use smithay::backend::renderer::element::memory::MemoryRenderBufferRenderElement;
                match MemoryRenderBufferRenderElement::from_buffer(
                    renderer,
                    Point::from((0.0, 0.0)),
                    buf,
                    None,
                    None,
                    Some(Size::<i32, Logical>::from(screen_size)),
                    Kind::Unspecified,
                ) {
                    Ok(el) => vec![CanvasRenderElement::MemoryBuf(el)],
                    Err(e) => {
                        tracing::error!("Failed to render bg image: {e:?}");
                        Vec::new()
                    }
                }
            }
            Self::Dots => crate::dot_grid::dot_grid_elements(
                viewport,
                screen_size,
                dot_color,
                grid_spacing,
                dot_size,
            )
            .into_iter()
            .map(CanvasRenderElement::from)
            .collect(),
            Self::Solid => Vec::new(),
        }
    }
}
