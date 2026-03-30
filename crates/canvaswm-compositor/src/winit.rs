use smithay::{
    backend::{
        renderer::{
            damage::OutputDamageTracker,
            element::{
                memory::MemoryRenderBufferRenderElement, solid::SolidColorRenderElement,
                surface::WaylandSurfaceRenderElement, utils::RescaleRenderElement, Element, Id,
                Kind, RenderElement, UnderlyingStorage,
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
    utils::{Buffer, Logical, Physical, Point, Rectangle, Scale, Size, Transform},
};
use std::time::Duration;

use canvaswm_render::decorations::DecorationShaders;

use crate::CanvasWM;

/// Render element enum specialized for GlesRenderer.
/// We can't use the macro because PixelShaderElement only implements RenderElement<GlesRenderer>.
pub enum CanvasRenderElement {
    Rescaled(RescaleRenderElement<WaylandSurfaceRenderElement<GlesRenderer>>),
    DotGrid(SolidColorRenderElement),
    Shader(PixelShaderElement),
    MemoryBuf(MemoryRenderBufferRenderElement<GlesRenderer>),
}

impl Element for CanvasRenderElement {
    fn id(&self) -> &Id {
        match self {
            Self::Rescaled(e) => e.id(),
            Self::DotGrid(e) => e.id(),
            Self::Shader(e) => e.id(),
            Self::MemoryBuf(e) => e.id(),
        }
    }

    fn current_commit(&self) -> CommitCounter {
        match self {
            Self::Rescaled(e) => e.current_commit(),
            Self::DotGrid(e) => e.current_commit(),
            Self::Shader(e) => e.current_commit(),
            Self::MemoryBuf(e) => e.current_commit(),
        }
    }

    fn src(&self) -> Rectangle<f64, Buffer> {
        match self {
            Self::Rescaled(e) => e.src(),
            Self::DotGrid(e) => e.src(),
            Self::Shader(e) => e.src(),
            Self::MemoryBuf(e) => e.src(),
        }
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        match self {
            Self::Rescaled(e) => e.geometry(scale),
            Self::DotGrid(e) => e.geometry(scale),
            Self::Shader(e) => e.geometry(scale),
            Self::MemoryBuf(e) => e.geometry(scale),
        }
    }

    fn transform(&self) -> Transform {
        match self {
            Self::Rescaled(e) => e.transform(),
            Self::DotGrid(e) => e.transform(),
            Self::Shader(e) => e.transform(),
            Self::MemoryBuf(e) => e.transform(),
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
            Self::MemoryBuf(e) => e.damage_since(scale, commit),
        }
    }

    fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        match self {
            Self::Rescaled(e) => e.opaque_regions(scale),
            Self::DotGrid(e) => e.opaque_regions(scale),
            Self::Shader(e) => e.opaque_regions(scale),
            Self::MemoryBuf(e) => e.opaque_regions(scale),
        }
    }

    fn alpha(&self) -> f32 {
        match self {
            Self::Rescaled(e) => e.alpha(),
            Self::DotGrid(e) => e.alpha(),
            Self::Shader(e) => e.alpha(),
            Self::MemoryBuf(e) => e.alpha(),
        }
    }

    fn kind(&self) -> Kind {
        match self {
            Self::Rescaled(e) => e.kind(),
            Self::DotGrid(e) => e.kind(),
            Self::Shader(e) => e.kind(),
            Self::MemoryBuf(e) => e.kind(),
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
            Self::MemoryBuf(e) => {
                RenderElement::<GlesRenderer>::draw(e, frame, src, dst, damage, opaque_regions)
            }
        }
    }

    fn underlying_storage(&self, renderer: &mut GlesRenderer) -> Option<UnderlyingStorage<'_>> {
        match self {
            Self::Rescaled(e) => e.underlying_storage(renderer),
            Self::DotGrid(e) => RenderElement::<GlesRenderer>::underlying_storage(e, renderer),
            Self::Shader(e) => RenderElement::<GlesRenderer>::underlying_storage(e, renderer),
            Self::MemoryBuf(e) => RenderElement::<GlesRenderer>::underlying_storage(e, renderer),
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

impl From<MemoryRenderBufferRenderElement<GlesRenderer>> for CanvasRenderElement {
    fn from(e: MemoryRenderBufferRenderElement<GlesRenderer>) -> Self {
        Self::MemoryBuf(e)
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

    // Load background image (if bg mode is "image")
    let bg_image_buffer: Option<smithay::backend::renderer::element::memory::MemoryRenderBuffer> =
        if state.config.background.mode == "image" {
            match state.config.background.image_path.as_deref() {
                Some(path) => match canvaswm_render::image_bg::load_image(path) {
                    Ok(img) => {
                        use smithay::backend::renderer::element::memory::MemoryRenderBuffer;
                        let buf = MemoryRenderBuffer::from_slice(
                            &img.data,
                            smithay::backend::allocator::Fourcc::Abgr8888,
                            (img.width as i32, img.height as i32),
                            1,
                            Transform::Normal,
                            None,
                        );
                        tracing::info!("Background image loaded: {}x{}", img.width, img.height);
                        Some(buf)
                    }
                    Err(e) => {
                        tracing::error!("Background image error: {e}. Falling back to solid.");
                        None
                    }
                },
                None => {
                    tracing::warn!("Background mode is 'image' but no image_path set.");
                    None
                }
            }
        } else {
            None
        };

    // Compile decoration shaders (shadow, border, title bar)
    let deco_shaders: Option<DecorationShaders> = {
        let (renderer, _) = backend.bind().unwrap();
        match DecorationShaders::compile(renderer) {
            Ok(shaders) => {
                tracing::info!("Decoration shaders compiled");
                Some(shaders)
            }
            Err(e) => {
                tracing::error!("Decoration shader error: {e}");
                None
            }
        }
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
                    state
                        .viewport
                        .tick_animations(Duration::from_secs_f64(dt_secs));

                    // Advance scroll momentum
                    if let Some((dx, dy)) =
                        state.pan_momentum.tick(Duration::from_secs_f64(dt_secs))
                    {
                        state.viewport.pan(dx, dy);
                    }

                    // Edge auto-pan (during grabs)
                    state.apply_edge_pan();

                    // Write state file for external tools
                    state.write_state_file();

                    // Poll IPC socket for commands
                    if let Some(listener) = state.ipc_listener.take() {
                        crate::ipc::poll_and_handle(&listener, state);
                        state.ipc_listener = Some(listener);
                    }

                    let size = backend.window_size();
                    let damage = Rectangle::from_size(size);
                    let viewport = &state.viewport;
                    let zoom = viewport.zoom;

                    // Non-renderer background elements (dots — no GPU needed)
                    let mut dot_bg_elements: Option<Vec<CanvasRenderElement>> =
                        if bg_shader.is_none() && bg_image_buffer.is_none() {
                            Some(
                                canvaswm_render::dot_grid::dot_grid_elements(
                                    viewport,
                                    (size.w, size.h),
                                    state.config.background.dot_color,
                                    state.config.background.grid_spacing,
                                    state.config.background.dot_size,
                                )
                                .into_iter()
                                .map(CanvasRenderElement::from)
                                .collect(),
                            )
                        } else {
                            None
                        };

                    {
                        let (renderer, mut framebuffer) = backend.bind().unwrap();

                        // Build background elements (some need the renderer)
                        let bg_elements: Vec<CanvasRenderElement> = if let Some(ref shader) = bg_shader {
                            // Animated shader background
                            let elapsed = state.start_time.elapsed().as_secs_f32();
                            let uniforms = canvaswm_render::shader_bg::build_uniforms(
                                elapsed,
                                (viewport.camera_x as f32, viewport.camera_y as f32),
                                zoom as f32,
                                (size.w as f32, size.h as f32),
                            );
                            let area = Rectangle::from_size(Size::<i32, Logical>::from((size.w, size.h)));
                            let element = PixelShaderElement::new(
                                shader.clone(),
                                area,
                                None,
                                1.0,
                                uniforms,
                                Kind::Unspecified,
                            );
                            vec![CanvasRenderElement::Shader(element)]
                        } else if let Some(ref buf) = bg_image_buffer {
                            // Still image background — stretched to fill screen
                            use smithay::backend::renderer::element::memory::MemoryRenderBufferRenderElement;
                            match MemoryRenderBufferRenderElement::from_buffer(
                                renderer,
                                Point::from((0.0, 0.0)),
                                buf,
                                None,    // alpha
                                None,    // src crop
                                Some(Size::<i32, Logical>::from((size.w, size.h))), // dst size (stretch to screen)
                                Kind::Unspecified,
                            ) {
                                Ok(element) => vec![CanvasRenderElement::MemoryBuf(element)],
                                Err(e) => {
                                    tracing::error!("Failed to render bg image: {e:?}");
                                    Vec::new()
                                }
                            }
                        } else if let Some(dots) = dot_bg_elements.take() {
                            dots
                        } else {
                            Vec::new() // solid color fallback (bg color handles it)
                        };

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
                            state.space.render_elements_for_region(
                                renderer,
                                &visible_region,
                                1.0,
                                1.0,
                            );

                        // Apply zoom via RescaleRenderElement
                        let zoom_scale: Scale<f64> = Scale::from((zoom, zoom));
                        let origin = Point::<i32, Physical>::from((0, 0));
                        let space_elements: Vec<CanvasRenderElement> = raw_space_elements
                            .into_iter()
                            .map(|e| {
                                CanvasRenderElement::from(RescaleRenderElement::from_element(
                                    e, origin, zoom_scale,
                                ))
                            })
                            .collect();

                        // Generate decoration elements (shadows, borders, title bars)
                        let deco_elements =
                            generate_decoration_elements(state, &deco_shaders, zoom);

                        // Generate corner clip elements (drawn on top of windows)
                        let corner_clip_elements =
                            generate_corner_clip_elements(state, &deco_shaders, zoom);

                        // Generate minimap elements
                        let minimap_windows: Vec<canvaswm_render::minimap::MinimapWindow> =
                            state.space.elements().filter_map(|window| {
                                let loc = state.space.element_location(window)?;
                                let geo = window.geometry();
                                let focused = state.focus_history.first().map_or(false, |f| f == window);
                                Some(canvaswm_render::minimap::MinimapWindow {
                                    x: loc.x as f64,
                                    y: loc.y as f64,
                                    w: geo.size.w as f64,
                                    h: geo.size.h as f64,
                                    focused,
                                })
                            }).collect();
                        let minimap_elems: Vec<CanvasRenderElement> =
                            canvaswm_render::minimap::minimap_elements(
                                &state.viewport,
                                (size.w, size.h),
                                &minimap_windows,
                            )
                            .into_iter()
                            .map(CanvasRenderElement::from)
                            .collect();

                        // Corner clip for the minimap background
                        let minimap_clip: Vec<CanvasRenderElement> = if let Some(ref shaders) = deco_shaders {
                            let mm_w = 200i32;
                            let mm_h = 140i32;
                            let mm_margin = 16i32;
                            let mm_x = mm_margin;
                            let mm_y = size.h - mm_h - mm_margin;
                            let mm_radius = 6.0f32;
                            let area = Rectangle::new(
                                Point::<i32, Logical>::from((mm_x, mm_y)),
                                Size::<i32, Logical>::from((mm_w, mm_h)),
                            );
                            let uniforms = DecorationShaders::corner_clip_uniforms(
                                mm_radius,
                                (mm_w as f32, mm_h as f32),
                                state.config.background.color,
                            );
                            vec![CanvasRenderElement::Shader(PixelShaderElement::new(
                                shaders.corner_clip.clone(),
                                area,
                                None,
                                1.0,
                                uniforms,
                                Kind::Unspecified,
                            ))]
                        } else {
                            Vec::new()
                        };

                        // Compose: minimap clip on top, minimap, corner clips above windows, windows, decorations behind, bg at back
                        let mut all_elements: Vec<CanvasRenderElement> = Vec::with_capacity(
                            minimap_clip.len() + minimap_elems.len() + corner_clip_elements.len() + space_elements.len() + deco_elements.len() + bg_elements.len(),
                        );
                        all_elements.extend(minimap_clip);
                        all_elements.extend(minimap_elems);
                        all_elements.extend(corner_clip_elements);
                        all_elements.extend(space_elements);
                        all_elements.extend(deco_elements);
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
                    crate::ipc::cleanup();
                    state.loop_signal.stop();
                }
                _ => (),
            };
        })?;

    Ok(())
}

/// Generate decoration elements (shadows, borders, title bars) for all visible windows.
fn generate_decoration_elements(
    state: &CanvasWM,
    deco_shaders: &Option<DecorationShaders>,
    zoom: f64,
) -> Vec<CanvasRenderElement> {
    let deco_shaders = match deco_shaders {
        Some(s) => s,
        None => return Vec::new(),
    };

    let config = &state.config;
    let shadow_enabled = config.effects.shadows;
    let shadow_radius = config.effects.shadow_radius as f32;
    let corner_radius = if config.effects.corner_rounding {
        config.effects.corner_radius as f32
    } else {
        0.0
    };
    let border_width = config.decorations.border_width as f32;
    let ssd_mode = config.decorations.mode == "server";
    let title_height = config.decorations.title_bar_height;

    let focused_window = state.focus_history.first();
    let mut elements = Vec::new();

    for window in state.space.elements() {
        let Some(loc) = state.space.element_location(window) else {
            continue;
        };
        let geo = window.geometry();
        let win_w = geo.size.w as f32;
        let win_h = geo.size.h as f32;

        let is_focused = focused_window.map_or(false, |f| f == window);
        let border_color = if is_focused {
            config.decorations.focused_color
        } else {
            config.decorations.unfocused_color
        };

        // Convert canvas-space window position to screen-space for element placement
        let (screen_x, screen_y) = state.viewport.canvas_to_screen(loc.x as f64, loc.y as f64);
        let screen_w = (win_w as f64 * zoom) as i32;
        let screen_h = (win_h as f64 * zoom) as i32;
        let scaled_radius = corner_radius * zoom as f32;
        let scaled_border = border_width * zoom as f32;
        let scaled_shadow_radius = shadow_radius * zoom as f32;

        // Shadow element (rendered behind the window)
        if shadow_enabled && shadow_radius > 0.0 {
            let spread = scaled_shadow_radius;
            let shadow_x = screen_x as i32 - spread as i32;
            let shadow_y = screen_y as i32 - spread as i32;
            let shadow_w = screen_w + spread as i32 * 2;
            let shadow_h = screen_h + spread as i32 * 2;

            let area = Rectangle::new(
                Point::<i32, Logical>::from((shadow_x, shadow_y)),
                Size::<i32, Logical>::from((shadow_w, shadow_h)),
            );
            let uniforms = DecorationShaders::shadow_uniforms(
                [0.0, 0.0, 0.0, 0.6],
                scaled_radius,
                (screen_w as f32, screen_h as f32),
                spread,
            );
            elements.push(CanvasRenderElement::Shader(PixelShaderElement::new(
                deco_shaders.shadow.clone(),
                area,
                None,
                1.0,
                uniforms,
                Kind::Unspecified,
            )));
        }

        // Border element (around the window)
        if border_width > 0.0 {
            let bw = scaled_border.ceil() as i32;
            let area = Rectangle::new(
                Point::<i32, Logical>::from((screen_x as i32 - bw, screen_y as i32 - bw)),
                Size::<i32, Logical>::from((screen_w + bw * 2, screen_h + bw * 2)),
            );
            let uniforms = DecorationShaders::border_uniforms(
                border_color,
                scaled_radius + scaled_border,
                scaled_border,
                ((screen_w + bw * 2) as f32, (screen_h + bw * 2) as f32),
            );
            elements.push(CanvasRenderElement::Shader(PixelShaderElement::new(
                deco_shaders.border.clone(),
                area,
                None,
                1.0,
                uniforms,
                Kind::Unspecified,
            )));
        }

        // SSD title bar element (above the window)
        if ssd_mode {
            let scaled_title_h = (title_height as f64 * zoom) as i32;
            let area = Rectangle::new(
                Point::<i32, Logical>::from((screen_x as i32, screen_y as i32 - scaled_title_h)),
                Size::<i32, Logical>::from((screen_w, scaled_title_h)),
            );
            let uniforms = DecorationShaders::title_bar_uniforms(
                config.decorations.title_bar_color,
                scaled_radius / screen_w.max(1) as f32, // normalized radius
            );
            elements.push(CanvasRenderElement::Shader(PixelShaderElement::new(
                deco_shaders.title_bar.clone(),
                area,
                None,
                1.0,
                uniforms,
                Kind::Unspecified,
            )));
        }
    }

    elements
}

/// Generate corner clip elements drawn on top of windows to round their corners.
/// Uses the background color to paint over the sharp corners outside a rounded rect.
fn generate_corner_clip_elements(
    state: &CanvasWM,
    deco_shaders: &Option<DecorationShaders>,
    zoom: f64,
) -> Vec<CanvasRenderElement> {
    let deco_shaders = match deco_shaders {
        Some(s) => s,
        None => return Vec::new(),
    };

    let config = &state.config;
    if !config.effects.corner_rounding || config.effects.corner_radius <= 0.0 {
        return Vec::new();
    }

    let corner_radius = config.effects.corner_radius as f32;
    let border_width = config.decorations.border_width as f32;
    let bg_color = config.background.color;
    let mut elements = Vec::new();

    for window in state.space.elements() {
        let Some(loc) = state.space.element_location(window) else {
            continue;
        };
        let geo = window.geometry();
        let bbox = window.bbox();

        // Surface origin in canvas space (full surface including CSD)
        let surf_x = loc.x as f64 - geo.loc.x as f64 + bbox.loc.x as f64;
        let surf_y = loc.y as f64 - geo.loc.y as f64 + bbox.loc.y as f64;
        let full_w = bbox.size.w as f64;
        let full_h = bbox.size.h as f64;

        // Extend the clip area outward to also cover the border element
        let scaled_border = (border_width * zoom as f32).ceil() as i32;
        let scaled_radius = corner_radius * zoom as f32;
        // Outer radius must match the border shader's outer radius
        let outer_radius = scaled_radius + scaled_border as f32;

        let (screen_x, screen_y) = state.viewport.canvas_to_screen(surf_x, surf_y);
        let screen_w = (full_w * zoom) as i32;
        let screen_h = (full_h * zoom) as i32;

        // Clip area extends beyond window surface by border width on all sides
        let clip_x = screen_x as i32 - scaled_border;
        let clip_y = screen_y as i32 - scaled_border;
        let clip_w = screen_w + scaled_border * 2;
        let clip_h = screen_h + scaled_border * 2;

        let area = Rectangle::new(
            Point::<i32, Logical>::from((clip_x, clip_y)),
            Size::<i32, Logical>::from((clip_w, clip_h)),
        );
        let uniforms = DecorationShaders::corner_clip_uniforms(
            outer_radius,
            (clip_w as f32, clip_h as f32),
            bg_color,
        );
        elements.push(CanvasRenderElement::Shader(PixelShaderElement::new(
            deco_shaders.corner_clip.clone(),
            area,
            None,
            1.0,
            uniforms,
            Kind::Unspecified,
        )));
    }

    elements
}
