//! The XP block of 2048's `RaceSummary` page: what the race paid, the profile's
//! total and the rank bar, counted up the way the original's per-frame update
//! counts them (`docs/ghidra/functions/vita-2048-eu-v104/endrace-summary.md`,
//! "The XP block").
//!
//! The model is a pure function of the ticks the page has been up, so a test
//! reads any instant without a clock.

use oag_tables::xp::Ranks;

/// Race XP and the event total count up at this many points a second.
/// Measured: the `200.0` constant in `FUN_810d056c`.
const EVENT_RATE: f32 = 200.0;
/// The rank counter climbs from the pre-race total at this rate. Measured: `8000.0`.
const RANK_RATE: f32 = 8000.0;
/// Width in pixels of `TotalXPFill` at a full bar. Measured: `48.0`.
const FILL_FULL: f32 = 48.0;
/// `RankItem`'s authored width, which the flip scales.
pub const RANK_ITEM_WIDTH: f32 = 64.0;
/// Seconds the rank badge's flip lasts. Measured: `1.25`.
const FLIP_SECONDS: f32 = 1.25;
/// Seconds `RESULTS_NEW_RANK` holds in place of the total. Measured: `3.0`.
const BANNER_SECONDS: f32 = 3.0;
/// The two total lines swap every this many seconds once the count is done.
/// Measured: the timer wraps at `6.0` and the profile total shows past `3.0`.
const LABEL_HALF_CYCLE: f32 = 3.0;

/// The disc's wording of the three lines that take a number, with `%d` left in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XpWords {
    /// `RESULTS_EVENT_TOTAL`.
    pub event_total: String,
    /// `TOTAL_NUMBER_XP`.
    pub profile_total: String,
    /// `RESULTS_NEW_RANK`.
    pub new_rank: String,
}

/// What a finished event paid and where it left the profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct XpResult {
    /// The `RaceXP` line: the race's tally less the completion bonus.
    pub race_xp: u32,
    /// The completion bonus (`PassBonusXP`); `0` hides its box.
    pub bonus_xp: u32,
    /// The race's whole tally, bonus included (the `TotalXP` line's count).
    pub event_total: u32,
    /// The profile's total after banking.
    pub total_after: u32,
    /// This title's rank table.
    pub ranks: Ranks,
    /// The wording.
    pub words: XpWords,
}

/// What the XP widgets show at one instant.
#[derive(Debug, Clone, PartialEq)]
pub struct XpFrame {
    /// `RaceXP` / `SpeedRaceXP`.
    pub race_xp: u32,
    /// `TotalXP`'s text.
    pub label: String,
    /// `RankText`.
    pub rank_text: String,
    /// `RankItem`'s width factor and `RankText`'s scale factor (`1.0` at rest).
    pub flip: f32,
    /// `TotalXPFill`'s width in pixels.
    pub fill_px: u32,
}

fn with_number(template: &str, number: u32) -> String {
    template.replacen("%d", &number.to_string(), 1)
}

impl XpResult {
    /// The profile total before the race banked.
    fn before(&self) -> u32 {
        self.total_after.saturating_sub(self.event_total)
    }

    /// The rank-counter value at `seconds` (`+0xdc`).
    fn counter(&self, seconds: f32) -> u32 {
        let climbed = (RANK_RATE * seconds) as u32;
        self.before().saturating_add(climbed).min(self.total_after)
    }

    /// The instants the rank counter reaches a new rank's threshold, with that rank.
    fn crossings(&self) -> Vec<(f32, u32)> {
        let before = self.before();
        let from = self.ranks.rank(before) + 1;
        let to = self.ranks.rank(self.total_after);
        (from..=to)
            .filter_map(|rank| {
                let threshold = *self.ranks.thresholds.get(rank as usize - 1)?;
                Some((threshold.saturating_sub(before) as f32 / RANK_RATE, rank))
            })
            .collect()
    }

    /// `TotalXPFill`'s width for `counter`: its fraction of the way from this
    /// rank's threshold to the next, in 48ths, a full bar only when it is full.
    fn fill_px(&self, counter: u32) -> u32 {
        let rank = self.ranks.rank(counter);
        let low = self.ranks.threshold_xp(rank, counter);
        let high = self.ranks.threshold_xp(rank + 1, counter);
        let fraction = if high > low {
            (counter - low) as f32 / (high - low) as f32
        } else {
            1.0
        };
        let px = (fraction * FILL_FULL) as u32;
        if px == FILL_FULL as u32 && fraction < 1.0 {
            px - 1
        } else {
            px
        }
    }

    /// The widgets at `ticks` 60 Hz ticks after the page came up.
    #[must_use]
    pub fn at(&self, ticks: u32) -> XpFrame {
        let seconds = ticks as f32 / 60.0;
        let counted = (EVENT_RATE * seconds) as u32;
        let race_xp = self.race_xp.min(counted);
        let event_now = self.event_total.min(self.bonus_xp.saturating_add(counted));
        let counted_at = self.event_total.saturating_sub(self.bonus_xp) as f32 / EVENT_RATE;
        let counter = self.counter(seconds);
        let rank = self.ranks.rank(counter);

        let mut label = if event_now < self.event_total {
            with_number(&self.words.event_total, event_now)
        } else if (seconds - counted_at) % (2.0 * LABEL_HALF_CYCLE) > LABEL_HALF_CYCLE {
            with_number(&self.words.profile_total, self.total_after)
        } else {
            with_number(&self.words.event_total, self.event_total)
        };

        let mut flip = 1.0;
        let mut rank_text = rank.to_string();
        for (at, new_rank) in self.crossings() {
            let since = seconds - at;
            if (0.0..BANNER_SECONDS).contains(&since) {
                label.clone_from(&self.words.new_rank);
            }
            if (0.0..FLIP_SECONDS).contains(&since) {
                flip = ((3.2 * std::f32::consts::PI * since).cos() + 1.0) * 0.5;
                rank_text = if since > 0.9375 {
                    new_rank.to_string()
                } else if since >= 0.3125 {
                    String::new()
                } else {
                    (new_rank - 1).to_string()
                };
            }
        }
        XpFrame {
            race_xp,
            label,
            rank_text,
            flip,
            fill_px: self.fill_px(counter),
        }
    }
}
