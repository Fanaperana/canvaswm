//! Window decoration shaders for SSD borders, shadows, and corner rounding.
//!
//! These are rendered as PixelShaderElement quads placed around/behind each window,
//! using the canvas viewport zoom to scale correctly.

use smithay::{
    backend::renderer::{
        element::Kind,
        gles::{
            element::PixelShaderElement, GlesPixelProgram, GlesRenderer, Uniform, UniformName,
            UniformType,
        },
    },
    utils::{Logical, Point, Rectangle, Size},
};

use crate::element::CanvasRenderElement;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Focused shadow color (subtle, but visible).
const SHADOW_COLOR_FOCUSED: [f32; 4] = [0.0, 0.0, 0.0, 0.22];

/// Unfocused shadow color (lighter than focused).
const SHADOW_COLOR_UNFOCUSED: [f32; 4] = [0.0, 0.0, 0.0, 0.14];

/// Relative border width scale for active/inactive windows.
const ACTIVE_BORDER_SCALE: f32 = 0.45;
const INACTIVE_BORDER_SCALE: f32 = 0.45;

/// Relative shadow spread scale for active/inactive windows.
const ACTIVE_SHADOW_SCALE: f32 = 0.80;
const INACTIVE_SHADOW_SCALE: f32 = 0.60;

/// Element render alpha (fully opaque shader).
const ELEMENT_ALPHA: f32 = 1.0;

/// Minimum screen-pixel width for a border element (1 px).
const MIN_SCREEN_BORDER: i32 = 1;

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
    float a = u_color.a * alpha * d;
    gl_FragColor = vec4(u_color.rgb * a, a);
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
uniform float u_border_width;

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

    // Zero the shadow inside the window AND inside the border ring.
    // Without this, the shadow bleeds through the semi-transparent inactive
    // border, making it appear as a second dark ring (double-layer effect).
    // We shift the shadow start outward by u_border_width so it only renders
    // beyond the outer edge of the border ring.
    float d = dist - u_border_width;
    float shadow = step(0.0, d) * (1.0 - smoothstep(0.0, u_spread, d));
    shadow = shadow * shadow; // softer, Gaussian-like falloff

    float a = u_shadow_color.a * shadow * alpha;
    gl_FragColor = vec4(u_shadow_color.rgb * a, a);
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
    
    // Border region: outside inner, inside outer.
    // Use a tighter AA band so thin borders stay crisp instead of muddy.
    float aa = 0.35;
    float outer_mask = 1.0 - smoothstep(-aa, aa, outer);
    float inner_mask = smoothstep(-aa, aa, inner);
    float border = outer_mask * inner_mask;
    
    float a = u_color.a * border * alpha;
    gl_FragColor = vec4(u_color.rgb * a, a);
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
    // Start masking slightly outside the mathematical edge so we do not
    // erode the border antialiasing band (which appears as corner "cuts").
    float outside = smoothstep(0.5, 1.5, dist);
    float a = u_bg_color.a * outside * alpha;
    gl_FragColor = vec4(u_bg_color.rgb * a, a);
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
                    UniformName::new("u_border_width", UniformType::_1f),
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
        border_width: f32,
    ) -> Vec<Uniform<'static>> {
        vec![
            Uniform::new("u_shadow_color", color),
            Uniform::new("u_radius", radius),
            Uniform::new("u_window_size", [window_size.0, window_size.1]),
            Uniform::new("u_spread", spread),
            Uniform::new("u_border_width", border_width),
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

// ---------------------------------------------------------------------------
// Window descriptor — decouples rendering from compositor state
// ---------------------------------------------------------------------------

/// Backend-agnostic description of a visible window for decoration rendering.
///
/// Collected by the compositor from its `Space` and passed into the render
/// functions below, keeping the render crate free of compositor types.
pub struct WindowInfo {
    /// Screen-space position (already transformed from canvas via viewport).
    pub screen_x: f64,
    pub screen_y: f64,
    /// Screen-space dimensions (content geometry × zoom).
    pub screen_w: i32,
    pub screen_h: i32,
    /// Full surface bounding-box screen position (includes CSD frame).
    pub bbox_screen_x: f64,
    pub bbox_screen_y: f64,
    /// Full surface bounding-box screen dimensions.
    pub bbox_screen_w: i32,
    pub bbox_screen_h: i32,
    /// Whether this window currently has keyboard focus.
    pub focused: bool,
}

/// Decoration configuration extracted from the global config.
#[derive(Clone)]
pub struct DecorationParams {
    pub shadow_enabled: bool,
    pub shadow_radius: f32,
    pub corner_radius: f32,
    pub border_width: f32,
    pub ssd_mode: bool,
    pub title_height: i32,
    pub focused_color: [f32; 4],
    pub unfocused_color: [f32; 4],
    pub title_bar_color: [f32; 4],
    pub bg_color: [f32; 4],
}

// ---------------------------------------------------------------------------
// Decoration element generation
// ---------------------------------------------------------------------------

/// Generate shadow, border, and title-bar elements for every visible window.
///
/// These are rendered *behind* window surfaces in the compositor layer stack.
pub fn generate_decoration_elements(
    shaders: &DecorationShaders,
    windows: &[WindowInfo],
    params: &DecorationParams,
    zoom: f64,
) -> Vec<CanvasRenderElement> {
    let mut elements = Vec::new();

    let scaled_radius = params.corner_radius * zoom as f32;
    let scaled_border = params.border_width * zoom as f32;
    let scaled_shadow_radius = params.shadow_radius * zoom as f32;

    for win in windows {
        let border_color = if win.focused {
            params.focused_color
        } else {
            params.unfocused_color
        };

        let border_scale = if win.focused {
            ACTIVE_BORDER_SCALE
        } else {
            INACTIVE_BORDER_SCALE
        };
        let border_width = scaled_border * border_scale;

        // Shadow is pushed LAST so it is furthest back in the front-to-back
        // render order. Smithay renders elements[0] on top; shadow must be
        // behind the border ring, not in front of it.  Collecting it first
        // and appending after the border elements achieves this.
        let mut shadow_elem: Option<CanvasRenderElement> = None;
        if params.shadow_enabled && params.shadow_radius > 0.0 {
            let spread = scaled_shadow_radius
                * if win.focused {
                    ACTIVE_SHADOW_SCALE
                } else {
                    INACTIVE_SHADOW_SCALE
                };
            let sx = win.screen_x as i32 - spread as i32;
            let sy = win.screen_y as i32 - spread as i32;
            let sw = win.screen_w + spread as i32 * 2;
            let sh = win.screen_h + spread as i32 * 2;

            let area = Rectangle::new(
                Point::<i32, Logical>::from((sx, sy)),
                Size::<i32, Logical>::from((sw, sh)),
            );
            let uniforms = DecorationShaders::shadow_uniforms(
                if win.focused {
                    SHADOW_COLOR_FOCUSED
                } else {
                    SHADOW_COLOR_UNFOCUSED
                },
                scaled_radius,
                (win.screen_w as f32, win.screen_h as f32),
                spread,
                border_width,
            );
            shadow_elem = Some(CanvasRenderElement::Shader(PixelShaderElement::new(
                shaders.shadow.clone(),
                area,
                None,
                ELEMENT_ALPHA,
                uniforms,
                Kind::Unspecified,
            )));
        }

        // Border
        if params.border_width > 0.0 {
            let bw = (border_width.ceil() as i32).max(MIN_SCREEN_BORDER);
            let area = Rectangle::new(
                Point::<i32, Logical>::from((win.screen_x as i32 - bw, win.screen_y as i32 - bw)),
                Size::<i32, Logical>::from((win.screen_w + bw * 2, win.screen_h + bw * 2)),
            );
            let uniforms = DecorationShaders::border_uniforms(
                border_color,
                scaled_radius + border_width,
                border_width,
                (
                    (win.screen_w + bw * 2) as f32,
                    (win.screen_h + bw * 2) as f32,
                ),
            );
            elements.push(CanvasRenderElement::Shader(PixelShaderElement::new(
                shaders.border.clone(),
                area,
                None,
                ELEMENT_ALPHA,
                uniforms,
                Kind::Unspecified,
            )));
        }

        // SSD title bar — skip entirely when height is zero to avoid a
        // degenerate zero-height quad that some GLES drivers render as a
        // full-size rect filled with the title bar colour.
        if params.ssd_mode {
            let th = (params.title_height as f64 * zoom) as i32;
            if th > 0 {
                let area = Rectangle::new(
                    Point::<i32, Logical>::from((win.screen_x as i32, win.screen_y as i32 - th)),
                    Size::<i32, Logical>::from((win.screen_w, th)),
                );
                let uniforms = DecorationShaders::title_bar_uniforms(
                    params.title_bar_color,
                    scaled_radius / win.screen_w.max(1) as f32,
                );
                elements.push(CanvasRenderElement::Shader(PixelShaderElement::new(
                    shaders.title_bar.clone(),
                    area,
                    None,
                    ELEMENT_ALPHA,
                    uniforms,
                    Kind::Unspecified,
                )));
            }
        }

        // Push shadow last — it must be furthest back in the front-to-back
        // element list so the border ring renders visually on top of it.
        if let Some(s) = shadow_elem {
            elements.push(s);
        }
    }

    elements
}

// ---------------------------------------------------------------------------
// Corner clip element generation
// ---------------------------------------------------------------------------

/// Generate corner-clip overlay elements drawn *on top* of window content.
///
/// Uses the background colour to paint over sharp corners outside a rounded
/// rectangle, covering both the window surface (including CSD frame) and the
/// border decoration area.
pub fn generate_corner_clip_elements(
    shaders: &DecorationShaders,
    windows: &[WindowInfo],
    params: &DecorationParams,
    zoom: f64,
) -> Vec<CanvasRenderElement> {
    if params.corner_radius <= 0.0 {
        return Vec::new();
    }

    let scaled_radius = params.corner_radius * zoom as f32;
    let scaled_border = params.border_width * zoom as f32;

    let mut elements = Vec::with_capacity(windows.len());

    for win in windows {
        let border_scale = if win.focused {
            ACTIVE_BORDER_SCALE
        } else {
            INACTIVE_BORDER_SCALE
        };
        let border_px = (scaled_border * border_scale).ceil() as i32;
        // Expand the clip radius a touch so the clip does not bite into the
        // rounded border edge at fractional scales.
        let outer_radius = scaled_radius + border_px as f32 + 0.75;

        let clip_x = win.bbox_screen_x as i32 - border_px;
        let clip_y = win.bbox_screen_y as i32 - border_px;
        let clip_w = win.bbox_screen_w + border_px * 2;
        let clip_h = win.bbox_screen_h + border_px * 2;

        let area = Rectangle::new(
            Point::<i32, Logical>::from((clip_x, clip_y)),
            Size::<i32, Logical>::from((clip_w, clip_h)),
        );
        let uniforms = DecorationShaders::corner_clip_uniforms(
            outer_radius,
            (clip_w as f32, clip_h as f32),
            params.bg_color,
        );
        elements.push(CanvasRenderElement::Shader(PixelShaderElement::new(
            shaders.corner_clip.clone(),
            area,
            None,
            ELEMENT_ALPHA,
            uniforms,
            Kind::Unspecified,
        )));
    }

    elements
}
