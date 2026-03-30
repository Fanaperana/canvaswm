use canvaswm_canvas::Viewport;
use smithay::{
    backend::renderer::element::{solid::SolidColorRenderElement, Id, Kind},
    utils::{Physical, Point, Rectangle, Size},
};

/// Minimap dimensions and styling.
const MINIMAP_WIDTH: i32 = 200;
const MINIMAP_HEIGHT: i32 = 140;
const MINIMAP_MARGIN: i32 = 16;
const MINIMAP_PADDING: f64 = 100.0;

const BG_COLOR: [f32; 4] = [0.05, 0.05, 0.08, 0.75];
const WINDOW_COLOR: [f32; 4] = [0.4, 0.4, 0.55, 0.8];
const FOCUSED_COLOR: [f32; 4] = [0.5, 0.6, 0.9, 0.9];
const VIEWPORT_COLOR: [f32; 4] = [1.0, 1.0, 1.0, 0.35];
const VIEWPORT_BORDER: i32 = 1;

/// A window rect in canvas coordinates for minimap rendering.
pub struct MinimapWindow {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
    pub focused: bool,
}

/// Generate minimap overlay elements at the bottom-left of the screen.
///
/// Returns solid color elements representing the minimap background,
/// window rectangles, and the current viewport indicator.
pub fn minimap_elements(
    viewport: &Viewport,
    screen_size: (i32, i32),
    windows: &[MinimapWindow],
) -> Vec<SolidColorRenderElement> {
    if windows.is_empty() {
        return Vec::new();
    }

    // Compute bounding box of all windows
    let mut min_x = f64::MAX;
    let mut min_y = f64::MAX;
    let mut max_x = f64::MIN;
    let mut max_y = f64::MIN;
    for w in windows {
        min_x = min_x.min(w.x);
        min_y = min_y.min(w.y);
        max_x = max_x.max(w.x + w.w);
        max_y = max_y.max(w.y + w.h);
    }

    // Also include the current viewport visible area in the bounding box
    let (cam_x, cam_y, vis_w, vis_h) = viewport.visible_rect();
    min_x = min_x.min(cam_x);
    min_y = min_y.min(cam_y);
    max_x = max_x.max(cam_x + vis_w);
    max_y = max_y.max(cam_y + vis_h);

    // Add padding around the combined bbox
    min_x -= MINIMAP_PADDING;
    min_y -= MINIMAP_PADDING;
    max_x += MINIMAP_PADDING;
    max_y += MINIMAP_PADDING;

    let canvas_w = max_x - min_x;
    let canvas_h = max_y - min_y;
    if canvas_w <= 0.0 || canvas_h <= 0.0 {
        return Vec::new();
    }

    // Scale factor to fit canvas bbox into minimap area
    let scale_x = (MINIMAP_WIDTH - 4) as f64 / canvas_w;
    let scale_y = (MINIMAP_HEIGHT - 4) as f64 / canvas_h;
    let scale = scale_x.min(scale_y);

    // Minimap origin on screen (bottom-left)
    let mm_x = MINIMAP_MARGIN;
    let mm_y = screen_size.1 - MINIMAP_HEIGHT - MINIMAP_MARGIN;

    let mut elements = Vec::with_capacity(windows.len() + 4);

    // Background
    elements.push(SolidColorRenderElement::new(
        Id::new(),
        Rectangle::new(
            Point::<i32, Physical>::from((mm_x, mm_y)),
            Size::from((MINIMAP_WIDTH, MINIMAP_HEIGHT)),
        ),
        0usize,
        BG_COLOR,
        Kind::Unspecified,
    ));

    // Helper: convert canvas coords to minimap screen coords
    let to_mm = |cx: f64, cy: f64| -> (i32, i32) {
        let lx = ((cx - min_x) * scale) as i32 + mm_x + 2;
        let ly = ((cy - min_y) * scale) as i32 + mm_y + 2;
        (lx, ly)
    };

    // Window rectangles
    for w in windows {
        let (wx, wy) = to_mm(w.x, w.y);
        let ww = ((w.w * scale) as i32).max(2);
        let wh = ((w.h * scale) as i32).max(2);

        // Clamp to minimap bounds
        let clamped_x = wx.max(mm_x);
        let clamped_y = wy.max(mm_y);
        let clamped_w = ww.min(mm_x + MINIMAP_WIDTH - clamped_x);
        let clamped_h = wh.min(mm_y + MINIMAP_HEIGHT - clamped_y);
        if clamped_w <= 0 || clamped_h <= 0 {
            continue;
        }

        let color = if w.focused {
            FOCUSED_COLOR
        } else {
            WINDOW_COLOR
        };

        elements.push(SolidColorRenderElement::new(
            Id::new(),
            Rectangle::new(
                Point::<i32, Physical>::from((clamped_x, clamped_y)),
                Size::from((clamped_w, clamped_h)),
            ),
            0usize,
            color,
            Kind::Unspecified,
        ));
    }

    // Viewport indicator (the visible area on canvas)
    let (vx, vy) = to_mm(cam_x, cam_y);
    let vw = ((vis_w * scale) as i32).max(2);
    let vh = ((vis_h * scale) as i32).max(2);

    // Clamp viewport rect to minimap bounds
    let vx = vx.max(mm_x);
    let vy = vy.max(mm_y);
    let vw = vw.min(mm_x + MINIMAP_WIDTH - vx);
    let vh = vh.min(mm_y + MINIMAP_HEIGHT - vy);

    if vw > 0 && vh > 0 {
        let b = VIEWPORT_BORDER;
        // Top edge
        elements.push(SolidColorRenderElement::new(
            Id::new(),
            Rectangle::new(Point::<i32, Physical>::from((vx, vy)), Size::from((vw, b))),
            0usize,
            VIEWPORT_COLOR,
            Kind::Unspecified,
        ));
        // Bottom edge
        elements.push(SolidColorRenderElement::new(
            Id::new(),
            Rectangle::new(
                Point::<i32, Physical>::from((vx, vy + vh - b)),
                Size::from((vw, b)),
            ),
            0usize,
            VIEWPORT_COLOR,
            Kind::Unspecified,
        ));
        // Left edge
        elements.push(SolidColorRenderElement::new(
            Id::new(),
            Rectangle::new(
                Point::<i32, Physical>::from((vx, vy + b)),
                Size::from((b, vh - b * 2)),
            ),
            0usize,
            VIEWPORT_COLOR,
            Kind::Unspecified,
        ));
        // Right edge
        elements.push(SolidColorRenderElement::new(
            Id::new(),
            Rectangle::new(
                Point::<i32, Physical>::from((vx + vw - b, vy + b)),
                Size::from((b, vh - b * 2)),
            ),
            0usize,
            VIEWPORT_COLOR,
            Kind::Unspecified,
        ));
    }

    elements
}
