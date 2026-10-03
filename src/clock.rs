// ============================================================================
// clock.rs — Host-side wall-clock sampling
// ============================================================================

//! The motherboard owns the *logical* clock. This module owns the *physical*
//! wall-clock that lives at the host boundary: it measures how much real time
//! elapsed between ticks and produces the `frame_delta_ns` value that an
//! application freezes into its pins.
//!
//! Nothing here is part of the semantic core; it exists so that applications
//! do not each re-implement delta sampling and tab-away clamping.

use std::time::Instant;

/// Wall-clock sampler for the sampling phase of the tick lifecycle.
pub struct Clock {
    last: Instant,
    frame_ns: u64,
    max_delta_ns: u64,
}

impl Clock {
    /// Create a clock targeting `frame_hz` ticks per second.
    ///
    /// `max_delta_ns` caps the reported delta so that a suspended process
    /// resumes gracefully instead of taking one enormous step. A value of `0`
    /// disables the cap.
    pub fn new(frame_hz: u32, max_delta_ns: u64) -> Self {
        let frame_ns = if frame_hz == 0 {
            0
        } else {
            1_000_000_000 / u64::from(frame_hz)
        };
        Self {
            last: Instant::now(),
            frame_ns,
            max_delta_ns,
        }
    }

    /// Nominal nanoseconds per frame for this clock.
    pub fn frame_ns(&self) -> u64 {
        self.frame_ns
    }

    /// Configured maximum reported delta in nanoseconds.
    pub fn max_delta_ns(&self) -> u64 {
        self.max_delta_ns
    }

    /// Restart the clock from "now".
    pub fn reset(&mut self) {
        self.last = Instant::now();
    }

    /// Sample nanoseconds elapsed since the previous sample, clamped to
    /// [`Clock::max_delta_ns`] (unless that cap is `0`).
    pub fn sample_delta_ns(&mut self) -> u64 {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last).as_nanos() as u64;
        self.last = now;
        if self.max_delta_ns == 0 {
            elapsed
        } else {
            elapsed.min(self.max_delta_ns)
        }
    }
}
