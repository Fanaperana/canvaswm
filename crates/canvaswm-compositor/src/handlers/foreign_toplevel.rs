//! ext-foreign-toplevel-list-v1 handler.
//!
//! Exposes the list of mapped windows to external tools (task bars, etc.).

use crate::CanvasWM;
use smithay::{
    delegate_foreign_toplevel_list,
    wayland::foreign_toplevel_list::{ForeignToplevelListHandler, ForeignToplevelListState},
};

impl ForeignToplevelListHandler for CanvasWM {
    fn foreign_toplevel_list_state(&mut self) -> &mut ForeignToplevelListState {
        &mut self.foreign_toplevel_state
    }
}

delegate_foreign_toplevel_list!(CanvasWM);
