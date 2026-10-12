//! A title's original opponent where it differs from Pulse's: the branches
//! Wipeout HD's driver adds, read off its executable
//! (`docs/ghidra/functions/ps3-hdfury-eu/ai-controls.md`), and the ones
//! Wipeout 2048's adds to HD's
//! (`docs/ghidra/functions/vita-2048-eu-v104/ai-controls.md`). Data, not a
//! comparison on a title's name (ADR-0058). A title with `None` flies Pulse's
//! driver as recovered (`docs/ghidra/functions/psp-pulse-usa/ai-controls.md`).

use crate::pre_race::Sourced;

/// One title's variant of the original driver.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OriginalDriver {
    /// The formation spacing a finished racer steers by (Pulse `50`).
    pub finished_spread: Sourced<f32>,
    /// A finished racer's formation slot becomes `8 - place` (Pulse: no).
    pub finished_wander_to_place: Sourced<bool>,
    /// An Eliminator race leaves the `WhenBehind` rubber-band term out (Pulse: no).
    pub eliminator_drops_when_behind: Sourced<bool>,
    /// The round robin flags racer `1` first rather than racer `0`.
    pub round_robin_pre_increment: Sourced<bool>,
    /// Circuits on which the steering's lookahead time is halved near the
    /// start line (Pulse: none).
    pub halved_lookahead: &'static [HalvedLookahead],
    /// Every opponent runs its thrust law and lane choice every tick (Pulse:
    /// on its round-robin turn).
    pub every_tick: Sourced<bool>,
    /// The ring's lateral rate follows the event's two AI skills (Pulse: a
    /// fixed `0.001`).
    pub event_ring_rate: Sourced<bool>,
    /// The lane search runs its two directions in a per-driver coin's order
    /// (Pulse: up, then down).
    pub coin_ordered_search: Sourced<bool>,
    /// A choice scored above `0.5` moves the preferred lane (Pulse: no).
    pub preferred_follows_choice: Sourced<bool>,
}

/// One circuit whose original opponent halves its lookahead time outside a
/// window of lap distance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct HalvedLookahead {
    /// The `PI_Track` id the executable compares the race's circuit with, as
    /// the executable spells it (e.g. `05_Track`); which environment and
    /// direction that is comes from the disc's own plugin definition.
    pub track: &'static str,
    /// The speed class the rule is limited to, by name (`Phantom`), or `None`
    /// for every class.
    pub class: Option<&'static str>,
    /// The lap-distance window, in lap-length units, outside which the
    /// lookahead halves.
    pub window: (f32, f32),
}

impl OriginalDriver {
    /// This race's halved-lookahead window: the first row naming `track` (a
    /// `PI_Track` id) whose class is `class` or any, compared without case.
    #[must_use]
    pub fn halved_lookahead_for(&self, track: &str, class: &str) -> Option<(f32, f32)> {
        self.halved_lookahead
            .iter()
            .find(|row| {
                row.track.eq_ignore_ascii_case(track)
                    && row.class.is_none_or(|c| c.eq_ignore_ascii_case(class))
            })
            .map(|row| row.window)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROWS: [HalvedLookahead; 2] = [
        HalvedLookahead {
            track: "05_Track",
            class: Some("Phantom"),
            window: (250.0, 4600.0),
        },
        HalvedLookahead {
            track: "20_Track",
            class: None,
            window: (400.0, 5400.0),
        },
    ];

    const DRIVER: OriginalDriver = OriginalDriver {
        finished_spread: Sourced::measured(75.0),
        finished_wander_to_place: Sourced::measured(true),
        eliminator_drops_when_behind: Sourced::measured(true),
        round_robin_pre_increment: Sourced::measured(true),
        halved_lookahead: &ROWS,
        every_tick: Sourced::measured(false),
        event_ring_rate: Sourced::measured(false),
        coin_ordered_search: Sourced::measured(false),
        preferred_follows_choice: Sourced::measured(false),
    };

    #[test]
    fn a_row_matches_its_circuit_and_class_and_nothing_else() {
        assert_eq!(
            DRIVER.halved_lookahead_for("05_track", "PHANTOM"),
            Some((250.0, 4600.0))
        );
        assert_eq!(DRIVER.halved_lookahead_for("05_Track", "Venom"), None);
        assert_eq!(
            DRIVER.halved_lookahead_for("20_Track", "Venom"),
            Some((400.0, 5400.0))
        );
        assert_eq!(DRIVER.halved_lookahead_for("17_Track", "Phantom"), None);
    }
}
