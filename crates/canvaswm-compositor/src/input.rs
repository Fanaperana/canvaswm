use std::time::Instant;

use smithay::{
    backend::input::{
        Axis, AxisSource, ButtonState, Event, InputBackend,
        InputEvent, KeyState, KeyboardKeyEvent, PointerAxisEvent, PointerButtonEvent,
        PointerMotionAbsoluteEvent,
    },
    input::{
        keyboard::{keysyms, FilterResult},
        pointer::{AxisFrame, ButtonEvent, Focus, GrabStartData, MotionEvent},
    },
    reexports::wayland_server::protocol::wl_surface::WlSurface,
    utils::{Logical, Point, SERIAL_COUNTER},
};

use canvaswm_input::{Action, Direction};

use crate::{
    grabs::{MoveSurfaceGrab, ResizeSurfaceGrab},
    state::CanvasWM,
};

/// Mouse button codes from linux/input-event-codes.h
const BTN_LEFT: u32 = 0x110;
const BTN_RIGHT: u32 = 0x111;

impl CanvasWM {
    pub fn process_input_event<I: InputBackend>(&mut self, event: InputEvent<I>) {
        match event {
            InputEvent::Keyboard { event, .. } => {
                self.handle_keyboard(event);
            }
            InputEvent::PointerMotionAbsolute { event, .. } => {
                self.handle_pointer_motion(event);
            }
            InputEvent::PointerButton { event, .. } => {
                self.handle_pointer_button(event);
            }
            InputEvent::PointerAxis { event, .. } => {
                self.handle_pointer_axis(event);
            }
            _ => {}
        }
    }

    fn handle_keyboard<I: InputBackend>(&mut self, event: impl KeyboardKeyEvent<I>) {
        let serial = SERIAL_COUNTER.next_serial();
        let time = Event::time_msec(&event);
        let key_code = event.key_code();
        let state = event.state();

        let Some(keyboard) = self.seat.get_keyboard() else { return };

        let action = keyboard.input::<Action, _>(
            self,
            key_code,
            state,
            serial,
            time,
            |_, modifiers, keysym| {
                let sym = keysym.modified_sym().raw();

                if state == KeyState::Pressed {
                    // Alt-Tab cycling
                    if modifiers.alt && sym == keysyms::KEY_Tab {
                        if modifiers.shift {
                            return FilterResult::Intercept(Action::CycleBackward);
                        }
                        return FilterResult::Intercept(Action::CycleForward);
                    }

                    if modifiers.logo {
                        match sym {
                            keysyms::KEY_Return => return FilterResult::Intercept(Action::SpawnTerminal),
                            keysyms::KEY_d => return FilterResult::Intercept(Action::SpawnLauncher),
                            keysyms::KEY_q => return FilterResult::Intercept(Action::CloseWindow),
                            keysyms::KEY_0 => return FilterResult::Intercept(Action::ResetCanvas),
                            keysyms::KEY_equal => return FilterResult::Intercept(Action::ZoomIn),
                            keysyms::KEY_minus => return FilterResult::Intercept(Action::ZoomOut),
                            keysyms::KEY_w => return FilterResult::Intercept(Action::ZoomToFit),
                            keysyms::KEY_c => return FilterResult::Intercept(Action::CenterWindow),
                            keysyms::KEY_f => return FilterResult::Intercept(Action::ToggleFullscreen),
                            keysyms::KEY_Home => return FilterResult::Intercept(Action::HomeToggle),
                            keysyms::KEY_r => return FilterResult::Intercept(Action::ReloadConfig),
                            keysyms::KEY_Escape => return FilterResult::Intercept(Action::Quit),
                            // Directional navigation: Super+Arrow
                            keysyms::KEY_Left => return FilterResult::Intercept(Action::NavigateDirection(Direction::Left)),
                            keysyms::KEY_Right => return FilterResult::Intercept(Action::NavigateDirection(Direction::Right)),
                            keysyms::KEY_Up => return FilterResult::Intercept(Action::NavigateDirection(Direction::Up)),
                            keysyms::KEY_Down => return FilterResult::Intercept(Action::NavigateDirection(Direction::Down)),
                            _ => {}
                        }

                        // Super+Shift+Arrow = nudge window
                        if modifiers.shift {
                            match sym {
                                keysyms::KEY_Left => return FilterResult::Intercept(Action::NudgeWindow(Direction::Left)),
                                keysyms::KEY_Right => return FilterResult::Intercept(Action::NudgeWindow(Direction::Right)),
                                keysyms::KEY_Up => return FilterResult::Intercept(Action::NudgeWindow(Direction::Up)),
                                keysyms::KEY_Down => return FilterResult::Intercept(Action::NudgeWindow(Direction::Down)),
                                _ => {}
                            }
                        }
                    }
                }

                // End Alt-Tab cycle on Alt release
                if state == KeyState::Released {
                    // Alt released while cycling — commit selection
                    if (sym == keysyms::KEY_Alt_L || sym == keysyms::KEY_Alt_R) && !modifiers.alt {
                        // We can't intercept on release easily; handle in main
                    }
                }

                FilterResult::Forward
            },
        );

        // End cycle when Alt is released
        if state == KeyState::Released {
            let modifiers = keyboard.modifier_state();
            if !modifiers.alt && self.cycle_state.is_some() {
                self.end_cycle();
            }
        }

        if let Some(action) = action {
            self.execute_action(action);
        }
    }

    fn execute_action(&mut self, action: Action) {
        match action {
            Action::SpawnTerminal => {
                for term in &["alacritty", "foot", "kitty", "gnome-terminal"] {
                    if std::process::Command::new(term).spawn().is_ok() {
                        break;
                    }
                }
            }
            Action::SpawnLauncher => {
                for launcher in &["fuzzel", "wofi", "rofi -show drun", "bemenu-run"] {
                    let parts: Vec<&str> = launcher.split_whitespace().collect();
                    let mut cmd = std::process::Command::new(parts[0]);
                    for arg in &parts[1..] {
                        cmd.arg(arg);
                    }
                    if cmd.spawn().is_ok() {
                        break;
                    }
                }
            }
            Action::CloseWindow => {
                // Close focused window (most recent in focus history)
                if let Some(window) = self.focus_history.first().cloned() {
                    if let Some(toplevel) = window.toplevel() {
                        toplevel.send_close();
                    }
                } else if let Some(window) = self.space.elements().last().cloned() {
                    if let Some(toplevel) = window.toplevel() {
                        toplevel.send_close();
                    }
                }
            }
            Action::ResetCanvas => {
                self.viewport.reset();
                self.pan_momentum.stop();
            }
            Action::ZoomIn => {
                let (sx, sy) = (self.cursor_pos.x, self.cursor_pos.y);
                self.viewport.zoom_at(sx, sy, 1.15);
            }
            Action::ZoomOut => {
                let (sx, sy) = (self.cursor_pos.x, self.cursor_pos.y);
                self.viewport.zoom_at(sx, sy, 1.0 / 1.15);
            }
            Action::ZoomToFit => {
                if let Some((x, y, w, h)) = self.all_windows_bbox() {
                    self.viewport.zoom_to_fit(x, y, w, h, 50.0);
                }
            }
            Action::CenterWindow => {
                // Center on focused or most-recently-focused window
                if let Some(window) = self.focus_history.first().cloned() {
                    if let Some(loc) = self.space.element_location(&window) {
                        let size = window.geometry().size;
                        let cx = loc.x as f64 + size.w as f64 / 2.0;
                        let cy = loc.y as f64 + size.h as f64 / 2.0;
                        self.viewport.animate_to(cx, cy);
                    }
                }
            }
            Action::ToggleFullscreen => {
                self.toggle_fullscreen();
            }
            Action::FitWindow => {
                if let Some(window) = self.focus_history.first().cloned() {
                    if let Some(loc) = self.space.element_location(&window) {
                        let size = window.geometry().size;
                        let w = size.w as f64;
                        let h = size.h as f64;
                        let cx = loc.x as f64 + w / 2.0;
                        let cy = loc.y as f64 + h / 2.0;
                        let target_zoom = self.viewport.fit_zoom(w, h, 50.0);
                        self.viewport.animate_to_window(cx, cy, target_zoom);
                    }
                }
            }
            Action::NavigateDirection(dir) => {
                self.navigate_direction(dir.to_unit_vec());
            }
            Action::PanDirection(dir) => {
                let (dx, dy) = dir.to_unit_vec();
                let step = self.config.navigation.pan_step;
                self.viewport.pan(dx * step, dy * step);
            }
            Action::NudgeWindow(dir) => {
                let (dx, dy) = dir.to_unit_vec();
                let step = self.config.navigation.nudge_step;
                if let Some(window) = self.focus_history.first().cloned() {
                    if let Some(loc) = self.space.element_location(&window) {
                        let new_loc = smithay::utils::Point::<i32, Logical>::from((
                            loc.x + (dx * step) as i32,
                            loc.y + (dy * step) as i32,
                        ));
                        self.space.map_element(window, new_loc, false);
                    }
                }
            }
            Action::HomeToggle => {
                self.viewport.home_toggle();
            }
            Action::CycleForward => {
                self.cycle_forward();
            }
            Action::CycleBackward => {
                self.cycle_backward();
            }
            Action::ReloadConfig => {
                self.reload_config();
            }
            Action::Exec(cmd) => {
                if let Err(e) = std::process::Command::new("sh").arg("-c").arg(&cmd).spawn() {
                    tracing::warn!("Failed to exec '{}': {}", cmd, e);
                }
            }
            Action::Quit => {
                self.loop_signal.stop();
            }
        }
    }

    /// Toggle fullscreen for the focused window.
    fn toggle_fullscreen(&mut self) {
        use crate::state::FullscreenState;

        // If already fullscreen, restore
        if let Some(fs) = self.fullscreen.take() {
            self.space.map_element(fs.window.clone(), fs.saved_location, false);
            self.viewport.camera_x = fs.saved_camera.0;
            self.viewport.camera_y = fs.saved_camera.1;
            self.viewport.zoom = fs.saved_zoom;
            // Restore window size
            if let Some(toplevel) = fs.window.toplevel() {
                toplevel.with_pending_state(|s| {
                    s.size = Some((fs.saved_size.0, fs.saved_size.1).into());
                });
                toplevel.send_pending_configure();
            }
            return;
        }

        // Fullscreen the focused window
        if let Some(window) = self.focus_history.first().cloned() {
            let saved_location = self.space.element_location(&window).unwrap_or_default();
            let saved_camera = (self.viewport.camera_x, self.viewport.camera_y);
            let saved_zoom = self.viewport.zoom;
            let size = window.geometry().size;
            let saved_size = (size.w, size.h);

            // Set fullscreen state
            self.fullscreen = Some(FullscreenState {
                window: window.clone(),
                saved_location,
                saved_camera,
                saved_zoom,
                saved_size,
            });

            // Position at viewport origin and resize to fill
            let canvas_x = self.viewport.camera_x as i32;
            let canvas_y = self.viewport.camera_y as i32;
            let screen_w = (self.viewport.width / self.viewport.zoom) as i32;
            let screen_h = (self.viewport.height / self.viewport.zoom) as i32;

            self.space.map_element(
                window.clone(),
                smithay::utils::Point::<i32, Logical>::from((canvas_x, canvas_y)),
                true,
            );

            if let Some(toplevel) = window.toplevel() {
                toplevel.with_pending_state(|s| {
                    s.size = Some((screen_w, screen_h).into());
                });
                toplevel.send_pending_configure();
            }
        }
    }

    fn handle_pointer_motion<I: InputBackend>(
        &mut self,
        event: impl PointerMotionAbsoluteEvent<I>,
    ) {
        let Some(output) = self.space.outputs().next() else { return };
        let Some(output_geo) = self.space.output_geometry(output) else { return };

        // Screen-space position
        let screen_pos = event.position_transformed(output_geo.size) + output_geo.loc.to_f64();
        let old_cursor = self.cursor_pos;
        self.cursor_pos = Point::from((screen_pos.x, screen_pos.y));

        // If panning (Super+LMB), move the viewport
        if self.panning {
            let dx = self.cursor_pos.x - old_cursor.x;
            let dy = self.cursor_pos.y - old_cursor.y;
            self.viewport.pan(dx, dy);
            self.pan_momentum.accumulate(dx, dy, Instant::now());
            return;
        }

        let serial = SERIAL_COUNTER.next_serial();
        let Some(pointer) = self.seat.get_pointer() else { return };

        // Convert screen position to canvas position for focus
        let (cx, cy) = self.viewport.screen_to_canvas(screen_pos.x, screen_pos.y);
        let canvas_pos: Point<f64, Logical> = Point::from((cx, cy));

        let under = self.surface_under(canvas_pos);

        pointer.motion(
            self,
            under,
            &MotionEvent {
                location: canvas_pos,
                serial,
                time: Event::time_msec(&event),
            },
        );
        pointer.frame(self);
    }

    fn handle_pointer_button<I: InputBackend>(&mut self, event: impl PointerButtonEvent<I>) {
        let serial = SERIAL_COUNTER.next_serial();
        let button = event.button_code();
        let button_state = event.state();

        let Some(keyboard) = self.seat.get_keyboard() else { return };
        let modifiers = keyboard.modifier_state();

        // Super+LMB = pan viewport
        if button == BTN_LEFT && modifiers.logo {
            if button_state == ButtonState::Pressed {
                self.panning = true;
                self.pan_momentum.stop();
            } else {
                self.panning = false;
                self.pan_momentum.launch();
            }
            return;
        }

        let Some(pointer) = self.seat.get_pointer() else { return };

        // Alt+LMB = move window, Alt+RMB = resize window
        if button_state == ButtonState::Pressed && modifiers.alt && !pointer.is_grabbed() {
            let (cx, cy) = self.viewport.screen_to_canvas(self.cursor_pos.x, self.cursor_pos.y);
            let canvas_pos: Point<f64, Logical> = Point::from((cx, cy));

            if let Some((window, _loc)) = self
                .space
                .element_under(canvas_pos)
                .map(|(w, l)| (w.clone(), l))
            {
                let Some(initial_window_location) = self.space.element_location(&window) else { return };

                let start_data = GrabStartData {
                    focus: self
                        .surface_under(canvas_pos)
                        .map(|(s, loc)| (s, loc.to_i32_round())),
                    button,
                    location: canvas_pos,
                };

                if button == BTN_LEFT {
                    let grab = MoveSurfaceGrab {
                        start_data,
                        window: window.clone(),
                        initial_window_location,
                    };
                    if let Some(toplevel) = window.toplevel() {
                        keyboard.set_focus(
                            self,
                            Some(toplevel.wl_surface().clone()),
                            serial,
                        );
                    }
                    pointer.set_grab(self, grab, serial, Focus::Clear);
                } else if button == BTN_RIGHT {
                    use crate::grabs::resize_grab::ResizeEdge;
                    use smithay::utils::Rectangle;

                    let initial_window_size = window.geometry().size;
                    let edges = ResizeEdge::BOTTOM_RIGHT;

                    let grab = ResizeSurfaceGrab::start(
                        start_data,
                        window.clone(),
                        edges,
                        Rectangle::new(initial_window_location, initial_window_size),
                    );
                    if let Some(toplevel) = window.toplevel() {
                        keyboard.set_focus(
                            self,
                            Some(toplevel.wl_surface().clone()),
                            serial,
                        );
                    }
                    pointer.set_grab(self, grab, serial, Focus::Clear);
                }
                return;
            }
        }

        if ButtonState::Pressed == button_state && !pointer.is_grabbed() {
            // Click focus
            let (cx, cy) = self.viewport.screen_to_canvas(self.cursor_pos.x, self.cursor_pos.y);
            let canvas_pos: Point<f64, Logical> = Point::from((cx, cy));

            if let Some((window, _loc)) = self
                .space
                .element_under(canvas_pos)
                .map(|(w, l)| (w.clone(), l))
            {
                self.space.raise_element(&window, true);
                if let Some(toplevel) = window.toplevel() {
                    keyboard.set_focus(
                        self,
                        Some(toplevel.wl_surface().clone()),
                        serial,
                    );
                }
                self.update_focus_history(&window);
                self.space.elements().for_each(|window| {
                    if let Some(toplevel) = window.toplevel() {
                        toplevel.send_pending_configure();
                    }
                });
            } else {
                self.space.elements().for_each(|window| {
                    window.set_activated(false);
                    if let Some(toplevel) = window.toplevel() {
                        toplevel.send_pending_configure();
                    }
                });
                keyboard.set_focus(self, Option::<WlSurface>::None, serial);
            }
        }

        pointer.button(
            self,
            &ButtonEvent {
                button,
                state: button_state,
                serial,
                time: Event::time_msec(&event),
            },
        );
        pointer.frame(self);
    }

    fn handle_pointer_axis<I: InputBackend>(&mut self, event: impl PointerAxisEvent<I>) {
        let source = event.source();

        let horizontal_amount = event
            .amount(Axis::Horizontal)
            .unwrap_or_else(|| event.amount_v120(Axis::Horizontal).unwrap_or(0.0) * 15.0 / 120.);
        let vertical_amount = event
            .amount(Axis::Vertical)
            .unwrap_or_else(|| event.amount_v120(Axis::Vertical).unwrap_or(0.0) * 15.0 / 120.);

        let Some(keyboard) = self.seat.get_keyboard() else { return };
        let modifiers = keyboard.modifier_state();

        if modifiers.logo {
            // Super + scroll = zoom at cursor
            let factor = if vertical_amount < 0.0 { 1.05 } else { 1.0 / 1.05 };
            self.viewport
                .zoom_at(self.cursor_pos.x, self.cursor_pos.y, factor);
            return;
        }

        // Scroll on empty canvas = pan viewport
        let (cx, cy) = self
            .viewport
            .screen_to_canvas(self.cursor_pos.x, self.cursor_pos.y);
        let canvas_pos: Point<f64, Logical> = Point::from((cx, cy));
        let over_window = self.space.element_under(canvas_pos).is_some();

        if !over_window {
            // Pan the viewport
            self.viewport.pan(-horizontal_amount, -vertical_amount);
            self.pan_momentum
                .accumulate(-horizontal_amount, -vertical_amount, Instant::now());

            // If finger lifted, launch momentum
            if source == AxisSource::Finger {
                let h_stopped = event.amount(Axis::Horizontal) == Some(0.0);
                let v_stopped = event.amount(Axis::Vertical) == Some(0.0);
                if h_stopped && v_stopped {
                    self.pan_momentum.launch();
                }
            }
            return;
        }

        // Normal scroll — forward to focused client
        let horizontal_amount_discrete = event.amount_v120(Axis::Horizontal);
        let vertical_amount_discrete = event.amount_v120(Axis::Vertical);

        let mut frame = AxisFrame::new(Event::time_msec(&event)).source(source);
        if horizontal_amount != 0.0 {
            frame = frame.value(Axis::Horizontal, horizontal_amount);
            if let Some(discrete) = horizontal_amount_discrete {
                frame = frame.v120(Axis::Horizontal, discrete as i32);
            }
        }
        if vertical_amount != 0.0 {
            frame = frame.value(Axis::Vertical, vertical_amount);
            if let Some(discrete) = vertical_amount_discrete {
                frame = frame.v120(Axis::Vertical, discrete as i32);
            }
        }

        if source == AxisSource::Finger {
            if event.amount(Axis::Horizontal) == Some(0.0) {
                frame = frame.stop(Axis::Horizontal);
            }
            if event.amount(Axis::Vertical) == Some(0.0) {
                frame = frame.stop(Axis::Vertical);
            }
        }

        let Some(pointer) = self.seat.get_pointer() else { return };
        pointer.axis(self, frame);
        pointer.frame(self);
    }
}
