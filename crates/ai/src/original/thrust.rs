//! `AI_ComputeOpponentThrust` (`0x08855904`): the original opponent's throttle,
//! read whole and reproduced on 1,140 live samples
//! (`docs/ghidra/functions/psp-pulse-usa/ai-controls.md`, confidence 94).
//!
//! The result is a percentage of the player's own full throttle on the craft's
//! own engine, and above `100` it also lifts the engine's acceleration cap
//! (`oag_physics::engine`). Every table value comes from the class's
//! `AIRaceStats_<class>.xml` ([`ClassRace`]); the literals below are the
//! executable's own code constants, cited to the same page.

use oag_tables::ai_race_stats::ClassRace;

/// One entry of the racer table, as the thrust law reads it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Racer {
    /// Race progress in lap-length units (`racer+0x240`): the distance covered
    /// along the circuit, laps included. Only differences between racers and
    /// against the start are read, so the origin is free.
    pub progress: f32,
    /// Race position, `1`-based (`racer+0x344`).
    pub place: u8,
    /// The racer's formation slot (`driver+0x74`), see [`super::Wander`].
    pub spread: f32,
}

/// What one opponent's thrust law reads, besides its class tables.
#[derive(Debug, Clone, Copy)]
pub struct ThrustInput<'a> {
    /// Every racer, the player included, in racer-table order.
    pub racers: &'a [Racer],
    /// Which entry of [`Self::racers`] is the craft being driven.
    pub own: usize,
    /// Which entry is the player's craft.
    pub player: usize,
    /// The driven racer has finished (`racer+0x82`). Only ever true for the
    /// player's own driver past the line; an opponent is still racing.
    pub finished: bool,
    /// The player's craft is in ship state `4..=6`, which turns the rubber band
    /// off. Never seen live; static reading.
    pub rubber_band_off: bool,
    /// Race time in seconds (`RM+0x2b8`).
    pub race_time: f32,
    /// `AI_ResolveSkillScale`'s value for this race.
    pub skill: f32,
}

/// The law's output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Thrust {
    /// Throttle, percent of full: `1.0` at the least, uncapped above `100`
    /// until the craft is near the player after 20 s.
    pub throttle: f32,
    /// The formation gap the law steered toward (`driver+0x9c`, logged live).
    pub gap: f32,
}

/// Places beyond the eighth read the eighth row, and a zero place the first:
/// the tables have eight rows and the original indexes them unchecked.
fn row(place: u8) -> usize {
    usize::from(place.clamp(1, 8)) - 1
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a * (1.0 - t) + t * b
}

/// `AI_ComputeOpponentThrust`, after `Ai_UpdateSpreadWander` has refreshed
/// the driven racer's [`Racer::spread`].
///
/// The arithmetic is written in the order `scripts/psp-ai-thrust.py --predict`
/// recomputes it in `float32`, which reproduced every live sample to `1e-3`
/// with a worst residual of `0.00000`; reassociating it would move bits.
#[must_use]
pub fn opponent_thrust(table: &ClassRace, input: &ThrustInput<'_>) -> Thrust {
    let thrust_table = &table.thrust;
    let a = &thrust_table.ai_thrust;
    let s = input.skill;
    let (lo, hi, base) = if s < 2.0 { (0, 1, 1.0) } else { (1, 2, 2.0) };
    let t = s - base;
    let (offset, multiplier) = thrust_table.skill_terms(s);
    let spread_mul = lerp(table.spread_multiplier[lo], table.spread_multiplier[hi], t);

    let me = input.racers[input.own];
    let player = input.racers[input.player];
    let r = player.place;
    let base_thrust = a[row(r)];
    let spread_dist = if input.finished {
        50.0
    } else {
        table.spread_dist[row(r)] * spread_mul
    };
    let ref_place = if r <= 1 { 2 } else { r - 1 };

    let mut lead = 100_000.0_f32;
    let mut trail = 100_000.0_f32;
    let mut reference = None;
    for (k, racer) in input.racers.iter().enumerate() {
        if racer.place == ref_place {
            reference = Some(*racer);
        }
        if input.finished || k != input.player {
            let d = player.progress - racer.progress;
            lead = lead.min(d);
            trail = trail.min(-d);
        }
    }
    lead = (lead - table.when_leading.min_speed_change_dist).max(0.0);
    trail = (trail - table.when_behind.min_speed_change_dist).max(0.0);
    let reference = reference.unwrap_or(me);

    let gap = reference.progress + (me.spread - reference.spread) * spread_dist - me.progress;
    let time = input.race_time;
    let gain = (2.0 - time * 0.08).max(0.5);
    let step = (gap * gain).max(-base_thrust * 0.3).min(base_thrust * 0.3);
    let mut x = base_thrust + step;
    if !input.rubber_band_off {
        x += (lead * table.when_leading.speed_mul).min(table.when_leading.max_speed_change);
        x -= (trail * table.when_behind.speed_mul).min(table.when_behind.max_speed_change);
    }
    let throttle = offset + (x - a[0]) * multiplier + a[0];
    let cap = if time > 20.0 && (me.progress - player.progress).abs() < 350.0 {
        100.0
    } else {
        1000.0
    };
    Thrust {
        throttle: throttle.min(cap).max(1.0),
        gap,
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
pub(crate) use tests::table as tests_table;
