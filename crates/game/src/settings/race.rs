//! The `[race]` section: what the RACE page last chose.
//!
//! Its own module rather than another block in [`crate::settings`] for the
//! same reason [`crate::settings::Controls`] has one: it is a
//! self-contained unit, here a menu-picked launch configuration rather than a
//! pilot preference.

use serde::{Deserialize, Serialize};

/// What the Race page of the menus last chose.
///
/// Persisted for the same reason `source.image` is: a player who always races
/// one team on one circuit should not have to say so twice. These are archive
/// path components and speed-class names, **not display names** - the words the
/// original shows a player for a circuit are content this project does not ship.
/// See `docs/architecture/adr/0006-no-copyrighted-content.md`.
///
/// The command line still wins for the run it is given on, and `--race` skips
/// the menus entirely, so these are what the menus set rather than a second
/// place to configure a race from.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Race {
    /// Which mode's rules run: `time_trial`, `speed_lap` or `zone`.
    ///
    /// One of `oag_race::Mode::ALL`'s tokens. A file carrying anything else
    /// leaves the mode at the default rather than failing the boot, the same way
    /// an unknown speed class does.
    #[serde(default = "default_mode")]
    pub mode: String,
    /// Speed class: `venom`, `flash`, `rapier` or `phantom`.
    #[serde(default = "default_class")]
    pub class: String,
    /// Team id, as the source's own plugin definition spells it.
    ///
    /// **Not `oag_pulse::race::TEAMS`**, which is Pulse's base eight and was what this
    /// said while a stand-in in `boot::load_teams` could still put that list in
    /// front of a player on any source. The roster is the disc's now, so what
    /// is storable here is whatever that disc declares - twelve on the PS2
    /// pressing and on Wipeout HD, nine on Pure.
    #[serde(default = "default_team")]
    pub team: String,
    /// Whether the player has ever picked a team: moved Ship Select's cursor
    /// or changed a TEAM row. Pure opens that screen on Feisar for a profile
    /// that has not - see [`Self::opening_team`] - and [`Self::team`] alone
    /// cannot say "chose the default team" from "never chose".
    #[serde(default)]
    pub team_chosen: bool,
    /// Which circuit, by the id its own plugin definition gives it.
    ///
    /// **A race, not a directory.** `16_Track` and `32_Track` are two entries
    /// and one folder, the second being the first driven the other way, so this
    /// is not a path component and must not be turned into one here - only the
    /// source can say which `.vex` an id loads. See [`oag_raceplay::catalogue`].
    #[serde(default = "default_track")]
    pub track: String,
    /// Which numbered variant of `team`, as a plain digit or the raw suffix
    /// its own directory uses (`"1"`, `"_c1"`), or empty when `team` offers
    /// none. See [`oag_title::RaceDefaults::team_variants`].
    ///
    /// Empty is not "the first variant"; it is "this team has no second
    /// directory at all", which is every team but Wipeout 2048's five and
    /// Wipeout HD/Fury's twelve.
    #[serde(default)]
    pub variant: String,
    /// Whether the player has ever picked a model on Ship Select. Wipeout
    /// HD/Fury opens that screen on `concept1` for a profile that has not -
    /// see [`Self::opening_variant`] - and an empty [`Self::variant`] alone
    /// cannot say "chose the classic hull" from "never chose".
    #[serde(default)]
    pub variant_chosen: bool,
    /// Which `PI_ModelSkin` of `team` the craft is painted in, by the name
    /// the definition declares (`Alternative`), or empty for the baseline
    /// paint. What Ship Select's livery row picks on a title whose teams
    /// declare skins - Pulse - and what a race's `--skin` reads. See
    /// `oag_livery::ship_skin`.
    #[serde(default)]
    pub skin: String,
    /// The Eliminator kill target the `KILLS` row holds, as the disc's own
    /// list spells it (`"5"` to `"25"`), or empty for "untouched", which races
    /// [`oag_race::Mode::ELIMINATOR_KILL_TARGET_DEFAULT`]. Read only in an
    /// Eliminator; see [`Self::eliminator_kill_target`].
    #[serde(default)]
    pub kill_target: String,
    /// The Eliminator score target the `TARGET` row holds, as the disc's slider steps spell it
    /// (`"200"` to `"600"` on HD), or empty for "untouched", which races the slider's minimum.
    /// Read only in an Eliminator; see [`Self::eliminator_score_target`].
    #[serde(default)]
    pub score_target: String,
    /// The `WEAPONS` row's pick, as the disc's own `Weapons` list spells it
    /// (`"On"` or `"Off"`), or empty for "untouched", which is the list's
    /// authored default (`On`). Read only in a single race, the one mode whose
    /// row is editable; see [`Self::weapons_override`].
    #[serde(default)]
    pub weapons: String,
    /// The windows the layout screen last chose, spelled as `--window-seats` spells them
    /// (`"3,1"`, `"4,2v"`), or empty for one window, which is a split screen in the one.
    /// What the screen starts from next time; another head count is dealt anew from it
    /// ([`oag_ui_screens::window_layout::Picker::new`]). **Chosen, not measured.**
    #[serde(default)]
    pub window_seats: String,
    /// The monitor of each further window of [`Self::window_seats`], by the name the platform
    /// gives it, window 2 first. A name this machine no longer has is its first monitor.
    #[serde(default)]
    pub window_monitors: Vec<String>,
    /// Who sat in each pane of [`Self::window_seats`] the last time the seat assignment screen
    /// went on, one device key per pane in pane order (`oag_game::split_screen::device_key`:
    /// `keyboard`, `keyboard 2`, a pad's name and its number among pads of that name). The next
    /// screen opens with each seated device back in its pane. **Chosen, not measured.**
    #[serde(default)]
    pub window_panes: Vec<String>,
}

/// Time trial: the mode the RACE page opens on, and the one the reference
/// captures under `data/traces/` were taken in.
fn default_mode() -> String {
    oag_race::Mode::TimeTrial.name().to_string()
}
fn default_class() -> String {
    "venom".to_string()
}
fn default_team() -> String {
    oag_raceplay::DEFAULT_TEAM.to_string()
}
/// The circuit the reference scenario is on, so the default run is the one
/// every capture under `data/traces/` was taken against.
fn default_track() -> String {
    "16_Track".to_string()
}

impl Default for Race {
    fn default() -> Self {
        Self {
            mode: default_mode(),
            class: default_class(),
            team: default_team(),
            team_chosen: false,
            track: default_track(),
            variant: String::new(),
            variant_chosen: false,
            skin: String::new(),
            kill_target: String::new(),
            score_target: String::new(),
            weapons: String::new(),
            window_seats: String::new(),
            window_monitors: Vec::new(),
            window_panes: Vec::new(),
        }
    }
}

impl Race {
    /// The weapons switch a launch in `mode` carries, or `None` to leave it to
    /// [`oag_race::Mode::weapons_enabled`].
    ///
    /// Only a single race honours the `WEAPONS` row: the original greys it in
    /// every other mode and forces its value there (Time Trial, Speed Lap and
    /// Zone `Off`, Eliminator `On` - `docs/formats/race-setup.md`), which is
    /// what `Mode::weapons_enabled` already says.
    #[must_use]
    pub fn weapons_override(&self, mode: oag_race::Mode) -> Option<bool> {
        if mode != oag_race::Mode::SingleRace {
            return None;
        }
        match self.weapons.as_str() {
            "On" => Some(true),
            "Off" => Some(false),
            _ => None,
        }
    }

    /// The kill target a launch in `mode` carries: the `KILLS` row's pick in an
    /// Eliminator, and `None` in every other mode (nothing reads it there) or
    /// while the row is untouched, which leaves the default to `Race::start`.
    #[must_use]
    pub fn eliminator_kill_target(&self, mode: oag_race::Mode) -> Option<u32> {
        (mode == oag_race::Mode::Eliminator)
            .then(|| self.kill_target.parse().ok())
            .flatten()
    }

    /// The score target a launch in `mode` carries: the `TARGET` row's pick in an Eliminator,
    /// and `None` in every other mode or while the row is untouched, which leaves the
    /// slider's minimum to `oag_raceplay::score_law`.
    #[must_use]
    pub fn eliminator_score_target(&self, mode: oag_race::Mode) -> Option<u32> {
        (mode == oag_race::Mode::Eliminator)
            .then(|| self.score_target.parse().ok())
            .flatten()
    }

    /// The team Ship Select opens on: the stored team, or the title's
    /// fresh-profile row ([`oag_title::RaceDefaults::fresh_team`]) while the
    /// player has not picked one. A stored team other than the unpicked
    /// default always wins, so a config written before [`Self::team_chosen`]
    /// existed keeps what it had.
    #[must_use]
    pub fn opening_team(&self, title: &oag_title::Title) -> &str {
        match title.race.fresh_team {
            Some(fresh) if !self.team_chosen && self.team == default_team() => fresh.team,
            _ => &self.team,
        }
    }

    /// Ship Select opened showing `shown`: stored as the team, unless it is the title's
    /// fresh-profile row standing in for a team never picked. That row is shown, not stored, so
    /// a profile that only looked at Pure's list still opens Pulse's on its own first row;
    /// [`Self::pick_team`] stores it once confirmed. Whether anything changed.
    pub fn open_team(&mut self, title: &oag_title::Title, shown: &str) -> bool {
        let fresh_row = !self.team_chosen
            && self.team == default_team()
            && title
                .race
                .fresh_team
                .is_some_and(|fresh| fresh.team == shown);
        if fresh_row || self.team == shown {
            return false;
        }
        self.team = shown.to_string();
        true
    }

    /// The player picked `team`: moved the cursor to it, confirmed it or chose it on a TEAM row.
    pub fn pick_team(&mut self, team: &str) {
        self.team = team.to_string();
        self.team_chosen = true;
    }

    /// The model Ship Select opens on: the stored variant, or on Wipeout
    /// HD/Fury the fresh-profile default (`oag_hd::race::FRESH_PROFILE_VARIANT`)
    /// while the player has not picked one. A non-empty stored variant always
    /// wins, so a config written before [`Self::variant_chosen`] existed keeps
    /// what it had.
    #[must_use]
    pub fn opening_variant(&self, title: &oag_title::Title) -> &str {
        match title.race.fresh_variant {
            Some(fresh) if self.variant.is_empty() && !self.variant_chosen => fresh.variant,
            _ => &self.variant,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_race::Mode;

    /// A Pure profile that has never picked a team opens Team Selection on Feisar,
    /// the definition's first row, where the original's fresh memory stick rests
    /// (`docs/ui/selection-screens.md`, "The row a fresh profile opens on"); Pulse
    /// keeps its stored default, Assegai, its own first row. A pick, or a stored
    /// team from before the flag existed, wins.
    #[test]
    fn pure_opens_team_selection_on_feisar_until_a_team_is_picked() {
        let mut race = Race::default();
        assert_eq!(race.opening_team(oag_pure::TITLE), "Feisar");
        assert_eq!(race.opening_team(oag_pulse::TITLE), "Assegai");

        race.team_chosen = true;
        assert_eq!(race.opening_team(oag_pure::TITLE), "Assegai");

        let stored = Race {
            team: "Qirex".to_string(),
            ..Race::default()
        };
        assert_eq!(stored.opening_team(oag_pure::TITLE), "Qirex");
    }

    /// Opening Pure's list on its fresh row does not store it: the profile is still unpicked, and
    /// Pulse's Ship Select still opens on Assegai. Confirming stores it as a pick, for both.
    #[test]
    fn a_fresh_row_only_shown_is_not_another_titles_team() {
        let mut race = Race::default();
        assert!(!race.open_team(oag_pure::TITLE, "Feisar"));
        assert_eq!(race.opening_team(oag_pure::TITLE), "Feisar");
        assert_eq!(race.opening_team(oag_pulse::TITLE), "Assegai");

        race.pick_team("Feisar");
        assert_eq!(race.opening_team(oag_pure::TITLE), "Feisar");
        assert_eq!(race.opening_team(oag_pulse::TITLE), "Feisar");
    }

    /// The TARGET row reaches a launch only in an Eliminator, and only once touched.
    #[test]
    fn the_target_pick_is_an_eliminator_launch_option_only() {
        let untouched = Race::default();
        assert_eq!(untouched.eliminator_score_target(Mode::Eliminator), None);
        let picked = Race {
            score_target: "400".to_string(),
            ..Race::default()
        };
        assert_eq!(picked.eliminator_score_target(Mode::Eliminator), Some(400));
        assert_eq!(picked.eliminator_score_target(Mode::SingleRace), None);
        assert_eq!(picked.eliminator_kill_target(Mode::Eliminator), None);
    }
}
