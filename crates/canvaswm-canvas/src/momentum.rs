use std::collections::VecDeque;
use std::time::{Duration, Instant};

/// Sliding-window velocity tracker for scroll/gesture input.
/// Computes launch velocity from recent displacement over a fixed time window.
#[derive(Clone, Default)]
pub struct VelocityTracker {
    samples: VecDeque<(Instant, f64, f64)>,
}

const VELOCITY_WINDOW: Duration = Duration::from_millis(80);

impl VelocityTracker {
    pub fn push(&mut self, now: Instant, dx: f64, dy: f64) {
        self.samples.push_back((now, dx, dy));
        let cutoff = now - VELOCITY_WINDOW;
        while self.samples.front().is_some_and(|(t, _, _)| *t < cutoff) {
            self.samples.pop_front();
        }
    }

    /// Total displacement / elapsed time = px/sec. Zero if < 2 samples.
    pub fn launch_velocity(&self) -> (f64, f64) {
        if self.samples.len() < 2 {
            return (0.0, 0.0);
        }
        let first_time = self.samples.front().unwrap().0;
        let last_time = self.samples.back().unwrap().0;
        let elapsed = (last_time - first_time).as_secs_f64();
        if elapsed < 1e-6 {
            return (0.0, 0.0);
        }
        let (total_x, total_y) = self.samples.iter().fold((0.0, 0.0), |(ax, ay), (_, dx, dy)| (ax + dx, ay + dy));
        (total_x / elapsed, total_y / elapsed)
    }

    pub fn clear(&mut self) {
        self.samples.clear();
    }
}

/// Stop threshold in px/sec (15 px/sec ≈ 0.25 px/frame at 60Hz).
const MOMENTUM_STOP_THRESHOLD: f64 = 15.0;

/// Scroll momentum physics with time-based friction.
/// Velocity is in px/sec; friction is applied via `powf(dt * 60)` for frame-rate independence.
#[derive(Clone)]
pub struct MomentumState {
    pub vx: f64,
    pub vy: f64,
    pub tracker: VelocityTracker,
    pub friction: f64,
    pub coasting: bool,
}

impl MomentumState {
    pub fn new(friction: f64) -> Self {
        Self {
            vx: 0.0,
            vy: 0.0,
            tracker: VelocityTracker::default(),
            friction,
            coasting: false,
        }
    }

    /// Record an input delta. Resets coasting — we're receiving live input.
    pub fn accumulate(&mut self, dx: f64, dy: f64, now: Instant) {
        self.tracker.push(now, dx, dy);
        self.coasting = false;
    }

    /// Snapshot launch velocity from the tracker and begin coasting.
    pub fn launch(&mut self) {
        let (vx, vy) = self.tracker.launch_velocity();
        self.vx = vx;
        self.vy = vy;
        self.coasting = true;
        self.tracker.clear();
    }

    /// Advance momentum by `dt`. Returns Some((dx, dy)) canvas delta to apply, or None.
    pub fn tick(&mut self, dt: Duration) -> Option<(f64, f64)> {
        if !self.coasting {
            return None;
        }
        let speed = (self.vx * self.vx + self.vy * self.vy).sqrt();
        if speed < MOMENTUM_STOP_THRESHOLD {
            self.stop();
            return None;
        }

        let dt_secs = dt.as_secs_f64();

        // Speed-dependent friction: gentle scrolls stop quickly, fast flings coast longer
        let effective_friction = speed_dependent_friction(self.friction, speed);
        let decay = effective_friction.powf(dt_secs * 60.0);
        let delta = (self.vx * dt_secs, self.vy * dt_secs);
        self.vx *= decay;
        self.vy *= decay;
        Some(delta)
    }

    pub fn stop(&mut self) {
        self.vx = 0.0;
        self.vy = 0.0;
        self.tracker.clear();
        self.coasting = false;
    }

    pub fn is_active(&self) -> bool {
        self.coasting
    }
}

/// Derive effective per-frame friction from the config value and current speed.
fn speed_dependent_friction(friction: f64, speed: f64) -> f64 {
    let low_friction = (friction - 0.06).clamp(0.80, 0.95);
    let high_friction = (friction + 0.025).clamp(0.95, 0.995);
    let reference_speed = 2500.0;
    let t = (speed / reference_speed).min(1.0);
    low_friction + t * (high_friction - low_friction)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn momentum_decays_to_stop() {
        let mut m = MomentumState::new(0.92);
        m.vx = 1000.0;
        m.vy = 0.0;
        m.coasting = true;

        let dt = Duration::from_millis(16);
        let mut total = 0.0;
        for _ in 0..1000 {
            match m.tick(dt) {
                Some((dx, _)) => total += dx,
                None => break,
            }
        }
        assert!(total > 0.0);
        assert!(!m.coasting);
    }
}
