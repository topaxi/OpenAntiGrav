//! What a fired weapon *shows*: the blasts [`Race::ignite_blast`] plays, the
//! flares riding each live projectile, the Missile's own wall-bounce burst,
//! and the sprites/matrices a caller with no model draws them as instead.
//!
//! Split out of `weapons.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. Its tests
//! are in `race/tests/weapons.rs` and `race/tests/scene.rs`, alongside the
//! rest of the weapon-visuals coverage.

mod flares;
mod laid;
pub(in crate::race) use flares::*;

use super::*;

impl Race {
    /// Keeps [`LEACHBEAM_CHARGING_EFFECT`] on the player while they hold a
    /// LeachBeam - the disc's own `Data\Psys\*.POB`, played through the same
    /// [`psys::Stage`] every other weapon's are, with its trigger read out of
    /// the executable (see [`LEACHBEAM_CHARGING_EFFECT`]).
    ///
    /// The beam's other effect, [`LEACHBEAM_ENERGY_EFFECT`], rides the ribbon
    /// rather than a craft, so it is driven from
    /// [`Self::advance_leach_beam_ribbon`].
    pub(in crate::race) fn advance_leach_beam_visual(&mut self) {
        // The charge: up exactly while slot 0 holds a LeachBeam, which is the
        // whole of `FUN_0883f540`'s own gate. Nothing chosen here.
        let holding = self.sim.world.ships[0].pickup.weapon
            == Some(oag_tables::weapons::Weapon::LeachBeam)
            && self.sim.world.ships[0].active;
        let holder = self.sim.world.ships[0].physics.body.position;
        match (
            holding.then(|| self.view.effects.get(LEACHBEAM_CHARGING_EFFECT).cloned()),
            self.view.leach_charge_effect,
        ) {
            (Some(Some(effect)), None) => {
                self.view.leach_charge_effect = self.view.stage.attach(&effect, holder, 1.0);
            }
            (Some(Some(_)), Some(playing)) => self.view.stage.follow(playing, holder),
            // Not holding one any more (or the file never loaded): tear the
            // instance down, the same way the original despawns it the moment
            // its own two-part gate stops holding.
            (None, Some(playing)) | (Some(None), Some(playing)) => {
                self.view.stage.detach(playing);
                self.view.leach_charge_effect = None;
            }
            (None, None) | (Some(None), None) => {}
        }
    }

    /// One tick of `LeachBeam_Advance`'s presentation half for as long as a
    /// *locked* beam exists - through its disconnect linger too, which the
    /// original also advances (measured: the ribbon's cursor steps and the
    /// firing craft's pulse strength is written on every frame of the linger).
    /// An [`oag_gameplay::projectile::leach_beam::Kind::Unlocked`] beam draws
    /// nothing.
    ///
    /// Advances [`RaceView::leach_beam_ribbon`] (see [`oag_render::beam`]) and
    /// keeps [`LEACHBEAM_ENERGY_EFFECT`] on it: **re-spawned every time the
    /// ribbon's cursor wraps** - the pulse block that also plays
    /// `LEACHENERGY` - one segment short of the target, then walked back
    /// toward the shooter one chain point a tick, which is where
    /// `LeachBeam_Advance` writes the effect's own matrix. The previous
    /// instance is detached at each re-spawn so its particles finish where
    /// they are; the original hands its handle to `FUN_088f3298`, whose effect
    /// on live particles was not read, so the detach is **chosen, not
    /// measured**.
    pub(in crate::race) fn advance_leach_beam_ribbon(&mut self) {
        let locked = self
            .sim
            .world
            .leach_beam
            .filter(|beam| beam.kind == oag_gameplay::projectile::leach_beam::Kind::Locked);
        let Some(beam) = locked else {
            self.view.leach_beam_ribbon = None;
            if let Some(playing) = self.view.leach_beam_effect.take() {
                self.view.stage.detach(playing);
            }
            return;
        };
        let owner = self.sim.world.ships[beam.owner as usize]
            .physics
            .body
            .position;
        let target = self.sim.world.ships[beam.target as usize]
            .physics
            .body
            .position;
        let dt = self.sim.dt;
        // Split the two fields rather than borrowing `self.view` twice at
        // once - the same idiom `Race::force_trail_sparks`'s own
        // `(player, rng)` pair uses.
        let (ribbon, rng) = (
            &mut self.view.leach_beam_ribbon,
            &mut self.view.leach_beam_rng,
        );
        let ribbon = ribbon.get_or_insert_with(|| oag_render::beam::Ribbon::new(rng));
        let pulsed = ribbon.advance(dt, (target - owner).length(), beam.range, rng);
        let at = ribbon.energy_point(owner, target, beam.range);

        let effect = self.view.effects.get(LEACHBEAM_ENERGY_EFFECT).cloned();
        match (effect, self.view.leach_beam_effect, at) {
            (Some(effect), playing, Some(at)) if pulsed || playing.is_none() => {
                if let Some(playing) = playing {
                    self.view.stage.detach(playing);
                }
                self.view.leach_beam_effect = self.view.stage.attach(&effect, at, 1.0);
            }
            (_, Some(playing), Some(at)) => self.view.stage.follow(playing, at),
            _ => {}
        }
    }

    /// The firing craft's LeachBeam hull-overlay pulse on `slot` this tick, or
    /// `None` when it draws nothing: `Mesh_DrawBatchSet` gates
    /// `HullOverlay_SubmitLeachBeamBatched` on a live beam
    /// (`DAT_08b317ac == 2`) and on the craft's weapon record `+0` being
    /// positive, and `LeachBeam_UpdatePool` writes that `+0` with
    /// `LeachBeam_PulseStrength` every tick - measured in play, linger
    /// included. See [`oag_render::beam::Ribbon::pulse_strength`].
    #[must_use]
    pub fn leach_overlay_pulse(&self, slot: usize) -> Option<f32> {
        let beam = self.sim.world.leach_beam?;
        if usize::from(beam.owner) != slot {
            return None;
        }
        self.view.leach_beam_ribbon.as_ref()?.pulse_strength()
    }

    /// The ribbon's geometry for this frame, or empty when there is no
    /// locked beam to draw - see [`oag_render::beam::build`].
    ///
    /// `camera_right`/`camera_up` are the view's own world-space axes, which
    /// the two strips are widened along. `alpha` is the link's own coverage:
    /// `1.0` while connected, and while disconnected a linear fade to `0.0`
    /// over [`oag_gameplay::projectile::leach_beam::DISCONNECT_LINGER_SECONDS`] -
    /// `LeachBeam_BuildStrip`'s own recovered alpha write.
    #[must_use]
    pub(in crate::race) fn leach_beam_ribbon_vertices(
        &self,
        camera_right: oag_core::math::Vec3,
        camera_up: oag_core::math::Vec3,
    ) -> Vec<oag_render::mesh::GpuVertex> {
        use oag_gameplay::projectile::leach_beam::{DISCONNECT_LINGER_SECONDS, Kind};

        let (Some(beam), Some(ribbon)) = (self.sim.world.leach_beam, &self.view.leach_beam_ribbon)
        else {
            return Vec::new();
        };
        if beam.kind != Kind::Locked {
            return Vec::new();
        }
        let alpha = match beam.disconnected_at {
            None => 1.0,
            Some(disconnected_at) => {
                (1.0 - (beam.age - disconnected_at) / DISCONNECT_LINGER_SECONDS).clamp(0.0, 1.0)
            }
        };
        let shooter = &self.sim.world.ships[beam.owner as usize];
        // The shooter's node basis: the drawn hull's own rotation, the body's
        // orientation with its visual roll - `drawable::model_matrix_of`
        // without the model-space yaw and scale.
        let rotation = shooter.physics.body.orientation
            * oag_render::roll::rotation(oag_core::math::Vec3::NEG_Z, shooter.physics.roll_phase);
        let frame = oag_render::beam::Frame {
            owner: shooter.physics.body.position,
            target: self.sim.world.ships[beam.target as usize]
                .physics
                .body
                .position,
            owner_right: rotation * oag_core::math::Vec3::X,
            owner_up: rotation * oag_core::math::Vec3::Y,
            camera_right,
            camera_up,
            range: beam.range,
            alpha,
        };
        oag_render::beam::build(ribbon, &frame)
    }

    /// Keeps [`QUAKE_EFFECT`] and its own transform riding the travelling
    /// wave, one instance for the whole race.
    ///
    /// Needs `self.sim.course` (to place the wave along the ring) and
    /// `self.sim.spline` (to read the track's own width there), which is why this
    /// cannot live in `oag_gameplay::projectile::quake` at all - see that
    /// module's own doc comment on the split.
    ///
    /// **Recovered position and scale, chosen orientation.** `Quake_Update`
    /// builds its transform from two edge points sampled across the track at
    /// the wave's own position: position is their midpoint, scale is their
    /// separation divided by `50.0` -
    /// `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`'s "What
    /// `Quake_Update` builds from those two points" section. This engine has
    /// no per-progress edge-point record of its own the way the original's
    /// `SplinePt` does; it recovers the same two points from
    /// [`Spline`]'s own sample at the ring point nearest the wave's own
    /// progress - `pos`/`lateral`/`half_width_left`/`half_width_right`, the
    /// same fields [`oag_render::track::build_model`] already draws the
    /// ribbon's own edges from. **Orientation is not established by anything
    /// read** - the original's own basis build never consumes its
    /// `AiTrack_LocatePosition` call, confirmed independently this pass by
    /// re-deriving `Quake_Update` rather than only quoting the earlier read -
    /// so this draws the effect axis-aligned at the recovered position and
    /// scale alone, which is **chosen, not measured; no confidence score**.
    ///
    /// Detaches the instance the tick the wave goes away (`self.sim.world.quake`
    /// becomes `None`), the same "hand the slot back, let the particles fade"
    /// shape [`Race::advance_projectile_flares`] already takes.
    pub(in crate::race) fn advance_quake_visual(&mut self) {
        let Some(wave) = self.sim.world.quake else {
            if let Some(playing) = self.view.quake_effect.take() {
                self.view.stage.detach(playing);
            }
            return;
        };
        let Some(course) = self.sim.course.as_ref() else {
            return;
        };
        let Some(index) = course_index_near_progress(course, wave.progress) else {
            return;
        };
        let Some(position) = course.position(index) else {
            return;
        };
        let Some((_, sample, _)) = self.sim.spline.nearest(position) else {
            return;
        };
        let lateral = Vec3::from_array(sample.lateral);
        let centre = Vec3::from_array(sample.pos);
        let left = centre - lateral * sample.half_width_left;
        let right = centre + lateral * sample.half_width_right;
        let midpoint = (left + right) * 0.5;
        let scale = (right - left).length() / 50.0;

        match (
            self.view.effects.get(QUAKE_EFFECT).cloned(),
            self.view.quake_effect,
        ) {
            (Some(effect), None) => {
                self.view.quake_effect = self.view.stage.attach(&effect, midpoint, scale);
            }
            (Some(_), Some(playing)) => {
                self.view.stage.follow(playing, midpoint);
                self.view.stage.rescale(playing, scale);
            }
            (None, _) => {}
        }
    }
    /// Plays the explosion a weapon that just went off authored - its own,
    /// not another weapon's.
    ///
    /// **Recovered for the Rocket, the Missile, the Mine and the Bomb.** See
    /// [`Race::blast_for`] for the map and the reading behind each arm.
    /// `orientation` is the frozen pose a laid charge detonated with
    /// ([`oag_gameplay::projectile::mine::frozen_pose`]) - unused by every
    /// arm but the Bomb's, which needs it to place its own two `.vex`
    /// models; see [`Self::spawn_bomb_blast_model`].
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
    /// took no `kind` at all; until 2026-09-23 a Bomb drew nothing at all.
    ///
    /// [`RaceView::sparks`] is deliberately not reused for it: that system
    /// re-anchors to the hull every tick, so a burst ignited at an impact
    /// would emit from the craft instead. It is one hull-mounted emitter, and
    /// this is why the stage exists alongside it.
    ///
    /// [`blast_for`]: Race::blast_for
    pub(in crate::race) fn ignite_blast(
        &mut self,
        kind: oag_tables::weapons::Weapon,
        point: Vec3,
        struck: Option<usize>,
        orientation: Quat,
    ) {
        let Some((name, at)) = self.blast_for(kind, point, struck) else {
            return;
        };
        // The Plasma's own three-model detonation - `PLASMA_BLAST_EFFECT`
        // below is `WO_PLASMA_FLASH`, a separate `.pob` particle system; the
        // halo and two hemispheres are their own `.vex` models with their
        // own render-side pool, on the same terms as the Rocket's, the
        // Mine's and the Bomb's own bodies. See `blast_models` module doc
        // comment for why that pool lives on `self.view` and is advanced
        // independently of this effect stage.
        if kind == oag_tables::weapons::Weapon::Plasma {
            self.spawn_plasma_blast_model(at);
        }
        // The Bomb's own two-model detonation - `BOMB_SMOKERING_EFFECT`
        // below is one third of it, the psys smoke ring; the hemisphere and
        // the shockwave are their own `.vex` models, on `bomb_blast`'s own
        // render-side pool, the same shape as the Plasma's own three above.
        if kind == oag_tables::weapons::Weapon::Bomb {
            self.spawn_bomb_blast_model(at, orientation);
        }
        let Some(effect) = self.view.effects.get(name).cloned() else {
            return;
        };
        // Neutral severity: the field the collision sparks derive from an
        // impulse is the *hull's*, and nothing on the rocket path has been
        // read as feeding it. See `oag_render::sparks::severity`.
        self.view.stage.play(&effect, at, 1.0);
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
    /// - **`Cannon`**: [`CANNON_SPARKS_EFFECT`] when `struck` is `None` - a
    ///   wall or track hit - and nothing when it is `Some`. `Cannon_UpdateRound`
    ///   (`0x0886593c`) only calls `Psys_Spawn_q` off its own world-collision
    ///   raycast; the separate craft-proximity test that produces a `Some`
    ///   `struck` here (`FUN_088579a8`/`FUN_08857f2c`) applies damage and a
    ///   sound cue but never spawns a particle effect. See
    ///   `oag_gameplay::projectile::cannon`'s module doc for the full read.
    /// - **`Plasma`**: [`PLASMA_BLAST_EFFECT`] always, whatever it struck, the
    ///   same shape as the Missile's and the Mine's. `Plasmas_Update`
    ///   (`0x0886b490`) runs one teardown pass over every bolt carrying the
    ///   destroy bit and calls `Plasma_SpawnDetonation` (`0x0886ac88`) with the
    ///   bolt's own position for each - a wall hit and a timed-out bolt reach
    ///   it identically, so there is no split to mirror. Wired 2026-09-09; it
    ///   returned `None` before, when the teardown was unread. **On HD,
    ///   [`PLASMA_LIGHTNING_EXPAND_EFFECT`] instead** - a different
    ///   executable's own file, read 2026-09-17; see that constant's doc
    ///   comment.
    /// - **`Bomb`**: [`BOMB_SMOKERING_EFFECT`] always, whatever it struck -
    ///   `Bomb_Detonate` (`0x088640c8`) calls `BombBlast_Construct`
    ///   unconditionally on `play_visual`, which every caller in this
    ///   engine's own fuse/trip paths sets. That is one third of the
    ///   detonation: the other two are `explosion_hemisphere.vex` and
    ///   `Bomb_Shockwave.vex`, their own `.vex` models this function cannot
    ///   carry - [`Self::ignite_blast`]'s own Bomb branch spawns those into
    ///   `bomb_blast`'s render-side pool alongside this effect. See
    ///   `docs/ghidra/functions/psp-pulse-usa/mine.md#2026-09-23-the-blasts-own-per-tick-animator-read`.
    /// - **Everything else**: `None`. Guessing here - playing the Mine's
    ///   file, or the Rocket's, which is what this did for every kind before
    ///   2026-08-26 - is exactly the invention `CLAUDE.md` forbids.
    pub(in crate::race) fn blast_for(
        &self,
        kind: oag_tables::weapons::Weapon,
        point: Vec3,
        struck: Option<usize>,
    ) -> Option<(&'static str, Vec3)> {
        match kind {
            oag_tables::weapons::Weapon::Rocket => Some(match struck {
                Some(slot) if self.sim.world.ships[slot].active => (
                    CRAFT_BLAST_EFFECT,
                    self.sim.world.ships[slot].physics.body.position - Vec3::Y * CRAFT_BLAST_DROP,
                ),
                // A craft that has gone inactive since the hit falls back to
                // the impact point rather than reading a stale pose - still
                // its own effect, because what was struck is what chose the
                // file.
                Some(_) => (CRAFT_BLAST_EFFECT, point),
                None => (TRACK_BLAST_EFFECT, point),
            }),
            oag_tables::weapons::Weapon::Missile => Some((MISSILE_EXPLO_EFFECT, point)),
            oag_tables::weapons::Weapon::Mine => Some((MINE_EXPLO_EFFECT, point)),
            oag_tables::weapons::Weapon::Bomb => Some((BOMB_SMOKERING_EFFECT, point)),
            oag_tables::weapons::Weapon::Cannon if struck.is_none() => {
                Some((CANNON_SPARKS_EFFECT, point))
            }
            // HD plays its own file, `WO_PLASMA_LIGHTNING_EXPAND` -
            // `WeaponExplosions_Start`, not `Plasma_SpawnDetonation`'s
            // `WO_PLASMA_FLASH`. `WO_PLASMA_LIGHTNING_COLLAPSE`, the other
            // half of HD's pair, fires later and is not from here - see
            // `Race::advance_plasma_blast_models`.
            oag_tables::weapons::Weapon::Plasma => Some((
                if self.view.hd_plasma_blast {
                    PLASMA_LIGHTNING_EXPAND_EFFECT
                } else {
                    PLASMA_BLAST_EFFECT
                },
                point,
            )),
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
        for (slot, projectile) in self.sim.world.projectiles.slots.iter().enumerate() {
            if !bounced_this_tick(projectile.kind, before[slot], projectile.bounces) {
                continue;
            }
            // Looked up per slot rather than once outside the loop, because two
            // weapons bounce and they play different files - see
            // [`bounce_effect_for`]. A weapon whose file did not load plays
            // nothing and does not fall back to the other's.
            let Some(effect) = bounce_effect_for(projectile.kind)
                .and_then(|name| self.view.effects.get(name))
                .cloned()
            else {
                continue;
            };
            self.view.stage.play(&effect, projectile.position, 1.0);
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
    /// [`oag_vex::pob::flags::LOOPING`], so it runs until detached.
    ///
    /// Anchored at the `Engine Flare` locator under the craft's *current*
    /// transform, the same point the procedural flare uses, so the two are
    /// interchangeable rather than merely similar.
    pub(in crate::race) fn advance_engine_flares(&mut self) {
        let Some(effect) = self.view.effects.get(ENGINE_FLARE_EFFECT).cloned() else {
            return;
        };
        for slot in 0..MAX_SHIPS {
            let nozzle = self.sim.world.ships[slot]
                .active
                .then(|| self.nozzle_of(slot))
                .flatten();
            match (nozzle, self.view.engine_flare[slot]) {
                (Some(at), Some(playing)) => self.view.stage.follow(playing, at),
                (Some(at), None) => {
                    self.view.engine_flare[slot] = self.view.stage.attach(&effect, at, 1.0)
                }
                // Gone or never had a locator: hand the instance back and
                // let its particles fade rather than cutting them.
                (None, Some(playing)) => {
                    self.view.stage.detach(playing);
                    self.view.engine_flare[slot] = None;
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
    /// flight; every emitter involved is [`oag_vex::pob::flags::LOOPING`],
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
        for (slot, projectile) in self.sim.world.projectiles.slots.iter().enumerate() {
            let name = flare_effect_for(projectile.kind);
            let (primary, orbiting) = if projectile.kind
                == Some(oag_tables::weapons::Weapon::Missile)
            {
                let age = oag_gameplay::projectile::MAX_FLIGHT_SECONDS - projectile.lifetime;
                let (a, b) = missile_flare_anchors(projectile.position, projectile.velocity, age);
                (a, Some(b))
            } else {
                (projectile.position, None)
            };
            // Only the primary anchor ever carries a non-neutral scale - see
            // `plasma_flare_scale`. The Missile's orbiting anchor rides at
            // the stage's own neutral `1.0`, the same as a Rocket or a
            // Shuriken.
            //
            // **The halving condition, worked out from two sites.** `craft+0x6d`
            // is only ever written for the *player's* own craft - camera.md's
            // SELECT-view cycle sets it on "the player craft" alone - so a
            // charging opponent's bolt reads the byte at its own, forever-zero
            // `+0x6d` and never halves; only a *player-owned* charging bolt can.
            // `self.draws_own_ship()` is this engine's own reading of that same
            // byte (`oag_display::CameraView::draws_own_ship`, confidence 82 -
            // raised from 70 once `shield-pickup.md` found a second, independent
            // consumer), inverted (`craft+0x6d == 1` means *hide* the ship, which
            // is `draws_own_ship() == false`) and race-wide rather than per-craft
            // because only the player's own camera can be internal at all.
            let cockpit = projectile.owner == 0 && !self.draws_own_ship();
            let scale = plasma_flare_scale(projectile.kind, projectile.charge, cockpit);
            advance_one_flare(
                &mut self.view.stage,
                &self.view.effects,
                name,
                primary,
                scale,
                &mut self.view.projectile_flare[slot],
            );
            // The Rocket's flare frame is its own basis turned `-pi/2` about
            // row 0, so the emitter's `+Y` is the velocity - measured live,
            // `rocket-visuals.md`'s 2026-09-24 section. Every other rider
            // keeps world up: none of their frames was read.
            if let (Some(oag_tables::weapons::Weapon::Rocket), Some(instance)) =
                (projectile.kind, self.view.projectile_flare[slot])
            {
                let up = projectile.velocity.try_normalize().unwrap_or(Vec3::Y);
                self.view.stage.orient(instance, up);
            }
            // The Missile's second, orbiting anchor - see `missile_flare_anchors`.
            // `orbiting` is `None` for every other kind, so this rides nothing
            // and only ever tears down a leftover instance from a slot that
            // was a Missile last tick.
            advance_one_flare(
                &mut self.view.stage,
                &self.view.effects,
                orbiting.and(name),
                orbiting.unwrap_or(primary),
                1.0,
                &mut self.view.projectile_flare_orbit[slot],
            );
        }
    }

    /// This frame's fallback projectile sprites, in the engine flare's shape.
    ///
    /// `right` and `up` are the camera's, read out of the view matrix the same
    /// way the exhaust's are.
    ///
    /// `modelled` says, **per live projectile's own kind**, whether the
    /// caller is already drawing it as a mesh - [`ROCKET_MODEL_ENTRY`],
    /// [`MINE_MODEL_ENTRY`] or [`BOMB_MODEL_ENTRY`] - in which case this draws
    /// **nothing** for that one: the glow around a modelled rocket is
    /// [`ROCKET_FLARE_EFFECT`], played off the disc through [`RaceView::stage`],
    /// and a billboard on top of it would be a second invented one. What is
    /// left here is the case where a kind's own model did not load, or a kind
    /// (the Missile, the Plasma, the Shuriken) has no model at all, and a
    /// projectile would otherwise be invisible - see
    /// [`PROJECTILE_SPRITE_HALF_SIZE`].
    ///
    /// **Was one flag for every kind at once, fixed 2026-09-05 alongside
    /// [`Self::projectile_model_matrices`]'s own kind filter.** A single
    /// `bool` gated *all* projectiles on whether the Rocket's model happened
    /// to load, so on a real disc (where it does) a live Missile, Plasma or
    /// Shuriken drew nothing at all: no mesh, because it authors none, and no
    /// billboard, because the flag said "modelled" for a kind it was never
    /// about. `modelled` is now asked once per live projectile's own kind.
    #[must_use]
    pub fn projectile_sprites(
        &self,
        right: Vec3,
        up: Vec3,
        modelled: impl Fn(oag_tables::weapons::Weapon) -> bool,
    ) -> Vec<oag_render::mesh::GpuVertex> {
        let mut vertices = Vec::new();
        for projectile in &self.sim.world.projectiles.slots {
            let Some(kind) = projectile.kind else {
                continue;
            };
            if modelled(kind) {
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
    /// **The basis is recovered, and the quarter-turn is not the model's.**
    /// `Rocket_Update` (`0x0885d2a8`) rebuilds a basis every tick at
    /// `rocket+0x60`: row 0 `n x f`, row 1 the surface normal `n`
    /// re-orthogonalised against `f`, row 2 the normalised velocity `f`, row 3
    /// the position - a rotation, determinant `+1`. That matrix is what it
    /// copies into the model's scene node (`0x08945284`). Only *after* that
    /// does it copy the basis to `rocket+0xa0` and turn the copy `-pi/2`
    /// about its own row 0 (`Math_RotateByAxisAngle`, `0x08a6b6b4`) - and
    /// `+0xa0` is the frame `Rocket_Init` handed `WO_ROCKET_FLARE` by pointer,
    /// so the quarter-turn orients the flare, never the dart. Confidence 85;
    /// see `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`'s
    /// 2026-09-24 section.
    ///
    /// The original's second basis vector is the surface normal the rocket
    /// rides (`rocket+0x100`), which this engine carries as
    /// `Projectile::surface` - so that is what the side axis is built from,
    /// with world up only as the fallback when the rocket flies along it.
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
        self.projectile_model_matrices(oag_tables::weapons::Weapon::Rocket)
    }

    /// Where each live mine is, and the pose it landed in, for the model draw.
    ///
    /// **No longer translation only.** `oag_gameplay::projectile::mine::at_rest`
    /// is the recovered velocity a laid mine carries - zero, so
    /// [`Self::projectile_model_matrices`]'s `forward == Vec3::ZERO` branch is
    /// exactly what fires here - but that branch now draws
    /// `Projectile::orientation` rather than a bare translation. See
    /// [`oag_gameplay::projectile::mine::frozen_pose`] for what `Mine_Init`
    /// copies at drop, at what confidence, and which half of this reading is
    /// chosen rather than measured.
    #[must_use]
    pub fn mine_model_matrices(&self) -> Vec<Mat4> {
        self.projectile_model_matrices(oag_tables::weapons::Weapon::Mine)
    }

    /// Where each live bomb is, for the model draw - the Mine's, one size up.
    ///
    /// Same reading as [`Self::mine_model_matrices`], carried over on the same
    /// terms [`oag_gameplay::projectile::mine::frozen_pose`] already applies
    /// [`oag_gameplay::projectile::mine::at_rest`] to both weapons: the Bomb's
    /// own spawn helper is unread, so nothing confirms the same matrix-copy
    /// happens there, and nothing rules it out either.
    #[must_use]
    pub fn bomb_model_matrices(&self) -> Vec<Mat4> {
        self.projectile_model_matrices(oag_tables::weapons::Weapon::Bomb)
    }

    /// Where each live Cannon round is, for the model draw.
    ///
    /// **The model is the one the original hangs on the round, and nothing
    /// beside it.** `Cannon_Construct` (`0x088651d8`) loads
    /// [`CANNON_MODEL_ENTRY`] into every round instance's own scene node, so
    /// this is playing the disc's data rather than standing in for it. Each
    /// round *also* builds two display lists of hand-written quads textured
    /// from the disc's `Cannon_bolt.mip` and `Cannon_muzzle_flash.mip`.
    ///
    /// **Which list is which is now settled, confidence 88 - not the load-order
    /// guess this comment used to carry at confidence 40.** The per-round draw
    /// function, `FUN_0886545c` (EU `FUN_088652b8`), is instruction-level
    /// unambiguous: it binds `g_cannon_bolt_texture` and calls
    /// `Gu_CallList(instance+0x240)` - the list `FUN_08864cd0` (EU
    /// `FUN_08864b2c`) built from `instance+0x100`/`+0x160` - every frame a
    /// round is live; then, **only while `instance+0xc8 < 0.1` seconds since
    /// spawn**, it binds `g_cannon_muzzle_flash_texture` and calls
    /// `Gu_CallList(instance+0x440)` - the list `FUN_08864dc4` (EU
    /// `FUN_08864c20`) built from `instance+0x1c0`, sized and coloured from
    /// `Psys_RandIntRange(0x96,0xff)` each time it is rebuilt. So
    /// `FUN_08864cd0`/`FUN_08864b2c` is the **bolt** (drawn for the round's
    /// whole flight, as a streak between its previous and current position)
    /// and `FUN_08864dc4`/`FUN_08864c20` is the **muzzle flash** (drawn only
    /// for the round's first tenth of a second, sized and tinted at random).
    /// This is an independent, stronger check than the archive's
    /// entry-adjacency cross-check (1057/1058) the earlier reading proposed
    /// but never spent - it confirms the same answer.
    ///
    /// **Neither is drawn here regardless**, and that is still an honest
    /// absence rather than a regression: both are hand-authored GU quads with
    /// their own two `.mip` textures, not `Data\Psys` particle effects, so
    /// drawing them for real needs a textured-billboard path this project's
    /// mesh/texture-upload machinery does not yet expose outside
    /// `oag_render::mesh_render` - reaching into it is out of this pass's own
    /// file ownership. Inventing an untextured stand-in quad instead would be
    /// exactly the kind of plausible-looking invention `CLAUDE.md` forbids,
    /// so this stays as a documented next step rather than an approximation.
    /// See `docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`.
    ///
    /// Velocity-oriented like the Rocket's rather than pose-oriented like the
    /// Mine's, for the reason the shared helper below gives: a round is a body
    /// in flight with a direction of travel, and
    /// `oag_gameplay::projectile::cannon::launch` gives it no independent
    /// orientation to read.
    #[must_use]
    pub fn cannon_model_matrices(&self) -> Vec<Mat4> {
        self.projectile_model_matrices(oag_tables::weapons::Weapon::Cannon)
    }

    /// Where each live Plasma bolt's own head is, for the model draw.
    ///
    /// **HD only** - `oag_gameplay`'s `Projectile` carries no charge/flight
    /// distinction the mesh needs to be hidden during, so this draws
    /// whenever a Plasma is live, charging or flying, the same set
    /// [`Race::advance_projectile_flares`] rides the glow on. Velocity-
    /// oriented like the Rocket's and the Cannon round's: `Plasma_PostUpdate`
    /// (`0x00121418`) does place the bolt with a full basis, not a bare
    /// translation, but that function's own second axis (a carried surface
    /// normal, at confidence roughly 55 - not renamed, per `CLAUDE.md`'s
    /// below-70 rule) is not resolved enough to port. **Chosen, not
    /// measured**: this reuses the Rocket's velocity-alone basis rather than
    /// inventing the second vector, and a charging bolt (`velocity ==
    /// Vec3::ZERO`) falls into [`Self::projectile_model_matrices`]'s own
    /// `orientation` branch, which for a projectile that has never flown is
    /// simply the identity - an upright bolt on the craft's nose, not a
    /// measured wind-up pose. See
    /// `docs/ghidra/functions/ps3-hdfury-eu/plasma.md`.
    #[must_use]
    pub fn plasma_ball_model_matrices(&self) -> Vec<Mat4> {
        self.projectile_model_matrices(oag_tables::weapons::Weapon::Plasma)
    }

    /// This frame's vertices for every live Cannon round's two hand-built
    /// quads: the bolt streak into `bolt`, the muzzle flash into `flash`
    /// while the round's age is under
    /// [`oag_render::weapon_quads::geometry::FLASH_WINDOW_SECONDS`].
    ///
    /// `right`/`up` are the camera's own basis vectors, the same ones
    /// [`Self::projectile_sprites`] takes. **The flash's rotation, size and
    /// alpha are rolled from [`oag_render::weapon_quads::random::flash_roll`],
    /// seeded from the round's own slot index and [`World::tick`] - not from
    /// `self.sim.world.rng`.** Rolling it from the simulation's own seeded
    /// stream would advance that stream once per live round per tick for a
    /// value nothing in `oag_gameplay` ever reads back, moving every
    /// committed determinism hash for a purely cosmetic reason - see
    /// `oag_render::weapon_quads::random`'s own doc comment.
    ///
    /// The previous tick's position - the bolt's other endpoint - is derived
    /// as `position - velocity * dt` rather than stored on `Projectile`,
    /// since both are already there and adding a field for one draw call
    /// would be new simulation state for a render-only need.
    pub fn cannon_quad_vertices(
        &self,
        right: Vec3,
        up: Vec3,
        bolt: &mut Vec<oag_render::mesh::GpuVertex>,
        flash: &mut Vec<oag_render::mesh::GpuVertex>,
    ) {
        use oag_render::weapon_quads::{geometry, random};
        let dt = TickRate::DEFAULT.dt();
        let tick = self.sim.world.tick as u32;
        for (slot, projectile) in self.sim.world.projectiles.slots.iter().enumerate() {
            if projectile.kind != Some(oag_tables::weapons::Weapon::Cannon) {
                continue;
            }
            let curr = projectile.position;
            let prev = curr - projectile.velocity * dt;
            bolt.extend(geometry::bolt_vertices(prev, curr, right, up));
            let age = oag_gameplay::projectile::MAX_FLIGHT_SECONDS - projectile.lifetime;
            if age < geometry::FLASH_WINDOW_SECONDS {
                let (rotation, half_size, alpha) = random::flash_roll(slot as u32, tick);
                flash.extend(geometry::flash_vertices(
                    curr, right, up, half_size, rotation, alpha,
                ));
            }
        }
    }

    /// Where each live projectile of one `kind` is and how it is oriented, for
    /// the model draw - shared by [`Self::rocket_model_matrices`],
    /// [`Self::mine_model_matrices`], [`Self::bomb_model_matrices`] and
    /// [`Self::cannon_model_matrices`].
    ///
    /// **Filters on `kind` specifically, which is the fix over what this did
    /// before 2026-09-05.** Before, the filter was `kind.is_some()` - any live
    /// projectile at all - so a live Missile, Plasma, Shuriken, Mine or Bomb
    /// was drawn as a **Rocket** mesh whenever one was airborne alongside a
    /// loaded [`ROCKET_MODEL_ENTRY`], because [`Race::rocket_model_matrices`]'s
    /// one caller zips its result against the Rocket's own drawables with no
    /// kind check on either side. No test caught it because every test here
    /// only ever spawns a single Rocket - see
    /// `a_rocket_and_a_missile_in_flight_do_not_share_a_model_slot` for the
    /// one that does not. The same shape of bug, in the model path rather
    /// than the effect path, as the mine that used to ride
    /// [`ROCKET_FLARE_EFFECT`] from the moment it landed - see
    /// [`Race::advance_projectile_flares`]'s doc comment.
    ///
    /// **Neither branch applies a [`MODEL_YAW`]-style correction.** `MODEL_YAW`
    /// exists because `Ship.vex`'s own hull is authored with its nose along a
    /// different axis than [`oag_physics::Body::forward`] uses, found by
    /// measuring which end of the mesh is wider - a **per-model** fact, not a
    /// property of `.vex` as a format. The velocity-built branch above has a
    /// direct check: `the_rocket_model_is_longest_along_the_axis_it_is_flown_down`
    /// measures `Pulse_Rocket.vex` against its own flight direction and needs
    /// no such correction. **The `orientation` branch's model was viewed the
    /// same way `MODEL_YAW` itself was measured** (`oag-view --mesh`,
    /// `Pulse_Mine.vex`) and turns out to be a caltrop - three spikes at
    /// roughly 120° about one axis, not a directional hull - so a yaw error
    /// about that axis has no "wrong way round" to fall into the way a ship's
    /// nose/tail does. Left open is only whether that axis itself agrees with
    /// [`Body::orientation`]'s convention, a coarser question a single static
    /// mesh view cannot settle; `Pulse_Bomb.vex` was not separately viewed.
    ///
    /// **Both branches are rotations (determinant `+1`) since 2026-09-24.**
    /// The velocity branch used to build `side = forward x reference`, `up =
    /// side x forward` - a reflection - so every Rocket, Cannon round and HD
    /// Plasma head drew mirrored, and none could take its material's own
    /// back-face cull (`load::weapon_models::cull_as_authored`). It now builds
    /// `side = reference x forward`, `up = forward x side`, which is the
    /// original's own `Rocket_Update` basis, measured live, with
    /// `Projectile::surface` as its `n`. The `orientation` branch (the laid
    /// Mine and Bomb) was always a rotation; on Pulse it is now the
    /// executable's own measured pose instead - see `laid`.
    #[must_use]
    fn projectile_model_matrices(&self, kind: oag_tables::weapons::Weapon) -> Vec<Mat4> {
        self.sim
            .world
            .projectiles
            .slots
            .iter()
            .enumerate()
            .filter(|(_, projectile)| projectile.kind == Some(kind))
            .map(|(slot, projectile)| {
                let forward = projectile.velocity.normalize_or_zero();
                if forward == Vec3::ZERO {
                    // Every weapon that reaches this branch lays rather than
                    // flies (`oag_gameplay::projectile::mine::at_rest`). On
                    // Pulse the pose is the executable's own, measured - see
                    // `laid`; elsewhere `orientation` is the frozen pose it
                    // landed with, chosen, not measured.
                    return laid::matrix(slot, projectile, self.view.pulse_laid_pose);
                }
                // `surface` is the normal the projectile rides - the
                // original's own `n` (`rocket+0x100`), seeded to world up at
                // spawn. It, then world up, then any perpendicular, so a
                // projectile flying along one of them still gets a basis.
                let reference = [projectile.surface, Vec3::Y]
                    .into_iter()
                    .find(|axis| forward.dot(*axis).abs() < 0.999)
                    .unwrap_or_else(|| forward.any_orthonormal_vector());
                // `reference x forward`, then `forward x side`: a rotation
                // (determinant `+1`), mapping the model's own `+X` onto
                // `up x forward` - `Rocket_Update`'s own row 0 (`n x f`,
                // `vcrsp.t` at `0x0885d988`). The other order is a
                // reflection, and drew every one of these mirrored until
                // 2026-09-24.
                let side = reference.cross(forward).normalize_or_zero();
                let up = forward.cross(side);
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
