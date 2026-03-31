#![allow(irrefutable_let_patterns)]

mod drm;
mod grabs;
mod handlers;
mod input;
mod ipc;
mod state;
mod winit;

use smithay::reexports::{calloop::EventLoop, wayland_server::Display};
pub use state::CanvasWM;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    // Handle --check-config flag
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--check-config") {
        match canvaswm_config::Config::validate(None) {
            Ok(()) => {
                println!("Config is valid.");
                return Ok(());
            }
            Err(e) => {
                eprintln!("Config error: {e}");
                std::process::exit(1);
            }
        }
    }

    let mut event_loop: EventLoop<CanvasWM> = EventLoop::try_new()?;
    let display: Display<CanvasWM> = Display::new()?;
    let mut state = CanvasWM::new(&mut event_loop, display);

    // Select backend: --backend=drm for native, default is winit
    let use_drm = args.iter().any(|a| a == "--backend=drm" || a == "--drm");

    if use_drm {
        let _drm_data = crate::drm::init_drm(&mut event_loop, &mut state)?;
        tracing::info!("DRM/KMS backend initialized");
    } else {
        crate::winit::init_winit(&mut event_loop, &mut state)?;
    }

    // Set WAYLAND_DISPLAY so child processes connect to us
    std::env::set_var("WAYLAND_DISPLAY", &state.socket_name);

    // Ensure terminals get proper color support (standard Wayland compositor practice)
    if std::env::var_os("TERM").is_none() {
        std::env::set_var("TERM", "xterm-256color");
    }
    if std::env::var_os("COLORTERM").is_none() {
        std::env::set_var("COLORTERM", "truecolor");
    }

    // Force Wayland backend for toolkit apps (prevents them falling back to X11)
    std::env::set_var("GDK_BACKEND", "wayland");
    std::env::set_var("QT_QPA_PLATFORM", "wayland");
    std::env::set_var("SDL_VIDEODRIVER", "wayland");
    std::env::set_var("MOZ_ENABLE_WAYLAND", "1");
    // Remove DISPLAY so X11 apps don't try to connect to host X server
    std::env::remove_var("DISPLAY");

    // Set env vars from config (can override the defaults above)
    for (k, v) in &state.config.env {
        std::env::set_var(k, v);
    }

    println!("╔════════════════════════════════════════════════════════╗");
    println!("║           CanvasWM — Infinite Canvas Compositor       ║");
    println!("╠════════════════════════════════════════════════════════╣");
    println!("║  Super+Return       Open terminal                     ║");
    println!("║  Super+D            Open app launcher                 ║");
    println!("║  Super+Q            Close focused window              ║");
    println!("║  Super+LMB drag     Pan viewport                      ║");
    println!("║  Super+Scroll       Zoom at cursor                    ║");
    println!("║  Super+=/-          Zoom in / out                     ║");
    println!("║  Super+W            Zoom-to-fit (overview)            ║");
    println!("║  Super+C            Center focused window             ║");
    println!("║  Super+F            Toggle fullscreen                 ║");
    println!("║  Super+Arrow        Navigate to window                ║");
    println!("║  Super+Home         Home toggle                       ║");
    println!("║  Super+R            Reload config                     ║");
    println!("║  Alt+Tab            Cycle windows                     ║");
    println!("║  Super+0            Reset viewport                    ║");
    println!("║  Alt+LMB drag       Move window                       ║");
    println!("║  MMB drag           Move window (nested mode)         ║");
    println!("║  Alt+RMB drag       Resize window                     ║");
    println!("║  Super+Escape       Quit                              ║");
    println!("╚════════════════════════════════════════════════════════╝");
    println!();
    println!("Wayland socket: {:?}", state.socket_name);

    // Run autostart commands from config
    for cmd in &state.config.autostart {
        if let Err(e) = std::process::Command::new("sh").arg("-c").arg(cmd).spawn() {
            tracing::warn!("Autostart failed for '{}': {}", cmd, e);
        }
    }

    // Spawn a default client if requested via CLI (or auto-spawn terminal)
    spawn_client();

    event_loop.run(None, &mut state, move |_| {
        // CanvasWM is running
    })?;

    Ok(())
}

fn init_logging() {
    if let Ok(env_filter) = tracing_subscriber::EnvFilter::try_from_default_env() {
        tracing_subscriber::fmt().with_env_filter(env_filter).init();
    } else {
        tracing_subscriber::fmt().init();
    }
}

fn spawn_client() {
    let mut args = std::env::args().skip(1);
    let flag = args.next();
    let arg = args.next();

    match (flag.as_deref(), arg) {
        (Some("-c") | Some("--command"), Some(command)) => {
            std::process::Command::new(command).spawn().ok();
        }
        _ => {
            for term in &["alacritty", "foot", "kitty"] {
                if std::process::Command::new(term).spawn().is_ok() {
                    return;
                }
            }
        }
    }
}
