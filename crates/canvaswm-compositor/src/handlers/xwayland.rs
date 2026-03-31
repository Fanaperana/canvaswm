//! XWayland support — X11 compatibility layer.
//!
//! Implements the `XwmHandler` and `XWaylandShellHandler` traits so X11
//! applications can run inside the CanvasWM compositor.

use crate::{
    grabs::{MoveSurfaceGrab, ResizeSurfaceGrab},
    CanvasWM,
};
use smithay::{
    delegate_xwayland_shell,
    desktop::Window,
    input::pointer::{Focus, GrabStartData as PointerGrabStartData},
    utils::{Logical, Point, Rectangle, SERIAL_COUNTER},
    wayland::xwayland_shell::{XWaylandShellHandler, XWaylandShellState},
    xwayland::{
        xwm::{Reorder, ResizeEdge as X11ResizeEdge, XwmId},
        X11Surface, X11Wm, XwmHandler,
    },
};

impl XWaylandShellHandler for CanvasWM {
    fn xwayland_shell_state(&mut self) -> &mut XWaylandShellState {
        &mut self.xwayland_shell_state
    }
}

delegate_xwayland_shell!(CanvasWM);

impl XwmHandler for CanvasWM {
    fn xwm_state(&mut self, _xwm: XwmId) -> &mut X11Wm {
        self.xwm.as_mut().expect("XWayland WM not initialized")
    }

    fn new_window(&mut self, _xwm: XwmId, _window: X11Surface) {}

    fn new_override_redirect_window(&mut self, _xwm: XwmId, window: X11Surface) {
        // Override-redirect windows (menus, tooltips) map themselves; just place them
        // at the canvas location passed through configure_request.
        let geo = window.geometry();
        let smithay_window = Window::new_x11_window(window);
        self.space.map_element(smithay_window, geo.loc, true);
    }

    fn map_window_request(&mut self, _xwm: XwmId, window: X11Surface) {
        window.set_mapped(true).ok();

        // Choose the initial canvas location: viewport center with collision avoidance.
        let center_sx = self.viewport.width / 2.0;
        let center_sy = self.viewport.height / 2.0;
        let (cx, cy) = self.viewport.screen_to_canvas(center_sx, center_sy);

        let existing: Vec<(f64, f64, f64, f64)> = self
            .space
            .elements()
            .filter_map(|w| {
                let loc = self.space.element_location(w)?;
                let size = w.geometry().size;
                Some((loc.x as f64, loc.y as f64, size.w as f64, size.h as f64))
            })
            .collect();
        let gap = if self.config.snap.enabled {
            self.config.snap.gap
        } else {
            20.0
        };
        let (nx, ny) = canvaswm_canvas::find_free_position(cx, cy, 0.0, 0.0, &existing, gap);

        let smithay_window = Window::new_x11_window(window);
        self.space
            .map_element(smithay_window.clone(), (nx as i32, ny as i32), false);

        // Raise and focus the new X11 window.
        let serial = SERIAL_COUNTER.next_serial();
        self.space.raise_element(&smithay_window, true);
        if let Some(surface) = smithay_window.x11_surface().and_then(|x| x.wl_surface()) {
            self.seat
                .get_keyboard()
                .unwrap()
                .set_focus(self, Some(surface), serial);
        }
        self.update_focus_history(&smithay_window);

        let est_w = 600.0_f64;
        let est_h = 400.0_f64;
        self.viewport.animate_to(nx + est_w / 2.0, ny + est_h / 2.0);

        tracing::info!(
            "X11 window mapped: {:?} at ({}, {})",
            smithay_window.x11_surface().map(|s| s.title()),
            nx,
            ny
        );
    }

    fn mapped_override_redirect_window(&mut self, _xwm: XwmId, _window: X11Surface) {}

    fn unmapped_window(&mut self, _xwm: XwmId, window: X11Surface) {
        window.set_mapped(false).ok();
        // Remove from space if we have a Window wrapper for it.
        let to_remove = self
            .space
            .elements()
            .find(|w| {
                w.x11_surface()
                    .map_or(false, |s| s.window_id() == window.window_id())
            })
            .cloned();
        if let Some(w) = to_remove {
            self.focus_history.retain(|x| x != &w);
            self.space.unmap_elem(&w);
        }
    }

    fn destroyed_window(&mut self, _xwm: XwmId, window: X11Surface) {
        let to_remove = self
            .space
            .elements()
            .find(|w| {
                w.x11_surface()
                    .map_or(false, |s| s.window_id() == window.window_id())
            })
            .cloned();
        if let Some(w) = to_remove {
            self.focus_history.retain(|x| x != &w);
            self.space.unmap_elem(&w);
        }
    }

    fn configure_request(
        &mut self,
        _xwm: XwmId,
        window: X11Surface,
        x: Option<i32>,
        y: Option<i32>,
        w: Option<u32>,
        h: Option<u32>,
        _reorder: Option<Reorder>,
    ) {
        let geo = window.geometry();
        let nx = x.unwrap_or(geo.loc.x);
        let ny = y.unwrap_or(geo.loc.y);
        let nw = w.unwrap_or(geo.size.w as u32) as i32;
        let nh = h.unwrap_or(geo.size.h as u32) as i32;
        let _ = window.configure(Rectangle::new((nx, ny).into(), (nw, nh).into()));
    }

    fn configure_notify(
        &mut self,
        _xwm: XwmId,
        window: X11Surface,
        geometry: Rectangle<i32, Logical>,
        _above: Option<smithay::xwayland::xwm::X11Window>,
    ) {
        // Keep the space location in sync when XWayland reports a geometry change.
        let smithay_window = self
            .space
            .elements()
            .find(|w| {
                w.x11_surface()
                    .map_or(false, |s| s.window_id() == window.window_id())
            })
            .cloned();
        if let Some(w) = smithay_window {
            self.space.map_element(w, geometry.loc, false);
        }
    }

    fn resize_request(
        &mut self,
        _xwm: XwmId,
        window: X11Surface,
        button: u32,
        resize_edge: X11ResizeEdge,
    ) {
        let Some(pointer) = self.seat.get_pointer() else {
            return;
        };
        let serial = SERIAL_COUNTER.next_serial();
        let location = pointer.current_location();

        let smithay_window = self
            .space
            .elements()
            .find(|w| {
                w.x11_surface()
                    .map_or(false, |s| s.window_id() == window.window_id())
            })
            .cloned();

        let Some(smithay_window) = smithay_window else {
            return;
        };
        let Some(initial_window_location) = self.space.element_location(&smithay_window) else {
            return;
        };

        let focus = pointer
            .current_focus()
            .map(|f| (f, Point::from((0.0, 0.0))));

        let start_data = PointerGrabStartData {
            focus,
            button,
            location,
        };
        let initial_window_size = smithay_window.geometry().size;

        let grab = ResizeSurfaceGrab::start(
            start_data,
            smithay_window,
            resize_edge.into(),
            Rectangle::new(initial_window_location, initial_window_size),
        );
        pointer.set_grab(self, grab, serial, Focus::Clear);
    }

    fn move_request(&mut self, _xwm: XwmId, window: X11Surface, button: u32) {
        let Some(pointer) = self.seat.get_pointer() else {
            return;
        };
        let serial = SERIAL_COUNTER.next_serial();
        let location = pointer.current_location();

        let smithay_window = self
            .space
            .elements()
            .find(|w| {
                w.x11_surface()
                    .map_or(false, |s| s.window_id() == window.window_id())
            })
            .cloned();
        let Some(smithay_window) = smithay_window else {
            return;
        };
        let Some(initial_window_location) = self.space.element_location(&smithay_window) else {
            return;
        };

        let focus = pointer
            .current_focus()
            .map(|f| (f, Point::from((0.0, 0.0))));

        let start_data = PointerGrabStartData {
            focus,
            button,
            location,
        };
        let grab = MoveSurfaceGrab {
            start_data,
            window: smithay_window,
            initial_window_location,
        };
        pointer.set_grab(self, grab, serial, Focus::Clear);
    }
}
