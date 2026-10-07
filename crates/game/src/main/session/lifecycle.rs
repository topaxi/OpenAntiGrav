//! What the window going away does to a race: the decision, and the pause.
//!
//! The simulation never sees any of this. A window event reaches the
//! composition root, which either parks the race through the very path the
//! Android Back key uses ([`Session::back`]: the pause menu over a parked race)
//! or, in a `--race` run that has no menus, sets the freeze the Start button
//! sets ([`Session::paused`]). Either way only whether `Race::tick` runs
//! changes, so no state hash, replay or ghost can tell.
//!
//! Which events count, and the default of the one setting, are **chosen, not
//! measured**: no original title was observed on a phone or a desktop window.

use log::info;

use crate::stage::Stage;

use super::Session;

/// A window event that may pause a race.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Away {
    /// Android `Suspended`: home, app switch, screen off.
    Suspended,
    /// The desktop window was minimised or fully hidden.
    Minimized,
    /// The window lost keyboard focus (alt-tab, the Steam overlay).
    FocusLost,
}

/// What the race should do about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Pause {
    /// Leave it: not racing, already paused, or focus loss with the setting off.
    Nothing,
    /// Open the pause menu over the parked race.
    Menu,
    /// Freeze the tick with no menu, for a run that has no menus.
    Freeze,
}

/// Which event pauses which state. `running` is an unfinished race on
/// screen and not frozen by Start; `has_menus` is whether a front end exists
/// to open.
pub(crate) fn decide(
    away: Away,
    pause_on_focus_loss: bool,
    running: bool,
    has_menus: bool,
) -> Pause {
    if !running || (away == Away::FocusLost && !pause_on_focus_loss) {
        Pause::Nothing
    } else if has_menus {
        Pause::Menu
    } else {
        Pause::Freeze
    }
}

/// Whether the audio device should stop for this event. Focus loss leaves it
/// playing: a player who alt-tabs to a second window still hears the race
/// menu's music, and nothing is wrong with that.
pub(crate) fn silences(away: Away) -> bool {
    matches!(away, Away::Suspended | Away::Minimized)
}

impl Session {
    /// Applies [`decide`] to the live session.
    pub(crate) fn window_away(&mut self, away: Away) {
        let running = matches!(&self.stage, Stage::Race(stage) if !stage.race.finished())
            && !self.paused;
        let pause = decide(
            away,
            self.settings.display.pause_on_focus_loss,
            running,
            self.shell.is_some(),
        );
        info!("window away ({away:?}): {pause:?}");
        match pause {
            Pause::Nothing => {}
            Pause::Menu => self.back(),
            Pause::Freeze => self.paused = true,
        }
        if silences(away) {
            self.audio.set_suspended(true);
        }
    }

    /// The window is back on screen: the device restarts. A race that was
    /// paused stays paused.
    pub(crate) fn window_back(&mut self) {
        self.audio.set_suspended(false);
        // The gap is not a frame anyone presented.
        self.stalled = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suspend_and_minimise_always_pause_a_running_race() {
        for away in [Away::Suspended, Away::Minimized] {
            assert_eq!(decide(away, false, true, true), Pause::Menu);
            assert_eq!(decide(away, true, true, true), Pause::Menu);
        }
    }

    #[test]
    fn focus_loss_pauses_only_when_the_setting_allows_it() {
        assert_eq!(decide(Away::FocusLost, true, true, true), Pause::Menu);
        assert_eq!(decide(Away::FocusLost, false, true, true), Pause::Nothing);
    }

    #[test]
    fn nothing_pauses_what_is_not_a_running_race() {
        for away in [Away::Suspended, Away::Minimized, Away::FocusLost] {
            assert_eq!(decide(away, true, false, true), Pause::Nothing);
        }
    }

    #[test]
    fn a_run_without_menus_freezes_instead() {
        assert_eq!(decide(Away::Suspended, true, true, false), Pause::Freeze);
    }

    #[test]
    fn only_hidden_windows_silence_the_audio() {
        assert!(silences(Away::Suspended));
        assert!(silences(Away::Minimized));
        assert!(!silences(Away::FocusLost));
    }
}
