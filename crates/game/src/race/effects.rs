//! Every craft's exhaust, the particle stage the disc's own effects play on,
//! and the sparks a contact throws.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::*;

/// Where a trail-hit burst sits on a struck craft: on its hull, facing the
/// contact.
///
/// The original parents the system to the nearest of ten attachment nodes at
/// `craft + 0x79d0..+0x79f4`, so it rides the ship. This engine has no such
/// nodes, so it puts the burst on the craft's own bounding sphere in the
/// direction of the contact - where the nearest of those nodes would be - and
/// re-spawns it every tick, which is what makes it follow.
///
/// **`reach` unconditionally, never the distance to the contact.** Clamping to
/// `min(|contact - centre|, reach)` was the second wrong answer here, and it is
/// a no-op precisely when it matters: the burst only fires when the craft is
/// *within* reach of the ribbon, so the clamp always picked the distance and
/// left the burst sitting on the trail - which is exactly how it looked.
pub(super) fn hull_contact_point(centre: Vec3, contact: Vec3, reach: f32) -> Vec3 {
    (contact - centre)
        .try_normalize()
        .map_or(centre, |direction| centre + direction * reach)
}

/// Every `Data\Psys` effect this race loads, and what triggers it.
///
/// **The list is the trigger set, not the asset set.** There are 35 effects on
/// the PSP disc and 41 on the PS2 one; what decides whether one appears here is
/// whether the *executable's* reason for playing it has been recovered, because
/// an effect with no recovered trigger would just be this engine guessing when
/// to fire it. `crates/game/tests/psys_inventory_ground_truth.rs` holds every
/// effect on both discs against this list and fails if one is neither played
/// nor explicitly recorded as having no recovered trigger.
///
/// **The two [`TRAIL_HITSHIP_EFFECT`] variants are the one exception**, and a
/// narrow one: their consumer, spawner and colour select are all read, and
/// only the geometric *test* - which runs in the `Trails` SPU job - is not.
/// Nothing about the effect is invented, only the moment it fires, and
/// [`Race::advance_trail_hits`] names each approximation. HD is also outside
/// the inventory test's reach (it reads the PSP and PS2 discs only), so those
/// two names are checked by nothing there - stated rather than relied on.
///
/// **It is a superset across sources, not a per-disc list.** An entry absent
/// from the mounted archives is reported by the loader and skipped, so naming
/// a PS2-only effect here costs a PSP race one report line and nothing else.
pub const RACE_EFFECTS: [&str; 7] = [
    sparks::DAMAGE_EFFECT,
    ROCKET_FLARE_EFFECT,
    TRACK_BLAST_EFFECT,
    CRAFT_BLAST_EFFECT,
    ENGINE_FLARE_EFFECT,
    TRAIL_HITSHIP_EFFECT,
    TRAIL_HITSHIP_RED_EFFECT,
];

impl Race {
    /// Advances every active craft's exhaust and lays down its trail sample.
    ///
    /// The original updates the exhaust inside the per-craft update, so all eight
    /// flares ramp, flicker and trail on their own state; this is that fan-out.
    /// A craft's ramp follows *its* thrust and *its* speed, so an opponent
    /// coasting into a corner dims while the leader on the straight does not.
    ///
    /// **Slot order, and slot 0 first.** Each craft draws from its own generator
    /// ([`exhaust_seed`]), so the order cannot change what any of them sees - but
    /// the player's stream is the one captures are pinned against, and keeping it
    /// first keeps the tick's shape the same as when the player was the only
    /// craft with a flare at all.
    ///
    /// The trail sample is one per tick, which is what `Trail_Update` does per
    /// frame - it takes no `dt` at all. The direction is the nozzle's own
    /// backwards axis, so a segment keeps the orientation the craft had when it
    /// was laid down rather than swinging with the current pose as the ship
    /// turns.
    pub(super) fn advance_exhausts(&mut self) {
        for slot in 0..self.world.ship_count as usize {
            if !self.world.ships[slot].active {
                continue;
            }
            let ship = &self.world.ships[slot].physics;
            let position = ship.body.position;
            let thrust = ship.thrust;
            let speed = ship.body.linear_velocity.length();
            let back = -ship.body.forward();
            self.exhaust[slot].advance(self.dt, thrust, speed, &mut self.exhaust_rng[slot]);
            if self.hd_trail_active {
                // HD's flame blends: the boost side opens on the same 0.2
                // gate the plume reveal reads - HD's own `DAT_008b2ff4` is
                // the PSP's `BOOST_GATE` value exactly.
                self.hd_flame[slot].advance(
                    thrust > 0.0,
                    self.exhaust[slot].boost_timer() > exhaust::BOOST_GATE,
                );
                let rng = &mut self.exhaust_rng[slot];
                self.hd_sprite[slot].advance(|| rng.next_f32());
            }
            if let Some(nozzle) = self.nozzle_of(slot) {
                // How far this hull reaches from its own origin, in world
                // units: the trail-hit test's stand-in for the bounding sphere
                // HD hands its SPU job. Taken here because the nozzle is
                // already in hand and it is the one authored point on the hull
                // this engine knows the world position of.
                self.hull_reach[slot] = (nozzle - position).length();
                self.exhaust[slot].push_trail(nozzle, back);
                if self.hd_trail_active {
                    // HD's tube wants the craft's *up* (fin 0's axis) and the
                    // normalised speed its scroll and stretch ride on. The
                    // throttle share of `speed01` is authored against a 0..1
                    // throttle; this engine's thrust input is effectively
                    // binary, so on/off is the whole of it here.
                    let s01 = exhaust::hd::speed01(
                        self.exhaust[slot].speed_kmh(),
                        if thrust > 0.0 { 1.0 } else { 0.0 },
                    );
                    self.hd_trail[slot].push(nozzle, self.world.ships[slot].physics.body.up(), s01);
                }
            }
        }
    }

    /// Fires `WO_TRAIL_HITSHIP` on a craft that has flown into a trail.
    ///
    /// **The mechanism is read; the test is not.** Wipeout HD's `Trails` SPU
    /// job raises a per-trail flag (bit 61 of the trail block's `+0x11d0`),
    /// `Trail_SpawnHitEffect` consumes it once a frame and spawns the system
    /// on the craft involved, and the red variant is chosen by the same
    /// `craft + 0x7d2c` byte that reddens the ribbon. All of that is
    /// decompiled (`docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md`).
    /// What runs inside the SPU job is not, so **three things here are this
    /// engine's**, named rather than buried:
    ///
    /// 1. **The test.** A craft counts as in a trail when its centre comes
    ///    within its own [`Race::hull_reach`] plus the ribbon's measured
    ///    half-width of that trail's centre line
    ///    ([`exhaust::hd::Tube::nearest`]). Both terms are measured - the
    ///    reach from the craft's own authored nozzle locator, the half-width
    ///    from the running game - but that a sphere against a centre line is
    ///    the original's test is an assumption, taken because the SPU job is
    ///    handed exactly one bounding-sphere-shaped vec4 per craft and nothing
    ///    per fin. **A reach in the wrong space is what this got wrong first**:
    ///    `mesh::Model::radius` is a model-space AABB half-extent reading 69 to
    ///    379 on HD's hulls, which put every craft inside every trail from tick
    ///    0, fired 24 bursts on the grid and then never re-armed for the rest
    ///    of the race.
    /// 2. **The rate: every tick a craft is inside, not once on entry.** The
    ///    asset settles this and a first pass got it wrong. `WO_TRAIL_HITSHIP`
    ///    is authored as a **one-shot**: duration 1 tick, `looping` false,
    ///    5 particles of 0.2..0.5 units whose size channel is down to a third
    ///    by 17 % of its life. A system shaped like that is meant to be
    ///    re-fired while the condition holds - fired once per entry it is five
    ///    streaks and all but invisible, which is exactly how this first
    ///    behaved. It also matches the original's own shape, where the flag is
    ///    consumed *and cleared every frame*, so a craft that stays inside
    ///    re-raises it. Against the emitter's 2,000 live cap, a continuous
    ///    stream is a few hundred particles. What stays unread is whether the
    ///    SPU job really does re-raise it every frame, which is why this is
    ///    still listed as an approximation.
    /// 3. **The attachment point.** The original picks the nearest of ten hull
    ///    nodes at `craft + 0x79d0..+0x79f4` and *parents* the system to it, so
    ///    the burst rides the ship. This engine has no such nodes, so it takes
    ///    the point on the intruder's own bounding sphere facing the contact -
    ///    where the nearest of ten hull nodes would be - and re-spawns it every
    ///    tick, which is what makes it follow. **Spawning at the contact point
    ///    on the ribbon was the first attempt and is wrong**: it reads as sparks
    ///    on the trail rather than on the craft, which is how it was caught.
    ///
    /// **Which craft it lands on is settled by observation, not by the code.**
    /// The disassembly ties the attachment node, the colour and the float all
    /// to the craft at `+0x11f0`, and nothing read says whether that is the
    /// trail's owner or the craft that flew through it. A sighting in the
    /// running original - sparks on the *ship*, in the trail's colours -
    /// resolves it: the intruder. So the burst plays on the intruder and takes
    /// the intruder's own red flag, which on a uniformly Fury or classic grid
    /// is the same answer either way.
    ///
    /// A craft is never tested against its own trail: its nozzle *is* the head
    /// sample, so it would be permanently inside it.
    pub(super) fn advance_trail_hits(&mut self) {
        if !self.hd_trail_active {
            return;
        }
        self.force_trail_sparks();
        for owner in 0..self.world.ship_count as usize {
            if !self.world.ships[owner].active || !self.hd_trail[owner].ready() {
                continue;
            }
            for intruder in 0..self.world.ship_count as usize {
                if intruder == owner || !self.world.ships[intruder].active {
                    continue;
                }
                let at = self.world.ships[intruder].physics.body.position;
                let reach = self.hull_reach[intruder] + exhaust::hd::FIN_HALF_WIDTH;
                let hit = self.hd_trail[owner]
                    .nearest(at)
                    .is_some_and(|(_, distance)| distance <= reach);
                let bit = 1u8 << intruder;
                if hit {
                    self.trail_inside[owner] |= bit;
                } else {
                    self.trail_inside[owner] &= !bit;
                    continue;
                }
                let name = if self.hd_trail_red[intruder] > 0.5 {
                    TRAIL_HITSHIP_RED_EFFECT
                } else {
                    TRAIL_HITSHIP_EFFECT
                };
                let Some(effect) = self.effects.get(name).cloned() else {
                    continue;
                };
                // **On the hull, not on the ribbon.** The original parents the
                // system to the nearest of ten attachment nodes on the craft
                // (`craft + 0x79d0..+0x79f4`), so it rides the ship; putting it
                // at the contact point on the ribbon instead reads as sparks on
                // the *trail*, which is what this did first and what a player
                // spotted. This engine has no equivalent of those nodes, so it
                // takes the point on the craft's own bounding sphere facing the
                // contact - the place the nearest of ten hull nodes would be -
                // and re-spawns it every tick, which is what makes it follow.
                let contact = self.hd_trail[owner].nearest(at).map_or(at, |(on, _)| on);
                let toward = contact - at;
                let point = toward.try_normalize().map_or(at, |dir| {
                    at + dir * toward.length().min(self.hull_reach[intruder])
                });
                // Neutral severity, as the rocket blast uses: the scale the
                // collision sparks derive from an impulse has no counterpart
                // here, and the float the original carries into the spawner
                // (`+0x11dc`) is unread.
                self.stage.play(&effect, point, 1.0);
            }
        }
    }

    /// `--trail-sparks`: the burst, every tick, on the player's own ribbon.
    ///
    /// **A verification aid for the *drawing*, and the two failures it
    /// separates are real ones this hit in order.** The trigger fires a handful
    /// of times in a whole race and almost never in front of the camera, so
    /// "I saw nothing" cannot tell a burst that never played from one that
    /// played and drew nothing.
    ///
    /// Everything but the *placement* is the real path - the same effect, the
    /// same rate, the same [`hd_trail_red`](Race::hd_trail_red) colour select.
    /// The placement is the one thing chosen rather than derived: two units
    /// above the player's nozzle, which is on screen and clear of the exhaust
    /// plume. Three alternatives were measured and each looks like nothing or
    /// like the plume - at the nozzle it lands inside the plume; eight units
    /// back along the ribbon it sits at the camera's near plane; and letting
    /// the real pairwise path through with the distance test bypassed spawns it
    /// on whichever rival's ribbon is nearest, which when the player leads is a
    /// hundred units behind. **So this placement is for looking at, and says
    /// nothing about where a real hit lands.**
    fn force_trail_sparks(&mut self) {
        if !self.trail_sparks {
            return;
        }
        // The same gate the real path has: a craft with no nozzle locator has
        // no trail either, so there is nothing to demonstrate.
        if self.nozzle_of(0).is_none() {
            return;
        }
        // The real placement, with only the contact *direction* chosen: on the
        // hull, facing straight up, so it is on screen and clear of the exhaust
        // plume. What appears under the flag is therefore anchored exactly as a
        // real hit is - see [`hull_contact_point`].
        let body = self.world.ships[0].physics.body;
        let probe = hull_contact_point(
            body.position,
            body.position + body.up() * 10.0,
            self.hull_reach[0].max(1.0),
        );
        let name = if self.hd_trail_red[0] > 0.5 {
            TRAIL_HITSHIP_RED_EFFECT
        } else {
            TRAIL_HITSHIP_EFFECT
        };
        let Some(effect) = self.effects.get(name).cloned() else {
            return;
        };
        self.stage.play(&effect, probe, 1.0);
    }

    /// One craft's HD trail geometry this frame, empty unless this race's
    /// ribbon is HD's own tube (and then until the ring fills).
    ///
    /// The brightness handed down is `intensity * speed_ramp` - the product
    /// HD's `EngineFlare_Update` writes into the trail block every frame,
    /// and both factors are the same quantities [`Exhaust`] already tracks
    /// with HD's own constants (rise 0.25/s, fall 0.5/s; floor 100 km/h,
    /// span 500).
    #[must_use]
    pub fn hd_trail_vertices(&self, slot: usize) -> Vec<oag_render::mesh::GpuVertex> {
        if !self.hd_trail_active {
            return Vec::new();
        }
        let exhaust = &self.exhaust[slot];
        // HD's ramp, not Pulse's: the gained speed field saturates it near
        // 400 km/h, so a craft at pace trails at or near full brightness -
        // see `hd::speed_ramp` and the live table on engine-trail.md.
        let brightness = exhaust.intensity() * exhaust::hd::speed_ramp(exhaust.speed_kmh());
        let s01 = exhaust::hd::speed01(
            exhaust.speed_kmh(),
            if exhaust.engine_on() { 1.0 } else { 0.0 },
        );
        self.hd_trail[slot].vertices(brightness, s01, self.hd_trail_red[slot])
    }

    /// Whether this race draws HD's trail tube instead of the PSP ribbon.
    #[must_use]
    pub fn hd_trail_active(&self) -> bool {
        self.hd_trail_active
    }

    /// One craft's HD flame blends, for the flare model's group scales.
    #[must_use]
    pub fn hd_flame_of(&self, slot: usize) -> &exhaust::hd::Flame {
        &self.hd_flame[slot]
    }

    /// One craft's sprite-flare quad: `Engine_Flare_Rich.gtf` at the nozzle,
    /// at this tick's jittered radius and walked alpha. Empty off HD or
    /// where no locator exists.
    #[must_use]
    pub fn hd_sprite_quad(
        &self,
        slot: usize,
        right: Vec3,
        up: Vec3,
    ) -> Vec<oag_render::mesh::GpuVertex> {
        if !self.hd_trail_active {
            return Vec::new();
        }
        let Some(nozzle) = self.nozzle_of(slot) else {
            return Vec::new();
        };
        let sprite = &self.hd_sprite[slot];
        exhaust::sprite(nozzle, right, up, sprite.radius(), sprite.alpha()).to_vec()
    }

    /// Advances every craft's shield shell, and starts the fade on the tick its
    /// timer runs out.
    ///
    /// **Driven off `shield_pickup_timer` rather than off a second flag**, so
    /// the picture cannot disagree with the simulation about whether a craft is
    /// protected. The original has the same single source: `Shield_Update`
    /// (`0x08861630`) is what both lowers the fire flag and calls the visual's
    /// deactivate, in that order, on the tick the countdown reaches zero.
    ///
    /// The shell is **not** hidden here - `ShipShield::deactivate` starts a fade
    /// the shell finishes on its own, which is the original's arrangement and
    /// the reason a shield dissolves rather than blinking out. So a shell
    /// outlives its craft's protection by a few frames, on purpose.
    ///
    /// Runs over every slot rather than the active ones: an inactive craft's
    /// shell has nothing to draw it, and skipping the advance would freeze a
    /// mid-fade shell for a craft that comes back.
    pub(super) fn advance_shields(&mut self) {
        for slot in 0..MAX_SHIPS {
            if self.world.ships[slot].physics.shield_pickup_timer <= 0.0 {
                self.shield[slot].deactivate();
            }
            self.shield[slot].advance(self.dt);
        }
    }

    /// One craft's shield shell, by slot.
    #[must_use]
    pub fn shield_of(&self, slot: usize) -> &ShipShield {
        &self.shield[slot]
    }

    /// The **player's** exhaust animation state.
    ///
    /// Slot 0, which is what every capture and every pinned exhaust number is
    /// taken against. [`Self::exhaust_of`] is the one to reach for when drawing
    /// the field.
    #[must_use]
    pub fn exhaust(&self) -> &Exhaust {
        &self.exhaust[0]
    }

    /// One craft's exhaust animation state.
    ///
    /// Indexed by ship slot, the player at 0. A slot past [`Self::ship_count`]
    /// holds a cold `Exhaust` that nothing advances - drawn, it would be an
    /// invisible flare at whatever pose an unused ship slot carries, so a caller
    /// iterating the field bounds itself by `ship_count` the way the hull draw
    /// does.
    ///
    /// # Panics
    ///
    /// If `slot` is not a ship slot.
    #[must_use]
    pub fn exhaust_of(&self, slot: usize) -> &Exhaust {
        &self.exhaust[slot]
    }

    /// Held throttle on the original's `0..=100` scale, for
    /// [`Race::force_boost_state`]'s synthetic warmup.
    ///
    /// **This constant exists because a `1.0` here was a 100x bug**, found by
    /// the first numeric comparison of our exhaust state against a capture's
    /// (`oag-game --trace-out` against `data/traces/pad0-boost.csv`). The live
    /// path at [`Race::tick`] passes `ship.thrust`, and
    /// `oag_physics::ship::Ship::thrust` is documented as the raw `0..=100`
    /// throttle the original stores at `craft+0x2b8` - the capture reads
    /// `throttle = 100` at the compared tick. Passing `1.0` charged the boost
    /// accumulator at `1/3000` per tick instead of `100/3000`, so a posed frame
    /// never left the speed ramp's floor: `0.5436` against the original's
    /// saturated `1.0000`, an error that did **not** move when the pose age was
    /// corrected, which is what proved it was the code rather than the
    /// parameter.
    ///
    /// A named constant rather than a bare `100.0` so the scale is stated where
    /// it is used; the two are easy to confuse precisely because
    /// `ShipControls::thrust` on the *input* side is `0.0..=1.0`.
    const FULL_THRUST: f32 = 100.0;

    /// Drives the exhaust to the state a speed pad entry `age` seconds ago
    /// would leave it in - `CaptureOptions::pose_boost`.
    ///
    /// Replays [`Exhaust::advance`] at the fixed step rather than poking
    /// fields: enough ticks of full thrust to reach `entry_intensity`,
    /// [`Exhaust::boost`] on the entry edge, then `age` more seconds of the
    /// same advance, so the flare size, the plume reveal timer and the flicker
    /// generator all sit exactly where a real crossing puts them.
    ///
    /// `entry_intensity` is `None` for a saturated ramp, which is what a craft
    /// that has been racing for four seconds or more actually has. **Pass the
    /// measured value when comparing against a capture taken after a
    /// teleport**: `psp-drive.py place` leaves the flare's ramp wherever the
    /// craft's idle time left it. That is not a cosmetic difference -
    /// intensity sets the flare's resting half-size through
    /// `(i * 0.6 + 0.4) * 2.5` (`1.2` against a saturated `2.5`) and every one
    /// of the ribbon's three staggered layer alphas - so a saturated render
    /// against an unsaturated capture differs in *state* before it differs in
    /// anything a renderer does.
    ///
    /// Matching only the entry value is enough to match the whole curve,
    /// because the ramp climbs at the same recovered `0.25`/s on both sides
    /// through the posed age. **That rate is now confirmed numerically**: the
    /// capture's own per-tick rise reads `0.0041708`, and `0.25 / 59.94` is
    /// `0.0041708`.
    ///
    /// **Pass the intensity of the tick *before* the pad fires, not the entry
    /// tick's.** [`Exhaust::boost`] arms the timer and the *next* `advance`
    /// is the entry tick - it both decays `boost_timer` by one step and raises
    /// intensity by one step, which is exactly what the capture's entry row
    /// already shows (`boost_timer` reads `0.783316`, i.e. `0.8 - dt`, not
    /// `0.8`). So passing the entry row's own intensity counts that tick twice.
    /// Measured on `pad0-boost.csv` tick 62: the entry row's `0.1292277` lands
    /// `+0.0080` high, the row before it (`0.1250567`) lands `+0.0003` - and
    /// that remainder is the fixed-60 Hz against variable-59.94 Hz difference
    /// [ADR-0007](../../../docs/architecture/adr/0007-fixed-timestep-vs-original.md)
    /// accepts, not a model error.
    ///
    /// `speed` is `None` for the racing `120` units/s. It reaches the picture
    /// only through the speed ramp's floor on the boost accumulator, so it
    /// matters when a capture's speed is far from that.
    pub fn force_boost_state(
        &mut self,
        age: f32,
        entry_intensity: Option<f32>,
        speed: Option<f32>,
    ) {
        let speed = speed.unwrap_or(120.0);
        // Whole ticks, then a **short final tick** for the remainder, so the
        // ramp lands exactly on `target` instead of up to one tick past it.
        //
        // This used to be `ceil`, and the overshoot was real: the first numeric
        // comparison against a capture measured `intensity` `+0.0041` high, one
        // whole tick of `INTENSITY_RISE * dt`, purely from rounding the warmup
        // up. Still a replay of `advance` rather than a poked field - the
        // principle this function is built on - because a shortened step is
        // something the original's own variable timestep does anyway.
        let target = entry_intensity.unwrap_or(1.0).clamp(0.0, 1.0);
        let ticks = target / exhaust::INTENSITY_RISE / self.dt;
        // **The player's, and only the player's.** A pose comparison is against
        // one captured craft; posing the whole field would put seven opponents
        // into a boost the capture says nothing about.
        let (player, rng) = (&mut self.exhaust[0], &mut self.exhaust_rng[0]);
        for _ in 0..(ticks.floor() as u32) {
            player.advance(self.dt, Self::FULL_THRUST, speed, rng);
        }
        let remainder = (ticks - ticks.floor()) * self.dt;
        if remainder > 0.0 {
            player.advance(remainder, Self::FULL_THRUST, speed, rng);
        }
        player.boost(exhaust::BOOST_SECONDS);
        let aged = (age / self.dt).round() as u32;
        for _ in 0..aged {
            player.advance(self.dt, Self::FULL_THRUST, speed, rng);
        }

        // **A posed frame otherwise has no ribbon at all, and that silently
        // wrecks a boost comparison.** `Trail_DrawRibbon` refuses to draw until
        // its ring is full, our [`Exhaust::trail_ready`] reproduces that, and a
        // `--pose-from --ticks 0` capture never runs a tick that would push a
        // sample - so every posed frame before this was missing the one element
        // that carries the exhaust's colour. Measured on the first real pad
        // capture: the original's rear reads magenta (mean `(224, 164, 217)`,
        // peaks near `(252, 109, 214)`, red and blue both far above green) over
        // 6,355 pixels of one frame, against 491 pixels of blue-dominant violet
        // `(180, 141, 229)` in ours - and most of that difference was the
        // missing ribbon, not the flare or the plume.
        //
        // The history is laid down straight, back along the nozzle's own axis at
        // `speed * dt` per sample, oldest pushed first. That is exactly what a
        // real run produces here: a speed pad sits on a straight, and over the
        // ten ticks a full ring spans the captured positions are collinear to
        // far below a pixel. It is a *capture* approximation and nothing in the
        // game uses it - a real race pushes one true sample per tick from
        // `Race::tick`.
        if let Some(nozzle) = self.nozzle() {
            let back = -self.ship().physics.body.forward();
            self.exhaust[0].clear_trail();
            for k in (0..exhaust::TRAIL_SAMPLES).rev() {
                let behind = back * (speed * self.dt * k as f32);
                self.exhaust[0].push_trail(nozzle + behind, back);
            }
            // The HD tube on the same straight-line terms, when it is the
            // ribbon this race draws.
            if self.hd_trail_active {
                let up = self.ship().physics.body.up();
                let s01 = exhaust::hd::speed01(self.exhaust[0].speed_kmh(), 1.0);
                self.hd_trail[0].clear();
                for k in (0..exhaust::hd::SAMPLES).rev() {
                    let behind = back * (speed * self.dt * k as f32);
                    self.hd_trail[0].push(nozzle + behind, up, s01);
                }
            }
        }
    }

    /// The collision sparks' geometry this frame, split by blend class.
    ///
    /// The pool and the effect it plays are handed out together because
    /// neither means anything alone - a particle carries an index into the
    /// effect's emitters rather than a copy of their parameters. Empty when
    /// the disc's own effect did not load.
    #[must_use]
    pub fn spark_vertices(
        &self,
        right: Vec3,
        up: Vec3,
    ) -> (
        Vec<oag_render::mesh::GpuVertex>,
        Vec<oag_render::mesh::GpuVertex>,
    ) {
        self.effects
            .get(sparks::DAMAGE_EFFECT)
            .map(|effect| self.sparks.vertices(effect, right, up))
            .unwrap_or_default()
    }

    /// Everything the [`psys::Stage`] is playing this frame, split by blend
    /// class the same way [`Self::spark_vertices`] is.
    ///
    /// The rocket flares and the detonations today. Uploaded through the same
    /// [`oag_render::psys::Pipeline`] as the sparks - one pass, two buffers,
    /// no third pipeline per effect.
    #[must_use]
    pub fn stage_vertices(
        &self,
        right: Vec3,
        up: Vec3,
    ) -> (
        Vec<oag_render::mesh::GpuVertex>,
        Vec<oag_render::mesh::GpuVertex>,
    ) {
        self.stage.vertices(right, up)
    }

    /// The pool the rocket effects play in.
    #[must_use]
    pub fn stage(&self) -> &psys::Stage {
        &self.stage
    }

    /// Whether this source authors an engine flare as a particle effect, and
    /// it loaded.
    ///
    /// The renderer asks this to decide whether to draw
    /// [`oag_render::exhaust`]'s procedural flare quad: drawing both would
    /// put an invented glow on top of the authored one, which is the exact
    /// thing `CLAUDE.md`'s do-not-invent rule forbids. The trail ribbon and
    /// the boost plume are unaffected - the PS2's effect is the *flare*, and
    /// neither of those has an authored counterpart on either disc.
    #[must_use]
    pub fn engine_flare_effect(&self) -> bool {
        self.effects.get(ENGINE_FLARE_EFFECT).is_some()
    }

    /// How many spark bursts the contact rule has triggered.
    ///
    /// Independent of whether an effect was loaded to play them - see
    /// [`Self::sparks_ignitions`].
    #[must_use]
    pub fn spark_ignitions(&self) -> u32 {
        self.sparks_ignitions
    }

    /// The collision sparks' current particle pool.
    #[must_use]
    pub fn sparks(&self) -> &psys::System {
        &self.sparks
    }

    /// Raises `ABSORB` or `.COLLISIONS` for one craft, on that craft's own
    /// re-arm.
    ///
    /// **The timer is per craft**, because the original's gate is: the cooldown
    /// lives on the effect the *craft* owns, so eight hulls scraping eight
    /// walls make eight sounds rather than sharing one 0.8-second window.
    ///
    /// It cannot share [`Race::sparks_cooldown`] either: a shielded contact
    /// never ignites, so that timer would sit at zero and `ABSORB` would fire
    /// sixty times a second through a scrape.
    ///
    /// `ShipCollisionFx_Trigger` plays `"COLLISIONS"` once per call that gets
    /// past its own 0.8-second gate, which is the same 0.8 seconds
    /// `oag_render::sparks::COLLISION_COOLDOWN` carries - one constant in the
    /// original, read twice here. `"ABSORB"` comes from `FUN_08840640`, which
    /// **bypasses** that gate and staggers its own ten instances by 0.1 s; how
    /// often the game calls it is not recovered, so it is re-armed on the same
    /// 0.8 s and that is a stated approximation rather than a reading. See
    /// `docs/.../contact-response.md`.
    pub(super) fn raise_contact_cue(&mut self, slot: usize, impact: bool, shielded: bool) {
        let Some(cooldown) = self.contact_cue_cooldown.get_mut(slot) else {
            return;
        };
        *cooldown = (*cooldown - self.dt).max(0.0);
        if !impact || *cooldown > 0.0 {
            return;
        }
        *cooldown = oag_render::sparks::COLLISION_COOLDOWN;
        let cue = if shielded {
            crate::audio::sfx::Cue::Absorb
        } else {
            crate::audio::sfx::Cue::Collision
        };
        self.cues.push(crate::audio::sfx::CueEvent::new(cue, slot));
    }

    /// Takes the one-shot sound cues this tick raised, leaving the queue empty.
    ///
    /// The seam ADR-0018 asks for: cues are a per-tick *output*, so the caller
    /// that owns the mixer drains them **inside the fixed-step loop**, next to
    /// the tick that raised them. Draining once per frame instead would
    /// coalesce or duplicate them on a frame that stepped twice or none.
    ///
    /// A caller with no audio need never call this. The queue is one `Vec` and
    /// a race that is never drained grows it by at most a handful of entries a
    /// second, all of them fixed-size - but the composition root does drain it,
    /// and a headless test that does not is bounded by its own tick count.
    pub fn drain_cues(&mut self) -> Vec<crate::audio::sfx::CueEvent> {
        std::mem::take(&mut self.cues)
    }

    /// The cues raised so far and not yet drained, for tests.
    #[must_use]
    pub fn pending_cues(&self) -> &[crate::audio::sfx::CueEvent] {
        &self.cues
    }

    /// The decoded sound banks this race loaded.
    #[must_use]
    pub fn sounds(&self) -> &crate::audio::sfx::Banks {
        &self.sounds
    }

    /// Whether the player's craft is mid-explosion.
    ///
    /// A *level*, like [`Self::shield_is_up`] and for the same reason:
    /// `Ship_SetState`'s case 4 opens `~BLOWUP` with a handle and keeps it at
    /// `craft+0xcac`, so it is a held voice rather than a one-shot. See
    /// [zone-mode.md](../../../docs/ghidra/functions/psp-pulse-usa/zone-mode.md).
    ///
    /// **Where the original releases that handle is unread**, so this port ties
    /// the voice to the state's own recovered `0.5 s` - the shortest lifetime
    /// the evidence supports, and stated rather than assumed. If the original
    /// turns out to hold it through state 5 as well, this predicate is the one
    /// line that changes.
    #[must_use]
    pub fn craft_is_exploding(&self) -> bool {
        self.world.ships[0].physics.craft_state == oag_physics::CraftState::Destroyed
    }

    /// Whether the player's shield pickup is currently up.
    ///
    /// A *level*, not an edge, and that is what `~SHIELD` wants:
    /// `Shield_Activate` opens a looping voice with a handle and the shield's
    /// own expiry releases it, so the voice's lifetime is exactly this
    /// predicate's. The same timer gates the collision sparks - see
    /// [`Self::tick`] - so "the shell is up" is one fact read in two places.
    #[must_use]
    pub fn shield_is_up(&self) -> bool {
        self.world.ships[0].physics.shield_pickup_timer > 0.0
    }

    /// The track's speed-pad trigger volumes.
    ///
    /// Empty on a track that authors none, which is an ordinary state rather than
    /// a failure - see [`Setup::speedup_pads`].
    #[must_use]
    pub fn speedup_pads(&self) -> &[oag_formats::pads::PadVolume] {
        &self.speedup_pads
    }

    /// Seconds each weapon pad has left before it can hand out another
    /// pickup - `0.0` (or less) means armed and ready. Empty on a track with
    /// no weapon pads, or when the mode disarms them (`Race::weapon_pads`'s
    /// own doc comment has why an empty list is the disarmed state rather
    /// than a per-tick check).
    ///
    /// One entry per weapon pad, in the track file's own node order - the
    /// same order `oag_render::mesh::Model::node_vertex_ranges` walks the
    /// weapon pad model in, which is what lets a caller pair position `i`
    /// here with that list's `i`-th vertex range.
    #[must_use]
    pub fn weapon_pad_refresh_left(&self) -> &[f32] {
        &self.weapon_pad_refresh_left
    }

    /// The player's `engine_flare` locator in **world** space, or `None` when the
    /// ship model carries no `Engine Flare` node.
    ///
    /// Composed through [`Race::ship_model_matrix`], which is the only correct
    /// route: the locator is authored in `.vex` model space, so it has to pick up
    /// [`MODEL_YAW`] exactly as the hull's vertices do. A nozzle built from
    /// `body.forward()` instead would be a half-turn out and would sit on the
    /// ship's nose.
    #[must_use]
    pub fn nozzle(&self) -> Option<Vec3> {
        self.nozzle_of(0)
    }

    /// One craft's `engine_flare` locator in **world** space.
    ///
    /// [`Self::nozzle`] one slot over. The *model-space* locator is the same for
    /// every craft while the whole field wears the player's hull; what differs is
    /// the matrix it is carried through, which is that craft's own pose.
    ///
    /// # Panics
    ///
    /// If `slot` is not a ship slot.
    #[must_use]
    pub fn nozzle_of(&self, slot: usize) -> Option<Vec3> {
        let local = self.nozzle_local_of(slot)?;
        Some(model_matrix_of(&self.world.ships[slot]).transform_point3(local))
    }

    /// The `engine_flare` locator in **model space** - the pivot a transform
    /// composed with the craft's own matrix wants, where [`Self::nozzle_of`]
    /// is the world-space point the trail is laid from. Mixing the two up
    /// scales the flame about a point hundreds of units away.
    #[must_use]
    pub fn nozzle_local_of(&self, slot: usize) -> Option<Vec3> {
        self.nozzles.get(slot).copied().flatten()
    }
}

/// The exhaust flicker seed for one craft.
///
/// Slot 0 is [`EXHAUST_SEED`] itself and an opponent is that plus its slot, so
/// eight craft side by side on the grid do not flicker in lockstep. Adjacent
/// seeds are safe to use this way because `Rng::new` runs a SplitMix64 expansion
/// over the seed before the first draw, which is what decorrelates them; a raw
/// LCG would need a stride instead.
pub(super) fn exhaust_seed(slot: usize) -> u64 {
    EXHAUST_SEED + slot as u64
}
