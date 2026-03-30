/// Actions the compositor can perform in response to input.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Open a terminal emulator.
    SpawnTerminal,
    /// Open an application launcher.
    SpawnLauncher,
    /// Close the focused window.
    CloseWindow,
    /// Reset the canvas viewport to origin and zoom 1.0.
    ResetCanvas,
    /// Zoom in centered on cursor.
    ZoomIn,
    /// Zoom out centered on cursor.
    ZoomOut,
    /// Zoom-to-fit: zoom out to show all windows.
    ZoomToFit,
    /// Center the focused window in the viewport.
    CenterWindow,
    /// Toggle fullscreen for focused window.
    ToggleFullscreen,
    /// Fit window to viewport (maximize/restore toggle).
    FitWindow,
    /// Navigate to nearest window in a direction.
    NavigateDirection(Direction),
    /// Pan viewport by keyboard step.
    PanDirection(Direction),
    /// Nudge focused window position.
    NudgeWindow(Direction),
    /// Toggle home: go to (0,0) or return to previous position.
    HomeToggle,
    /// Cycle windows forward (Alt-Tab).
    CycleForward,
    /// Cycle windows backward (Alt-Shift-Tab).
    CycleBackward,
    /// Reload config file.
    ReloadConfig,
    /// Execute a shell command.
    Exec(String),
    /// Quit the compositor.
    Quit,
}

/// Cardinal + diagonal directions for navigation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl Direction {
    /// Unit vector for this direction.
    pub fn to_unit_vec(&self) -> (f64, f64) {
        match self {
            Direction::Up => (0.0, -1.0),
            Direction::Down => (0.0, 1.0),
            Direction::Left => (-1.0, 0.0),
            Direction::Right => (1.0, 0.0),
        }
    }
}
