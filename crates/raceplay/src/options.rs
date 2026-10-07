//! What a race is asked to load and what comes back: [`Options`] in, [`Setup`]
//! and [`Loaded`] out, plus the two overrides a capture drives them with.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::*;

/// What to load, and which of it to draw.
#[derive(Debug, Clone)]
pub struct Options {
    /// A disc image, or a directory extracted with `oag-unpack`.
    pub source: String,
    /// Where the craft, HUD and grid roster load from, or `None` for
    /// `source` - today's behaviour, unchanged.
    ///
    /// **The one thing a Race Remix adds.** `source` still supplies the track
    /// and everything that goes with it - collision, environment, pads,
    /// weapon tuning; `craft_source`, when it names a different release,
    /// supplies the livery, HUD, exhaust/flare and boost plume instead. See
    /// [`oag_source::remix::Remix`].
    pub craft_source: Option<String>,
    /// Directories to look in for downloadable content (`oag_source::dlc`), mounted
    /// behind `source`'s own archives.
    ///
    /// Independent of which release `source` is, on purpose: the original tied
    /// a pack to its own territory's disc and this does not. See
    /// `docs/formats/dlc-pack.md`.
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
    /// Team id, which selects both the handling stats and the model, or `None`
    /// for the source's own.
    ///
    /// **`None` rather than a constant, for the reason [`Self::track`] gives
    /// and one more of its own.** `--team`'s help has promised since it landed
    /// that leaving it out takes the value "from the source's own title, the
    /// same way `--track` does" - and until 2026-08-18 the track half of that
    /// refactor had landed and the team half had not, so every title was served
    /// `oag_pulse::race::DEFAULT_TEAM`. It worked on HD only because a PSARC
    /// folds case: `Data\Ships\Assegai\handlingstats.xml` normalises onto the
    /// `assegai` the manifest actually stores. Finding S3 of that day's review,
    /// and the reason `oag_title::RaceDefaults::team` existed with no reader.
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
    /// label, its roster membership - stays exactly `team`'s. See
    /// `oag_title::race::HullVariant`.
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
    /// The team ids the *opponents* may fly, in the caller's own order.
    ///
    /// The caller supplies them because they come off the player's own disc -
    /// `catalogue::all_teams` over the plugin definition and any mounted DLC
    /// pack - and `load` may not invent a team id any more than it may invent a
    /// path. **Empty means the whole grid wears the player's hull**, which is
    /// what this engine did before liveries and what a source with no readable
    /// definition still gets.
    ///
    /// Which of them ends up in which slot is [`oag_livery::teams_for_slots`],
    /// Pulse's own roster draw off the race seed.
    pub opponent_teams: Vec<String>,
    /// A per-AI-slot team override, index `0` being grid slot `1` (slot `0`
    /// is always the player) - **what authored data replaces
    /// [`oag_livery::teams_for_slots`]'s own draw with**, where a caller has one to offer.
    ///
    /// `None` at an index leaves that slot exactly what
    /// [`oag_livery::teams_for_slots`] would already give it; an index
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
    /// Whether a Wipeout 2048 Zone race draws its own `trackZone.rcsmodel`.
    ///
    /// Off by default: that model's road shader (`fc01_dummy`) is unread, so
    /// a Zone race on it draws no road at all. Until the shader is read, Zone
    /// races the ordinary circuit model; the Zone ground-truth test turns
    /// this on to keep the loader honest.
    pub zone_model: bool,
    /// Whether a Pulse hull's `0x2000` extra pass is built at all
    /// ([`oag_render::shine`]). `true` is the game; `false` is `--no-hull-shine`,
    /// the headless way to take the pass out of a frame and so measure what it
    /// adds.
    pub hull_shine: bool,
    /// Whether each Pulse craft's `shipwreck.vex` is loaded and swapped in once
    /// the craft is out of the race. `false` is `--no-hull-wreck`.
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
    /// Draw the driveable ribbon instead of the track's art meshes.
    ///
    /// **A development view, not a style.** The ribbon is the geometry the
    /// simulation spawns on and queries, so ship-plus-ribbon shows directly
    /// whether the ship is where the physics thinks it is. Default is off, which
    /// means a race draws the map.
    pub ribbon: bool,
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
    /// Play the trail-hit sparks on the player regardless of the trigger.
    ///
    /// A verification aid for the draw path alone - see the `--trail-sparks`
    /// flag, which is the only thing that sets it, and
    /// `Race::advance_trail_hits` for why the two need separating.
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
    /// It changes nothing about a real race: every caller that does not set it
    /// gets [`SEED`], which is the fixed value the field replaced.
    pub seed: Option<u64>,
    /// Put the craft here instead of on its grid slot.
    ///
    /// **A capture aid, not a spawn.** The point is that two circuits, or a
    /// frame of ours and a frame of the original, can be photographed from the
    /// same place instead of by running the same number of ticks and hoping.
    /// `None` spawns normally.
    pub pose: Option<PoseRequest>,
    /// Render from this camera instead of the chase camera.
    ///
    /// The other half of the same capture aid: a ship pose replicates *what*
    /// the original showed, and only a replicated camera replicates *how it was
    /// framed*. `None` uses the chase camera as always.
    pub camera: Option<CameraOverride>,
}

/// The two capture aids [`Options::pose`] and [`Options::camera`] carry.
mod capture;
pub use capture::{CameraOverride, PoseRequest};

impl Default for Options {
    fn default() -> Self {
        Self {
            source: oag_source::source::DEFAULT_IMAGE.to_string(),
            craft_source: None,
            difficulty: oag_ai::Difficulty::default(),
            // Empty rather than the search path: a default that read the
            // filesystem would make two runs of the same test differ by what
            // the developer happens to have downloaded. The composition root
            // fills this in from `source::resolve_dlc`.
            dlc: Vec::new(),
            track: None,
            team: None,
            hull_variant: None,
            skin: None,
            // Empty for the same reason `dlc` is: the ids come off the
            // player's own disc, and `load` may not invent one. The
            // composition root fills it from the catalogue.
            opponent_teams: Vec::new(),
            // Empty is "no override anywhere" - see the field's own doc
            // comment. Only `race::load_event` ever sets this.
            grid_teams: Vec::new(),
            zone_model: false,
            hull_shine: true,
            hull_wreck: true,
            track_shine: true,
            // The rung every measured title shares, named rather than
            // defaulted from an enum: a title whose ladder was never read must
            // not silently inherit one that was.
            class: SpeedClass::Venom.as_str().to_string(),
            mode: Mode::default(),
            language: None,
            eliminator_kill_target: None,
            laps_override: None,
            weapons_override: None,
            zone_stage: None,
            ribbon: false,
            collision: false,
            opponents: false,
            trail_sparks: false,
            seed: None,
            pose: None,
            camera: None,
        }
    }
}

/// Everything a race needs with no GPU anywhere in sight.
///
/// Split out from [`Loaded`] so a test can build one by hand, or take one off a
/// disc, and run a whole race against it on a machine with no graphics driver.
#[derive(Debug, Clone)]
pub struct Setup {
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
    /// The speed class the race is run in, spelled the way the disc spells it.
    ///
    /// Most of what the class decides is already resolved by the time a `Setup`
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
    /// See [`Options::trail_sparks`] - a draw-path verification aid.
    pub trail_sparks: bool,
    /// The world generator's seed, already resolved from [`Options::seed`].
    pub seed: u64,
    /// The decoded spline graph, as the file has it.
    pub ai: AiTrack,
    /// The spline resampled for locating a ship, and for placing it.
    pub spline: Spline,
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
    /// Wipeout HD's trail: `Some` with the per-slot blue-to-red colour mix
    /// when the title authors its ribbon as HD's two-texture template,
    /// `None` on every other source, where the ribbon is the PSP preset.
    ///
    /// HD patches the ribbon material's mix parameter to 1.0 exactly when a
    /// craft's model-variant name is `concept1`, `nitro`, `detonator` or
    /// `chrome_c1` (`strcasecmp` in the flag setter writing `craft+0x7d2c`) -
    /// the Fury skins - which is what turns those trails red where a classic
    /// HD craft's is blue. The loader derives the flag from each slot's team
    /// directory: `*_c1` is the concept skin, `*_n1` the nitro one. See
    /// `docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md`.
    pub hd_trail: Option<[f32; oag_gameplay::MAX_SHIPS]>,
    /// Which three colours a fired shield's shell moves between.
    ///
    /// A title fact, not a per-craft one - every shield on one source shares
    /// a palette. Resolved from `craft_title`, the same title
    /// [`assets::shield_entry_names`] and [`assets::boost_entry_name`]
    /// already key their own per-title path off, so a race whose craft model
    /// comes from HD draws HD's own colours rather than Pulse's borrowed
    /// ones. See [`oag_render::shield::Palette`]'s own doc comment for what
    /// each title's palette carries and which parts are measured.
    pub shield_palette: oag_render::shield::Palette,
    /// Whether a Plasma detonation plays Wipeout HD's own three-model scale
    /// ease rather than Pulse's baked anim-time scrub.
    ///
    /// A title fact, the same footing [`Self::shield_palette`] is on:
    /// `craft_title.weapon_models.plasma_blast_hd.is_some()`. The two
    /// mechanisms are read off different executables and do not unify - see
    /// `blast_models`'s own module doc comment - so this is what
    /// `Race::plasma_blast_draws` and `Race::advance_plasma_blast_models`
    /// branch on rather than re-deriving which title is live from anywhere
    /// else.
    pub hd_plasma_blast: bool,
    /// Whether a Bomb detonation plays Wipeout HD's own blast object (four
    /// models, eleven instances, one per-tick update) rather than Pulse's two
    /// eased models: `craft_title.weapon_models.bomb_blast_hd.is_some()`. See
    /// `bomb_blast::hd`.
    pub hd_bomb_blast: bool,
    /// Whether a laid Mine and Bomb take Pulse's own measured poses - the
    /// Mine spun and scaled by `Mine_PoseNode`, the Bomb squared to the world
    /// by `Bomb_Init` - rather than the frozen craft pose. A title fact:
    /// `craft_title.looks.laid_pose`. See
    /// `weapons::visuals::laid`.
    pub pulse_laid_pose: bool,
    /// Whether the grid is laid out the way Pulse PSP's `Race_ComputeGridLayout` does: a walk
    /// of the located curve with the original's scaled record and its edge-chord heading
    /// (`oag_gameplay::grid_walk`), rather than a walk of resampled samples that take the
    /// node's heading or the sample tangent (`oag_gameplay::orientation_on_sample`, still
    /// the fallback where the walk refuses a node). Pulse off a PSP disc only, the one grid
    /// layout function read and measured against the original:
    /// `docs/physics/grid-state.md`.
    pub grid_frame_from_sample: bool,
    /// Whether weapon detonations start `ScreenFlash_Start`'s full-screen
    /// wash - Pulse off a PSP disc only, the one executable its consumer is
    /// read off. See `oag_fx::flash`.
    pub screen_flash: bool,
    /// How this title staggers `WO_WEAPON_ABSORB` over a hull, or `None` on a
    /// title whose absorb path is unread. See `race::absorb`.
    pub absorb_burst: Option<oag_title::Burst>,
    /// Each slot's absorb locators in its hull's model space, in the order
    /// the original collects them: the `Ship Collision Fx` set on Pulse and
    /// Pure, the `absorb` set on HD.
    pub absorb_anchors: Vec<Vec<Vec3>>,
    /// Each slot's `Ship Collision Fx` locators for the hit sparks a landed
    /// weapon hit throws, or none on a title whose path is unread. See
    /// `race::hit_sparks`.
    pub hit_spark_anchors: Vec<Vec<oag_livery::SparkAnchor>>,
    /// Each slot's `Ship Collision Fx` locators for the spark HD's Cannon
    /// throws on the craft it hits, or none on every other title. See
    /// `race::hit_sparks::throw_weapon_spark`.
    pub weapon_spark_anchors: Vec<Vec<oag_livery::SparkAnchor>>,
    /// Each slot's wreck `Ship Collision Fx` locators, where its destruction
    /// effects spawn - see `race::wreck_fx`.
    pub wreck_anchors: Vec<Vec<oag_livery::SparkAnchor>>,
    /// Each slot's `arc_anchor_point` in its hull's model space, when the title
    /// builds the HD-lineage magstrip arc wake (`Some`); a slot whose hull
    /// authors none is `None` and draws no wake. `None` on every other title.
    /// See `race::magstrip_wake`.
    pub magstrip_wake: Option<[Option<Mat4>; oag_gameplay::MAX_SHIPS]>,
    /// Whether the over-strip effect is the title's `.pob` rather than the arc
    /// wake: see `oag_title::weapons::WeaponModels::magstrip_pob`.
    pub magstrip_pob: bool,
    /// The circuit's authored cameras, where the player's camera stands once
    /// their craft is destroyed - see `race::destroy_camera`. Empty off Pulse.
    pub destroy_stations: Vec<oag_render::camera::destroy::Station>,
    /// The circuit's pre-race flyby, dormant until a windowed session begins it - see
    /// `race::intro_camera`. `None` where the title plays none (`oag_title::pre_race`) and on a circuit with no
    /// `start_grid.vex`.
    pub intro_camera: Option<(
        oag_vex::grid_camera::GridCamera,
        &'static oag_title::pre_race::PreRace,
    )>,
    /// The team id each grid slot flies (`Feisar`), for the HUD's per-craft
    /// rows - the Eliminator's kill column. Ids, not names: the string table
    /// turns one into the name a player reads at draw time. Empty where a
    /// caller builds a `Setup` by hand.
    pub slot_teams: Vec<String>,
    /// Every collidable triangle of the track.
    pub collision: CollisionWorld,
    /// The force law's parameter set for one team in one speed class.
    pub handling: Handling,
    /// How far and how fast the airbrake flaps move, from the same document.
    ///
    /// Separate from [`Self::handling`] because it is per *team* rather than
    /// per speed class, and because no force term reads it - see
    /// `oag_gameplay::airbrake_graphics_for`.
    pub airbrake_graphics: oag_gameplay::AirbrakeGraphics,
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
    /// The chase camera's seven values, from `<ExternalCameraFar>`.
    pub chase: ChaseParams,
    /// The same seven from `<ExternalCameraClose>`: the nearer of the two
    /// external views [`oag_display::display::CameraView`] cycles.
    pub chase_close: ChaseParams,
    /// The cockpit view's five values, from `<InternalCamera>`.
    pub internal: InternalParams,
    /// Each grid slot's `engine_flare` locator, in **that slot's own hull's**
    /// model space. Slot 0 is the player's.
    ///
    /// `None` means that hull carries no `Engine Flare` node, and the exhaust
    /// is then not drawn rather than guessed at. Every team whose `Ship.vex`
    /// resolves by name has **exactly one**, a direct child of `world` - see
    /// `docs/ghidra/functions/psp-pulse-usa/exhaust.md`. One nozzle, centred,
    /// not one per visible engine.
    ///
    /// **Per slot since liveries landed.** The eight hulls are different
    /// models, so one team's locator carried onto another's craft puts the
    /// flare inside the fuselage.
    pub nozzles: Vec<Option<Vec3>>,
    /// Wipeout HD's engine light per slot - `EngineLightData.xml` and the
    /// flare locator's Z axis - or `None` on every other title and on a
    /// craft that ships neither. Indexed like [`Self::nozzles`], which
    /// carries the locator's position. See [`crate::engine_light`].
    pub engine_lights: Vec<Option<oag_livery::engine_light::EngineLight>>,
    /// Whether the circuit's `.envsettings` leaves the SPU vertex lights on -
    /// `"Lighting.Enable spu vertex lights"`, on by default and authored off
    /// on two HD circuits. See `race::load::engine_light`.
    pub spu_vertex_lights: bool,
    /// Each slot's `Ship Collision Fx` locators, in that hull's model space.
    ///
    /// The disc's own hull spark anchors - the set the original picks the
    /// nearest of for a collision, and this engine's stand-in for the ten
    /// nodes `Trail_HitShipEffect` parents a trail hit to. Per slot because
    /// the hulls are different models.
    pub spark_anchors: Vec<Vec<Vec3>>,
    /// The `Ship Collision Fx` locators, in the ship model's own space.
    ///
    /// The original attaches up to 10 and `Ship_DispatchCollisionFx`
    /// (`docs/ghidra/functions/psp-pulse-usa/contact-response.md`) triggers the
    /// one **nearest the contact**; the spark burst then emits from that
    /// node, and along its authored `+Y`, as it rides the hull. Empty means
    /// the model authors none, and the burst falls back to anchoring at the
    /// contact point itself, aimed along world up.
    pub collision_fx: Vec<oag_livery::SparkAnchor>,
    /// The circuit's own placed effects - see `race::scenery_fx`. Empty off Pulse PSP.
    pub scenery_fx: super::scenery_fx::Plan,
    /// Every [`RACE_EFFECTS`] entry that loaded, parsed from the disc's own
    /// `Data\Psys\*.POB`.
    ///
    /// An effect missing from here - no such archive entry, or a blob that
    /// would not parse - is **not drawn** rather than approximated: every
    /// number that shapes one lives in its file, so there is nothing left to
    /// fall back to and a stand-in would be this engine's invention rather
    /// than the game's. Headless tests that build a `Race` by hand leave the
    /// library empty, which is the same path.
    pub effects: psys::Library,
    /// Each [`Trigger`] resolved against [`Self::effects`] once at load, off the
    /// craft's title's table. Empty where nothing loaded.
    pub handles: super::EffectHandles,
    /// The decoded sound cues, out of `Data\Sound\*.bnk`.
    ///
    /// The audio counterpart of [`Self::effects`] and held to the same rule: a
    /// cue that would not resolve is **silent** rather than substituted, and a
    /// source that ships no sound banks at all - Wipeout Pure - loads an empty
    /// library and races without effects. Headless tests that build a `Race` by
    /// hand leave it empty, which is the same path.
    ///
    /// Decoded PCM and nothing device-shaped, so this belongs beside the
    /// simulation the way the particle library does; the mixer, the voices and
    /// the choice of which alternate sounds all live in
    /// [`oag_sound::Audio`]. See `docs/architecture/adr/0018-audio-mixer-architecture.md`.
    pub sounds: oag_sound::sfx::Banks,
    /// The circuit's **own** authored sound emitters, the ambience that belongs
    /// to the track rather than to any craft.
    ///
    /// Beside [`Self::sounds`] for the same reason it is here: decoded data
    /// with nothing device-shaped in it. Empty on a Zone circuit, which authors
    /// none of the three `.vex` audio classes at all, and on any title whose
    /// circuits have never been swept for them.
    pub track_emitters: oag_sound::sfx::TrackEmitters,
    /// Zone mode's milestone announcer, decoded from this title's own
    /// `oag_title::ZoneAnnouncer` when it has one.
    ///
    /// Empty on every other mode and on a title with no recovered ladder -
    /// [`Self::sounds`]'s rule applies unchanged: a milestone that will not
    /// resolve plays nothing rather than a substitute.
    pub announcer: oag_sound::sfx::Announcer,
    /// Zone mode's speed-class announcer, [`Self::announcer`]'s sibling,
    /// decoded from this title's own `oag_title::ZoneClassAnnouncer` when it
    /// has one.
    pub class_announcer: oag_sound::sfx::ClassAnnouncer,
    /// This title's own zone-number-to-speed-class ladder, when it is
    /// recovered - the same [`oag_title::ZoneStages`]
    /// [`crate::zone_grade::ZoneGrade`] carries, copied here so
    /// [`Self::class_announcer`] can be triggered off the same edge without
    /// reaching into the render-facing scene state to get it. `None` on every
    /// title but HD/Fury, which is `oag_title::ZoneStages`' own standing today.
    pub zone_stages: Option<&'static oag_title::ZoneStages>,
    /// Whether this title's `ready` and `go` start-of-race voice has been
    /// measured, so the race raises [`oag_sound::sfx::Cue::Ready`] and
    /// [`oag_sound::sfx::Cue::Go`] at the ticks [`crate::countdown`]
    /// pins. Straight from `oag_title::RaceDefaults::countdown_voice` being
    /// `Some`.
    ///
    /// A flag rather than the banks: which bank plays is decoded audio and
    /// rides [`Self::sounds`]; whether the simulation asks for the cue at all
    /// is title data, and must not change with whether a bank decoded.
    pub countdown_voice: bool,
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
    /// by [`Race::test_weapon_pads`] in the one mode that arms them, and decoded
    /// on every mode regardless, which is what lets
    /// `the_weapon_pads_are_drawn_where_they_trigger` check the geometry against
    /// them. Two independent decodes of the same nodes agreeing is worth more
    /// than either alone.
    pub weapon_pads: Vec<oag_vex::pads::PadVolume>,
    /// The weapon table for this race, out of `WeaponStats_Race.xml`.
    ///
    /// `None` when the file is absent or unreadable, which means a pad hands
    /// nothing out rather than handing out an invented weapon - the same choice
    /// [`Setup::speedup_pads`]' tunables make. See `docs/formats/weapon-stats.md`.
    pub weapons: Option<oag_tables::weapons::WeaponStats>,
    /// The odds an opponent fires each weapon at, out of the title's
    /// `WeaponAIstats.xml` - see [`oag_title::weapons::Weapons::ai`].
    ///
    /// `None` runs the race on [`super::FireLaw::Ours`]: the original's law
    /// needs these numbers and nothing invents them.
    pub weapon_ai: Option<oag_tables::weapons::ai::WeaponAiStats>,
    /// Restricts which weapons a `Weapon Pad` may hand out to exactly this
    /// list, or does not restrict at all when empty - the same "empty is no
    /// override" convention [`Options::grid_teams`] uses.
    ///
    /// **Nothing sets this except `race::load::campaign::load_event`**, off
    /// the 2048 campaign event's own `M_WEAPONSET` -
    /// `oag_tables::mjolnir::campaign::WeaponSet::allowed_weapons`, the bits
    /// that project has pinned at confidence >= 70 (see
    /// `docs/formats/2048-campaign.md`'s "The weapon set gate" section).
    /// Every non-2048 race, and a 2048 event whose weapon set decodes to
    /// nothing recognised, draws exactly as it always did - see
    /// [`oag_weapons::pickup::draw`]'s own `allowed` parameter.
    pub allowed_weapons: Vec<oag_tables::weapons::Weapon>,
    /// Seconds a weapon pad is inert for after it is crossed, for this race's
    /// speed class.
    ///
    /// `<WeaponPad refresh_time>` out of the same `<GlobalClass>` block the
    /// speed-pad tunables come from, stamped onto the pad by the original's
    /// `WeaponPads_TestCraft` - see [`oag_tables::handling::WeaponPad`], which
    /// records why it is a debounce rather than a respawn. Zero when the global
    /// file could not be read.
    pub weapon_pad_refresh: f32,
    /// Where [`Options::pose`] asked for the craft to start, already resolved
    /// against the spline. `None` uses the ordinary spawn.
    pub pose_override: Option<Pose>,
    /// [`Options::camera`], carried through to [`Race::view`]. Plain data, so
    /// it rides in the simulation half even though only the renderer reads it.
    pub camera_override: Option<CameraOverride>,
}

/// A [`Setup`] plus the geometry to draw it with.
#[derive(Debug)]
pub struct Loaded {
    /// The simulation half.
    pub setup: Setup,
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
    /// The circuit's own `stats.xml`, for the HUD's `RECORD` readout. `None`
    /// off Pulse's PSP disc, or when the file did not read - the load report
    /// says which.
    pub track_stats: Option<oag_tables::track_stats::TrackStats>,
    /// What to draw for the track.
    pub track_model: Model,
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
    /// Per slot rather than one shared model since 2026-08-15: the eight teams
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
    /// The model a laid Bomb is drawn as: [`BOMB_MODEL_ENTRY`]. `None` on the
    /// same terms.
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
    pub liveries: Vec<Livery>,
    /// The collision soup, if [`Options::collision`] asked for it.
    pub collision_model: Option<Model>,
    /// The track's authored `fogCube` volumes, for [`Scene`] to sample per
    /// frame at the camera.
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
    /// The magstrip arc wake's two textures, `[atlas, contact]`, when the title
    /// builds the class and both decode - see `load::magstrip_wake`. `None`
    /// draws no wake rather than a stand-in.
    pub magstrip_wake_textures: Option<[FlareTexture; 2]>,
    /// One `blob` shadow silhouette per grid slot, slot 0 the player's.
    ///
    /// The disc's own where the source ships one - Wipeout HD's nine
    /// `ambient_shadow.gtf`, one per team - and a generated falloff where it
    /// does not, which is every other title. Loaded whatever
    /// `[render_profiles.<title>] shadows` says, so moving that row applies
    /// live rather than at the next race; the tier costs nothing while it is
    /// `off` because `Scene::render` never uploads a quad. See
    /// [`crate::shadow`] and `oag_render::shadow`.
    pub shadows: Vec<oag_render::shadow::Silhouette>,
    /// One authored shadow hull per grid slot, for the `original` tier, where
    /// the craft's own model carries one.
    ///
    /// `None` where it does not, and the load report says which - a craft with
    /// no `Dynamic Shadow Occluder` casts no `original` shadow rather than
    /// borrowing another team's. See [`crate::shadow::hulls`].
    pub shadow_hulls: Vec<Option<oag_vex::shadow_occluder::Occluder>>,
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

/// What [`load_event`](super::load_event) resolved for `RaceStage` to
/// grade the race against, once it finishes - see [`Loaded::campaign_2048_event`].
#[derive(Debug, Clone)]
pub struct Campaign2048Progress {
    /// The `SP.xml` instance name - the key
    /// `oag_game::records::Store::record_campaign` persists a medal under,
    /// the same way Pulse/HD's own launch persists one under a cell's name.
    pub name: String,
    /// The event's own pass/elite bars, when it authors them - `None` for a
    /// Speed Lap or a generic template event, which never carries a medal
    /// but still opens whatever names it as a prerequisite once finished.
    /// See `oag_2048::campaign::event_objectives`.
    pub objectives: Option<oag_2048::campaign::EventObjectives>,
}

/// [`Setup::headless`]: the disc-free minimum, for the headless-sim binary.
mod headless;

impl Options {
    /// Whether this race runs with weapons: the override when one is set,
    /// otherwise [`Mode::weapons_enabled`]. Every reader that used to ask the
    /// mode asks this, so pads, damage and the load report agree.
    #[must_use]
    pub fn weapons_on(&self) -> bool {
        self.weapons_override
            .unwrap_or_else(|| self.mode.weapons_enabled())
    }
}

impl Setup {
    /// [`Options::weapons_on`], on the half of the options a running race keeps.
    #[must_use]
    pub fn weapons_on(&self) -> bool {
        self.weapons_override
            .unwrap_or_else(|| self.mode.weapons_enabled())
    }
}
