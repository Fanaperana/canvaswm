//! Unix domain socket IPC for external tool communication.
//!
//! Listens on `$XDG_RUNTIME_DIR/canvaswm.sock` and accepts simple
//! newline-delimited JSON commands. Responses are JSON objects.
//!
//! ## Commands
//!
//! - `{"cmd": "get_state"}` — viewport position, zoom, window count
//! - `{"cmd": "get_windows"}` — list all windows with positions/sizes
//! - `{"cmd": "focus_window", "id": N}` — focus window by index
//! - `{"cmd": "set_zoom", "zoom": 0.5}` — set zoom level
//! - `{"cmd": "pan_to", "x": 100, "y": 200}` — pan camera
//! - `{"cmd": "reload_config"}` — hot-reload config
//! - `{"cmd": "navigate", "direction": "left|right|up|down"}`

use serde::{Deserialize, Serialize};

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::PathBuf;

use smithay::wayland::compositor;
use smithay::wayland::shell::xdg::XdgToplevelSurfaceData;

use crate::CanvasWM;

/// IPC request from a client.
#[derive(Debug, Deserialize)]
pub struct IpcRequest {
    pub cmd: String,
    #[serde(default)]
    pub id: Option<usize>,
    #[serde(default)]
    pub zoom: Option<f64>,
    #[serde(default)]
    pub x: Option<f64>,
    #[serde(default)]
    pub y: Option<f64>,
    #[serde(default)]
    #[allow(dead_code)]
    pub command: Option<String>,
    #[serde(default)]
    pub direction: Option<String>,
}

/// IPC response to a client.
#[derive(Debug, Serialize)]
pub struct IpcResponse {
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl IpcResponse {
    pub fn success(data: impl Serialize) -> Self {
        Self {
            ok: true,
            error: None,
            data: Some(serde_json::to_value(data).unwrap_or_default()),
        }
    }
    pub fn error(msg: impl Into<String>) -> Self {
        Self {
            ok: false,
            error: Some(msg.into()),
            data: None,
        }
    }
    fn ok() -> Self {
        Self {
            ok: true,
            error: None,
            data: None,
        }
    }
}

/// Viewport state for IPC response.
#[derive(Debug, Serialize)]
pub struct ViewportState {
    pub camera_x: f64,
    pub camera_y: f64,
    pub zoom: f64,
    pub window_count: usize,
}

/// Window info for IPC response.
#[derive(Debug, Serialize)]
pub struct WindowInfo {
    pub index: usize,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub app_id: String,
    pub title: String,
    pub focused: bool,
}

/// Get the IPC socket path.
pub fn socket_path() -> Option<PathBuf> {
    std::env::var("XDG_RUNTIME_DIR")
        .ok()
        .map(|d| PathBuf::from(d).join("canvaswm.sock"))
}

/// Create and bind the IPC listener. Removes stale socket if present.
pub fn create_listener() -> Option<UnixListener> {
    let path = socket_path()?;
    // Remove stale socket
    let _ = std::fs::remove_file(&path);
    match UnixListener::bind(&path) {
        Ok(listener) => {
            listener.set_nonblocking(true).ok()?;
            tracing::info!("IPC socket: {}", path.display());
            Some(listener)
        }
        Err(e) => {
            tracing::error!("Failed to bind IPC socket: {e}");
            None
        }
    }
}

/// Poll for IPC requests and handle them against compositor state.
pub fn poll_and_handle(listener: &UnixListener, state: &mut CanvasWM) {
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream.set_nonblocking(false).ok();
                stream
                    .set_read_timeout(Some(std::time::Duration::from_millis(100)))
                    .ok();
                let reader = BufReader::new(&stream);
                for line in reader.lines() {
                    match line {
                        Ok(line) if !line.is_empty() => {
                            let resp = match serde_json::from_str::<IpcRequest>(&line) {
                                Ok(req) => handle_request(&req, state),
                                Err(e) => IpcResponse::error(format!("Parse error: {e}")),
                            };
                            let _ = write_response(&stream, &resp);
                        }
                        _ => break,
                    }
                }
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(e) => {
                tracing::warn!("IPC accept error: {e}");
                break;
            }
        }
    }
}

fn handle_request(req: &IpcRequest, state: &mut CanvasWM) -> IpcResponse {
    match req.cmd.as_str() {
        "get_state" => {
            let vp = &state.viewport;
            IpcResponse::success(ViewportState {
                camera_x: vp.camera_x,
                camera_y: vp.camera_y,
                zoom: vp.zoom,
                window_count: state.space.elements().count(),
            })
        }
        "get_windows" => {
            let focused = if state.active_focus {
                state.focus_history.first()
            } else {
                None
            };
            let windows: Vec<WindowInfo> = state
                .space
                .elements()
                .enumerate()
                .map(|(i, w)| {
                    let loc = state.space.element_location(w).unwrap_or_default();
                    let geo = w.geometry();
                    let (app_id, title) = w
                        .toplevel()
                        .map(|t| {
                            compositor::with_states(t.wl_surface(), |states| {
                                states
                                    .data_map
                                    .get::<XdgToplevelSurfaceData>()
                                    .map(|data| {
                                        let attrs = data.lock().unwrap();
                                        (
                                            attrs.app_id.clone().unwrap_or_default(),
                                            attrs.title.clone().unwrap_or_default(),
                                        )
                                    })
                                    .unwrap_or_default()
                            })
                        })
                        .unwrap_or_default();
                    WindowInfo {
                        index: i,
                        x: loc.x,
                        y: loc.y,
                        width: geo.size.w,
                        height: geo.size.h,
                        app_id,
                        title,
                        focused: focused == Some(w),
                    }
                })
                .collect();
            IpcResponse::success(windows)
        }
        "focus_window" => {
            let Some(idx) = req.id else {
                return IpcResponse::error("missing 'id' field");
            };
            let window = state.space.elements().nth(idx).cloned();
            match window {
                Some(w) => {
                    state.update_focus_history(&w);
                    state.space.raise_element(&w, true);
                    if let Some(loc) = state.space.element_location(&w) {
                        let size = w.geometry().size;
                        let cx = loc.x as f64 + size.w as f64 / 2.0;
                        let cy = loc.y as f64 + size.h as f64 / 2.0;
                        state.viewport.animate_to(cx, cy);
                    }
                    IpcResponse::ok()
                }
                None => IpcResponse::error(format!("No window at index {idx}")),
            }
        }
        "set_zoom" => {
            let Some(z) = req.zoom else {
                return IpcResponse::error("missing 'zoom' field");
            };
            state.viewport.zoom = z.clamp(0.1, state.viewport.max_zoom);
            IpcResponse::ok()
        }
        "pan_to" => {
            let (x, y) = (req.x.unwrap_or(0.0), req.y.unwrap_or(0.0));
            state.viewport.animate_to(x, y);
            IpcResponse::ok()
        }
        "reload_config" => {
            state.config = canvaswm_config::Config::load();
            IpcResponse::ok()
        }
        "exec" => {
            // Disabled: arbitrary shell execution via IPC is a security risk.
            // Use config-defined keybindings or autostart for launching commands.
            IpcResponse::error("'exec' command is disabled for security reasons")
        }
        "navigate" => {
            let Some(ref dir) = req.direction else {
                return IpcResponse::error("missing 'direction' field");
            };
            let step = state.config.navigation.pan_step;
            match dir.as_str() {
                "left" => state.viewport.pan(-step, 0.0),
                "right" => state.viewport.pan(step, 0.0),
                "up" => state.viewport.pan(0.0, -step),
                "down" => state.viewport.pan(0.0, step),
                _ => return IpcResponse::error(format!("Unknown direction: {dir}")),
            }
            IpcResponse::ok()
        }
        other => IpcResponse::error(format!("Unknown command: {other}")),
    }
}

fn write_response(
    stream: &std::os::unix::net::UnixStream,
    resp: &IpcResponse,
) -> std::io::Result<()> {
    let mut stream = stream;
    let json = serde_json::to_string(resp)?;
    stream.write_all(json.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()
}

/// Clean up the socket file on shutdown.
pub fn cleanup() {
    if let Some(path) = socket_path() {
        let _ = std::fs::remove_file(path);
    }
}
