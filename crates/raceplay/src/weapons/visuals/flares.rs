//! The free functions behind [`Race::advance_projectile_flares`] and
//! [`Race::ignite_missile_bounces`]: which effect a projectile rides, how
//! one riding instance attaches, follows and detaches, the Missile's two
//! orbiting anchors, the Plasma's charge-scaled glow, and the bounce burst's
//! gate and name.
//!
//! Split out of `visuals.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py` on 2026-09-16, when the Plasma's charge glow
//! and its cockpit halving took that file to 1,008; a move, with no behaviour
//! change. The seam is the one the file already had - `impl Race` above,
//! free functions below - and every name keeps its `pub(crate)`
//! visibility so `weapons.rs`'s test re-exports resolve unchanged.

use super::super::*;

/// Which flare effect, if any, a projectile of this kind rides - the
/// Rocket's [`Trigger::RocketFlare`], the Missile's [`Trigger::MissileFlare`],
/// or `None` for everything else, including a Mine or a Bomb.
///
/// Split out of [`Race::advance_projectile_flares`] so the gate is testable
/// without a loaded `psys` library: the bug this replaced (`kind.is_none()`,
/// true for any live projectile) could only be seen with a real effect
/// asset attached, which a headless unit test has no disc to load. See that
/// method's doc comment for the reading.
#[must_use]
pub(crate) fn flare_effect_for(kind: Option<oag_tables::weapons::Weapon>) -> Option<Trigger> {
    match kind? {
        oag_tables::weapons::Weapon::Rocket => Some(Trigger::RocketFlare),
        oag_tables::weapons::Weapon::Missile => Some(Trigger::MissileFlare),
        oag_tables::weapons::Weapon::Plasma => Some(Trigger::PlasmaFlare),
        oag_tables::weapons::Weapon::Shuriken => Some(Trigger::ShurikenFlare),
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
/// slots - [`RaceView::projectile_flare`] and [`RaceView::projectile_flare_orbit`] -
/// live at once without a second borrow of `self`.
pub(super) fn advance_one_flare(
    stage: &mut psys::Stage,
    handles: &EffectHandles,
    trigger: Option<Trigger>,
    at: Vec3,
    scale: f32,
    playing: &mut Option<psys::Playing>,
) {
    match (trigger, *playing) {
        // Gone, or a weapon with no flare of its own: release the instance
        // (templates at once, the emitters' own particles fade out) rather
        // than riding a weapon that never authored this effect.
        (None, Some(instance)) => {
            stage.release(instance);
            *playing = None;
        }
        (None, None) => {}
        (Some(_), Some(instance)) => {
            stage.follow(instance, at);
            stage.rescale(instance, scale);
        }
        // Freshly in the air. `attach` returning `None` means the stage is
        // full of flares already, and this projectile simply flies without
        // one rather than evicting someone else's.
        (Some(trigger), None) => {
            if let Some(effect) = handles.get(trigger).cloned() {
                *playing = stage.attach(&effect, at, scale);
            }
        }
    }
}

/// The riding flare's own scale for one tick, by the projectile's kind and
/// charge state - `1.0` (the stage's own neutral value) for everything that
/// is not a Plasma at all.
///
/// **Ports `Plasma_UpdateCharge`'s `(1.0 - remaining) * 0.75`** - see
/// [`Trigger::PlasmaFlare`]'s doc comment and
/// `docs/ghidra/functions/psp-pulse-usa/plasma.md`'s "The charge is real,
/// and it is not `charge_time`" section. `remaining` there is a countdown in
/// *seconds*, `1.0` at press and `0.0` at release, exactly like this
/// engine's own `projectile.charge`; the fraction is written as
/// `(CHARGE_SECONDS - charge) / CHARGE_SECONDS` rather than the original's
/// bare `1.0 - charge` so this stays correct if
/// [`oag_weapons::projectile::plasma::CHARGE_SECONDS`] ever moves off its
/// current `1.0` - the two forms are identical today only because the
/// constant happens to equal the wind-up the original hardcodes. `charge` is
/// clamped into `[0, CHARGE_SECONDS]` first: this engine's own countdown
/// never leaves that range, but a flying bolt's `charge` sits pinned at
/// exactly `0.0`, so the clamp is what makes the formula read as "the last
/// charging value" rather than needing a second branch for it - see below.
///
/// **Ports the further `* 0.5` when the firing craft's `+0x6d` flag is set -
/// closed 2026-09-16.** `craft+0x6d` is the internal/cockpit camera flag:
/// `camera.md`'s SELECT-view cycle writes `1` for `OPT_INT` and `0` for both
/// external views, on the player craft alone, and `shield-pickup.md` found a
/// second, independent consumer (`ShipShield_Update`'s own cockpit-shell
/// branch) that raised the reading from 70 to 82. This engine already models
/// the same byte as `oag_display::CameraView::draws_own_ship` - `false` for
/// the internal view - consumed here as `cockpit`, the caller's own
/// `projectile.owner == 0 && !self.draws_own_ship()`: `+0x6d` is only ever
/// *written* for the player's own craft (camera.md: "setting one flag on the
/// player craft"), so an opponent's charging bolt reads a byte that is always
/// zero in the original and never halves, whichever craft the local camera
/// happens to be looking at.
///
/// **Not frozen the way the ramp itself is, and that is a known, accepted
/// gap.** `Plasma_UpdateCharge` - and therefore this halving - only runs
/// while `charging != 0` in the original; once a bolt is released the
/// function is never called again, so whatever `craft+0x6d` last read stays
/// baked into the value forever, the same freeze [`Self::plasma_flare_scale`]'s
/// own "after release" section already documents for the ramp. This engine
/// has no per-projectile record of "was the camera internal at the tick this
/// bolt released", so `cockpit` is read live every tick, charging or flying
/// alike - correct for the charging phase, since the original reads the byte
/// fresh on every charging tick too, and wrong only in the narrow case of a
/// player switching camera view during a bolt's own brief flight, where the
/// original's frozen value and this port's live one can disagree for that
/// bolt's remaining ticks. Adding a frozen field is `oag_weapons::projectile`
/// work, out of this function's own reach.
///
/// **What `scale` actually multiplies is the same thing on both sides.**
/// `Stage::rescale`/`System::rescale` change an instance's *severity* - the
/// field this project's own read of `ParticleSystem_DeriveScaledParams`
/// (`docs/ghidra/functions/psp-pulse-usa/particle-system.md`) establishes
/// multiplies every emitter's ejection speed and every particle's drawn
/// size, not a one-off matrix scale applied once at spawn. "Scales the
/// effect by" is this project's own phrase for exactly that field wherever
/// else it has been read (`contact-response.md`'s collision sparks), and
/// `plasma.md`'s own reading lists the matrix push and the scale as two
/// *separate* operations on the same instance - itself evidence the scale is
/// not folded into the matrix - so this is the right call and not a
/// stand-in for one.
///
/// **After release, the scale does not reset - it freezes at the wind-up's
/// own maximum, `0.75`, and this is not a choice.** `Plasma_Launch`'s
/// recovered body (`plasma.md`) is a complete seven-statement listing - every
/// field it writes is named with its own offset comment - and none of them is
/// a severity or scale write; `Psys_Reparent_q` moves the instance's matrix,
/// it does not touch severity either. So nothing in the recovered release
/// path changes the value `Plasma_UpdateCharge` last wrote, which this
/// engine's own countdown pins at exactly `0.75` (`remaining == 0.0`) the
/// tick charge hits zero - carrying that value forward rather than resetting
/// to `1.0` is what the recovered functions imply, not a stand-in for a gap
/// in them. Confirmed against this engine's own screenshot probe: an early
/// draft that reset to `1.0` produced the single largest brightness jump in
/// the whole series exactly at release (a fixed-crop brightness metric going
/// from `0.205` at the last charging tick to `0.304` after release, bigger
/// than the entire ramp before it) - a pop the recovered functions give no
/// reason to expect, which is what caught the bug.
#[must_use]
pub(crate) fn plasma_flare_scale(
    kind: Option<oag_tables::weapons::Weapon>,
    charge: f32,
    cockpit: bool,
) -> f32 {
    if kind != Some(oag_tables::weapons::Weapon::Plasma) {
        return 1.0;
    }
    let charge_seconds = oag_weapons::projectile::plasma::CHARGE_SECONDS;
    let scale = (charge_seconds - charge.clamp(0.0, charge_seconds)) / charge_seconds * 0.75;
    if cockpit { scale * 0.5 } else { scale }
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
pub(crate) fn missile_flare_anchors(position: Vec3, velocity: Vec3, age: f32) -> (Vec3, Vec3) {
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
/// [`oag_weapons::projectile::Projectile::bounces`]), so `kind` gates this
/// the same way [`flare_effect_for`] gates on the weapon rather than trusting
/// the counter alone - a stray nonzero `bounces` on some other kind must
/// never read as a bounce.
#[must_use]
pub(crate) fn bounced_this_tick(
    kind: Option<oag_tables::weapons::Weapon>,
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
pub(crate) fn bounce_effect_for(kind: Option<oag_tables::weapons::Weapon>) -> Option<Trigger> {
    match kind? {
        oag_tables::weapons::Weapon::Missile => Some(Trigger::MissileBounce),
        oag_tables::weapons::Weapon::Shuriken => Some(Trigger::ShurikenBounce),
        _ => None,
    }
}

/// The course ring point whose own progress is nearest `progress`, by
/// circular distance.
///
/// A full scan rather than a binary search: `Course::progress_at` wraps at
/// the ring's own start index and is not monotonic in ring-index order
/// across that wrap, so a search that assumed monotonicity would find the
/// wrong point near the start line. The same "flat scan rather than a
/// spatial index" shape `Course::locate`'s own module doc comment argues
/// for, determinism aside: an occasional per-tick walk over a few thousand
/// points is cheap next to a wrong answer at a wrap, and this runs once a
/// tick for the one Quake wave that can ever exist, not once per craft.
///
/// `None` for an empty course.
pub(super) fn course_index_near_progress(course: &Course, progress: f32) -> Option<usize> {
    let length = course.length();
    (0..course.len())
        .filter_map(|index| {
            let at = course.progress_at(index)?;
            let raw = (at - progress).abs();
            let delta = if length > 0.0 {
                raw.min(length - raw)
            } else {
                raw
            };
            Some((index, delta))
        })
        .min_by(|(_, a), (_, b)| a.total_cmp(b))
        .map(|(index, _)| index)
}
