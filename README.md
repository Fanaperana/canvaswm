<div align="center">
  <h1>🖼️ canvaswm</h1>
  <p><strong>An infinite canvas Wayland compositor — arrange windows freely on a zoomable 2D surface.</strong></p>

  ![License](https://img.shields.io/github/license/Fanaperana/canvaswm?style=for-the-badge)
  ![Stars](https://img.shields.io/github/stars/Fanaperana/canvaswm?style=for-the-badge)
  ![Built with Rust](https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white)
  ![Wayland](https://img.shields.io/badge/Wayland-FFB300?style=for-the-badge&logo=wayland&logoColor=white)
  ![Last Commit](https://img.shields.io/github/last-commit/Fanaperana/canvaswm?style=for-the-badge)
</div>

---

## 🧪 What is this?

**canvaswm** is an experimental Wayland compositor built with [Smithay](https://github.com/Smithay/smithay). Instead of a traditional tiling or floating window layout, it places all windows on an infinite 2D canvas — you pan and zoom freely, like a whiteboard for your apps.

## ✨ Features

- 🗺️ Infinite zoomable 2D canvas for window management
- 🖱️ Pan and zoom with mouse/trackpad gestures
- 🦀 Built in Rust using the Smithay compositor framework
- 🐧 Native Wayland — no X11 required

## 📦 Requirements

- Linux with Wayland
- Rust toolchain (`rustup`)
- Smithay dependencies:

```bash
sudo apt install libwayland-dev libxkbcommon-dev libudev-dev libinput-dev libgbm-dev
```

## 🚀 Build & Run

```bash
git clone https://github.com/Fanaperana/canvaswm
cd canvaswm
cargo build --release
./target/release/canvaswm
```

> ⚠️ Run from a TTY or nested Wayland session for testing.

## 🗺️ Roadmap

- [ ] Smooth pan/zoom animations
- [ ] Window grouping / workspaces on canvas
- [ ] Keyboard-driven navigation
- [ ] Config file support

## 📄 License

MIT © [Prince Ralambomanarivo](https://github.com/Fanaperana)
