//! A post chain's GPU cost, which stops being reported the frame the chain stops running.

use super::{Meter, Stats};

/// What one optional chain (the motion blur, HD's bloom) costs on the GPU, as a
/// [`Meter`] that forgets when the chain is not drawn.
///
/// **A [`Meter`] on its own freezes.** A chain that is not encoded writes no
/// reading, so the window keeps its last samples and the overlay goes on
/// naming them: a split screen draws no motion blur (`docs/rendering/split-screen.md`,
/// rule 3), yet the row showed the cost of the last single view, or of the
/// first frames after a load, for as long as the race ran. A reading is a
/// frame or more old, so clearing alone is not enough either: the readings
/// of frames already in flight land after it and would refill the window, which is
/// what `floor` is for.
#[derive(Debug, Clone, Default)]
pub struct ChainCost {
    meter: Meter,
    /// The newest frame whose reading is no longer wanted: a reading for this
    /// frame or an earlier one is dropped when it lands.
    floor: u64,
}

impl ChainCost {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Records `seconds` for `frame`, unless the chain was already given up on at or after it.
    pub fn record(&mut self, frame: u64, seconds: f32) {
        if frame > self.floor {
            self.meter.record(seconds);
        }
    }

    /// The chain was not drawn on `frame` (it is off, or the view has none):
    /// forget what it cost and refuse every reading up to this frame.
    ///
    /// Returns whether a reading was being reported, so a caller can log the edge.
    pub fn idle(&mut self, frame: u64) -> bool {
        let reported = !self.meter.is_empty();
        self.meter.clear();
        self.floor = self.floor.max(frame);
        reported
    }

    /// A load or stall on `frame`, whose readings are a load and not a cost.
    pub fn stalled(&mut self, frame: u64) {
        self.idle(frame);
    }

    #[must_use]
    pub fn stats(&self) -> Option<Stats> {
        self.meter.stats()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mean_ms(cost: &ChainCost) -> Option<f32> {
        cost.stats().map(|stats| stats.mean_ms)
    }

    #[test]
    fn a_chain_that_runs_is_reported() {
        let mut cost = ChainCost::new();
        for frame in 1..=10 {
            cost.record(frame, 0.030);
        }
        assert!((mean_ms(&cost).unwrap() - 30.0).abs() < 0.01);
    }

    #[test]
    fn a_chain_that_stops_running_stops_being_reported() {
        let mut cost = ChainCost::new();
        for frame in 1..=10 {
            cost.record(frame, 0.030);
        }
        // A split begins on frame 11: nothing is encoded from here on.
        assert!(cost.idle(11));
        assert_eq!(mean_ms(&cost), None);
        assert!(!cost.idle(12), "already idle: no second edge");
    }

    #[test]
    fn readings_still_in_flight_do_not_refill_it() {
        let mut cost = ChainCost::new();
        cost.idle(11);
        // Frames 9 and 10 were encoded before the cut and their readings land late.
        cost.record(9, 0.030);
        cost.record(10, 0.030);
        cost.record(11, 0.030);
        assert_eq!(mean_ms(&cost), None);
        // The chain comes back (a single view again): frame 12 counts.
        cost.record(12, 0.0025);
        assert!((mean_ms(&cost).unwrap() - 2.5).abs() < 0.01);
    }

    #[test]
    fn a_stall_drops_what_was_recorded_and_what_is_in_flight() {
        let mut cost = ChainCost::new();
        cost.record(3, 0.200);
        cost.stalled(4);
        cost.record(4, 0.300);
        assert_eq!(mean_ms(&cost), None);
        cost.record(5, 0.002);
        assert!((mean_ms(&cost).unwrap() - 2.0).abs() < 0.01);
    }
}
