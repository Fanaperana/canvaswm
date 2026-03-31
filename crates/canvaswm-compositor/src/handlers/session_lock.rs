//! ext-session-lock-v1 handler.
//!
//! Delegates lock/unlock and surface lifecycle to the compositor state.

use crate::CanvasWM;
use smithay::{
    delegate_session_lock,
    reexports::wayland_server::protocol::wl_output::WlOutput,
    wayland::session_lock::{LockSurface, SessionLockHandler, SessionLockManagerState, SessionLocker},
};

impl SessionLockHandler for CanvasWM {
    fn lock_state(&mut self) -> &mut SessionLockManagerState {
        &mut self.session_lock_state
    }

    fn lock(&mut self, confirmation: SessionLocker) {
        tracing::info!("Session lock requested");
        self.locked = true;
        // Confirm the lock; a full implementation would wait for lock surfaces.
        confirmation.lock();
    }

    fn unlock(&mut self) {
        tracing::info!("Session unlocked");
        self.locked = false;
    }

    fn new_surface(&mut self, _surface: LockSurface, _output: WlOutput) {}
}

delegate_session_lock!(CanvasWM);
