//! The tick's view statements that stood inline in `Race::tick_body` and the simulation's
//! step functions, as named [`RaceView`] methods called where the statements stood. Each reads
//! the simulation it needs from its arguments and writes only the view.

use oag_core::math::{Quat, Vec3};
use oag_gameplay::MAX_SHIPS;
use oag_gameplay::cue::CueEvent;
use oag_render::camera::chase::Chase;
use oag_weapons::projectile::MAX_PROJECTILES;

use crate::player_step::PlayerStep;
use crate::{
    BOOST_FOV_CLOSE_RATE, BOOST_FOV_OPEN_RATE, FlapPhase, GRID_FLAP_LEVEL, RaceView, SimRace,
};
use oag_physics::Evaluated;

/// What the drawn bounce and blast read off the projectile pool before
/// `oag_weapons::projectile::step` runs ([`RaceView::before_projectile_step`]).
pub(crate) struct BeforeProjectileStep {
    /// Each slot's bounce count, for `ignite_missile_bounces` and the two bounce cues: a bounce
    /// never stops a projectile, so it never reaches `impacts` and the only way to see one is to
    /// compare this counter before and after.
    pub(crate) bounces: [u8; MAX_PROJECTILES],
    /// Each slot's orientation, for `ignite_blast`'s Bomb arm: a detonated slot resets to
    /// `Projectile::default()` inside `step` (`Quat::IDENTITY`), so the frozen pose a laid Bomb
    /// detonated with has to be read before that happens. `impacts` is slot-indexed the same
    /// way. Read for every slot rather than gated on `kind == Bomb`, since nothing here is on
    /// the determinism-hashed path.
    pub(crate) orientations: [Quat; MAX_PROJECTILES],
}

impl RaceView {
    /// The camera shakes' and the now-playing card's tick, before anything this tick can arm a
    /// shake (`Race::tick_body` says why first).
    pub(crate) fn advance_shakes(&mut self, core: &SimRace) {
        self.advance_now_playing();
        self.contact.shake.advance(core.sim.dt);
        self.advance_seat_shakes(core.sim.dt);
        if !self.camera_shake_on {
            self.contact.shake.cancel();
        }
    }

    /// Each flown slot's HUD message board, one tick on, and the decision the boards make for the
    /// simulation: a message that has just come up and is heard on this machine raises
    /// `Cue::Message` for the seat. The cue is returned for the tick to queue at this point, so the
    /// queue's order is the one it always had (ADR-0018: a cue is an output of the tick).
    pub(crate) fn message_cue(&mut self, core: &SimRace) -> Option<CueEvent> {
        let mut shown = false;
        for (slot, hud) in self.huds.iter_mut().enumerate() {
            if core.sim.world.person_flown(slot) {
                let board = &mut hud.messages;
                board.advance();
                let showing = board.just_shown();
                shown |= showing && core.heard_here(slot);
            }
        }
        shown.then(|| CueEvent::new(oag_gameplay::cue::Cue::Message, core.player_slot()))
    }

    /// The projectile pool as the tick found it, read just before the projectiles step: the
    /// view's half of the before/after edges the step's bounces and detonations are seen by.
    pub(crate) fn before_projectile_step(
        &self,
        world: &oag_gameplay::World,
    ) -> BeforeProjectileStep {
        BeforeProjectileStep {
            bounces: std::array::from_fn(|slot| world.projectiles.slots[slot].bounces),
            orientations: std::array::from_fn(|slot| world.projectiles.slots[slot].orientation),
        }
    }

    /// The particle pool's tick, after every effect this tick started is in it.
    pub(crate) fn advance_particles(&mut self, core: &SimRace) {
        self.stage.advance(core.sim.dt, &mut self.stage_rng);
    }

    /// The chase camera's tick and what is measured from it: the transit seats, the jump cut, the
    /// split-screen views, the finish director, the screen flash and the cockpit hit.
    pub(crate) fn advance_chase_camera(&mut self, core: &SimRace) {
        self.advance_transit_seats(core);
        let target = self.camera_target(core);
        // A craft put further round (`Race::move_to`) takes its camera with it, settled:
        // the spring would sweep the eye across the gap.
        let jumps = core.sim.jumps[core.player_slot()];
        if std::mem::replace(&mut self.camera_jumps, jumps) == jumps {
            let step = self.camera_step(core, core.player_slot());
            self.camera.advance(target, &self.chase_params, step);
        } else {
            self.camera = Chase::snapped(target, &self.chase_params);
        }
        // The split-screen views' own cameras, view-side: a no-op with one view.
        self.advance_viewpoints(core);
        // After the chase camera and before the eye is read: behind the end-race panels the
        // spectator director's pose replaces it. A no-op until the player has finished.
        self.advance_finish_camera(core);
        // After the camera, so the flash's falloff measures from this tick's eye.
        let eye = self.camera_position(core);
        if let Some(flash) = &mut self.screen_flash {
            flash.advance(core.sim.dt, eye);
        }
        self.advance_cockpit_hit(core);
    }

    /// Each craft's drawn state after this tick's camera: the hull shake, the exhaust, the trail
    /// hits, the shield shell, the boost lens kick and the airbrake flaps.
    pub(crate) fn advance_craft_looks(&mut self, core: &SimRace) {
        self.advance_hull_shakes(core);
        self.advance_exhausts(core);
        // After the trails have taken this tick's sample, so a craft is tested
        // against the ribbon as it stands now rather than one tick stale.
        self.advance_trail_hits(core);
        // Same argument, and after `step`/`step_opponents` so a shell that lost
        // its last tick of protection this frame starts fading this frame.
        self.advance_shields(core);

        // Open while the boost's own timer runs, close once it has expired, both
        // as an exponential approach so neither edge is a step. Driven from
        // `pad_timer` rather than from the exhaust, because the exhaust's boost
        // is a `max` that other things may one day also arm and this must follow
        // the pad specifically.
        if self.boost_fov_kick != oag_display::display::BoostFovKick::OFF {
            let boosting = core.sim.world.ships[core.player_slot()].physics.pad_timer > 0.0;
            self.boost_kick = step_boost_kick(self.boost_kick, boosting, core.sim.dt);
        }

        // The flaps, on the fixed tick beside the camera and the exhaust, and
        // for the same reason: a headless capture calls only `tick`, so an
        // animation advanced in the frame loop would be at a different angle in
        // a screenshot than in a window at the same tick count.
        //
        // Two rates, not one. `up_speed` and `down_speed` are authored beside
        // each other in `<AirbrakeGraphics>` and are *not* the force law's
        // `gain`/`falloff`, so a flap deploys and returns at its own pace while
        // the airbrake it depicts ramps at another. The state it chases is the
        // ramped `airbrake_left`/`airbrake_right` on `0..=100` rather than the
        // raw input, so a flap follows the airbrake the ship actually has.
        //
        // **The rates are on the airbrake's own `0..=100` scale**, per second,
        // which is why `self.flaps` holds a level rather than an angle. Not
        // read out of the binary - no consumer of `<AirbrakeGraphics>` was
        // located - but not a coin flip either: every team authors
        // `up_speed = 500` against a `down_speed` of 80 to 120, and on this
        // reading that is a flap that snaps out in 0.2 s and folds back over
        // about a second, which is what an airbrake does. Read as *fractions*
        // of full deflection per second instead, 500 would be full travel in
        // two milliseconds and the parameter would not be worth authoring.
        // It is also the convention `<Airbrake gain/falloff>` already uses on
        // the same scale, one element away.
        let phase = self.flap_phase_at(core, core.sim.world.tick);
        self.flap_phase = phase;
        for slot in 0..MAX_SHIPS {
            // The grid state forces both flaps fully out, every craft, whoever flies it
            // (`docs/physics/grid-state.md`, "The flaps on the grid"). Written, not chased, so
            // the first tick already draws them full; once the state ends the chase below
            // folds them back at `down_speed`. The state is the countdown clock's, as
            // `ShipState::on_grid` is (`Race::tick` writes it from `tick + 1` before the tick
            // that has since advanced `world.tick`), so a craft rebuilt mid-countdown by a
            // rescue keeps them out.
            if phase == FlapPhase::Grid {
                self.flaps[slot] = [GRID_FLAP_LEVEL; 2];
                continue;
            }
            let targets = Self::flap_targets(core, slot);
            for (flap, target) in self.flaps[slot].iter_mut().zip(targets) {
                let (rate, rising) = if target > *flap {
                    (self.flap_graphics.up_speed, true)
                } else {
                    (self.flap_graphics.down_speed, false)
                };
                let step = rate * core.sim.dt;
                *flap = if rising {
                    (*flap + step).min(target)
                } else {
                    (*flap - step).max(target)
                };
            }
        }
    }

    /// The levels `slot`'s flaps chase, `0..=100`: its airbrakes for a craft a person flies, zero
    /// for every other, so a seat handed to the AI does not keep its last deflection.
    fn flap_targets(core: &SimRace, slot: usize) -> [f32; 2] {
        let ship = &core.sim.world.ships[slot].physics;
        let flown = core.sim.world.person_flown(slot);
        [ship.airbrake_left, ship.airbrake_right]
            .map(|level| if flown { level } else { 0.0 }.clamp(0.0, 100.0))
    }

    /// What the grid's flaps are doing at race tick `tick` (a `World::tick`): held out, folding
    /// from GO at the hull's `down_speed`, or done (always, with the grid's force pinned off).
    fn flap_phase_at(&self, core: &SimRace, tick: u64) -> FlapPhase {
        if !self.grid_flaps {
            return FlapPhase::Race;
        }
        if oag_race::RaceState::thrust_gated(tick) {
            return FlapPhase::Grid;
        }
        let step = self.flap_graphics.down_speed * core.sim.dt;
        let since_go = tick - (oag_race::COUNTDOWN_TICKS - 1);
        if step > 0.0 && (since_go as f32) * step < GRID_FLAP_LEVEL {
            FlapPhase::Fold
        } else {
            FlapPhase::Race
        }
    }

    /// Puts every craft's drawn flaps where race tick `tick` has them, for a restored or posed
    /// state that no tick of this view led to: full on the grid, partway down the fold in the
    /// ticks after GO, and, from the first tick past the fold, the level the craft's airbrakes
    /// give. Past the fold from a view that was already past it, nothing moves: the flaps follow
    /// the airbrakes there, and a correction that rewinds a few ticks must not snap them.
    pub(crate) fn settle_flaps(&mut self, core: &SimRace, tick: u64) {
        let phase = self.flap_phase_at(core, tick);
        let was = std::mem::replace(&mut self.flap_phase, phase);
        if phase == FlapPhase::Race && was == FlapPhase::Race {
            return;
        }
        let step = self.flap_graphics.down_speed * core.sim.dt;
        let since_go = tick.saturating_sub(oag_race::COUNTDOWN_TICKS - 1) as f32;
        for slot in 0..MAX_SHIPS {
            let targets = Self::flap_targets(core, slot);
            let levels = match phase {
                FlapPhase::Grid => [GRID_FLAP_LEVEL; 2],
                FlapPhase::Fold => {
                    let folding = (GRID_FLAP_LEVEL - step * since_go).max(0.0);
                    targets.map(|target| target.max(folding))
                }
                FlapPhase::Race => targets,
            };
            self.put_flaps(slot, levels);
        }
    }

    /// Forgets last tick's opponent wall contacts, before the field steps.
    pub(crate) fn clear_field_walls(&mut self) {
        self.field_sparks.walls = [None; MAX_SHIPS];
    }

    /// Opponent `slot`'s wall contact this tick, for [`Self::advance_field_sparks`].
    pub(crate) fn note_field_wall(&mut self, slot: usize, wall: oag_physics::WallResponse) {
        self.field_sparks.walls[slot] = Some(wall);
    }

    /// Forgets last tick's craft-to-craft contacts, before the pairs resolve.
    pub(crate) fn clear_pair_contacts(&mut self) {
        self.pair_contacts = [None; MAX_SHIPS];
    }

    /// `slot`'s craft-to-craft contact this tick, for the seat's contact effects.
    pub(crate) fn note_pair_contact(&mut self, slot: usize, point: Vec3, impulse: f32) {
        self.pair_contacts[slot] = Some(crate::contact_fx::PairContact { point, impulse });
    }

    /// Marks `slot`'s lap as spoiled by a craft-to-craft bump when `spoiled`, for its lap
    /// messages (`crate::lap_messages`).
    pub(crate) fn spoil_lap(&mut self, slot: usize, spoiled: bool) {
        self.pair_spoil[slot] |= spoiled;
    }

    /// Drops `slot`'s exhaust ribbons, so a craft put back does not draw one across the gap.
    pub(crate) fn clear_trails(&mut self, slot: usize) {
        self.exhaust[slot].clear_trail();
        self.hd_trail[slot].clear();
    }

    /// Bulges `slot`'s shield shell, with no journal entry: a Quake's absorbed hit.
    pub(crate) fn bulge_shield(&mut self, slot: usize) {
        self.shield[slot].hit();
    }

    /// On a tick that is not drawn, the incoming-fire raises it queued are dropped rather than
    /// shown late.
    pub(crate) fn forget_incoming_raises(&mut self) {
        self.incoming_raises.clear();
    }
}

/// One tick of the boost's field-of-view kick: open while the boost's own timer runs, close once
/// it has expired, both as an exponential approach so neither edge is a step. The player's
/// ([`RaceView::boost_kick`](crate::view::RaceView::boost_kick)) and every other local seat's
/// (`Cam::boost_kick`) are this one rule.
pub(crate) fn step_boost_kick(kick: f32, boosting: bool, dt: f32) -> f32 {
    let (target, rate) = if boosting {
        (1.0, BOOST_FOV_OPEN_RATE)
    } else {
        (0.0, BOOST_FOV_CLOSE_RATE)
    };
    let kick = kick + (target - kick) * rate * dt;
    // Snapped, so a closed kick is *exactly* zero and `projection` takes
    // its bit-identical path rather than an `atan(tan(x))` round trip
    // that never quite settles.
    if !boosting && kick < 1.0e-3 {
        0.0
    } else {
        kick
    }
}

impl RaceView {
    /// The seat's shield coming up this tick, and the decision it makes for the simulation: the
    /// announcer's `Cue::ShieldActive` on the edge, returned for the tick to queue at this point.
    ///
    /// The announcer, on the shield coming up and not on it being up. `Shield_Activate` plays
    /// `"shieldactive"` one line above the `Sound_PlayLooping` that opens `~SHIELD`, so the two
    /// share an instant and differ in shape: a one-shot on this edge, a held voice on the level
    /// the edge starts. The level half is `SimRace::shield_is_up`, read by the audio layer; the
    /// edge is here, with every other cue edge. The edge is the view's (`shield_was_up`), so a
    /// replayed tick neither raises nor re-arms it.
    pub(crate) fn shield_up_cue(&mut self, core: &SimRace, slot: usize) -> Option<CueEvent> {
        let shielded = core.sim.world.ships[slot].physics.shield_pickup_timer > 0.0;
        let cue = (shielded && !self.shield_was_up)
            .then(|| CueEvent::new(oag_gameplay::cue::Cue::ShieldActive, slot));
        self.shield_was_up = shielded;
        cue
    }

    /// The seat's collision sparks and the shake a hit arms, for its wall contact this tick
    /// (`slot`, `evaluated`). The two contact sounds are raised for every flown slot in
    /// [`Race::tick`](crate::Race::tick), the shield edge by [`Self::shield_up_cue`].
    ///
    /// Everything the tick draws on a wall contact, in one place so
    /// [`Race::tick_sim_only`](crate::Race::tick_sim_only) can skip it whole. The one piece of
    /// simulation state beside it is `contact_cue_cooldown`, which the sim-only path advances
    /// itself.
    pub(crate) fn advance_contact_fx(
        &mut self,
        core: &SimRace,
        slot: usize,
        evaluated: &Evaluated,
    ) {
        let model_matrix = self.ship_model_matrix_of(core, slot);
        self.contact.advance(
            &self.handles,
            &self.collision_fx,
            &core.sim.world.ships[slot],
            model_matrix,
            core.sim.dt,
            crate::contact_fx::Touch {
                evaluated,
                pair: self.pair_contacts[slot].or_else(|| {
                    crate::floor_sparks::not_racing_record(
                        &core.sim.world.ships[slot],
                        &evaluated.wall,
                    )
                }),
            },
        );
    }

    /// Every other local person's sparks and shake, from their own craft's wall contact.
    /// A split screen's or a window's extra seat has a [`ContactFx`](crate::contact_fx::ContactFx)
    /// beside its camera; the seat `own` is the one [`Self::advance_contact_fx`] ran.
    pub(crate) fn advance_seat_contact_fx(
        &mut self,
        core: &SimRace,
        steps: &[Option<PlayerStep>],
        own: usize,
    ) {
        for (slot, step) in steps.iter().enumerate() {
            let Some(step) = step else { continue };
            if slot == own || self.viewpoints.contact[slot].is_none() {
                continue;
            }
            let model_matrix = self.ship_model_matrix_of(core, slot);
            if let Some(fx) = self.viewpoints.contact[slot].as_mut() {
                fx.advance(
                    &self.handles,
                    &self.collision_fx,
                    &core.sim.world.ships[slot],
                    model_matrix,
                    core.sim.dt,
                    crate::contact_fx::Touch {
                        evaluated: &step.evaluated,
                        pair: self.pair_contacts[slot].or_else(|| {
                            crate::floor_sparks::not_racing_record(
                                &core.sim.world.ships[slot],
                                &step.evaluated.wall,
                            )
                        }),
                    },
                );
            }
        }
    }
}
