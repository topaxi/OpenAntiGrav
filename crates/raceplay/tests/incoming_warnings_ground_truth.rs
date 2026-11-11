//! An opponent's fire raises the viewer's incoming-weapon warning on a real Pulse circuit,
//! by the window `docs/ghidra/functions/psp-pulse-usa/hud-incoming-warnings.md` recovered.
//!
//! **`#[ignore]`d and never run in CI.** It needs a disc image:
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/^incoming_warnings_ground_truth::/)'
//! ```
//!
//! The player drives on the autopilot so the field spreads out round it. Every opponent
//! that holds nothing is handed the weapon under test, and each time one starts using it the
//! next tick's readout is checked against `oag_hud::incoming::window`: the weapon's icon is
//! lit exactly when the gap `progress(player) - progress(firer)` was inside the window.

use oag_gameplay::PlayerInputs;
use oag_hud::incoming::Side;
use oag_tables::weapons::Weapon;

/// Ticks one warning runs for, with a margin: 2.0 s at 60 Hz.
const RUN: usize = 130;
/// A gap this close to a window edge is not judged: the standings move under a fire.
const EDGE: f32 = 25.0;

fn progress(race: &oag_raceplay::Race, slot: usize) -> Option<f32> {
    race.sim.world.ships[slot].standing.progress
}

/// What an opponent's pickup looked like at the start of a tick.
#[derive(Clone, Copy, PartialEq)]
struct Use {
    weapon: Option<Weapon>,
    dropping: bool,
    cannon_rounds: u8,
}

impl Use {
    fn of(race: &oag_raceplay::Race, slot: usize) -> Self {
        let held = &race.sim.world.ships[slot].pickup;
        Self {
            weapon: held.weapon,
            dropping: held.is_dropping(),
            cannon_rounds: held.cannon_rounds,
        }
    }

    /// Whether the use of `weapon` began between `self` and `after`: a spent pickup, a mine
    /// cluster started, or a Cannon burst's first round.
    fn began(self, after: Self, weapon: Weapon) -> bool {
        self.weapon == Some(weapon)
            && match weapon {
                Weapon::Mine | Weapon::Bomb => !self.dropping && after.dropping,
                Weapon::Cannon => self.cannon_rounds == 0 && after.cannon_rounds > 0,
                _ => after.weapon.is_none(),
            }
    }
}

fn watch(weapon: Weapon) -> (usize, usize) {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return (usize::MAX, usize::MAX);
    };
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        seed: Some(1),
        ..oag_raceplay::Options::default()
    })
    .expect("loading the race");
    let mut race = oag_raceplay::Race::start(loaded.setup.clone());
    race.watch_laps(&loaded.hud, None);
    race.set_autopilot(true);
    race.set_fire_law(oag_raceplay::FireLaw::Ours);
    let (icon, low, high) = oag_hud::incoming::window(weapon).expect("a weapon with a window");

    let opponents = 1..usize::from(race.ship_count());
    let length = race.course().map(oag_race::Course::length);
    let (mut judged_in, mut judged_out) = (0, 0);
    // The next tick's check on a fire just seen: whether the icon must be lit, and the gap
    // it was judged at.
    let mut check: Option<(bool, f32)> = None;
    let mut quiet_until = 0;
    for tick in 0..6000 {
        for slot in opponents.clone() {
            let ship = &mut race.sim.world.ships[slot];
            if ship.pickup.weapon.is_none() {
                ship.pickup.weapon = Some(weapon);
            }
        }
        let before: Vec<_> = opponents
            .clone()
            .map(|slot| {
                (
                    Use::of(&race, slot),
                    progress(&race, 0),
                    progress(&race, slot),
                )
            })
            .collect();
        race.tick(&PlayerInputs::none());
        let lit = match icon.side() {
            Side::Rear => race.readout().incoming.rear,
            Side::Forward => race.readout().incoming.forward,
        };

        // A second use on this very tick restarts the timer, which makes this tick unlit again.
        let restarted = opponents
            .clone()
            .enumerate()
            .any(|(index, slot)| before[index].0.began(Use::of(&race, slot), weapon));
        if let Some((expected, gap)) = check.take().filter(|_| !restarted) {
            assert_eq!(
                lit,
                expected.then_some(icon),
                "{weapon:?} at tick {tick}: used from a gap of {gap}"
            );
        }
        // Every use begun this tick is judged together: one inside its window lights the icon
        // whatever another opponent's use on the same tick was, and an edge case decides nothing.
        let (mut inside, mut outside, mut unsure) = (None, None, false);
        for (index, slot) in opponents.clone().enumerate() {
            let (was, player, firer) = before[index];
            if !was.began(Use::of(&race, slot), weapon) {
                continue;
            }
            let (Some(player), Some(firer)) = (player, firer) else {
                unsure = true;
                continue;
            };
            let gap = oag_weapons::projectile::missile::wrapped_gap(
                player - firer,
                length.unwrap_or(0.0),
            );
            if gap > low + EDGE && gap < high - EDGE {
                inside = Some(gap);
            } else if gap < low - EDGE || gap > high + EDGE {
                outside = Some(gap);
            } else {
                unsure = true;
            }
        }
        if let Some(gap) = inside {
            judged_in += 1;
            check = Some((true, gap));
            quiet_until = tick + RUN;
        } else if unsure {
            // A use the test cannot judge may have lit the icon: no unlit check until it clears.
            quiet_until = tick + RUN;
        } else if let Some(gap) = outside.filter(|_| tick >= quiet_until) {
            judged_out += 1;
            check = Some((false, gap));
        }
    }
    println!("{weapon:?}: judged {judged_in} inside the window, {judged_out} outside");
    (judged_in, judged_out)
}

fn assert_judged(weapon: Weapon) {
    let (inside, _) = watch(weapon);
    assert!(
        inside > 0,
        "{weapon:?} was never used from inside its window; nothing was shown to light"
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_quake_from_behind_lights_the_rear_icon() {
    assert_judged(Weapon::Quake);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_cannon_burst_from_behind_lights_the_rear_icon() {
    assert_judged(Weapon::Cannon);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_mine_dropped_ahead_lights_the_forward_icon() {
    assert_judged(Weapon::Mine);
}
