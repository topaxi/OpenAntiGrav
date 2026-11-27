//! What a race is asked to load and what comes back: [`Options`] in, [`SimSetup`], [`ViewSetup`]
//! and [`Loaded`] out, plus the two overrides a capture drives them with.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::*;

/// Whether the trail-hit burst plays on a title whose original never shows it
/// (`graphics.trail_hits`, off by default: an enhancement of this project's).
///
/// A lender is used only where the craft's title has no `WO_TRAIL_HITSHIP` of its own
/// (`oag_title::Title::effect_on(Trigger::TrailHitship)` is `None`: Pulse and Pure). 2048 and Omega
/// author it but their originals never fire it (`oag_title::Effects::trail_hits_native`), so they
/// play their own only with this on. HD plays its own whatever this holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum TrailHits {
    /// No burst but HD's own: nothing on Pulse and Pure, and 2048's and Omega's own are dropped.
    #[default]
    Off,
    /// Play the burst off `lender`, a source whose title authors it (found by the composition
    /// root); `None` when this machine has none, which the loader report says.
    On {
        /// The source to read the effect from.
        lender: Option<String>,
    },
}

/// Whether the wingtip contrails are laid on a title whose original has none
/// (`graphics.wingtip_contrails`, off by default: an enhancement of this project's).
///
/// A lender is used only where the craft's title carries no contrails of its own
/// (`oag_title::Title::wingtip` is `None`: Pulse and Pure); HD, 2048 and Omega lay theirs whatever
/// this holds.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum WingtipContrails {
    /// Only the titles that carry them.
    #[default]
    Off,
    /// Lay them off `lender`, a source whose title carries them (found by the composition root);
    /// `None` when this machine has none, which the loader report says.
    On {
        /// The source to read the config and the texture from.
        lender: Option<String>,
    },
}

/// What to load, and which of it to draw.
#[derive(Debug, Clone)]
pub struct Options {
    /// A disc image, or a directory extracted with `oag-unpack`.
    pub source: String,
    /// Where the craft, HUD and grid roster load from, or `None` for
    /// `source` - today's behaviour, unchanged.
    ///
    /// **The one thing a two-title race adds.** `source` still supplies the track
    /// and everything that goes with it - collision, environment, pads,
    /// weapon tuning; `craft_source`, when it names a different release,
    /// supplies the livery, HUD, exhaust/flare and boost plume instead. See
    /// [`oag_source::pair::SourcePair`].
    pub craft_source: Option<String>,
    /// Directories to look in for downloadable content (`oag_source::dlc`), mounted
    /// behind `source`'s own archives.
    ///
    /// Independent of which release `source` is, on purpose: the original tied
    /// a pack to its own territory's disc and this does not. See `docs/formats/dlc-pack.md`.
    pub dlc: Vec<PathBuf>,
    /// Archive entry name of the track's `.vex`, or `None` for the source's own.
    ///
    /// **`None` rather than a constant, because the two titles share no circuit
    /// directory.** `Data\Environments\16_Track\track.vex` resolves on no Pure
    /// pressing, so a default baked in here - which is what this field used to
    /// carry - meant every Pure race asked for a name that hashes to nothing and
    /// failed inside the archive with a message about a missing entry rather than
    /// about a missing circuit.
    ///
    /// Resolved by [`load`] once the source is open and its title known, from
    /// [`oag_title::RaceDefaults::track`], and the choice is reported. It cannot
    /// be resolved earlier: which title a source is comes from the serial in its
    /// own filesystem, which is read by opening it.
    pub track: Option<String>,
    /// A foreign (Unity) track to race instead of any circuit of the source
    /// (`crate::foreign`); the source still supplies craft, rules and physics.
    pub foreign_track: Option<PathBuf>,
    /// The simulation modules of the mods that are on for this race
    /// ([`crate::extensions`]); empty is a vanilla race. The composition root fills it.
    pub extensions: Vec<crate::extensions::Source>,
    /// Finds more of them on the load thread ([`crate::extensions::Finder`]); `None`
    /// looks for none.
    pub find_extensions: Option<crate::extensions::Finder>,
    /// The simulation modules a recording ran (`Header::mods`), when this load rebuilds one:
    /// the race runs exactly these, with their recorded picks, and not the others the mods
    /// that are on offer. `None` is an ordinary race; `Some` of nothing is a recording of a
    /// vanilla race. Set by [`Self::apply_replay`].
    pub recorded_mods: Option<Vec<oag_replay::header::ModUse>>,
    /// Opens the texture packs the player has on, on the load thread
    /// ([`crate::texture_pack::Finder`]); `None` draws the disc's own textures.
    pub find_texture_packs: Option<crate::texture_pack::Finder>,
    /// Mounts no player override directory around the source, only this project's own fallback
    /// layer: a race that must not depend on the player's files (the benchmark with its mods
    /// off, `docs/tools/in-game-benchmark.md`).
    pub no_overrides: bool,
    /// The ring the player's `graphics.boost_zoom` borrows, handed in by the composition
    /// root (the one place that may name HD's); `None` is off. Used only where
    /// `oag_title::RaceDefaults::borrows_zoom_ring`.
    pub boost_zoom: Option<&'static oag_title::ZoomRing>,
    /// The player's `graphics.trail_hits`: whether a craft flying in a rival's engine trail plays
    /// a title's trail-hit burst on a title that has none of its own (Pulse and Pure).
    /// Off by default. See [`TrailHits`].
    pub trail_hits: TrailHits,
    /// The player's `graphics.wingtip_contrails`: whether a title with none of its own lays HD's
    /// wingtip contrails after a speed pad. Off by default. See [`WingtipContrails`].
    pub wingtip_contrails: WingtipContrails,
    /// The player's `graphics.own_shield`: whether a title offered it (`oag_title::Looks::own_shield`:
    /// Pulse and Pure) draws our own HD/Fury-style shield in place of the disc's. Off by default.
    pub own_shield: bool,
    /// The player's `graphics.own_rockets`: whether a title offered it (`oag_title::Looks::own_rockets`:
    /// Pulse and Pure) draws our own rocket effect in place of the disc's flare and blasts. Off by
    /// default. See `crate::own_rockets`.
    pub own_rockets: bool,
    /// The law the circuit's classified water slots are drawn under: the player's
    /// `graphics.water`, already resolved against the discs this machine has
    /// (`oag_game::water_setting::resolve`), since this crate cannot see them.
    /// `original` by default. See `oag_mesh::mesh::rcs::build_scene_with`.
    pub water: oag_display::display::Water,
    /// The player's `graphics.shadow_resolution`: how many texels the shadow maps get, read
    /// when the scene is built. See `oag_render::shadow::map::Resolution`.
    pub shadow_resolution: oag_display::display::ShadowResolution,
    /// The player's `graphics.scene_precision`: whether Wipeout HD's scene is drawn into the
    /// original's 8-bit target or a float one, read when the scene is built. A title whose
    /// scene does not store encoded values ignores it. See
    /// [`oag_display::display::ScenePrecision`].
    pub scene_precision: oag_display::display::ScenePrecision,
    /// The player's `graphics.circuit_lighting`: how much of the authored rig a circuit's plain
    /// surfaces compute, read when the scene is built. See
    /// [`oag_display::display::CircuitLighting`].
    pub circuit_lighting: oag_display::display::CircuitLighting,
    /// The player's `graphics.reflection_resolution`: how many texels the screen-space glass and
    /// sea target gets, read when the circuit loads. See
    /// [`oag_display::display::ReflectionResolution`].
    pub reflection_resolution: oag_display::display::ReflectionResolution,
    /// Whether the frame loop times the glass and sea target's pass, which is what steps
    /// `auto` (`oag_present::drs::reflection`), or `None` where the race loads before the device
    /// exists (the `--race` routes); read only for the loader report's line.
    pub reflection_timed: Option<bool>,
    /// Team id, which selects both the handling stats and the model, or `None`
    /// for the source's own.
    ///
    /// **`None` rather than a constant, for the reason [`Self::track`] gives
    /// and one more of its own.** `--team`'s help has promised since it landed
    /// that leaving it out takes the value "from the source's own title, the
    /// same way `--track` does" - so a title is never served `oag_pulse::race::DEFAULT_TEAM`, which would work
    /// on HD only because a PSARC folds case (`Data\Ships\Assegai\handlingstats.xml`
    /// normalises onto the `assegai` the manifest stores).
    ///
    /// Resolved by [`load`] once the source is open and its title known, from
    /// [`oag_title::RaceDefaults::team`], and the choice is reported.
    pub team: Option<String>,
    /// The player's own alternate hull file, for a title whose
    /// [`oag_title::RaceDefaults::hull_variants`] offers one - `None` for
    /// [`Self::team`]'s own baseline hull, on every title including one with
    /// no such axis at all.
    ///
    /// **Not a team-identity change**, unlike [`Self::team`]'s own RACE-page
    /// VARIANT sibling on HD/2048: [`oag_livery::load`] applies this to
    /// slot 0 alone, and everything else about the team - its tuning, its
    /// label, its roster membership - stays exactly `team`'s. See `oag_title::race::HullVariant`.
    pub hull_variant: Option<String>,
    /// The `PI_ModelSkin` the **player's** craft is painted with - the
    /// declared name (`Alternative`, `Eliminator`), not a path.
    ///
    /// `None` is the hull's own textures, which is every race before this
    /// existed and every race that does not ask.
    ///
    /// **Which skin a race flies is this project's choice, not the
    /// original's, and no unlock is checked.** The original appears to select
    /// `Eliminator` on a global state check rather than on a player pick, but
    /// what that state *is* is not settled - see
    /// `oag_livery::ship_skin`'s own module docs for the evidence, and for
    /// what would replace this. Labelled chosen rather than measured.
    ///
    /// Applied to slot 0 alone, like [`Self::hull_variant`] and for the same
    /// reason: nothing offers an opponent a paint job of their own.
    pub skin: Option<String>,
    /// The livery folder the **player's** craft is painted from, where its model names one other
    /// than its directory's own: Wipeout HD/Fury's and Omega's `texturelocation` (`livery2` for
    /// `SKIN1`), the part after [`oag_title::race::LIVERY_SEPARATOR`] of a stored variant id.
    /// Every texture the hull names under its own livery folder is read from this one instead
    /// (`oag_livery::LiveryFolder`). `None` is the hull's own. Slot 0 alone, like
    /// [`Self::skin`]: an opponent flying the player's variant keeps the directory's livery.
    pub livery: Option<String>,
    /// The team ids the *opponents* may fly, in the caller's own order.
    ///
    /// The caller supplies them because they come off the player's own disc -
    /// `catalogue::all_teams` over the plugin definition and any mounted DLC
    /// pack - and `load` may not invent a team id any more than it may invent a
    /// path. **Empty means the whole grid wears the player's hull**, which is
    /// what this engine did before liveries and what a source with no readable
    /// definition still gets.
    ///
    /// Which of them ends up in which slot is [`oag_race::roster::teams_for_slots`],
    /// Pulse's own roster draw off the race seed.
    pub opponent_teams: Vec<String>,
    /// A per-AI-slot team override, index `0` being grid slot `1` (slot `0`
    /// is always the player) - **what authored data replaces
    /// [`oag_race::roster::teams_for_slots`]'s own draw with**, where a caller has one to offer.
    ///
    /// `None` at an index leaves that slot exactly what
    /// [`oag_race::roster::teams_for_slots`] would already give it; an index
    /// past the end of this list is the same as `None` there. Empty
    /// (`Vec::new()`, [`Default`]'s own value) changes nothing anywhere -
    /// every caller outside the one below.
    ///
    /// **The one caller today is Wipeout 2048's own campaign**:
    /// `race::load_event` (`crates/raceplay/src/load/campaign.rs`) sets
    /// this from `oag_2048::campaign::craft::grid_craft`'s own reading of an
    /// event's `M_PGRIDSHIPMODELDATA` - the disc's own AI grid, when it
    /// authors one, in place of the roster draw. See that
    /// module's doc comment for what fraction of `SP.xml`'s events author it
    /// and how fully.
    pub grid_teams: Vec<Option<String>>,
    /// What each person seated after the first picked for their own craft,
    /// index `0` being grid slot `1` - the split screen's Ship Select, once per
    /// seat. **Empty is every race with one person**, and changes nothing
    /// anywhere.
    ///
    /// Slot `i + 1` flies `seat_picks[i].team` in its hull variant and skin,
    /// and the opponents fill the slots after the seated ones; a team a person
    /// took is moved out of the opponents' draw rather than flown twice. See
    /// `load::roster::apply_seat_picks`.
    pub seat_picks: Vec<SeatPick>,
    /// Whether a Wipeout 2048 Zone race draws its own `trackZone.rcsmodel`.
    ///
    /// The game turns it on (`fc01_dummy`'s road reads as the circuit's fog colour); [`Options::default`] keeps it off so a
    /// library caller that names no Zone model gets the ordinary circuit.
    pub zone_model: bool,
    /// Whether a Pulse hull's `0x2000` extra pass is built at all
    /// ([`oag_render::shine`]). `true` is the game; `false` is `--no-hull-shine`,
    /// the headless way to take the pass out of a frame and so measure what it
    /// adds.
    pub hull_shine: bool,
    /// Whether each craft's `shipwreck.vex` is loaded and swapped in once the title's
    /// destroyed sequence reaches it (`oag_title::destroyed::WreckFrom`). `false` is
    /// `--no-hull-wreck`.
    pub hull_wreck: bool,
    /// Whether a Pulse circuit's own extra pass is drawn: its `*_shinemap`
    /// batches under their chrome map. `false` is `--no-track-shine`.
    pub track_shine: bool,
    /// Speed class the handling parameters are read for, spelled the way the
    /// disc spells it.
    ///
    /// **A name rather than [`SpeedClass`]**, because the ladder's length is
    /// per-title measured data and no four-variant enum can carry every
    /// title's. Wipeout Pure authors five rungs - `VECTOR` below `VENOM` - in
    /// every race team's `handlingstats.xml`, in its engine-wide
    /// `<GlobalClass>` table and in its pickup odds; Pulse and HD author four.
    /// The name is resolved against the file that authored it, by
    /// `oag_tables::handling::Stats::class_named` and its siblings, so a title
    /// that authors four never grows a fabricated fifth entry.
    ///
    /// Matched case-insensitively wherever it is resolved. See
    /// `oag_title::SpeedClasses`, which is where a title's ladder is measured.
    pub class: String,
    /// How good the opponents are.
    ///
    /// Applied by degrading the measured tuning rather than by boosting a weak
    /// one, and it never reads the player - see [`oag_ai::Difficulty`].
    pub difficulty: oag_ai::Difficulty,
    /// The original's integer difficulty (`g_skill_level`: `0` Easy, `1` Medium, `2` Hard) the
    /// race flies at, where it has a rung of its own (a campaign cell's); `None` takes
    /// [`oag_ai::Difficulty::level`] of [`Self::difficulty`].
    ///
    /// It picks every craft's shield pool out of `<Misc>`'s three slots
    /// (`oag_tables::handling::Misc::shield_for`). The original reads one global for the
    /// player and the opponents alike (`docs/formats/handling-stats.md`, "Which difficulty
    /// picks the shield slot").
    pub skill_level: Option<u8>,
    /// Which mode's rules the race runs under.
    ///
    /// Also selects the HUD layout: a Zone run draws `Zone_HUD.xml`, the other
    /// two share `TimeTrial_HUD.xml`. See [`hud_layout`].
    pub mode: Mode,
    /// The HUD's own preferred language, by its plugin's own English name -
    /// `None` takes the chain's default the same way an unset
    /// `boot::Options::language` does everywhere else a source is opened.
    ///
    /// **Threaded through explicitly rather than read off a global.** The
    /// HUD used to be handed `None` outright at its one call site
    /// ([`hud_layout`]'s caller, `load::load`), which is a bug this field
    /// exists to close: `None` is the chain's default language, and on the
    /// PSP EU pressing that default is French rather than English - so a
    /// player who picked German got a HUD that happened to look right only
    /// if they had picked French. Every launch site sets this from the live
    /// `settings::Settings::language` at the moment it builds these
    /// `Options` - `main::prepare::Pending::race_options` for `--race`, and
    /// `main::session::Session::launch_race` for every menu-driven route,
    /// which re-reads it there rather than trusting whatever this field held
    /// when `Session::race_options` was last built, precisely because the
    /// OPTIONS page's LANGUAGE row can change the live setting without
    /// rebuilding this struct at all.
    pub language: Option<String>,
    /// The player's `HUD Style`: an entry of the craft's title's disc list
    /// ([`oag_title::HudLayouts::skins`]), or `None` for the list's default. A name the
    /// title's list does not hold draws the default skin and says so in the loader report.
    /// Presentation only: nothing the simulation, a replay or a net message carries.
    pub hud_style: Option<String>,
    /// The `Campaign=` value of the downloadable campaign this race is an event of
    /// ([`oag_title::AddOn::campaign`]), or `None` for every other race. The HUD then reads
    /// that package's [`oag_title::AddOn::hud`] in place of the title's own, for the modes it
    /// lists. Set by the campaign map's cell launch (`Session::launch_tier_cell`).
    pub add_on_campaign: Option<String>,
    /// The race is an event of a campaign (an original's cell, a downloadable one's, a mod's, a
    /// 2048 event). What a simulation module is told about its race
    /// (`oag_ext_abi::RACE_CAMPAIGN`); nothing else reads it.
    pub campaign: bool,
    /// The kill count that ends an Eliminator event, or `None` for
    /// [`Mode::ELIMINATOR_KILL_TARGET_DEFAULT`].
    ///
    /// **A parameter rather than a constant, on purpose.** The real number
    /// lives on the campaign's own `PI_Cell` records (`10`, `7` or `5` -
    /// `Eliminator_UpdateKillTarget`,
    /// `docs/ghidra/functions/psp-pulse-usa/race-campaign.md`), and that
    /// campaign is not wired into this engine - `--mode eliminator` outside
    /// it has no cell to read one from. This is the escape hatch for the day
    /// it is: a campaign loader would fill it in per cell instead of leaving
    /// it `None`.
    pub eliminator_kill_target: Option<u32>,
    /// A campaign cell's score target. A score Eliminator takes this and never the `KILLS`
    /// pick above, which another title's kill count may have stored.
    pub eliminator_score_target: Option<u32>,
    /// Laps this race ends after, overriding [`Mode::laps_target`]'s own
    /// per-class table - or `None` for that table's own answer, which is
    /// every caller outside the campaign.
    ///
    /// **Only meaningful for the two modes whose own [`Mode::laps_target`]
    /// already returns `Some`** - [`Mode::TimeTrial`] and
    /// [`Mode::SingleRace`]. A caller must not set this for [`Mode::SpeedLap`]
    /// or [`Mode::Zone`]: both author a `laps` attribute on their own
    /// campaign cells (`7` and `0`) that is display convention, not an
    /// ending - `docs/gameplay/race-modes.md` measures both as never ending
    /// on their own, live, and turning either field into
    /// [`oag_race::RaceState::laps_target`] would end a race the original
    /// does not. [`Mode::Eliminator`] ends on a kill count instead and its
    /// campaign cells author no `laps` attribute at all - see
    /// [`Self::eliminator_kill_target`].
    ///
    /// **The real number is per campaign cell, not this table.** A cell's
    /// own `laps="%d"` is the authority the moment a campaign launch can set
    /// it (`oag_tables::race_campaign::Cell::laps`) - `Mode::laps_target`'s
    /// table is the fallback for a Custom Race outside the campaign, which
    /// has no cell to read one from. See that constant's own doc for the
    /// same retirement clause [`Self::eliminator_kill_target`] carries.
    pub laps_override: Option<u32>,
    /// The weapons switch this race runs with, or `None` for
    /// [`Mode::weapons_enabled`]'s own answer.
    ///
    /// **Set only for a single race**, the one mode whose `WEAPONS` row the
    /// original leaves editable - `Settings::race::weapons_override`. In every
    /// other mode the mode decides, so nothing else sets it.
    pub weapons_override: Option<bool>,
    /// Force the Zone colour grade to a stage, instead of resting where the
    /// title's own ladder leaves it.
    ///
    /// **A development override, and the only way to see HD/Fury's Zone look
    /// at all today.** What advances the stage during a race is recovered on
    /// 2048 and not on HD, so an HD Zone race otherwise sits on whichever
    /// stage its loader left - see `crate::zone_grade`. Comparing a
    /// frame against the original needs the two on the same rung, and the
    /// original visibly is not on `Start`: at Moa Therma's start line it is
    /// already showing `Sub Venom`'s cyan (`Scene.Base Colour` =
    /// `0.003922 0.847059 1.000000`), where `Start` authors a flat
    /// `3.0 3.0 3.0` with no hue in it at all.
    ///
    /// Clamped to the stages the loaded file names. `None` leaves the ladder
    /// alone, which is what every non-development caller wants.
    pub zone_stage: Option<u32>,
    /// Hold 2048's race-composite exposure at this value instead of adapting it.
    ///
    /// **A comparison switch, never the play default.** Vita3K never writes the
    /// bloom gate's target back to guest memory, so the original's exposure
    /// step reads zero and never runs there: every 2048 reference frame is drawn
    /// at the 1.15 `Exposure_Set` resets to (`oag_post::wo_composite::Exposure`).
    /// The compare tooling passes `--hold-exposure 1.15` so a pair compares like
    /// with like. `None` adapts, as a Vita does. No effect on a title without the
    /// chain.
    pub exposure_hold: Option<f32>,
    /// Draw the driveable ribbon instead of the track's art meshes.
    ///
    /// **A development view, not a style.** The ribbon is the geometry the
    /// simulation spawns on and queries, so ship-plus-ribbon shows directly
    /// whether the ship is where the physics thinks it is. Default is off, which
    /// means a race draws the map.
    pub ribbon: bool,
    /// Read no sound bank for the race: the effect, engine and voice cues, the announcers and
    /// the countdown are never decoded, so none plays and the load reports none of them. The
    /// soundtrack is [`oag_sound::Audio::set_soundtrack`]'s, not this. For a run that is
    /// looking at the picture and has no one to hear it.
    pub no_effects: bool,
    /// Overlay the collision soup - the geometry the physics world is actually
    /// made of - the same view `oag-view --collision` draws, wireframe and
    /// Cage-excluded, on top of whichever track model was chosen above.
    pub collision: bool,
    /// Force the rest of the grid to spawn even though `mode` says nothing
    /// races there.
    ///
    /// **A verification aid, not a menu choice - the same shape as [`pose`]
    /// and [`camera`] below.** [`Mode::has_opponents`] is `false` for every
    /// mode this crate implements, because the original races all three
    /// single-ship; this exists so `crates/game/tests/race_ground_truth.rs`'s
    /// grid-geometry check and `just play`'s own visual QA can still put
    /// eight craft on the grid without pretending a fourth mode exists.
    /// `false` follows the mode, which is what every real race does.
    ///
    /// [`pose`]: Options::pose
    /// [`camera`]: Options::camera
    pub opponents: bool,
    /// How many split-screen views the race will be drawn in; `0` and `1` are
    /// no split. **Only the load needs it**, to read a title's authored
    /// two-player split HUD ([`oag_title::hud::SplitLayouts`]) and pack its
    /// textures into the HUD sheet when there are two views; with one view the
    /// sheet and the layout are exactly what they always were.
    pub views: u8,
    /// How many people fly the race, across every window: the number of person-flown slots, which
    /// the starting grid reads to put every person on the back rows in seat order. `0`, the
    /// default, means one person per view ([`Self::views`]), which is right for one window and
    /// is what every split race built before windows had. **A multi-window race must set it**:
    /// `views` is then what one window's HUD needs (`--window-seats 1,1` is one view), not the
    /// head count, and reading it for the grid put the second person at the front as an opponent.
    pub people: u8,
    /// The class has no controller that defines how to fly it
    /// (`oag_ai::coverage`): the race fields no opponents and says so. Set by
    /// the race box, a two-title race page and the command line; a campaign launch leaves
    /// it `false` because its own controller (the original's) is chosen by the
    /// tables its title authors for the class.
    pub no_driver_for_class: bool,
    /// [`Options::source`] may be a server extraction (`oag_source::extract`): set by the
    /// headless server only. A client never opens one.
    pub server_source: bool,
    /// Play the trail-hit sparks on the player regardless of the trigger.
    ///
    /// A verification aid for the draw path alone - see the `--trail-sparks`
    /// flag, which is the only thing that sets it, and
    /// `RaceView::advance_trail_hits` for why the two need separating.
    pub trail_sparks: bool,
    /// The world generator's seed, or `None` for [`SEED`].
    ///
    /// **A verification aid too**, and it exists because one already-recovered
    /// thing became untestable without it: `crates/game/tests/race_ground_truth.rs`
    /// asserts a fired Turbo's *magnitude* off the disc's own `<Engine turbo>`,
    /// which needs a pad crossing that actually draws a Turbo. That was
    /// automatic while `oag_weapons::pickup::IMPLEMENTED` held one weapon and
    /// stopped being so the moment it held two. A seed the test can choose is
    /// what keeps that assertion pointed at the weapon it is about, rather than
    /// weakening it to "whatever the pad handed over".
    ///
    /// A caller that does not set it gets [`SEED`]. The front end sets a fresh seed per
    /// race (`oag_game::race_seed`), so only `--race`, headless runs and tests ride the fixed
    /// one.
    pub seed: Option<u64>,
    /// Put the craft here instead of on its grid slot.
    ///
    /// **A capture aid, not a spawn.** The point is that two circuits, or a
    /// frame of ours and a frame of the original, can be photographed from the
    /// same place instead of by running the same number of ticks and hoping.
    /// `None` spawns normally.
    pub pose: Option<PoseRequest>,
    /// Put the opponents here too, one exact pose per grid slot, index `0` being grid slot `1`
    /// (the same indexing as [`Self::grid_teams`], whose teams a capture sets beside it).
    ///
    /// **A capture aid for a mid-race frame** of the original: the whole field read out of its
    /// RAM at one tick. Empty (the default) changes nothing anywhere. Non-empty, together with
    /// [`Self::pose`], keeps the grid's opponents alive and puts each one on its pose instead of
    /// on its grid row; a slot past the end of the list stays on its grid row.
    pub field: Vec<Pose>,
    /// The HUD's place, lap and clocks, imposed. See [`CaptureHud`]; `None` for every race.
    pub hud: Option<CaptureHud>,
    /// Seconds added to the world meshes' texture-animation clock, which is the race's own tick
    /// count over 60 otherwise. **A capture aid**: the original feeds every world mesh a clock that
    /// is the HUD clock plus a per-state constant (13.6 to 24.3 s on the 24 Pulse PSP grid states;
    /// inferred to be the time since the circuit loaded), and a frame of it is only comparable when ours
    /// is put at the same phase. Read off the original's RAM, never derived; `0.0` is every race.
    pub world_clock_offset: f32,
    /// The start gantry's animation time in seconds, imposed on the board for the frame. **A capture
    /// aid**: the original's start board loops over a window of its own node's time (`[3.83, 5.25)`
    /// s before the first line crossing, 86 ticks a loop) whose phase the race clock alone does not
    /// give, so a pair is only comparable when ours shows the phase the original's RAM held. Read
    /// off the gantry node's `+0xc0`, never derived; `None` is every race.
    pub gantry_seconds: Option<f32>,
    /// The engine flame surfaces' clock in seconds, imposed for the frame. **A capture aid**: HD's
    /// flame scrolls its noise by `2 * time` against the frame context's boot clock (engine
    /// parameter slot 0), which no race clock of ours gives, so a still pair matches the noise's
    /// phase only when ours draws at the clock the original's RAM held. Read off RAM, never
    /// derived; `None` is every race, where the flame rides the world clock.
    pub flame_seconds: Option<f32>,
    /// Render from this camera instead of the chase camera.
    ///
    /// The other half of the same capture aid: a ship pose replicates *what*
    /// the original showed, and only a replicated camera replicates *how it was
    /// framed*. `None` uses the chase camera as always.
    pub camera: Option<CameraOverride>,
    /// Replace the authored field of view with this, in vertical degrees, while the chase
    /// camera stays: [`CameraOverride::fov_deg`] for a capture that keeps its own camera rig.
    /// A posed craft has no velocity to widen the field with, so a comparison against a frame
    /// taken at speed passes the frame's field here. Ignored when [`Self::camera`] is set,
    /// whose own `fov_deg` stands in for it. `None` is every race.
    pub fov_override: Option<f32>,
    /// The race is an attract demo: nobody in it. The title's demo screen is read for it
    /// ([`Loaded::demo_overlay`]).
    pub demo: bool,
}

/// The two capture aids [`Options::pose`] and [`Options::camera`] carry.
mod capture;
pub use capture::{
    CameraOverride, CaptureHeadToHead, CaptureHud, CaptureShieldFlash, CaptureSong, CaptureZone,
    PoseRequest,
};

/// [`Options`]' `Default`.
mod defaults;
mod seat;
pub use seat::SeatPick;

/// What the simulation of a race is built from: everything [`Race::start`] reads to
/// build the [`RaceSim`] and its [`crate::outcome::Outcome`], and nothing only the
/// picture or the sound reads (that is [`ViewSetup`]).
///
/// No field names the disc, a renderer, an audio device, a window or the extension
/// host, so a test can build one by hand, or take one off a disc, and run a whole
/// race against it on a machine with no graphics driver. The simulation modules a
/// race runs travel beside it ([`Loaded::modules`]), because a compiled module is
/// the extension host's type.
#[derive(Debug, Clone)]
pub struct SimSetup {
    /// Which mode's rules the race runs under.
    pub mode: Mode,
    /// Unresolved, like [`Options::eliminator_kill_target`] - [`Race::start`]
    /// is what applies [`Mode::ELIMINATOR_KILL_TARGET_DEFAULT`].
    pub eliminator_kill_target: Option<u32>,
    /// See [`Options::laps_override`] - carried through unresolved the same
    /// way [`Self::eliminator_kill_target`] is, and applied to
    /// [`oag_race::RaceState::laps_target`] by [`Race::start`] once the
    /// class-derived table has already run.
    pub laps_override: Option<u32>,
    /// See [`Options::weapons_override`]; resolved by [`Self::weapons_on`].
    pub weapons_override: Option<bool>,
    /// How good the opponents are. See [`Options::difficulty`].
    pub difficulty: oag_ai::Difficulty,
    /// The finished player's thrust law, off the disc. See `race::finished_thrust`.
    pub finished_thrust: Option<super::finished_thrust::FinishedThrust>,
    /// What the campaign's original opponent reads, for a title that flies
    /// it. See `race::original_tables`.
    pub original_tables: Option<super::original_tables::OriginalTables>,
    /// The speed class the race is run in, spelled the way the disc spells it.
    ///
    /// Most of what the class decides is already resolved by the time a `SimSetup`
    /// exists - the handling block, the gravity scale and the speed-pad tunables
    /// are all per-class and all baked in above. It is carried anyway because
    /// [`Self::weapons`]' pickup odds are indexed by class *inside* a table this
    /// keeps whole, and resolving that at load would throw away the other
    /// classes' rows for no gain.
    ///
    /// A name rather than [`SpeedClass`], for the reason
    /// [`Options::class`] gives.
    pub class: String,
    /// Whether to spawn the rest of the grid regardless of what `mode` says.
    ///
    /// The resolved form of [`Options::opponents`] - see its doc comment.
    /// `mode.has_opponents()` already decides this correctly for every mode
    /// this crate implements; the field exists only so a verification build
    /// can override that decision.
    pub opponents: bool,
    /// See [`Options::no_driver_for_class`]: the grid is left empty.
    pub no_opponents: bool,
    /// How many people fly this race at once: [`Options::people`], else [`Options::views`], at least one. Only the
    /// starting grid reads it - two or more people take the back of the grid in order
    /// ([`Race::start`]); one person, the default, changes nothing.
    pub people: u8,
    /// The world generator's seed, already resolved from [`Options::seed`].
    pub seed: u64,
    /// The decoded spline graph, as the file has it.
    pub ai: AiTrack,
    /// The spline resampled for locating a ship, and for placing it.
    pub spline: Spline,
    /// The Workshop id of a foreign track, which keys its records and ghosts.
    pub foreign: Option<String>,
    /// Zone mode's speed law and recharge, when the source carries them.
    pub zone: Option<oag_tables::handling::Zone>,
    /// The same graph walked into a closed ring, for lap counting.
    ///
    /// `None` when the primary chain does not close, which means this track gets
    /// no lap counter rather than a wrong one. The load report says so.
    pub course: Option<Course>,
    /// The track's own authored grid slot, when it has one.
    ///
    /// `None` is not a broken track: it means this source's file carries no
    /// `Start Position` node, and a ship goes on the spline instead. Every PSP
    /// track has one, so on that path this is always `Some`.
    pub start_position: Option<StartPosition>,
    /// A laid Mine or Bomb starts at the rear anchor: `looks.laid_from_rear`.
    pub laid_from_rear: bool,
    /// Whether the grid is laid out the way Pulse PSP's `Race_ComputeGridLayout` does: a walk
    /// of the located curve with the original's scaled record and its edge-chord heading
    /// (`oag_gameplay::grid_walk`), rather than a walk of resampled samples that take the
    /// node's heading or the sample tangent (`oag_gameplay::orientation_on_sample`, still
    /// the fallback where the walk refuses a node). Pulse off a PSP disc only, the one grid
    /// layout function read and measured against the original:
    /// `docs/physics/grid-state.md`.
    pub grid_frame_from_sample: bool,
    /// The team id each grid slot flies (`Feisar`), for the HUD's per-craft
    /// rows - the Eliminator's kill column. Ids, not names: the string table
    /// turns one into the name a player reads at draw time. Empty where a
    /// caller builds a `SimSetup` by hand.
    pub slot_teams: Vec<String>,
    /// [`Self::slot_teams`] as the title's string table keys them
    /// ([`oag_title::Title::team_key`]), which the Eliminator's rows name each craft by.
    pub row_teams: Vec<String>,
    /// The lock-on's grids and law, which [`Race::start`] applies. See [`Sighting`].
    pub sight: Sighting,
    /// Every collidable triangle of the track.
    pub collision: CollisionWorld,
    /// The force law's parameter set for one team in one speed class.
    pub handling: Handling,
    /// Each grid slot's own tuning where it flies a different team's file than
    /// the player, on every title; `None`, or an empty list, is the player's [`Self::handling`], as every opponent
    /// flew before.
    pub opponent_handling: Vec<Option<Handling>>,
    /// The player's rocket and mine counts, from the same `handlingstats.xml` as
    /// [`Self::handling`] (`oag_weapons::counts`).
    pub weapon_counts: oag_weapons::counts::WeaponCounts,
    /// Each grid slot's own counts, from the file [`Self::opponent_handling`] read; `None`,
    /// or an empty list, is the player's [`Self::weapon_counts`].
    pub opponent_weapon_counts: Vec<Option<oag_weapons::counts::WeaponCounts>>,
    /// The per-class scale on grounded gravity, from `<GlobalClass><GravityMul/>`.
    ///
    /// `1.0` when the engine-wide file could not be read, which leaves gravity
    /// exactly as it was before this was decoded. On the sim half deliberately:
    /// it reaches `oag_physics::forces::Environment` every tick and a headless
    /// race must fall the same way a drawn one does.
    pub class_gravity_scale: f32,
    /// The disc's `<StartBoost>`, `None` where it authors none: the launch boost.
    /// Presence is the only gate. See `oag_physics::launch`.
    pub start_boost: Option<oag_physics::launch::StartBoost>,
    /// The laws of the player's two Pilot Assist levels: the disc's where it authors
    /// them, a labelled stand-in where it does not (`load::pilot_assist`).
    pub pilot_assist: oag_physics::pilot_assist::Laws,
    /// The disc's `<Collision shipWallCofR>`, the craft-wall restitution; `None` where it
    /// authors none, which leaves the engine's ([`oag_physics::laws::Laws::wall_restitution`]).
    pub wall_restitution: Option<f32>,
    /// The chase camera's seven values, from `<ExternalCameraFar>`.
    pub chase: ChaseParams,
    /// Each slot's `Ship Muzzle` locator in its hull's model space, indexed like
    /// [`ViewSetup::nozzles`] and read in both halves (`load::muzzle`): the weapon node a
    /// Plasma bolt charges on and leaves from (`oag_gameplay::world::Ship::muzzle`),
    /// and where its head draws while it charges. `None` for a hull that authors
    /// none or a version whose class id is unread.
    pub muzzles: Vec<Option<Vec3>>,
    /// Slot 0's `cannon_flash_left` and `cannon_flash_right` locators in its hull's model space
    /// (`load::muzzle::cannon_flash`), `[left, right]`: where Detonator's pulse cannon fires.
    /// `[None, None]` off Detonator and for a hull that authors none.
    pub cannon_flash: [Option<Vec3>; 2],
    /// This title's own zone-number-to-speed-class ladder, when it is recovered - the same
    /// [`oag_title::ZoneStages`] [`crate::zone_grade::ZoneGrade`] carries, copied here so
    /// [`ViewSetup::class_announcer`] can be triggered off the same edge without reaching into the
    /// render-facing scene state to get it. `None` on every title but HD/Fury.
    pub zone_stages: Option<&'static oag_title::ZoneStages>,
    /// Whether this title's `ready` and `go` voice has been measured, so the race raises
    /// [`oag_gameplay::cue::Cue::Ready`] and [`oag_gameplay::cue::Cue::Go`] at the ticks
    /// [`crate::countdown`] pins: `oag_title::RaceDefaults::countdown_voice` being `Some`. A flag
    /// rather than the banks: which bank plays rides [`ViewSetup::sounds`]; whether the simulation asks
    /// for the cue at all is title data, and must not change with whether a bank decoded.
    pub countdown_voice: bool,
    /// Whether the start also plays the start beat, [`oag_gameplay::cue::Cue::StartBeat`], with
    /// `ready`: `oag_title::CountdownVoice::beat` being `Some`.
    pub countdown_beat: bool,
    /// The title's grid hover, `oag_title::RaceDefaults::launch_hover` ([`crate::launch_hover`]).
    pub launch_hover: Option<&'static oag_title::launch_hover::LaunchHover>,
    /// The title's hover probe set, `oag_title::RaceDefaults::hover_rig` ([`crate::launch_hover`]).
    pub hover_rig: Option<&'static oag_title::hover_rig::HoverRig>,
    /// The title's craft-body laws, `oag_title::RaceDefaults::craft_laws` ([`crate::launch_hover`]).
    pub craft_laws: Option<&'static oag_title::craft_laws::CraftLaws>,
    /// The title's Autopilot engine law, `oag_title::RaceDefaults::autopilot_engine`
    /// (`RaceSim::autopilot_engine_flags`).
    pub autopilot_engine: oag_title::autopilot_engine::AutopilotEngine,
    /// Whether an opponent on this project's driver flies the Autopilot it
    /// holds rather than absorbing it: the title's
    /// `oag_title::RaceDefaults::opponent_autopilot` at the race's class
    /// (`RaceSim::set_opponent_autopilot` overrides it).
    pub opponent_autopilot: bool,
    /// How a destroyed craft's sequence runs, `oag_title::Title::destroyed`. The explosion,
    /// dwell and settle timers reach every craft's physics; whether a wreck leaves the world
    /// reads here.
    pub destroyed: oag_title::destroyed::DestroyedCraft,
    /// What colours a weapon pad's light bars, `oag_title::Title::weapon_pad_glow`.
    pub weapon_pad_glow: &'static oag_title::weapon_pad::WeaponPadGlow,
    /// When a locked LeachBeam moves energy,
    /// `oag_title::weapons::Weapons::leach_drain`.
    pub leach_drain: oag_title::weapons::LeachDrain,
    /// How a weapon blast's push reaches a craft,
    /// `oag_title::weapons::Weapons::blast_push`.
    pub blast_push: oag_title::weapons::BlastPush,
    /// How many LeachBeams may be live at once,
    /// `oag_title::weapons::Weapons::leach_beams`.
    pub leach_beams: u8,
    /// How a Quake's wave reaches a craft, `oag_title::weapons::Weapons::quake_hit`.
    pub quake_hit: oag_title::weapons::QuakeHit,
    /// Whether an absorb press cancels a running Autopilot,
    /// `oag_title::weapons::Weapons::autopilot_absorb_cancels`.
    pub autopilot_absorb_cancels: bool,
    /// Whether a weapon pad grants a craft whose Autopilot runs,
    /// `oag_title::weapons::Weapons::autopilot_pad_grants`.
    pub autopilot_pad_grants: bool,
    /// Whether a Rocket, Missile, Shuriken or Plasma bolt is born carrying the located track's
    /// up rather than the craft's, `oag_title::weapons::Weapons::launch_normal_is_track_up`.
    pub launch_normal_is_track_up: bool,
    /// Whether the projectiles' floor arms are Pulse's decompiled laws,
    /// `oag_title::weapons::Weapons::floor_arms_steer_to_ride_point`.
    pub floor_arms_steer_to_ride_point: bool,
    /// Whether the Rocket and Plasma sweep first and ride the normal they were launched with,
    /// `oag_title::weapons::Weapons::floor_arms_ride_launch_normal`.
    pub floor_arms_ride_launch_normal: bool,
    /// Whether a Plasma bolt leaves at the craft's display scale,
    /// `oag_title::weapons::Weapons::plasma_release_at_display_scale`.
    pub plasma_release_at_display_scale: bool,
    /// `oag_title::weapons::Weapons::rocket_launch_at_display_scale`.
    pub rocket_launch_at_display_scale: bool,
    /// What the firing craft's box does to its projectiles' first probes,
    /// `oag_title::weapons::Weapons::own_craft_box_hides_track`.
    pub own_craft_box_hides_track: oag_title::weapons::OwnCraftBox,
    /// `oag_title::weapons::Weapons::surface_probe_lengths`.
    pub surface_probe_lengths: oag_title::weapons::ProbeLengths,
    /// `oag_title::weapons::Weapons::rocket_open_air`.
    pub rocket_open_air: oag_title::weapons::RocketOpenAir,
    /// Which ways a craft is put back are on, and the numbers they fire on:
    /// `oag_race::recovery::ResetTriggers::PULSE` with the title's
    /// `oag_title::Title::reset_enables` applied.
    pub reset_triggers: oag_race::recovery::ResetTriggers,
    /// How a craft's contact with the track or another craft is heard,
    /// `oag_title::Title::contact_cues`.
    pub contact_cues: oag_title::contact_cue::ContactCues,
    /// Which terms the opponents' fire-or-absorb decision rolls by,
    /// `oag_title::weapons::Weapons::opponent_fire`.
    pub opponent_fire: oag_title::weapons::OpponentFire,
    /// What an Eliminator is won on, `None` for kills: `oag_title::Title::eliminator` with the
    /// opponents' scale resolved for this race's difficulty.
    pub score_law: Option<oag_race::ScoreLaw>,
    /// How a score Eliminator's opponents spend their weapons:
    /// `oag_title::Title::score_pace`. Read only where [`Self::score_law`] is `Some`.
    pub score_pace_terms: oag_title::eliminator::ScorePaceTerms,
    /// The Eliminator flip, `None` outside an Eliminator and on a title without one:
    /// `oag_title::Title::eliminator_flip`.
    pub flip_law: Option<oag_physics::flip::FlipLaw>,
    /// The track's speedup pads, as trigger volumes.
    ///
    /// The same nodes [`Loaded::pad_model`] draws, decoded for what they *do*
    /// rather than for what they look like: an oriented box in each pad's own
    /// space, plus the world matrix that places it and gives the push its
    /// direction. Empty for a track that authors none.
    ///
    /// Nothing consumes these yet - the boost is the next milestone - but they
    /// belong to the simulation half rather than to [`Loaded`], because a
    /// headless race has to be able to trigger a pad without a GPU.
    pub speedup_pads: Vec<oag_vex::pads::PadVolume>,
    /// The track's weapon pads, as trigger volumes.
    ///
    /// Everything [`Self::speedup_pads`] says applies, one class over. Consumed
    /// by [`SimRace::test_weapon_pads`] in the one mode that arms them, and decoded
    /// on every mode regardless, which is what lets
    /// `the_weapon_pads_are_drawn_where_they_trigger` check the geometry against
    /// them. Two independent decodes of the same nodes agreeing is worth more
    /// than either alone.
    pub weapon_pads: Vec<oag_vex::pads::PadVolume>,
    /// The weapon table for this race, out of `WeaponStats_Race.xml`.
    ///
    /// `None` when the file is absent or unreadable, which means a pad hands
    /// nothing out rather than handing out an invented weapon - the same choice
    /// [`SimSetup::speedup_pads`]' tunables make. See `docs/formats/weapon-stats.md`.
    pub weapons: Option<oag_tables::weapons::WeaponStats>,
    /// The odds an opponent fires each weapon at, out of the title's
    /// `WeaponAIstats.xml` - see [`oag_title::weapons::Weapons::ai`].
    ///
    /// `None` runs the race on [`super::FireLaw::Ours`]: the original's law
    /// needs these numbers and nothing invents them.
    pub weapon_ai: Option<oag_tables::weapons::ai::WeaponAiStats>,
    /// Detonator's layout files, out of the title's `DetonatorLayouts` entries
    /// ([`oag_title::Detonator::layouts`]): the groups of clusters and the stage mine counts.
    ///
    /// `None` off Detonator, and where the files are absent or unreadable, which lays no mines
    /// rather than invented ones.
    pub detonator: Option<oag_weapons::detonator::Layout>,
    /// What Detonator's pads do on this title ([`oag_title::Detonator::pads`]); `None` keeps
    /// every pad on its race behaviour.
    pub detonator_pads: Option<oag_title::DetonatorPads>,
    /// Restricts which weapons a `Weapon Pad` may hand out to exactly this
    /// list, or does not restrict at all when empty - the same "empty is no
    /// override" convention [`Options::grid_teams`] uses.
    ///
    /// Set from [`oag_title::Title::pickup_roster`] for the race's mode, then
    /// replaced by `race::load::campaign::load_event` with a 2048 campaign
    /// event's own `M_WEAPONSET` -
    /// `oag_tables::mjolnir::campaign::WeaponSet::allowed_weapons`, the bits
    /// that project has pinned at confidence >= 70 (see
    /// `docs/formats/2048-campaign.md`'s "The weapon set gate" section).
    /// A title with no roster, outside such an event, restricts nothing - see
    /// [`oag_weapons::pickup::draw`]'s own `allowed` parameter.
    pub allowed_weapons: Vec<oag_tables::weapons::Weapon>,
    /// The order a person's roll passes a disallowed weapon's weight along
    /// ([`oag_title::pickup_roster::PickupRoster::player_order`]), or `None`
    /// where the title has no roster.
    pub pickup_order: Option<&'static [oag_tables::weapons::Weapon]>,
    /// Seconds a weapon pad is inert for after it is crossed, for this race's
    /// speed class.
    ///
    /// `<WeaponPad refresh_time>` out of the same `<GlobalClass>` block the
    /// speed-pad tunables come from, stamped onto the pad by the original's
    /// `WeaponPads_TestCraft` - see [`oag_tables::handling::WeaponPad`], which
    /// records why it is a debounce rather than a respawn. Zero when the global
    /// file could not be read, and in an Eliminator on a title that never stamps a
    /// pad there (`oag_title::weapons::Weapons::pads_stamp_in_eliminator`): the pad
    /// stays armed.
    pub weapon_pad_refresh: f32,
    /// Where [`Options::pose`] asked for the craft to start, already resolved
    /// against the spline. `None` uses the ordinary spawn.
    pub pose_override: Option<Pose>,
    /// [`Options::field`], carried to [`Race::start`].
    pub field_poses: Vec<Pose>,
    /// The Zone counters of [`Options::hud`], which [`Race::start`] writes into the
    /// player's race state so a Zone capture starts on the original's zone. `None`
    /// for every race but a capture's.
    pub capture_zone: Option<CaptureZone>,
}

/// A race's two setups plus the geometry to draw it with.
#[cfg(feature = "view")]
#[derive(Debug)]
pub struct Loaded {
    /// The simulation half.
    pub setup: SimSetup,
    /// The picture's and the sound's half.
    pub view: ViewSetup,
    /// The simulation modules the load compiled ([`crate::extensions::prepare`]), which
    /// [`Race::start`] takes beside [`Self::setup`].
    pub modules: crate::extensions::Prepared,
    /// A race already started from a copy of the three, where the
    /// loader had a thread to spare: the web build's load worker runs
    /// [`crate::Race::start`] (the AI's speed plans, 0.65 s for an HD circuit
    /// in the browser) so the page's thread does not (docs/tools/web.md,
    /// "Threads"). Natively the load worker starts one on a forked circuit when
    /// asked to learn the speed plans ahead ([`crate::LoadWorker::spawn`]'s
    /// `plan_ahead`).
    /// `None` from [`crate::load`]; the scene build starts one itself then.
    pub started: Option<Box<crate::Race>>,
    /// Which title the source turned out to be.
    ///
    /// **Carried out of [`load`] because `--race` has no other way to know**,
    /// and without it every `--race` run silently drew with
    /// `settings::RenderProfile::default()` rather than the player's own
    /// `[render_profiles.<title>]` - so `--render-scale 100` and
    /// `--render-scale 200` produced an identical scene pass and `fsr3` never
    /// ran at all. `load` resolves the title anyway, to pick the default track
    /// and team (see [`Options::track`]); this is the same value, kept.
    ///
    /// The track's title rather than `craft_title`: a render profile is about
    /// how expensive the *scene* is, and the circuit is the scene.
    pub title: &'static oag_title::Title,
    /// The archive entry of the circuit the race was built on: what `Options::track` named, or
    /// the title's default, or the first circuit the source ships when the default is not in
    /// it. A Zone race carries the zone variant. `foreign:<id>` for a foreign track.
    pub circuit: String,
    /// Which console the track's own source is for, carried out of [`load`]
    /// the same way `title` is and for the same reason: `--race` has no
    /// other way to know it, and `crate::settings::profile_key` needs both
    /// halves - the track's, not `craft_title`'s, on the same reasoning
    /// `title`'s own doc gives.
    pub platform: oag_disc::Platform,
    /// The HUD's layout, atlas, fonts and strings.
    pub hud: oag_hud::Assets,
    /// The track-description panel drawn over the pre-race flyby, off Pulse's PSP disc only.
    /// `None` elsewhere and when a part of it would not read - the load report says which. See
    /// [`crate::track_panel`].
    pub track_panel: Option<crate::track_panel::Assets>,
    /// The screen an attract demo draws over its race: `Some` for an [`Options::demo`] race of a
    /// title that authors one, when it read. See [`crate::demo_overlay`].
    pub demo_overlay: Option<crate::demo_overlay::Assets>,
    /// The circuit's own `stats.xml`, for the HUD's `RECORD` readout. `None`
    /// off Pulse's PSP disc, or when the file did not read - the load report
    /// says which.
    pub track_stats: Option<oag_tables::track_stats::TrackStats>,
    /// What to draw for the track.
    pub track_model: Model,
    /// Vineta K's behind-the-glass chunks and their fog; empty elsewhere.
    pub behind_glass: crate::BehindGlassModels,
    /// The start gantry and where it stands, when this circuit authors a mount
    /// for one and the model loads.
    ///
    /// `None` is a circuit with no gantry drawn at all, never a gantry at a
    /// borrowed coordinate: the mount is measured off each circuit's own
    /// geometry and every circuit's answer differs. See `race::gantry` and
    /// `docs/rendering/start-gantry.md`.
    ///
    /// **Beside it, the adverts** the other slots' placeholder quads show, each drawn
    /// through its own camera - see [`crate::adverts::Billboards`].
    pub billboards: crate::adverts::Billboards,
    /// One hull, plume, nozzle and spark-anchor set per grid slot, each off
    /// its own team's directory. Slot 0 is the player's.
    ///
    /// Per slot rather than one shared model: the eight teams
    /// the disc declares are eight *different* hulls (845 to 1,497 triangles),
    /// not one hull repainted, so a shared model was visibly one team's ship
    /// eight times over. See [`oag_livery`], which also records which half
    /// of this is recovered and which half is this project's.
    /// The model a Rocket in flight is drawn as: `Data\Weapons\Rocket.vex`.
    ///
    /// **Recovered, confidence 85.** `Rocket_Ctor` (`0x0885cc24`) builds the
    /// projectile a scene node of `.vex` class `0x3e9` from exactly this entry
    /// and keeps the handle at `+0x110`, which `Rocket_Update` drives every
    /// tick. See `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`.
    /// **A rocket is a model, not a billboard**, which is what this engine drew
    /// before. The file is 3712 bytes, 84 vertices, 28 triangles, radius 1.34:
    /// a finned dart.
    ///
    /// `None` when the source carries no entry under that name, on the same
    /// terms as [`Self::boost_model`] - a missing rocket model falls back to the
    /// billboard rather than failing the race.
    pub rocket_model: Option<Model>,
    /// The model a Cannon round in flight is drawn as:
    /// [`CANNON_MODEL_ENTRY`]. `None` on the same terms as
    /// [`Self::rocket_model`], and a round then draws as nothing at all
    /// rather than as a stand-in - see that constant.
    pub cannon_model: Option<Model>,
    /// The model a laid Mine is drawn as: [`MINE_MODEL_ENTRY`]. `None` on the
    /// same terms as [`Self::rocket_model`].
    pub mine_model: Option<Model>,
    /// The model a laid Bomb is drawn as: [`BOMB_MODEL_ENTRY`]. `None` on the same terms.
    pub bomb_model: Option<Model>,
    /// The Plasma's own detonation: a halo and two hemispheres, each `None`
    /// on the same terms as [`Self::rocket_model`] - see `blast_models`.
    pub plasma_blast_models: blast_models::PlasmaBlastModels,
    /// The Bomb's own detonation: a hemisphere and a shockwave, `None` on
    /// every title but Pulse - see `bomb_blast`.
    pub bomb_blast_models: bomb_blast::BombBlastModels,
    /// The LeachBeam's own ball, HD only:
    /// `Data\Weapons\hd_leachbeam_ball_bloomring.vex`. `None` on the same
    /// terms as [`Self::rocket_model`] - a source with no entry, or none
    /// recovered on this title's own [`oag_title::weapons::WeaponModels`],
    /// draws no ball at all rather than a stand-in. See
    /// `oag_fx::beam::hd_ball` for the position law it is drawn at.
    pub leach_ball_model: Option<Model>,
    /// The sphere a fired Shield shows from **inside** the cockpit, drawn
    /// instead of the per-team shell when the camera is in the craft.
    ///
    /// One entry for the whole game rather than one per team, which is the
    /// original's own arrangement: `ShipShield_Construct` (`0x0885db38`) builds
    /// the shell's name from the team's directory and this one from a literal
    /// `Data\Weapons`. `None` on a source that does not carry it, on the same
    /// terms as [`Self::rocket_model`].
    pub shield_cockpit: Option<Model>,
    /// The countdown's own `<Mode3D><Model>` mesh, `Data\HUD\Cockpit_321GO.vex` -
    /// see `oag_hud::countdown`. `None` on the same terms as
    /// [`Self::rocket_model`], and also whenever this mode's own layout carries
    /// no `Cockpit321Go` widget to place it by (Pure's, and any other title's).
    pub countdown_model: Option<(Model, oag_hud::Model)>,
    /// Detonator's medal models, each with the `<ImageModel>` that places it (empty outside
    /// Detonator, on a title that does not show them, and for a medal that did not decode).
    pub medal_models: Vec<(oag_hud::ImageModel, Model)>,
    pub liveries: Vec<Livery>,
    /// The collision soup, if [`Options::collision`] asked for it.
    pub collision_model: Option<Model>,
    /// The track's authored `fogCube` volumes, for [`Scene`] to sample per frame at the camera.
    pub fog_volumes: Vec<oag_vex::fog::FogVolume>,
    /// `05_Track`'s cloud puffs and the shared texture they draw with -
    /// `None` for every other circuit and for a ribbon build. See
    /// `crates/fx/src/cloud.rs` and
    /// `docs/ghidra/functions/psp-pulse-usa/clouds.md`.
    pub clouds: Option<(oag_fx::cloud::Layer, FlareTexture)>,
    /// The circuit's own light rig, out of its `.envsettings`.
    ///
    /// [`mesh_render::Light::stand_in`] for every title that authors none,
    /// which is all of them but Wipeout HD.
    pub light: mesh_render::Light,
    /// The player's [`Options::circuit_lighting`], for the scene build.
    #[cfg(feature = "view")]
    pub circuit_lighting: oag_display::display::CircuitLighting,
    /// The circuit's baked probes, which a hull's ambient is blended from: HD's `track.probes`
    /// or Omega's `track.shprobes` - see [`crate::env_probes::Baked`]. Empty where the circuit
    /// ships none (every other title, Omega's Zone circuits), and where a file did not read
    /// (the load report says which).
    #[cfg(feature = "view")]
    pub probes: crate::env_probes::Baked,
    /// The circuit's authored distance fog, out of its `.envsettings`.
    ///
    /// Wipeout HD's, and `None` everywhere else: Pulse fogs through the
    /// `fogCube` volumes in [`Self::fog_volumes`] instead, and the two never
    /// coexist - HD authors no `fogCube` node at all. Static for the race
    /// where a volume sample is per-frame, which is why it is a value here
    /// rather than something [`Scene`] samples.
    pub authored_fog: Option<mesh_render::Fog>,
    /// The circuit's authored `HDR and Bloom` values, out of the same
    /// `.envsettings`.
    ///
    /// Wipeout HD's, and `None` everywhere else. `Some` is what switches the
    /// race onto the linear float scene target and the read `FunkLayerBloom`
    /// chain - see `oag_post::hd_bloom`.
    pub hd_bloom: Option<oag_post::hd_bloom::Params>,
    /// Omega's `Tonemap` block, read into the curve its executable applies -
    /// see `oag_post::omega_tonemap`. `None` for every other title.
    pub omega_tonemap: Option<oag_post::omega_tonemap::Params>,
    /// 2048's exposure and bloom keys, run through its `wo_*` race chain - see
    /// `oag_post::wo_composite`. `None` for every other title.
    pub wo_composite: Option<oag_post::wo_composite::Params>,
    /// The Zone colour grade this title lays over the circuit, stage by
    /// stage, in a Zone race.
    ///
    /// `None` outside Zone, and `None` in Zone on a title that ships no
    /// `.effectSettings` table - Pulse and Pure, whose discs were searched for
    /// one rather than assumed empty. See [`oag_title::ZonePalette`] and
    /// [`crate::zone_grade::ZoneGrade`], whose module docs carry the one
    /// thing this is still missing: what selects a stage during a race, which
    /// is unrecovered on both titles that do ship a table.
    pub zone_grade: Option<crate::zone_grade::ZoneGrade>,
    /// The track's `Skycube`, when it authors one.
    ///
    /// Built from the same blob as [`Self::track_model`] and indexing the same
    /// textures, but kept separate because it is drawn camera-centred and out of
    /// depth. `None` for a driveable-ribbon build, which has no art meshes at
    /// all, and for any track that authors no sky.
    pub sky_model: Option<Model>,
    /// The track's `Speedup Pad` geometry, when it authors any.
    ///
    /// Built from the same blob as [`Self::track_model`] and drawn on the same
    /// pipeline, but kept separate because pads belong to no visibility
    /// `section` and the track model's draw calls are what the PVS indexes.
    /// `None` for a driveable-ribbon build and for any track that authors none.
    pub pad_model: Option<Model>,
    /// The track's `Weapon Pad` geometry, when it authors any.
    ///
    /// Everything [`Self::pad_model`]'s note says applies here too; the two are
    /// separate because they are separate gameplay objects. **Nothing consumes
    /// a weapon pad yet** - it is drawn and no more.
    pub weapon_pad_model: Option<Model>,
    /// The track's authored visibility partition, when it decoded.
    ///
    /// `None` when the track declares no `section` nodes - a driveable-ribbon
    /// build has no art meshes to place, and Pure tracks do not use Pulse's
    /// class numbering at all. The first tier is then skipped.
    pub visibility: Option<TrackVisibility>,
    /// The trail ribbon's noise texture off the disc, when it decodes.
    pub noise: Option<FlareTexture>,
    /// The ribbon's blend, when the title **authors** one rather than leaving
    /// it to the recovered PSP preset.
    ///
    /// `None` on Pulse and Pure, where `exhaust::TRAIL_BLEND` is what
    /// `Trail_BuildStateList` records and there is nothing on the disc to
    /// override it with. `Some` on Wipeout HD, whose ribbon material carries
    /// its own factor pair - `SrcAlpha`/`One`, which is the same additive
    /// equation the PSP preset and the PS2's GS register write both land on,
    /// from a third independent source.
    pub trail_blend: Option<wgpu::BlendState>,
    /// The ribbon's own coverage texture, for a title whose material names two.
    ///
    /// `None` on Pulse, Pure and the PS2 port, which name one - and on a
    /// Wipeout HD source whose first texture would not decode, which the load
    /// report says rather than leaving the ribbon quietly shaped by the PSP
    /// preset. See `assets::trail_texture`.
    pub trail_shape: Option<FlareTexture>,
    /// The engine-flare texture off the disc, when it decodes.
    ///
    /// `None` falls back to [`Exhaust`]'s procedural glow, and the load report
    /// says so - it is not a silent substitution.
    pub flare: Option<FlareTexture>,
    /// The Cannon round's bolt streak and muzzle flash textures off the
    /// disc, when each decodes - `Data\Weapons\Textures\Cannon_bolt.mip` and
    /// `Cannon_muzzle_flash.mip` on Pulse (see
    /// `crate::CANNON_BOLT_TEXTURE_ENTRY`), the title's own
    /// `oag_title::weapons::CannonLook` entries elsewhere - and that look
    /// itself. One tuple field rather than three for the same reason
    /// `weapon_models::cannon_quads` returns them unnamed - see that
    /// function's own doc. A `None` texture falls back to
    /// [`FlareTexture::placeholder`] in `Scene::new`, and the load report
    /// says so; every entry resolves on a real disc, so a `None` here is a
    /// decode failure or a missing archive set, not an unauthored asset.
    pub cannon_quad_textures: crate::CannonAssets,
    /// The ghost ship's static, `WeaponModels::ghost_static`,
    /// when it decodes. `None` draws the ghost without its third pass, and
    /// the load report says why.
    pub ghost_static: Option<FlareTexture>,
    /// The LeachBeam ribbon's own texture, off the disc, when it decodes -
    /// see `assets::leach_beam_texture`.
    ///
    /// `None` draws no ribbon body at all rather than a stand-in, on the same
    /// "never invent what the assets author" terms every optional asset here
    /// follows - see `oag_fx::beam`.
    pub leach_beam_texture: Option<FlareTexture>,
    /// The magstrip arc wake's two textures and the Rocket smoke's texture and
    /// ramp, each where the title builds it and it decodes - see
    /// [`crate::rocket_smoke::RibbonTextures`].
    pub ribbon_textures: crate::rocket_smoke::RibbonTextures,
    /// Both shadow tiers' per-slot assets and the maps' resolution. Loaded whatever
    /// `[render_profiles.<title>] shadows` says, so moving that row applies live. See
    /// [`crate::shadow::Assets`].
    pub shadows: crate::shadow::Assets,
    /// The Wipeout 2048 campaign event this load resolved, if
    /// [`load_event`](super::load_event) is what built it - `None` for every
    /// other title and for an ordinary `--race`/menu launch on 2048 itself.
    ///
    /// Carried on [`Loaded`] rather than threaded through `Session` the way
    /// `campaign_cell` is (Pulse/HD's own grid launch): `load_event` already
    /// has the parsed `SP.xml` `Document` in hand to resolve
    /// [`Campaign2048Progress::objectives`], and a second `Document` parse
    /// at grading time would cost an archive read `RaceStage::observation`
    /// has no business making. See `crates/raceplay/src/load/campaign.rs`.
    pub campaign_2048_event: Option<Campaign2048Progress>,
    /// The road spans a Quake ripples, placed on the course - see
    /// `oag_render::ripple`. Empty everywhere but Pulse off a PSP disc, the one
    /// source the ripple was read from.
    pub ripples: super::Ripples,
    /// Lines worth printing once, describing what was found.
    pub report: Vec<String>,
}

/// What [`load`] returns in a build without the `view` feature: the
/// simulation's half of a race and what names it, nothing drawn or played.
/// Each field is documented on the full struct's field of the same name.
#[cfg(not(feature = "view"))]
#[derive(Debug)]
pub struct Loaded {
    pub setup: SimSetup,
    /// Empty: a build without the `view` feature draws nothing.
    pub view: ViewSetup,
    pub modules: crate::extensions::Prepared,
    pub title: &'static oag_title::Title,
    /// The archive entry of the circuit the race was built on: what `Options::track` named, or
    /// the title's default, or the first circuit the source ships when the default is not in
    /// it. A Zone race carries the zone variant. `foreign:<id>` for a foreign track.
    pub circuit: String,
    pub platform: oag_disc::Platform,
    pub report: Vec<String>,
}

/// What [`load_event`](super::load_event) resolved for `RaceStage` to
/// grade the race against, once it finishes - see [`Loaded::campaign_2048_event`].
#[derive(Debug, Clone)]
pub struct Campaign2048Progress {
    /// The `SP.xml` instance name, the key `oag_game::records::Store::record_campaign`
    /// persists a medal under, as Pulse/HD's launch does a cell's name.
    pub name: String,
    /// The event's own pass/elite bars, when it authors them - `None` for a
    /// Speed Lap or a generic template event, which never carries a medal
    /// but still opens whatever names it as a prerequisite once finished.
    /// See `oag_2048::campaign::event_objectives`.
    pub objectives: Option<oag_2048::campaign::EventObjectives>,
    /// The event's opponent AI over the title's defaults; `None` keeps this project's driver.
    pub ai: Option<oag_title::original_driver::EventAi>,
    /// The disc's `XPAwards.xml`, read where `SP.xml` is; `None` when the package lacks it.
    pub xp_awards: Option<oag_tables::xp::Awards>,
    /// The title's own `XPRanks.xml`, which ranks the profile's total on the end-of-race page.
    pub xp_ranks: Option<oag_tables::xp::Ranks>,
    /// Which `XPAwards.xml` column this event's kind pays: race and Speed Lap events the `race`
    /// column, Elimination `elim`, Zone `zone`. **Chosen, not measured**: the original's mode to
    /// column map is unread.
    pub xp_column: oag_tables::xp::Column,
}

/// [`SimSetup::headless`]: the disc-free minimum, for the headless-sim binary.
mod headless;

/// [`ViewSetup`]: what only the picture and the sound of a race are built from.
mod view_setup;
pub use view_setup::ViewSetup;

/// [`Sighting`]: the reticle's grids and law.
mod sighting;
pub use sighting::Sighting;

/// `weapons_on` for [`Options`] and [`SimSetup`].
mod weapons;
