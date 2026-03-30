//! Image background support for CanvasWM.
//!
//! Loads a still image (PNG, JPEG, WebP) and provides it as RGBA pixel data
//! suitable for uploading to a GPU texture via smithay's MemoryRenderBuffer.

use std::path::Path;

/// Loaded background image in RGBA format.
pub struct BgImage {
    pub data: Vec<u8>,
    pub width: u32,
    pub height: u32,
}

/// Load an image from a file path, converting to RGBA8.
/// Supports paths relative to `~/.config/canvaswm/` or absolute paths.
pub fn load_image(path: &str) -> Result<BgImage, String> {
    let p = Path::new(path);
    let resolved = if p.is_absolute() {
        p.to_path_buf()
    } else if let Some(config_dir) = dirs::config_dir() {
        config_dir.join("canvaswm").join(p)
    } else {
        p.to_path_buf()
    };

    let img = image::open(&resolved)
        .map_err(|e| format!("Failed to load image '{}': {e}", resolved.display()))?;

    let rgba = img.to_rgba8();
    let (width, height) = rgba.dimensions();

    // Convert RGBA to the format smithay expects (Abgr8888 = RGBA in memory on little-endian)
    Ok(BgImage {
        data: rgba.into_raw(),
        width,
        height,
    })
}
