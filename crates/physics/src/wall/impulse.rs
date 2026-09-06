//! The pending collision-impulse consumer, split out of `wall.rs` under the
//! 1,000-line cap; see `scripts/check-file-size.py`. Conceptually this is not
//! about wall contact at all - `Ship_ApplyCollisionImpulse` is the consumer
//! for *any* posted impulse, weapon or rival - it lives under `wall` because
//! [`super::STUN_PER_CONTACT`], the timer it arms, already does.

use oag_core::math::Vec3;

use crate::ship::ShipState;

/// Seconds of [`ShipState::stun_timer`] a **posted** collision impulse adds.
///
/// `craft+0x290 += 0.5` in `Ship_ApplyCollisionImpulse` (`0x0883f274`).
/// **Added, not assigned**, so a ship that keeps being hit accumulates stun
/// rather than holding a flat half second. Confidence 85; see
/// `docs/ghidra/functions/psp-pulse-usa/engine.md`.
///
/// # Nothing in this crate arms it, and that is a correction
///
/// [`super::resolve`] used to, on the first frame of every inward contact. It
/// was wrong, and the evidence is three-legged:
///
/// - `Ship_ApplyCollisionImpulse` is gated on a **pending impulse vector** at
///   `entity->0x4c + 0x110` being non-zero, and it zeroes that vector on the way
///   out. Nothing on the track-contact path writes it: `Body_ResolveContact`
///   applies its impulse through `Body_ApplyImpulseAtPoint` and records the
///   contact into the observation ring, and `FUN_088418e0` - the only consumer of
///   that ring - computes camera and audio amplitudes from literals `0.05`,
///   `0.7`, `0.0125` and `1.0` and writes no impulse anywhere.
/// - `data/traces/talons-junction-time-trial-lap.csv` is 3,146 ticks of a Time
///   Trial capture (not a complete lap despite the name, it stalls and
///   reverses somewhere in it; see `docs/tools/oag-trace.md`'s own
///   correction), `4.8 %` of it in wall contact, and `stun_timer` is
///   **`0.0` on every tick**.
/// - `data/traces/talons-junction-standing-start.csv` is 300 more ticks, 230 of
///   them one continuous wall scrape, and `stun_timer` is **`0.0` on every tick**
///   there too.
///
/// So the timer belongs to being hit by something - a rival, a weapon - and not
/// to touching the track. The gate it feeds is real and stays
/// ([`ShipState::stun_timer`] still cuts thrust and lateral grip); the arming
/// site is simply not in this crate yet, the same way nothing arms
/// [`ShipState::slowdown_timer`].
///
/// **What that mistake cost, so nobody re-derives it**: with the old
/// single-deepest-probe contact model it fired rarely enough to look harmless.
/// The moment the hull grew its ten real sample points it fired on `2,045` of a
/// `3,146`-tick open-loop run - `65 %` of the race with no engine at all - and
/// the craft crawled `742` units instead of a lap. Two earlier bugs in the same
/// place are recorded history: arming *per frame* rather than per impact
/// (`0.5 s` armed against a `dt` decay is 30:1 at 60 Hz, reported from play as
/// "at some point the racer completely stopped accelerating"), and this one.
///
/// The original's impulse path also projects the posted impulse onto the ship's
/// forward axis before applying it, and scales the ship's own forward axis by the
/// impulse magnitude rather than using the impulse direction - see
/// [`apply_pending_impulse`], which is that whole path, not just this constant.
///
/// # A fired Shield suppresses both halves, and that is the other half of it
///
/// `Ship_ApplyCollisionImpulse` wraps the projection *and* the `+= 0.5` in
/// `if ((craft->0x1b8 & 0x10) == 0)`, and bit `0x10` is the running Shield
/// pickup - read 2026-08-19, see
/// `docs/ghidra/functions/psp-pulse-usa/shield-pickup.md`. The posted vector is
/// zeroed on the way out either way, so a shielded craft does not bank the
/// knockback and take it late.
///
/// **Ported in [`apply_pending_impulse`]**, gate and all - the day something
/// posts [`ShipState::pending_impulse`], the shield gate is already there
/// rather than needing to be discovered afterwards as "the shield does not
/// seem to do anything to a rival hit". [`ShipState::shield_pickup_timer`] is
/// the flag.
///
/// The writer is found, 2026-08-19, by a live write breakpoint on the field
/// during a driven Single Race: two of them, both in the weapon-code region.
/// `Weapon_PostBlastImpulse_q` (`0x0886794c`) computes a distance falloff off
/// a per-weapon stats table and **accumulates** an impulse into the target's
/// pending vector - a rocket, mine or missile explosion is the unconfirmed
/// guess for its caller (`FUN_08867b50`, found but unread). A second, unnamed
/// writer loops over what reads like a short entity list doing the same
/// accumulate, also with a found-but-unread caller. `Ship_ApplyCollisionImpulse`
/// itself is now read at instruction level too: the forward-axis projection
/// overwrites the pending vector's storage in place before applying it
/// through the same `Body_ApplyImpulseAtPoint` (`0x0884d64c`) [`super::resolve`]
/// already reproduces the math of - finding that call target needed a live
/// memory read plus the image base, since Ghidra's *static* disassembly of a
/// `jal` operand in this database is the unrelocated `R_MIPS_26` field, not
/// the real address (an already-documented trap that also silences
/// `get_xrefs_to`; the fix for both is the same image-relative
/// `search_instructions`). `Ship_ApplyCollisionImpulse` itself has a Rust
/// port now, [`apply_pending_impulse`], and so does one of the two writers -
/// `Weapon_PostBlastImpulse_q` is [`post_blast_impulse`]. The second, unnamed
/// writer does not. All four callers - both writers' and the consumer's - are
/// found, none read, which is why even the ported writer has nothing calling
/// it yet: this crate has no weapon trigger, no `radius`/`power` table, and no
/// settled `source`/`target` selection to wire it to. See
/// `docs/ghidra/functions/psp-pulse-usa/contact-response.md`.
///
/// It also fixes the *scope* of the shield, which is easy to over-read: this is
/// the only impulse it touches. `Body_ResolveContact` applies the track impulse
/// unconditionally, so a shielded craft bounces off a wall exactly as it
/// otherwise would - it just pays no energy for it, per
/// [`crate::damage::apply_contact`].
pub const STUN_PER_CONTACT: f32 = 0.5;

/// Drains [`ShipState::pending_impulse`], the way `Ship_ApplyCollisionImpulse`
/// (`0x0883f274`) does. Called once a tick, for every ship, whatever posted the
/// pending impulse (or nothing).
///
/// Read at instruction level 2026-08-19; full derivation and confidence in
/// `docs/ghidra/functions/psp-pulse-usa/contact-response.md`. Five steps:
///
/// 1. **All-zero is a no-op.** The original tests `x`, `y`, `z` separately and
///    returns without touching anything else if all three are `0.0`; this is
///    `!= Vec3::ZERO`, which is the same test.
/// 2. **A running Shield suppresses the rest, but still consumes the vector.**
///    Both halves below are skipped; [`ShipState::pending_impulse`] is zeroed
///    regardless, at the bottom of this function either way.
/// 3. **The projection.** `magnitude = pending.length()`; the original reads a
///    forward axis off the ship's own scene node and keeps only *which way it
///    pointed relative to that axis* - `sign = dot(forward, pending) < 0.0 ? -1
///    : 1` - discarding the pending vector's own direction entirely.
///    `impulse = forward * (sign * magnitude)`.
/// 4. **The impulse is applied at the ship's own position.** The original calls
///    `Body_ApplyImpulseAtPoint(body, point, impulse)` with `point` read from
///    the same scene node the forward axis came from - which is to say, the
///    ship's own current position, not a contact point. `Body_ApplyImpulseAtPoint`
///    computes a lever arm from `point - body.position`; with `point` and
///    `body.position` the same value by construction, that lever is zero and
///    the angular term vanishes, so this crate applies the linear half only
///    (`body.linear_velocity += impulse / mass`) rather than routing through
///    `resolve_contact`'s general point-impulse math (`super::resolve_contact`)
///    for a term that would always be zero. Nothing here rotates a stunned
///    ship - only a straight push along its own forward axis, in whichever
///    sense the hit came from.
/// 5. **The timer arm.** [`STUN_PER_CONTACT`] added to [`ShipState::stun_timer`],
///    skipped under the Shield same as step 3-4.
///
/// # What still has to exist before this does anything
///
/// Nothing writes [`ShipState::pending_impulse`] yet - see its own doc. This
/// function is therefore always a no-op today, correctly: it is the *consumer*
/// half of the stun path, provable and testable on its own by setting the field
/// directly, independent of whichever producer lands first.
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
/// `Weapon_PostBlastImpulse_q` (`0x0886794c`), read at instruction level
/// 2026-08-19; full derivation and the parts left unported are in
/// `docs/ghidra/functions/psp-pulse-usa/contact-response.md`. The original
/// reads `target_position` from `target+0x50` on a struct that a live check
/// at a breakpoint confirmed *is* the target's `RigidBody` - the same one
/// [`ShipState::body`] ports - by direct pointer equality against craft
/// enumerated off `Ship_ApplyCollisionImpulse`; `target+0x50` itself held a
/// live, track-scale, per-craft value at the hit, matching what this crate
/// calls [`crate::ship::Body::position`]. (That live check also turned up an
/// unresolved tension with `rigid-body.md`'s own reading of `body+0x40..0x70`
/// as inertia-tensor storage - see that page's "Open contradiction" note; it
/// does not affect this substitution, which only needed `target+0x50`'s
/// identity settled, not `+0x40..0x70`'s.) The arithmetic this function
/// ports, in the original's order:
///
/// 1. `d = |target_position - source_position|`.
/// 2. `falloff = 1.0 - d / radius` - **not clamped**, faithfully: a target
///    beyond `radius` gets a negative `falloff`, which flips the impulse to
///    pull the target *toward* the source rather than push it away. The
///    original does not guard this and neither does this port; whether the
///    original's caller always gates by distance first is unread.
/// 3. `direction = normalize(target_position - source_position)` - the same
///    difference vector step 1 already computed, zero-guarded
///    ([`Vec3::normalize_or_zero`]) the way the original substitutes
///    `MaxFloat` before its reciprocal: a target exactly at the source gets no
///    push, not a `NaN` one.
/// 4. `pending_impulse += direction * (falloff * power)` - **accumulated**,
///    not assigned, so two blasts landing on the same ship in the same tick
///    stack rather than the later one overwriting the earlier.
///
/// # What this function deliberately does not port
///
/// The original also: adds `stats->0xe8` to an unrelated accumulator at
/// `target+0x120`; adds `stats->0xfc` to a second unrelated one at
/// `target+0x130`; writes a literal `4` to `target+0x138`; records the
/// source's weapon-stat index onto `target+0x13c`; and conditionally sets a
/// byte flag at `target+0x124` off a three-state field on the weapon's own
/// stats block. None of these feed [`apply_pending_impulse`] or anything else
/// this crate reads yet - each is measured in the original, none is
/// understood well enough to give a Rust field a meaning, so none is ported.
/// Porting any of them without a settled reading would be inventing what they
/// do, which this project does not do - see `CLAUDE.md`, "Never invent what
/// the assets already author."
///
/// # Nothing calls this yet, and now the shape of what would is known
///
/// The original's caller, `FUN_08867b50`, is read in full: given a source
/// craft index, it sweeps *every* craft as a candidate target, box-then-
/// sphere range-checks each one against a per-weapon-type radius, and calls
/// this function once per candidate that qualifies - which is where
/// `target`/`source` and the range check this crate would need to reproduce
/// actually come from. `FUN_08867b50`'s own caller, `FUN_08867370`, is read
/// too: it decrements a per-craft countdown timer every tick and fires the
/// sweep for whichever craft's timer expires - a shape that fits a proximity
/// mine better than a rocket or missile, though unconfirmed. Still open:
/// what arms that timer, what feeds `FUN_08867370` its own per-tick call, and
/// the `radius`/`power` table itself, which this crate has not parsed. See
/// `docs/ghidra/functions/psp-pulse-usa/contact-response.md` for the full
/// two-hop read. So this is a correct, tested, pure function with no wiring
/// into any weapon or explosion yet, the same shape [`apply_pending_impulse`]
/// was before this: provable on directly-supplied inputs, independent of
/// whichever caller lands first.
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
