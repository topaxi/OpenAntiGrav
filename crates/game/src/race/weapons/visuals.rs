//! What a fired weapon *shows*: the blasts [`Race::ignite_blast`] plays, the
//! flares riding each live projectile, the Missile's own wall-bounce burst,
//! and the sprites/matrices a caller with no model draws them as instead.
//!
//! Split out of `weapons.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. Its tests
//! are in `race/tests/weapons.rs` and `race/tests/scene.rs`, alongside the
//! rest of the weapon-visuals coverage.

use super::*;

impl Race {
    /// Plays the explosion a weapon that just went off authored - its own,
    /// not another weapon's.
    ///
    /// **Recovered for the Rocket, the Missile and the Mine; deliberately
    /// silent for the Bomb.** See [`Race::blast_for`] for the map and the
    /// reading behind each arm - the Bomb draws nothing here because its own
    /// teardown, a distinct function from the Mine's, has not been read, not
    /// because it has stopped exploding: the damage and impulse in
    /// `oag_gameplay::projectile::blast` still land.
    ///
    /// Nothing is drawn when the effect did not load, or when [`blast_for`]
    /// says this kind has none. That is deliberate and it is the rule for
    /// every effect here: an authored particle system is not something to
    /// approximate with billboards, and reusing *another* weapon's file is
    /// the same invention wearing a real asset - see [`Self::advance_projectile_flares`]'s
    /// doc comment for the flare bug this was the blast-side twin of. Until
    /// 2026-08-12 this method drew three expanding additive puffs and a
    /// separate eight-billboard smoke trail, all invented; until 2026-08-26
    /// it drew the *Rocket's* `TRACK_BLAST_EFFECT`/`CRAFT_BLAST_EFFECT` for
    /// every kind of impact, mine and missile included, because [`blast_for`]
    /// took no `kind` at all.
    ///
    /// [`Race::sparks`] is deliberately not reused for it: that system
    /// re-anchors to the hull every tick, so a burst ignited at an impact
    /// would emit from the craft instead. It is one hull-mounted emitter, and
    /// this is why the stage exists alongside it.
    ///
    /// [`blast_for`]: Race::blast_for
    pub(in crate::race) fn ignite_blast(
        &mut self,
        kind: oag_formats::weapons::Weapon,
        point: Vec3,
        struck: Option<usize>,
    ) {
        let Some((name, at)) = self.blast_for(kind, point, struck) else {
            return;
        };
        let Some(effect) = self.effects.get(name).cloned() else {
            return;
        };
        // Neutral severity: the field the collision sparks derive from an
        // impulse is the *hull's*, and nothing on the rocket path has been
        // read as feeding it. See `oag_render::sparks::severity`.
        self.stage.play(&effect, at, 1.0);
    }

    /// Which explosion a hit plays and where, or `None` for a weapon with no
    /// recovered detonation effect - split out from [`Race::ignite_blast`] so
    /// the recovered part can be asserted without a disc to load the effect
    /// from.
    ///
    /// - **`Rocket`**: [`TRACK_BLAST_EFFECT`] from `Rocket_Update`'s
    ///   (`0x0885d2a8`) two collision branches, [`CRAFT_BLAST_EFFECT`] from
    ///   `Rocket_SpawnCraftExplosion_q` (`0x0886ed34`) on the craft-hit path.
    ///   A craft hit is drawn at the *struck craft's* own position dropped by
    ///   [`CRAFT_BLAST_DROP`], not at the rocket's impact point; a craft that
    ///   has since gone inactive falls back to the impact point rather than
    ///   reading a stale pose.
    /// - **`Missile`**: [`MISSILE_EXPLO_EFFECT`] always, whatever it struck -
    ///   see that constant's doc comment for why there is one file and no
    ///   drop offset, unlike the Rocket's.
    /// - **`Mine`**: [`MINE_EXPLO_EFFECT`] always, whatever it struck, the
    ///   same shape as the Missile's - see that constant's doc comment for
    ///   `Mine_SpawnExplosion`.
    /// - **`Bomb`, and everything else**: `None`. The Bomb's own teardown -
    ///   a distinct function from the Mine's, per its own separate pool
    ///   cursor - is not chased, so whether it reaches `Mine_SpawnExplosion`
    ///   too or an equivalent of its own naming `WO_BOMB_SMOKERING` is open.
    ///   Guessing here - playing the Mine's file, or the Rocket's, which is
    ///   what this did for every kind before 2026-08-26 - is exactly the
    ///   invention `CLAUDE.md` forbids.
    pub(in crate::race) fn blast_for(
        &self,
        kind: oag_formats::weapons::Weapon,
        point: Vec3,
        struck: Option<usize>,
    ) -> Option<(&'static str, Vec3)> {
        match kind {
            oag_formats::weapons::Weapon::Rocket => Some(match struck {
                Some(slot) if self.world.ships[slot].active => (
                    CRAFT_BLAST_EFFECT,
                    self.world.ships[slot].physics.body.position - Vec3::Y * CRAFT_BLAST_DROP,
                ),
                // A craft that has gone inactive since the hit falls back to
                // the impact point rather than reading a stale pose - still
                // its own effect, because what was struck is what chose the
                // file.
                Some(_) => (CRAFT_BLAST_EFFECT, point),
                None => (TRACK_BLAST_EFFECT, point),
            }),
            oag_formats::weapons::Weapon::Missile => Some((MISSILE_EXPLO_EFFECT, point)),
            oag_formats::weapons::Weapon::Mine => Some((MINE_EXPLO_EFFECT, point)),
            _ => None,
        }
    }

    /// Plays [`MISSILE_BOUNCE_EFFECT`] at every wall a missile glanced off
    /// this tick.
    ///
    /// **Not reached from `Impact`.** A bounce is not a detonation -
    /// `oag_gameplay::projectile::step` reports only what *stopped* a
    /// projectile, and a bouncing missile does not stop. So this reads
    /// `before`, a snapshot of every slot's [`oag_gameplay::projectile::Projectile::bounces`]
    /// taken right before `step`, and fires wherever a live Missile's own
    /// counter went up by exactly one this tick - the only way it moves, per
    /// [`oag_gameplay::projectile::missile::MAX_BOUNCES`]. `Projectile::bounces`
    /// is public simulation state read here rather than threaded through a
    /// new out-parameter the way [`Race::tick`]'s `absorbed` is, because nothing
    /// about the gate is deterministic-critical or hidden - it is the same
    /// counter [`crate::race::hash`] already hashes.
    ///
    /// A slot whose kind changed (a detonation freed it, a new weapon took
    /// it) cannot show a false bounce: [`oag_gameplay::Projectile::default`]
    /// resets `bounces` to zero, so the count only ever goes *down* across
    /// such a transition, never up, and this only looks for an increase.
    ///
    /// Called after `projectile::step`, so a bounce plays where the missile
    /// actually is this tick - a few hundredths of a unit past the wall it
    /// struck, at [`oag_gameplay::projectile::missile::BOUNCE_PUSH_OFF`], rather
    /// than exactly on it.
    pub(in crate::race) fn ignite_missile_bounces(
        &mut self,
        before: &[u8; oag_gameplay::projectile::MAX_PROJECTILES],
    ) {
        for (slot, projectile) in self.world.projectiles.slots.iter().enumerate() {
            if !bounced_this_tick(projectile.kind, before[slot], projectile.bounces) {
                continue;
            }
            // Looked up per slot rather than once outside the loop, because two
            // weapons bounce and they play different files - see
            // [`bounce_effect_for`]. A weapon whose file did not load plays
            // nothing and does not fall back to the other's.
            let Some(effect) = bounce_effect_for(projectile.kind)
                .and_then(|name| self.effects.get(name))
                .cloned()
            else {
                continue;
            };
            self.stage.play(&effect, projectile.position, 1.0);
        }
    }

    /// Keeps an [`ENGINE_FLARE_EFFECT`] instance on every active craft's
    /// nozzle, where the source authors one.
    ///
    /// **The PS2 port authors an engine flare as a particle effect and the
    /// PSP does not.** On a PSP-sourced race the effect is absent from the
    /// library, nothing attaches, and [`oag_render::exhaust`]'s procedural
    /// flare draws as it always has. On a PS2-sourced one the asset plays
    /// and the procedural quad steps aside - see
    /// [`Race::engine_flare_effect`], which is what the renderer asks.
    ///
    /// The trigger needs no reverse-engineering: an engine flare is on for
    /// as long as the craft is, which is why this one is wired where the
    /// other PS2-only effects are not. Both its emitters are
    /// [`oag_formats::pob::flags::LOOPING`], so it runs until detached.
    ///
    /// Anchored at the `Engine Flare` locator under the craft's *current*
    /// transform, the same point the procedural flare uses, so the two are
    /// interchangeable rather than merely similar.
    pub(in crate::race) fn advance_engine_flares(&mut self) {
        let Some(effect) = self.effects.get(ENGINE_FLARE_EFFECT).cloned() else {
            return;
        };
        for slot in 0..MAX_SHIPS {
            let nozzle = self.world.ships[slot]
                .active
                .then(|| self.nozzle_of(slot))
                .flatten();
            match (nozzle, self.engine_flare[slot]) {
                (Some(at), Some(playing)) => self.stage.follow(playing, at),
                (Some(at), None) => self.engine_flare[slot] = self.stage.attach(&effect, at, 1.0),
                // Gone or never had a locator: hand the instance back and
                // let its particles fade rather than cutting them.
                (None, Some(playing)) => {
                    self.stage.detach(playing);
                    self.engine_flare[slot] = None;
                }
                (None, None) => {}
            }
        }
    }

    /// Keeps each live projectile's own flare instance riding it - the
    /// Rocket's [`ROCKET_FLARE_EFFECT`], the Missile's [`MISSILE_FLARE_EFFECT`]
    /// - and takes it off the ones that are gone.
    ///
    /// **Recovered, one weapon at a time; see [`flare_effect_for`] for the
    /// map.** `Rocket_Init` (`0x0885cdb8`) and `Missile_Init` (`0x0885a160`)
    /// each attach their own file at launch and ride it for the whole
    /// flight; every emitter involved is [`oag_formats::pob::flags::LOOPING`],
    /// so the authored duration is not a countdown and the effect does not
    /// need re-triggering to outlast it.
    ///
    /// **Gated on the weapon specifically, not on "any live projectile".**
    /// A Mine or a Bomb has no flare at all in the source. Matching on
    /// `kind.is_none()` alone used to attach the Rocket's looping flare to a
    /// mine or a bomb the instant it was laid - a fire riding a charge from
    /// the moment it landed rather than from its detonation, which
    /// [`Race::ignite_blast`] is the only thing that plays.
    ///
    /// A projectile that stops riding a flare has it **detached, not
    /// killed**: emission stops and the particles already out finish their
    /// own lives, so the smoke outlives the weapon the way it does in the
    /// original. That is also the bug the invented rocket trail could not
    /// avoid - it had to drop the whole streak the instant the slot vacated,
    /// or the next projectile to take that slot would have inherited it.
    ///
    /// **A slot's own instance is always detached before that slot can be
    /// reused by a different weapon**, which is what lets one `Option`
    /// stand for either effect without recording which: `place` (`spawn`,
    /// `spawn_guided`, `lay`) only ever takes a slot whose `kind` is already
    /// `None`, and `kind` only turns `None` inside `projectile::step`, which
    /// this method always runs after in the same tick - so a freed slot has
    /// already passed through a `None` tick of its own before it can be
    /// reused, and this method detaches on exactly that tick.
    ///
    /// **Called after `projectile::step`**, so a flare is anchored where its
    /// weapon ended the tick.
    pub(in crate::race) fn advance_projectile_flares(&mut self) {
        for (slot, projectile) in self.world.projectiles.slots.iter().enumerate() {
            let name = flare_effect_for(projectile.kind);
            let (primary, orbiting) = if projectile.kind
                == Some(oag_formats::weapons::Weapon::Missile)
            {
                let age = oag_gameplay::projectile::MAX_FLIGHT_SECONDS - projectile.lifetime;
                let (a, b) = missile_flare_anchors(projectile.position, projectile.velocity, age);
                (a, Some(b))
            } else {
                (projectile.position, None)
            };
            advance_one_flare(
                &mut self.stage,
                &self.effects,
                name,
                primary,
                &mut self.projectile_flare[slot],
            );
            // The Missile's second, orbiting anchor - see `missile_flare_anchors`.
            // `orbiting` is `None` for every other kind, so this rides nothing
            // and only ever tears down a leftover instance from a slot that
            // was a Missile last tick.
            advance_one_flare(
                &mut self.stage,
                &self.effects,
                orbiting.and(name),
                orbiting.unwrap_or(primary),
                &mut self.projectile_flare_orbit[slot],
            );
        }
    }

    /// This frame's fallback projectile sprites, in the engine flare's shape.
    ///
    /// `right` and `up` are the camera's, read out of the view matrix the same
    /// way the exhaust's are.
    ///
    /// `modelled` says the caller is drawing the rockets as
    /// [`ROCKET_MODEL_ENTRY`]'s mesh, in which case this draws **nothing**:
    /// the glow around a modelled rocket is [`ROCKET_FLARE_EFFECT`], played
    /// off the disc through [`Race::stage`], and a billboard on top of it
    /// would be a second invented one. What is left here is the case where
    /// the model did not load and a projectile would otherwise be invisible -
    /// see [`PROJECTILE_SPRITE_HALF_SIZE`].
    #[must_use]
    pub fn projectile_sprites(
        &self,
        right: Vec3,
        up: Vec3,
        modelled: bool,
    ) -> Vec<oag_render::mesh::GpuVertex> {
        let mut vertices = Vec::new();
        if modelled {
            return vertices;
        }
        for projectile in &self.world.projectiles.slots {
            if projectile.kind.is_none() {
                continue;
            }
            vertices.extend(exhaust::sprite(
                projectile.position,
                right,
                up,
                PROJECTILE_SPRITE_HALF_SIZE,
                1.0,
            ));
        }
        vertices
    }

    /// Where each live rocket is and how it is oriented, for the model draw.
    ///
    /// **Forward alignment is recovered; the quarter-turn is not.**
    /// `Rocket_Update` (`0x0885d2a8`) rebuilds a basis every tick from the
    /// rocket's normalised velocity and the surface normal under it, hands it to
    /// the model's scene node, and rotates it by a further `-pi/2` about an axis
    /// this project has not resolved - the call is an unresolved import stub.
    /// So this aligns the model along its velocity, which is the evidenced part,
    /// and does **not** invent the extra rotation. See
    /// `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`.
    ///
    /// The original's second basis vector is the *track* normal, which this
    /// engine does not carry on a projectile (its rockets fly straight and never
    /// consult the surface - the same page records that gap). World up stands in,
    /// which only decides the model's roll about its own length.
    ///
    /// One entry per live rocket, in slot order, so the caller can zip it
    /// against its drawables.
    ///
    /// # No rendered frame has yet contained a rocket
    ///
    /// Worth stating rather than leaving to be discovered. What *is* checked:
    /// the model loads off a real disc and its long axis is the one aimed down
    /// the velocity here
    /// (`the_rocket_model_is_longest_along_the_axis_it_is_flown_down`), the
    /// bases below are orthonormal and velocity-aligned including the
    /// straight-up degenerate case, and the draw is wired exactly as the ships'
    /// and plumes' are. What is **not**: a captured frame with a rocket in it.
    ///
    /// The headless capture path cannot produce one. `--race` gives a craft that
    /// holds the throttle and does not steer, so over 2400 ticks it never
    /// reaches a `Weapon Pad`, never gets a pickup, and the telemetry line never
    /// reports one held. A first attempt at this misread three pieces of
    /// **track scenery** as a fanned volley - they render identically in
    /// `time_trial`, where no rocket can exist, which is the check that settles
    /// it and the one to repeat before believing any future frame:
    ///
    /// ```sh
    /// cargo run -p oag-game -- --race --mode single_race --hold cross \
    ///     --press square --ticks 900 --screenshot /tmp/on.png
    /// cargo run -p oag-game -- --race --mode time_trial  --hold cross \
    ///     --press square --ticks 900 --screenshot /tmp/off.png
    /// magick compare -metric AE /tmp/on.png /tmp/off.png null:
    /// ```
    ///
    /// Closing it wants a craft that can drive to a pad - the AI, or an input
    /// script replayed through `oag-trace run --script`.
    #[must_use]
    pub fn rocket_model_matrices(&self) -> Vec<Mat4> {
        self.world
            .projectiles
            .slots
            .iter()
            .filter(|projectile| projectile.kind.is_some())
            .map(|projectile| {
                let forward = projectile.velocity.normalize_or_zero();
                if forward == Vec3::ZERO {
                    return Mat4::from_translation(projectile.position);
                }
                // `Vec3::Y` is a poor reference exactly when the rocket is flying
                // straight up or down; `any_orthonormal_vector` is the fallback
                // rather than a silently degenerate basis.
                let reference = if forward.dot(Vec3::Y).abs() > 0.999 {
                    forward.any_orthonormal_vector()
                } else {
                    Vec3::Y
                };
                let side = forward.cross(reference).normalize_or_zero();
                let up = side.cross(forward);
                Mat4::from_cols(
                    side.extend(0.0),
                    up.extend(0.0),
                    forward.extend(0.0),
                    projectile.position.extend(1.0),
                )
            })
            .collect()
    }
}

/// Which flare effect, if any, a projectile of this kind rides - the
/// Rocket's [`ROCKET_FLARE_EFFECT`], the Missile's [`MISSILE_FLARE_EFFECT`],
/// or `None` for everything else, including a Mine or a Bomb.
///
/// Split out of [`Race::advance_projectile_flares`] so the gate is testable
/// without a loaded `psys` library: the bug this replaced (`kind.is_none()`,
/// true for any live projectile) could only be seen with a real effect
/// asset attached, which a headless unit test has no disc to load. See that
/// method's doc comment for the reading.
#[must_use]
pub(in crate::race) fn flare_effect_for(
    kind: Option<oag_formats::weapons::Weapon>,
) -> Option<&'static str> {
    match kind? {
        oag_formats::weapons::Weapon::Rocket => Some(ROCKET_FLARE_EFFECT),
        oag_formats::weapons::Weapon::Missile => Some(MISSILE_FLARE_EFFECT),
        oag_formats::weapons::Weapon::Plasma => Some(PLASMA_FLARE_EFFECT),
        oag_formats::weapons::Weapon::Shuriken => Some(SHURIKEN_FLARE_EFFECT),
        _ => None,
    }
}

/// One riding flare instance's whole state machine for one tick: attach,
/// follow, or detach, by name and position - shared by
/// [`Race::advance_projectile_flares`]'s primary anchor (every rider) and
/// orbiting anchor (the Missile's second one only).
///
/// A free function taking the stage and the library by reference rather than
/// a `&mut Race` method, so the caller can hold two `&mut Option<Playing>`
/// slots - [`Race::projectile_flare`] and [`Race::projectile_flare_orbit`] -
/// live at once without a second borrow of `self`.
fn advance_one_flare(
    stage: &mut psys::Stage,
    effects: &psys::Library,
    name: Option<&'static str>,
    at: Vec3,
    playing: &mut Option<psys::Playing>,
) {
    match (name, *playing) {
        // Gone, or a weapon with no flare of its own: hand the instance back
        // and let it fade out rather than riding a weapon that never
        // authored this effect.
        (None, Some(instance)) => {
            stage.detach(instance);
            *playing = None;
        }
        (None, None) => {}
        (Some(_), Some(instance)) => stage.follow(instance, at),
        // Freshly in the air. `attach` returning `None` means the stage is
        // full of flares already, and this projectile simply flies without
        // one rather than evicting someone else's.
        (Some(name), None) => {
            if let Some(effect) = effects.get(name).cloned() {
                *playing = stage.attach(&effect, at, 1.0);
            }
        }
    }
}

/// The Missile's own two flare anchors this tick, orbiting its flight line -
/// the primary anchor first, the second in the returned tuple.
///
/// **Recovered, confidence 80.** `Missile_Update` (`0x0885a918`) rebuilds an
/// orthonormal basis every tick a non-degenerate velocity is present -
/// `back = normalize(-velocity)`, `up = normalize(world_up - back *
/// dot(world_up, back))`, `right = cross(up, back)` - then places two
/// points:
///
/// ```text
/// θ        = age * 15.0
/// lateral  = 1.5
/// vertical = min(age * 6.0, 3.0)
/// anchor_a = position + right * lateral * sin(θ) + up * vertical * cos(θ)
/// anchor_b = position + right * lateral * (1 - sin(θ)) + up * vertical * (1 - cos(θ))
/// ```
///
/// **A `sin`/`cos` pair driven by the same angle is a rotation, not a
/// crossfade.** The two anchors orbit the missile's own flight line,
/// growing from a point at launch to a fixed `1.5` x `3.0`-unit ellipse over
/// the first half second, at a constant `15 rad/s` (~2.4 Hz) for the whole
/// flight - `age` is unclamped seconds since launch (`Missile_Update` adds
/// `dt` to it every tick with no `min`), so this runs for the whole
/// `SELF_DETONATE_SECONDS` flight, not just a launch transient. Read from
/// `Missile_Init`'s two `Psys_Spawn_q` calls (one per anchor, both
/// `WO_MISSILE_HEAD`, at entity offsets `+0xf0` and `+0x130`) and
/// `Missile_Update`'s per-tick rewrite of both.
///
/// **Independently consistent with a maintainer's own description from
/// play**, given before any of this was decompiled: "the missile rotates
/// with two trails".
///
/// **What is not settled.** The handedness of `right = cross(up, back)` is
/// cosmetic - it only swaps which of the two visually-interchangeable
/// anchors leads. Whether the basis rebuild is ever skipped mid-flight is
/// not fully chased: the original guards it on the velocity being within
/// about eight degrees of the *previous* tick's up axis
/// (`-0.99 < dot < 0.99`) and keeps flying the stale basis on that one
/// tick, which this engine cannot reproduce without carrying basis state
/// between ticks - a gap that would show as a one-tick pop in an unusual
/// flight, not a wrong steady-state shape.
#[must_use]
pub(in crate::race) fn missile_flare_anchors(
    position: Vec3,
    velocity: Vec3,
    age: f32,
) -> (Vec3, Vec3) {
    let back = (-velocity).try_normalize().unwrap_or(Vec3::NEG_Z);
    let up_seed = Vec3::Y;
    let up = (up_seed - back * up_seed.dot(back))
        .try_normalize()
        // Ours, for the one case the original's own stale-basis guard
        // covers and this function cannot: flying (anti)parallel to world
        // up. `Vec3::X` stands in for the seed rather than leaving a NaN
        // basis to propagate.
        .unwrap_or_else(|| {
            (Vec3::X - back * Vec3::X.dot(back))
                .try_normalize()
                .unwrap_or(Vec3::X)
        });
    let right = up.cross(back);

    let theta = age * 15.0;
    let (sin_theta, cos_theta) = (theta.sin(), theta.cos());
    let lateral = 1.5;
    let vertical = (age * 6.0).min(3.0);

    let a = position + right * (lateral * sin_theta) + up * (vertical * cos_theta);
    let b = position + right * (lateral * (1.0 - sin_theta)) + up * (vertical * (1.0 - cos_theta));
    (a, b)
}

/// Whether a projectile's own bounce counter says it glanced off a wall this
/// tick - split out of [`Race::ignite_missile_bounces`] for the same reason
/// [`flare_effect_for`] is: testable without a loaded `psys` library.
///
/// Only a Missile carries a counter that ever moves (see
/// [`oag_gameplay::projectile::Projectile::bounces`]), so `kind` gates this
/// the same way [`flare_effect_for`] gates on the weapon rather than trusting
/// the counter alone - a stray nonzero `bounces` on some other kind must
/// never read as a bounce.
#[must_use]
pub(in crate::race) fn bounced_this_tick(
    kind: Option<oag_formats::weapons::Weapon>,
    before: u8,
    now: u8,
) -> bool {
    bounce_effect_for(kind).is_some() && now > before
}

/// Which burst a weapon plays on a wall it glances off, or `None` for one that
/// does not glance at all.
///
/// Two weapons bounce and **each authors its own file**, which is the whole
/// reason this is a map rather than a boolean: `Missile_Update` spawns
/// `WO_MISSILE_BOUNCE` and `Shuriken_Bounce` (`0x088778ac`) spawns
/// `WO_SHURIKEN_BOUNCE`, both read directly out of `.rodata`. Playing one
/// weapon's file for the other is the bug that shipped as a mine riding the
/// Rocket's flare.
pub(in crate::race) fn bounce_effect_for(
    kind: Option<oag_formats::weapons::Weapon>,
) -> Option<&'static str> {
    match kind? {
        oag_formats::weapons::Weapon::Missile => Some(MISSILE_BOUNCE_EFFECT),
        oag_formats::weapons::Weapon::Shuriken => Some(SHURIKEN_BOUNCE_EFFECT),
        _ => None,
    }
}
