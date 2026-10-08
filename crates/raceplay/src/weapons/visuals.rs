//! What a fired weapon *shows*: the blasts [`Race::ignite_blast`] plays, the
//! flares riding each live projectile, the Missile's own wall-bounce burst,
//! and the sprites/matrices a caller with no model draws them as instead.
//!
//! Split out of `weapons.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. Its tests
//! are in `race/tests/weapons.rs` and `race/tests/scene.rs`, alongside the
//! rest of the weapon-visuals coverage.

mod cannon;
mod flares;
mod flash;
mod hooks;
mod laid;
mod quake;
pub(crate) use cannon::{CannonAssets, CannonDraw};
pub(crate) use flares::*;
pub(crate) use flash::flash_for;

use super::*;

impl Race {
    /// Keeps [`Trigger::LeachbeamCharging`] on every craft that is holding a
    /// LeachBeam - the disc's own `Data\Psys\*.POB`, played through the same
    /// [`psys::Stage`] every other weapon's are, with its trigger read out of
    /// the executable (see [`Trigger::LeachbeamCharging`]).
    ///
    /// The beam's other effect, [`Trigger::LeachbeamEnergy`], rides the ribbon
    /// rather than a craft, so it is driven from
    /// [`Self::advance_leach_beam_ribbon`].
    pub(crate) fn advance_leach_beam_visual(&mut self) {
        let effect = self.view.handles.get(Trigger::LeachbeamCharging).cloned();
        for slot in 0..MAX_SHIPS {
            // The charge: up exactly while the craft holds a LeachBeam and is
            // in state 1, which is the whole of `FUN_0883f540`'s own gate, run
            // for every craft. Nothing chosen here.
            let ship = &self.sim.world.ships[slot];
            let holding =
                ship.pickup.weapon == Some(oag_tables::weapons::Weapon::LeachBeam) && ship.active;
            let holder = ship.physics.body.position;
            match (
                holding.then_some(&effect),
                self.view.leach_charge_effect[slot],
            ) {
                (Some(Some(effect)), None) => {
                    self.view.leach_charge_effect[slot] =
                        self.view.stage.attach(effect, holder, 1.0);
                }
                (Some(Some(_)), Some(playing)) => self.view.stage.follow(playing, holder),
                // Not holding one any more (or the file never loaded): tear
                // the instance down, the same way the original despawns it the
                // moment its own two-part gate stops holding.
                (None, Some(playing)) | (Some(None), Some(playing)) => {
                    self.view.stage.detach(playing);
                    self.view.leach_charge_effect[slot] = None;
                }
                (None, None) | (Some(None), None) => {}
            }
        }
    }

    /// One tick of `LeachBeam_Advance`'s presentation half for as long as a
    /// *locked* beam exists - through its disconnect linger too, which the
    /// original also advances (measured: the ribbon's cursor steps and the
    /// firing craft's pulse strength is written on every frame of the linger).
    /// An [`oag_weapons::projectile::leach_beam::Kind::Unlocked`] beam draws
    /// nothing.
    ///
    /// Advances [`RaceView::leach_beam_ribbon`] (see [`oag_fx::beam`]) and
    /// keeps [`Trigger::LeachbeamEnergy`] on it: **re-spawned every time the
    /// ribbon's cursor wraps** - the pulse block that also plays
    /// `LEACHENERGY` - one segment short of the target, then walked back
    /// toward the shooter one chain point a tick, which is where
    /// `LeachBeam_Advance` writes the effect's own matrix. The previous
    /// instance is **killed** at each re-spawn and when the beam retires:
    /// the original hands its handle to `Psys_ReleaseHandle` (`0x088f3298`)
    /// with `now == 0` at both (`LeachBeam_Advance`, and `FUN_08872e64`, the
    /// teardown `LeachBeam_UpdatePool` calls on retire), which sets the dead
    /// bit, and the manager frees the instance and its live particles on its
    /// next tick (read 2026-09-30, see `particle-system.md`).
    pub(crate) fn advance_leach_beam_ribbon(&mut self) {
        let locked = self
            .sim
            .world
            .leach_beam
            .filter(|beam| beam.kind == oag_weapons::projectile::leach_beam::Kind::Locked);
        let Some(beam) = locked else {
            self.view.leach_beam_ribbon = None;
            self.view.leach_ball_elapsed = 0.0;
            if let Some(playing) = self.view.leach_beam_effect.take() {
                self.view.stage.kill(playing);
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
        let ribbon = ribbon.get_or_insert_with(|| oag_fx::beam::Ribbon::new(rng));
        let pulsed = ribbon.advance(dt, (target - owner).length(), beam.range, rng);
        let spline = &self.sim.spline;
        let at = ribbon.energy_point(owner, target, beam.range, &|p| spline.tube_frame(p));

        // `LEACHENERGY`: the same pulse block that re-spawns
        // `WO_LEACHBEAM_ENERGY` below, per `LeachBeam_Advance`'s own reading
        // (`docs/ghidra/functions/psp-pulse-usa/cannon-quake-leachbeam.md`).
        // Gated on `pulsed` alone, not the effect match arm's `playing.is_none()`
        // fallback below - that fallback is a redraw safety net, not a second
        // firing of the original's pulse block, and `Ribbon::new`'s `cursor: 0`
        // already makes the very first `advance` call pulse, so the two agree
        // on tick one anyway.
        if pulsed && let Some(at) = at {
            self.sim.cues.push(oag_sound::sfx::CueEvent::at_point(
                oag_sound::sfx::Cue::LeachEnergy,
                at,
            ));
        }

        let effect = self.view.handles.get(Trigger::LeachbeamEnergy).cloned();
        match (effect, self.view.leach_beam_effect, at) {
            (Some(effect), playing, Some(at)) if pulsed || playing.is_none() => {
                if let Some(playing) = playing {
                    self.view.stage.kill(playing);
                }
                self.view.leach_beam_effect = self.view.stage.attach(&effect, at, 1.0);
            }
            (_, Some(playing), Some(at)) => self.view.stage.follow(playing, at),
            _ => {}
        }

        self.advance_leach_ball_hd(owner, target, dt);
    }

    /// Wipeout HD's own LeachBall drain trip - `LeachBall_Advance`'s
    /// recovered law ([`oag_fx::beam::hd_ball`]), run alongside Pulse's
    /// ribbon above rather than instead of it: both are driven from the same
    /// `beam`, and a title with no `WO_LEACHBEAM_ABSORB` asset mounted simply
    /// never resolves the effect below, the same "an absent name costs one
    /// report line" rule [`RACE_EFFECTS`]'s own doc comment states.
    ///
    /// **Fires the burst.** The model's own per-frame placement is a
    /// separate read of the same accumulator - see
    /// [`Self::leach_ball_model_matrix`] - since a draw call needs this
    /// frame's position whether or not this tick happened to wrap.
    fn advance_leach_ball_hd(&mut self, owner: Vec3, target: Vec3, dt: f32) {
        let length = (target - owner).length();
        let wrapped = oag_fx::beam::hd_ball::advance(&mut self.view.leach_ball_elapsed, dt, length);
        if !wrapped {
            return;
        }
        let at =
            oag_fx::beam::hd_ball::position(self.view.leach_ball_elapsed, length, owner, target);
        if let Some(effect) = self.view.handles.get(Trigger::LeachbeamAbsorb) {
            self.view.stage.play(effect, at, 1.0);
        }
    }

    /// Where the LeachBeam ball model sits this frame, or `None` when
    /// nothing is drawn - no [`oag_weapons::projectile::leach_beam::Kind::Locked`]
    /// beam this tick, the same gate [`Self::advance_leach_beam_ribbon`]
    /// takes for the ribbon.
    ///
    /// Reads the owner's and target's positions fresh rather than any this
    /// tick's [`Self::advance_leach_ball_hd`] captured, so a caller at draw
    /// time (after the tick that moved them) still places the ball against
    /// where the beam's two ends are now, not where they were a tick ago.
    ///
    /// **Translation only - identity rotation and scale, both chosen, not
    /// measured.** `LeachBall_Advance`'s own placement call
    /// (`_opd_FUN_001141d8`) reads as "orthonormalise/place a matrix onto the
    /// node" at confidence 70 (`docs/ghidra/functions/ps3-hdfury-eu/weapons.md`,
    /// "2026-09-25" section) but was not resolved past that, so nothing pins
    /// an orientation or a scale for this build to reproduce. An identity
    /// rotation is a determinant-`+1` matrix like every other HD weapon body's
    /// placement since 2026-09-24, so `load::weapon_models::cull_as_authored`
    /// still applies correctly if the model's own materials ask for it.
    #[must_use]
    pub fn leach_ball_model_matrix(&self) -> Option<Mat4> {
        let beam = self
            .sim
            .world
            .leach_beam
            .filter(|beam| beam.kind == oag_weapons::projectile::leach_beam::Kind::Locked)?;
        let owner = self.sim.world.ships[beam.owner as usize]
            .physics
            .body
            .position;
        let target = self.sim.world.ships[beam.target as usize]
            .physics
            .body
            .position;
        let length = (target - owner).length();
        let at =
            oag_fx::beam::hd_ball::position(self.view.leach_ball_elapsed, length, owner, target);
        Some(Mat4::from_translation(at))
    }

    /// The firing craft's LeachBeam hull-overlay pulse on `slot` this tick, or
    /// `None` when it draws nothing: `Mesh_DrawBatchSet` gates
    /// `HullOverlay_SubmitLeachBeamBatched` on a live beam
    /// (`DAT_08b317ac == 2`) and on the craft's weapon record `+0` being
    /// positive, and `LeachBeam_UpdatePool` writes that `+0` with
    /// `LeachBeam_PulseStrength` every tick - measured in play, linger
    /// included. See [`oag_fx::beam::Ribbon::pulse_strength`].
    #[must_use]
    pub fn leach_overlay_pulse(&self, slot: usize) -> Option<f32> {
        let beam = self.sim.world.leach_beam?;
        if usize::from(beam.owner) != slot {
            return None;
        }
        self.view.leach_beam_ribbon.as_ref()?.pulse_strength()
    }

    /// The ribbon's geometry for this frame, or empty when there is no
    /// locked beam to draw - see [`oag_fx::beam::build`].
    ///
    /// `camera_right`/`camera_up` are the view's own world-space axes, which
    /// the two strips are widened along. `alpha` is the link's own coverage:
    /// `1.0` while connected, and while disconnected a linear fade to `0.0`
    /// over [`oag_weapons::projectile::leach_beam::DISCONNECT_LINGER_SECONDS`] -
    /// `LeachBeam_BuildStrip`'s own recovered alpha write.
    #[must_use]
    pub(crate) fn leach_beam_ribbon_vertices(
        &self,
        camera_right: oag_core::math::Vec3,
        camera_up: oag_core::math::Vec3,
    ) -> Vec<oag_mesh::mesh::GpuVertex> {
        use oag_weapons::projectile::leach_beam::{DISCONNECT_LINGER_SECONDS, Kind};

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
        let frame = oag_fx::beam::Frame {
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
        let spline = &self.sim.spline;
        oag_fx::beam::build(ribbon, &frame, &|p| spline.tube_frame(p))
    }

    /// Plays the explosion a weapon that just went off authored - its own,
    /// not another weapon's.
    ///
    /// **Recovered for the Rocket, the Missile, the Mine and the Bomb.** See
    /// [`Race::blast_for`] for the map and the reading behind each arm.
    /// `orientation` is the frozen pose a laid charge detonated with
    /// ([`oag_weapons::projectile::mine::frozen_pose`]) - unused by every
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
    /// it drew the *Rocket's* `Trigger::TrackBlast`/`Trigger::CraftBlast` for
    /// every kind of impact, mine and missile included, because [`blast_for`]
    /// took no `kind` at all; until 2026-09-23 a Bomb drew nothing at all.
    ///
    /// [`RaceView::sparks`] is deliberately not reused for it: that system
    /// re-anchors to the hull every tick, so a burst ignited at an impact
    /// would emit from the craft instead. It is one hull-mounted emitter, and
    /// this is why the stage exists alongside it.
    ///
    /// [`blast_for`]: Race::blast_for
    pub(crate) fn ignite_blast(
        &mut self,
        kind: oag_tables::weapons::Weapon,
        point: Vec3,
        struck: Option<usize>,
        orientation: Quat,
    ) {
        let Some((trigger, at)) = self.blast_for(kind, point, struck) else {
            return;
        };
        // The Plasma's own three-model detonation - `Trigger::PlasmaBlast`
        // below is `WO_PLASMA_FLASH`, a separate `.pob` particle system; the
        // halo and two hemispheres are their own `.vex` models with their
        // own render-side pool, on the same terms as the Rocket's, the
        // Mine's and the Bomb's own bodies. See `blast_models` module doc
        // comment for why that pool lives on `self.view` and is advanced
        // independently of this effect stage.
        if kind == oag_tables::weapons::Weapon::Plasma {
            self.spawn_plasma_blast_model(at);
        }
        // The Bomb's own two-model detonation - `Trigger::BombSmokering`
        // below is one third of it, the psys smoke ring; the hemisphere and
        // the shockwave are their own `.vex` models, on `bomb_blast`'s own
        // render-side pool, the same shape as the Plasma's own three above.
        if kind == oag_tables::weapons::Weapon::Bomb {
            self.spawn_bomb_blast_model(at, orientation);
        }
        // HD's Missile explosion model: entered only when the missile reaches
        // a craft, at that craft - `missile_blast`.
        if let (oag_tables::weapons::Weapon::Missile, Some(slot)) = (kind, struck) {
            let ship = &self.sim.world.ships[slot];
            self.spawn_hd_missile_blast(ship.physics.body.position, ship.physics.body.orientation);
        }
        // Each detonation's own `ScreenFlash_Start`, whether or not its
        // effect loaded - see [`flash_for`] and `oag_fx::flash`.
        if let (Some(kind), Some(flash)) = (
            flash_for(kind, struck.is_some()),
            &mut self.view.screen_flash,
        ) {
            flash.start(kind, at);
        }
        let Some(effect) = self.view.handles.get(trigger).cloned() else {
            return;
        };
        // Neutral severity: the field the collision sparks derive from an
        // impulse is the *hull's*, and nothing on the rocket path has been
        // read as feeding it. See `oag_fx::sparks::severity`.
        self.view.stage.play(&effect, at, 1.0);
    }

    /// Which explosion a hit plays and where, or `None` for a weapon with no
    /// recovered detonation effect - split out from [`Race::ignite_blast`] so
    /// the recovered part can be asserted without a disc to load the effect
    /// from.
    ///
    /// - **`Rocket`**: [`Trigger::TrackBlast`] from `Rocket_Update`'s
    ///   (`0x0885d2a8`) two collision branches, [`Trigger::CraftBlast`] from
    ///   `Rocket_SpawnCraftExplosion_q` (`0x0886ed34`) on the craft-hit path.
    ///   A craft hit is drawn at the *struck craft's* own position dropped by
    ///   [`oag_title::engine_effects::CRAFT_BLAST_DROP`], not at the rocket's impact point; a craft that
    ///   has since gone inactive falls back to the impact point rather than
    ///   reading a stale pose.
    /// - **`Missile`**: [`Trigger::MissileExplo`] always, whatever it struck -
    ///   see that constant's doc comment for why there is one file and no
    ///   drop offset, unlike the Rocket's.
    /// - **`Mine`**: [`Trigger::MineExplo`] always, whatever it struck, the
    ///   same shape as the Missile's - see that constant's doc comment for
    ///   `Mine_SpawnExplosion`.
    /// - **`Cannon`**: [`Trigger::CannonSparks`] when `struck` is `None` - a
    ///   wall or track hit - and nothing when it is `Some`. `Cannon_UpdateRound`
    ///   (`0x0886593c`) only calls `Psys_Spawn_q` off its own world-collision
    ///   raycast; the separate craft-proximity test that produces a `Some`
    ///   `struck` here (`FUN_088579a8`/`FUN_08857f2c`) applies damage and a
    ///   sound cue but never spawns a particle effect. See
    ///   `oag_weapons::projectile::cannon`'s module doc for the full read.
    /// - **`Plasma`**: [`Trigger::PlasmaBlast`] always, whatever it struck, the
    ///   same shape as the Missile's and the Mine's. `Plasmas_Update`
    ///   (`0x0886b490`) runs one teardown pass over every bolt carrying the
    ///   destroy bit and calls `Plasma_SpawnDetonation` (`0x0886ac88`) with the
    ///   bolt's own position for each - a wall hit and a timed-out bolt reach
    ///   it identically, so there is no split to mirror. Wired 2026-09-09; it
    ///   returned `None` before, when the teardown was unread. **On HD,
    ///   [`Trigger::PlasmaLightningExpand`] instead** - a different
    ///   executable's own file, read 2026-09-17; see that constant's doc
    ///   comment.
    /// - **`Bomb`**: [`Trigger::BombSmokering`] always, whatever it struck -
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
    pub(crate) fn blast_for(
        &self,
        kind: oag_tables::weapons::Weapon,
        point: Vec3,
        struck: Option<usize>,
    ) -> Option<(Trigger, Vec3)> {
        match kind {
            oag_tables::weapons::Weapon::Rocket => Some(match struck {
                Some(slot) if self.sim.world.ships[slot].active => (
                    Trigger::CraftBlast,
                    self.sim.world.ships[slot].physics.body.position
                        - Vec3::Y * oag_title::engine_effects::CRAFT_BLAST_DROP,
                ),
                // A craft that has gone inactive since the hit falls back to
                // the impact point rather than reading a stale pose - still
                // its own effect, because what was struck is what chose the
                // file.
                Some(_) => (Trigger::CraftBlast, point),
                None => (Trigger::TrackBlast, point),
            }),
            oag_tables::weapons::Weapon::Missile => Some((Trigger::MissileExplo, point)),
            oag_tables::weapons::Weapon::Mine => Some((Trigger::MineExplo, point)),
            oag_tables::weapons::Weapon::Bomb => Some((Trigger::BombSmokering, point)),
            // `FUN_08870c78`, the blade's teardown: a fuse running out and a
            // craft hit both reach it.
            oag_tables::weapons::Weapon::Shuriken => Some((Trigger::ShurikenExpire, point)),
            oag_tables::weapons::Weapon::Cannon if struck.is_none() => {
                Some((Trigger::CannonSparks, point))
            }
            // HD plays its own file, `WO_PLASMA_LIGHTNING_EXPAND` -
            // `WeaponExplosions_Start`, not `Plasma_SpawnDetonation`'s
            // `WO_PLASMA_FLASH`. `WO_PLASMA_LIGHTNING_COLLAPSE`, the other
            // half of HD's pair, fires later and is not from here - see
            // `Race::advance_plasma_blast_models`.
            oag_tables::weapons::Weapon::Plasma => Some((
                if self.view.hd_plasma_blast {
                    Trigger::PlasmaLightningExpand
                } else {
                    Trigger::PlasmaBlast
                },
                point,
            )),
            _ => None,
        }
    }

    /// Plays [`Trigger::MissileBounce`] at every wall a missile glanced off
    /// this tick.
    ///
    /// **Not reached from `Impact`.** A bounce is not a detonation -
    /// `oag_weapons::projectile::step` reports only what *stopped* a
    /// projectile, and a bouncing missile does not stop. So this reads
    /// `before`, a snapshot of every slot's [`oag_weapons::projectile::Projectile::bounces`]
    /// taken right before `step`, and fires wherever a live Missile's own
    /// counter went up by exactly one this tick - the only way it moves, per
    /// [`oag_weapons::projectile::missile::MAX_BOUNCES`]. `Projectile::bounces`
    /// is public simulation state read here rather than threaded through a
    /// new out-parameter the way [`Race::tick`]'s `absorbed` is, because nothing
    /// about the gate is deterministic-critical or hidden - it is the same
    /// counter [`crate::hash`] already hashes.
    ///
    /// A slot whose kind changed (a detonation freed it, a new weapon took
    /// it) cannot show a false bounce: [`oag_weapons::projectile::Projectile::default`]
    /// resets `bounces` to zero, so the count only ever goes *down* across
    /// such a transition, never up, and this only looks for an increase.
    ///
    /// Called after `projectile::step`, so a bounce plays where the missile
    /// actually is this tick - a few hundredths of a unit past the wall it
    /// struck, at [`oag_weapons::projectile::missile::BOUNCE_PUSH_OFF`], rather
    /// than exactly on it.
    pub(crate) fn ignite_missile_bounces(
        &mut self,
        before: &[u8; oag_weapons::projectile::MAX_PROJECTILES],
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
                .and_then(|trigger| self.view.handles.get(trigger))
                .cloned()
            else {
                continue;
            };
            self.view.stage.play(&effect, projectile.position, 1.0);
        }
    }

    /// Keeps an [`Trigger::EngineFlare`] instance on every active craft's
    /// nozzle, where the source authors one.
    ///
    /// **The PS2 port authors an engine flare as a particle effect and the
    /// PSP does not.** On a PSP-sourced race the effect is absent from the
    /// library, nothing attaches, and [`oag_fx::exhaust`]'s procedural
    /// flare draws as it always has. On a PS2-sourced one the asset plays
    /// and the procedural quad steps aside - see
    /// [`Race::engine_flare_effect`], which is what the renderer asks.
    ///
    /// The trigger needs no reverse-engineering: an engine flare is on for
    /// as long as the craft is, which is why this one is wired where the
    /// other PS2-only effects are not. Both its emitters are
    /// [`oag_pob::flags::LOOPING`], so it runs until detached.
    ///
    /// Anchored at the `Engine Flare` locator under the craft's *current*
    /// transform, the same point the procedural flare uses, so the two are
    /// interchangeable rather than merely similar.
    pub(crate) fn advance_engine_flares(&mut self) {
        let Some(effect) = self.view.handles.get(Trigger::EngineFlare).cloned() else {
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
    /// Rocket's [`Trigger::RocketFlare`], the Missile's [`Trigger::MissileFlare`]
    /// - and takes it off the ones that are gone.
    ///
    /// **Recovered, one weapon at a time; see [`flare_effect_for`] for the
    /// map.** `Rocket_Init` (`0x0885cdb8`) and `Missile_Init` (`0x0885a160`)
    /// each attach their own file at launch and ride it for the whole
    /// flight; every emitter involved is [`oag_pob::flags::LOOPING`],
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
    /// A projectile that stops riding a flare **releases** it, not kills it
    /// (`Psys_ReleaseHandle` with `now != 0`, which `RocketPool_Update`,
    /// `MissilePool_Update` and `Plasmas_Update` all call, read 2026-09-30):
    /// emission stops and the template particles go at once - the missile's
    /// `glow` lives 3600 ticks and the plasma head's 65535 - while the
    /// particles the emitters already put out finish their own lives, so the
    /// smoke outlives the weapon the way it does in the original. That is also the bug the invented rocket trail could not
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
    pub(crate) fn advance_projectile_flares(&mut self) {
        for (slot, projectile) in self.sim.world.projectiles.slots.iter().enumerate() {
            let name = flare_effect_for(projectile.kind);
            let (primary, orbiting) = match projectile.kind {
                Some(oag_tables::weapons::Weapon::Missile) => {
                    let age = oag_weapons::projectile::MAX_FLIGHT_SECONDS - projectile.lifetime;
                    let (a, b) =
                        missile_flare_anchors(projectile.position, projectile.velocity, age);
                    (a, Some(b))
                }
                // The blade's trail rides the same point as its head, on its
                // own quarter-turned frame - see `Trigger::ShurikenTrail`.
                Some(oag_tables::weapons::Weapon::Shuriken) => {
                    (projectile.position, Some(projectile.position))
                }
                _ => (projectile.position, None),
            };
            let second = if projectile.kind == Some(oag_tables::weapons::Weapon::Shuriken) {
                Some(Trigger::ShurikenTrail)
            } else {
                name
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
                &self.view.handles,
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
            // The blade's head frame is its basis unrotated, so the emitter's
            // `+Y` is the surface normal it rides; its trail frame is turned
            // `-pi/2` about row 0, which puts `+Y` along the velocity, the
            // Rocket's measured case. `Shuriken_Update` `0x08877bdc`.
            if let (Some(oag_tables::weapons::Weapon::Shuriken), Some(instance)) =
                (projectile.kind, self.view.projectile_flare[slot])
            {
                self.view.stage.orient(
                    instance,
                    projectile.surface.try_normalize().unwrap_or(Vec3::Y),
                );
            }
            // The second anchor: the Missile's orbiting one (see
            // `missile_flare_anchors`) or the Shuriken's trail. `orbiting` is
            // `None` for every other kind, so this rides nothing and only ever
            // tears down a leftover instance from a slot that held one.
            advance_one_flare(
                &mut self.view.stage,
                &self.view.handles,
                orbiting.and(second),
                orbiting.unwrap_or(primary),
                1.0,
                &mut self.view.projectile_flare_orbit[slot],
            );
            if let (Some(oag_tables::weapons::Weapon::Shuriken), Some(instance)) =
                (projectile.kind, self.view.projectile_flare_orbit[slot])
            {
                let up = projectile.velocity.try_normalize().unwrap_or(Vec3::Y);
                self.view.stage.orient(instance, up);
            }
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
    /// [`Trigger::RocketFlare`], played off the disc through [`RaceView::stage`],
    /// and a billboard on top of it would be a second invented one. What is
    /// left here is the case where a kind's own model did not load, or a kind
    /// (the Missile, the Plasma) has no model at all, and a
    /// projectile would otherwise be invisible - see
    /// [`PROJECTILE_SPRITE_HALF_SIZE`].
    ///
    /// **Was one flag for every kind at once, fixed 2026-09-05 alongside
    /// [`Self::projectile_model_matrices`]'s own kind filter.** A single
    /// `bool` gated *all* projectiles on whether the Rocket's model happened
    /// to load, so on a real disc (where it does) a live Missile, Plasma or
    /// Shuriken (before its own model was drawn) drew nothing at all: no mesh, and no
    /// billboard, because the flag said "modelled" for a kind it was never
    /// about. `modelled` is now asked once per live projectile's own kind.
    #[must_use]
    pub fn projectile_sprites(
        &self,
        right: Vec3,
        up: Vec3,
        modelled: impl Fn(oag_tables::weapons::Weapon) -> bool,
    ) -> Vec<oag_mesh::mesh::GpuVertex> {
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
    /// # Checking a frame with a rocket in it
    ///
    /// `--race` gives a craft that holds the throttle and never reaches a
    /// pad, so a rocket needs `--give rocket` (`--mode eliminator` for a
    /// Shuriken, with the other grid slots off - a grid-mate takes the blade
    /// on its first tick) or a replayed input script (`--input-script`). Track scenery can read as a volley: compare
    /// against `--mode time_trial`, where no rocket can exist.
    #[must_use]
    pub fn rocket_model_matrices(&self) -> Vec<Mat4> {
        self.projectile_model_matrices(oag_tables::weapons::Weapon::Rocket)
    }

    /// Where each live mine is, and the pose it landed in, for the model draw.
    ///
    /// **No longer translation only.** `oag_weapons::projectile::mine::at_rest`
    /// is the recovered velocity a laid mine carries - zero, so
    /// [`Self::projectile_model_matrices`]'s `forward == Vec3::ZERO` branch is
    /// exactly what fires here - but that branch now draws
    /// `Projectile::orientation` rather than a bare translation. See
    /// [`oag_weapons::projectile::mine::frozen_pose`] for what `Mine_Init`
    /// copies at drop, at what confidence, and which half of this reading is
    /// chosen rather than measured.
    #[must_use]
    pub fn mine_model_matrices(&self) -> Vec<Mat4> {
        self.projectile_model_matrices(oag_tables::weapons::Weapon::Mine)
    }

    /// Where each live bomb is, for the model draw - the Mine's, one size up.
    ///
    /// Same reading as [`Self::mine_model_matrices`], carried over on the same
    /// terms [`oag_weapons::projectile::mine::frozen_pose`] already applies
    /// [`oag_weapons::projectile::mine::at_rest`] to both weapons: the Bomb's
    /// own spawn helper is unread, so nothing confirms the same matrix-copy
    /// happens there, and nothing rules it out either.
    #[must_use]
    pub fn bomb_model_matrices(&self) -> Vec<Mat4> {
        self.projectile_model_matrices(oag_tables::weapons::Weapon::Bomb)
    }

    /// Where each live Shuriken blade is: `Shuriken_Update`'s (`0x08877bdc`)
    /// basis - `n x f`, `n`, `f`, position - handed to the model node
    /// unrotated, with `n` the exact surface normal and `f` the velocity
    /// flattened against it (the Rocket's does the reverse). The update
    /// writes no spin of its own.
    #[must_use]
    pub fn shuriken_model_matrices(&self) -> Vec<Mat4> {
        self.projectile_model_matrices(oag_tables::weapons::Weapon::Shuriken)
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
    /// [`Trigger::RocketFlare`] from the moment it landed - see
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
                    // flies (`oag_weapons::projectile::mine::at_rest`). On
                    // Pulse the pose is the executable's own, measured - see
                    // `laid`; elsewhere `orientation` is the frozen pose it
                    // landed with, chosen, not measured.
                    return laid::matrix(
                        slot,
                        projectile,
                        self.view.pulse_laid_pose,
                        self.view.laid_pose_scaled,
                    );
                }
                // `surface` is the normal the projectile rides - the
                // original's own `n` (`rocket+0x100`), seeded to world up at
                // spawn. It, then world up, then any perpendicular, so a
                // projectile flying along one of them still gets a basis.
                if kind == oag_tables::weapons::Weapon::Shuriken {
                    // `Shuriken_Update` (`0x08877bdc`) keeps the carried
                    // normal exact as row 1 and flattens the velocity
                    // against it (`vdot`/`vscl`/`vsub`, `0x08877f6c`), so a
                    // blade lies in its surface even when it flies off it.
                    let up = projectile.surface.try_normalize().unwrap_or(Vec3::Y);
                    let flat = forward - up * forward.dot(up);
                    if let Some(front) = flat.try_normalize() {
                        let side = up.cross(front);
                        return Mat4::from_cols(
                            side.extend(0.0),
                            up.extend(0.0),
                            front.extend(0.0),
                            projectile.position.extend(1.0),
                        );
                    }
                }
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
