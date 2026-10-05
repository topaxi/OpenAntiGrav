//! Which of the end-of-race screens is on top, and what moves it on.
//!
//! The GPU-free half of [`super::EndRaceRuntime`], split out so the order
//! `Race End Photo` -> `EndRace Results` -> (`Rewards`) -> `Menu` and the
//! point at which a press may leave the first of them are tested without a
//! device. The drawing of each screen stays with the runtime.

use oag_ui_screens::endrace::photo;

/// Which of the screens is on top. `Photo` is `Race End Photo`, the state
/// the original sits in between the flag and `EndRace Results`; see
/// [`oag_ui_screens::endrace::photo`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Which {
    Photo,
    Results,
    Rewards,
    Menu,
}

/// Whether a race's end-race flow opens on `Race End Photo`: the race ended in a way whose
/// world runs on (the line, or a Single Race wreck - `Race::runs_on_after_the_end`), the disc's
/// screen read, and the title is not Wipeout HD/Fury, whose equivalent was not read.
pub(super) fn opens_on_photo(runs_on: bool, photo_read: bool, hd: bool) -> bool {
    runs_on && photo_read && !hd
}

/// The screen on top and, on `Race End Photo`, the ticks since the finish.
#[derive(Debug, Clone, Copy)]
pub(super) struct Flow {
    which: Which,
    photo_ticks: u32,
}

impl Flow {
    /// Starts on `Race End Photo` when the disc's screen read, on `Results`
    /// otherwise - the order the race had before that screen was read.
    pub(super) fn new(with_photo: bool) -> Self {
        Self {
            which: if with_photo {
                Which::Photo
            } else {
                Which::Results
            },
            photo_ticks: 0,
        }
    }

    pub(super) fn which(&self) -> Which {
        self.which
    }

    /// Ticks since the finish while on `Race End Photo`, for its draw list.
    pub(super) fn photo_ticks(&self) -> u32 {
        self.photo_ticks
    }

    /// One tick. Only `Race End Photo` keeps a clock.
    pub(super) fn tick(&mut self) {
        if self.which == Which::Photo {
            self.photo_ticks = self.photo_ticks.saturating_add(1);
        }
    }

    /// Whether a confirm press may leave the screen on top. `Race End Photo`
    /// reads it only once the original's front end has entered the state
    /// ([`photo::entered`]); before that a thrust button held across the
    /// line, or a click, is aimed at a race still on `InGame`. Every other
    /// screen reads it at once.
    pub(super) fn takes_confirm(&self) -> bool {
        self.which != Which::Photo || photo::entered(self.photo_ticks)
    }

    /// The next screen: `Race End Photo` goes to `Results`, which goes to
    /// `Rewards` when a campaign cell was in play (the disc's own
    /// `EndRaceRewardsRedirect` branch) and straight to `Menu` otherwise;
    /// `Rewards` always goes to `Menu`, which stays.
    pub(super) fn advance(&mut self, campaign_rewards: bool) {
        self.which = match self.which {
            Which::Photo => Which::Results,
            Which::Results if campaign_rewards => Which::Rewards,
            Which::Results | Which::Rewards | Which::Menu => Which::Menu,
        };
    }

    /// `VIEW RESULTS AGAIN`: back to `Results`, never to `Race End Photo`.
    pub(super) fn view_results_again(&mut self) {
        self.which = Which::Results;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_race_whose_world_runs_on_opens_on_the_photo_and_not_hd() {
        assert!(opens_on_photo(true, true, false));
        assert!(
            !opens_on_photo(false, true, false),
            "a Zone wreck, an Eliminator end"
        );
        assert!(
            !opens_on_photo(true, false, false),
            "the screen did not read"
        );
        assert!(!opens_on_photo(true, true, true), "HD's was not read");
    }

    #[test]
    fn a_race_whose_photo_screen_read_starts_there_and_one_whose_did_not_does_not() {
        assert_eq!(Flow::new(true).which(), Which::Photo);
        assert_eq!(Flow::new(false).which(), Which::Results);
    }

    #[test]
    fn a_press_before_the_state_is_entered_does_nothing_and_after_it_goes_to_the_results() {
        let mut flow = Flow::new(true);
        for _ in 0..photo::ENTER_TICKS - 1 {
            flow.tick();
            assert!(!flow.takes_confirm(), "tick {}", flow.photo_ticks());
        }
        flow.tick();
        assert!(flow.takes_confirm());
        flow.advance(false);
        assert_eq!(flow.which(), Which::Results);
    }

    #[test]
    fn the_clock_stops_when_the_state_is_left() {
        let mut flow = Flow::new(true);
        for _ in 0..photo::ENTER_TICKS {
            flow.tick();
        }
        flow.advance(false);
        flow.tick();
        assert_eq!(flow.photo_ticks(), photo::ENTER_TICKS);
        assert!(flow.takes_confirm());
    }

    #[test]
    fn the_panels_follow_in_the_disc_order_and_view_results_again_skips_the_photo() {
        let mut flow = Flow::new(false);
        flow.advance(true);
        assert_eq!(flow.which(), Which::Rewards);
        flow.advance(true);
        assert_eq!(flow.which(), Which::Menu);
        flow.advance(true);
        assert_eq!(flow.which(), Which::Menu);
        flow.view_results_again();
        assert_eq!(flow.which(), Which::Results);
        flow.advance(false);
        assert_eq!(flow.which(), Which::Menu);
    }
}
