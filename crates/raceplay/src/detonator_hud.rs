//! What Detonator's HUD reads off the cannon: the readout, and the streak line's timers.
//!
//! The binding of each widget to a value is `docs/ghidra/functions/ps3-hdfury-eu/detonator-hud.md`
//! ("The answer in one table"); the widgets are drawn by `oag_hud::Detonator`.

use oag_weapons::detonator::cannon::{
    Awards, Cannon, POOL, STREAK_FIFTEEN, STREAK_FIVE, STREAK_TEN,
};

/// Seconds the streak line is held at full alpha, then fades over [`BONUS_FADE`]
/// (`Hud_RaiseChainBonus` stores `2.0` and `1.0`, 80).
const BONUS_HOLD: f32 = 2.0;
const BONUS_FADE: f32 = 1.0;

/// The streak line and the pips it recoloured, kept between ticks. Display only.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct ChainWatch {
    /// The awards as of the last tick, to see one become paid.
    seen: Awards,
    /// Volleys whose pip a streak award recoloured.
    tinted: u16,
    /// The line: its streak, amount and seconds since it was raised.
    line: Option<(u8, i32, f32)>,
}

/// Bit `i` set where volley `i` has hit (`hits[2i] > 1` or `hits[2i + 1] > 1`: a round is `1` once
/// fired and more once it has hit something).
pub(super) fn hit_volleys(cannon: &Cannon) -> u16 {
    (0..POOL / 2).fold(0, |mask, volley| {
        let hit = cannon.hits[2 * volley] > 1 || cannon.hits[2 * volley + 1] > 1;
        mask | (u16::from(hit) << volley)
    })
}

impl ChainWatch {
    /// One tick: an award newly paid raises its line and recolours the streak's pips, ending at
    /// the latest volley that hit; a clip with nothing hit clears the pips.
    pub(super) fn advance(&mut self, cannon: &Cannon, dt: f32) {
        let hit = hit_volleys(cannon);
        let (now, before) = (cannon.awards, self.seen);
        let raised = [
            (now.fifteen && !before.fifteen, 15, STREAK_FIFTEEN),
            (now.ten && !before.ten, 10, STREAK_TEN),
            (now.second_five && !before.second_five, 5, STREAK_FIVE),
            (now.first_five && !before.first_five, 5, STREAK_FIVE),
        ];
        for (new, streak, amount) in raised {
            if new && hit != 0 {
                self.line = Some((streak, amount, 0.0));
                let last = 15 - hit.leading_zeros();
                let first = (last + 1).saturating_sub(u32::from(streak));
                self.tinted |= (first..=last).fold(0, |mask, volley| mask | (1 << volley));
            }
        }
        if hit == 0 {
            self.tinted = 0;
        }
        self.seen = now;
        self.line = self
            .line
            .map(|(streak, amount, age)| (streak, amount, age + dt))
            .filter(|&(_, _, age)| age < BONUS_HOLD + BONUS_FADE);
    }

    /// The streak line as the HUD draws it.
    fn bonus(&self) -> Option<oag_hud::DetonatorBonus> {
        self.line
            .map(|(streak, amount, age)| oag_hud::DetonatorBonus {
                streak,
                amount,
                alpha: (1.0 - (age - BONUS_HOLD) / BONUS_FADE).clamp(0.0, 1.0),
            })
    }
}

/// The best medal the score has reached against the cell's targets: `score >= target`, a
/// counting mode (`PI_Cell_EvaluateMedalForValue` flips for mode 14, so the HUD can never show a
/// medal the results then withhold). Gold, silver and bronze share one rectangle, so only the
/// best tier reached is shown (**chosen**: the original's per-widget enable rule is unread, 50).
#[must_use]
pub(super) fn medal(
    score: i32,
    targets: Option<oag_tables::race_campaign::MedalTargets>,
) -> Option<oag_tables::race_campaign::Medal> {
    let targets = targets?;
    let value = i64::from(score);
    if value <= 0 {
        return None;
    }
    oag_tables::race_campaign::evaluate_targets(
        &[targets.gold, targets.silver, targets.bronze],
        value,
        true,
    )
}

/// The readout of a Detonator race from its cannon and stage.
///
/// A spent bullet icon is one volley fired: the clip refills at the start of a reload lockout,
/// so the icons read `clip` of the fifteen (`detonator-hud.md`, `0x0007e950`, 55).
#[must_use]
pub(super) fn readout(
    cannon: &Cannon,
    stage: u32,
    emp: (u32, bool),
    medal: Option<oag_tables::race_campaign::Medal>,
    watch: &ChainWatch,
) -> oag_hud::Detonator {
    oag_hud::Detonator {
        medal,
        emp_charge: emp.0,
        emp_ready: emp.1,
        score: cannon.score,
        clip: cannon.clip,
        stage,
        hit: hit_volleys(cannon),
        tinted: watch.tinted,
        bonus: watch.bonus(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DT: f32 = 1.0 / 60.0;

    /// A cannon whose volleys `0..n` each hit (their first round marked `2`).
    fn after(volleys: usize, awards: Awards) -> Cannon {
        let mut cannon = Cannon::new();
        cannon.clip = (15 - volleys) as u8;
        for volley in 0..volleys {
            cannon.hits[2 * volley] = 2;
            cannon.hits[2 * volley + 1] = 1;
        }
        cannon.awards = awards;
        cannon
    }

    #[test]
    fn the_medal_follows_the_score_through_each_target() {
        use oag_tables::race_campaign::{Medal, MedalTargets};
        let targets = Some(MedalTargets {
            gold: 9_000,
            silver: 6_000,
            bronze: 3_000,
        });
        for (score, shown) in [
            (0, None),
            (2_999, None),
            (3_000, Some(Medal::Bronze)),
            (5_999, Some(Medal::Bronze)),
            (6_000, Some(Medal::Silver)),
            (8_999, Some(Medal::Silver)),
            (9_000, Some(Medal::Gold)),
            (50_000, Some(Medal::Gold)),
        ] {
            assert_eq!(medal(score, targets), shown, "score {score}");
        }
        assert_eq!(medal(50_000, None), None, "no cell, no medal");
    }

    #[test]
    fn a_volley_lights_its_pip_once_a_round_hit() {
        let mut cannon = Cannon::new();
        cannon.hits[0] = 1;
        cannon.hits[1] = 1;
        cannon.hits[4] = 1;
        cannon.hits[5] = 3;
        assert_eq!(
            hit_volleys(&cannon),
            0b100,
            "fired is not hit; only volley 2 hit"
        );
    }

    #[test]
    fn the_readout_carries_the_cannon_and_the_stage() {
        let mut cannon = after(3, Awards::default());
        cannon.score = 12_345;
        let shown = readout(&cannon, 3, (5, true), None, &ChainWatch::default());
        assert_eq!((shown.emp_charge, shown.emp_ready), (5, true));
        assert_eq!(
            (
                shown.score,
                shown.clip,
                shown.stage,
                shown.hit,
                shown.tinted
            ),
            (12_345, 12, 3, 0b111, 0)
        );
        assert!(shown.bonus.is_none());
    }

    #[test]
    fn a_five_streak_raises_its_line_and_recolours_five_pips() {
        let mut watch = ChainWatch::default();
        watch.advance(&after(4, Awards::default()), DT);
        assert!(watch.bonus().is_none());
        let paid = Awards {
            first_five: true,
            ..Awards::default()
        };
        watch.advance(&after(5, paid), DT);
        let bonus = watch.bonus().expect("the line is up");
        assert_eq!(
            (bonus.streak, bonus.amount, bonus.alpha),
            (5, STREAK_FIVE, 1.0)
        );
        assert_eq!(watch.tinted, 0b1_1111);
    }

    #[test]
    fn the_line_holds_two_seconds_then_fades_over_one() {
        let mut watch = ChainWatch::default();
        let paid = Awards {
            ten: true,
            ..Awards::default()
        };
        let cannon = after(10, paid);
        watch.advance(&cannon, DT);
        for _ in 0..(1.9 / DT) as usize {
            watch.advance(&cannon, DT);
        }
        assert_eq!(watch.bonus().map(|b| b.alpha), Some(1.0));
        for _ in 0..(1.0 / DT) as usize {
            watch.advance(&cannon, DT);
        }
        let alpha = watch.bonus().map(|b| b.alpha).expect("still fading");
        assert!(alpha > 0.0 && alpha < 1.0, "{alpha}");
        for _ in 0..(1.0 / DT) as usize {
            watch.advance(&cannon, DT);
        }
        assert!(watch.bonus().is_none(), "gone after three seconds");
    }

    #[test]
    fn a_reload_clears_the_recoloured_pips() {
        let mut watch = ChainWatch::default();
        let paid = Awards {
            first_five: true,
            ..Awards::default()
        };
        watch.advance(&after(5, paid), DT);
        assert_ne!(watch.tinted, 0);
        watch.advance(&Cannon::new(), DT);
        assert_eq!(watch.tinted, 0);
    }
}
