use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

/// Top-level configuration for CanvasWM.
/// Supports loading from TOML, JSON, or YAML.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Input settings (keyboard, mouse, trackpad).
    pub input: InputConfig,
    /// Navigation and animation settings.
    pub navigation: NavigationConfig,
    /// Zoom settings.
    pub zoom: ZoomConfig,
    /// Scroll / viewport panning settings.
    pub scroll: ScrollConfig,
    /// Visual effects settings.
    pub effects: EffectsConfig,
    /// Canvas background settings.
    pub background: BackgroundConfig,
    /// Window decoration settings.
    pub decorations: DecorationConfig,
    /// Cursor settings.
    pub cursor: CursorConfig,
    /// Window snapping settings.
    pub snap: SnapConfig,
    /// Edge auto-pan settings.
    pub edge_pan: EdgePanConfig,
    /// Output configuration.
    pub output: OutputConfig,
    /// Custom keybindings (keysym string -> action string).
    pub keybindings: HashMap<String, String>,
    /// Programs to run at startup.
    pub autostart: Vec<String>,
    /// Environment variables to set for child processes.
    pub env: HashMap<String, String>,
    /// Window rules for per-app customization.
    pub window_rules: Vec<WindowRule>,
}

/// Keyboard layout and repeat settings.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct InputConfig {
    pub keyboard: KeyboardConfig,
    pub trackpad: TrackpadConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct KeyboardConfig {
    pub layout: String,
    pub variant: String,
    pub options: String,
    pub model: String,
    /// Keys per second repeat rate.
    pub repeat_rate: i32,
    /// Milliseconds before repeat starts.
    pub repeat_delay: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct TrackpadConfig {
    pub tap_to_click: bool,
    pub natural_scroll: bool,
    pub accel_speed: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct NavigationConfig {
    /// Camera lerp factor per frame at 60fps (higher = faster). Range 0.0–1.0.
    pub animation_speed: f64,
    /// Pixels per nudge-window action.
    pub nudge_step: f64,
    /// Pixels per keyboard pan-viewport action.
    pub pan_step: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ZoomConfig {
    /// Multiplier per zoom keypress (1.1 = 10% per press).
    pub step: f64,
    /// Canvas-space padding for zoom-to-fit.
    pub fit_padding: f64,
    /// Maximum zoom level.
    pub max_zoom: f64,
    /// Snap-to-1.0 dead zone (±).
    pub snap_threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ScrollConfig {
    /// Viewport pan speed multiplier.
    pub speed: f64,
    /// Momentum decay per frame (0.90 = snappy, 0.98 = floaty).
    pub friction: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EffectsConfig {
    /// Number of Kawase blur passes (0 = disabled).
    pub blur_radius: u32,
    /// Per-pass blur texel spread.
    pub blur_strength: f64,
    /// Enable window shadows.
    pub shadows: bool,
    /// Shadow radius in pixels.
    pub shadow_radius: f64,
    /// Enable corner rounding on windows.
    pub corner_rounding: bool,
    /// Corner radius in pixels.
    pub corner_radius: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct BackgroundConfig {
    /// "shader", "dots", "solid", or "image".
    pub mode: String,
    /// Path to custom GLSL fragment shader (for mode "shader").
    pub shader_path: Option<String>,
    /// Path to background image file (for mode "image"). Supports PNG, JPEG, WebP.
    pub image_path: Option<String>,
    /// Image display mode: "stretch", "fill", "center", "tile".
    pub image_mode: String,
    /// Solid background color [r, g, b, a] (0.0–1.0).
    pub color: [f32; 4],
    /// Dot grid spacing in canvas pixels.
    pub grid_spacing: f64,
    /// Dot size in canvas pixels.
    pub dot_size: f64,
    /// Dot color [r, g, b, a].
    pub dot_color: [f32; 4],
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DecorationConfig {
    /// "client" (CSD preferred), "server" (SSD), or "none".
    pub mode: String,
    /// Title bar height in pixels (SSD).
    pub title_bar_height: i32,
    /// Background color for SSD title bar [r, g, b, a].
    pub title_bar_color: [f32; 4],
    /// Focused window border color [r, g, b, a].
    pub focused_color: [f32; 4],
    /// Unfocused window border color [r, g, b, a].
    pub unfocused_color: [f32; 4],
    /// Border width for SSD.
    pub border_width: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CursorConfig {
    pub theme: Option<String>,
    pub size: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SnapConfig {
    pub enabled: bool,
    /// Gap between snapped windows in canvas pixels.
    pub gap: f64,
    /// Activation threshold in screen pixels.
    pub distance: f64,
    /// Screen pixels past snap to break free.
    pub break_force: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct EdgePanConfig {
    /// Activation zone width from viewport edge in pixels.
    pub zone: f64,
    /// Minimum pan speed (px/frame at zone boundary).
    pub speed_min: f64,
    /// Maximum pan speed (px/frame at viewport edge).
    pub speed_max: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct OutputConfig {
    /// Default output scale.
    pub scale: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowRule {
    /// Match by app_id (glob pattern).
    pub app_id: Option<String>,
    /// Match by title (glob pattern).
    pub title: Option<String>,
    /// Force decoration mode: "client", "server", "none".
    pub decoration: Option<String>,
    /// Pin window to screen (doesn't pan with canvas).
    pub pinned: Option<bool>,
    /// Widget mode (always below, excluded from navigation).
    pub widget: Option<bool>,
    /// Fixed position [x, y] in canvas coords.
    pub position: Option<[i32; 2]>,
    /// Fixed size [w, h].
    pub size: Option<[i32; 2]>,
    /// Window opacity (0.0–1.0).
    pub opacity: Option<f64>,
    /// Enable blur behind this window.
    pub blur: Option<bool>,
}

// ── Defaults ──

impl Default for Config {
    fn default() -> Self {
        Self {
            input: InputConfig::default(),
            navigation: NavigationConfig::default(),
            zoom: ZoomConfig::default(),
            scroll: ScrollConfig::default(),
            effects: EffectsConfig::default(),
            background: BackgroundConfig::default(),
            decorations: DecorationConfig::default(),
            cursor: CursorConfig::default(),
            snap: SnapConfig::default(),
            edge_pan: EdgePanConfig::default(),
            output: OutputConfig::default(),
            keybindings: HashMap::new(),
            autostart: Vec::new(),
            env: HashMap::new(),
            window_rules: Vec::new(),
        }
    }
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            keyboard: KeyboardConfig::default(),
            trackpad: TrackpadConfig::default(),
        }
    }
}

impl Default for KeyboardConfig {
    fn default() -> Self {
        Self {
            layout: "us".into(),
            variant: String::new(),
            options: String::new(),
            model: String::new(),
            repeat_rate: 25,
            repeat_delay: 200,
        }
    }
}

impl Default for TrackpadConfig {
    fn default() -> Self {
        Self {
            tap_to_click: true,
            natural_scroll: true,
            accel_speed: 0.0,
        }
    }
}

impl Default for NavigationConfig {
    fn default() -> Self {
        Self {
            animation_speed: 0.3,
            nudge_step: 20.0,
            pan_step: 100.0,
        }
    }
}

impl Default for ZoomConfig {
    fn default() -> Self {
        Self {
            step: 1.1,
            fit_padding: 100.0,
            max_zoom: 1.0,
            snap_threshold: 0.05,
        }
    }
}

impl Default for ScrollConfig {
    fn default() -> Self {
        Self {
            speed: 1.5,
            friction: 0.94,
        }
    }
}

impl Default for EffectsConfig {
    fn default() -> Self {
        Self {
            blur_radius: 0,
            blur_strength: 1.1,
            shadows: true,
            shadow_radius: 14.0,
            corner_rounding: true,
            corner_radius: 12.0,
        }
    }
}

impl Default for BackgroundConfig {
    fn default() -> Self {
        Self {
            mode: "dots".into(),
            shader_path: None,
            image_path: None,
            image_mode: "fill".into(),
            color: [0.118, 0.118, 0.180, 1.0], // Catppuccin Mocha base
            grid_spacing: 60.0,
            dot_size: 2.0,
            dot_color: [0.3, 0.3, 0.4, 0.4],
        }
    }
}

impl Default for DecorationConfig {
    fn default() -> Self {
        Self {
            mode: "client".into(),
            title_bar_height: 25,
            title_bar_color: [0.18, 0.18, 0.25, 1.0],
            focused_color: [0.55, 0.55, 0.85, 1.0],
            unfocused_color: [0.3, 0.3, 0.4, 0.6],
            border_width: 2,
        }
    }
}

impl Default for CursorConfig {
    fn default() -> Self {
        Self {
            theme: None,
            size: 24,
        }
    }
}

impl Default for SnapConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            gap: 12.0,
            distance: 24.0,
            break_force: 32.0,
        }
    }
}

impl Default for EdgePanConfig {
    fn default() -> Self {
        Self {
            zone: 100.0,
            speed_min: 4.0,
            speed_max: 20.0,
        }
    }
}

impl Default for OutputConfig {
    fn default() -> Self {
        Self { scale: 1.0 }
    }
}

impl Default for WindowRule {
    fn default() -> Self {
        Self {
            app_id: None,
            title: None,
            decoration: None,
            pinned: None,
            widget: None,
            position: None,
            size: None,
            opacity: None,
            blur: None,
        }
    }
}

// ── Loading ──

/// Standard config file search paths.
fn config_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    // XDG_CONFIG_HOME/canvaswm/
    if let Some(config_dir) = dirs::config_dir() {
        let base = config_dir.join("canvaswm");
        paths.push(base.join("config.toml"));
        paths.push(base.join("config.json"));
        paths.push(base.join("config.yaml"));
        paths.push(base.join("config.yml"));
    }

    // Fallback: ~/.config/canvaswm/
    if let Some(home) = dirs::home_dir() {
        let base = home.join(".config").join("canvaswm");
        paths.push(base.join("config.toml"));
        paths.push(base.join("config.json"));
        paths.push(base.join("config.yaml"));
        paths.push(base.join("config.yml"));
    }

    paths
}

impl Config {
    /// Load config from the first found config file.
    /// Falls back to defaults if no config file exists.
    pub fn load() -> Self {
        for path in config_paths() {
            if path.exists() {
                match Self::load_from(&path) {
                    Ok(config) => {
                        tracing::info!("Loaded config from {}", path.display());
                        return config;
                    }
                    Err(e) => {
                        tracing::error!("Failed to parse {}: {e}", path.display());
                    }
                }
            }
        }
        tracing::info!("No config file found, using defaults");
        Self::default()
    }

    /// Load config from a specific file, detecting format from extension.
    pub fn load_from(path: &PathBuf) -> Result<Self, String> {
        let contents = std::fs::read_to_string(path)
            .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;

        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        match ext {
            "toml" => toml::from_str(&contents).map_err(|e| format!("TOML parse error: {e}")),
            "json" => serde_json::from_str(&contents).map_err(|e| format!("JSON parse error: {e}")),
            "yaml" | "yml" => {
                serde_yaml::from_str(&contents).map_err(|e| format!("YAML parse error: {e}"))
            }
            _ => Err(format!("Unknown config format: .{ext}")),
        }
    }

    /// Validate config without starting the compositor.
    /// If path is None, search default locations.
    pub fn validate(path: Option<&PathBuf>) -> Result<(), String> {
        if let Some(p) = path {
            let _config = Self::load_from(p)?;
        } else {
            for p in config_paths() {
                if p.exists() {
                    let _config = Self::load_from(&p)?;
                    return Ok(());
                }
            }
            return Err("No config file found".into());
        }
        Ok(())
    }

    /// Hot-reload: reload from the same paths, merge with defaults.
    pub fn reload(&mut self) -> bool {
        for path in config_paths() {
            if path.exists() {
                match Self::load_from(&path) {
                    Ok(config) => {
                        tracing::info!("Config reloaded from {}", path.display());
                        *self = config;
                        return true;
                    }
                    Err(e) => {
                        tracing::error!("Config reload failed: {e}");
                    }
                }
            }
        }
        false
    }

    /// Config directory path (for writing state files, etc.)
    pub fn config_dir() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("canvaswm"))
    }

    /// Runtime directory (for state file, socket, etc.)
    pub fn runtime_dir() -> Option<PathBuf> {
        dirs::runtime_dir()
            .or_else(|| std::env::var("XDG_RUNTIME_DIR").ok().map(PathBuf::from))
            .map(|d| d.join("canvaswm"))
    }
}
