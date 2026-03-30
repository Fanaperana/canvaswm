/// Find the nearest item in a 90° cone from `origin` in the given direction.
///
/// Uses dot/cross product against the direction unit vector: a candidate is
/// in the cone when `dot > 0 && |cross| <= dot` (within ±45° of the direction).
/// Scores by `distance / cos(angle)` — targets aligned with the exact direction
/// are preferred even if further away.
pub fn find_nearest<W: PartialEq>(
    origin: (f64, f64),
    dir: (f64, f64),
    items: impl Iterator<Item = (W, (f64, f64))>,
    skip: Option<&W>,
) -> Option<W> {
    let (ux, uy) = dir;
    let mut best: Option<(W, f64)> = None;

    for (item, center) in items {
        if skip.is_some_and(|s| s == &item) {
            continue;
        }
        let dx = center.0 - origin.0;
        let dy = center.1 - origin.1;
        let dot = dx * ux + dy * uy;
        let cross = (dx * uy - dy * ux).abs();
        if dot > 0.0 && cross <= dot {
            // score = dist² / dot ∝ dist / cos(angle), avoids sqrt
            let dist_sq = dx * dx + dy * dy;
            let score = dist_sq / dot;
            if best.as_ref().is_none_or(|(_, d)| score < *d) {
                best = Some((item, score));
            }
        }
    }

    best.map(|(w, _)| w)
}

/// Bounding box of all windows. Returns None if empty.
pub fn all_windows_bbox(
    windows: impl Iterator<Item = (i32, i32, i32, i32)>, // (x, y, w, h)
) -> Option<(f64, f64, f64, f64)> {
    let mut min_x = i32::MAX;
    let mut min_y = i32::MAX;
    let mut max_x = i32::MIN;
    let mut max_y = i32::MIN;
    let mut any = false;

    for (x, y, w, h) in windows {
        any = true;
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x + w);
        max_y = max_y.max(y + h);
    }

    if any {
        Some((
            min_x as f64,
            min_y as f64,
            (max_x - min_x) as f64,
            (max_y - min_y) as f64,
        ))
    } else {
        None
    }
}

/// Closest point on an axis-aligned rect to `origin`.
pub fn closest_point_on_rect(
    origin: (f64, f64),
    rect_x: f64,
    rect_y: f64,
    rect_w: f64,
    rect_h: f64,
) -> (f64, f64) {
    (
        origin.0.clamp(rect_x, rect_x + rect_w),
        origin.1.clamp(rect_y, rect_y + rect_h),
    )
}

/// Fraction of a rectangle's area visible in the current viewport (0.0–1.0).
#[allow(clippy::too_many_arguments)]
pub fn visible_fraction(
    rect_x: f64,
    rect_y: f64,
    rect_w: f64,
    rect_h: f64,
    cam_x: f64,
    cam_y: f64,
    viewport_w: f64,
    viewport_h: f64,
    zoom: f64,
) -> f64 {
    let area = rect_w * rect_h;
    if area <= 0.0 {
        return 0.0;
    }
    let vw = viewport_w / zoom;
    let vh = viewport_h / zoom;

    let ix_min = rect_x.max(cam_x);
    let ix_max = (rect_x + rect_w).min(cam_x + vw);
    let iy_min = rect_y.max(cam_y);
    let iy_max = (rect_y + rect_h).min(cam_y + vh);

    let iw = (ix_max - ix_min).max(0.0);
    let ih = (iy_max - iy_min).max(0.0);

    (iw * ih) / area
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn find_nearest_right() {
        let origin = (0.0, 0.0);
        let items = vec![
            ("a", (100.0, 0.0)),
            ("b", (-100.0, 0.0)),
            ("c", (200.0, 0.0)),
        ];
        let result = find_nearest(origin, (1.0, 0.0), items.into_iter(), None::<&&str>);
        assert_eq!(result, Some("a"));
    }

    #[test]
    fn find_nearest_skips_self() {
        let origin = (0.0, 0.0);
        let items = vec![("self", (10.0, 0.0)), ("other", (20.0, 0.0))];
        let result = find_nearest(origin, (1.0, 0.0), items.into_iter(), Some(&"self"));
        assert_eq!(result, Some("other"));
    }

    #[test]
    fn find_nearest_outside_cone() {
        let origin = (0.0, 0.0);
        let items = vec![("diagonal", (50.0, 100.0))];
        let result = find_nearest(origin, (1.0, 0.0), items.into_iter(), None::<&&str>);
        assert_eq!(result, None);
    }

    #[test]
    fn bbox_computation() {
        let windows = vec![(0, 0, 100, 100), (200, 200, 50, 50)];
        let bbox = all_windows_bbox(windows.into_iter());
        assert_eq!(bbox, Some((0.0, 0.0, 250.0, 250.0)));
    }
}
