//! The race side of HD's authored mix: which state the groups chase. The groups,
//! law and loader are in [`crate::hd_mix`]; this hands the mix to `Audio` from
//! the race's banks when a race starts with no front end behind it (`--race`),
//! and says whether the grid or the race is running.

use super::super::Audio;
use super::super::hd_mix::{Live, Maps, State};

impl Audio {
    /// Chooses this tick's mix state from the race. A no-op on a title with no
    /// authored mix.
    ///
    /// Three of the original's states are not modelled: critical-energy,
    /// player-dead and post-race (the fly-over's `PreRace` is set when the race
    /// loads, [`Audio::enter_race_mix`]), which change only a few groups;
    /// `RaceNormal` is used throughout. Chosen, not measured.
    pub(super) fn hd_race_mix(&mut self, mix: Option<&Maps>, gated: bool) {
        if self.hd.live.is_none()
            && let Some(maps) = mix
        {
            self.hd.live = Some(Live::new(maps.clone()));
        }
        self.hd.state = if gated {
            State::Countdown
        } else {
            State::RaceNormal
        };
    }
}
