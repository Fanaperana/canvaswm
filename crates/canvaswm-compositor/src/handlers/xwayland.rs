//! XWayland support — X11 compatibility layer.
//!
//! Implements the `XwmHandler` trait so X11 applications can run
//! inside the CanvasWM compositor.

use crate::CanvasWM;
use smithay::{
    utils::{Logical, Rectangle},
    xwayland::{
        xwm::{Reorder, ResizeEdge, XwmId},
        X11Surface, X11Wm, XwmHandler,
    },
};

impl XwmHandler for CanvasWM {
    fn xwm_state(&mut self, _xwm: XwmId) -> &mut X11Wm {
        self.xwm.as_mut().expect("XWayland WM not initialized")
    }

    fn new_window(&mut self, _xwm: XwmId, _window: X11Surface) {}

    fn new_override_redirect_window(&mut self, _xwm: XwmId, _window: X11Surface) {}

    fn map_window_request(&mut self, _xwm: XwmId, window: X11Surface) {
        window.set_mapped(true).ok();
        // TODO: create a smithay Window wrapper for X11Surface and map it in the space
        tracing::info!("X11 window map request: {:?}", window.title());
    }

    fn mapped_override_redirect_window(&mut self, _xwm: XwmId, _window: X11Surface) {}

    fn unmapped_window(&mut self, _xwm: XwmId, window: X11Surface) {
        tracing::debug!("X11 window unmapped");
        window.set_mapped(false).ok();
    }

    fn destroyed_window(&mut self, _xwm: XwmId, _window: X11Surface) {
        tracing::debug!("X11 window destroyed");
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
        let x = x.unwrap_or(geo.loc.x);
        let y = y.unwrap_or(geo.loc.y);
        let w = w.unwrap_or(geo.size.w as u32);
        let h = h.unwrap_or(geo.size.h as u32);
        let _ = window.configure(Rectangle::from_loc_and_size((x, y), (w as i32, h as i32)));
    }

    fn configure_notify(
        &mut self,
        _xwm: XwmId,
        _window: X11Surface,
        _geometry: Rectangle<i32, Logical>,
        _above: Option<smithay::xwayland::xwm::X11Window>,
    ) {
    }

    fn resize_request(
        &mut self,
        _xwm: XwmId,
        _window: X11Surface,
        _button: u32,
        _resize_edge: ResizeEdge,
    ) {
        // TODO: initiate a resize grab
    }

    fn move_request(&mut self, _xwm: XwmId, _window: X11Surface, _button: u32) {
        // TODO: initiate a move grab
    }
}
