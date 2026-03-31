//! Winit backend — orchestration only.
//!
//! All rendering logic (backgrounds, decorations, minimap, corner clipping)
//! lives in `canvaswm-render`. This module wires up the winit event loop,
//! manages frame timing, and composes the final element list each frame.

use smithay::{
    backend::{
        renderer::{
            damage::OutputDamageTracker, element::surface::WaylandSurfaceRenderElement,
            element::utils::RescaleRenderElement, gles::GlesRenderer,
        },
        winit::{self, WinitEvent},
    },
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::calloop::EventLoop,
    utils::{Physical, Point, Rectangle, Scale, Transform},
};
use std::time::Duration;

use canvaswm_render::{
    decorations::{self, DecorationParams, DecorationShaders, WindowInfo},
    minimap, panel, Background, CanvasRenderElement,
};

use crate::CanvasWM;

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

/// Output refresh rate hint (milli-Hz). 60 000 mHz = 60 Hz.
const REFRESH_RATE: i32 = 60_000;

/// Extra pixels added to the visible-region query to avoid popping at edges.
const CULL_MARGIN: i32 = 2;

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

pub fn init_winit(
    event_loop: &mut EventLoop<CanvasWM>,
    state: &mut CanvasWM,
) -> Result<(), Box<dyn std::error::Error>> {
    let (mut backend, winit) = winit::init()?;

    let mode = Mode {
        size: backend.window_size(),
        refresh: REFRESH_RATE,
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

    // ----- one-time GPU resource init -----
    let background = {
        let (renderer, _) = backend.bind().unwrap();
        Background::init(
            renderer,
            &state.config.background.mode,
            state.config.background.shader_path.as_deref(),
            state.config.background.image_path.as_deref(),
        )
    };

    let deco_shaders: Option<DecorationShaders> = {
        let (renderer, _) = backend.bind().unwrap();
        match DecorationShaders::compile(renderer) {
            Ok(s) => {
                tracing::info!("Decoration shaders compiled");
                Some(s)
            }
            Err(e) => {
                tracing::error!("Decoration shader error: {e}");
                None
            }
        }
    };

    let mut damage_tracker = OutputDamageTracker::from_output(&output);
    let mut last_frame = std::time::Instant::now();

    // ----- event loop -----
    event_loop
        .handle()
        .insert_source(winit, move |event, _, state| {
            match event {
                WinitEvent::Resized { size, .. } => {
                    output.change_current_state(
                        Some(Mode {
                            size,
                            refresh: REFRESH_RATE,
                        }),
                        None,
                        None,
                        None,
                    );
                    state.viewport.resize(size.w as f64, size.h as f64);
                }
                WinitEvent::Input(event) => state.process_input_event(event),
                WinitEvent::Redraw => {
                    // --- frame timing ---
                    let now = std::time::Instant::now();
                    let dt = Duration::from_secs_f64((now - last_frame).as_secs_f64());
                    last_frame = now;

                    state.viewport.tick_animations(dt);
                    if let Some((dx, dy)) = state.pan_momentum.tick(dt) {
                        state.viewport.pan(dx, dy);
                    }
                    state.apply_edge_pan();
                    state.write_state_file();

                    // IPC
                    if let Some(listener) = state.ipc_listener.take() {
                        crate::ipc::poll_and_handle(&listener, state);
                        state.ipc_listener = Some(listener);
                    }

                    let size = backend.window_size();
                    let damage = Rectangle::from_size(size);
                    let zoom = state.viewport.zoom;

                    {
                        let (renderer, mut framebuffer) = backend.bind().unwrap();

                        // 1. Background
                        let bg_elements = background.render_elements(
                            renderer,
                            &state.viewport,
                            (size.w, size.h),
                            state.start_time.elapsed().as_secs_f32(),
                            state.config.background.dot_color,
                            state.config.background.grid_spacing,
                            state.config.background.dot_size,
                        );

                        // 2. Camera & visible-region query
                        let cam_x = state.viewport.camera_x as i32;
                        let cam_y = state.viewport.camera_y as i32;
                        state.space.map_output(&output, (cam_x, cam_y));

                        let vis_w = (size.w as f64 / zoom).ceil() as i32 + CULL_MARGIN;
                        let vis_h = (size.h as f64 / zoom).ceil() as i32 + CULL_MARGIN;
                        let visible_region =
                            Rectangle::new((cam_x, cam_y).into(), (vis_w, vis_h).into());

                        let raw: Vec<WaylandSurfaceRenderElement<GlesRenderer>> = state
                            .space
                            .render_elements_for_region(renderer, &visible_region, 1.0, 1.0);

                        let zoom_scale: Scale<f64> = Scale::from((zoom, zoom));
                        let origin = Point::<i32, Physical>::from((0, 0));
                        let space_elements: Vec<CanvasRenderElement> = raw
                            .into_iter()
                            .map(|e| {
                                CanvasRenderElement::from(RescaleRenderElement::from_element(
                                    e, origin, zoom_scale,
                                ))
                            })
                            .collect();

                        // 3. Window info for decorations
                        let windows = collect_window_infos(state, zoom);
                        let params = build_deco_params(state);

                        let deco_elements = match deco_shaders {
                            Some(ref s) => decorations::generate_decoration_elements(
                                s, &windows, &params, zoom,
                            ),
                            None => Vec::new(),
                        };

                        let corner_clip_elements = match deco_shaders {
                            Some(ref s) => decorations::generate_corner_clip_elements(
                                s, &windows, &params, zoom,
                            ),
                            None => Vec::new(),
                        };

                        // 4. Minimap
                        let mm_windows = collect_minimap_windows(state);
                        let minimap_elems: Vec<CanvasRenderElement> = minimap::minimap_elements(
                            &state.viewport,
                            (size.w, size.h),
                            &mm_windows,
                        )
                        .into_iter()
                        .map(CanvasRenderElement::from)
                        .collect();

                        let minimap_clip: Vec<CanvasRenderElement> = match deco_shaders {
                            Some(ref s) => minimap::minimap_clip_element(
                                s,
                                (size.w, size.h),
                                state.config.background.color,
                            )
                            .into_iter()
                            .collect(),
                            None => Vec::new(),
                        };

                        // 5. Panel
                        let panel_position = match state.config.panel.position.as_str() {
                            "bottom" => panel::PanelPosition::Bottom,
                            _ => panel::PanelPosition::Top,
                        };
                        let panel_elems: Vec<CanvasRenderElement> = if state.config.panel.enabled {
                            let pw = collect_panel_windows(state);
                            panel::panel_elements(panel_position, (size.w, size.h), &pw)
                                .into_iter()
                                .map(CanvasRenderElement::from)
                                .collect()
                        } else {
                            Vec::new()
                        };

                        let panel_clip: Vec<CanvasRenderElement> = if state.config.panel.enabled {
                            match deco_shaders {
                                Some(ref s) => panel::panel_clip_element(
                                    s,
                                    panel_position,
                                    (size.w, size.h),
                                    state.config.background.color,
                                )
                                .into_iter()
                                .collect(),
                                None => Vec::new(),
                            }
                        } else {
                            Vec::new()
                        };

                        // 6. Compose (front → back)
                        let total = panel_clip.len()
                            + panel_elems.len()
                            + minimap_clip.len()
                            + minimap_elems.len()
                            + corner_clip_elements.len()
                            + space_elements.len()
                            + deco_elements.len()
                            + bg_elements.len();

                        let mut all = Vec::with_capacity(total);
                        all.extend(panel_clip);
                        all.extend(panel_elems);
                        all.extend(minimap_clip);
                        all.extend(minimap_elems);
                        all.extend(corner_clip_elements);
                        all.extend(space_elements);
                        all.extend(deco_elements);
                        all.extend(bg_elements);

                        damage_tracker
                            .render_output(
                                renderer,
                                &mut framebuffer,
                                0,
                                &all,
                                state.config.background.color,
                            )
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
                    crate::ipc::cleanup();
                    state.loop_signal.stop();
                }
                _ => (),
            };
        })?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Helpers — extract compositor state into render-crate DTOs
// ---------------------------------------------------------------------------

/// Collect [`WindowInfo`] descriptors for every mapped window.
fn collect_window_infos(state: &CanvasWM, zoom: f64) -> Vec<WindowInfo> {
    let focused = if state.active_focus {
        state.focus_history.first()
    } else {
        None
    };

    state
        .space
        .elements()
        .filter_map(|window| {
            let loc = state.space.element_location(window)?;
            let geo = window.geometry();
            let bbox = window.bbox();

            let (screen_x, screen_y) = state.viewport.canvas_to_screen(loc.x as f64, loc.y as f64);
            let screen_w = (geo.size.w as f64 * zoom) as i32;
            let screen_h = (geo.size.h as f64 * zoom) as i32;

            // Full surface (CSD) bbox in canvas space
            let surf_x = loc.x as f64 - geo.loc.x as f64 + bbox.loc.x as f64;
            let surf_y = loc.y as f64 - geo.loc.y as f64 + bbox.loc.y as f64;
            let (bsx, bsy) = state.viewport.canvas_to_screen(surf_x, surf_y);

            Some(WindowInfo {
                screen_x,
                screen_y,
                screen_w,
                screen_h,
                bbox_screen_x: bsx,
                bbox_screen_y: bsy,
                bbox_screen_w: (bbox.size.w as f64 * zoom) as i32,
                bbox_screen_h: (bbox.size.h as f64 * zoom) as i32,
                focused: focused == Some(window),
            })
        })
        .collect()
}

/// Build [`DecorationParams`] from the current config snapshot.
fn build_deco_params(state: &CanvasWM) -> DecorationParams {
    let cfg = &state.config;
    let radius = if cfg.effects.corner_rounding {
        cfg.effects.corner_radius as f32
    } else {
        0.0
    };

    DecorationParams {
        shadow_enabled: cfg.effects.shadows,
        shadow_radius: cfg.effects.shadow_radius as f32,
        corner_radius: radius,
        border_width: cfg.decorations.border_width as f32,
        ssd_mode: cfg.decorations.mode == "server",
        title_height: cfg.decorations.title_bar_height,
        focused_color: cfg.decorations.focused_color,
        unfocused_color: cfg.decorations.unfocused_color,
        title_bar_color: cfg.decorations.title_bar_color,
        bg_color: cfg.background.color,
    }
}

/// Collect [`minimap::MinimapWindow`] entries for every mapped window.
fn collect_minimap_windows(state: &CanvasWM) -> Vec<minimap::MinimapWindow> {
    let focused = if state.active_focus {
        state.focus_history.first()
    } else {
        None
    };

    state
        .space
        .elements()
        .filter_map(|window| {
            let loc = state.space.element_location(window)?;
            let geo = window.geometry();
            Some(minimap::MinimapWindow {
                x: loc.x as f64,
                y: loc.y as f64,
                w: geo.size.w as f64,
                h: geo.size.h as f64,
                focused: focused == Some(window),
            })
        })
        .collect()
}

/// Collect [`panel::PanelWindow`] entries for every mapped window.
fn collect_panel_windows(state: &CanvasWM) -> Vec<panel::PanelWindow> {
    let focused = if state.active_focus {
        state.focus_history.first()
    } else {
        None
    };

    state
        .space
        .elements()
        .map(|window| panel::PanelWindow {
            focused: focused == Some(window),
        })
        .collect()
}
