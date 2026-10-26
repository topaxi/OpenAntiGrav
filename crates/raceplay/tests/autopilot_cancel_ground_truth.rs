//! The Autopilot pickup's cancel on a real disc, per title: absorb cancels it where the title
//! has a cancel, fire never does, and nothing is granted from a pad while it runs.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not
//! ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/^autopilot_cancel_ground_truth::/)'
//! ```
//!
//! What only a disc can say: that a title's `Weapons::autopilot_absorb_cancels` reaches the
//! race through the loader, against the weapon table the disc ships (the headless tests set
//! the field on the simulation directly), and that the sequence holds on a real craft under
//! the real force law. The run prints one row per tick of the window it asserts on.

use oag_gameplay::PlayerInputs;
use oag_gameplay::input::{Button, Input};
use oag_tables::weapons::Weapon;

const CROSS: u32 = Button::Cross.bit();
const SQUARE: u32 = Button::Square.bit();
const CIRCLE: u32 = Button::Circle.bit();

/// Edge-correct input: one `begin_frame` per tick, so a button held two ticks is one press.
struct Pad(Input);

impl Pad {
    fn tick(&mut self, mask: u32) -> PlayerInputs {
        self.0.begin_frame(mask);
        PlayerInputs::single(oag_gameplay::InputSnapshot {
            buttons: self.0,
            ..oag_gameplay::InputSnapshot::new()
        })
    }
}

fn started(source: String) -> oag_raceplay::Race {
    let loaded = oag_raceplay::load(&oag_raceplay::Options {
        source,
        mode: oag_race::Mode::SingleRace,
        seed: Some(1),
        ..oag_raceplay::Options::default()
    })
    .expect("loading the race");
    oag_raceplay::Race::start(loaded.setup)
}

/// One row of the trace: the press, the timer, whether the pilot has the craft, the slot.
fn row(race: &oag_raceplay::Race, label: &str, mask: u32) {
    let ship = &race.sim.world.ships[0];
    println!(
        "tick {:>4}  {label:<8} buttons {mask:#06x}  timer {:>7.3}  flown {:<5}  slot {:?}",
        race.sim.world.tick,
        ship.autopilot_timer,
        race.flown_for_the_player(0),
        ship.pickup.weapon,
    );
}

/// Arms the Autopilot, presses fire into it, presses absorb, and returns whether it was still
/// flown on the tick after the absorb.
///
/// The asserts that hold on every title are inside: fire never cancels and never re-arms, a
/// held Rocket survives a fire press, and no projectile leaves the craft.
fn flown_after_absorb(source: String) -> bool {
    let mut race = started(source);
    let mut pad = Pad(Input::new());
    for _ in 0..oag_race::COUNTDOWN_TICKS + 60 {
        race.tick(&pad.tick(CROSS));
    }

    race.sim.world.ships[0].pickup.weapon = Some(Weapon::Autopilot);
    race.tick(&pad.tick(CROSS | SQUARE));
    row(&race, "arm", CROSS | SQUARE);
    assert!(race.flown_for_the_player(0), "the Autopilot did not arm");
    assert_eq!(race.sim.world.ships[0].pickup.weapon, None);
    race.tick(&pad.tick(CROSS));
    row(&race, "release", CROSS);

    let before = race.sim.world.ships[0].autopilot_timer;
    race.sim.world.ships[0].pickup.weapon = Some(Weapon::Rocket);
    race.tick(&pad.tick(CROSS | SQUARE));
    row(&race, "fire", CROSS | SQUARE);
    assert!(race.flown_for_the_player(0), "a fire press cancelled it");
    assert_eq!(
        race.sim.world.ships[0].pickup.weapon,
        Some(Weapon::Rocket),
        "a fire press under the Autopilot fired the held pickup"
    );
    assert!(
        race.sim.world.ships[0].autopilot_timer < before,
        "the timer did not run on"
    );
    assert_eq!(
        race.sim.world.projectiles.live(),
        0,
        "a projectile left the craft"
    );

    race.sim.world.ships[0].pickup.weapon = Some(Weapon::Autopilot);
    race.tick(&pad.tick(CROSS));
    race.tick(&pad.tick(CROSS | SQUARE));
    row(&race, "refire", CROSS | SQUARE);
    assert_eq!(
        race.sim.world.ships[0].pickup.weapon,
        Some(Weapon::Autopilot),
        "a held Autopilot was fired into a running one"
    );
    race.sim.world.ships[0].pickup.weapon = None;

    race.tick(&pad.tick(CROSS));
    row(&race, "release", CROSS);
    race.tick(&pad.tick(CROSS | CIRCLE));
    row(&race, "absorb", CROSS | CIRCLE);
    race.tick(&pad.tick(CROSS));
    row(&race, "after", CROSS);
    race.flown_for_the_player(0)
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulse_cancels_on_absorb_and_not_on_fire() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    assert!(
        !flown_after_absorb(image.display().to_string()),
        "Pulse's absorb did not cancel the Autopilot"
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pure_has_no_cancel() {
    let Some(image) = oag_testdata::image("data/images/pure-psp-usa.chd") else {
        return;
    };
    assert!(
        flown_after_absorb(image.display().to_string()),
        "Pure's absorb cancelled an Autopilot its executable cannot cancel"
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn hd_cancels_on_absorb_and_not_on_fire() {
    let Some(image) = oag_testdata::image("hdfury-ps3-eu-dec.iso") else {
        return;
    };
    assert!(
        !flown_after_absorb(image.display().to_string()),
        "HD's absorb did not cancel the Autopilot"
    );
}
