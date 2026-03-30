use std::time::Duration;

use smithay::{
    backend::{
        renderer::{
            damage::OutputDamageTracker,
            element::{
                solid::SolidColorRenderElement,
                surface::WaylandSurfaceRenderElement,
                utils::RescaleRenderElement,
                Element, Id, Kind, RenderElement, UnderlyingStorage,
            },
            gles::{
                element::PixelShaderElement, GlesError, GlesFrame, GlesPixelProgram, GlesRenderer,
            },
            utils::{CommitCounter, DamageSet, OpaqueRegions},
        },
        winit::{self, WinitEvent},
    },
    output::{Mode, Output, PhysicalProperties, Subpixel},
    reexports::calloop::EventLoop,
    utils::{Buffer, Physical, Point, Rectangle, Scale, Transform},
};

use crate::CanvasWM;

/// Render element enum specialized for GlesRenderer.
/// We can't use the macro because PixelShaderElement only implements RenderElement<GlesRenderer>.
pub enum CanvasRenderElement {
    Rescaled(RescaleRenderElement<WaylandSurfaceRenderElement<GlesRenderer>>),
    DotGrid(SolidColorRenderElement),
    Shader(PixelShaderElement),
}

impl Element for CanvasRenderElement {
    fn id(&self) -> &Id {
        match self {
            Self::Rescaled(e) => e.id(),
            Self::DotGrid(e) => e.id(),
            Self::Shader(e) => e.id(),
        }
    }

    fn current_commit(&self) -> CommitCounter {
        match self {
            Self::Rescaled(e) => e.current_commit(),
            Self::DotGrid(e) => e.current_commit(),
            Self::Shader(e) => e.current_commit(),
        }
    }

    fn src(&self) -> Rectangle<f64, Buffer> {
        match self {
            Self::Rescaled(e) => e.src(),
            Self::DotGrid(e) => e.src(),
            Self::Shader(e) => e.src(),
        }
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        match self {
            Self::Rescaled(e) => e.geometry(scale),
            Self::DotGrid(e) => e.geometry(scale),
            Self::Shader(e) => e.geometry(scale),
        }
    }

    fn transform(&self) -> Transform {
        match self {
            Self::Rescaled(e) => e.transform(),
            Self::DotGrid(e) => e.transform(),
            Self::Shader(e) => e.transform(),
        }
    }

    fn damage_since(
        &self,
        scale: Scale<f64>,
        commit: Option<CommitCounter>,
    ) -> DamageSet<i32, Physical> {
        match self {
            Self::Rescaled(e) => e.damage_since(scale, commit),
            Self::DotGrid(e) => e.damage_since(scale, commit),
            Self::Shader(e) => e.damage_since(scale, commit),
        }
    }

    fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        match self {
            Self::Rescaled(e) => e.opaque_regions(scale),
            Self::DotGrid(e) => e.opaque_regions(scale),
            Self::Shader(e) => e.opaque_regions(scale),
        }
    }

    fn alpha(&self) -> f32 {
        match self {
            Self::Rescaled(e) => e.alpha(),
            Self::DotGrid(e) => e.alpha(),
            Self::Shader(e) => e.alpha(),
        }
    }

    fn kind(&self) -> Kind {
        match self {
            Self::Rescaled(e) => e.kind(),
            Self::DotGrid(e) => e.kind(),
            Self::Shader(e) => e.kind(),
        }
    }
}

impl RenderElement<GlesRenderer> for CanvasRenderElement {
    fn draw(
        &self,
        frame: &mut GlesFrame<'_, '_>,
        src: Rectangle<f64, Buffer>,
        dst: Rectangle<i32, Physical>,
        damage: &[Rectangle<i32, Physical>],
        opaque_regions: &[Rectangle<i32, Physical>],
    ) -> Result<(), GlesError> {
        match self {
            Self::Rescaled(e) => e.draw(frame, src, dst, damage, opaque_regions),
            Self::DotGrid(e) => {
                RenderElement::<GlesRenderer>::draw(e, frame, src, dst, damage, opaque_regions)
            }
            Self::Shader(e) => {
                RenderElement::<GlesRenderer>::draw(e, frame, src, dst, damage, opaque_regions)
            }
        }
    }

    fn underlying_storage(&self, renderer: &mut GlesRenderer) -> Option<UnderlyingStorage<'_>> {
        match self {
            Self::Rescaled(e) => e.underlying_storage(renderer),
            Self::DotGrid(e) => {
                RenderElement::<GlesRenderer>::underlying_storage(e, renderer)
            }
            Self::Shader(e) => {
                RenderElement::<GlesRenderer>::underlying_storage(e, renderer)
            }
        }
    }
}

// Conversion impls for convenience
impl From<RescaleRenderElement<WaylandSurfaceRenderElement<GlesRenderer>>> for CanvasRenderElement {
    fn from(e: RescaleRenderElement<WaylandSurfaceRenderElement<GlesRenderer>>) -> Self {
        Self::Rescaled(e)
    }
}

impl From<SolidColorRenderElement> for CanvasRenderElement {
    fn from(e: SolidColorRenderElement) -> Self {
        Self::DotGrid(e)
    }
}

impl From<PixelShaderElement> for CanvasRenderElement {
    fn from(e: PixelShaderElement) -> Self {
        Self::Shader(e)
    }
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

    // Compile background shader (if bg mode is "shader")
    let bg_shader: Option<GlesPixelProgram> = if state.config.background.mode == "shader" {
        let (renderer, _) = backend.bind().unwrap();
        match canvaswm_render::shader_bg::compile_background_shader(
            renderer,
            state.config.background.shader_path.as_deref(),
        ) {
            Ok(prog) => {
                tracing::info!("Background shader compiled successfully");
                Some(prog)
            }
            Err(e) => {
                tracing::error!("Background shader error: {e}. Falling back to dots.");
                None
            }
        }
    } else {
        None
    };

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

                    // Background elements: either shader or dot grid
                    let bg_elements: Vec<CanvasRenderElement> = if let Some(ref shader) = bg_shader {
                        // Shader background — full screen quad with viewport uniforms
                        let elapsed = state.start_time.elapsed().as_secs_f32();
                        let uniforms = canvaswm_render::shader_bg::build_uniforms(
                            elapsed,
                            (viewport.camera_x as f32, viewport.camera_y as f32),
                            zoom as f32,
                            (size.w as f32, size.h as f32),
                        );
                        let area = smithay::utils::Rectangle::from_size(
                            smithay::utils::Size::<i32, smithay::utils::Logical>::from((size.w, size.h)),
                        );
                        let element = smithay::backend::renderer::gles::element::PixelShaderElement::new(
                            shader.clone(),
                            area,
                            None,
                            1.0,
                            uniforms,
                            smithay::backend::renderer::element::Kind::Unspecified,
                        );
                        vec![CanvasRenderElement::Shader(element)]
                    } else {
                        // Dot grid background
                        canvaswm_render::dot_grid::dot_grid_elements(
                            viewport,
                            (size.w, size.h),
                            state.config.background.dot_color,
                            state.config.background.grid_spacing,
                            state.config.background.dot_size,
                        )
                        .into_iter()
                        .map(CanvasRenderElement::from)
                        .collect()
                    };

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
                        let space_elements: Vec<CanvasRenderElement> =
                            raw_space_elements
                                .into_iter()
                                .map(|e| {
                                    CanvasRenderElement::from(
                                        RescaleRenderElement::from_element(e, origin, zoom_scale),
                                    )
                                })
                                .collect();

                        // Compose: windows first (on top), then background (behind)
                        let mut all_elements: Vec<CanvasRenderElement> =
                            Vec::with_capacity(space_elements.len() + bg_elements.len());
                        all_elements.extend(space_elements);
                        all_elements.extend(bg_elements);

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
