//! The attract demo a title plays when its main menu is left alone: how long the menu waits, how
//! long a demo runs, what ends it, and how its race is chosen.
//!
//! Per title (ADR-0058). Pulse's numbers are measured live
//! (`docs/ghidra/functions/psp-pulse-usa/attract-demo.md`). HD authors the demo's screens in `demo_definition.xml`: `Demo Launch Real`
//! sets the `Mode` global to `Demo` and launches, `Demo InGame` leaves on `forward="any"`, so any
//! press ends it. It authors no timer, so its two durations are Pulse's. 2048 and Omega take HD's
//! (`docs/ui/attract-demo.md`).

use crate::pre_race::Sourced;

/// How a demo's race is picked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    /// A circuit and a team drawn from what the player can already pick, from the seeded `Rng`.
    /// Chosen, not measured: the original's rule is unread, and a recovered one is a new variant
    /// here rather than an edit of this one.
    SeededFromPlayable,
    /// Every circuit of the title's race list, **unlocks ignored**, and a team out of the ones it
    /// marks `multiplayer`; the circuit and the team both differ from the previous demo's. Pulse
    /// PSP, measured live (`docs/ghidra/functions/psp-pulse-usa/attract-demo.md`).
    AnyCircuitMultiplayerTeam,
}

/// Where a demo that ran its full length goes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AfterEnd {
    /// Back to the main menu, whose idle timer then starts the next demo.
    MainMenu,
    /// Straight into the next demo.
    NextDemo,
}

/// What ends a demo before its time is up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exit {
    /// A press of any button, key, click, pointer move or touch.
    AnyInput,
}

/// How a demo's race is set up, apart from which circuit and team.
///
/// Pulse's are measured: `Demo_Construct` and the tests of the game mode in the executable
/// (`docs/ghidra/functions/psp-pulse-usa/attract-demo.md`, "The rest of the set-up" and
/// "What else mode 2 changes").
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Rules {
    /// The speed class every demo flies, by the name the disc spells it.
    pub class: Sourced<&'static str>,
    /// Craft on the grid.
    pub craft: Sourced<u32>,
    /// The laps the race is set to. A demo that is not [`Self::ends_on_laps`] runs them out and
    /// races on.
    pub laps: Sourced<u32>,
    /// Whether finishing the laps ends the demo. The original's does not: only the timeout does.
    pub ends_on_laps: Sourced<bool>,
    /// Whether pickups are on.
    pub weapons: Sourced<bool>,
    /// What the opponents' weapon-use odds are multiplied by.
    pub ai_weapon_use: Sourced<f32>,
    /// Whether the pre-race flyby plays, and the track description with it. The original skips
    /// the flyby as soon as it has started and shows no description. The Now Playing caption is
    /// part of the race HUD, which a demo hides whole.
    pub intro: Sourced<bool>,
    /// Whether hits shake the camera.
    pub camera_shake: Sourced<bool>,
    /// Whether the race writes statistics to the player's profile.
    pub records_profile: Sourced<bool>,
}

impl Rules {
    /// Pulse PSP's, all measured.
    pub const PULSE: Self = Self {
        class: Sourced::measured("Flash"),
        craft: Sourced::measured(8),
        laps: Sourced::measured(3),
        ends_on_laps: Sourced::measured(false),
        weapons: Sourced::measured(true),
        ai_weapon_use: Sourced::measured(2.0),
        intro: Sourced::measured(false),
        camera_shake: Sourced::measured(false),
        records_profile: Sourced::measured(false),
    };

    /// `from`'s rules, not re-measured on this title.
    #[must_use]
    pub const fn inherited(from: &'static str) -> Self {
        Self {
            class: Sourced::inherited("Flash", from),
            craft: Sourced::inherited(8, from),
            laps: Sourced::inherited(3, from),
            ends_on_laps: Sourced::inherited(false, from),
            weapons: Sourced::inherited(true, from),
            ai_weapon_use: Sourced::inherited(2.0, from),
            intro: Sourced::inherited(false, from),
            camera_shake: Sourced::inherited(false, from),
            records_profile: Sourced::inherited(false, from),
        }
    }
}

/// The front-end screen a demo shows over its race.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Screen {
    /// The XML entry that authors it, by name.
    pub entry: &'static str,
    /// The state in that file.
    pub state: &'static str,
}

/// Which camera a demo watches its race through.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Camera {
    /// The title's usual race camera, on the craft the camera slot follows.
    Chase,
    /// The spectator director that runs after the player's finish: a subject craft, trackside
    /// stations and cuts between them (`oag_raceplay::finish_camera`). Pulse PSP, measured.
    Spectator,
}

/// One title's attract demo.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Demo {
    /// Seconds without a press on the main menu before a demo launches.
    pub idle_seconds: Sourced<u32>,
    /// Seconds a demo runs before it is over.
    pub duration_seconds: Sourced<u32>,
    /// What ends a demo early.
    pub exit: Sourced<Exit>,
    /// Where a demo that ran its full length goes when the menu launched it. A run started with
    /// `--demo` chains whatever this says.
    pub after_end: Sourced<AfterEnd>,
    /// How the race is picked.
    pub selection: Sourced<Selection>,
    /// The file the title authors its demo in, by entry name, when it does: its timeouts replace
    /// [`Self::idle_seconds`] and [`Self::duration_seconds`] and its circuit list is the cycle
    /// (`oag_tables::demo_config`). 2048 and Omega ship one; Pulse and HD do not.
    pub config: Option<&'static str>,
    /// How the race is set up.
    pub rules: Rules,
    /// The screen drawn over the race, when the title authors one.
    pub screen: Option<Sourced<Screen>>,
    /// Which camera the race is watched through.
    pub camera: Sourced<Camera>,
}

impl Demo {
    /// Pulse's, all of it measured: the 60 s idle test, the 120 s run, any button ending it, the next
    /// demo after it and the circuit and team law.
    pub const PULSE: Self = Self {
        idle_seconds: Sourced::measured(60),
        duration_seconds: Sourced::measured(120),
        exit: Sourced::measured(Exit::AnyInput),
        after_end: Sourced::measured(AfterEnd::NextDemo),
        selection: Sourced::measured(Selection::AnyCircuitMultiplayerTeam),
        config: None,
        rules: Rules::PULSE,
        screen: Some(Sourced::measured(Screen {
            entry: "Data\\Plugins\\PI001\\GUI\\Demo_Definition.xml",
            state: "Demo InGame",
        })),
        camera: Sourced::measured(Camera::Spectator),
    };

    /// A title that takes the two durations from `from` (Pulse's, by way of HD where `from` is
    /// HD) and chooses the rest, with the exit as `exit`.
    #[must_use]
    pub const fn inheriting(from: &'static str, exit: Sourced<Exit>) -> Self {
        Self {
            idle_seconds: Sourced::inherited(60, from),
            duration_seconds: Sourced::inherited(120, from),
            exit,
            after_end: Sourced::inherited(AfterEnd::NextDemo, from),
            selection: Sourced::inherited(Selection::AnyCircuitMultiplayerTeam, from),
            config: None,
            rules: Rules::inherited(from),
            screen: None,
            camera: Sourced::inherited(Camera::Spectator, from),
        }
    }

    /// Fixed-timestep ticks (60 Hz) the main menu waits.
    #[must_use]
    pub const fn idle_ticks(&self) -> u32 {
        self.idle_seconds.value * TICKS_PER_SECOND
    }

    /// Fixed-timestep ticks (60 Hz) a demo runs.
    #[must_use]
    pub const fn duration_ticks(&self) -> u32 {
        self.duration_seconds.value * TICKS_PER_SECOND
    }
}

/// The simulation's fixed rate (`oag_core::tick`); this crate depends on nothing, so it is spelled
/// out.
const TICKS_PER_SECOND: u32 = 60;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pulse_waits_a_minute_and_runs_two() {
        assert_eq!(Demo::PULSE.idle_ticks(), 3600);
        assert_eq!(Demo::PULSE.duration_ticks(), 7200);
    }
}
