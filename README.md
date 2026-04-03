<div align="center">

# 🖼️ CanvasWM

**An infinite canvas Wayland compositor**

Arrange windows freely on a zoomable 2D surface — no grids, no tiling constraints.
Pan, zoom, and navigate your workspace like a design tool.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Rust](https://img.shields.io/badge/Rust-2021_Edition-orange.svg?logo=rust)](https://www.rust-lang.org/)
[![Wayland](https://img.shields.io/badge/Wayland-Compositor-yellow.svg?logo=wayland)](https://wayland.freedesktop.org/)
[![Smithay](https://img.shields.io/badge/Built_with-Smithay_0.7-purple.svg)](https://github.com/Smithay/smithay)
[![GitHub issues](https://img.shields.io/github/issues/Fanaperana/canvaswm)](https://github.com/Fanaperana/canvaswm/issues)
[![GitHub stars](https://img.shields.io/github/stars/Fanaperana/canvaswm)](https://github.com/Fanaperana/canvaswm/stargazers)
[![PRs Welcome](https://img.shields.io/badge/PRs-welcome-brightgreen.svg)](CONTRIBUTING.md)

</div>

---

## What is CanvasWM?

CanvasWM is a Wayland compositor that replaces the traditional desktop metaphor with an **infinite 2D canvas**. Instead of switching between virtual desktops or fighting with tiling layouts, you place windows anywhere on an unbounded surface and navigate with pan and zoom — just like Figma, Miro, or a maps application.

### Key Ideas

- **Infinite canvas** — windows live in a continuous 2D coordinate space with no edges
- **Zoom to overview** — zoom out to see all your windows at once, zoom in to focus
- **Momentum scrolling** — physics-based pan with natural deceleration
- **Spatial memory** — remember where things are by their position, not their workspace number

## Features

| Category | Features |
|---|---|
| **Canvas** | Infinite 2D space, smooth zoom (scroll + pinch), momentum scrolling, camera animations |
| **Windows** | Free placement, 8-direction resize, directional navigation, snap-to-grid, alt-tab cycling |
| **Rendering** | Custom GLSL shader backgrounds, dot-grid overlay, rounded corners, drop shadows, SSD borders |
| **Backgrounds** | Animated shaders, still images (PNG/JPEG/WebP), scrolling dot grid, solid colour |
| **Minimap** | Live overview panel showing all windows and the current viewport |
| **Config** | TOML/JSON/YAML, hot-reload, per-app window rules, custom keybindings |
| **IPC** | Unix socket interface for external tooling |
| **Backends** | Winit (development), DRM/KMS (bare metal, WIP) |
| **Protocols** | XDG Shell, XDG Decoration, SHM, Compositor, Output, Seat, Data Device, XWayland |

## Default Keybindings

| Shortcut | Action |
|---|---|
| <kbd>Super</kbd> + <kbd>Return</kbd> | Spawn terminal |
| <kbd>Super</kbd> + <kbd>D</kbd> | App launcher |
| <kbd>Super</kbd> + <kbd>Q</kbd> | Close window |
| <kbd>Super</kbd> + <kbd>=</kbd> / <kbd>-</kbd> | Zoom in / out |
| <kbd>Super</kbd> + <kbd>W</kbd> | Zoom to fit all windows |
| <kbd>Super</kbd> + <kbd>0</kbd> | Reset viewport |
| <kbd>Super</kbd> + <kbd>C</kbd> | Center focused window |
| <kbd>Super</kbd> + <kbd>F</kbd> | Toggle fullscreen |
| <kbd>Super</kbd> + <kbd>Home</kbd> | Toggle home position |
| <kbd>Super</kbd> + <kbd>Arrows</kbd> | Navigate to nearest window |
| <kbd>Super</kbd> + <kbd>Shift</kbd> + <kbd>Arrows</kbd> | Nudge window |
| <kbd>Alt</kbd> + <kbd>Tab</kbd> | Cycle windows |
| <kbd>Super</kbd> + <kbd>LMB drag</kbd> | Pan viewport |
| <kbd>Super</kbd> + <kbd>Scroll</kbd> | Zoom at cursor |
| <kbd>Alt</kbd> + <kbd>LMB drag</kbd> | Move window |
| <kbd>Alt</kbd> + <kbd>RMB drag</kbd> | Resize window |
| <kbd>Super</kbd> + <kbd>R</kbd> | Reload config |
| <kbd>Super</kbd> + <kbd>Escape</kbd> | Quit |

## Getting Started

### Prerequisites

- **Rust** 1.75+ (2021 edition)
- **Wayland** development libraries
- **Linux** with a Wayland-capable graphics driver

#### Debian / Ubuntu

```bash
sudo apt install libwayland-dev libxkbcommon-dev libudev-dev libinput-dev \
  libgbm-dev libdrm-dev libseat-dev libsystemd-dev
```

#### Fedora

```bash
sudo dnf install wayland-devel libxkbcommon-devel systemd-devel libinput-devel \
  mesa-libgbm-devel libdrm-devel libseat-devel
```

#### Arch Linux

```bash
sudo pacman -S wayland libxkbcommon libinput libseat mesa
```

### Build & Run

```bash
git clone https://github.com/Fanaperana/canvaswm.git
cd canvaswm
cargo build --release
```

#### Development mode (runs inside a Winit window)

```bash
cargo run
```

#### Native mode (from a TTY, replaces your display server)

```bash
./target/release/canvaswm --backend=drm
```

#### Validate your config without starting

```bash
./target/release/canvaswm --check-config
```

## Configuration

CanvasWM loads configuration from `~/.config/canvaswm/` in TOML, JSON, or YAML format.

<details>
<summary><strong>Example <code>config.toml</code></strong></summary>

```toml
[background]
mode = "dots"           # "shader", "image", "dots", or "solid"
color = [0.08, 0.08, 0.12, 1.0]
grid_spacing = 60.0
dot_size = 2.0
dot_color = [0.3, 0.3, 0.4, 0.4]
# shader_path = "~/.config/canvaswm/bg.glsl"
# image_path = "~/.config/canvaswm/wallpaper.png"

[zoom]
step = 1.1
fit_padding = 100.0
max_zoom = 1.0

[scroll]
speed = 1.5
friction = 0.94

[effects]
shadows = true
shadow_radius = 24.0
corner_rounding = true
corner_radius = 12.0

[decorations]
mode = "server"         # "server", "client", or "none"
border_width = 2.0
focused_color = [0.4, 0.5, 0.9, 1.0]
unfocused_color = [0.3, 0.3, 0.3, 1.0]

[navigation]
animation_speed = 0.3
nudge_step = 20
pan_step = 100

[snap]
enabled = true
gap = 10
activation_distance = 20
break_force = 50

# Per-app rules
[[window_rules]]
app_id = "firefox"
pinned = true
opacity = 1.0

[[autostart]]
command = "waybar"
```

</details>

### Custom Shader Backgrounds

CanvasWM supports live GLSL fragment shaders as backgrounds with these uniforms:

| Uniform | Type | Description |
|---|---|---|
| `u_time` | `float` | Elapsed seconds since startup |
| `u_camera` | `vec2` | Camera position on the canvas |
| `u_zoom` | `float` | Current zoom level |
| `u_resolution` | `vec2` | Output resolution in pixels |

## Architecture

CanvasWM is structured as a Rust workspace with five crates:

```
canvaswm/
├── canvaswm-canvas       # Pure math — viewport transforms, momentum physics, snapping
├── canvaswm-config       # TOML/JSON/YAML config parsing with hot-reload
├── canvaswm-input        # Action and direction type definitions
├── canvaswm-render       # GLSL shaders, decorations, backgrounds, minimap, elements
└── canvaswm-compositor   # Main binary — Smithay event loop, input handling, IPC
```

The canvas and config crates have **zero** Wayland dependencies, making them independently testable. The render crate depends only on Smithay's renderer types. The compositor crate wires everything together.

## Roadmap

- [ ] Multi-pass Kawase blur for window backgrounds
- [ ] DRM/KMS bare-metal backend (event loop completion)
- [ ] Layer-shell protocol (status bars, launchers)
- [ ] XWayland window rendering
- [ ] Workspace presets (save/restore canvas layouts)
- [ ] Touchpad gesture recognition (three-finger pan, pinch zoom)
- [ ] Screencopy protocol for screenshots

## Contributing

Contributions are welcome! Please read our [Contributing Guide](CONTRIBUTING.md) and [Code of Conduct](CODE_OF_CONDUCT.md) before opening a PR.

## License

This project is licensed under the [MIT License](LICENSE).

## Acknowledgements

- [Smithay](https://github.com/Smithay/smithay) — the Wayland compositor library that makes this possible
- [wlroots](https://gitlab.freedesktop.org/wlroots/wlroots) — for pioneering modular compositor architecture
- The Wayland protocol community for the protocol specifications
