//! [`ViewSetup`]: what only the picture and the sound of a race are built from, beside the
//! [`SimSetup`] the simulation is.

#[cfg(feature = "view")]
use super::*;

/// What only the picture and the sound of a race read: the particle library, the sound
/// banks, the hull locators the effects anchor on, the camera rigs the player can cycle,
/// the drawn reticle's grid. [`Race::start`] takes it beside a [`SimSetup`] and builds the
/// [`RaceView`] out of it; nothing in it reaches [`RaceSim`] or the state hash.
#[cfg(feature = "view")]
#[derive(Debug, Clone)]
pub struct ViewSetup {
    /// See [`Options::trail_sparks`] - a draw-path verification aid.
    pub trail_sparks: bool,
    /// The launch request the options described, resolved, which a recording of the race writes
    /// into its header so the file can rebuild it ([`Options::replay_setup`]). `None` for a
    /// setup built by hand.
    pub request: Option<oag_replay::header::Setup>,
    /// Whether the race is an event of the title's own single-player campaign: 2048's
    /// `SP.xml`, set by [`load_event`](super::load_event) and by nothing else. It is what the HUD
    /// reads as "an event object exists" - [`oag_hud::Readout::campaign_event`]. A DLC circuit
    /// raced as the HD or Fury Campaign, or any `--track` race, has none.
    pub campaign_event: bool,
    /// A campaign event's pass score, where it is below the score the race ends on: the target
    /// the HUD shows until it is met. Set by [`load_event`](super::load_event).
    pub score_pass: Option<u32>,
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
    /// Whether the PSP-preset ribbon draws by the PS2's law
    /// ([`oag_fx::exhaust::TrailLaw::Baked`]): the title's `ps2_trail` rule on the
    /// disc the race loaded from. Read off `Looks`, never off the title's name.
    pub ps2_trail: bool,
    /// Which three colours a fired shield's shell moves between.
    ///
    /// A title fact, not a per-craft one - every shield on one source shares
    /// a palette. Resolved from `craft_title`, the same title
    /// [`oag_livery::entry::shield_entry_names`] and [`oag_livery::entry::boost_entry_name`]
    /// already key their own per-title path off, so a race whose craft model
    /// comes from HD draws HD's own colours rather than Pulse's borrowed
    /// ones. See [`oag_render::shield::Palette`]'s own doc comment for what
    /// each title's palette carries and which parts are measured.
    pub shield_palette: oag_render::shield::Palette,
    /// Each slot's authored settled shield colour (`Livery::shield_colour`),
    /// which replaces `shield_palette.target` for that slot's shell. Empty or
    /// `None` where the craft's shell authors none.
    pub shield_targets: Vec<Option<[f32; 4]>>,
    /// Whether a Plasma detonation plays HD's three-model scale ease rather than
    /// Pulse's baked anim-time scrub: `plasma_blast_hd.is_some()`. The two are
    /// read off different executables and do not unify (`blast_models`).
    pub hd_plasma_blast: bool,
    /// The constants Pulse's and Pure's shared three-model blast runs:
    /// `plasma_blast_pulse`'s law, `None` where the title has no such blast.
    pub plasma_blast_law: Option<oag_title::weapons::PlasmaBlastLaw>,
    /// Pure's Disruptor explosion halo, `None` where there is none.
    pub disruptor_halo_law: Option<oag_title::weapons::DisruptorHalo>,
    /// Pure's on-craft Disruptor effect, `None` where there is none.
    pub disruptor_craft_law: Option<oag_title::weapons::DisruptorCraftFx>,
    /// Whether a Bomb detonation plays HD's own blast object rather than Pulse's
    /// two eased models: `bomb_blast_hd.is_some()`. See `bomb_blast::hd`.
    pub hd_bomb_blast: bool,
    /// Whether a Bomb detonation's shockwave fades while it widens:
    /// `bomb_blast_pulse.shockwave_fades`. Pulse's does, Pure's does not.
    pub bomb_shockwave_fades: bool,
    /// Whether a Missile hit plays HD's explosion: `missile_blast_hd.is_some()`.
    pub hd_missile_blast: bool,
    /// Whether the Detonator cannon submits its point lights: `Detonator::cannon_lights`.
    pub detonator_cannon_lights: bool,
    /// Whether the EMP wave submits its point light: `Detonator::emp_light`.
    pub detonator_emp_light: bool,
    /// How fast a laid Detonator mine turns, radians per second of its age: `Detonator::mine_spin`,
    /// `None` off Detonator.
    pub detonator_mine_spin: Option<f32>,
    /// The craft's cannon gauge in a Detonator race: `Detonator::ammo_gauge`, `None` off Detonator.
    pub detonator_ammo_gauge: Option<oag_title::DetonatorAmmoGauge>,
    /// The craft's plasma shells in a Detonator race: `Detonator::hull_charge`, `None` off Detonator.
    pub detonator_hull_charge: Option<oag_title::DetonatorHullCharge>,
    /// Whether a laid Mine and Bomb take Pulse's own measured poses - the
    /// Mine spun and scaled by `Mine_PoseNode`, the Bomb squared to the world
    /// by `Bomb_Init` - rather than the frozen craft pose. A title fact:
    /// `craft_title.looks.laid_pose`. See
    /// `weapons::visuals::laid`.
    pub pulse_laid_pose: bool,
    /// A laid Mine alone takes the spun, 0.6-scaled pose: `looks.laid_pose_scaled`.
    pub laid_pose_scaled: bool,
    /// Whether weapon detonations start `ScreenFlash_Start`'s full-screen
    /// wash - Pulse off a PSP disc only, the one executable its consumer is
    /// read off. See `oag_fx::flash`.
    pub screen_flash: bool,
    /// Whether the Quake wave alone starts that wash: a title that draws none
    /// of its other detonations' flashes but whose own wave is read
    /// (`oag_title::QuakeLook::flash`). Ignored with [`Self::screen_flash`].
    pub quake_flash: bool,
    /// Whether a Rocket's craft blast carries the struck craft's velocity
    /// (`oag_title::Effects::craft_blast_carry`).
    pub craft_blast_carry: bool,
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
    /// The noise the cockpit view's hit static is drawn from
    /// (`WeaponModels::cockpit_hit`), or `None` on a title that draws none or
    /// whose entry did not load. See `race::cockpit_hit`.
    pub cockpit_hit_noise: Option<oag_fx::exhaust::FlareTexture>,
    /// Each slot's wreck `Ship Collision Fx` locators, where its destruction
    /// effects spawn - see `race::wreck_fx`.
    pub wreck_anchors: Vec<Vec<oag_livery::SparkAnchor>>,
    /// The Zone end's shell and debris, where the title ships and loaded them - see
    /// `race::zone_end`.
    pub zone_end: Option<std::sync::Arc<oag_livery::ZoneEndModels>>,
    /// The ordinary craft's debris pieces, and what throws them - see `race::ship_debris`.
    pub ship_debris: Option<std::sync::Arc<oag_livery::ShipDebrisModels>>,
    pub ship_debris_spec: Option<oag_title::destroyed::ShipDebris>,
    /// Whether the title throws the big blast (and its shockwave) when a wreck's state times
    /// out: its table names [`Trigger::WreckExplosion`]. See `race::wreck_fx`.
    pub wreck_shockwave: bool,
    /// Each slot's `arc_anchor_point` in its hull's model space, when the title
    /// builds the HD-lineage magstrip arc wake (`Some`); a slot whose hull
    /// authors none is `None` and draws no wake. See `race::magstrip_wake`.
    pub magstrip_wake: Option<[Option<Mat4>; oag_gameplay::MAX_SHIPS]>,
    /// Whether the over-strip effect is the title's `.pob`: see `WeaponModels::magstrip_pob`.
    pub magstrip_pob: bool,
    /// Each slot's `arc_anchor_point` for HD's LeachBeam strip, `Some` when drawn.
    pub leach_strip: Option<[Option<Mat4>; oag_gameplay::MAX_SHIPS]>,
    /// The Missile's ribbon config where the title draws them and the texture
    /// loaded, else `None`: see [`crate::missile_smoke`].
    pub missile_trail: Option<oag_title::weapons::MissileTrail>,
    /// The wingtip contrails' config and each slot's locators where the race draws them and the
    /// texture loaded, else `None`: see [`crate::wingtip`].
    pub wingtip: Option<crate::wingtip::WingtipDraw>,
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
    /// The archive entry of the circuit's flyby voice-over, where the title plays one and the
    /// disc holds it - see [`Race::intro_voice_entry`].
    pub intro_voice: Option<String>,
    /// How far and how fast the airbrake flaps move, from the same document.
    ///
    /// Separate from [`SimSetup::handling`] because it is per *team* rather than
    /// per speed class, and because no force term reads it - see
    /// `oag_gameplay::airbrake_graphics_for`.
    pub airbrake_graphics: oag_gameplay::AirbrakeGraphics,
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
    /// Each locator's Z axis, indexed like [`Self::nozzles`] (`oag_livery::Livery::nozzle_axis`).
    pub nozzle_axes: Vec<Option<Vec3>>,
    /// The engine light per slot (HD, 2048, Omega) - `EngineLightData.xml` and the
    /// flare locator's Z axis - or `None` on a title with no engine light law and on a
    /// craft that ships neither. Indexed like [`Self::nozzles`], which
    /// carries the locator's position. See [`crate::engine_light`].
    pub engine_lights: Vec<Option<oag_livery::engine_light::EngineLight>>,
    /// Whether the circuit's `.envsettings` leaves the SPU vertex lights on -
    /// `"Lighting.Enable spu vertex lights"`, on by default and authored off
    /// on two HD circuits. See `race::load::engine_light`.
    pub spu_vertex_lights: bool,
    /// The circuit's `wopoint`/`wospot` lights and how many hull light slots its
    /// directionals leave them - Pulse's hull lights only, empty everywhere else.
    /// See [`mesh_render::HullPointLights`] and [`Race::ship_hull_points`].
    pub hull_points: mesh_render::HullPointLights,
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
    /// The pad rumble patterns the event map fires, out of this title's archive
    /// (`oag_input::rumble::map::pattern_names`). Empty where the title ships
    /// none (Pulse and Pure): such a race rumbles nothing.
    pub rumble: oag_tables::rumble::Library,
    /// The title's drawn-hull shake, `oag_title::RaceDefaults::hull_shake` ([`crate::hull_shake`]).
    pub hull_shake: Option<oag_title::hull_shake::HullShake>,
    /// What the drawn hull's matrix is multiplied by for this race's mode: the title's
    /// Detonator hull scale over the `0.75` the model carries ([`hull_draw_factor`]), `1.0` in
    /// every other race.
    pub hull_draw_factor: f32,
    /// HD/Fury's zoom-streak ring, `oag_title::RaceDefaults::zoom_ring`; `None` elsewhere.
    pub zoom_ring: Option<&'static oag_title::ZoomRing>,
    /// Whether [`Self::zoom_ring`] is HD's ring borrowed on a title that draws none
    /// of its own ([`Options::boost_zoom`]), and so is drawn by the scene's own host
    /// pass rather than inside HD's bloom chain. **Chosen, not measured.**
    pub zoom_borrowed: bool,
    /// Each Detonator stage's `Detonator Mine Colour` bytes, stage `0` first, where the title
    /// recolours a live mine's blast from it (`oag_title::Effects::mine_blast_recolour`); empty
    /// elsewhere.
    pub mine_blast_colours: Vec<Option<[u8; 4]>>,
    /// [`Options::hud`], carried through to the readout.
    pub capture_hud: Option<CaptureHud>,
    /// Whether this race's HUD stays hidden until the race is released:
    /// `oag_title::RaceDefaults::hud_waits_for_go` naming the race's mode.
    pub hud_waits_for_go: bool,
    /// The song-name marquee, read at load for a title that authors one.
    pub now_playing: Option<crate::now_playing::View>,
    /// [`Options::world_clock_offset`], carried to the scene's animation clock.
    pub world_clock_offset: f32,
    /// [`Options::gantry_seconds`], carried to the gantry's clock.
    pub gantry_seconds: Option<f32>,
    /// [`Options::flame_seconds`], carried to the flame surfaces' clock.
    pub flame_seconds: Option<f32>,
    /// [`Options::camera`], carried through to [`Race::view`]. Plain data, so
    /// it rides in the simulation half even though only the renderer reads it.
    pub camera_override: Option<CameraOverride>,
    /// [`Options::fov_override`], carried through to [`Race::view`].
    pub fov_override: Option<f32>,
    /// The drawn reticle's grids and law, off the loaded HUD: [`SimSetup::sight`]'s lock
    /// grid seen from this machine's screen ([`Sighting::view_sight`]).
    pub sight: Sighting,
    /// The track's authored section-visibility rows, for the multiplayer tag gate
    /// (`oag_hud::player_tags::in_view_row`). `None` where the track names no sections.
    pub track_sections: Option<oag_vex::pvs::TrackPvs>,
}

/// A build without the `view` feature draws and plays nothing, so it has nothing to build
/// a view from.
#[cfg(not(feature = "view"))]
#[derive(Debug, Clone, Default)]
pub struct ViewSetup;

#[cfg(feature = "view")]
impl ViewSetup {
    /// [`Self::headless`] for a race loaded from `options` onto `circuit` ([`SimLoad::circuit`]),
    /// carrying the launch request a recording of it writes into its header.
    #[must_use]
    pub fn headless_on(options: &Options, circuit: &str) -> Self {
        Self {
            request: Some(options.replay_setup_on(circuit)),
            ..Self::headless()
        }
    }

    /// The view of [`SimSetup::headless`]'s race: every field the empty value its type
    /// defines, so a test that builds a race by hand draws and plays nothing it did not ask for.
    #[must_use]
    pub fn headless() -> Self {
        Self {
            trail_sparks: false,
            request: None,
            campaign_event: false,
            score_pass: None,
            hd_trail: None,
            ps2_trail: false,
            // No shield to draw either, and `Palette` has no measured "empty"
            // constant of its own - only Pulse's and HD's real, measured
            // colours - so a zeroed literal is the honest absence here,
            // matching every other visual field on this constructor rather
            // than picking one title's real palette for a race that draws
            // nothing.
            shield_targets: Vec::new(),
            shield_palette: oag_render::shield::Palette {
                activation: [0.0; 4],
                target: [0.0; 4],
                hit: [0.0; 4],
                tints_shell: true,
                flickers_shell_alpha: true,
                colour_rate: oag_render::shield::COLOUR_RATE,
                target_alpha_from_cockpit: false,
            },
            // Nothing draws on this constructor, Pulse's mechanism included -
            // see `shield_palette` above for the same reasoning.
            hd_plasma_blast: false,
            plasma_blast_law: None,
            disruptor_halo_law: None,
            disruptor_craft_law: None,
            hd_bomb_blast: false,
            bomb_shockwave_fades: false,
            hd_missile_blast: false,
            detonator_cannon_lights: false,
            detonator_emp_light: false,
            detonator_mine_spin: None,
            detonator_ammo_gauge: None,
            detonator_hull_charge: None,
            pulse_laid_pose: false,
            laid_pose_scaled: false,
            screen_flash: false,
            quake_flash: false,
            craft_blast_carry: false,
            absorb_burst: None,
            absorb_anchors: Vec::new(),
            hit_spark_anchors: Vec::new(),
            weapon_spark_anchors: Vec::new(),
            cockpit_hit_noise: None,
            wreck_anchors: Vec::new(),
            zone_end: None,
            ship_debris: None,
            ship_debris_spec: None,
            wreck_shockwave: false,
            magstrip_wake: None,
            magstrip_pob: false,
            leach_strip: None,
            missile_trail: None,
            wingtip: None,
            destroy_stations: Vec::new(),
            intro_camera: None,
            intro_voice: None,
            // Zero travel at zero speed: the flaps neither open nor move.
            airbrake_graphics: oag_gameplay::AirbrakeGraphics {
                amount: 0.0,
                up_speed: 0.0,
                down_speed: 0.0,
            },
            chase_close: super::headless::ZERO_CHASE,
            internal: InternalParams {
                fov: 0.0,
                headtilt: None,
                height: 0.0,
                length: 0.0,
                pitch: 0.0,
            },
            nozzles: Vec::new(),
            nozzle_axes: Vec::new(),
            engine_lights: Vec::new(),
            spu_vertex_lights: true,
            hull_points: oag_mesh::mesh_render::HullPointLights::none(),
            spark_anchors: Vec::new(),
            collision_fx: Vec::new(),
            scenery_fx: Default::default(),
            effects: psys::Library::default(),
            handles: Default::default(),
            sounds: oag_sound::sfx::Banks::default(),
            track_emitters: oag_sound::sfx::TrackEmitters::default(),
            announcer: oag_sound::sfx::Announcer::default(),
            class_announcer: oag_sound::sfx::ClassAnnouncer::default(),
            rumble: oag_tables::rumble::Library::default(),
            hull_shake: None,
            hull_draw_factor: 1.0,
            zoom_ring: None,
            zoom_borrowed: false,
            mine_blast_colours: Vec::new(),
            capture_hud: None,
            hud_waits_for_go: false,
            now_playing: None,
            world_clock_offset: 0.0,
            gantry_seconds: None,
            flame_seconds: None,
            camera_override: None,
            fov_override: None,
            sight: Sighting::default(),
            track_sections: None,
        }
    }
}

#[cfg(not(feature = "view"))]
impl ViewSetup {
    /// Nothing, as [`ViewSetup`] itself is in this build.
    #[must_use]
    pub fn headless() -> Self {
        Self
    }

    /// Nothing, as [`ViewSetup`] itself is in this build.
    #[must_use]
    pub fn headless_on(_options: &super::Options, _circuit: &str) -> Self {
        Self
    }
}
