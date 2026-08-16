//! Every craft's exhaust, the particle stage the disc's own effects play on,
//! and the sparks a contact throws.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::*;

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
            let thrust = ship.thrust;
            let speed = ship.body.linear_velocity.length();
            let back = -ship.body.forward();
            self.exhaust[slot].advance(self.dt, thrust, speed, &mut self.exhaust_rng[slot]);
            if let Some(nozzle) = self.nozzle_of(slot) {
                self.exhaust[slot].push_trail(nozzle, back);
            }
        }
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

    /// The track's speed-pad trigger volumes.
    ///
    /// Empty on a track that authors none, which is an ordinary state rather than
    /// a failure - see [`Setup::speedup_pads`].
    #[must_use]
    pub fn speedup_pads(&self) -> &[oag_formats::pads::PadVolume] {
        &self.speedup_pads
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
        let local = self.nozzles.get(slot).copied().flatten()?;
        Some(model_matrix_of(&self.world.ships[slot]).transform_point3(local))
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
