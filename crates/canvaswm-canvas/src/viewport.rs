/// The infinite canvas viewport.
///
/// Windows live on a 2D plane with coordinates in "canvas space".
/// The screen is a viewport into this canvas, defined by a camera position and zoom level.
///
/// Coordinate system:
/// - **Canvas space**: absolute positions where windows live (infinite 2D plane)
/// - **Screen space**: pixel positions on the physical output (0,0 = top-left)
///
/// Transform: `screen = (canvas - camera) * zoom`
/// Inverse:   `canvas = screen / zoom + camera`
use std::time::Duration;

/// Hard minimum zoom level — prevents division by zero.
pub const MIN_ZOOM_FLOOR: f64 = 0.001;
/// Default maximum zoom level.
pub const MAX_ZOOM: f64 = 1.0;

#[derive(Debug, Clone)]
pub struct Viewport {
    /// Camera position: the canvas-space coordinate at the top-left of the screen.
    pub camera_x: f64,
    pub camera_y: f64,
    /// Zoom level: 1.0 = normal, <1.0 = zoomed out (see more), >1.0 = zoomed in.
    pub zoom: f64,
    /// Viewport size in screen pixels (updated on resize).
    pub width: f64,
    pub height: f64,

    // Animation targets
    /// Target camera position for smooth animation (None = no animation).
    pub camera_target: Option<(f64, f64)>,
    /// Target zoom level for smooth animation.
    pub zoom_target: Option<f64>,
    /// Canvas point that should stay on-screen center during zoom animation.
    pub zoom_animation_center: Option<(f64, f64)>,
    /// Saved camera + zoom for HomeToggle return.
    pub home_return: Option<(f64, f64, f64)>,

    /// Configurable snap threshold.
    pub snap_threshold: f64,
    /// Configurable max zoom.
    pub max_zoom: f64,
    /// Animation speed (lerp factor, 0–1, applied per frame at 60fps).
    pub animation_speed: f64,
}

impl Default for Viewport {
    fn default() -> Self {
        Self {
            camera_x: 0.0,
            camera_y: 0.0,
            zoom: 1.0,
            width: 1280.0,
            height: 720.0,
            camera_target: None,
            zoom_target: None,
            zoom_animation_center: None,
            home_return: None,
            snap_threshold: 0.05,
            max_zoom: MAX_ZOOM,
            animation_speed: 0.3,
        }
    }
}

impl Viewport {
    /// Pan the viewport by a delta in screen pixels.
    /// Converts screen-space movement to canvas-space movement.
    /// Cancels any active animation.
    pub fn pan(&mut self, screen_dx: f64, screen_dy: f64) {
        self.camera_target = None;
        self.zoom_target = None;
        self.zoom_animation_center = None;
        self.camera_x -= screen_dx / self.zoom;
        self.camera_y -= screen_dy / self.zoom;
    }

    /// Zoom around a point in screen coordinates (cursor-anchored zoom).
    /// The canvas point under (`screen_x`, `screen_y`) stays fixed after zoom.
    pub fn zoom_at(&mut self, screen_x: f64, screen_y: f64, factor: f64) {
        let new_zoom = (self.zoom * factor).clamp(MIN_ZOOM_FLOOR, self.max_zoom);

        // Canvas point under cursor before zoom
        let canvas_x = self.camera_x + screen_x / self.zoom;
        let canvas_y = self.camera_y + screen_y / self.zoom;

        self.zoom = self.snap_zoom(new_zoom);

        // Adjust camera so the same canvas point stays under cursor
        self.camera_x = canvas_x - screen_x / self.zoom;
        self.camera_y = canvas_y - screen_y / self.zoom;
    }

    /// Set zoom to a specific level, anchored at screen center.
    pub fn set_zoom(&mut self, zoom: f64) {
        let cx = self.width / 2.0;
        let cy = self.height / 2.0;
        let factor = zoom / self.zoom;
        self.zoom_at(cx, cy, factor);
    }

    /// Reset viewport to origin with zoom 1.0.
    pub fn reset(&mut self) {
        self.camera_x = 0.0;
        self.camera_y = 0.0;
        self.zoom = 1.0;
        self.camera_target = None;
        self.zoom_target = None;
        self.zoom_animation_center = None;
    }

    /// Convert screen coordinates to canvas coordinates.
    pub fn screen_to_canvas(&self, screen_x: f64, screen_y: f64) -> (f64, f64) {
        (
            self.camera_x + screen_x / self.zoom,
            self.camera_y + screen_y / self.zoom,
        )
    }

    /// Convert canvas coordinates to screen coordinates.
    pub fn canvas_to_screen(&self, canvas_x: f64, canvas_y: f64) -> (f64, f64) {
        (
            (canvas_x - self.camera_x) * self.zoom,
            (canvas_y - self.camera_y) * self.zoom,
        )
    }

    /// The visible canvas rectangle (in canvas-space coordinates).
    pub fn visible_rect(&self) -> (f64, f64, f64, f64) {
        let w = self.width / self.zoom;
        let h = self.height / self.zoom;
        (self.camera_x, self.camera_y, w, h)
    }

    /// Camera position that centers a canvas point on screen.
    pub fn camera_to_center(&self, canvas_x: f64, canvas_y: f64) -> (f64, f64) {
        (
            canvas_x - self.width / (2.0 * self.zoom),
            canvas_y - self.height / (2.0 * self.zoom),
        )
    }

    /// Center the viewport on a canvas point (instant).
    pub fn center_on(&mut self, canvas_x: f64, canvas_y: f64) {
        let (cx, cy) = self.camera_to_center(canvas_x, canvas_y);
        self.camera_x = cx;
        self.camera_y = cy;
    }

    /// Animate camera to center on a canvas point.
    pub fn animate_to(&mut self, canvas_x: f64, canvas_y: f64) {
        let (cx, cy) = self.camera_to_center(canvas_x, canvas_y);
        self.camera_target = Some((cx, cy));
    }

    /// Animate to center a window, also animating zoom to target_zoom.
    pub fn animate_to_window(&mut self, canvas_x: f64, canvas_y: f64, target_zoom: f64) {
        self.zoom_animation_center = Some((canvas_x, canvas_y));
        self.zoom_target = Some(target_zoom.clamp(MIN_ZOOM_FLOOR, self.max_zoom));
        // camera_target is derived from zoom_animation_center during tick
    }

    /// Center on a rectangular region, adjusting zoom to fit with padding.
    pub fn zoom_to_fit(&mut self, x: f64, y: f64, w: f64, h: f64, padding: f64) {
        let padded_w = w + padding * 2.0;
        let padded_h = h + padding * 2.0;
        let zoom_x = self.width / padded_w;
        let zoom_y = self.height / padded_h;
        let new_zoom = zoom_x.min(zoom_y).clamp(MIN_ZOOM_FLOOR, self.max_zoom);
        let center_x = x + w / 2.0;
        let center_y = y + h / 2.0;
        self.animate_to_window(center_x, center_y, new_zoom);
    }

    /// Compute zoom level that fits a bbox inside viewport with padding.
    pub fn fit_zoom(&self, w: f64, h: f64, padding: f64) -> f64 {
        let padded_w = w + padding * 2.0;
        let padded_h = h + padding * 2.0;
        let zoom_x = self.width / padded_w;
        let zoom_y = self.height / padded_h;
        zoom_x.min(zoom_y).clamp(MIN_ZOOM_FLOOR, self.max_zoom)
    }

    /// Dynamic minimum zoom — allows zooming out far enough to see all windows.
    pub fn dynamic_min_zoom(&self, bbox_w: f64, bbox_h: f64, padding: f64) -> f64 {
        let fit = self.fit_zoom(bbox_w, bbox_h, padding);
        (fit * 0.5).max(MIN_ZOOM_FLOOR)
    }

    /// Update viewport dimensions (call on output resize).
    pub fn resize(&mut self, width: f64, height: f64) {
        self.width = width;
        self.height = height;
    }

    /// Tick animations. Returns true if viewport changed (needs redraw).
    pub fn tick_animations(&mut self, dt: Duration) -> bool {
        let mut changed = false;

        let factor = self.animation_factor(dt);

        // Zoom animation
        if let Some(target_zoom) = self.zoom_target {
            let dz = target_zoom - self.zoom;
            if dz.abs() < 0.001 {
                self.zoom = target_zoom;
                self.zoom_target = None;
            } else {
                self.zoom += dz * factor;
            }
            changed = true;
        }

        // Combined zoom + camera animation via center point
        if let Some((center_x, center_y)) = self.zoom_animation_center {
            let vc_x = self.width / 2.0;
            let vc_y = self.height / 2.0;

            let current_center_x = self.camera_x + vc_x / self.zoom;
            let current_center_y = self.camera_y + vc_y / self.zoom;

            let cx = current_center_x + (center_x - current_center_x) * factor;
            let cy = current_center_y + (center_y - current_center_y) * factor;

            self.camera_x = cx - vc_x / self.zoom;
            self.camera_y = cy - vc_y / self.zoom;

            // Check if converged
            if self.zoom_target.is_none() {
                let dx = center_x - current_center_x;
                let dy = center_y - current_center_y;
                if dx * dx + dy * dy < 0.25 {
                    self.camera_x = center_x - vc_x / self.zoom;
                    self.camera_y = center_y - vc_y / self.zoom;
                    self.zoom_animation_center = None;
                } else {
                    // Hand off to camera animation
                    let final_cam_x = center_x - vc_x / self.zoom;
                    let final_cam_y = center_y - vc_y / self.zoom;
                    self.zoom_animation_center = None;
                    self.camera_target = Some((final_cam_x, final_cam_y));
                }
            }
            changed = true;
        }

        // Camera animation (standalone or post-zoom handoff)
        if let Some((target_x, target_y)) = self.camera_target {
            let dx = target_x - self.camera_x;
            let dy = target_y - self.camera_y;

            if dx * dx + dy * dy < 0.25 {
                self.camera_x = target_x;
                self.camera_y = target_y;
                self.camera_target = None;
            } else {
                self.camera_x += dx * factor;
                self.camera_y += dy * factor;
            }
            changed = true;
        }

        changed
    }

    /// Whether any animation is in progress.
    pub fn is_animating(&self) -> bool {
        self.camera_target.is_some()
            || self.zoom_target.is_some()
            || self.zoom_animation_center.is_some()
    }

    /// Frame-rate independent lerp factor.
    fn animation_factor(&self, dt: Duration) -> f64 {
        let dt_secs = dt.as_secs_f64();
        1.0 - (1.0 - self.animation_speed).powf(dt_secs * 60.0)
    }

    /// Snap zoom to 1.0 if within dead zone.
    fn snap_zoom(&self, z: f64) -> f64 {
        if (z - 1.0).abs() < self.snap_threshold {
            1.0
        } else {
            z
        }
    }

    /// Home toggle: go to (0,0) zoom 1.0, or return to saved position.
    pub fn home_toggle(&mut self) {
        if let Some((cx, cy, z)) = self.home_return.take() {
            self.animate_to_window(cx + self.width / (2.0 * z), cy + self.height / (2.0 * z), z);
        } else {
            // Save current state
            self.home_return = Some((self.camera_x, self.camera_y, self.zoom));
            self.animate_to_window(0.0, 0.0, 1.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn screen_canvas_round_trip() {
        let vp = Viewport {
            camera_x: 100.0,
            camera_y: 200.0,
            zoom: 0.5,
            ..Default::default()
        };
        let (cx, cy) = vp.screen_to_canvas(400.0, 300.0);
        let (sx, sy) = vp.canvas_to_screen(cx, cy);
        assert!((sx - 400.0).abs() < 1e-9);
        assert!((sy - 300.0).abs() < 1e-9);
    }

    #[test]
    fn zoom_at_keeps_point_fixed() {
        let mut vp = Viewport::default();
        let (cx_before, cy_before) = vp.screen_to_canvas(640.0, 360.0);
        vp.zoom_at(640.0, 360.0, 0.5);
        let (cx_after, cy_after) = vp.screen_to_canvas(640.0, 360.0);
        assert!((cx_before - cx_after).abs() < 1e-9);
        assert!((cy_before - cy_after).abs() < 1e-9);
    }

    #[test]
    fn zoom_out_increases_visible_area() {
        let mut vp = Viewport::default();
        let (_, _, w1, h1) = vp.visible_rect();
        vp.set_zoom(0.5);
        let (_, _, w2, h2) = vp.visible_rect();
        assert!(w2 > w1);
        assert!(h2 > h1);
    }

    #[test]
    fn reset_returns_to_origin() {
        let mut vp = Viewport::default();
        vp.pan(100.0, 200.0);
        vp.zoom_at(0.0, 0.0, 2.0);
        vp.reset();
        assert_eq!(vp.camera_x, 0.0);
        assert_eq!(vp.camera_y, 0.0);
        assert_eq!(vp.zoom, 1.0);
    }
}
