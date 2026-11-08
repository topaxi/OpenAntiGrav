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
    /// A row of the campaign picker (`super::Definition::add_campaign_picker`):
    /// opens a mod's campaign, which the row names by its id
    /// ([`super::CUSTOM_CAMPAIGN_ROW`] and the campaign's key). Never written in
    /// a menu file: the picker exists only when a mod adds a campaign.
    OpenCustomCampaign,
    /// The MULTIPLAYER page's LOCAL MULTIPLAYER row: starts the multiplayer flow, which seats
    /// the people ("press a button to join"), lays out the windows and goes on to the RACE
    /// page and Track and Ship Select - see `oag_ui_screens::join` and
    /// `crate::main::session::multiplayer` in `oag-game`.
    OpenMultiplayerLocal,
    /// The LAN page's HOST GAME row: the RACE page, Track Select and Ship Select, then a lobby
    /// that waits for the others - `crate::main::session::lan` in `oag-game`.
    HostLan,
    /// The LAN page's JOIN GAME row: the list of hosts discovery finds, and a typed address
    /// for the ones it does not.
    JoinLan,
    /// The MULTIPLAYER page's ONLINE row: asks for the lobby's address, hash and a name,
    /// connects, and opens the ONLINE page - `crate::main::session::online` in `oag-game`.
    OpenOnline,
    /// The ONLINE page's CREATE GAME row: the RACE page, Track Select and Ship Select, then
    /// a room on the lobby.
    OnlineCreate,
    /// The ONLINE page's JOIN GAME row: Ship Select, then the lobby's list of rooms.
    OnlineJoin,
    /// The OPTIONS page's PLAYER NAME row: opens the name entry on the stored name
    /// (`crate::main::session::player_name` in `oag-game`).
    EditPlayerName,
    /// The OPTIONS page's PLAYER TAG row, on titles that use a tag.
    EditPlayerTag,
    /// The pause menu's PHOTO MODE row: resumes the parked race into the one photo mode
    /// (`crate::main::session::photo` in `oag-game`). The row is on the page only while a
    /// race with one view is parked ([`super::Menu::offer`]).
    PhotoMode,
    /// The pause menu's RESTART RACE row: the parked race again from the grid, on the options
    /// it ran with (`crate::main::session::pause_menu` in `oag-game`).
    RestartRace,
    /// The pause menu's QUIT RACE row: leaves the parked race for the front end, or for the
    /// campaign cell it was launched from.
    QuitRace,
    /// The REPLAYS page's WATCH row: opens the replay the REPLAY row is on in the viewer
    /// (`crate::main::session::replays` in `oag-game`).
    WatchReplay,
    /// The REPLAYS page's KEEP row: copies an automatic replay into the player's own folder,
    /// where the ring never reaches it.
    KeepReplay,
    /// The REPLAYS page's DELETE row: removes the replay the REPLAY row is on.
    DeleteReplay,
}

impl Action {
    /// The spelling used in the definition file.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::LaunchRace => "launch_race",
            Self::Quit => "quit",
            Self::SavePilot => "save_pilot",
            Self::NewPilot => "new_pilot",
            Self::RenamePilot => "rename_pilot",
            Self::DeletePilot => "delete_pilot",
            Self::OpenCampaign => "open_campaign",
            Self::OpenCustomCampaign => "open_custom_campaign",
            Self::OpenMultiplayerLocal => "multiplayer_local",
            Self::HostLan => "lan_host",
            Self::JoinLan => "lan_join",
            Self::OpenOnline => "online_open",
            Self::OnlineCreate => "online_create",
            Self::OnlineJoin => "online_join",
            Self::EditPlayerName => "edit_player_name",
            Self::EditPlayerTag => "edit_player_tag",
            Self::PhotoMode => "photo_mode",
            Self::RestartRace => "restart_race",
            Self::QuitRace => "quit_race",
            Self::WatchReplay => "watch_replay",
            Self::KeepReplay => "keep_replay",
            Self::DeleteReplay => "delete_replay",
        }
    }

    /// The page the game pushes itself once this action has done its work, which the
    /// definition's reachability check counts as linked from the row: the ONLINE page opens
    /// when the lobby has welcomed this machine.
    #[must_use]
    pub fn opens_page(self) -> Option<&'static str> {
        match self {
            Self::OpenOnline => Some("online"),
            _ => None,
        }
    }

    /// Parses a definition file's spelling.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "launch_race" => Some(Self::LaunchRace),
            "quit" => Some(Self::Quit),
            "save_pilot" => Some(Self::SavePilot),
            "new_pilot" => Some(Self::NewPilot),
            "rename_pilot" => Some(Self::RenamePilot),
            "delete_pilot" => Some(Self::DeletePilot),
            "open_campaign" => Some(Self::OpenCampaign),
            "open_custom_campaign" => Some(Self::OpenCustomCampaign),
            "multiplayer_local" => Some(Self::OpenMultiplayerLocal),
            "lan_host" => Some(Self::HostLan),
            "lan_join" => Some(Self::JoinLan),
            "online_open" => Some(Self::OpenOnline),
            "online_create" => Some(Self::OnlineCreate),
            "online_join" => Some(Self::OnlineJoin),
            "edit_player_name" => Some(Self::EditPlayerName),
            "edit_player_tag" => Some(Self::EditPlayerTag),
            "photo_mode" => Some(Self::PhotoMode),
            "restart_race" => Some(Self::RestartRace),
            "quit_race" => Some(Self::QuitRace),
            "watch_replay" => Some(Self::WatchReplay),
            "keep_replay" => Some(Self::KeepReplay),
            "delete_replay" => Some(Self::DeleteReplay),
            _ => None,
        }
    }

    /// Every action, for error messages and for the integrity check.
    #[must_use]
    pub fn all() -> [Self; 22] {
        [
            Self::LaunchRace,
            Self::Quit,
            Self::SavePilot,
            Self::NewPilot,
            Self::RenamePilot,
            Self::DeletePilot,
            Self::OpenCampaign,
            Self::OpenCustomCampaign,
            Self::OpenMultiplayerLocal,
            Self::HostLan,
            Self::JoinLan,
            Self::OpenOnline,
            Self::OnlineCreate,
            Self::OnlineJoin,
            Self::EditPlayerName,
            Self::EditPlayerTag,
            Self::PhotoMode,
            Self::RestartRace,
            Self::QuitRace,
            Self::WatchReplay,
            Self::KeepReplay,
            Self::DeleteReplay,
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
    /// The window sizes this screen's own shape suggests, plus the saved one if it is not among
    /// them. Supplied because the list depends on the monitor the window is on.
    WindowSizes,
    /// `native` and smaller sizes in the screen's own shape; supplied for the same reason as
    /// [`Self::WindowSizes`].
    FullscreenResolutions,
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
    /// The `HUD Style` skins this title's disc lists (`HD`, `2097`, `WIP3OUT`), valued and
    /// shown by the disc's own entry strings (`oag_title::HudLayouts::skins`). Supplied for
    /// the reason [`Self::FrontEndStyles`] is; a title that lists none has the row dropped
    /// (`Definition::drop_unavailable_hud_style`).
    HudStyles,
    /// The values of the title's `Safe Area Setting` slider (`oag_title::hud::SafeArea`), in
    /// percent, lowest first: HD's own 80 to 100 in steps of 2, and the shared slider of the
    /// titles whose disc authors none. Supplied rather than spelled so the row's range is the
    /// title's data.
    SafeAreas,
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
    /// A `speed_classes_union` row unions them instead - see [`Self::SpeedClassesUnion`].
    SpeedClasses,
    /// [`Self::SpeedClasses`]' sibling for a page that mixes titles: every speed class any
    /// title whose source this machine can currently open authors, as one
    /// ladder.
    ///
    /// A grid that mixes titles is not restricted to one title's rungs, so
    /// this row unions them - and unions only what is actually *available*,
    /// the same way [`Self::Titles`] does. A machine holding one disc offers
    /// that disc's ladder; a class whose title is not on this machine is not
    /// offered at all, rather than shown and then unable to load.
    SpeedClassesUnion,
    /// The AI difficulties: the four tiers, plus a `none` option the row
    /// is pinned to when the chosen class has no controller
    /// (`oag_ai::coverage`), labelled with the reason.
    AiDifficulties,
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
    /// Resupplied whenever the `PILOT` or `AXIS` row moves, the same idiom a
    /// `tracks_of` row follows for its title - a menu whose PILOT row
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
    /// Every mod this run has, the built-in one first: its name as the label, its id as the
    /// value, and a mod that did not load listed dimmed with its error as the reason
    /// (`oag_game::mods::Listing`). The row moves the detail lines and the ENABLED row
    /// under it; nothing stores it.
    Mods,
    /// The MODS page's ENABLED row for the mod the MOD row is on: `on` and `off`, or one
    /// dimmed value with the reason for a mod that cannot be switched (the built-in, a
    /// candidate that never opened). Re-supplied each time the MOD row moves.
    ModSwitch,
    /// A template row that expands to one row per music source
    /// ([`super::Definition::expand_rows`]; `oag_sound::pool`). Never supplied as a list.
    MusicPool,
    /// The recorded races under the data directory, `saved/` then `auto/`, newest first: the
    /// REPLAYS page's REPLAY row, whose detail lines are the file's (`oag_game::replays`).
    Replays,
}

impl ValueSource {
    /// Whether a row on this source is left out of the menu while the source
    /// offers fewer than two values: a choice the player's discs or the booted
    /// title leave empty (or with one value, which is no choice). A source
    /// that is not listed keeps its row, drawn with nothing to step.
    #[must_use]
    pub fn hidden_unless_chooseable(self) -> bool {
        matches!(self, Self::MusicSources | Self::RaceVariant)
    }

    /// The spelling used in the definition file.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Languages => "languages",
            Self::Tracks => "tracks",
            Self::Monitors => "monitors",
            Self::WindowSizes => "window_sizes",
            Self::FullscreenResolutions => "fullscreen_resolutions",
            Self::Renderers => "renderers",
            Self::RaceModes => "race_modes",
            Self::MusicSources => "music_sources",
            Self::FrontEndStyles => "front_end_styles",
            Self::HudStyles => "hud_styles",
            Self::SafeAreas => "safe_areas",
            Self::Teams => "teams",
            Self::Titles => "titles",
            Self::CraftTitles => "craft_titles",
            Self::SpeedClasses => "speed_classes",
            Self::SpeedClassesUnion => "speed_classes_union",
            Self::AiDifficulties => "ai_difficulties",
            Self::RaceVariant => "race_variant",
            Self::Pilots => "pilots",
            Self::PilotAxes => "pilot_axes",
            Self::PilotAxisLow => "pilot_axis_low",
            Self::PilotAxisHigh => "pilot_axis_high",
            Self::ScreenFilters => "screen_filters",
            Self::KillTargets => "kill_targets",
            Self::Weapons => "weapons",
            Self::Mods => "mods",
            Self::ModSwitch => "mod_switch",
            Self::MusicPool => "music_pool",
            Self::Replays => "replays",
        }
    }

    /// Parses a definition file's spelling.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "languages" => Some(Self::Languages),
            "tracks" => Some(Self::Tracks),
            "monitors" => Some(Self::Monitors),
            "window_sizes" => Some(Self::WindowSizes),
            "fullscreen_resolutions" => Some(Self::FullscreenResolutions),
            "renderers" => Some(Self::Renderers),
            "race_modes" => Some(Self::RaceModes),
            "music_sources" => Some(Self::MusicSources),
            "front_end_styles" => Some(Self::FrontEndStyles),
            "hud_styles" => Some(Self::HudStyles),
            "safe_areas" => Some(Self::SafeAreas),
            "teams" => Some(Self::Teams),
            "titles" => Some(Self::Titles),
            "craft_titles" => Some(Self::CraftTitles),
            "speed_classes" => Some(Self::SpeedClasses),
            "race_variant" => Some(Self::RaceVariant),
            "speed_classes_union" => Some(Self::SpeedClassesUnion),
            "ai_difficulties" => Some(Self::AiDifficulties),
            "pilots" => Some(Self::Pilots),
            "pilot_axes" => Some(Self::PilotAxes),
            "pilot_axis_low" => Some(Self::PilotAxisLow),
            "pilot_axis_high" => Some(Self::PilotAxisHigh),
            "screen_filters" => Some(Self::ScreenFilters),
            "kill_targets" => Some(Self::KillTargets),
            "weapons" => Some(Self::Weapons),
            "mods" => Some(Self::Mods),
            "mod_switch" => Some(Self::ModSwitch),
            "music_pool" => Some(Self::MusicPool),
            "replays" => Some(Self::Replays),
            _ => None,
        }
    }
}

/// The closed set of lists a `values_from` table may ask for, with an optional argument:
/// `values_from = { query = "tracks_of", of = "<setting key>" }` (ADR-0082).
///
/// `of` names the setting whose current value is the argument; empty means the booted title.
/// A mod cannot define a query, only choose one, so a typo is a load error and a mod cannot
/// invent behaviour for a list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Query {
    /// Every title this machine can open a source for.
    TitlesAvailable,
    /// The circuits of the title `of` names, or the custom tracks when it names CUSTOM.
    TracksOf,
    /// The teams of the title `of` names, as a craft title offers them (ADR-0035's guest
    /// roster folded in), narrowed to the ones that author the `class` setting's class.
    TeamsOf,
    /// The craft variants of the title `of` names, for the team its `team` setting holds
    /// (the title's first team without one).
    VariantsOf,
    /// The speed classes of every available title, merged.
    SpeedClassesUnion,
    /// The AI difficulties.
    AiDifficulties,
    /// The custom circuits the player holds.
    CustomTracks,
}

impl Query {
    /// Every query, in documentation order.
    pub const ALL: [Self; 7] = [
        Self::TitlesAvailable,
        Self::TracksOf,
        Self::TeamsOf,
        Self::VariantsOf,
        Self::SpeedClassesUnion,
        Self::AiDifficulties,
        Self::CustomTracks,
    ];

    /// The spelling used in the definition file.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::TitlesAvailable => "titles_available",
            Self::TracksOf => "tracks_of",
            Self::TeamsOf => "teams_of",
            Self::VariantsOf => "variants_of",
            Self::SpeedClassesUnion => "speed_classes_union",
            Self::AiDifficulties => "ai_difficulties",
            Self::CustomTracks => "custom_tracks",
        }
    }

    /// Parses a definition file's spelling.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|query| query.name() == name)
    }

    /// Whether `of` means anything to this query. The others answer the same list
    /// wherever they are asked.
    #[must_use]
    pub fn takes_of(self) -> bool {
        matches!(self, Self::TracksOf | Self::TeamsOf | Self::VariantsOf)
    }

    /// The second setting a query with an `of` may read, by the key it is written under:
    /// `tracks_of` takes the `craft_title` a CUSTOM track must race with, `teams_of` the
    /// `class` its teams must author, `variants_of` the `team` whose variants it lists.
    #[must_use]
    pub fn companion(self) -> Option<&'static str> {
        match self {
            Self::TracksOf => Some("craft_title"),
            Self::TeamsOf => Some("class"),
            Self::VariantsOf => Some("team"),
            _ => None,
        }
    }

    /// The supply key a query with an empty `of` answers through: the lists the booted title
    /// already gets under their fixed names.
    #[must_use]
    pub fn booted_source(self) -> Option<ValueSource> {
        match self {
            Self::TitlesAvailable => Some(ValueSource::Titles),
            Self::TracksOf => Some(ValueSource::Tracks),
            Self::TeamsOf => Some(ValueSource::Teams),
            Self::VariantsOf => Some(ValueSource::RaceVariant),
            Self::SpeedClassesUnion => Some(ValueSource::SpeedClassesUnion),
            Self::AiDifficulties => Some(ValueSource::AiDifficulties),
            Self::CustomTracks => None,
        }
    }
}

/// A `values_from` table that names a query with an argument the fixed names cannot say.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryRef {
    /// Which list.
    pub query: Query,
    /// The setting key whose value is the argument; never empty here (an empty `of` is one of
    /// the fixed names' lists).
    pub of: String,
    /// The setting key [`Query::companion`] names, when the row gives one.
    pub companion: Option<String>,
}
