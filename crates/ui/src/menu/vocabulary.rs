//! The two closed sets a definition entry may name: [`Action`] and
//! [`ValueSource`]. Split out of [`super`] under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

/// Every action a menu entry is allowed to name.
///
/// A closed set, checked by [`super::Definition::check`], so a typo in the
/// asset is a load error rather than an entry that does nothing when
/// pressed. Adding an action means adding it here and handling it in the
/// composition root, which is the point: the compiler and the check
/// together make "defined but not wired up" impossible to ship.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Action {
    /// Leave the menus and start a race with whatever the race pages hold.
    LaunchRace,
    /// Leave the menus and start a race whose track and craft may name
    /// different titles - whatever the `remix` page holds. Its own action
    /// rather than a second meaning for `LaunchRace`, since the two pages
    /// store their picks under different settings keys (`remix.*` against
    /// `race.*`) and a shared action would have to guess which page fired it.
    LaunchRemix,
    /// Close the game.
    Quit,
    /// Writes the pilot editor's current axis edit to disk. See
    /// `crate::pilots::set_axis`.
    SavePilot,
    /// Creates a new pilot file from whichever one is currently selected on
    /// the pilot editor's own list, auto-named (`pilot-1`, `pilot-2`, ...).
    ///
    /// **Kept alongside [`Self::RenamePilot`] rather than replaced by it.**
    /// Auto-naming was the thing that let create-from-template ship before
    /// there was any text entry at all, and it is still the faster of the two
    /// for a player copying a pilot to tinker with: one press against a name
    /// typed on a grid. See `crate::pilots::template`.
    NewPilot,
    /// Opens the on-screen keyboard on the selected pilot's name and, on
    /// accept, moves its file.
    ///
    /// **Only a pilot with a file of its own can be renamed.** An untouched
    /// built-in lives in the binary and has nothing on disk to move - see
    /// `crate::pilots::rename_pilot`. Gated in Rust off `Entry::from_file`
    /// and not by a `disabled_by` on the four built-in *names*: the moment a
    /// player saves an edit to `aggressive` a file exists, and renaming that
    /// file is perfectly legitimate.
    RenamePilot,
    /// Asks, and then deletes the selected pilot's file.
    ///
    /// Behind a `oag_ui_screens::prompt::Confirm` because it destroys a file, and
    /// because for a file named after a built-in "delete" is not what
    /// happens: the file was *replacing* the built-in, so removing it
    /// restores it. See `crate::pilots::is_built_in_name`.
    DeletePilot,
    /// Opens the Race Campaign's `Grid Selection` screen - see
    /// `oag_ui_screens::campaign` and `crate::main::session::campaign` in `oag-game`.
    ///
    /// **A stub past `Grid Selection`/`Cell Selection` themselves.** Neither
    /// screen launches a race here - see `oag_ui_screens::campaign`'s own module doc
    /// for why, and the `campaign` handover thread's "Next Steps".
    OpenCampaign,
}

impl Action {
    /// The spelling used in the definition file.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::LaunchRace => "launch_race",
            Self::LaunchRemix => "launch_remix",
            Self::Quit => "quit",
            Self::SavePilot => "save_pilot",
            Self::NewPilot => "new_pilot",
            Self::RenamePilot => "rename_pilot",
            Self::DeletePilot => "delete_pilot",
            Self::OpenCampaign => "open_campaign",
        }
    }

    /// Parses a definition file's spelling.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "launch_race" => Some(Self::LaunchRace),
            "launch_remix" => Some(Self::LaunchRemix),
            "quit" => Some(Self::Quit),
            "save_pilot" => Some(Self::SavePilot),
            "new_pilot" => Some(Self::NewPilot),
            "rename_pilot" => Some(Self::RenamePilot),
            "delete_pilot" => Some(Self::DeletePilot),
            "open_campaign" => Some(Self::OpenCampaign),
            _ => None,
        }
    }

    /// Every action, for error messages and for the integrity check.
    #[must_use]
    pub fn all() -> [Self; 8] {
        [
            Self::LaunchRace,
            Self::LaunchRemix,
            Self::Quit,
            Self::SavePilot,
            Self::NewPilot,
            Self::RenamePilot,
            Self::DeletePilot,
            Self::OpenCampaign,
        ]
    }
}

/// Where a `choice`'s values come from when the definition cannot name them.
///
/// A closed set for the same reason [`Action`] is: a typo has to be a load
/// error, not a row that is empty at runtime. The definition says *which* list
/// it wants and the composition root supplies it through [`super::Menu::supply`],
/// so this module still knows nothing about discs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueSource {
    /// The languages this source actually carries, which is a property of the
    /// disc: the USA PSP release ships English only, and a menu that offered
    /// French on it would be offering something that cannot be selected.
    Languages,
    /// The circuits this source offers, from its own plugin definition.
    ///
    /// Not a directory listing: `16_Track` and `32_Track` are two races and one
    /// folder. And the label is the localised name out of the string table, so
    /// what a player reads never appears in this repository. See
    /// `crate::catalogue`.
    Tracks,
    /// The screens this machine has, which is a property of the desk rather
    /// than of the disc - the first source here that is.
    ///
    /// Supplied for the same reason the other two are: a definition file cannot
    /// know them, and a row offering a monitor that is not plugged in would be
    /// offering something that cannot be selected. `default` is always first,
    /// so there is a way back from a screen that has since been unplugged.
    Monitors,
    /// The race modes, labelled from the disc's own string table.
    ///
    /// The list itself is fixed - it is `oag_race::Mode::ALL` - so unlike the
    /// other sources this one is not supplied because the set is unknown. It is
    /// supplied because the *labels* are: what a player reads is the front end's
    /// own event text off their disc, in their own language, and so it cannot be
    /// spelled in a definition file that ships in this repository.
    RaceModes,
    /// The adapters this machine can present with, which is the other property
    /// of the desk rather than of the disc.
    ///
    /// Supplied and not spelled for a reason this list makes especially plain:
    /// nothing that could be written in a definition file describes a GPU. What
    /// is on it depends on the card, the driver and whether a *software* driver
    /// is installed at all - see `crate::adapter`. `default` is always first,
    /// so there is a way back from an adapter that has since been uninstalled.
    Renderers,
    /// Which release's soundtrack can be played, which is a property of what
    /// discs the *player owns* rather than of the one that booted.
    ///
    /// The first source whose answer is routinely **nothing**: choosing between
    /// two encodes needs both discs, and most people have one. Supplied empty
    /// there, which draws the row unusable rather than offering a swap that
    /// cannot happen - see [`super::Menu::supply`]. Hiding it instead would leave
    /// the AUDIO page with no way to say that the setting exists and this
    /// machine cannot reach it, which is the same argument
    /// [`super::Menu::is_disabled`] makes for greying a row rather than
    /// removing it.
    MusicSources,
    /// The front-end stylings this source ships art for.
    ///
    /// Supplied rather than spelled for the reason [`Self::Tracks`] is: the
    /// values are the *disc's* own - Wipeout HD's `OPT_FE_STYLE` offers `HD`
    /// and `FURY` and ships every loading-screen illustration twice - and no
    /// other title in hand offers more than one. A source with a single styling
    /// supplies one row, which draws the setting as the fact it is rather than
    /// as a choice that does nothing.
    FrontEndStyles,
    /// The teams this source offers, from its own plugin definition and from
    /// any `crate::dlc` mounted behind it.
    ///
    /// Supplied rather than spelled for the reason [`Self::Tracks`] is, and one
    /// more: the roster is **not fixed**. A player who owns a pack has teams a
    /// definition file in this repository could not have listed, and which of
    /// them they own is not knowable until the archives are open.
    ///
    /// The value stored is the team **id** - the folder under `Data\Ships\` -
    /// and the label is that id looked up in the string table, which is not the
    /// same string: the Mirage pack's team is `Mantis`. See
    /// `crate::catalogue`.
    Teams,
    /// Every title this machine can currently open a source for, deduplicated
    /// by title. `TRACK TITLE`'s own axis - see
    /// `crate::launcher::distinct_titles`.
    Titles,
    /// [`Self::Titles`]' sibling for `CRAFT TITLE`, not the same list: it
    /// also offers "Wipeout HD" when this machine has Wipeout 2048 but no
    /// real HD source, since 2048 reships HD/Fury's own roster under a tree
    /// of its own - see `oag_title::RaceDefaults::guest_roster` and
    /// `session::menus::Session::craft_title_choices`.
    CraftTitles,
    /// The circuits the *currently chosen* `TRACK TITLE` offers - unlike
    /// [`Self::Tracks`], resupplied every time `remix.track_title` changes
    /// rather than fixed for the session.
    RemixTracks,
    /// [`Self::RemixTracks`]' sibling for `CRAFT TITLE`'s roster.
    RemixTeams,
    /// Which of the *booted* title's own teams `race.team` names carries more
    /// than one selectable directory - Wipeout 2048's own five and Wipeout
    /// HD/Fury's twelve, both today. See
    /// [`oag_title::RaceDefaults::team_variants`].
    ///
    /// Supplied empty for a team with nothing to pick, the same idiom
    /// [`Self::MusicSources`] uses - a row with nothing to offer draws
    /// unusable rather than lying about having a choice.
    RaceVariant,
    /// The speed classes the **booted** title's own data authors, filtered to
    /// the ones this build can actually put a ship on.
    ///
    /// Supplied rather than spelled because the ladder is a property of the
    /// release: Wipeout Pure's per-team `handlingstats.xml` authors five
    /// `<Class>` rungs and Wipeout Pulse's authors four, each read off its own
    /// disc. A literal list in the definition file would have to be either
    /// wrong for Pure or silently reused as if measured - the trap
    /// `oag_title::SpeedClasses` was added to close.
    ///
    /// The ordinary RACE page offers **this title's** ladder and no other's.
    /// RACE REMIX unions them instead - see [`Self::RemixSpeedClasses`].
    SpeedClasses,
    /// [`Self::SpeedClasses`]' sibling for RACE REMIX: every speed class any
    /// title whose source this machine can currently open authors, as one
    /// ladder.
    ///
    /// A grid that mixes titles is not restricted to one title's rungs, so
    /// this row unions them - and unions only what is actually *available*,
    /// the same way [`Self::Titles`] does. A machine holding one disc offers
    /// that disc's ladder; a class whose title is not on this machine is not
    /// offered at all, rather than shown and then unable to load.
    RemixSpeedClasses,
    /// [`Self::RaceVariant`]'s sibling for RACE REMIX's craft-side `TEAM` -
    /// scoped to whichever title `CRAFT TITLE` picked rather than to the
    /// title this process booted from.
    RemixVariant,
    /// Every pilot the in-game editor's roster currently holds - the four
    /// built-ins plus whatever `crate::pilots::directory` has, in
    /// `crate::pilots::Roster`'s own order.
    ///
    /// Supplied rather than spelled for the reason [`Self::Teams`] is: the
    /// roster is not fixed, and how many files a player has authored is not
    /// knowable here.
    Pilots,
    /// Every axis `crate::pilots::AXES` names, in its own draw order.
    ///
    /// The list itself is fixed, but it is supplied rather than spelled for
    /// the same reason [`Self::RaceModes`] is a source despite being a fixed
    /// list too: one list, walked in one place, is what keeps a landed axis
    /// from being wired into the loader and forgotten here.
    PilotAxes,
    /// The low end of whichever pilot and axis [`Self::Pilots`] and
    /// [`Self::PilotAxes`] currently hold, as a set of selectable numbers
    /// within `crate::pilots::limit`'s range for that axis.
    ///
    /// Resupplied whenever the `PILOT` or `AXIS` row moves, the same idiom
    /// [`Self::RemixTracks`] uses for `TRACK TITLE` - a menu whose PILOT row
    /// just changed should show that pilot's own bounds on the very next
    /// frame, not the previous one's.
    PilotAxisLow,
    /// [`Self::PilotAxisLow`]'s sibling for the high end of the range.
    PilotAxisHigh,
    /// Every screen filter this machine can offer: `off`, the built-ins, and
    /// whatever `.wgsl` files the player's own `shaders/` directory holds -
    /// supplied for the reason [`Self::Renderers`] is, nothing writable in a
    /// definition file could name a file on someone else's disk. See
    /// `oag_game::screen`.
    ScreenFilters,
    /// The kill targets the race box's `KILLS` row offers, off the disc's own
    /// `Eliminations` list (`oag_game::boot::RaceSetup::kill_targets`). Empty only
    /// if the list will not read: a title that names no such list has the row
    /// dropped instead (`Definition::drop_rows_picked_on_screen`).
    KillTargets,
    /// The two states of the race box's `WEAPONS` row, off the disc's own
    /// `Weapons` list: stored as its `On`/`Off`, shown as its `FE_ON`/`FE_OFF`
    /// strings (`oag_game::boot::RaceSetup::weapon_choices`). Empty only if the
    /// list will not read; a title with no such list has the row dropped.
    Weapons,
}

impl ValueSource {
    /// The spelling used in the definition file.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Languages => "languages",
            Self::Tracks => "tracks",
            Self::Monitors => "monitors",
            Self::Renderers => "renderers",
            Self::RaceModes => "race_modes",
            Self::MusicSources => "music_sources",
            Self::FrontEndStyles => "front_end_styles",
            Self::Teams => "teams",
            Self::Titles => "titles",
            Self::CraftTitles => "craft_titles",
            Self::RemixTracks => "remix_tracks",
            Self::RemixTeams => "remix_teams",
            Self::SpeedClasses => "speed_classes",
            Self::RemixSpeedClasses => "remix_speed_classes",
            Self::RaceVariant => "race_variant",
            Self::RemixVariant => "remix_variant",
            Self::Pilots => "pilots",
            Self::PilotAxes => "pilot_axes",
            Self::PilotAxisLow => "pilot_axis_low",
            Self::PilotAxisHigh => "pilot_axis_high",
            Self::ScreenFilters => "screen_filters",
            Self::KillTargets => "kill_targets",
            Self::Weapons => "weapons",
        }
    }

    /// Parses a definition file's spelling.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "languages" => Some(Self::Languages),
            "tracks" => Some(Self::Tracks),
            "monitors" => Some(Self::Monitors),
            "renderers" => Some(Self::Renderers),
            "race_modes" => Some(Self::RaceModes),
            "music_sources" => Some(Self::MusicSources),
            "front_end_styles" => Some(Self::FrontEndStyles),
            "teams" => Some(Self::Teams),
            "titles" => Some(Self::Titles),
            "craft_titles" => Some(Self::CraftTitles),
            "remix_tracks" => Some(Self::RemixTracks),
            "remix_teams" => Some(Self::RemixTeams),
            "speed_classes" => Some(Self::SpeedClasses),
            "remix_speed_classes" => Some(Self::RemixSpeedClasses),
            "race_variant" => Some(Self::RaceVariant),
            "remix_variant" => Some(Self::RemixVariant),
            "pilots" => Some(Self::Pilots),
            "pilot_axes" => Some(Self::PilotAxes),
            "pilot_axis_low" => Some(Self::PilotAxisLow),
            "pilot_axis_high" => Some(Self::PilotAxisHigh),
            "screen_filters" => Some(Self::ScreenFilters),
            "kill_targets" => Some(Self::KillTargets),
            "weapons" => Some(Self::Weapons),
            _ => None,
        }
    }
}
