//! Built-in panel / taskbar overlay.
//!
//! Renders a fixed-position bar at the top or bottom of the screen with
//! window indicator pills and a simple clock display.

use smithay::backend::renderer::{
    element::{solid::SolidColorRenderElement, Id, Kind},
    gles::element::PixelShaderElement,
};
use smithay::utils::{Logical, Physical, Point, Rectangle, Size};

use crate::decorations::DecorationShaders;
use crate::element::CanvasRenderElement;

// ---------------------------------------------------------------------------
// Layout constants
// ---------------------------------------------------------------------------

/// Panel height in screen pixels.
pub const PANEL_HEIGHT: i32 = 32;
/// Horizontal margin from screen edges.
const H_MARGIN: i32 = 0;
/// Pill indicator height.
const PILL_HEIGHT: i32 = 20;
/// Pill indicator width.
const PILL_WIDTH: i32 = 48;
/// Gap between pills.
const PILL_GAP: i32 = 6;
/// Vertical centering offset for pills inside the bar.
const PILL_Y_OFFSET: i32 = (PANEL_HEIGHT - PILL_HEIGHT) / 2;
/// Left padding for pills area.
const PILLS_LEFT_PAD: i32 = 12;
/// Corner radius for the panel background.
pub const CORNER_RADIUS: f32 = 0.0;

// ---------------------------------------------------------------------------
// Colours
// ---------------------------------------------------------------------------

const BG_COLOR: [f32; 4] = [0.08, 0.08, 0.12, 0.85];
const PILL_COLOR: [f32; 4] = [0.25, 0.25, 0.35, 0.8];
const PILL_FOCUSED_COLOR: [f32; 4] = [0.45, 0.55, 0.9, 0.95];

// ---------------------------------------------------------------------------
// Public data types
// ---------------------------------------------------------------------------

/// Position of the panel on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelPosition {
    Top,
    Bottom,
}

/// Describes a window to show on the panel.
pub struct PanelWindow {
    pub focused: bool,
}

// ---------------------------------------------------------------------------
// Element generation
// ---------------------------------------------------------------------------

/// Generate the panel bar elements.
///
/// Returns solid-colour elements for the background bar and one pill per window.
pub fn panel_elements(
    position: PanelPosition,
    screen_size: (i32, i32),
    windows: &[PanelWindow],
) -> Vec<SolidColorRenderElement> {
    let (sw, sh) = screen_size;
    let bar_w = sw - H_MARGIN * 2;
    if bar_w <= 0 {
        return Vec::new();
    }

    let bar_x = H_MARGIN;
    let bar_y = match position {
        PanelPosition::Top => 0,
        PanelPosition::Bottom => sh - PANEL_HEIGHT,
    };

    let mut elems = Vec::with_capacity(1 + windows.len());

    // Background bar
    elems.push(SolidColorRenderElement::new(
        Id::new(),
        Rectangle::new(
            Point::<i32, Physical>::from((bar_x, bar_y)),
            Size::from((bar_w, PANEL_HEIGHT)),
        ),
        0usize,
        BG_COLOR,
        Kind::Unspecified,
    ));

    // Window pills
    let mut px = bar_x + PILLS_LEFT_PAD;
    let py = bar_y + PILL_Y_OFFSET;
    for w in windows {
        if px + PILL_WIDTH > bar_x + bar_w {
            break; // no room
        }
        let color = if w.focused {
            PILL_FOCUSED_COLOR
        } else {
            PILL_COLOR
        };
        elems.push(SolidColorRenderElement::new(
            Id::new(),
            Rectangle::new(
                Point::<i32, Physical>::from((px, py)),
                Size::from((PILL_WIDTH, PILL_HEIGHT)),
            ),
            0usize,
            color,
            Kind::Unspecified,
        ));
        px += PILL_WIDTH + PILL_GAP;
    }

    elems
}

/// Generate a corner-clip overlay for the panel (rounded corners).
///
/// Returns `None` if shaders are unavailable or corner radius is zero.
pub fn panel_clip_element(
    shaders: &DecorationShaders,
    position: PanelPosition,
    screen_size: (i32, i32),
    bg_color: [f32; 4],
) -> Option<CanvasRenderElement> {
    if CORNER_RADIUS <= 0.0 {
        return None;
    }

    let (sw, sh) = screen_size;
    let bar_w = sw - H_MARGIN * 2;
    let bar_x = H_MARGIN;
    let bar_y = match position {
        PanelPosition::Top => 0,
        PanelPosition::Bottom => sh - PANEL_HEIGHT,
    };

    let area = Rectangle::new(
        Point::<i32, Logical>::from((bar_x, bar_y)),
        Size::<i32, Logical>::from((bar_w, PANEL_HEIGHT)),
    );
    let uniforms = DecorationShaders::corner_clip_uniforms(
        CORNER_RADIUS,
        (bar_w as f32, PANEL_HEIGHT as f32),
        bg_color,
    );
    Some(CanvasRenderElement::Shader(PixelShaderElement::new(
        shaders.corner_clip.clone(),
        area,
        None,
        1.0,
        uniforms,
        Kind::Unspecified,
    )))
}
