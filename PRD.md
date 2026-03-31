# CanvasWM Product Requirements Document

**Version:** 1.0  
**Date:** 2026-03-30  
**Status:** Active

---

## 1. Vision

CanvasWM is a trackpad-first infinite canvas Wayland compositor. Windows float on an unbounded 2D plane; the user pans, zooms, and navigates. No workspaces, no tiling — just canvas.

The goal of this PRD is to close every gap identified in the competitive analysis against driftwm and ship canvaswm's unique differentiators (minimap, IPC, animated backgrounds) as flagship features.

---

## 2. Current State Audit

### ✅ Working
- Infinite 2D canvas viewport (pan, zoom, momentum, animation)
- Window move, resize, snap, edge auto-pan
- Directional navigation (Super+Arrow), Alt-Tab, home toggle, zoom-to-fit
- SSD/CSD decoration system (shadows, borders, corner rounding — after PRE fix)
- Shader + image + dot-grid backgrounds
- Minimap overlay
- Built-in panel/taskbar
- IPC Unix socket
- Hot-reload config (TOML/JSON/YAML)
- Window rules (app_id/title glob match → position, size, opacity, pinned, decoration)
- XDG Shell, XDG Decoration, Layer Shell (state registered, not rendered)
- Winit backend (nested compositor, dev mode)
- XWayland handler wired (`xwm` field in state), surface mapping incomplete
- Kawase blur shaders compiled, multi-pass render NOT wired
- DRM backend: session + udev + GPU discovery complete, render loop NOT wired

### ❌ Not Working
- DRM/KMS backend render loop (can't run on real hardware)
- Trackpad gestures (libinput — no gesture event handling)
- Layer shell surface rendering
- XWayland window mapping into Space
- Blur + per-window opacity rendering
- Multi-monitor (single output only)
- Session lock (`ext-session-lock`)
- Screencasting (`wlr-screencopy`, `ext-image-capture-source`)
- Foreign toplevel management (taskbars/docks)
- Canvas bookmarks/anchors
- Widget/pinned-window mode
- Focus-follows-mouse
- Minimap click-to-jump
- Output management protocol (`zwlr-output-management`)
- `--check-config` incomplete (no `validate()` fn in config crate)

---

## 3. Requirements

Features are grouped in priority tiers. **P0 = ship-blocking**, **P1 = next release**, **P2 = differentiators**.

---

### P0 — Needs Real Hardware (Blockers)

#### REQ-01: Complete DRM/KMS Backend

**Why:** canvaswm cannot run outside a nested session. This blocks all real users.

**What:**
- Wire the render loop in `drm.rs`: per-output `OutputSurface` with `DrmCompositor`, damage tracking, frame submission
- Handle udev hotplug (`UdevEvent::Added` / `Removed`) to add/remove outputs
- Handle `SessionEvent::PauseSession` / `ActivateSession` for VT switch
- Wire libinput through the event loop (mouse, keyboard, gestures)
- Auto-detect backend: TTY → DRM, nested Wayland → winit (already done for winit)
- Output configuration from `[[outputs]]` config sections (scale, transform, mode, position)

**Acceptance criteria:**
- `cargo run -- --backend=drm` from a TTY opens the compositor on real hardware
- VT switching (Ctrl+Alt+F2) pauses and resumes cleanly
- Mouse and keyboard work

---

#### REQ-02: Trackpad Gesture Support

**Why:** This is the defining UX of a canvas WM. Without it, canvaswm is just a floating WM.

**What:**
- Handle libinput gesture events in the input module:
  - `GestureSwipeBegin/Update/End` (2/3/4 finger)
  - `GesturePinchBegin/Update/End` (2/3 finger)
  - `GestureHoldBegin/End` (3/4 finger)
- Default gesture bindings (configurable via `[gestures]` in config):

| Gesture | Action |
|---------|--------|
| 3-finger swipe | Pan viewport (continuous) |
| 2-finger pinch on canvas | Zoom at cursor (continuous) |
| 3-finger pinch anywhere | Zoom at cursor (continuous) |
| 4-finger swipe | Navigate to nearest window in direction |
| 4-finger pinch-in | Zoom-to-fit |
| 4-finger pinch-out | Home toggle |
| 3-finger doubletap-swipe on window | Move window |
| Alt + 3-finger swipe on window | Resize window |

- Add `[gestures]` config section with thresholds (`swipe_threshold`, `pinch_in_threshold`, `pinch_out_threshold`)
- All gesture bindings fully configurable; `[gestures.on-window]`, `[gestures.on-canvas]`, `[gestures.anywhere]` context sections
- Unbound gestures forwarded to focused app via `wp_pointer_gestures`

**Acceptance criteria:**
- 3-finger swipe pans the canvas smoothly with momentum
- 2-finger pinch zooms at cursor
- 4-finger swipe jumps to nearest window in direction
- Gesture config section in `config.toml` overrides defaults

---

#### REQ-03: Layer Shell Surface Rendering

**Why:** Without layer shell, waybar, fuzzel, mako, and swaylock won't work. This is expected by every Wayland user.

**What:**
- `WlrLayerShellState` is already registered. Wire layer surfaces into the render pipeline in `winit.rs` and `drm.rs`
- Layer surfaces render at correct z-order: background < bottom < windows < top < overlay
- Layer surfaces with exclusive zone adjust window placement area
- Layer surfaces with keyboard interactivity receive focus correctly

**Acceptance criteria:**
- `waybar` renders correctly at the top/bottom of screen
- `fuzzel`/`wofi` overlay works
- `mako` notifications appear

---

### P0 — Core Feature Completeness

#### REQ-04: XWayland Window Mapping

**Why:** Steam, JetBrains IDEs, Wine/Proton, and many legacy apps require XWayland.

**What:**
- In `xwayland.rs` `map_window_request`: create a `Window` from the `X11Surface` and map it in `state.space` at a sensible position (canvas center of viewport)
- In `resize_request`: initiate a `ResizeSurfaceGrab` for X11 windows
- In `move_request`: initiate a `MoveSurfaceGrab` for X11 windows
- Handle `configure_notify` to update window position in Space
- Handle override-redirect windows (tooltips, menus): render at their requested position without focus/raise
- Ensure `DISPLAY` env var is set to XWayland's display after init
- XWayland windows receive CSD/SSD decoration treatment identically to Wayland windows

**Acceptance criteria:**
- Steam launches and is movable/resizable
- X11 terminal (xterm) works
- Wine/Proton games launch

---

#### REQ-05: Blur + Per-Window Opacity

**Why:** Frosted-glass terminals are a flagship visual feature. The shaders are already written — just needs wiring.

**What:**
- Multi-pass Kawase blur in the render pipeline:
  1. Render scene to texture (without the blurred window)
  2. Downsample N times (N = `blur_radius` config)
  3. Upsample N times with `blur_strength` spread
  4. Composite blurred texture behind the window using window shape as mask
- Per-window opacity: multiply final window texture alpha by `opacity` from window rules
- Blur and opacity set via window rules (`blur = true`, `opacity = 0.85`)
- Config: `[effects]` `blur_radius` (passes) and `blur_strength` (spread)

**Acceptance criteria:**
- `[[window_rules]] app_id = "Alacritty" blur = true opacity = 0.85` produces a frosted-glass terminal
- Blur updates correctly when panning (background changes under blurred window)
- Performance: no visible frame drops on a mid-range GPU

---

### P1 — Daily Driver Features

#### REQ-06: Multi-Monitor Support

**Why:** Most desktops have 2+ monitors. canvaswm currently supports one output only.

**What:**
- Each output has independent `Viewport` state (camera, zoom, momentum, animation)
- Move `viewport`, `pan_momentum`, `panning`, `edge_pan_velocity` from global `CanvasWM` state into a per-output struct
- Input routing: all input events (mouse, keyboard, gestures) are routed to the active output (the output the cursor is currently on)
- Cursor crosses freely between monitor boundaries in screen space
- Dragging a window across monitor boundary adjusts canvas position to stay under cursor
- `Super+Alt+Arrow` sends focused window to adjacent output
- Output outline: render a thin rectangle on the canvas showing where other monitors' viewports are looking (configurable color/thickness/opacity via `[output.outline]` config)
- Output config in `[[outputs]]` sections: `name`, `scale`, `transform`, `position`, `mode`
- `zwlr-output-management` protocol for tools like `wlr-randr` / `wdisplays`

**Acceptance criteria:**
- Two monitors independently pan/zoom the canvas
- Window dragged past monitor edge appears on the other monitor
- `wlr-randr` can list and configure outputs

---

#### REQ-07: Session Lock

**Why:** Required for any daily driver compositor. `swaylock` is the standard tool.

**What:**
- Implement `ext-session-lock` v1 protocol
- When lock is requested: overlay each output with the lock surface, block all input to regular windows, render only lock surfaces
- Unlock: remove lock surfaces, restore focus
- Default keybinding: `Super+L` → `spawn swaylock`

**Acceptance criteria:**
- `swaylock` works: screen blanks, password restores session
- Killing swaylock from another TTY doesn't leave compositor in locked state

---

#### REQ-08: Screencasting & Screenshots

**Why:** OBS, Firefox screen share, Discord, `grim` — essential for normal use.

**What:**
- `wlr-screencopy-v1`: allows `grim` to take screenshots
- `ext-image-capture-source` + `ext-image-copy-capture` (or `xdg-desktop-portal-wlr`): PipeWire-based screencasting for OBS, browsers, Discord
- Default keybinding: `Print` → `spawn grim` (screenshot)
- Output screencopy captures what's currently rendered (including decorations, minimap disabled during capture)

**Acceptance criteria:**
- `grim -o eDP-1 screenshot.png` produces a correct screenshot
- OBS can capture the compositor output via the portal

---

#### REQ-09: Canvas Bookmarks / Anchors

**Why:** High-impact navigation feature; driftwm users love it. Low implementation effort.

**What:**
- Config: `anchors = [[x1, y1], [x2, y2], ...]` — named canvas positions (Y-up coordinate system, converted to compositor Y-down internally)
- Default: `anchors = [[0, 0]]` (origin only)
- Keybindings: `Super+1` through `Super+4` jump the camera to anchors[0..3]
- "Jump to anchor" animates the camera (same lerp as navigate-to-window)
- Add `GoToAnchor(usize)` action to `canvaswm-input/src/lib.rs`
- IPC: `get_anchors` and `go_to_anchor <n>` commands

**Acceptance criteria:**
- `Super+1` jumps to the first configured anchor with animation
- `Super+2` jumps to the second anchor if defined; no-op if not configured

---

#### REQ-10: Widget / Pinned-Window Mode

**Why:** Allows pinning clocks, system stats widgets on the canvas. Low effort via window rules.

**What:**
- Window rule field: `widget = true`
  - Window is immovable (grab attempts ignored)
  - Stacked below all normal windows (z-order)
  - Excluded from Alt-Tab cycling and directional navigation
  - Excluded from zoom-to-fit calculations
- Window rule field: `pinned_to_screen = true`
  - Window position is in screen/viewport coordinates (not canvas), so it stays fixed on screen as the canvas pans
  - Works for both layer-shell and xdg-toplevel
- Ensure window rules are applied at `map_window_request` (XWayland) and `new_toplevel` (Wayland)

**Acceptance criteria:**
- `[[window_rules]] app_id = "eww" widget = true` pins eww output below windows, immovable
- Widget window does not appear in Alt-Tab or `Super+Arrow` navigation

---

#### REQ-11: Foreign Toplevel Management

**Why:** Taskbars and docks need `zwlr-foreign-toplevel-management-v1` to list and switch to windows. Needed for `crystal-dock` and waybar taskbar modules.

**What:**
- Implement `zwlr-foreign-toplevel-management-v1`
- Emit `toplevel_manager.new_toplevel` for every mapped window
- Implement `activate` request: pan the active output's viewport to center on the activated window and focus it
- Implement `close` and `set_fullscreen`/`unset_fullscreen`
- Update title and app_id on change

**Acceptance criteria:**
- waybar's `wlr/taskbar` module shows open windows and clicking one jumps to it
- `crystal-dock` lists windows

---

### P2 — Differentiators (canvaswm-exclusive features)

#### REQ-12: Interactive Minimap

**Why:** The minimap is canvaswm's most unique feature over driftwm. Making it interactive elevates it significantly.

**What:**
- Click on a window in the minimap → viewport animates to center on that window and focus it
- Minimap uses pointer events: on click, convert minimap pixel coordinates back to canvas coordinates, find the nearest window, and call the existing `animate_to` + focus logic
- Hover on minimap window → show tooltip with `app_id` / window title
- Config options: `minimap.enabled`, `minimap.size` (px), `minimap.position` (`bottom-left`, `bottom-right`, `top-left`, `top-right`), `minimap.opacity`

**Acceptance criteria:**
- Clicking a window in the minimap pans the viewport to it smoothly
- Minimap position is configurable

---

#### REQ-13: Animated Backgrounds (Time-Uniform Shaders)

**Why:** The `u_time` uniform is already passed to background shaders — no one else does animated infinite canvas backgrounds. Ship it as a flagship feature.

**What:**
- Background re-renders every frame when shader uses `u_time` (detect via config flag `animate = true` in `[background]`)
- When `animate = false` (default): background cached, only re-rendered on pan/zoom (current behavior, zero idle GPU cost)
- Document the available uniforms in a `docs/shaders.md` file:
  - `u_time` — elapsed seconds (float)
  - `u_camera` — canvas camera position (vec2)
  - `u_zoom` — zoom level (float)
  - `u_resolution` — output size in pixels (vec2)
- Ship 2-3 example animated shaders alongside the default dot-grid

**Acceptance criteria:**
- A shader using `u_time` for animation renders at display refresh rate when `animate = true`
- Default dot-grid still caches correctly with `animate = false`

---

#### REQ-14: IPC Ecosystem Integration

**Why:** canvaswm has the richest IPC of any comparable compositor. Leverage it with tooling.

**What:**
- Extend IPC commands:
  - `list_anchors` — return configured anchors
  - `go_to_anchor <n>` — jump to anchor
  - `get_output_info` — per-output camera/zoom state
  - `set_window_opacity <id> <val>` — runtime opacity override
  - `list_rules` — dump active window rules
- Ship a shell wrapper script `canvaswm-msg` (similar to `swaymsg`) in `extras/`
- waybar custom module example: show canvas x, y, zoom via `canvaswm-msg get_state`

**Acceptance criteria:**
- `canvaswm-msg get_windows` prints JSON window list
- `canvaswm-msg pan_to 1000 500` pans the viewport
- waybar custom module showing canvas coordinates works

---

#### REQ-15: Focus-Follows-Mouse

**Why:** Power user feature. Low effort — just an option in the pointer motion handler.

**What:**
- Config: `focus_follows_mouse = false` (default, current behavior: click-to-focus)
- When `true`: keyboard focus tracks the pointer as it moves over windows, without raising them
- Moving to empty canvas preserves current focus
- Widgets (`widget = true`) are ignored for focus-follows-mouse
- Click still focuses + raises in both modes

**Acceptance criteria:**
- Setting `focus_follows_mouse = true` makes keyboard input follow pointer between windows without clicking

---

#### REQ-16: `--check-config` Completion

**Why:** Already advertised but the `Config::validate()` function doesn't exist.

**What:**
- Add `pub fn validate(path: Option<&str>) -> Result<(), String>` to `canvaswm-config/src/lib.rs`
- Load config, check all fields are in valid ranges (e.g. opacity 0.0–1.0, zoom step > 1.0, etc.)
- Print warnings for unknown keys (best-effort with serde's `deny_unknown_fields` or manual check)

**Acceptance criteria:**
- `canvaswm --check-config` on a valid config prints "Config OK"
- `canvaswm --check-config` on an invalid config prints the error and exits 1

---

### P2 — Packaging & Distribution

#### REQ-17: Packaging

**Why:** driftwm has AUR + Fedora RPM + Nix flake. canvaswm has none. This is the biggest community/adoption gap.

**What:**
- `install.sh`: install binary, session `.desktop` file, example config, example shaders
- `Makefile`: `make install` / `make uninstall`
- `flake.nix`: Nix flake for NixOS and `nix develop` dev shell
- `PKGBUILD`: Arch Linux AUR package
- `.desktop` session file: allows display managers to list canvaswm as a session option

**Acceptance criteria:**
- `sudo make install` installs canvaswm and registers it in display manager
- NixOS users can add canvaswm as a session via the flake

---

## 4. Non-Goals (explicitly out of scope)

- Tiling layout modes
- Workspaces / virtual desktops
- Minimize / iconify
- Built-in compositor panel (keep simple existing panel; defer full taskbar to waybar)
- Vulkan rendering backend (GLES via smithay is sufficient)
- KDE Plasma Shell protocol
- Tablet pressure input

---

## 5. Implementation Order

| # | Requirement | Effort | Impact |
|---|-------------|--------|--------|
| 1 | REQ-04 XWayland window mapping | S | High |
| 2 | REQ-03 Layer shell rendering | M | High |
| 3 | REQ-09 Canvas bookmarks/anchors | S | Medium |
| 4 | REQ-10 Widget/pinned-window mode | S | Medium |
| 5 | REQ-16 --check-config completion | S | Low |
| 6 | REQ-15 Focus-follows-mouse | S | Medium |
| 7 | REQ-05 Blur + opacity | M | High |
| 8 | REQ-11 Foreign toplevel management | M | High |
| 9 | REQ-07 Session lock | M | High |
| 10 | REQ-08 Screencasting | M | High |
| 11 | REQ-12 Interactive minimap | M | High (differentiator) |
| 12 | REQ-13 Animated backgrounds | S | High (differentiator) |
| 13 | REQ-14 IPC ecosystem tools | S | Medium (differentiator) |
| 14 | REQ-02 Trackpad gestures | L | Critical |
| 15 | REQ-06 Multi-monitor | L | High |
| 16 | REQ-01 DRM/KMS backend | L | Critical |
| 17 | REQ-17 Packaging | M | High |

Effort: S = days, M = 1–2 weeks, L = 2–4 weeks

---

## 6. Success Metrics

- canvaswm runs on bare metal (TTY, `--backend=drm`)
- waybar, fuzzel, mako, swaylock all work
- Steam and JetBrains IDEs launch via XWayland
- A frosted-glass blurred terminal renders correctly
- 3-finger swipe pans the canvas on a real trackpad
- canvaswm is installable via AUR and Nix flake
- Zero grey-rectangle artifacts or rendering glitches

---

## 7. Open Questions

1. **Smithay version**: Current `0.7` — verify all required protocols (`ext-session-lock`, `ext-image-capture-source`, `zwlr-foreign-toplevel`) are available in this version before implementing
2. **Blur performance**: Kawase multi-pass via render-to-texture may require `GlesRenderer::render_texture_to_target` — verify API availability in smithay 0.7
3. **Multi-monitor viewport state**: Refactor global `viewport`/`pan_momentum`/`panning` fields into a `HashMap<OutputName, ViewportState>` — decide if this is a breaking state file change
4. **Gesture forwarding** (`wp_pointer_gestures`): Confirm smithay 0.7 has `PointerGesturesState` and the delegate macro
