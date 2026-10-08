//! Wipeout HD's craft hovers low on the grid and rises after the green light.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not
//! ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test hd_grid_hover_ground_truth --run-ignored all
//! ```
//!
//! The original's hover target is `0.75 * min(5.5, entry+0x348)` with `+0x348` a `3.0x` clamp on
//! the grid and a seconds timer from GO (`docs/ghidra/functions/ps3-hdfury-eu/hover-target.md`).
//! Measured on RPCS3, Time Trial, Venom, Talons Junction slot 0, three boots
//! (`docs/physics/hd-ride-height.md`): the craft's height above the floor (`y + 54.080`) reads
//! `2.13-2.19` from the flyby through the countdown, `3.20` 1.4 s after GO, `4.00` from about
//! 2.5 s, and `4.00` at rest. Pulse carries no such clamp and is unchanged.

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;
use oag_raceplay::Race;

const TRACK: &str = r"Data\Environments\Talons_Junction\track.vex";
/// The floor under grid slot 0, read off the original's own height field (flyby and grid agree).
const FLOOR_Y: f32 = -54.080;
const GO: u64 = oag_race::COUNTDOWN_TICKS;

fn started() -> Option<Race> {
    let image = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::TimeTrial,
        track: Some(TRACK.to_string()),
        ..race::Options::default()
    })
    .expect("loading the race");
    Some(Race::start(loaded.setup))
}

fn height(race: &Race) -> f32 {
    race.ship().physics.body.position.y - FLOOR_Y
}

fn run_to(race: &mut Race, tick: u64) {
    while race.sim.world.tick < tick {
        race.tick(&PlayerInputs::none());
    }
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_craft_starts_and_waits_on_the_grid_at_the_lowered_hover() {
    let Some(mut race) = started() else { return };
    let placed = height(&race);
    assert!((2.10..2.22).contains(&placed), "placement {placed}");
    run_to(&mut race, GO - 2);
    let end = height(&race);
    assert!((2.10..2.22).contains(&end), "end of the countdown {end}");
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_craft_rises_along_the_released_target_and_rests_at_the_race_height() {
    let Some(mut race) = started() else { return };
    run_to(&mut race, GO + 84);
    let rising = height(&race);
    assert!((3.1..3.3).contains(&rising), "1.4 s after GO {rising}");
    run_to(&mut race, GO + 300);
    let rest = height(&race);
    assert!((3.93..4.07).contains(&rest), "5 s after GO {rest}");
}
