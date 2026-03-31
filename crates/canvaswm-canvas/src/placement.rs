/// Rectangle: (x, y, w, h)
type Rect = (f64, f64, f64, f64);

/// Check if two rectangles overlap (with optional gap).
fn rects_overlap(a: Rect, b: Rect, gap: f64) -> bool {
    let (ax, ay, aw, ah) = a;
    let (bx, by, bw, bh) = b;
    ax < bx + bw + gap && ax + aw + gap > bx && ay < by + bh + gap && ay + ah + gap > by
}

/// Find a non-overlapping position for a new window near `(cx, cy)`.
///
/// Tries the target position first, then spirals outward in a grid pattern
/// until a free slot is found. `gap` is the minimum space between windows.
pub fn find_free_position(
    cx: f64,
    cy: f64,
    win_w: f64,
    win_h: f64,
    existing: &[(f64, f64, f64, f64)],
    gap: f64,
) -> (f64, f64) {
    // Default size estimate for windows that haven't committed geometry yet
    let w = if win_w > 0.0 { win_w } else { 600.0 };
    let h = if win_h > 0.0 { win_h } else { 400.0 };

    let candidate = (cx - w / 2.0, cy - h / 2.0, w, h);

    // Check if the target position is free
    if !existing.iter().any(|e| rects_overlap(candidate, *e, gap)) {
        return (candidate.0, candidate.1);
    }

    // Spiral outward: try offsets in expanding rings
    let step_x = w + gap;
    let step_y = h + gap;
    for ring in 1..20 {
        let r = ring as f64;
        // Try positions around the ring (right, down, left, up, and corners)
        let offsets = [
            (r, 0.0),
            (-r, 0.0),
            (0.0, r),
            (0.0, -r),
            (r, r),
            (-r, r),
            (r, -r),
            (-r, -r),
            (r, 0.5 * r),
            (-r, 0.5 * r),
            (0.5 * r, r),
            (0.5 * r, -r),
        ];
        for (dx, dy) in offsets {
            let nx = cx - w / 2.0 + dx * step_x;
            let ny = cy - h / 2.0 + dy * step_y;
            let cand = (nx, ny, w, h);
            if !existing.iter().any(|e| rects_overlap(cand, *e, gap)) {
                return (nx, ny);
            }
        }
    }

    // Fallback: cascade offset from last existing window
    let n = existing.len() as f64;
    (cx - w / 2.0 + n * 30.0, cy - h / 2.0 + n * 30.0)
}

/// Push overlapping windows apart (single iteration).
///
/// Returns a list of `(index, new_x, new_y)` for windows that should move.
/// Call repeatedly until the return is empty for full separation.
pub fn resolve_collisions(
    windows: &[(f64, f64, f64, f64)],
    gap: f64,
    strength: f64,
) -> Vec<(usize, f64, f64)> {
    let n = windows.len();
    let mut displacements = vec![(0.0_f64, 0.0_f64); n];

    for i in 0..n {
        for j in (i + 1)..n {
            let (ax, ay, aw, ah) = windows[i];
            let (bx, by, bw, bh) = windows[j];

            if !rects_overlap(windows[i], windows[j], gap) {
                continue;
            }

            // Compute overlap on each axis
            let overlap_x = (ax + aw + gap - bx).min(bx + bw + gap - ax);
            let overlap_y = (ay + ah + gap - by).min(by + bh + gap - ay);

            // Push along the axis with less overlap (cheaper separation)
            let (dx, dy) = if overlap_x < overlap_y {
                // Separate horizontally
                let center_a = ax + aw / 2.0;
                let center_b = bx + bw / 2.0;
                let sign = if center_a < center_b { -1.0 } else { 1.0 };
                (sign * overlap_x * 0.5 * strength, 0.0)
            } else {
                // Separate vertically
                let center_a = ay + ah / 2.0;
                let center_b = by + bh / 2.0;
                let sign = if center_a < center_b { -1.0 } else { 1.0 };
                (0.0, sign * overlap_y * 0.5 * strength)
            };

            displacements[i].0 += dx;
            displacements[i].1 += dy;
            displacements[j].0 -= dx;
            displacements[j].1 -= dy;
        }
    }

    displacements
        .into_iter()
        .enumerate()
        .filter(|(_, (dx, dy))| dx.abs() > 0.5 || dy.abs() > 0.5)
        .map(|(i, (dx, dy))| {
            (
                i,
                windows[i].0 + dx,
                windows[i].1 + dy,
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_overlap_placed_at_center() {
        let pos = find_free_position(500.0, 300.0, 600.0, 400.0, &[], 10.0);
        assert_eq!(pos, (200.0, 100.0)); // centered: 500 - 300, 300 - 200
    }

    #[test]
    fn test_avoids_existing_window() {
        let existing = vec![(200.0, 100.0, 600.0, 400.0)];
        let pos = find_free_position(500.0, 300.0, 600.0, 400.0, &existing, 10.0);
        // Should NOT be at (200, 100) since that's taken
        assert!(pos.0 != 200.0 || pos.1 != 100.0);
    }

    #[test]
    fn test_collision_resolution() {
        let windows = vec![
            (0.0, 0.0, 100.0, 100.0),
            (50.0, 50.0, 100.0, 100.0),
        ];
        let moves = resolve_collisions(&windows, 0.0, 1.0);
        assert!(!moves.is_empty());
    }
}
