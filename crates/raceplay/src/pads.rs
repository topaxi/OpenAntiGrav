//! Speed pads and weapon pads: the swept test a craft crosses one with, and
//! what each class does when it fires.
//!
//! Split out of `race.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. Its tests are
//! `race/tests/pads.rs`.

use super::*;

impl Race {
    /// Below this much movement in a tick, the pad test is swept instead of a
    /// point.
    ///
    /// `Pad_SweptTest`'s `25.0`. It reads backwards at first - the *slow* case
    /// gets the more careful test - and the reason is that the interpolation is
    /// only worth anything when the step is short enough that four samples cover
    /// it. Past this the ship has moved further than a pad is deep and four points
    /// would not close the gap either, so the original stops paying for them.
    /// Confidence 88: measured live against a running PPSSPP session - a
    /// stationary or driving craft never crosses it (`0.000162`/`0.778459`
    /// units of movement observed), a scripted 200-unit same-tick teleport
    /// does (`199.999969`, confirmed to take the direct-test branch by
    /// breakpoint). See `pads.md`'s `Pad_SweptTest` section.
    pub(super) const PAD_SWEEP_LIMIT: f32 = 25.0;

    /// How many interpolated points the swept test checks.
    ///
    /// `Pad_SweptTest` walks `t = 0.25, 0.5, 0.75, 1.0` - the destination is
    /// included and the origin is not, because the origin was this test's
    /// destination last tick.
    pub(super) const PAD_SWEEP_STEPS: u32 = 4;

    /// How far the ship moved since the last pad test, and the path it swept.
    ///
    /// One computation shared by both pad triggers, because the original's two -
    /// `Pads_TestCraft` and `WeaponPads_TestCraft` - run in the same craft
    /// update against the same recorded previous position. Computing it twice
    /// would be harmless today and wrong the moment either advanced the record,
    /// which is exactly the mistake `pad_previous_position.replace` invites.
    ///
    /// The sweep's destination is included and its origin is not, because the
    /// origin was the previous test's destination. A stationary ship and a long
    /// jump both fall through to the single point: interpolating a zero-length
    /// step adds nothing, and interpolating a long one does not close the gap.
    pub(super) fn pad_sweep(&mut self, slot: usize, position: Vec3) -> (f32, Vec<Vec3>) {
        let previous = self.sim.pad_previous_position[slot].replace(position);
        let moved = previous.map_or(f32::INFINITY, |from| position.distance(from));
        let sweep = match previous {
            Some(from) if (0.0..Self::PAD_SWEEP_LIMIT).contains(&moved) && moved > 0.0 => (1
                ..=Self::PAD_SWEEP_STEPS)
                .map(|step| from.lerp(position, step as f32 / Self::PAD_SWEEP_STEPS as f32))
                .collect(),
            _ => vec![position],
        };
        (moved, sweep)
    }

    /// Which way the speed pad under the ship pushes, or `None` if there is none.
    ///
    /// Reimplements `Pads_TestCraft` (`0x08887144`) and the two functions under
    /// it. Called once a tick with the position the tick *starts* at, and returns
    /// the direction `oag_physics`' step-15 term needs; see
    /// [`oag_physics::forces::Environment::pad_hit`], which is deliberately a
    /// per-tick containment answer rather than an entry edge.
    ///
    /// Also does the two things that happen on **entering a new** pad, because
    /// both are edges on the same value the original latches at `craft+0x1d0`:
    /// the Zone score, and arming the exhaust flare.
    ///
    /// `moved` and `sweep` come from [`Self::pad_sweep`] and are shared with
    /// [`Self::test_weapon_pads`], because the original's two trigger functions
    /// run in the same craft update against the same previous position.
    pub(super) fn test_speedup_pads(
        &mut self,
        slot: usize,
        position: Vec3,
        moved: f32,
        sweep: &[Vec3],
    ) -> Option<Vec3> {
        if self.sim.speedup_pads.is_empty() {
            return None;
        }

        let mut hit = None;
        for (index, pad) in self.sim.speedup_pads.iter().enumerate() {
            // The broadphase. Spend the distance travelled, and skip until it is
            // used up. `moved` is infinite on the first tick, so every pad is
            // measured once before any of them is skipped.
            self.sim.pad_distance[slot][index] -= moved;
            if self.sim.pad_distance[slot][index] > 0.0 {
                continue;
            }

            // Measured from the destination, which is where the cache has to be
            // correct from for the next tick's subtraction to mean anything.
            self.sim.pad_distance[slot][index] = pad.distance(position.to_array());

            // First pad wins. The cache above is still updated for every pad whose
            // turn it was, or a skipped one would keep a stale distance forever.
            if hit.is_none()
                && sweep.iter().any(|point| pad.contains(point.to_array()))
                && let Some(direction) = pad.direction()
            {
                hit = Some((index, Vec3::from_array(direction)));
            }
        }

        // Everything below is the *edge*, and it is deliberately outside the loop:
        // two pads overlapping on one tick is one entry, not two, matching the
        // original's single `craft+0x1d0` slot and its single `DAT_08b3435c` flag,
        // which `Zone_Update` (`0x0882f5cc`) consumes and clears once per tick.
        let entered = hit.map(|(index, _)| index);
        // A simulation mod's `scale-pad` (`crate::extensions`), the pad's race-wide factor
        // times the slot's: exactly 1.0 without a module. It scales the push alone; at 0
        // the pad gives no push, and the entry below pays no plume, sound, boost count
        // or Zone score (chosen). The edge and the cache are the pad's as ever, and the
        // module hears every entry.
        let factor = entered.map_or(1.0, |index| self.sim.extensions.pad_factor(slot, index));
        if entered != self.sim.pad_current[slot] {
            self.sim.pad_current[slot] = entered;
            if let Some(index) = entered {
                self.sim.extensions.pad_entered(slot, index, false);
            }
            if entered.is_some() && factor > 0.0 {
                // The lap table's third column: `Ship_ApplySpeedupPad` adds one to the
                // human craft's own per-lap counter on this same edge (`craft+0x900 +
                // lap * 0x10 + 0x94`, its only writer). See [`RunStats`].
                if self.sim.world.controllers[slot].is_human() {
                    #[cfg(feature = "view")]
                    let lap = self.sim.world.race[slot].lap;
                    #[cfg(feature = "view")]
                    if self.draws() {
                        self.view.run_stats.count_boost(lap);
                    }
                }
                // Zone mode only. `Ship_ApplySpeedupPad` raises its flag under
                // the mode selector `zone-mode.md` identifies.
                // **Whichever slot a person flies, not slot 0** - the same
                // single crossing while slot 0 is the only human, and the
                // score lands on the craft that actually crossed. Zone has no
                // AI field today, so this gate is about who is being scored
                // for rather than about excluding opponents.
                if self.sim.world.controllers[slot].is_human()
                    && self.sim.world.mode() == Mode::Zone
                {
                    self.sim.world.race[slot].score += oag_race::zone::SPEEDUP_PAD_SCORE;
                }
                // The visual, on the same edge and with the same **fixed**
                // duration the original uses. `ExhaustFlare_OnSpeedupPad`
                // (`0x08904f10`) is called from exactly here in
                // `Ship_ApplySpeedupPad`, inside its new-pad branch, and stores a
                // code literal - **not** `<SpeedupPads time>`. So the flare
                // outlives the force rather than expiring with it; see
                // `oag_fx::exhaust::BOOST_SECONDS`, which is where the reason
                // is written down.
                //
                // **The craft that crossed the pad, whichever it is.** Every
                // racer carries its own `Exhaust`, so an opponent's boost shows
                // as an opponent's plume rather than as the player's - the
                // original arms `ExhaustFlare_OnSpeedupPad` from inside the
                // per-craft update, with that craft's own flare.
                self.boost_plume(slot);
                #[cfg(feature = "view")]
                self.fire_zoom_boost(slot);
                // The sound is on the same edge and from the same branch:
                // `Ship_ApplySpeedupPad` calls `Sound_Play(..., "SPEEDUPPAD",
                // ...)` here, immediately beside `ExhaustFlare_OnSpeedupPad`.
                // The two used to differ only in that this port had no audio
                // system - see pads.md's own "Not determined" entry, which this
                // closes.
                //
                // **Whichever craft crossed, and that is the original's own
                // shape**: `ExhaustFlare_OnSpeedupPad` plays the cue off the
                // craft's emitter (`craft+0x50`) rather than off a global, so a
                // rival's pad arrives from where the rival is. This used to be
                // gated to slot 0 because nothing here could pan; the audio
                // layer decides how to place it now. See
                // `oag_sound::sfx::Placement::CraftUnlessPlayer`, which
                // carries the one hypothesis that split rides on.
                self.sim.cues.push(oag_gameplay::cue::CueEvent::new(
                    oag_gameplay::cue::Cue::SpeedupPad,
                    slot,
                ));
            }
        }

        if factor == 0.0 {
            return None;
        }
        // The direction's length is the push's scale (`Environment::pad_hit`); the jump
        // lift the physics adds to it is not scaled (chosen).
        hit.map(|(_, direction)| {
            if factor == 1.0 {
                direction
            } else {
                direction * factor
            }
        })
    }

    /// Crosses the ship against the track's weapon pads, granting a pickup.
    ///
    /// Reimplements `WeaponPads_TestCraft` (`0x0888727c`), which
    /// `docs/ghidra/functions/psp-pulse-usa/pads.md` reads as "the exact mirror
    /// of `Pads_TestCraft`" for the `world+0x10c` list - same swept test, same
    /// per-racer distance cache. So the broadphase and the sweep below are
    /// [`Self::test_speedup_pads`]' verbatim, and what differs is only what
    /// happens on a hit.
    ///
    /// # Three things happen here and one of them is partly recovered
    ///
    /// Worth separating at the call site rather than only in
    /// `docs/gameplay/pickups.md`, because they are easy to read as one ported
    /// branch:
    ///
    /// 1. **Stamping the pad** with `<WeaponPad refresh_time>` is the whole of
    ///    what the original's `WeaponPads_TestCraft` does on a hit, at confidence
    ///    90. It is done here on **any** hit, including one that grants nothing.
    /// 2. **Granting a pickup at all** is ours. No pickup-grant call site has
    ///    been found, so a crossing handing something over is this project's
    ///    reading of what a weapon pad is for. What is read is the gate around
    ///    it: `Ship_TestWeaponPads` (`0x0883efb4`) asks the pad only with an
    ///    **empty slot** and raises its grant request only with the Autopilot's
    ///    running bit clear (`0x0883f0c0`), so a craft flying the pickup is handed
    ///    nothing while the pad is still stamped. That half is
    ///    [`oag_title::weapons::Weapons::autopilot_pad_grants`].
    /// 3. **Only granting into an empty slot** agrees with that gate, with one
    ///    divergence: the original does not ask the pad at all while the slot is
    ///    full, so a full craft leaves the pad unstamped there, and here it
    ///    stamps. Left as it was, which costs a full craft the pad's cooldown
    ///    and cannot hand out two pickups from one crossing.
    ///
    /// Called once a tick with the position the tick *starts* at, like the speed
    /// pad's, and **after** it, so both spend the same `moved` and neither can
    /// see a position the other has already advanced past.
    pub(super) fn test_weapon_pads(
        &mut self,
        slot: usize,
        position: Vec3,
        moved: f32,
        sweep: &[Vec3],
    ) {
        if self.sim.weapon_pads.is_empty() {
            return;
        }

        // `WeaponPad_UpdateRefreshTimer` (`0x0892c034`): every pad counts its own
        // stamp down by `dt`, floored at zero, whether or not anything is near
        // it. Outside the hit loop because the original runs it as the class's
        // `update` method on every pad, every tick.
        // **Once a tick, not once a craft.** The timer belongs to the pad, so
        // the first craft through it is the one that runs the countdown and the
        // other seven read what it left. The primary slot is stepped first,
        // every tick: the player step runs before the opponents', lowest
        // person first, and the primary is always a person - slot 0 unless
        // slot 0 is handed to the AI.
        if slot == self.sim.world.primary_slot() {
            for left in &mut self.sim.weapon_pad_refresh_left {
                *left = (*left - self.sim.dt).max(0.0);
            }
        }

        let mut hit = None;
        for (index, pad) in self.sim.weapon_pads.iter().enumerate() {
            self.sim.weapon_pad_distance[slot][index] -= moved;
            if self.sim.weapon_pad_distance[slot][index] > 0.0 {
                continue;
            }
            self.sim.weapon_pad_distance[slot][index] = pad.distance(position.to_array());

            if hit.is_none() && sweep.iter().any(|point| pad.contains(point.to_array())) {
                hit = Some(index);
            }
        }

        // The edge, outside the loop for the same reason the speed pad's is: two
        // pads overlapping on one tick is one entry.
        if hit == self.sim.weapon_pad_current[slot] {
            return;
        }
        self.sim.weapon_pad_current[slot] = hit;
        let Some(index) = hit else {
            return;
        };
        // A simulation mod hears every entry, cooling down or not.
        self.sim.extensions.pad_entered(slot, index, true);

        // Point 1 above. Unconditional, and before the grant, so a pad that
        // hands nothing over is still spent.
        if self.sim.weapon_pad_refresh_left[index] > 0.0 {
            // Still cooling down. The stamp is not refreshed - re-stamping would
            // make a craft parked on a pad hold it inert forever, and the
            // original cannot reach this branch at all: its timer is what
            // `Pad_ContainsPoint` is asked about before the hit is reported.
            return;
        }
        self.sim.weapon_pad_refresh_left[index] = self.sim.weapon_pad_refresh;

        // **Nothing is granted to a craft whose Autopilot runs**, on any slot, an
        // opponent on this project's Autopilot included: `Ship_TestWeaponPads`
        // (`0x0883efb4`) raises the grant request only with `craft+0x1b8 & 0x800`
        // clear (`0x0883f0c0`), after `WeaponPads_TestCraft` has stamped the pad,
        // so the stamp above stays. Read statically on Pulse, confidence 80 - see
        // [`oag_title::weapons::Weapons::autopilot_pad_grants`].
        if self.sim.world.ships[slot].autopilot_timer > 0.0 && !self.sim.autopilot_pad_grants {
            return;
        }

        // Points 2 and 3.
        if !self.sim.world.ships[slot].pickup.is_empty() {
            return;
        }
        let Some(weapons) = self.sim.weapons.as_ref() else {
            return;
        };
        let Some(table) = oag_weapons::pickup::table_for(weapons, &self.sim.class) else {
            return;
        };
        // **The `human` column for a person's craft and the `ai` column for the rest**, and
        // the player's is blended with `front`/`back` by their place - which is
        // the original's own arrangement, recovered from `WeaponPickup_Grant`
        // (`0x08861d20`). The AI's column is spent flat, so an opponent's odds do
        // not depend on where it is; the rubber-banding is aimed at the player.
        // See `docs/gameplay/pickups.md`.
        // **Whichever slot a person flies**: their own place, their own column.
        let who = if self.sim.world.person_flown(slot) {
            oag_weapons::pickup::Driver::Human {
                place: self.places()[slot],
                field: self.sim.world.ship_count,
            }
        } else {
            oag_weapons::pickup::Driver::Ai
        };
        let last = self.sim.world.ships[slot].pickup.last;
        // `None` for every non-2048 race and for a 2048 event whose weapon
        // set decodes to nothing recognised - the same "empty is no
        // restriction" reading `Setup::allowed_weapons`'s own doc comment
        // gives. AI slots are gated the same as the player's: which weapons
        // an event's own `WeaponSetDefinition` allows is stated about the
        // pads, not about who crosses them - **chosen, not measured**, since
        // no consumer of this mask has been found in `eboot.elf` to read
        // otherwise.
        let allowed =
            (!self.sim.allowed_weapons.is_empty()).then_some(self.sim.allowed_weapons.as_slice());
        let drawn = oag_weapons::pickup::draw(&mut self.sim.world.rng, table, who, last, allowed);
        if let Some(weapon) = drawn {
            let pickup = &mut self.sim.world.ships[slot].pickup;
            pickup.grant(weapon);
            // A simulation mod's `pickup-roll`: this grant becomes the mod's own pickup,
            // with `last` still the drawn weapon, so the title's repeat rule is unchanged.
            if self.sim.extensions.rolls(weapon) {
                pickup.weapon = Some(oag_tables::weapons::Weapon::Extension);
            }
        }
    }
}
