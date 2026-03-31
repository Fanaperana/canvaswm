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
/// Uses a candidate-edge approach: generates potential positions by sliding
/// the new window to the edges of existing windows, then picks the closest
/// non-overlapping candidate to the desired center.
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

    // Desired top-left (centered on cx, cy)
    let ideal_x = cx - w / 2.0;
    let ideal_y = cy - h / 2.0;
    let ideal = (ideal_x, ideal_y, w, h);

    // If the ideal position is free, use it
    if !existing.iter().any(|e| rects_overlap(ideal, *e, gap)) {
        return (ideal_x, ideal_y);
    }

    // Generate candidate positions by placing the new window adjacent to
    // each existing window's edges (right, left, below, above), aligned
    // vertically/horizontally to the desired center.
    let mut candidates: Vec<(f64, f64)> = Vec::new();

    for &(ex, ey, ew, eh) in existing {
        // Right of existing window
        candidates.push((ex + ew + gap, cy - h / 2.0));
        // Left of existing window
        candidates.push((ex - w - gap, cy - h / 2.0));
        // Below existing window
        candidates.push((cx - w / 2.0, ey + eh + gap));
        // Above existing window
        candidates.push((cx - w / 2.0, ey - h - gap));

        // Also try aligning tops/bottoms
        candidates.push((ex + ew + gap, ey));
        candidates.push((ex - w - gap, ey));
        candidates.push((ex + ew + gap, ey + eh - h));
        candidates.push((ex - w - gap, ey + eh - h));
    }

    // Score each candidate by distance to the ideal position, pick closest free one
    let mut best: Option<(f64, f64, f64)> = None; // (x, y, dist²)
    for (px, py) in candidates {
        let cand = (px, py, w, h);
        if existing.iter().any(|e| rects_overlap(cand, *e, gap)) {
            continue;
        }
        let dx = px - ideal_x;
        let dy = py - ideal_y;
        let dist2 = dx * dx + dy * dy;
        if best.is_none() || dist2 < best.unwrap().2 {
            best = Some((px, py, dist2));
        }
    }

    if let Some((bx, by, _)) = best {
        return (bx, by);
    }

    // Fallback: cascade offset from last existing window
    let n = existing.len() as f64;
    (ideal_x + n * 30.0, ideal_y + n * 30.0)
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
        .map(|(i, (dx, dy))| (i, windows[i].0 + dx, windows[i].1 + dy))
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
        // Should NOT overlap with the existing window
        let new_rect = (pos.0, pos.1, 600.0, 400.0);
        assert!(!rects_overlap(new_rect, existing[0], 10.0));
    }

    #[test]
    fn test_placed_adjacent_no_gap_violation() {
        // Existing window at (100, 100, 600, 400)
        let existing = vec![(100.0, 100.0, 600.0, 400.0)];
        let gap = 20.0;
        let pos = find_free_position(400.0, 300.0, 600.0, 400.0, &existing, gap);
        let new_rect = (pos.0, pos.1, 600.0, 400.0);
        // Must not overlap (respecting gap)
        assert!(!rects_overlap(new_rect, existing[0], gap));
        // Should be reasonably close to intended center
        let dist = ((pos.0 + 300.0 - 400.0).powi(2) + (pos.1 + 200.0 - 300.0).powi(2)).sqrt();
        assert!(dist < 1500.0, "placed too far away: dist={dist}");
    }

    #[test]
    fn test_collision_resolution() {
        let windows = vec![(0.0, 0.0, 100.0, 100.0), (50.0, 50.0, 100.0, 100.0)];
        let moves = resolve_collisions(&windows, 0.0, 1.0);
        assert!(!moves.is_empty());
    }
}
