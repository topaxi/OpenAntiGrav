//! `Ai_PlanLane` (`0x08854e7c`) and the lane ring (`0x088533bc..0x08853534`),
//! read at instruction level 2026-10-12 for this build. The summary is
//! `docs/ghidra/functions/psp-pulse-usa/ai-controls.md`, "`Ai_PlanLane`".
//!
//! A lane is a coordinate in `[-1, 1]` across the AI corridor. The ring holds
//! the planned lane for the next 80 slots of `1/4000` lap each; the planner
//! appends to it at a rate-limited lateral velocity.

use oag_core::Rng;

/// Slots the ring holds (`AiLane_IsFull`: `count > 79`).
pub const RING: usize = 80;

/// Slots a lap (`AiLane_At`: `(int)(progress * 4000.0)`).
const SLOTS_PER_LAP: i32 = 4000;

/// The ring at `driver+0x214..+0x35c`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Ring {
    lanes: [f32; RING],
    /// `+0x214`: the progress slot the head stands for.
    head_slot: i32,
    /// `+0x218`: the head's index in [`Self::lanes`].
    head: usize,
    /// `+0x21c`: slots queued.
    count: usize,
}

impl Default for Ring {
    fn default() -> Self {
        Self {
            lanes: [0.0; RING],
            head_slot: 0,
            head: 0,
            count: 0,
        }
    }
}

/// `(slot - head + 102000) % 4000 - 2000`: the slots from the head to
/// `slot`, wrapped into `-2000..2000`.
fn slots_from(slot: i32, head_slot: i32) -> i32 {
    (slot - head_slot + 102_000).rem_euclid(SLOTS_PER_LAP) - 2000
}

/// `(int)(progress * 4000.0)`, truncating as the original's `trunc.w.s` does.
fn slot_of(progress: f32) -> i32 {
    (progress * 4000.0) as i32
}

impl Ring {
    /// `AiLane_At` (`0x088533bc`): the planned lane at `progress`, the
    /// nearest queued slot, no interpolation.
    #[must_use]
    pub fn at(&self, progress: f32) -> f32 {
        let mut offset = slots_from(slot_of(progress), self.head_slot);
        if self.count as i32 <= offset {
            offset = self.count as i32 - 1;
        }
        let offset = offset.max(0) as usize;
        self.lanes[(offset + self.head) % RING]
    }

    /// `AiLane_Advance` (`0x08853438`): drops the slots the craft has passed.
    pub fn advance(&mut self, progress: f32) {
        let passed = slots_from(slot_of(progress), self.head_slot);
        self.head_slot = slot_of(progress);
        if passed > 0 {
            let left = self.count as i32 - passed;
            self.head = (self.head + passed as usize) % RING;
            if left < 1 {
                self.lanes[self.head] = 0.0;
                self.count = 1;
            } else {
                self.count = left as usize;
            }
        }
    }

    /// `AiLane_Reset` (`0x08853534`): one slot holding `lane`.
    pub fn reset(&mut self, lane: f32) {
        self.lanes[self.head] = lane;
        self.count = 1;
    }

    /// `AiLane_Push` (`0x088534d8`).
    fn push(&mut self, lane: f32) {
        self.lanes[(self.head + self.count) % RING] = lane;
        self.count += 1;
    }

    /// `AiLane_Last` (`0x08853508`).
    #[must_use]
    fn last(&self) -> f32 {
        self.lanes[(self.head + self.count + RING - 1) % RING]
    }

    /// `AiLane_IsFull` (`0x088534c8`).
    #[must_use]
    fn is_full(&self) -> bool {
        self.count > RING - 1
    }
}

/// `Ai_AddLaneBump` (`0x08853550`): a trapezoid over the nine candidates,
/// written as the decompile reads.
fn add_bump(scores: &mut [f32; 9], centre: f32, half_width: f32, weight: f32) {
    if weight <= 0.001 {
        return;
    }
    let w = if half_width < 0.05 { 0.05 } else { half_width };
    let inv = 1.0 / w;
    let inner_lo = centre - w;
    let inner_hi = w + centre;
    for (i, score) in scores.iter_mut().enumerate() {
        let x = i as f32 * 0.25 - 1.0;
        if inner_lo - w < x && x < w + inner_hi {
            let add = if x < inner_lo {
                (x - (inner_lo - w)) * 0.95 * inv * weight
            } else if x < centre {
                weight * ((x - inner_lo) * 0.05 * inv + 0.95)
            } else if x < inner_hi {
                weight * (0.95 - (x - inner_hi) * 0.05 * inv)
            } else {
                (x - (w + inner_hi)) * -0.95 * inv * weight
            };
            *score += add;
        }
    }
}

/// What the planner reads of one tick, besides its own state.
#[derive(Debug, Clone, Copy)]
pub struct PlanInput {
    /// The craft's lane coordinate on its own sample, unclamped.
    pub own_lane: f32,
    /// This is the driver's round-robin tick (`racer+0x360`).
    pub lane_tick: bool,
    /// Signed count of this tick's contacts: `+1` for each on the craft's
    /// left, `-1` for each on its right.
    pub contact_sides: i32,
    /// The current point's `unk_0x5c`/`unk_0x5d`, the lane exclusion band.
    pub band: Option<(u8, u8)>,
}

/// `Ai_PlanLane`'s own state on the driver.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Planner {
    /// `driver+0x3c` cleared: the craft has been located once.
    located: bool,
    /// The ring.
    pub ring: Ring,
    /// `+0x1f0`: the nine smoothed candidate scores.
    scores: [f32; 9],
    /// `+0x1c8`: the preferred lane.
    preferred: f32,
    /// `+0x3b8`: the contact push.
    push: f32,
    /// `+0x3b4`: the ring's lateral velocity, lane units a slot.
    lane_velocity: f32,
    /// `+0x1b4`: the lane chosen on the last round-robin tick.
    chosen: f32,
    /// `+0x148`: which side of the band the target keeps to, `0` none.
    band_side: u8,
    /// `+0x44`: the lane target.
    pub target: f32,
    /// The ring's lateral rate, lane units a slot per slot: Pulse's `0.001`
    /// ([`super::RING_RATE`]), or a 2048 event's ([`super::event_ring_rate`],
    /// 2048 `driver+0xc4`).
    pub rate: f32,
    /// 2048's search coin (`driver+0xc8`), `None` on a title whose search
    /// has none: set, the choice searches up first; clear, down first.
    pub coin: Option<bool>,
    /// 2048 (`0x8119dcc4`): when the chosen candidate's smoothed score is
    /// above `0.5`, the preferred lane becomes `1 - chosen^2` on the chosen
    /// lane's side.
    pub preferred_follows_choice: bool,
}

impl Default for Planner {
    fn default() -> Self {
        Self {
            located: false,
            ring: Ring::default(),
            scores: [0.0; 9],
            preferred: 0.0,
            push: 0.0,
            lane_velocity: 0.0,
            chosen: 0.0,
            band_side: 0,
            target: 0.0,
            rate: super::RING_RATE,
            coin: None,
            preferred_follows_choice: false,
        }
    }
}

/// `Ai_RandUnit` (`0x08852334`): `rand() * 2^-31`, the generator's top 31 bits.
fn rand_unit(rng: &mut Rng) -> f32 {
    (rng.next_u32() >> 1) as f32 * 4.656_613e-10
}

impl Planner {
    /// `Ai_PlanLane` for an opponent, after `AiLane_Advance`. The authority
    /// ramp at its head lives on [`super::OriginalDriver`].
    pub fn plan(&mut self, input: &PlanInput, rng: &mut Rng) {
        if !self.located {
            self.located = true;
            let lane = input.own_lane.clamp(-3.0, 3.0);
            self.ring.reset(lane);
            let lane = lane.clamp(-1.0, 1.0);
            self.chosen = lane;
            self.lane_velocity = 0.0;
            self.preferred = 0.0;
        }
        if input.contact_sides != 0 {
            self.push += 0.2 * input.contact_sides as f32;
        }
        if input.lane_tick {
            let mut fresh = [0.0_f32; 9];
            add_bump(&mut fresh, 0.0, 1.0, 0.05);
            add_bump(&mut fresh, self.preferred, 0.15, 0.5);
            for (smoothed, new) in self.scores.iter_mut().zip(fresh) {
                *smoothed = *smoothed * 0.9 + new * 0.1;
            }
            let best = match self.coin {
                None => self.local_best(input.own_lane),
                Some(up_first) => self.coin_ordered_best(input.own_lane, up_first),
            };
            self.chosen = best as f32 * 0.25 - 1.0;
            if self.preferred_follows_choice && self.scores[best] > 0.5 {
                let p = 1.0 - self.chosen * self.chosen;
                self.preferred = if self.chosen < 0.0 { -p } else { p };
            }
        }
        if rng.below(400) == 125 {
            let u = rand_unit(rng) * 2.0 - 1.0;
            self.preferred = if u < 0.0 { u * u - 1.0 } else { 1.0 - u * u };
        }
        // 2048 (`0x8119d8f0`): one call in 200 redraws the search coin.
        if let Some(coin) = self.coin.as_mut()
            && rng.below(200) == 1
        {
            *coin = rng.below(2) == 1;
        }
        self.push = self.push.clamp(-2.0, 2.0);
        let sum = self.chosen + self.push;
        self.target = if 1.0 < sum { 1.0 } else { sum };
        if self.target < -1.0 {
            self.target = -1.0;
        }
        self.push *= 0.95;
        self.keep_out_of_band(input.band);
        self.fill();
    }

    /// The search `Ai_PlanLane` ends its choice with. It starts at the
    /// candidate nearest the craft's own lane and takes the best score over
    /// candidates `0..8` and that start, ties to the earlier visited. The
    /// first, global pass the original runs before it is overwritten by this
    /// one and so is not repeated. The `-1` blocking test never fires (no
    /// score reaches it) and is not reproduced.
    fn local_best(&self, own_lane: f32) -> usize {
        let start = (((own_lane + 1.0) * 0.5 * 8.0) as i32).clamp(0, 8) as usize;
        let mut best = start;
        let mut best_score = -1.0_f32;
        for i in start..8 {
            if self.scores[i] > best_score {
                best = i;
                best_score = self.scores[i];
            }
        }
        for i in (0..=start).rev() {
            if self.scores[i] > best_score {
                best = i;
                best_score = self.scores[i];
            }
        }
        best
    }

    /// 2048's search (`0x8119d534..0x8119d632`): from the candidate nearest
    /// the craft's own lane, with `-1` as the score to beat, two passes in the
    /// coin's order, up through candidate `8` and down through `0`; a strictly
    /// higher score wins, so a tie keeps the earlier visit, and the best
    /// carries from the first pass into the second. A score at or below `-1`
    /// ends a pass, which none reaches here (only the weapon avoidance this
    /// port does not carry writes one).
    fn coin_ordered_best(&self, own_lane: f32, up_first: bool) -> usize {
        let start = (((own_lane + 1.0) * 0.5 * 8.0) as i32).clamp(0, 8) as usize;
        let mut best = start;
        let mut best_score = -1.0_f32;
        let visit = |i: usize, best: &mut usize, best_score: &mut f32| {
            if self.scores[i] > *best_score {
                *best = i;
                *best_score = self.scores[i];
            }
        };
        for up in [up_first, !up_first] {
            if up {
                for i in start..9 {
                    visit(i, &mut best, &mut best_score);
                }
            } else {
                for i in (0..=start).rev() {
                    visit(i, &mut best, &mut best_score);
                }
            }
        }
        best
    }

    /// The authored band (`0x0885568c..0x0885574c`): with `lo < hi` the
    /// target is kept **out of** `[lo, hi]`, on the side it was first seen
    /// on: below `lo` if it started under the band's middle, above `hi`
    /// otherwise. The shipped default `0xff`/`0x00` makes `lo > hi` and
    /// disables it. Read at instruction level (`c.le.s f13,f12` then
    /// `swc1 f12,0x44(s0)` caps the target at `lo`).
    fn keep_out_of_band(&mut self, band: Option<(u8, u8)>) {
        let Some((lo_byte, hi_byte)) = band else {
            self.band_side = 0;
            return;
        };
        let lo = f32::from(lo_byte) / 127.5 - 1.0;
        let hi = f32::from(hi_byte) / 127.5 - 1.0;
        if lo >= hi {
            self.band_side = 0;
            return;
        }
        if self.band_side == 0 {
            self.band_side = if self.target < (lo + hi) * 0.5 { 1 } else { 2 };
        }
        if self.band_side == 1 && self.target > lo {
            self.target = lo;
        }
        if self.band_side == 2 && self.target < hi {
            self.target = hi;
        }
    }

    /// The ring fill at `Ai_PlanLane`'s tail (`0x08855758..0x088558bc`), in
    /// the instruction order: one rate `a` ([`Self::rate`], Pulse's `0.001`)
    /// that every step is built from at runtime (`a * 0.9` to speed up, `1.1 * a` to reverse, `a` to
    /// brake once `v^2 * (0.5 / a)` reaches the distance left, `a * 15` the
    /// clamp).
    fn fill(&mut self) {
        let a = self.rate;
        let mut last = self.ring.last();
        while !self.ring.is_full() {
            let v = self.lane_velocity;
            let d = self.target - last;
            let brake = v * v * (0.5 / a);
            let next = if d * v < 0.0 {
                if d <= 0.0 { v - 1.1 * a } else { v + 1.1 * a }
            } else if d <= 0.0 {
                if brake < -d { v - a * 0.9 } else { v + a }
            } else if d <= brake {
                v - a
            } else {
                v + a * 0.9
            };
            let limit = a * 15.0;
            self.lane_velocity = next.min(limit).max(-limit);
            last += self.lane_velocity;
            self.ring.push(last);
        }
    }
}

#[cfg(test)]
mod tests;
