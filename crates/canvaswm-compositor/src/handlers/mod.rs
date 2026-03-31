pub mod compositor;
pub mod layer_shell;
pub mod xdg_decoration;
pub mod xdg_shell;
pub mod xwayland;

use crate::CanvasWM;

use smithay::input::{Seat, SeatHandler, SeatState};
use smithay::reexports::wayland_server::protocol::wl_surface::WlSurface;
use smithay::reexports::wayland_server::Resource;
use smithay::wayland::output::OutputHandler;
use smithay::wayland::selection::data_device::{
    set_data_device_focus, ClientDndGrabHandler, DataDeviceHandler, DataDeviceState,
    ServerDndGrabHandler,
};
use smithay::wayland::selection::primary_selection::{
    set_primary_focus, PrimarySelectionHandler, PrimarySelectionState,
};
use smithay::wayland::selection::SelectionHandler;
use smithay::{delegate_data_device, delegate_output, delegate_primary_selection, delegate_seat, delegate_viewporter};

impl SeatHandler for CanvasWM {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<CanvasWM> {
        &mut self.seat_state
    }

    fn cursor_image(
        &mut self,
        _seat: &Seat<Self>,
        _image: smithay::input::pointer::CursorImageStatus,
    ) {
    }

    fn focus_changed(&mut self, seat: &Seat<Self>, focused: Option<&WlSurface>) {
        let dh = &self.display_handle;
        let client = focused.and_then(|s| dh.get_client(s.id()).ok());
        set_data_device_focus(dh, seat, client.clone());
        set_primary_focus(dh, seat, client);
    }
}

delegate_seat!(CanvasWM);

impl SelectionHandler for CanvasWM {
    type SelectionUserData = ();
}

impl DataDeviceHandler for CanvasWM {
    fn data_device_state(&self) -> &DataDeviceState {
        &self.data_device_state
    }
}

impl ClientDndGrabHandler for CanvasWM {}
impl ServerDndGrabHandler for CanvasWM {}

delegate_data_device!(CanvasWM);

impl PrimarySelectionHandler for CanvasWM {
    fn primary_selection_state(&self) -> &PrimarySelectionState {
        &self.primary_selection_state
    }
}
delegate_primary_selection!(CanvasWM);

impl OutputHandler for CanvasWM {}
delegate_output!(CanvasWM);
delegate_viewporter!(CanvasWM);
