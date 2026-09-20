//! [`RaceView`]: everything a race carries only so it can be shown or heard.
//!
//! `Race` was one 97-field struct in which the world, the collision broadphase
//! and the opponents' drivers sat interleaved with a chase camera, four
//! particle libraries and a sound bank. The field doc comments already told
//! the two apart - forty-odd of them say "Render-only state" in so many words,
//! and `Race::state_hash` names the same set as the state it must not fold in.
//! Nothing in the type made the boundary checkable, though, so the only thing
//! keeping a renderer handle out of the simulation was that nobody had put one
//! there yet.
//!
//! Splitting the presentation half out first is what makes that boundary cost
//! nothing to verify: **not one field here appears in `Race::state_hash`**, so
//! moving them cannot move a hash, and the simulation half is then whatever is
//! left rather than a second judgement call.
//!
//! # What is here and what is not
//!
//! Here: the camera and its shake, the exhaust and its trails, the shield
//! bubble and the HUD flash it drives, every particle system and its seeds,
//! the *loaded* sound banks, the boost kick, the airbrake flap animation, the
//! lock-on reticle and the finished scoreboard.
//!
//! Not here, deliberately: the cue vectors (`Race::cues`,
//! `Race::announcements`, `Race::class_announcements`). ADR-0018 calls a cue a
//! per-tick **output** of the simulation rather than presentation state, and
//! that is the reading this split keeps. They are produced by the tick and
//! drained by the caller; what plays them is the bank, and the bank is here.

use super::*;

/// The half of a race that exists to be looked at and listened to.
///
/// Reached as `Race::view`. Every field is out of the determinism hash by
/// construction - see the module doc comment - so a change here can move a
/// picture and cannot move a race.
#[derive(Debug)]
pub struct RaceView {
    /// Which of the three perspectives is live. **Render-only state**, on `Race`
    /// rather than in `World` for the same reason [`Self::exhaust`] is - and here
    /// it is load-bearing rather than tidy: cycling the view mid-race must not
    /// move a determinism hash or a replay by a single bit. See
    /// [`oag_display::display::CameraView`] and
    /// `tests::cycling_the_camera_changes_no_simulation_state`.
    pub(super) camera_view: oag_display::display::CameraView,
    pub(super) camera: Chase,
    /// The player's camera shake, armed by a hard wall hit. Render-only, for
    /// the same reason [`Self::camera`] is - see
    /// `oag_render::camera::shake`'s module doc comment for what is confirmed
    /// against the original and what this module chose on its own.
    pub(super) shake: oag_render::camera::shake::Shake,
    /// The shake's phase draw, deliberately **not** `world.rng` - see
    /// [`Self::exhaust_rng`].
    pub(super) shake_rng: Rng,
    /// Render from this pose instead of [`Self::camera`]. See [`CameraOverride`].
    pub(super) camera_override: Option<CameraOverride>,
    /// The external block the chase camera is currently flying, which is
    /// whichever of [`Self::chase_far`] / [`Self::chase_close`]
    /// [`Self::camera_view`] names. Kept as its own field rather than looked up
    /// per use, so every consumer reads one thing and cannot pick the wrong
    /// block.
    pub(super) chase_params: ChaseParams,
    /// `<ExternalCameraFar>`, kept so a view change can swap it back in.
    pub(super) chase_far: ChaseParams,
    /// `<ExternalCameraClose>`.
    pub(super) chase_close: ChaseParams,
    /// `<InternalCamera>`: the cockpit view, which has no spring and so no state
    /// of its own beside these five numbers.
    pub(super) internal_params: InternalParams,
    /// Each craft's exhaust animation state, advanced on the simulation tick.
    ///
    /// **One per racer, slot 0 the player's**, because the original runs
    /// `Exhaust_Update` per craft: a flare, a ribbon and a boost plume belong to
    /// the craft that is burning, not to the race. Indexed by ship slot, so
    /// `exhaust[n]` and `world.ships[n]` are the same craft; entries past
    /// `world.ship_count` are simply never advanced.
    ///
    /// Here rather than in `World` for the same reason [`Chase`] is: it is
    /// render-only state, so it must not enter a snapshot a replay or a
    /// determinism hash reads. `physics/src/probe.rs` destructures `ShipState`
    /// exhaustively on purpose, and adding a visual field there would move the
    /// pinned hashes for no reason.
    pub(super) exhaust: [Exhaust; MAX_SHIPS],
    /// The flicker's generators, deliberately **not** `world.rng`.
    ///
    /// The exhaust draws two random numbers per tick. Taking them from the
    /// simulation's generator would make the picture change what the simulation
    /// does next - the determinism rules exist to prevent exactly that. Seeded, so
    /// a capture at tick *n* is still reproducible.
    ///
    /// **One stream per craft** rather than one shared stream drawn from eight
    /// times a tick, for two reasons: a shared stream makes slot 0's flicker
    /// depend on how many opponents the mode fields, which would break every
    /// capture taken against a single-craft race, and it couples eight flares
    /// that should be independent. See [`exhaust_seed`].
    pub(super) exhaust_rng: [Rng; MAX_SHIPS],
    /// Wipeout HD's exhaust states. Render-only; see `oag_render::exhaust::hd`.
    pub(super) hd_trail: [exhaust::hd::Tube; MAX_SHIPS],
    pub(super) hd_trail_active: bool,
    pub(super) hd_trail_red: [f32; MAX_SHIPS],
    /// Whether a Plasma detonation plays Wipeout HD's own three-model scale
    /// ease rather than Pulse's baked anim-time scrub. See
    /// `options::Setup::hd_plasma_blast`.
    pub(super) hd_plasma_blast: bool,
    /// Origin-to-nozzle distance per craft, world units - the trail-hit reach.
    pub(super) hull_reach: [f32; MAX_SHIPS],
    /// `--trail-sparks`: see [`Race::force_trail_sparks`].
    pub(super) trail_sparks: bool,
    /// Who is inside whose trail, one bit per intruder; level, not edge.
    pub(super) trail_inside: [u8; MAX_SHIPS],
    pub(super) hd_flame: [exhaust::hd::Flame; MAX_SHIPS],
    pub(super) hd_sprite: [exhaust::hd::Sprite; MAX_SHIPS],
    /// Each craft's shield shell animation, when one is up.
    ///
    /// Render-only for the reason [`Self::exhaust`] is: the *simulation* half of
    /// a fired Shield is `oag_physics::ShipState::shield_pickup_timer`, which is
    /// hashed, and this is only the swell and flicker riding on it. Per craft
    /// because the original's shield object hangs off the ship entity. See
    /// [`oag_render::shield`].
    pub(super) shield: [ShipShield; MAX_SHIPS],
    /// Whether the player's shield was up on the previous tick.
    ///
    /// The latch behind `shieldactive`'s rising edge. Kept here rather than in
    /// the audio layer so that every cue *edge* is raised from one place - see
    /// [`Race::tick`] - and out of the hash like the rest of this group.
    pub(super) shield_was_up: bool,
    /// The player's shield percentage as of the previous tick.
    ///
    /// `Hud_UpdateEnergyBar`'s own `hud+0x118`. Read-only outside
    /// [`Race::advance_shield_flash`], which is the only thing that needs
    /// last tick's value rather than this one's - render-side memory, like
    /// [`Self::shield_was_up`] beside it, and out of the hash for the same
    /// reason.
    pub(super) shield_flash_prev: f32,
    /// The shield bar's post-hit flash accumulator.
    ///
    /// `Hud_UpdateEnergyBar`'s own `hud+0x11c`: `0.0` means "not flashing",
    /// and it counts *up* while running rather than down, wrapping to `0.0`
    /// at `1.0` - see [`Race::advance_shield_flash`] for why that is not
    /// rewritten as a countdown. `> 0.0` is what
    /// [`crate::hud::Readout::shield_flashing`] reads out of it every tick.
    /// Render-side, like [`Self::shield_was_up`], and out of the hash.
    pub(super) shield_flash_timer: f32,
    /// The `engine_flare` locator in model space, when the ship model has one.
    ///
    /// One value for the whole field, because every craft wears the player's hull
    /// today - see `Scene::new`. It becomes one per craft when per-team models
    /// land, at which point the locator moves with the model rather than with the
    /// race.
    pub(super) nozzles: Vec<Option<Vec3>>,
    /// Wipeout HD's engine light per slot - see [`Setup::engine_lights`] and
    /// `race::engine_light`. Render-side, out of the hash.
    pub(super) engine_lights: Vec<Option<crate::livery::engine_light::EngineLight>>,
    /// The circuit's `"Lighting.Enable spu vertex lights"`, on by default.
    pub(super) spu_vertex_lights: bool,
    /// Each craft's engine-light jitter this tick, and the stream it is
    /// drawn from - one per craft, like [`Self::exhaust_rng`], and separate
    /// from it so the flare's pinned flicker stream is untouched.
    pub(super) engine_light_jitter: [super::engine_light::Jitter; MAX_SHIPS],
    pub(super) engine_light_rng: [Rng; MAX_SHIPS],
    /// Authored hull spark anchors per slot, model space.
    pub(super) spark_anchors: Vec<Vec<Vec3>>,
    /// The `Ship Collision Fx` locators in model space - see
    /// [`Setup::collision_fx`].
    pub(super) collision_fx: Vec<crate::livery::SparkAnchor>,
    /// Collision sparks' particle pool, advanced on the simulation tick.
    ///
    /// Here rather than in `World`, for the same reason [`Self::exhaust`] is -
    /// see `oag_render::psys`'s module doc comment.
    pub(super) sparks: psys::System,
    /// Every `.pob` this race loaded - see [`Setup::effects`].
    pub(super) effects: psys::Library,
    /// The multi-instance pool everything *except* the hull-mounted sparks
    /// plays in: the rockets' flares and their detonations today, and
    /// whatever gets a recovered trigger next.
    ///
    /// [`Self::sparks`] stays its own [`psys::System`] because it is one
    /// permanently hull-mounted emitter re-ignited on a cooldown rather than
    /// an effect that comes and goes - see [`Self::sparks_anchor`].
    pub(super) stage: psys::Stage,
    /// The [`ENGINE_FLARE_EFFECT`] instance riding each craft's nozzle, on a
    /// source that authors one. All `None` on a PSP-sourced race.
    pub(super) engine_flare: [Option<psys::Playing>; MAX_SHIPS],
    /// The flare instance riding each live projectile's **primary** anchor,
    /// indexed by its [`oag_gameplay::projectile`] slot.
    ///
    /// The slot **is** the identity: it is fixed for a projectile's whole
    /// life and reused the moment the projectile is gone, which is exactly
    /// when the flare should be a fresh one.
    pub(super) projectile_flare: [Option<psys::Playing>; oag_gameplay::projectile::MAX_PROJECTILES],
    /// The Missile's own **second**, orbiting flare instance - see
    /// [`weapons::missile_flare_anchors`]. `None` for every slot that is not
    /// currently a live Missile; every other kind rides [`Self::projectile_flare`]
    /// alone.
    pub(super) projectile_flare_orbit:
        [Option<psys::Playing>; oag_gameplay::projectile::MAX_PROJECTILES],
    /// The [`weapons::visuals::QUAKE_EFFECT`] instance riding the travelling
    /// wave, or `None` when no Quake is in flight. One slot, not an array,
    /// the same shape [`oag_gameplay::World::quake`] itself takes - see that
    /// field's own doc comment for why.
    pub(super) quake_effect: Option<psys::Playing>,
    /// The [`weapons::visuals::LEACHBEAM_ENERGY_EFFECT`] instance riding the
    /// craft a beam is fastened to, or `None` when no link is connected.
    ///
    /// One slot for the same reason [`Self::quake_effect`] is one: the
    /// original allows a single beam in the whole race, so
    /// [`oag_gameplay::World::leach_beam`] is one `Option` and this follows it.
    pub(super) leach_beam_effect: Option<psys::Playing>,
    /// The [`weapons::visuals::LEACHBEAM_CHARGING_EFFECT`] instance riding a
    /// craft that is *holding* a LeachBeam, or `None`.
    ///
    /// **Not the fired weapon - the carried one.** `FUN_0883f540` spawns this
    /// on any craft whose held-weapon id is the LeachBeam's `10`, and despawns
    /// it the moment that stops being true, so a player sees the pickup
    /// charging before they ever press fire. One slot rather than eight because
    /// only the player's own craft is followed; see
    /// [`Race::advance_leach_beam_visual`].
    pub(super) leach_charge_effect: Option<psys::Playing>,
    /// The LeachBeam ribbon's own render-side state - the amplitude table and
    /// scroll phase [`oag_gameplay::projectile::leach_beam::Beam`]
    /// deliberately does not carry, see `oag_render::beam`'s module doc
    /// comment. `Some` for exactly as long as [`oag_gameplay::World::leach_beam`]
    /// is `Some(Kind::Locked)`, rebuilt fresh each time a new beam locks on.
    pub(super) leach_beam_ribbon: Option<oag_render::beam::Ribbon>,
    /// The LeachBeam ribbon's own generator, deliberately **not** `world.rng` -
    /// see [`Self::exhaust_rng`].
    pub(super) leach_beam_rng: Rng,
    /// The Plasma's own render-side detonation instances, one per
    /// [`blast_models::PLASMA_BLAST_SLOTS`] - see that module's own doc
    /// comment for why this lives here rather than in `World`, and
    /// [`Race::spawn_plasma_blast_model`] for what fills a slot.
    pub(super) plasma_blasts: [Option<blast_models::PlasmaBlast>; blast_models::PLASMA_BLAST_SLOTS],
    /// The stage's generator, deliberately **not** `world.rng` - see
    /// [`Self::exhaust_rng`].
    pub(super) stage_rng: Rng,
    /// How many bursts the *trigger* has fired, whether or not an effect
    /// was loaded to play them.
    ///
    /// Counted here rather than read off the pool because the two answer
    /// different questions: the pool knows how many times it was ignited,
    /// this knows how many times the contact rule said to - and only the
    /// second is [`Race`]'s to get right.
    pub(super) sparks_ignitions: u32,
    /// The sparks' generator, deliberately **not** `world.rng` - see
    /// [`Self::exhaust_rng`].
    pub(super) sparks_rng: Rng,
    /// Seconds remaining before another spark burst is allowed.
    ///
    /// Counts down every tick regardless of contact, and gates a burst
    /// alongside `oag_physics::wall::WallResponse::impact` - see
    /// `oag_render::sparks::COLLISION_COOLDOWN`'s doc comment for why this is
    /// a cooldown timer and not a one-shot edge latch: `impact` stays `true`
    /// for every tick of a sustained scrape, and spawning a burst on every
    /// one of those ticks is the per-frame-instead-of-per-impact bug
    /// `oag_physics::wall::STUN_PER_CONTACT`'s doc comment already records
    /// costing a session of play-testing, but the recovered original itself
    /// re-fires periodically rather than going silent for the rest of the
    /// scrape.
    pub(super) sparks_cooldown: f32,
    /// The burst's emitter anchor, **model space**: the `Ship Collision Fx`
    /// locator nearest the last impact, or the contact point itself mapped
    /// into model space when the model authors no locators. Transformed
    /// through the ship's current matrix every tick, so the burst rides the
    /// hull - position and spawn direction both - the way the original's
    /// scene-graph node does. See [`Self::sparks_anchor_up`] for the other
    /// half of that transform.
    pub(super) sparks_anchor: Option<Vec3>,
    /// The chosen locator's own authored `+Y`, **model space** - paired with
    /// [`Self::sparks_anchor`] and read the same tick. `None` alongside it.
    ///
    /// `oag_render::psys::System::advance` takes the *world*-space direction
    /// this maps to, every tick, the same way [`Self::sparks_anchor`]'s
    /// position is re-transformed every tick rather than baked once: a craft
    /// banking mid-scrape tilts the spray with it, not just the point it
    /// comes from.
    pub(super) sparks_anchor_up: Option<Vec3>,
    /// Whether the sparks are currently *attached* to a wall contact, for the
    /// effects that author [`oag_vex::pob::flags::LOOPING`].
    ///
    /// A looping emitter has no countdown -
    /// `oag_render::psys::EmitterSpec::run_ticks` is infinite and
    /// `psys::System::stop` is the only thing that ends it. Pulse authors
    /// `WO_SHIP_COLL_SPARK_DAMAGE` as a 32-tick burst and needs no owner, so
    /// this stays `false` there and the cooldown rule below is the whole
    /// trigger. Wipeout HD authors the same four emitters **looping** with a
    /// 4-tick duration, so something has to own them, and this latch is what
    /// holds the attachment across the ticks of one contact.
    ///
    /// **Confidence 55 on the owner, and reading HD's own dispatch
    /// complicated rather than confirmed it (2026-08-31, see
    /// `docs/ghidra/functions/ps3-hdfury-eu/ship-collision-fx.md`).** That a
    /// looping effect needs one is the flag's own contract - not in
    /// question. That the owner is *wall contact* specifically still is:
    /// HD's `ShipCollisionFx_Trigger_q` names two of its four `kind`
    /// branches from literal strings, and neither is
    /// `WO_SHIP_COLL_SPARK_DAMAGE` - one is a previously-unrecorded
    /// `WO_SHIP_SPARK_DAMAGE_WEAPON`, the other is the no-damage variant.
    /// What actually fires the plain damage variant this field models is
    /// still unread; do not treat that page as having settled this.
    pub(super) sparks_attached: bool,
    /// The decoded sound cues, straight out of [`Setup::sounds`]. Data, not a
    /// device - the mixer and the held voices are [`crate::audio::Audio`]'s.
    pub(super) sounds: crate::audio::sfx::Banks,
    /// The circuit's own authored emitters, straight out of
    /// [`Setup::track_emitters`]. Data, on the same terms [`Self::sounds`] is:
    /// the held voices they open live in [`crate::audio::Audio`].
    pub(super) track_emitters: crate::audio::sfx::TrackEmitters,
    /// Zone mode's milestone announcer, straight out of [`Setup::announcer`].
    pub(super) announcer: crate::audio::sfx::Announcer,
    /// Zone mode's speed-class announcer, straight out of
    /// [`Setup::class_announcer`].
    pub(super) class_announcer: crate::audio::sfx::ClassAnnouncer,
    /// How far the boost's field-of-view kick has opened, `0.0` to `1.0`.
    ///
    /// Render-only state, on `Race` rather than in `World` for the same reason
    /// [`Self::exhaust`] is. See [`oag_display::display::BoostFovKick`], which is
    /// where the "authored, not recovered" argument for the whole effect
    /// lives.
    pub(super) boost_kick: f32,
    /// How strong the kick is, `0` off. `[graphics] boost_fov_kick`.
    pub(super) boost_fov_kick: oag_display::display::BoostFovKick,
    /// How far each airbrake flap has swung, left then right, on `0..=100`.
    ///
    /// The airbrake's own scale rather than an angle, because that is the
    /// scale `up_speed`/`down_speed` ramp on - see the tick that advances it.
    /// [`Race::airbrake_flaps`] is what turns it into radians.
    ///
    /// **Render-only state**, on `Race` rather than in `World` for the same
    /// reason [`Self::boost_kick`] is - and here the separation is the
    /// original's own, not this crate's convenience. `<AirbrakeGraphics>`
    /// carries `up_speed` and `down_speed` *beside* `<Airbrake>`'s `gain` and
    /// `falloff`, which is the game saying outright that the flap the pilot
    /// sees and the airbrake the force law applies ramp at different rates.
    /// Putting these two floats in `ShipState` would move the determinism
    /// hashes every time somebody adjusted an animation.
    pub(super) flaps: [f32; 2],
    /// The authored deflection and the two rates, from `<AirbrakeGraphics>`.
    ///
    /// `amount` is already radians here: `oag_gameplay::airbrake_graphics_for`
    /// applies the loader's own degrees-to-radians scale, recovered at
    /// confidence 92 - see `docs/ghidra/functions/psp-pulse-usa/camera.md`.
    pub(super) flap_graphics: oag_gameplay::AirbrakeGraphics,
    /// The lock-on reticle, and what its tone is doing.
    ///
    /// **Render-only state**, on `Race` rather than in `World` for exactly the
    /// reason [`Self::exhaust`] is: the reticle is what the screen shows, it
    /// must not move a determinism hash, and the simulation does not know a
    /// renderer exists. The *lock it draws* is recomputed here every tick from
    /// the world rather than stored on a ship - see [`Race::update_sight`].
    ///
    /// The acquisition rule it carries is recovered and is **not** yet wired
    /// into firing: `Ship_FireHeldWeapon` gates the target it passes on the
    /// same flag this sets, so an unlocked press in the original fires a dumb
    /// missile. Doing that here changes which shots are guided and is its own
    /// change; see the handover thread.
    pub(super) sight: sight::Sight,
    /// What [`Self::sight`] said last tick, which is what the tone plays off.
    pub(super) sight_state: sight::State,
    /// The field-of-view setting the picture is being drawn at.
    ///
    /// A setter for the same reason [`Race::set_boost_fov_kick`] is one - it is
    /// a preference out of the settings file and a headless race has none - but
    /// unlike the kick it is read by [`Race::update_sight`] rather than by the
    /// camera, because a reticle drawn with the authored field while the player
    /// widened theirs sits off the craft it is supposed to be over. Defaults to
    /// [`oag_display::display::Fov::AUTHORED`], which is the identity.
    pub(super) sight_fov: oag_display::display::Fov,
    /// The results table, taken on the tick the race reached its finish
    /// condition, and `None` before that.
    ///
    /// **A snapshot rather than a live query**, and that is the whole point: "the
    /// race is over" has to mean one fixed table. The field is still moving when
    /// the player crosses - `oag_race::places` ranks whoever has not finished by
    /// distance covered, which is the honest answer at that instant - and a board
    /// recomputed a second later would show a different one.
    ///
    /// On `Race` rather than in `World`: it is derived from state the world
    /// already holds, so hashing it would hash the same facts twice, and a
    /// replay reproduces it by reaching the same tick. See [`Self::results`].
    pub(super) results: Option<crate::scoreboard::Board>,
}
