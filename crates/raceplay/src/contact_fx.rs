//! A seat's wall-contact effects: the camera shake a hit arms, the collision sparks, and the
//! cooldown and anchor that pace them.
//!
//! One [`ContactFx`] per seat that has its own view. The player's lives in
//! [`RaceView::contact`](super::view::RaceView::contact); in a split screen or a multi-window
//! race, every other person's seat has one beside its chase camera
//! ([`Viewpoints`](super::viewpoint::Viewpoints)), stepped by the same
//! [`ContactFx::advance`] from that seat's own craft and wall contact, so a second, third or
//! fourth local person sees sparks at their own craft and shakes on their own camera.
//!
//! **Nothing here is simulation state**: the wall contact is evaluated by the tick for every
//! flown slot regardless, and the state hash holds none of this.

use super::*;
use crate::handles::EffectHandles;

/// The effects one seat's wall hits raise.
#[derive(Debug)]
pub(super) struct ContactFx {
    /// The seat's camera shake, armed by a hard wall hit. Render-only, for
    /// the same reason [`RaceView::camera`] is - see
    /// `oag_render::camera::shake`'s module doc comment for what is confirmed
    /// against the original and what this module chose on its own.
    pub(super) shake: oag_render::camera::shake::Shake,
    /// The shake's phase draw, deliberately **not** `world.rng` - see
    /// [`RaceView::exhaust_rng`].
    pub(super) shake_rng: Rng,
    /// Collision sparks' particle pool, advanced on the simulation tick.
    ///
    /// Here rather than in `World`, for the same reason [`RaceView::exhaust`] is -
    /// see `oag_fx::psys`'s module doc comment.
    ///
    /// Boxed: a pool is some 25 KB, and a `Race` is moved by value.
    pub(super) sparks: Box<psys::System>,
    /// How many bursts the *trigger* has fired, whether or not an effect
    /// was loaded to play them.
    ///
    /// Counted here rather than read off the pool because the two answer
    /// different questions: the pool knows how many times it was ignited,
    /// this knows how many times the contact rule said to - and only the
    /// second is [`Race`]'s to get right.
    pub(super) ignitions: u32,
    /// The sparks' generator, deliberately **not** `world.rng` - see
    /// [`RaceView::exhaust_rng`].
    pub(super) rng: Rng,
    /// Seconds remaining before another spark burst is allowed.
    ///
    /// Counts down every tick regardless of contact, and gates a burst
    /// alongside `oag_physics::wall::WallResponse::impact` - see
    /// `oag_gameplay::cue::COLLISION_COOLDOWN`'s doc comment for why this is
    /// a cooldown timer and not a one-shot edge latch: `impact` stays `true`
    /// for every tick of a sustained scrape, and spawning a burst on every
    /// one of those ticks is the per-frame-instead-of-per-impact bug
    /// `oag_physics::wall::STUN_PER_CONTACT`'s doc comment already records
    /// costing a session of play-testing, but the recovered original itself
    /// re-fires periodically rather than going silent for the rest of the
    /// scrape.
    pub(super) cooldown: f32,
    /// The burst's emitter anchor, **model space**: the `Ship Collision Fx`
    /// locator nearest the last impact, or the contact point itself mapped
    /// into model space when the model authors no locators. Transformed
    /// through the ship's current matrix every tick, so the burst rides the
    /// hull - position and spawn direction both - the way the original's
    /// scene-graph node does. See [`Self::anchor_up`] for the other
    /// half of that transform.
    pub(super) anchor: Option<Vec3>,
    /// The chosen locator's own authored `+Y`, **model space** - paired with
    /// [`Self::anchor`] and read the same tick. `None` alongside it.
    ///
    /// `oag_fx::psys::System::advance` takes the *world*-space direction
    /// this maps to, every tick, the same way [`Self::anchor`]'s
    /// position is re-transformed every tick rather than baked once: a craft
    /// banking mid-scrape tilts the spray with it, not just the point it
    /// comes from.
    pub(super) anchor_up: Option<Vec3>,
    /// Whether the sparks are currently *attached* to a wall contact, for the
    /// effects that author [`oag_pob::flags::LOOPING`].
    ///
    /// A looping emitter has no countdown -
    /// `oag_fx::psys::EmitterSpec::run_ticks` is infinite and
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
    pub(super) attached: bool,
}

impl ContactFx {
    /// A seat that has not been hit. The seeds are the player's; a seat's generators are
    /// its own, so one seat's draws never move another's.
    pub(super) fn new() -> Self {
        Self {
            shake: oag_render::camera::shake::Shake::new(),
            shake_rng: Rng::new(SHAKE_SEED),
            sparks: Box::new(psys::System::new()),
            ignitions: 0,
            rng: Rng::new(SPARKS_SEED),
            // No sync frame to be mid-scrape on, so the first tick's contact -
            // if any - is always read as a fresh impact.
            cooldown: 0.0,
            anchor: None,
            anchor_up: None,
            attached: false,
        }
    }

    /// The collision sparks, the shake a hit arms, for the seat's wall contact this tick
    /// (`evaluated`, from `ship`, drawn with `model_matrix`).
    ///
    /// Everything [`Race::tick`] draws on a wall contact, in one place so
    /// [`Race::tick_sim_only`] can skip it whole. The contact sounds and the shield edge are
    /// not here: they are cues, raised by the tick.
    pub(super) fn advance(
        &mut self,
        handles: &EffectHandles,
        collision_fx: &[oag_livery::SparkAnchor],
        ship: &Ship,
        model_matrix: Mat4,
        dt: f32,
        evaluated: &Evaluated,
    ) {
        // Cooldown-gated, not edge-triggered - see `Self::sparks_cooldown`'s
        // doc comment for why a sustained scrape must re-fire periodically
        // rather than spawn once and go silent.
        self.cooldown = (self.cooldown - dt).max(0.0);
        // The trigger rule runs whether or not the disc's own effect
        // loaded - see `Setup::effects`. Only the pool needs it.
        let effect = handles.get(Trigger::CollisionSpark).cloned();
        // Two trigger modes, chosen by the effect's own flag and never by the
        // platform. A `LOOPING` emitter has no countdown, so an owner must let
        // go of it; a burst ends itself and the cooldown is only there to
        // space the next one. Pulse authors this effect as a burst and HD
        // authors it looping - see `Race::sparks_attached`.
        let attached_effect = effect
            .as_deref()
            .is_some_and(|effect| effect.roots().iter().any(|&r| effect.emitters[r].looping));
        // **A raised shield throws no hull sparks**, and that is recovered:
        // the contact loop's third `craft+0x1b8 & 0x10` gate (`0x0884255c`)
        // branches past `Ship_DispatchCollisionFx` (`0x0883de90`) entirely -
        // reaction #1 on `docs/.../contact-response.md`, the spark spawn. The
        // shell's own bulge is what a shielded contact shows instead, which is
        // the whole design: sparks are the hull being hurt and the hull is not
        // being hurt. See `docs/.../shield-pickup.md`.
        let shielded = ship.physics.shield_pickup_timer > 0.0;
        let can_fire = !shielded
            && if attached_effect {
                // Ignite once per contact, not once per cooldown: re-arming
                // would leave a chattering scrape with no sparks for up to
                // `COLLISION_COOLDOWN` at a time, which a burst never suffers
                // because it has already finished by then.
                evaluated.wall.impact && !self.attached
            } else {
                evaluated.wall.impact && self.cooldown <= 0.0
            };
        if can_fire && let Some(contact) = evaluated.wall.resolved {
            // `contact.point` is deliberately the *penetrating* hull sample
            // point, not the wall surface - see its doc comment on
            // `oag_physics::wall::WallContact`, which matches the original's
            // own `Collision_AddContact` exactly. Correct for physics, wrong
            // for a visual: that point sits up to `depth` units inside the
            // opaque wall, so a spark quad centred there is depth-tested away
            // by the wall's own geometry almost every time and reads as
            // nothing spawning. `point + normal * depth` is the same
            // correction `resolve_contact`'s `escape` vector already applies
            // to push the hull back out to the surface.
            let surface = contact.point + contact.normal * contact.depth;
            // The original triggers the `Ship Collision Fx` locator nearest
            // the contact and the burst emits from that node - and along its
            // authored `+Y` - from then on (`Ship_DispatchCollisionFx`); with
            // no locators authored, the surface point itself is mapped into
            // model space, aimed along world up, so it still rides the hull
            // with rotation.
            let (anchor_model, up_model) = collision_fx
                .iter()
                .min_by(|a, b| {
                    let da = (model_matrix.transform_point3(a.position) - surface).length_squared();
                    let db = (model_matrix.transform_point3(b.position) - surface).length_squared();
                    da.total_cmp(&db)
                })
                .map_or_else(
                    || (model_matrix.inverse().transform_point3(surface), Vec3::Y),
                    |anchor| (anchor.position, anchor.up),
                );
            self.anchor = Some(anchor_model);
            self.anchor_up = Some(up_model);
            if let Some(effect) = effect.as_deref() {
                self.sparks.ignite(
                    effect,
                    surface,
                    sparks::severity(evaluated.wall.impact_speed),
                );
            }
            // `Camera_ArmShake`'s own severity is the *raw* clamp
            // `Ship_DispatchCollisionFx` passes it, not the spark's
            // slope-and-floor-reshaped one - see
            // `oag_fx::sparks::clamped_intensity`. `mode = 3` (ahead) vs
            // `mode = 1` (elsewhere) is a dot product of the contact point
            // against the craft's forward, exactly as
            // `docs/ghidra/functions/ps2-pulse-eu/collision-shake.md`
            // records it.
            //
            // Re-armed on the same cooldown gate as the spark burst
            // (`can_fire`) rather than every tick a contact is live: the
            // original re-arms once per collision-function call on a fresh
            // contact, which this project's own contact evaluation does not
            // expose as a separate edge, and pairing the shake's cadence with
            // the visible spark burst is the natural substitute.
            let ahead =
                (contact.point - ship.physics.body.position).dot(ship.physics.body.forward()) > 0.0;
            let side = if ahead {
                oag_render::camera::shake::Side::Ahead
            } else {
                oag_render::camera::shake::Side::Elsewhere
            };
            let shake_severity = sparks::clamped_intensity(evaluated.wall.impact_speed);
            self.shake.arm(shake_severity, side, &mut self.shake_rng);
            self.ignitions += 1;
            self.cooldown = oag_gameplay::cue::COLLISION_COOLDOWN;
            self.attached = attached_effect;
        }
        // Releasing on "no contact at all" rather than on `impact` going false
        // keeps the fountain alive through a gentle slide after the hit -
        // `impact` is the *inbound* test and a scrape stops being inbound long
        // before it stops being a scrape. `stop` takes the emission and leaves
        // the live particles to finish their own lives, so the trail outlasts
        // the contact the way it does when a rocket detonates.
        //
        // Limit worth knowing: `stop` is all-or-nothing across a tree, so this
        // rule does not generalise to the four HD effects that mix `LOOPING`
        // with clear siblings - it would truncate the siblings. None of the
        // four is wired to this trigger.
        if self.attached && evaluated.wall.resolved.is_none() {
            self.sparks.stop();
            self.attached = false;
        }
        // Unconditional, like the exhaust: the emitters keep trickling and
        // already-live particles keep ageing even on a tick with no fresh
        // impact. The anchor is the chosen hull locator under the ship's
        // *current* transform, the way the original's scene-graph node rides
        // the craft - and so is the direction its authored `+Y` maps to: a
        // craft banking mid-scrape tilts the spray with it rather than
        // spraying along a fixed world up. `transform_vector3` rather than
        // `transform_point3`: it is a direction, not a point, so only the
        // matrix's rotation applies and not its translation.
        let anchor = self.anchor.map_or(ship.physics.body.position, |local| {
            model_matrix.transform_point3(local)
        });
        let up = self
            .anchor_up
            .map_or(Vec3::Y, |local| model_matrix.transform_vector3(local));
        if let Some(effect) = effect.as_deref() {
            self.sparks.advance(effect, dt, anchor, up, &mut self.rng);
        }
    }
}
