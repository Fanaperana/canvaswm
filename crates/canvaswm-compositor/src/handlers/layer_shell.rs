use crate::CanvasWM;
use smithay::{
    delegate_layer_shell,
    desktop::layer_map_for_output,
    output::Output,
    wayland::shell::wlr_layer::{
        Layer, LayerSurface, WlrLayerShellHandler, WlrLayerShellState,
    },
};

impl WlrLayerShellHandler for CanvasWM {
    fn shell_state(&mut self) -> &mut WlrLayerShellState {
        &mut self.layer_shell_state
    }

    fn new_layer_surface(
        &mut self,
        surface: LayerSurface,
        output: Option<smithay::reexports::wayland_server::protocol::wl_output::WlOutput>,
        layer: Layer,
        namespace: String,
    ) {
        // Resolve which output this layer surface targets
        let output = output
            .as_ref()
            .and_then(|o| Output::from_resource(o))
            .or_else(|| self.space.outputs().next().cloned());

        let Some(output) = output else {
            tracing::warn!("Layer surface rejected: no output");
            return;
        };

        tracing::info!(
            "New layer surface: namespace={namespace}, layer={layer:?}"
        );

        // Use the desktop layer map to properly map this surface
        let mut layer_map = layer_map_for_output(&output);
        let _ = layer_map.map_layer(&smithay::desktop::LayerSurface::new(
            surface,
            namespace,
        ));
    }

    fn layer_destroyed(&mut self, _surface: LayerSurface) {
        tracing::debug!("Layer surface destroyed");
    }
}

delegate_layer_shell!(CanvasWM);
