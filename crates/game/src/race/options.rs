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
    /// Directories to look in for [downloadable content](crate::dlc), mounted
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
    /// The team ids the *opponents* may fly, in the caller's own order.
    ///
    /// The caller supplies them because they come off the player's own disc -
    /// `catalogue::all_teams` over the plugin definition and any mounted DLC
    /// pack - and `load` may not invent a team id any more than it may invent a
    /// path. **Empty means the whole grid wears the player's hull**, which is
    /// what this engine did before liveries and what a source with no readable
    /// definition still gets.
    ///
    /// Which of them ends up in which slot is [`crate::livery::teams_for_slots`],
    /// and is this project's rule rather than the original's.
    pub opponent_teams: Vec<String>,
    /// Speed class the handling parameters are read for.
    pub class: SpeedClass,
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
    /// Whether ship and track models draw every child of an authored
    /// `LodGroup`, or only the higher-detail first one. See [`mesh::Lod`].
    pub lod: mesh::Lod,
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
    /// automatic while `oag_gameplay::pickup::IMPLEMENTED` held one weapon and
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

/// Where [`Options::pose`] puts the craft.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum PoseRequest {
    /// A position plus a yaw off the track's own direction there; pitch and
    /// roll come from the nearest spline sample. What `--pose X,Y,Z[,YAW]`
    /// always meant - see
    /// [`oag_gameplay::spawn::Pose::from_position_on_sample`].
    SplineAligned {
        /// Where to put the craft, world space.
        position: Vec3,
        /// Radians off the spline tangent at that point.
        yaw: f32,
    },
    /// A full pose, applied verbatim - a captured trace row's position and
    /// basis, nothing recomputed from the spline.
    Exact(Pose),
}

/// A camera pose imposed from outside, replacing the chase camera.
///
/// The one seam is [`Race::view`]: [`Race::camera_position`] derives from the
/// view matrix, so the PVS culling eye and the fog eye follow the override
/// without knowing it exists.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CameraOverride {
    /// The camera eye, world space.
    pub eye: Vec3,
    /// The camera's orientation, in the same convention as the ship body's:
    /// `-Z` is the look direction, `Y` is up. What
    /// [`oag_trace::replay::camera_orientation_of`] produces.
    pub orientation: Quat,
    /// Replace the disc's authored fov, in **vertical degrees**, with this
    /// value. `None` keeps the authored one.
    ///
    /// This was the calibration knob for settling the unit, and the unit is now
    /// settled (confidence 94). What it is *for* has changed rather than gone
    /// away: a matched-pose render still needs it, because
    /// [`oag_gameplay::spawn::Ship::place_at`] resets the body, so a posed craft
    /// has zero velocity and never gets the original's speed-dependent widen.
    /// Pass the fov computed from the captured tick's own forward velocity.
    pub fov_deg: Option<f32>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            source: crate::source::DEFAULT_IMAGE.to_string(),
            difficulty: oag_ai::Difficulty::default(),
            // Empty rather than the search path: a default that read the
            // filesystem would make two runs of the same test differ by what
            // the developer happens to have downloaded. The composition root
            // fills this in from `source::resolve_dlc`.
            dlc: Vec::new(),
            track: None,
            team: None,
            // Empty for the same reason `dlc` is: the ids come off the
            // player's own disc, and `load` may not invent one. The
            // composition root fills it from the catalogue.
            opponent_teams: Vec::new(),
            class: SpeedClass::Venom,
            mode: Mode::default(),
            ribbon: false,
            collision: false,
            lod: mesh::Lod::Both,
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
    /// How good the opponents are. See [`Options::difficulty`].
    pub difficulty: oag_ai::Difficulty,
    /// The speed class the race is run in.
    ///
    /// Most of what the class decides is already resolved by the time a `Setup`
    /// exists - the handling block, the gravity scale and the speed-pad tunables
    /// are all per-class and all baked in above. It is carried anyway because
    /// [`Self::weapons`]' pickup odds are indexed by class *inside* a table this
    /// keeps whole, and resolving that at load would throw away the other
    /// classes' rows for no gain.
    pub class: SpeedClass,
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
    pub zone: Option<oag_formats::handling::Zone>,
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
    /// The chase camera's seven values, from `<ExternalCameraFar>`.
    pub chase: ChaseParams,
    /// The same seven from `<ExternalCameraClose>`: the nearer of the two
    /// external views [`crate::display::CameraView`] cycles.
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
    /// node as it rides the hull. Empty means the model authors none, and
    /// the burst falls back to anchoring at the contact point itself.
    pub collision_fx: Vec<Vec3>,
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
    /// [`crate::audio::Audio`]. See `docs/architecture/adr/0018-audio-mixer-architecture.md`.
    pub sounds: crate::audio::sfx::Banks,
    /// Zone mode's milestone announcer, decoded from this title's own
    /// `oag_title::ZoneAnnouncer` when it has one.
    ///
    /// Empty on every other mode and on a title with no recovered ladder -
    /// [`Self::sounds`]'s rule applies unchanged: a milestone that will not
    /// resolve plays nothing rather than a substitute.
    pub announcer: crate::audio::sfx::Announcer,
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
    pub speedup_pads: Vec<oag_formats::pads::PadVolume>,
    /// The track's weapon pads, as trigger volumes.
    ///
    /// Everything [`Self::speedup_pads`] says applies, one class over. Consumed
    /// by [`Race::test_weapon_pads`] in the one mode that arms them, and decoded
    /// on every mode regardless, which is what lets
    /// `the_weapon_pads_are_drawn_where_they_trigger` check the geometry against
    /// them. Two independent decodes of the same nodes agreeing is worth more
    /// than either alone.
    pub weapon_pads: Vec<oag_formats::pads::PadVolume>,
    /// The weapon table for this race, out of `WeaponStats_Race.xml`.
    ///
    /// `None` when the file is absent or unreadable, which means a pad hands
    /// nothing out rather than handing out an invented weapon - the same choice
    /// [`Setup::speedup_pads`]' tunables make. See `docs/formats/weapon-stats.md`.
    pub weapons: Option<oag_formats::weapons::WeaponStats>,
    /// Seconds a weapon pad is inert for after it is crossed, for this race's
    /// speed class.
    ///
    /// `<WeaponPad refresh_time>` out of the same `<GlobalClass>` block the
    /// speed-pad tunables come from, stamped onto the pad by the original's
    /// `WeaponPads_TestCraft` - see [`oag_formats::handling::WeaponPad`], which
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
    /// The HUD's layout, atlas, fonts and strings.
    pub hud: crate::hud::Assets,
    /// What to draw for the track.
    pub track_model: Model,
    /// One hull, plume, nozzle and spark-anchor set per grid slot, each off
    /// its own team's directory. Slot 0 is the player's.
    ///
    /// Per slot rather than one shared model since 2026-08-15: the eight teams
    /// the disc declares are eight *different* hulls (845 to 1,497 triangles),
    /// not one hull repainted, so a shared model was visibly one team's ship
    /// eight times over. See [`crate::livery`], which also records which half
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
    /// The sphere a fired Shield shows from **inside** the cockpit, drawn
    /// instead of the per-team shell when the camera is in the craft.
    ///
    /// One entry for the whole game rather than one per team, which is the
    /// original's own arrangement: `ShipShield_Construct` (`0x0885db38`) builds
    /// the shell's name from the team's directory and this one from a literal
    /// `Data\Weapons`. `None` on a source that does not carry it, on the same
    /// terms as [`Self::rocket_model`].
    pub shield_cockpit: Option<Model>,
    pub liveries: Vec<Livery>,
    /// The collision soup, if [`Options::collision`] asked for it.
    pub collision_model: Option<Model>,
    /// The track's authored `fogCube` volumes, for [`Scene`] to sample per
    /// frame at the camera.
    pub fog_volumes: Vec<oag_formats::fog::FogVolume>,
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
    /// chain - see `oag_render::post::hd_bloom`.
    pub hd_bloom: Option<oag_render::post::hd_bloom::Params>,
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
    /// Lines worth printing once, describing what was found.
    pub report: Vec<String>,
}
