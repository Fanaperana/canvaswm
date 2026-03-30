//! Window snapping — magnetic edge alignment during move grabs.
//!
//! When a window is dragged near the edge of another window (within `distance` pixels),
//! it snaps to align with a configurable gap. Dragging further than `break_force`
//! pixels past the snap point breaks free.

/// Snap result for one axis.
#[derive(Debug, Clone, Copy)]
pub struct SnapResult {
    /// Suggested coordinate after snapping (None = no snap).
    pub position: Option<f64>,
    /// Whether we are currently locked to a snap line.
    pub locked: bool,
}

/// Compute snap for a window being moved.
///
/// `moving_rect`: (x, y, w, h) of the window being dragged.
/// `others`: iterator of (x, y, w, h) for all other windows.
/// `gap`: desired gap between snapped edges.
/// `distance`: snap activation threshold in canvas pixels.
/// `break_force`: pixels past snap to break free (0 = no break free).
///
/// Returns (snap_x, snap_y) — each is Some(new_position) if snapped.
pub fn compute_snap(
    moving_rect: (f64, f64, f64, f64),
    others: impl Iterator<Item = (f64, f64, f64, f64)>,
    gap: f64,
    distance: f64,
) -> (Option<f64>, Option<f64>) {
    let (mx, my, mw, mh) = moving_rect;

    // Edges of the moving window
    let m_left = mx;
    let m_right = mx + mw;
    let m_top = my;
    let m_bottom = my + mh;

    let mut best_snap_x: Option<(f64, f64)> = None; // (snap_to, dist)
    let mut best_snap_y: Option<(f64, f64)> = None;

    for (ox, oy, ow, oh) in others {
        let o_left = ox;
        let o_right = ox + ow;
        let o_top = oy;
        let o_bottom = oy + oh;

        // Only consider windows that overlap on the perpendicular axis
        let y_overlap = m_top < o_bottom + distance && m_bottom > o_top - distance;
        let x_overlap = m_left < o_right + distance && m_right > o_left - distance;

        if y_overlap {
            // Horizontal snapping
            // Left edge of moving → right edge of other (with gap)
            check_snap(m_left, o_right + gap, distance, &mut best_snap_x);
            // Right edge of moving → left edge of other (with gap)
            check_snap_trailing(m_right, o_left - gap, mw, distance, &mut best_snap_x);
            // Left-to-left alignment
            check_snap(m_left, o_left, distance, &mut best_snap_x);
            // Right-to-right alignment
            check_snap_trailing(m_right, o_right, mw, distance, &mut best_snap_x);
        }

        if x_overlap {
            // Vertical snapping
            // Top edge of moving → bottom edge of other (with gap)
            check_snap(m_top, o_bottom + gap, distance, &mut best_snap_y);
            // Bottom edge of moving → top edge of other (with gap)
            check_snap_trailing(m_bottom, o_top - gap, mh, distance, &mut best_snap_y);
            // Top-to-top alignment
            check_snap(m_top, o_top, distance, &mut best_snap_y);
            // Bottom-to-bottom alignment
            check_snap_trailing(m_bottom, o_bottom, mh, distance, &mut best_snap_y);
        }
    }

    (
        best_snap_x.map(|(pos, _)| pos),
        best_snap_y.map(|(pos, _)| pos),
    )
}

/// Check if a leading edge (left/top) should snap.
fn check_snap(edge: f64, target: f64, threshold: f64, best: &mut Option<(f64, f64)>) {
    let dist = (edge - target).abs();
    if dist < threshold
        && (best.is_none() || dist < best.unwrap().1) {
            *best = Some((target, dist));
        }
}

/// Check if a trailing edge (right/bottom) should snap. Returns position for leading edge.
fn check_snap_trailing(
    trailing_edge: f64,
    target: f64,
    size: f64,
    threshold: f64,
    best: &mut Option<(f64, f64)>,
) {
    let dist = (trailing_edge - target).abs();
    if dist < threshold {
        let leading_pos = target - size;
        if best.is_none() || dist < best.unwrap().1 {
            *best = Some((leading_pos, dist));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn snap_right_edge_to_left_edge() {
        // Moving window at (100, 100, 200, 200)
        // Other window at (310, 100, 200, 200) — gap=10 means snap right→left
        let others = vec![(310.0, 100.0, 200.0, 200.0)];
        let (sx, sy) = compute_snap(
            (100.0, 100.0, 200.0, 200.0),
            others.into_iter(),
            10.0,
            25.0,
        );
        // Right edge of moving (300) should snap to left edge of other - gap (310-10=300)
        // So x stays at 100 (300 - 200 = 100)
        assert_eq!(sx, Some(100.0));
        // Y also snaps because tops are aligned (both at 100, distance=0)
        assert_eq!(sy, Some(100.0));
    }

    #[test]
    fn no_snap_when_far() {
        let others = vec![(500.0, 500.0, 200.0, 200.0)];
        let (sx, sy) = compute_snap(
            (0.0, 0.0, 100.0, 100.0),
            others.into_iter(),
            10.0,
            25.0,
        );
        assert!(sx.is_none());
        assert!(sy.is_none());
    }

    #[test]
    fn snap_alignment() {
        // Both at same Y, moving window left edge near other's left edge
        let others = vec![(202.0, 0.0, 200.0, 200.0)];
        let (sx, _sy) = compute_snap(
            (200.0, 0.0, 100.0, 100.0),
            others.into_iter(),
            10.0,
            25.0,
        );
        // Should snap left-to-left: moving.left (200) → other.left (202)
        assert_eq!(sx, Some(202.0));
    }
}
