//! Pilot Assist keeps a new player off HD's walls, and only the player.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not
//! ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test pilot_assist_ground_truth --run-ignored all --no-capture
//! ```
//!
//! The player a new one is: thrust held from the start, the stick never touched. With the
//! option off that craft meets Talon's Junction's first wall and scrapes along the rest; with
//! it on HD's own `<PilotAssist>` rung yaws it round, as the RPCS3 capture on
//! `docs/ghidra/functions/ps3-hdfury-eu/pilot-assist.md` shows the original doing. The law
//! is `oag_physics::pilot_assist`; the numbers are the disc's.

use oag_core::buttons::{Button, Input};
use oag_gameplay::{InputSnapshot, PlayerInputs};
use oag_raceplay as race;
use oag_raceplay::Race;

const IMAGE: &str = "data/images/hdfury-ps3-eu-dec.iso";
/// The countdown and about 25 seconds of racing.
const TICKS: u64 = 1800;

fn started(mode: oag_race::Mode) -> Option<Race> {
    let image = oag_testdata::image(IMAGE)?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode,
        weapons_override: Some(false),
        track: Some(oag_hd::race::DEFAULT_TRACK.to_string()),
        ..race::Options::default()
    })
    .expect("loading the race");
    Some(Race::start(loaded.setup))
}

fn thrust() -> PlayerInputs {
    let mut buttons = Input::new();
    buttons.begin_frame(Button::Cross.bit());
    PlayerInputs::single(InputSnapshot {
        buttons,
        ..InputSnapshot::EMPTY
    })
}

/// Wall-contact ticks, separate contacts, and how far round the lap the craft got.
fn fly(race: &mut Race) -> (u32, u32, f32) {
    let inputs = thrust();
    let (mut ticks, mut contacts) = (0, 0);
    let mut touching = false;
    for _ in 0..TICKS {
        race.tick(&inputs);
        let now = race.sim.world.ships[0].physics.wall_contact_prev;
        ticks += u32::from(now);
        if now && !touching {
            contacts += 1;
        }
        touching = now;
    }
    let progress = race.sim.world.ships[0]
        .standing
        .distance(race.course().expect("a course"));
    (ticks, contacts, progress)
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn a_new_player_meets_fewer_walls_with_pilot_assist_on() {
    let (Some(mut off), Some(mut on)) = (
        started(oag_race::Mode::TimeTrial),
        started(oag_race::Mode::TimeTrial),
    ) else {
        return;
    };
    assert!(on.pilot_assist_available(), "HD authors <PilotAssist>");
    on.set_pilot_assist(true);
    let (off_ticks, off_contacts, off_progress) = fly(&mut off);
    let (on_ticks, on_contacts, on_progress) = fly(&mut on);
    println!(
        "thrust only, {TICKS} ticks: off {off_contacts} contacts / {off_ticks} contact ticks / \
         {off_progress:.0} units; on {on_contacts} contacts / {on_ticks} contact ticks / \
         {on_progress:.0} units"
    );
    assert!(off_contacts > 0, "the unassisted craft must meet a wall");
    assert!(
        on_contacts < off_contacts && on_ticks < off_ticks,
        "assist on {on_contacts} contacts / {on_ticks} ticks, off {off_contacts} / {off_ticks}"
    );
    assert!(
        on_progress > off_progress,
        "assisted {on_progress:.0} units against {off_progress:.0}"
    );
}

/// The maintainer's rule: the AI flies the player's physics, and Pilot Assist is not part
/// of them.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn no_opponent_ever_gets_the_assist() {
    let Some(mut race) = started(oag_race::Mode::SingleRace) else {
        return;
    };
    race.set_pilot_assist(true);
    let inputs = thrust();
    for _ in 0..600 {
        race.tick(&inputs);
    }
    assert_ne!(
        race.sim.world.ships[0].physics.pilot_assist,
        Default::default(),
        "the player's assist ran"
    );
    for slot in 1..race.sim.world.ship_count as usize {
        assert_eq!(
            race.sim.world.ships[slot].physics.pilot_assist,
            Default::default(),
            "slot {slot}"
        );
    }
}
