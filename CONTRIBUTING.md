# Contributing to CanvasWM

Thank you for your interest in contributing! This guide will help you get started.

## Code of Conduct

This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code. Please report unacceptable behavior to **canvaswm@dev**.

## How to Contribute

### Reporting Bugs

1. Check [existing issues](https://github.com/Fanaperana/canvaswm/issues) to avoid duplicates
2. Use the **Bug Report** issue template
3. Include:
   - CanvasWM version (`canvaswm --version`)
   - Your Linux distribution and Wayland compositor details
   - Steps to reproduce
   - Expected vs. actual behavior
   - Relevant log output (`RUST_LOG=debug cargo run 2>&1`)

### Suggesting Features

1. Open an issue using the **Feature Request** template
2. Describe the use case, not just the solution
3. Consider how it fits the infinite canvas paradigm

### Submitting Code

#### Setup

```bash
git clone https://github.com/Fanaperana/canvaswm.git
cd canvaswm
cargo build
cargo run  # Runs in a Winit window for development
```

#### Workflow

1. Fork the repository
2. Create a feature branch from `master`:
   ```bash
   git checkout -b feat/your-feature
   ```
3. Make your changes
4. Ensure the project builds without warnings:
   ```bash
   cargo build 2>&1 | grep "^warning"
   ```
5. Format your code:
   ```bash
   cargo fmt
   ```
6. Run clippy:
   ```bash
   cargo clippy -- -D warnings
   ```
7. Commit with a descriptive message following [Conventional Commits](https://www.conventionalcommits.org/):
   ```
   feat(canvas): add pinch-to-zoom gesture support
   fix(render): correct corner clip radius calculation
   refactor(compositor): extract input handling module
   docs: update keybinding table in README
   ```
8. Push and open a Pull Request

#### Commit Message Format

```
<type>(<scope>): <short summary>

<optional body>
```

**Types**: `feat`, `fix`, `refactor`, `docs`, `style`, `test`, `perf`, `ci`, `chore`

**Scopes**: `canvas`, `render`, `input`, `config`, `compositor`

### Project Structure

```
crates/
├── canvaswm-canvas       # Pure math (no Wayland deps) — viewport, momentum, snapping
├── canvaswm-config       # Config parsing (no Wayland deps) — TOML/JSON/YAML
├── canvaswm-input        # Action/Direction types
├── canvaswm-render       # GPU rendering — shaders, decorations, minimap, backgrounds
└── canvaswm-compositor   # Binary — event loop, input, IPC, Smithay integration
```

**Key principle**: Keep Wayland-free crates (`canvas`, `config`, `input`) independent so they remain testable without a display server.

### Code Style

- Follow idiomatic Rust conventions
- Use named constants instead of magic numbers
- Keep rendering logic in `canvaswm-render`, not in the compositor
- Prefer small, focused functions over large monolithic ones
- Add doc comments (`///`) for public APIs

### Areas Where Help Is Needed

- **DRM/KMS backend** — completing the bare-metal event loop
- **Layer-shell protocol** — status bars, launchers, overlays
- **XWayland rendering** — X11 window surface integration
- **Kawase blur** — multi-pass blur implementation
- **Tests** — unit tests for canvas math, config parsing
- **Documentation** — man pages, wiki content
- **Packaging** — AUR, Nix, Debian packages

## Questions?

Open a [discussion](https://github.com/Fanaperana/canvaswm/issues) or reach out in an issue. We're happy to help newcomers get oriented.
