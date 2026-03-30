//! DRM/KMS backend for native Wayland compositor operation.
//!
//! This module initializes real hardware via:
//! - libseat for session management (VT switching, privileges)
//! - udev to discover GPUs
//! - DRM/KMS for display output
//! - GBM + EGL for GPU-accelerated rendering
//! - libinput for input devices

use std::collections::HashMap;
use std::time::Duration;

use smithay::{
    backend::{
        allocator::gbm::GbmDevice,
        drm::{DrmDevice, DrmDeviceFd, DrmNode, NodeType},
        egl::EGLDisplay,
        libinput::{LibinputInputBackend, LibinputSessionInterface},
        renderer::{
            damage::OutputDamageTracker,
            gles::GlesRenderer,
        },
        session::libseat::LibSeatSession,
        session::{Event as SessionEvent, Session},
        udev::{self, UdevBackend, UdevEvent},
    },
    output::Output,
    reexports::{
        calloop::{
            timer::{TimeoutAction, Timer},
            EventLoop, RegistrationToken,
        },
        input::Libinput,
    },
};

use crate::CanvasWM;

/// Per-GPU device state.
#[allow(dead_code)]
struct GpuDevice {
    drm: DrmDevice,
    gbm: GbmDevice<DrmDeviceFd>,
    egl_display: EGLDisplay,
    renderer: GlesRenderer,
    surfaces: HashMap<u32, OutputSurface>,
    registration_token: RegistrationToken,
}

/// Per-output surface state.
#[allow(dead_code)]
struct OutputSurface {
    output: Output,
    damage_tracker: OutputDamageTracker,
}

/// State for the DRM/KMS backend.
pub struct DrmBackendData {
    #[allow(dead_code)]
    pub session: LibSeatSession,
    #[allow(dead_code)]
    pub primary_gpu: DrmNode,
}

/// Initialize the DRM/KMS backend.
///
/// This sets up:
/// 1. A libseat session for privilege management
/// 2. A udev monitor for GPU hotplug events
/// 3. Libinput for input device handling
/// 4. DRM devices for each connected GPU
pub fn init_drm(
    event_loop: &mut EventLoop<CanvasWM>,
    _state: &mut CanvasWM,
) -> Result<DrmBackendData, Box<dyn std::error::Error>> {
    // Initialize session
    let (session, session_notifier) = LibSeatSession::new()?;
    tracing::info!("Session initialized: seat = {}", session.seat());

    // Register session notifier
    let handle = event_loop.handle();
    handle
        .insert_source(session_notifier, |event, _, _state| match event {
            SessionEvent::PauseSession => {
                tracing::info!("Session paused (VT switch away)");
            }
            SessionEvent::ActivateSession => {
                tracing::info!("Session activated (VT switch back)");
            }
        })
        .map_err(|e| format!("Failed to register session notifier: {e}"))?;

    // Discover primary GPU via udev
    let primary_gpu = udev::primary_gpu(session.seat())
        .ok()
        .flatten()
        .and_then(|path| DrmNode::from_path(&path).ok())
        .and_then(|node| node.node_with_type(NodeType::Render).and_then(Result::ok))
        .unwrap_or_else(|| {
            udev::all_gpus(session.seat())
                .ok()
                .and_then(|gpus| gpus.into_iter().next())
                .and_then(|path| DrmNode::from_path(&path).ok())
                .expect("No GPU found")
        });

    tracing::info!("Primary GPU: {:?}", primary_gpu);

    // Initialize libinput
    let mut libinput_context =
        Libinput::new_with_udev::<LibinputSessionInterface<LibSeatSession>>(
            session.clone().into(),
        );
    libinput_context
        .udev_assign_seat(&session.seat())
        .unwrap();

    let libinput_backend = LibinputInputBackend::new(libinput_context.clone());
    handle
        .insert_source(libinput_backend, |event, _, state| {
            state.process_input_event(event);
        })
        .map_err(|e| format!("Failed to register libinput: {e}"))?;

    // Initialize udev backend for GPU hotplug
    let udev_backend = UdevBackend::new(session.seat())?;

    // Log already-connected GPUs
    for (device_id, path) in udev_backend.device_list() {
        if let Ok(node) = DrmNode::from_dev_id(device_id) {
            tracing::info!("Found GPU: {:?} at {:?}", node, path);
        }
    }

    handle
        .insert_source(udev_backend, move |event, _, _state| match event {
            UdevEvent::Added { device_id: _, path } => {
                tracing::info!("GPU added: {:?}", path);
            }
            UdevEvent::Changed { device_id } => {
                tracing::info!("GPU changed: dev_id={device_id}");
            }
            UdevEvent::Removed { device_id } => {
                tracing::info!("GPU removed: dev_id={device_id}");
            }
        })
        .map_err(|e| format!("Failed to register udev: {e}"))?;

    // Set up 60fps render timer
    let timer = Timer::immediate();
    handle
        .insert_source(timer, |_, _, _state| {
            // TODO: iterate outputs and render via DRM compositors
            TimeoutAction::ToDuration(Duration::from_millis(16))
        })
        .map_err(|e| format!("Failed to register render timer: {e}"))?;

    Ok(DrmBackendData {
        session,
        primary_gpu,
    })
}
