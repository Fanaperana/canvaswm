use crate::CanvasWM;
use smithay::{
    desktop::Window,
    input::pointer::{
        AxisFrame, ButtonEvent, GestureHoldBeginEvent, GestureHoldEndEvent,
        GesturePinchBeginEvent, GesturePinchEndEvent, GesturePinchUpdateEvent,
        GestureSwipeBeginEvent, GestureSwipeEndEvent, GestureSwipeUpdateEvent,
        GrabStartData as PointerGrabStartData, MotionEvent, PointerGrab,
        PointerInnerHandle, RelativeMotionEvent,
    },
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{Logical, Point},
};

pub struct MoveSurfaceGrab {
    pub start_data: PointerGrabStartData<CanvasWM>,
    pub window: Window,
    pub initial_window_location: Point<i32, Logical>,
}

impl PointerGrab<CanvasWM> for MoveSurfaceGrab {
    fn motion(
        &mut self,
        data: &mut CanvasWM,
        handle: &mut PointerInnerHandle<'_, CanvasWM>,
        _focus: Option<(WlSurface, Point<f64, Logical>)>,
        event: &MotionEvent,
    ) {
        handle.motion(data, None, event);

        let delta = event.location - self.start_data.location;
        let mut new_location = self.initial_window_location.to_f64() + delta;

        // Apply snapping if enabled
        if data.config.snap.enabled {
            let geo = self.window.geometry();
            let moving_rect = (new_location.x, new_location.y, geo.size.w as f64, geo.size.h as f64);
            let others = data.space.elements()
                .filter(|w| *w != &self.window)
                .filter_map(|w| {
                    let loc = data.space.element_location(w)?;
                    let size = w.geometry().size;
                    Some((loc.x as f64, loc.y as f64, size.w as f64, size.h as f64))
                });
            let (snap_x, snap_y) = canvaswm_canvas::compute_snap(
                moving_rect,
                others,
                data.config.snap.gap,
                data.config.snap.distance,
            );
            if let Some(sx) = snap_x {
                new_location.x = sx;
            }
            if let Some(sy) = snap_y {
                new_location.y = sy;
            }
        }

        data.space
            .map_element(self.window.clone(), new_location.to_i32_round(), true);
    }

    fn relative_motion(
        &mut self,
        data: &mut CanvasWM,
        handle: &mut PointerInnerHandle<'_, CanvasWM>,
        focus: Option<(WlSurface, Point<f64, Logical>)>,
        event: &RelativeMotionEvent,
    ) {
        handle.relative_motion(data, focus, event);
    }

    fn button(
        &mut self,
        data: &mut CanvasWM,
        handle: &mut PointerInnerHandle<'_, CanvasWM>,
        event: &ButtonEvent,
    ) {
        handle.button(data, event);
        const BTN_LEFT: u32 = 0x110;
        if !handle.current_pressed().contains(&BTN_LEFT) {
            handle.unset_grab(self, data, event.serial, event.time, true);
        }
    }

    fn axis(
        &mut self,
        data: &mut CanvasWM,
        handle: &mut PointerInnerHandle<'_, CanvasWM>,
        details: AxisFrame,
    ) {
        handle.axis(data, details)
    }

    fn frame(&mut self, data: &mut CanvasWM, handle: &mut PointerInnerHandle<'_, CanvasWM>) {
        handle.frame(data);
    }

    fn gesture_swipe_begin(
        &mut self,
        data: &mut CanvasWM,
        handle: &mut PointerInnerHandle<'_, CanvasWM>,
        event: &GestureSwipeBeginEvent,
    ) {
        handle.gesture_swipe_begin(data, event)
    }

    fn gesture_swipe_update(
        &mut self,
        data: &mut CanvasWM,
        handle: &mut PointerInnerHandle<'_, CanvasWM>,
        event: &GestureSwipeUpdateEvent,
    ) {
        handle.gesture_swipe_update(data, event)
    }

    fn gesture_swipe_end(
        &mut self,
        data: &mut CanvasWM,
        handle: &mut PointerInnerHandle<'_, CanvasWM>,
        event: &GestureSwipeEndEvent,
    ) {
        handle.gesture_swipe_end(data, event)
    }

    fn gesture_pinch_begin(
        &mut self,
        data: &mut CanvasWM,
        handle: &mut PointerInnerHandle<'_, CanvasWM>,
        event: &GesturePinchBeginEvent,
    ) {
        handle.gesture_pinch_begin(data, event)
    }

    fn gesture_pinch_update(
        &mut self,
        data: &mut CanvasWM,
        handle: &mut PointerInnerHandle<'_, CanvasWM>,
        event: &GesturePinchUpdateEvent,
    ) {
        handle.gesture_pinch_update(data, event)
    }

    fn gesture_pinch_end(
        &mut self,
        data: &mut CanvasWM,
        handle: &mut PointerInnerHandle<'_, CanvasWM>,
        event: &GesturePinchEndEvent,
    ) {
        handle.gesture_pinch_end(data, event)
    }

    fn gesture_hold_begin(
        &mut self,
        data: &mut CanvasWM,
        handle: &mut PointerInnerHandle<'_, CanvasWM>,
        event: &GestureHoldBeginEvent,
    ) {
        handle.gesture_hold_begin(data, event)
    }

    fn gesture_hold_end(
        &mut self,
        data: &mut CanvasWM,
        handle: &mut PointerInnerHandle<'_, CanvasWM>,
        event: &GestureHoldEndEvent,
    ) {
        handle.gesture_hold_end(data, event)
    }

    fn start_data(&self) -> &PointerGrabStartData<CanvasWM> {
        &self.start_data
    }

    fn unset(&mut self, _data: &mut CanvasWM) {}
}
