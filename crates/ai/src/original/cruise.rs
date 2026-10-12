//! Wipeout Pure's opponent throttle: the legacy block's cruise law, measured on
//! 1,688 of 1,689 live samples over two classes
//! (`docs/ghidra/functions/psp-pure-usa/ai-controls.md`, confidence 94).
//!
//! Not Pulse's [`super::opponent_thrust`]: Pure's opponent does not index a
//! table by the player's place or hold a formation. Each opponent cruises at a
//! speed drawn once per race for its grid slot, times a per-track speed tweak,
//! times one field-wide zone multiplier (the nearest opponent's gap to the
//! player, read off `ZoningStats`), times its own fatigue, and blends toward
//! its slot's `StartSpeed` during the launch. Every opponent runs it every
//! frame (measured: the snapshot sets every racer's round-robin byte).
//!
//! The tables are [`PureClassRace`] off the disc; the literals here are the
//! executable's code constants, cited to the evidence page. **The two draws
//! (`cruise` and the tweak) are the original's `rand()`; this crate draws them
//! from the seeded [`Rng`] instead, chosen, not measured** - the live samples
//! read the drawn values, not the stream that drew them.
//!
//! Not built here (named on the evidence page): the steering, Pure's ringless
//! lane planner and its other-craft scoring, the position-only rail, the pitch
//! fade and the fork hold. Nothing flies this law yet: no Pure campaign exists
//! in this project, and the original's opponent is the campaign's.

use oag_core::Rng;
use oag_tables::pure_ai_stats::{Fatigue, PureClassRace, ZoneCurve};

/// One draw `lo + u * (hi - lo)`, `u` uniform on `[0, 1)`, the shape of
/// `AiStats_LoadAll`'s `rand() * 2^-31 * (hi - lo) + lo`. The shipped tweak
/// ranges author `Min >= Max`; the formula takes them as written.
fn draw(lo: f32, hi: f32, rng: &mut Rng) -> f32 {
    rng.next_f32() * (hi - lo) + lo
}

/// What `AiStats_LoadAll` draws once per race for one class.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CruiseDraws {
    /// The speed tweak (`R+0x24`, every driver's `+0x190`), drawn from the
    /// track's `<ClassTweak Min Max>`.
    pub tweak: f32,
    /// The cruise speed per grid slot (`R+0x68[slot - 1]`), drawn from
    /// `NormalRaceSpeedMin..Max`.
    pub cruise: [f32; 8],
}

impl CruiseDraws {
    /// The draws, tweak first then slots `1..=8` in order (the original draws
    /// the tweak, then the eight slots, per class).
    #[must_use]
    pub fn draw(table: &PureClassRace, tweak: (f32, f32), rng: &mut Rng) -> Self {
        let tweak = draw(tweak.0, tweak.1, rng);
        let cruise = std::array::from_fn(|k| draw(table.cruise_min[k], table.cruise_max[k], rng));
        Self { tweak, cruise }
    }
}

/// `RaceManager_UpdateZoneSpeed` (`0x08810b30`): the curve's `SpeedPercent`
/// at `gap`, piecewise linear over `Position`, held flat past either end.
///
/// Written as the executable compares, so the bits match the live samples:
/// `gap <= Position[0]` is the first point, `Position[5] <= gap` the last.
#[must_use]
pub fn zone_speed(curve: &ZoneCurve, gap: f32) -> f32 {
    let (p, s) = (&curve.position, &curve.speed);
    if gap <= p[0] {
        return s[0];
    }
    if p[5] <= gap {
        return s[5];
    }
    let mut i = 0;
    while i + 1 < 5 && p[i + 1] <= gap {
        i += 1;
    }
    s[i] + ((gap - p[i]) / (p[i + 1] - p[i])) * (s[i + 1] - s[i])
}

/// `Craft_GapAlongTrack` (`0x08923a1c`): `other - player` race progress,
/// wrapped into `+/- half` a lap. Positive is the other craft ahead.
#[must_use]
pub fn gap_along_track(player: f32, other: f32, lap: f32) -> f32 {
    let mut g = other - player;
    while lap * 0.5 < g {
        g -= lap;
    }
    while g < -(lap * 0.5) {
        g += lap;
    }
    g
}

/// The field-wide zone multiplier (`RM+0x134`) for one frame: the nearest
/// opponent by [`gap_along_track`], `LeadZone` while the player leads, `1`
/// with no opponent. `opponents` excludes the player and any eliminated
/// craft. The original writes it once a frame from the positions it sees
/// then, so a driver reads the previous frame's value (measured).
#[must_use]
pub fn zone_multiplier(
    table: &PureClassRace,
    player: f32,
    player_leads: bool,
    opponents: impl IntoIterator<Item = f32>,
    lap: f32,
) -> f32 {
    let mut nearest: Option<f32> = None;
    for other in opponents {
        let g = gap_along_track(player, other, lap);
        if nearest.is_none_or(|n| g.abs() < n.abs()) {
            nearest = Some(g);
        }
    }
    let Some(gap) = nearest else {
        return 1.0;
    };
    let curve = if player_leads {
        &table.lead_zone
    } else {
        &table.zone
    };
    zone_speed(curve, gap)
}

/// One opponent's launch blend and fatigue (`Ai_UpdateFatigue`, `0x08919918`).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct CruiseState {
    /// `driver+0x1c0`: `1` holds the slot's `StartSpeed`, `0` the cruise.
    pub launch: f32,
    /// `driver+0xb0`: seconds of fatigue.
    pub fatigue_time: f32,
}

impl CruiseState {
    /// One frame. `launch_phase` is the race manager's flag (`RM+0x1e34`), set
    /// from the grid until the field has strung out past `GridSpread`;
    /// `own_gap` this craft's [`gap_along_track`] from the player;
    /// `race_time` seconds. Returns `driver+0x54`, the zone multiplier times
    /// the fatigue factor.
    ///
    /// The `1/60` and the `0.5`/`0.002` launch steps are per call literals in
    /// the original, not `dt`; at this project's fixed 60 Hz they are per tick.
    pub fn update(
        &mut self,
        fatigue: &Fatigue,
        launch_phase: bool,
        own_gap: f32,
        race_time: f32,
        zone: f32,
    ) -> f32 {
        self.launch = if launch_phase {
            (self.launch + 0.5).min(1.0)
        } else {
            (self.launch - 0.002).max(0.0)
        };
        let inside =
            fatigue.zone_back < own_gap && own_gap < fatigue.zone_front && race_time >= 10.0;
        let mut t = self.fatigue_time;
        if inside {
            t += 0.016_666_668;
            let cap = fatigue.start_time + fatigue.decay_time;
            if cap < t {
                t = cap;
            }
        } else {
            if 0.0 < t {
                t -= fatigue.restore_rate / 60.0;
            }
            if t < 0.0 {
                t = 0.0;
            }
        }
        self.fatigue_time = t;
        let factor = if 0.0 < t && fatigue.start_time <= t {
            let k = ((t - fatigue.start_time) / fatigue.decay_time).min(1.0);
            (fatigue.multiplier - 1.0) * k + 1.0
        } else {
            1.0
        };
        zone * factor
    }
}

/// The legacy-block thrust (`Ai_ComputeControls`, `0x0891ac04`) for grid slot
/// `slot` (`1..=8`): `x = cruise * tweak * zone_fatigue`, then
/// `x + (StartSpeed - x) * launch`. In the order `scripts/psp-pure-ai-thrust.py
/// --predict` reproduced every live sample in `float32`.
#[must_use]
pub fn cruise_thrust(
    table: &PureClassRace,
    draws: &CruiseDraws,
    slot: u8,
    zone_fatigue: f32,
    launch: f32,
) -> f32 {
    let k = usize::from(slot.clamp(1, 8)) - 1;
    let x = draws.cruise[k] * draws.tweak * zone_fatigue;
    x + (table.start_speed[k] - x) * launch
}

/// `Ai_Update`'s tail on the thrust (`0x08919764..0x08919870`): `x 0.7` while
/// the craft carries the flag at `craft+0xd0 -> +0x134`, `100` when Pulse's
/// per-point speed floor fires, and never below `35`. Static reading, not
/// exercised by the live samples.
#[must_use]
pub fn finish_thrust(thrust: f32, slowed: bool, below_speed_floor: bool) -> f32 {
    let mut t = if slowed { thrust * 0.7 } else { thrust };
    if below_speed_floor {
        t = 100.0;
    }
    t.max(35.0)
}

#[cfg(test)]
mod tests;
