//! Composite render element for CanvasWM.
//!
//! Wraps all element types the compositor can produce into a single enum
//! so smithay's damage-tracked renderer can process them uniformly.
//! Lives in `canvaswm-render` because it is the render layer's concern.

use smithay::{
    backend::renderer::{
        element::{
            memory::MemoryRenderBufferRenderElement, solid::SolidColorRenderElement,
            surface::WaylandSurfaceRenderElement, utils::RescaleRenderElement, Element, Id, Kind,
            RenderElement, UnderlyingStorage,
        },
        gles::{element::PixelShaderElement, GlesError, GlesFrame, GlesRenderer},
        utils::{CommitCounter, DamageSet, OpaqueRegions},
    },
    utils::{Buffer, Physical, Rectangle, Scale, Transform},
};

/// Unified render element for all CanvasWM visual layers.
///
/// We implement `Element` and `RenderElement<GlesRenderer>` manually because
/// `PixelShaderElement` only supports `GlesRenderer`, preventing use of the
/// smithay `render_elements!` macro with a generic renderer.
pub enum CanvasRenderElement {
    /// Window surface scaled by the viewport zoom factor.
    Rescaled(RescaleRenderElement<WaylandSurfaceRenderElement<GlesRenderer>>),
    /// Unscaled surface (e.g. layer-shell surfaces fixed to screen edges).
    Surface(WaylandSurfaceRenderElement<GlesRenderer>),
    /// Small solid-colour quads (dot grid, minimap rectangles).
    SolidColor(SolidColorRenderElement),
    /// GLSL pixel shader quads (background, decorations, corner clips).
    Shader(PixelShaderElement),
    /// CPU-side image buffer (wallpaper).
    MemoryBuf(MemoryRenderBufferRenderElement<GlesRenderer>),
}

// ---------------------------------------------------------------------------
// Element trait — delegates to inner variant
// ---------------------------------------------------------------------------

impl Element for CanvasRenderElement {
    fn id(&self) -> &Id {
        match self {
            Self::Rescaled(e) => e.id(),
            Self::Surface(e) => e.id(),
            Self::SolidColor(e) => e.id(),
            Self::Shader(e) => e.id(),
            Self::MemoryBuf(e) => e.id(),
        }
    }

    fn current_commit(&self) -> CommitCounter {
        match self {
            Self::Rescaled(e) => e.current_commit(),
            Self::Surface(e) => e.current_commit(),
            Self::SolidColor(e) => e.current_commit(),
            Self::Shader(e) => e.current_commit(),
            Self::MemoryBuf(e) => e.current_commit(),
        }
    }

    fn src(&self) -> Rectangle<f64, Buffer> {
        match self {
            Self::Rescaled(e) => e.src(),
            Self::Surface(e) => e.src(),
            Self::SolidColor(e) => e.src(),
            Self::Shader(e) => e.src(),
            Self::MemoryBuf(e) => e.src(),
        }
    }

    fn geometry(&self, scale: Scale<f64>) -> Rectangle<i32, Physical> {
        match self {
            Self::Rescaled(e) => e.geometry(scale),
            Self::Surface(e) => e.geometry(scale),
            Self::SolidColor(e) => e.geometry(scale),
            Self::Shader(e) => e.geometry(scale),
            Self::MemoryBuf(e) => e.geometry(scale),
        }
    }

    fn transform(&self) -> Transform {
        match self {
            Self::Rescaled(e) => e.transform(),
            Self::Surface(e) => e.transform(),
            Self::SolidColor(e) => e.transform(),
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
            Self::Surface(e) => e.damage_since(scale, commit),
            Self::SolidColor(e) => e.damage_since(scale, commit),
            Self::Shader(e) => e.damage_since(scale, commit),
            Self::MemoryBuf(e) => e.damage_since(scale, commit),
        }
    }

    fn opaque_regions(&self, scale: Scale<f64>) -> OpaqueRegions<i32, Physical> {
        match self {
            Self::Rescaled(e) => e.opaque_regions(scale),
            Self::Surface(e) => e.opaque_regions(scale),
            Self::SolidColor(e) => e.opaque_regions(scale),
            Self::Shader(e) => e.opaque_regions(scale),
            Self::MemoryBuf(e) => e.opaque_regions(scale),
        }
    }

    fn alpha(&self) -> f32 {
        match self {
            Self::Rescaled(e) => e.alpha(),
            Self::Surface(e) => e.alpha(),
            Self::SolidColor(e) => e.alpha(),
            Self::Shader(e) => e.alpha(),
            Self::MemoryBuf(e) => e.alpha(),
        }
    }

    fn kind(&self) -> Kind {
        match self {
            Self::Rescaled(e) => e.kind(),
            Self::Surface(e) => e.kind(),
            Self::SolidColor(e) => e.kind(),
            Self::Shader(e) => e.kind(),
            Self::MemoryBuf(e) => e.kind(),
        }
    }
}

// ---------------------------------------------------------------------------
// RenderElement<GlesRenderer> — delegates draw calls
// ---------------------------------------------------------------------------

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
            Self::Surface(e) => {
                RenderElement::<GlesRenderer>::draw(e, frame, src, dst, damage, opaque_regions)
            }
            Self::SolidColor(e) => {
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
            Self::Surface(e) => {
                RenderElement::<GlesRenderer>::underlying_storage(e, renderer)
            }
            Self::SolidColor(e) => {
                RenderElement::<GlesRenderer>::underlying_storage(e, renderer)
            }
            Self::Shader(e) => {
                RenderElement::<GlesRenderer>::underlying_storage(e, renderer)
            }
            Self::MemoryBuf(e) => {
                RenderElement::<GlesRenderer>::underlying_storage(e, renderer)
            }
        }
    }
}

// ---------------------------------------------------------------------------
// From conversions
// ---------------------------------------------------------------------------

impl From<RescaleRenderElement<WaylandSurfaceRenderElement<GlesRenderer>>> for CanvasRenderElement {
    fn from(e: RescaleRenderElement<WaylandSurfaceRenderElement<GlesRenderer>>) -> Self {
        Self::Rescaled(e)
    }
}

impl From<SolidColorRenderElement> for CanvasRenderElement {
    fn from(e: SolidColorRenderElement) -> Self {
        Self::SolidColor(e)
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

impl From<WaylandSurfaceRenderElement<GlesRenderer>> for CanvasRenderElement {
    fn from(e: WaylandSurfaceRenderElement<GlesRenderer>) -> Self {
        Self::Surface(e)
    }
}
