//! The pending collision-impulse consumer, split out of `wall.rs` under the 1,000-line cap.
//! Not about wall contact: `Ship_ApplyCollisionImpulse` consumes *any* posted impulse
//! (weapon or rival). It lives under `wall` because [`super::STUN_PER_CONTACT`], the timer
//! it arms, does.

use oag_core::math::Vec3;

use crate::ship::ShipState;

/// Seconds of [`ShipState::stun_timer`] a **posted** collision impulse adds: `craft+0x290 +=
/// 0.5` in `Ship_ApplyCollisionImpulse` (`0x0883f274`). **Added, not assigned**, so repeated
/// hits accumulate. Confidence 85; `docs/ghidra/functions/psp-pulse-usa/engine.md`.
///
/// # Nothing in this crate arms it, and that is a correction
///
/// [`super::resolve`] once armed it on the first frame of every inward contact. Wrong, on
/// three legs:
///
/// - `Ship_ApplyCollisionImpulse` is gated on a **pending impulse vector** at
///   `entity->0x4c + 0x110` being non-zero and zeroes it on the way out. Nothing on the
///   track-contact path writes it: `Body_ResolveContact` applies its impulse through
///   `Body_ApplyImpulseAtPoint` and records into the observation ring, whose only consumer
///   `FUN_088418e0` computes camera and audio amplitudes from the literals `0.05`, `0.7`,
///   `0.0125` and `1.0` and writes no impulse.
/// - `data/traces/talons-junction-time-trial-lap.csv` (3,146 ticks of a Time Trial, not a
///   complete lap, `docs/tools/oag-trace.md`; `4.8 %` in wall contact) has `stun_timer`
///   **`0.0` on every tick**.
/// - `data/traces/talons-junction-standing-start.csv` (300 ticks, 230 of one wall scrape)
///   has it **`0.0` on every tick** too.
///
/// So the timer belongs to being hit by a rival or weapon, not touching the track. The gate
/// it feeds is real ([`ShipState::stun_timer`] cuts thrust and lateral grip); only the
/// arming site is absent, as for [`ShipState::slowdown_timer`].
///
/// **What the mistake cost:** once the hull had ten sample points it fired on `2,045` of a
/// `3,146`-tick open-loop run, `65 %` of the race with no engine, and the craft crawled
/// `742` units. An earlier one armed *per frame* rather than per impact (`0.5 s` against a
/// `dt` decay is 30:1 at 60 Hz; "at some point the racer completely stopped accelerating").
///
/// # A fired Shield suppresses both halves
///
/// `Ship_ApplyCollisionImpulse` wraps the projection *and* the `+= 0.5` in `if
/// ((craft->0x1b8 & 0x10) == 0)`; bit `0x10` is the running Shield pickup (2026-08-19,
/// `docs/ghidra/functions/psp-pulse-usa/shield-pickup.md`). The posted vector is zeroed
/// either way, so a shielded craft does not bank the knockback. **Ported in
/// [`apply_pending_impulse`]** ([`ShipState::shield_pickup_timer`]). The shield touches only
/// this impulse: `Body_ResolveContact` applies the track impulse unconditionally, so a
/// shielded craft bounces off a wall as usual and pays no energy
/// ([`crate::damage::apply_contact`]).
///
/// # The writers (2026-08-19)
///
/// A live write breakpoint during a driven Single Race found two, both in the weapon-code
/// region. `Weapon_PostBlastImpulse_q` (`0x0886794c`) **accumulates** a distance-falloff
/// impulse off a per-weapon stats table (ported as [`post_blast_impulse`]; a rocket, mine or
/// missile explosion is the unconfirmed guess for its caller, `Mine_SweepCraftTrigger`). A
/// second, unnamed writer loops over what reads like a short entity list doing the same
/// accumulate (no port). All callers are found and none read, so nothing calls the ported
/// writer: this crate has no weapon trigger, no `radius`/`power` table and no settled
/// `source`/`target` selection. `Ship_ApplyCollisionImpulse` overwrites the pending vector's
/// storage with its forward-axis projection before applying it through
/// `Body_ApplyImpulseAtPoint` (`0x0884d64c`), the math [`super::resolve`] reproduces. (Finding
/// that call target needed a live memory read plus the image base: Ghidra's *static*
/// disassembly of a `jal` operand here is the unrelocated `R_MIPS_26` field, a documented
/// trap that also silences `get_xrefs_to`; the fix for both is image-relative
/// `search_instructions`.) See `contact-response.md`.
pub const STUN_PER_CONTACT: f32 = 0.5;

/// Drains [`ShipState::pending_impulse`] as `Ship_ApplyCollisionImpulse` (`0x0883f274`) does.
/// Called once a tick for every ship, whatever posted the impulse (or nothing). Read at
/// instruction level 2026-08-19; derivation and confidence in `contact-response.md`.
///
/// 1. **All-zero is a no-op**: the original tests `x`, `y`, `z` separately (`!= Vec3::ZERO`).
/// 2. **A running Shield suppresses the rest but still consumes the vector**, zeroed at the
///    bottom either way.
/// 3. **The projection.** `magnitude = pending.length()`; the original keeps only *which way
///    the vector pointed relative to the ship's forward axis* (`sign = dot(forward, pending) <
///    0.0 ? -1 : 1`) and discards its own direction: `impulse = forward * (sign * magnitude)`.
/// 4. **Applied at the ship's own position.** `Body_ApplyImpulseAtPoint`'s `point` is read
///    from the same scene node as the forward axis, so the lever arm is zero and the angular
///    term vanishes: only the linear half is applied (`linear_velocity += impulse / mass`),
///    a straight push along the ship's forward axis.
/// 5. **The timer arm**: [`STUN_PER_CONTACT`] onto [`ShipState::stun_timer`], skipped under
///    the Shield as 3-4.
///
/// Nothing writes [`ShipState::pending_impulse`] yet, so this is a correct no-op today: the
/// consumer half of the stun path, testable alone by setting the field.
pub fn apply_pending_impulse(state: &mut ShipState) {
    if state.pending_impulse == Vec3::ZERO {
        return;
    }

    if state.shield_pickup_timer <= 0.0 {
        let magnitude = state.pending_impulse.length();
        let forward = state.body.forward();
        let sign = if forward.dot(state.pending_impulse) < 0.0 {
            -1.0
        } else {
            1.0
        };
        let impulse = forward * (sign * magnitude);

        if state.body.mass > 0.0 {
            state.body.linear_velocity += impulse / state.body.mass;
        }

        state.stun_timer += STUN_PER_CONTACT;
    }

    state.pending_impulse = Vec3::ZERO;
}

/// A weapon blast's distance-falloff impulse, posted onto a target's
/// [`ShipState::pending_impulse`] for [`apply_pending_impulse`] to consume.
///
/// `Weapon_PostBlastImpulse_q` (`0x0886794c`), read at instruction level 2026-08-19
/// (`contact-response.md`). The original reads `target_position` from `target+0x50`, which
/// a live breakpoint check confirmed *is* the target's `RigidBody` (by pointer equality
/// against craft enumerated off `Ship_ApplyCollisionImpulse`) and held a live, track-scale
/// value matching [`crate::ship::Body::position`]. (That check also found a tension with
/// `rigid-body.md`'s reading of `body+0x40..0x70` as inertia storage; see its "Open
/// contradiction" note. It does not affect this substitution.) In the original's order:
///
/// 1. `d = |target_position - source_position|`.
/// 2. `falloff = 1.0 - d / radius`, **not clamped**: a target beyond `radius` gets a negative
///    falloff, which pulls it *toward* the source. The original does not guard this and
///    neither does this port; whether its caller always gates by distance is unread.
/// 3. `direction = normalize(target_position - source_position)`, zero-guarded
///    ([`Vec3::normalize_or_zero`]) as the original substitutes `MaxFloat` before its
///    reciprocal: a target exactly at the source gets no push, not a `NaN` one.
/// 4. `pending_impulse += direction * (falloff * power)`, **accumulated**, so two blasts on
///    one ship in one tick stack.
///
/// # Not ported
///
/// The original also adds `stats->0xe8` to an accumulator at `target+0x120` and `stats->0xfc`
/// to one at `target+0x130`, writes a literal `4` to `target+0x138`, records the source's
/// weapon-stat index at `target+0x13c`, and conditionally sets a byte at `target+0x124` off a
/// three-state field of the weapon's stats. None feeds anything this crate reads, and none is
/// understood well enough to name a Rust field; porting them would be inventing what they do
/// (`CLAUDE.md`, "Never invent what the assets already author").
///
/// # Nothing calls this yet
///
/// The original's caller `Mine_SweepCraftTrigger` sweeps *every* craft as a candidate target
/// for a source craft index, box-then-sphere range-checks each against a per-weapon-type
/// radius, and calls this once per qualifier. Its caller `MinePool_Update` decrements a
/// per-craft countdown each tick and fires the sweep on expiry, a shape fitting a proximity
/// mine better than a rocket or missile (unconfirmed). Open: what arms that timer, what feeds
/// `MinePool_Update`, and the `radius`/`power` table, unparsed (`contact-response.md`). So
/// this is a correct, tested pure function with no wiring.
pub fn post_blast_impulse(target: &mut ShipState, source_position: Vec3, radius: f32, power: f32) {
    let diff = target.body.position - source_position;
    let distance = diff.length();
    let falloff = 1.0 - distance / radius;
    let direction = diff.normalize_or_zero();

    target.pending_impulse += direction * (falloff * power);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ship::Body;

    /// A ship at the origin, at rest, at the identity orientation - so
    /// `Body::forward` is `Vec3::NEG_Z`.
    fn ship() -> ShipState {
        ShipState {
            body: Body {
                mass: 1.0,
                ..Body::default()
            },
            ..ShipState::default()
        }
    }

    /// A default ship has nothing pending, and this must leave it exactly
    /// alone - the shape every probe script relies on, since nothing yet
    /// produces a pending impulse for a real race to exercise.
    #[test]
    fn no_pending_impulse_is_a_true_no_op() {
        let mut state = ShipState::default();
        let before = state;

        apply_pending_impulse(&mut state);

        assert_eq!(state, before);
    }

    /// The headline behaviour: a pending impulse becomes a push along the
    /// ship's own forward axis, sized by the pending vector's magnitude
    /// alone, arms the stun and is consumed.
    #[test]
    fn a_pending_impulse_pushes_along_forward_and_arms_the_stun() {
        let mut state = ship();
        state.pending_impulse = Vec3::new(0.0, 0.0, -5.0);

        apply_pending_impulse(&mut state);

        assert_eq!(state.body.linear_velocity, Vec3::new(0.0, 0.0, -5.0));
        assert_eq!(state.stun_timer, STUN_PER_CONTACT);
        assert_eq!(state.pending_impulse, Vec3::ZERO);
    }

    /// The original discards the pending vector's own direction and keeps
    /// only its magnitude and which side of the ship's forward axis it fell
    /// on - so a pending impulse with a sideways component still produces a
    /// push purely along forward, never sideways.
    #[test]
    fn a_pending_impulse_off_axis_still_only_pushes_along_forward() {
        let mut state = ship();
        // Perpendicular to forward (`NEG_Z`), magnitude 5.
        state.pending_impulse = Vec3::new(3.0, 4.0, 0.0);

        apply_pending_impulse(&mut state);

        // A dot product of exactly zero takes the original's `>= 0.0`
        // branch, the same sign as the aligned case above - so this lands on
        // the *same* push, `-Z`, as
        // `a_pending_impulse_pushes_along_forward_and_arms_the_stun`, even
        // though the pending vector here points nowhere near forward at all.
        assert_eq!(state.body.linear_velocity, Vec3::new(0.0, 0.0, -5.0));
        assert_eq!(state.stun_timer, STUN_PER_CONTACT);
    }

    /// A pending impulse pointing the other way along forward flips which
    /// way the push goes, not just its size - the sign this crate reads off
    /// `dot(forward, pending) < 0.0`.
    #[test]
    fn a_pending_impulse_against_forward_pushes_the_other_way() {
        let mut state = ship();
        state.pending_impulse = Vec3::new(0.0, 0.0, 5.0);

        apply_pending_impulse(&mut state);

        assert_eq!(state.body.linear_velocity, Vec3::new(0.0, 0.0, 5.0));
    }

    /// A running Shield suppresses the push and the stun arm alike, but the
    /// pending vector is still consumed - it does not carry over to bank the
    /// knockback for later.
    #[test]
    fn a_running_shield_suppresses_the_impulse_but_still_consumes_it() {
        let mut state = ship();
        state.pending_impulse = Vec3::new(0.0, 0.0, -5.0);
        state.shield_pickup_timer = 1.0;

        apply_pending_impulse(&mut state);

        assert_eq!(state.body.linear_velocity, Vec3::ZERO);
        assert_eq!(state.stun_timer, 0.0);
        assert_eq!(state.pending_impulse, Vec3::ZERO);
    }

    /// A massless body cannot be pushed, and must not divide by zero trying -
    /// the same `mass > 0.0` guard `resolve_contact`'s own `inverse_mass` uses.
    #[test]
    fn a_massless_body_is_not_pushed_and_does_not_panic() {
        let mut state = ship();
        state.body.mass = 0.0;
        state.pending_impulse = Vec3::new(0.0, 0.0, -5.0);

        apply_pending_impulse(&mut state);

        assert_eq!(state.body.linear_velocity, Vec3::ZERO);
        // Both other effects are unconditional on mass, only the push is
        // guarded.
        assert_eq!(state.stun_timer, STUN_PER_CONTACT);
        assert_eq!(state.pending_impulse, Vec3::ZERO);
    }

    /// Well inside the radius, the impulse points away from the source,
    /// scaled by both the linear falloff and the weapon's own power.
    #[test]
    fn well_inside_the_radius_the_impulse_pushes_away_from_the_source() {
        let mut target = ship();
        target.body.position = Vec3::new(4.0, 0.0, 0.0);

        post_blast_impulse(&mut target, Vec3::ZERO, 10.0, 20.0);

        // distance 4, radius 10 -> falloff 0.6; power 20 -> magnitude 12,
        // directed along +x, away from the source at the origin.
        assert_eq!(target.pending_impulse, Vec3::new(12.0, 0.0, 0.0));
    }

    /// At the source's exact position the direction is undefined, and the
    /// original substitutes `MaxFloat` before its reciprocal rather than
    /// dividing by zero - `normalize_or_zero` is the same guard. Falloff is
    /// `1.0` here, but a zero direction still posts a zero impulse.
    #[test]
    fn at_the_source_s_own_position_the_impulse_is_zero_not_nan() {
        let mut target = ship();
        target.body.position = Vec3::ZERO;

        post_blast_impulse(&mut target, Vec3::ZERO, 10.0, 20.0);

        assert_eq!(target.pending_impulse, Vec3::ZERO);
    }

    /// Exactly at the radius, `falloff` is zero and the impulse is zero
    /// regardless of direction or power.
    #[test]
    fn at_the_edge_of_the_radius_the_falloff_is_zero() {
        let mut target = ship();
        target.body.position = Vec3::new(10.0, 0.0, 0.0);

        post_blast_impulse(&mut target, Vec3::ZERO, 10.0, 20.0);

        assert_eq!(target.pending_impulse, Vec3::ZERO);
    }

    /// Beyond the radius `falloff` goes negative - not clamped, the same as
    /// the original - which flips the impulse to pull the target toward the
    /// source rather than push it away. Faithful, not fixed: see this
    /// function's own doc for why the original leaves it this way.
    #[test]
    fn beyond_the_radius_the_falloff_goes_negative_and_pulls_inward() {
        let mut target = ship();
        target.body.position = Vec3::new(20.0, 0.0, 0.0);

        post_blast_impulse(&mut target, Vec3::ZERO, 10.0, 20.0);

        // distance 20, radius 10 -> falloff -1.0; power 20 -> magnitude -20
        // along +x, i.e. 20 units toward the source at the origin.
        assert_eq!(target.pending_impulse, Vec3::new(-20.0, 0.0, 0.0));
    }

    /// Two blasts in the same tick accumulate rather than the second
    /// overwriting the first - `vadd.q`, not a plain store, in the original.
    #[test]
    fn two_blasts_in_the_same_tick_accumulate() {
        let mut target = ship();
        target.body.position = Vec3::new(4.0, 0.0, 0.0);

        post_blast_impulse(&mut target, Vec3::ZERO, 10.0, 20.0);
        post_blast_impulse(&mut target, Vec3::new(4.0, 0.0, 4.0), 10.0, 20.0);

        // The first posts (12, 0, 0) as above. The second: source at
        // (4, 0, 4), target still at (4, 0, 0) -> diff (0, 0, -4), distance
        // 4, falloff 0.6, magnitude 12, direction -z -> (0, 0, -12).
        // Accumulated, not overwritten: (12, 0, -12).
        assert_eq!(target.pending_impulse, Vec3::new(12.0, 0.0, -12.0));
    }
}
