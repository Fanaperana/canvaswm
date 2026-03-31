use std::{ffi::OsString, os::unix::net::UnixListener, sync::Arc, time::Instant};

use smithay::{
    desktop::{PopupManager, Space, Window, WindowSurfaceType},
    input::{Seat, SeatState},
    reexports::{
        calloop::{generic::Generic, EventLoop, Interest, LoopSignal, Mode, PostAction},
        wayland_server::{
            backend::{ClientData, ClientId, DisconnectReason},
            protocol::wl_surface::WlSurface,
            Display, DisplayHandle,
        },
    },
    utils::{Logical, Point},
    wayland::{
        compositor::{CompositorClientState, CompositorState},
        output::OutputManagerState,
        selection::data_device::DataDeviceState,
        shell::{
            wlr_layer::WlrLayerShellState,
            xdg::{decoration::XdgDecorationState, XdgShellState},
        },
        shm::ShmState,
        socket::ListeningSocketSource,
    },
};

use canvaswm_canvas::{MomentumState, Viewport};
use canvaswm_config::Config;

pub struct CanvasWM {
    pub start_time: Instant,

    pub socket_name: OsString,
    pub display_handle: DisplayHandle,

    /// The 2D space where windows are mapped (canvas coordinates).
    pub space: Space<Window>,
    pub loop_signal: LoopSignal,

    /// The infinite canvas viewport (camera + zoom).
    pub viewport: Viewport,

    /// Scroll/pan momentum for smooth coasting after flick gestures.
    pub pan_momentum: MomentumState,

    /// Whether Super is held and LMB is dragging (pan mode).
    pub panning: bool,

    /// Track cursor position in screen coordinates.
    pub cursor_pos: Point<f64, Logical>,

    /// Configuration loaded from TOML/JSON/YAML.
    pub config: Config,

    // -- Focus / window management --
    /// Focus history for Alt-Tab cycling (most recent first).
    pub focus_history: Vec<Window>,
    /// Current Alt-Tab cycle index (None = not cycling).
    pub cycle_state: Option<usize>,
    /// Whether any window currently has active focus.
    pub active_focus: bool,

    // -- Edge auto-pan --
    /// Current edge pan velocity (screen-space px/frame), None = not panning.
    pub edge_pan_velocity: Option<(f64, f64)>,

    // -- Fullscreen --
    /// Currently fullscreened window + saved state.
    pub fullscreen: Option<FullscreenState>,

    // -- State file --
    /// Last time state file was written.
    pub state_file_last_write: Instant,

    // -- IPC --
    /// Unix domain socket for external tool communication.
    pub ipc_listener: Option<UnixListener>,

    // -- XWayland --
    /// XWayland window manager instance.
    pub xwm: Option<smithay::xwayland::X11Wm>,

    // Smithay protocol state
    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub xdg_decoration_state: XdgDecorationState,
    pub layer_shell_state: WlrLayerShellState,
    pub shm_state: ShmState,
    pub output_manager_state: OutputManagerState,
    pub seat_state: SeatState<CanvasWM>,
    pub data_device_state: DataDeviceState,
    pub popups: PopupManager,

    pub seat: Seat<Self>,
}

/// Saved state for a fullscreen window — restored on exit.
pub struct FullscreenState {
    pub window: Window,
    pub saved_location: Point<i32, Logical>,
    pub saved_camera: (f64, f64),
    pub saved_zoom: f64,
    pub saved_size: (i32, i32),
}

impl CanvasWM {
    pub fn new(event_loop: &mut EventLoop<Self>, display: Display<Self>) -> Self {
        let start_time = Instant::now();
        let dh = display.handle();
        let config = Config::load();

        let compositor_state = CompositorState::new::<Self>(&dh);
        let xdg_shell_state = XdgShellState::new::<Self>(&dh);
        let xdg_decoration_state = XdgDecorationState::new::<Self>(&dh);
        let layer_shell_state = WlrLayerShellState::new::<Self>(&dh);
        let shm_state = ShmState::new::<Self>(&dh, vec![]);
        let popups = PopupManager::default();
        let output_manager_state = OutputManagerState::new_with_xdg_output::<Self>(&dh);
        let data_device_state = DataDeviceState::new::<Self>(&dh);

        let mut seat_state = SeatState::new();
        let mut seat: Seat<Self> = seat_state.new_wl_seat(&dh, "canvaswm");
        seat.add_keyboard(
            Default::default(),
            config.input.keyboard.repeat_delay,
            config.input.keyboard.repeat_rate,
        )
        .unwrap();
        seat.add_pointer();

        let space = Space::default();
        let socket_name = Self::init_wayland_listener(display, event_loop);
        let loop_signal = event_loop.get_signal();

        // Create IPC listener
        let ipc_listener = crate::ipc::create_listener();

        // Apply config to viewport
        let viewport = Viewport {
            snap_threshold: config.zoom.snap_threshold,
            max_zoom: config.zoom.max_zoom,
            animation_speed: config.navigation.animation_speed,
            ..Default::default()
        };

        Self {
            start_time,
            display_handle: dh,
            space,
            loop_signal,
            socket_name,
            viewport,
            pan_momentum: MomentumState::new(config.scroll.friction),
            panning: false,
            cursor_pos: Point::from((0.0, 0.0)),
            config,
            focus_history: Vec::new(),
            cycle_state: None,
            active_focus: false,
            edge_pan_velocity: None,
            fullscreen: None,
            state_file_last_write: start_time,
            ipc_listener,
            xwm: None,
            compositor_state,
            xdg_shell_state,
            xdg_decoration_state,
            layer_shell_state,
            shm_state,
            output_manager_state,
            seat_state,
            data_device_state,
            popups,
            seat,
        }
    }

    fn init_wayland_listener(
        display: Display<CanvasWM>,
        event_loop: &mut EventLoop<Self>,
    ) -> OsString {
        let listening_socket = ListeningSocketSource::new_auto().unwrap();
        let socket_name = listening_socket.socket_name().to_os_string();
        let loop_handle = event_loop.handle();

        loop_handle
            .insert_source(listening_socket, move |client_stream, _, state| {
                state
                    .display_handle
                    .insert_client(client_stream, Arc::new(ClientState::default()))
                    .unwrap();
            })
            .expect("Failed to init wayland listener");

        loop_handle
            .insert_source(
                Generic::new(display, Interest::READ, Mode::Level),
                |_, display, state| {
                    // Safety: we don't drop the display
                    unsafe {
                        display.get_mut().dispatch_clients(state).unwrap();
                    }
                    Ok(PostAction::Continue)
                },
            )
            .unwrap();

        socket_name
    }

    /// Find the surface under a position in **canvas** coordinates.
    pub fn surface_under(
        &self,
        pos: Point<f64, Logical>,
    ) -> Option<(WlSurface, Point<f64, Logical>)> {
        self.space
            .element_under(pos)
            .and_then(|(window, location)| {
                window
                    .surface_under(pos - location.to_f64(), WindowSurfaceType::ALL)
                    .map(|(s, p)| (s, (p + location).to_f64()))
            })
    }

    /// Bounding box of all windows in canvas space.
    pub fn all_windows_bbox(&self) -> Option<(f64, f64, f64, f64)> {
        canvaswm_canvas::all_windows_bbox(self.space.elements().filter_map(|w| {
            let loc = self.space.element_location(w)?;
            let size = w.geometry().size;
            Some((loc.x, loc.y, size.w, size.h))
        }))
    }

    /// Update focus history when a window gains focus.
    pub fn update_focus_history(&mut self, window: &Window) {
        self.focus_history.retain(|w| w != window);
        self.focus_history.insert(0, window.clone());
        self.active_focus = true;
    }

    /// Cycle windows forward in focus history.
    pub fn cycle_forward(&mut self) {
        if self.focus_history.is_empty() {
            return;
        }
        let idx = match self.cycle_state {
            Some(i) => (i + 1) % self.focus_history.len(),
            None => {
                if self.focus_history.len() > 1 {
                    1
                } else {
                    0
                }
            }
        };
        self.cycle_state = Some(idx);
        if let Some(window) = self.focus_history.get(idx).cloned() {
            self.active_focus = true;
            let serial = smithay::utils::SERIAL_COUNTER.next_serial();
            self.space.raise_element(&window, true);
            if let Some(surface) = window.toplevel().map(|t| t.wl_surface().clone()) {
                self.seat
                    .get_keyboard()
                    .unwrap()
                    .set_focus(self, Some(surface), serial);
            }
            // Animate to window center
            if let Some(loc) = self.space.element_location(&window) {
                let size = window.geometry().size;
                let cx = loc.x as f64 + size.w as f64 / 2.0;
                let cy = loc.y as f64 + size.h as f64 / 2.0;
                self.viewport.animate_to(cx, cy);
            }
        }
    }

    /// Cycle windows backward in focus history.
    pub fn cycle_backward(&mut self) {
        if self.focus_history.is_empty() {
            return;
        }
        let idx = match self.cycle_state {
            Some(0) | None => self.focus_history.len().saturating_sub(1),
            Some(i) => i - 1,
        };
        self.cycle_state = Some(idx);
        if let Some(window) = self.focus_history.get(idx).cloned() {
            self.active_focus = true;
            let serial = smithay::utils::SERIAL_COUNTER.next_serial();
            self.space.raise_element(&window, true);
            if let Some(surface) = window.toplevel().map(|t| t.wl_surface().clone()) {
                self.seat
                    .get_keyboard()
                    .unwrap()
                    .set_focus(self, Some(surface), serial);
            }
            if let Some(loc) = self.space.element_location(&window) {
                let size = window.geometry().size;
                let cx = loc.x as f64 + size.w as f64 / 2.0;
                let cy = loc.y as f64 + size.h as f64 / 2.0;
                self.viewport.animate_to(cx, cy);
            }
        }
    }

    /// End Alt-Tab cycling: commit selection to focus history.
    pub fn end_cycle(&mut self) {
        if let Some(idx) = self.cycle_state.take() {
            if let Some(window) = self.focus_history.get(idx).cloned() {
                self.focus_history.retain(|w| w != &window);
                self.focus_history.insert(0, window);
            }
        }
    }

    /// Navigate to the nearest window in a direction from the focused window.
    pub fn navigate_direction(&mut self, dir: (f64, f64)) {
        let focused = self.focus_history.first().cloned();
        let origin = focused.as_ref().and_then(|w| {
            let loc = self.space.element_location(w)?;
            let size = w.geometry().size;
            Some((
                loc.x as f64 + size.w as f64 / 2.0,
                loc.y as f64 + size.h as f64 / 2.0,
            ))
        });
        let origin = match origin {
            Some(o) => o,
            None => {
                // No focused window, use viewport center
                let (cam_x, cam_y, w, h) = self.viewport.visible_rect();
                (cam_x + w / 2.0, cam_y + h / 2.0)
            }
        };

        let items = self.space.elements().filter_map(|w| {
            let loc = self.space.element_location(w)?;
            let size = w.geometry().size;
            Some((
                w.clone(),
                (
                    loc.x as f64 + size.w as f64 / 2.0,
                    loc.y as f64 + size.h as f64 / 2.0,
                ),
            ))
        });

        if let Some(target) = canvaswm_canvas::find_nearest(origin, dir, items, focused.as_ref()) {
            let serial = smithay::utils::SERIAL_COUNTER.next_serial();
            self.space.raise_element(&target, true);
            if let Some(surface) = target.toplevel().map(|t| t.wl_surface().clone()) {
                self.seat
                    .get_keyboard()
                    .unwrap()
                    .set_focus(self, Some(surface), serial);
            }
            self.update_focus_history(&target);
            if let Some(loc) = self.space.element_location(&target) {
                let size = target.geometry().size;
                let cx = loc.x as f64 + size.w as f64 / 2.0;
                let cy = loc.y as f64 + size.h as f64 / 2.0;
                self.viewport.animate_to_window(cx, cy, 1.0);
            }
        }
    }

    /// Compute edge auto-pan velocity based on cursor position.
    pub fn compute_edge_pan(&mut self) {
        let zone = self.config.edge_pan.zone;
        let (sw, sh) = (self.viewport.width, self.viewport.height);
        let (mx, my) = (self.cursor_pos.x, self.cursor_pos.y);

        let mut vx = 0.0_f64;
        let mut vy = 0.0_f64;

        if mx < zone {
            let depth = 1.0 - mx / zone;
            vx = -self.lerp_speed(depth);
        } else if mx > sw - zone {
            let depth = 1.0 - (sw - mx) / zone;
            vx = self.lerp_speed(depth);
        }

        if my < zone {
            let depth = 1.0 - my / zone;
            vy = -self.lerp_speed(depth);
        } else if my > sh - zone {
            let depth = 1.0 - (sh - my) / zone;
            vy = self.lerp_speed(depth);
        }

        if vx != 0.0 || vy != 0.0 {
            self.edge_pan_velocity = Some((vx, vy));
        } else {
            self.edge_pan_velocity = None;
        }
    }

    fn lerp_speed(&self, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        let t2 = t * t; // quadratic ramp
        self.config.edge_pan.speed_min
            + t2 * (self.config.edge_pan.speed_max - self.config.edge_pan.speed_min)
    }

    /// Apply edge pan velocity to camera (call each frame during a grab).
    pub fn apply_edge_pan(&mut self) {
        if let Some((vx, vy)) = self.edge_pan_velocity {
            let zoom = self.viewport.zoom;
            self.viewport.camera_x += vx / zoom;
            self.viewport.camera_y += vy / zoom;
        }
    }

    /// Write viewport state to runtime directory for external tools (waybar, etc.)
    pub fn write_state_file(&mut self) {
        // Throttle writes to ~10/sec
        if self.state_file_last_write.elapsed() < std::time::Duration::from_millis(100) {
            return;
        }
        self.state_file_last_write = Instant::now();

        let Some(dir) = Config::runtime_dir() else {
            return;
        };
        let _ = std::fs::create_dir_all(&dir);

        let vp = &self.viewport;
        let cx = vp.camera_x + vp.width / (2.0 * vp.zoom);
        let cy = vp.camera_y + vp.height / (2.0 * vp.zoom);

        let content = format!(
            "x={cx:.0}\ny={cy:.0}\nzoom={:.3}\nwindows={}\n",
            vp.zoom,
            self.space.elements().count(),
        );

        let tmp = dir.join("state.tmp");
        let path = dir.join("state");
        if std::fs::write(&tmp, content).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
    }

    /// Reload configuration from disk.
    pub fn reload_config(&mut self) {
        if self.config.reload() {
            // Apply changed settings
            self.viewport.snap_threshold = self.config.zoom.snap_threshold;
            self.viewport.max_zoom = self.config.zoom.max_zoom;
            self.viewport.animation_speed = self.config.navigation.animation_speed;
            self.pan_momentum.friction = self.config.scroll.friction;
            tracing::info!("Config applied");
        }
    }
}

#[derive(Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}
