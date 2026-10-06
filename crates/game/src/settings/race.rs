//! The `[race]` and `[remix]` sections: what the RACE and RACE REMIX pages
//! last chose.
//!
//! Its own module rather than two more blocks in [`crate::settings`] for the
//! same reason [`crate::settings::Controls`] has one: the pair is a
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
    /// The `WEAPONS` row's pick, as the disc's own `Weapons` list spells it
    /// (`"On"` or `"Off"`), or empty for "untouched", which is the list's
    /// authored default (`On`). Read only in a single race, the one mode whose
    /// row is editable; see [`Self::weapons_override`].
    #[serde(default)]
    pub weapons: String,
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
            track: default_track(),
            variant: String::new(),
            variant_chosen: false,
            skin: String::new(),
            kill_target: String::new(),
            weapons: String::new(),
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

/// What the RACE REMIX page last chose.
///
/// The same shape [`Race`] is, and stores the same *kind* of thing - archive
/// path components, not display names - for the same reason. Separate fields
/// rather than reusing [`Race::track`]/[`Race::team`]: a remix's TRACK/TEAM
/// rows are scoped to whichever title `track_title`/`craft_title` currently
/// name, which is not always the title this process booted from, so sharing
/// a setting with the ordinary RACE page would let the two pages clobber
/// each other's last pick.
///
/// `track_title`/`craft_title` store a title's own [`oag_title::Title::name`],
/// there being no shorter machine id since a title package is chosen once at
/// boot and never carries one. Two pressings of the same title (a US and an
/// EU disc) are one row on the picker, so this cannot name *which* pressing;
/// see [`crate::launcher::distinct_titles`].
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Remix {
    /// Which title the track loads from.
    ///
    /// Empty only until the menus first open: `session::remix_menu::settle`
    /// brings every field of this section into step with the list its row is
    /// given, so what is stored here and what the row shows agree from the
    /// first frame rather than after the player has nudged each one.
    #[serde(default)]
    pub track_title: String,
    /// Which circuit, by the id `track_title`'s own plugin definition gives
    /// it - [`Race::track`]'s rule, scoped to a different title.
    #[serde(default)]
    pub track: String,
    /// Which title the craft, HUD and grid roster load from, settled on the
    /// same terms as [`Self::track_title`].
    #[serde(default)]
    pub craft_title: String,
    /// Which team, by the id `craft_title`'s own plugin definition gives it -
    /// [`Race::team`]'s rule, scoped to a different title.
    #[serde(default)]
    pub team: String,
    /// Which variant of `team`, as a plain digit or the raw suffix its own
    /// directory uses (`"1"`, `"_c1"`), or empty when `team` offers none -
    /// [`Race::variant`]'s rule, scoped to a different title.
    ///
    /// Empty is not "the first variant"; it is "this team has no second
    /// directory at all", which is every team but Wipeout 2048's five and
    /// Wipeout HD/Fury's twelve - the latter reachable here through
    /// [`oag_title::RaceDefaults::guest_roster`] as well, since 2048 reships
    /// them.
    #[serde(default)]
    pub variant: String,
}
