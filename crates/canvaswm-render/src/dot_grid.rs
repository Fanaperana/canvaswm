use canvaswm_canvas::Viewport;
use smithay::{
    backend::renderer::element::{solid::SolidColorRenderElement, Id, Kind},
    utils::{Physical, Point, Rectangle, Size},
};

/// Generate dot-grid render elements that scroll and zoom with the canvas.
///
/// Returns small solid-color squares placed at regular intervals in canvas space,
/// transformed to screen space via the viewport. The grid gives spatial awareness
/// when panning — dots drift with the canvas, not stuck to the screen.
pub fn dot_grid_elements(
    viewport: &Viewport,
    screen_size: (i32, i32),
    dot_color: [f32; 4],
    grid_spacing: f64,
    dot_size: f64,
) -> Vec<SolidColorRenderElement> {
    let zoom = viewport.zoom;
    let (cam_x, cam_y, vis_w, vis_h) = viewport.visible_rect();

    // Screen-space dot size — clamp to at least 1px, scale with zoom
    let screen_dot = (dot_size * zoom).max(1.0).round() as i32;
    let screen_spacing = grid_spacing * zoom;

    // If spacing is too small (zoomed way out), skip every Nth dot
    if screen_spacing < 4.0 {
        return Vec::new();
    }

    // Find the first grid line visible on each axis
    let start_x = (cam_x / grid_spacing).floor() as i64;
    let start_y = (cam_y / grid_spacing).floor() as i64;
    let end_x = ((cam_x + vis_w) / grid_spacing).ceil() as i64;
    let end_y = ((cam_y + vis_h) / grid_spacing).ceil() as i64;

    // Safety: limit total dots to prevent OOM on extreme zoom-out
    let count_x = (end_x - start_x + 1).min(500) as usize;
    let count_y = (end_y - start_y + 1).min(500) as usize;

    let mut elements = Vec::with_capacity(count_x * count_y);
    let dot_geo_size: Size<i32, Physical> = (screen_dot, screen_dot).into();

    // Fade dots based on zoom (more transparent when zoomed out)
    let alpha = if zoom < 0.5 {
        (zoom / 0.5).clamp(0.0, 1.0) as f32
    } else {
        1.0
    };
    let color = [
        dot_color[0] * alpha,
        dot_color[1] * alpha,
        dot_color[2] * alpha,
        dot_color[3] * alpha,
    ];

    for gy in start_y..=end_y.min(start_y + count_y as i64) {
        for gx in start_x..=end_x.min(start_x + count_x as i64) {
            let canvas_x = gx as f64 * grid_spacing;
            let canvas_y = gy as f64 * grid_spacing;

            let (sx, sy) = viewport.canvas_to_screen(canvas_x, canvas_y);

            // Skip if off-screen
            let ix = sx.round() as i32 - screen_dot / 2;
            let iy = sy.round() as i32 - screen_dot / 2;
            if ix + screen_dot < 0
                || iy + screen_dot < 0
                || ix > screen_size.0
                || iy > screen_size.1
            {
                continue;
            }

            let geo: Rectangle<i32, Physical> = Rectangle::new(Point::from((ix, iy)), dot_geo_size);

            elements.push(SolidColorRenderElement::new(
                Id::new(),
                geo,
                0usize,
                color,
                Kind::Unspecified,
            ));
        }
    }

    elements
}
