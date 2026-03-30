use std::time::Duration;

use smithay::{
    backend::{
        renderer::{
            damage::OutputDamageTracker,
            element::{
                surface::WaylandSurfaceRenderElement,
                utils::RescaleRenderElement,
            },
            gles::GlesRenderer, ImportAll,
        },
        winit::{self, WinitEvent},
    },
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::calloop::EventLoop,
    utils::{Physical, Point, Rectangle, Scale, Transform},
};

use crate::CanvasWM;

// Combined render element enum with proper z-ordering:
// windows on top, dot grid behind.
smithay::backend::renderer::element::render_elements! {
    pub CanvasRenderElement<R> where
        R: ImportAll;
    Rescaled=RescaleRenderElement<WaylandSurfaceRenderElement<R>>,
    DotGrid=smithay::backend::renderer::element::solid::SolidColorRenderElement,
}

pub fn init_winit(
    event_loop: &mut EventLoop<CanvasWM>,
    state: &mut CanvasWM,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut backend, winit) = winit::init()?;

    let mode = Mode {
        size: backend.window_size(),
        refresh: 60_000,
    };

    let output = Output::new(
        "canvaswm".to_string(),
        PhysicalProperties {
            size: (0, 0).into(),
            subpixel: Subpixel::Unknown,
            make: "CanvasWM".into(),
            model: "Winit".into(),
        },
    );

    let _global = output.create_global::<CanvasWM>(&state.display_handle);
    output.change_current_state(
        Some(mode),
        Some(Transform::Flipped180),
        None,
        Some((0, 0).into()),
    );
    output.set_preferred(mode);

    state.space.map_output(&output, (0, 0));

    let mut damage_tracker = OutputDamageTracker::from_output(&output);
    let mut last_frame = std::time::Instant::now();

    event_loop
        .handle()
        .insert_source(winit, move |event, _, state| {
            match event {
                WinitEvent::Resized { size, .. } => {
                    output.change_current_state(
                        Some(Mode {
                            size,
                            refresh: 60_000,
                        }),
                        None,
                        None,
                        None,
                    );
                    state.viewport.resize(size.w as f64, size.h as f64);
                }
                WinitEvent::Input(event) => state.process_input_event(event),
                WinitEvent::Redraw => {
                    // Frame timing
                    let now = std::time::Instant::now();
                    let dt_secs = (now - last_frame).as_secs_f64();
                    last_frame = now;

                    // Advance viewport animations (camera lerp, zoom lerp)
                    state.viewport.tick_animations(Duration::from_secs_f64(dt_secs));

                    // Advance scroll momentum
                    if let Some((dx, dy)) = state.pan_momentum.tick(Duration::from_secs_f64(dt_secs)) {
                        state.viewport.pan(dx, dy);
                    }

                    // Edge auto-pan (during grabs)
                    state.apply_edge_pan();

                    // Write state file for external tools
                    state.write_state_file();

                    let size = backend.window_size();
                    let damage = Rectangle::from_size(size);
                    let viewport = &state.viewport;
                    let zoom = viewport.zoom;

                    // Generate dot grid as custom render elements
                    let dot_elements: Vec<CanvasRenderElement<GlesRenderer>> =
                        canvaswm_render::dot_grid::dot_grid_elements(
                            viewport,
                            (size.w, size.h),
                            state.config.background.dot_color,
                            state.config.background.grid_spacing,
                            state.config.background.dot_size,
                        )
                        .into_iter()
                        .map(CanvasRenderElement::from)
                        .collect();

                    {
                        let (renderer, mut framebuffer) = backend.bind().unwrap();

                        // Map the output to the camera position for correct
                        // element_for_region culling.
                        let cam_x = state.viewport.camera_x as i32;
                        let cam_y = state.viewport.camera_y as i32;
                        state.space.map_output(&output, (cam_x, cam_y));

                        // Get space (window) render elements in the visible region
                        let vis_w = (size.w as f64 / zoom).ceil() as i32 + 2;
                        let vis_h = (size.h as f64 / zoom).ceil() as i32 + 2;
                        let visible_region =
                            Rectangle::new((cam_x, cam_y).into(), (vis_w, vis_h).into());

                        let raw_space_elements: Vec<WaylandSurfaceRenderElement<GlesRenderer>> =
                            state
                                .space
                                .render_elements_for_region(renderer, &visible_region, 1.0, 1.0);

                        // Apply zoom via RescaleRenderElement
                        let zoom_scale: Scale<f64> = Scale::from((zoom, zoom));
                        let origin = Point::<i32, Physical>::from((0, 0));
                        let space_elements: Vec<CanvasRenderElement<GlesRenderer>> =
                            raw_space_elements
                                .into_iter()
                                .map(|e| {
                                    CanvasRenderElement::from(
                                        RescaleRenderElement::from_element(e, origin, zoom_scale),
                                    )
                                })
                                .collect();

                        // Compose: windows first (on top), then dot grid (behind)
                        let mut all_elements: Vec<CanvasRenderElement<GlesRenderer>> =
                            Vec::with_capacity(space_elements.len() + dot_elements.len());
                        all_elements.extend(space_elements);
                        all_elements.extend(dot_elements);

                        // Background color from config
                        let bg = state.config.background.color;

                        damage_tracker
                            .render_output(renderer, &mut framebuffer, 0, &all_elements, bg)
                            .unwrap();
                    }
                    backend.submit(Some(&[damage])).unwrap();

                    state.space.elements().for_each(|window| {
                        window.send_frame(
                            &output,
                            state.start_time.elapsed(),
                            Some(Duration::ZERO),
                            |_, _| Some(output.clone()),
                        )
                    });

                    state.space.refresh();
                    state.popups.cleanup();

                    let _ = state.display_handle.flush_clients();

                    backend.window().request_redraw();
                }
                WinitEvent::CloseRequested => {
                    state.loop_signal.stop();
                }
                _ => (),
            };
        })?;

    Ok(())
}
