//! A Detonator cannon round rides HD's Talon's Junction loop as the original's does.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not
//! ship.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/^detonator_loop_ground_truth::/)'
//! ```
//!
//! The original's round was polled live on RPCS3 through the whole loop (the craft set at the
//! loop's foot, one volley; `scripts/rpcs3-hd-pulse-round-poll.py --craft-at`,
//! `docs/ghidra/functions/ps3-hdfury-eu/detonator-cannon.md`, "Through the loop"). Its state at
//! age `1/30 s` seeds ours' round in the same place; the rows below are the original's position,
//! up row and drawn forward row at later ages, all on ours' 60 Hz grid.

use oag_core::math::Vec3;
use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

const TRACK: &str = r"Data\Environments\Talons_Junction\track.vex";

/// The original's round at age `0.0333`: position, up row (= surface normal), velocity.
const START: ([f32; 3], [f32; 3], [f32; 3]) = (
    [-259.55, -73.597, 55.526],
    [0.0184, 0.9994, -0.0279],
    [-499.8427, 9.4301, 8.268],
);

/// `(ticks of ours after the start, position, up row, forward row)` read off the original at
/// ages `0.2`, `0.3667` (climbing), `0.4667` (vertical), `0.6`, `0.7333` (inverted) and `0.8667`
/// (coming down), before its side-wall rebound at `0.9`.
const ALONG: [(u32, [f32; 3], [f32; 3], [f32; 3]); 6] = [
    (
        10,
        [-340.708, -61.757, 56.81],
        [0.3158, 0.9488, -0.0108],
        [-0.9487, 0.3159, 0.0129],
    ),
    (
        20,
        [-404.426, -15.66, 59.482],
        [0.8964, 0.4238, 0.1296],
        [-0.4355, 0.8966, 0.0804],
    ),
    (
        26,
        [-415.639, 30.912, 65.454],
        [0.9837, -0.0133, 0.1791],
        [-0.0156, 0.9872, 0.1587],
    ),
    (
        34,
        [-395.568, 88.719, 80.148],
        [0.7166, -0.6848, 0.1326],
        [0.6327, 0.7182, 0.2895],
    ),
    (
        42,
        [-342.266, 112.946, 101.753],
        [-0.0115, -0.9999, -0.0021],
        [0.9338, -0.0115, 0.3576],
    ),
    (
        50,
        [-287.529, 90.977, 125.033],
        [-0.6745, -0.7352, -0.0672],
        [0.6759, -0.6516, 0.3444],
    ),
];

/// **Through the loop.** Seeded with the original's round at the loop's foot, ours' round climbs,
/// goes inverted and comes down the other side on the original's path: its up row within `3`
/// degrees of the original's (the surface normal; measured under `0.1`), its heading within `3`
/// (measured `2.1` at most) and its position within `6` units. The position drifts sideways, `0.05`
/// units at age `0.2` growing to `4.8` at `0.87`; ours steps at 60 Hz and the original at 30 Hz,
/// the likely cause, not proven. A round that kept its launch up, or flew straight with no ride,
/// leaves the loop at its foot.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_cannon_round_rides_the_talons_junction_loop_as_the_original_does() {
    let Some(path) = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: path.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::Detonator,
        track: Some(TRACK.to_string()),
        ..race::Options::default()
    })
    .expect("loading an HD Detonator race on Talon's Junction");
    let mut race = race::Race::start(loaded.setup, loaded.view, loaded.modules);
    while oag_race::RaceState::thrust_gated(race.core.sim.world.tick) {
        race.tick(&PlayerInputs::none());
    }
    race.core.sim.world.detonator.live_mines = 0;
    let (pos, up, vel) = (
        Vec3::from_array(START.0),
        Vec3::from_array(START.1),
        Vec3::from_array(START.2),
    );
    race.core.sim.world.detonator.cannon.rounds[0] = oag_weapons::detonator::cannon::Round {
        live: true,
        pos,
        prev: pos,
        vel,
        normal: up,
        heading: vel.normalize(),
        age: 1.0 / 30.0,
        rebounds: 1,
        ..Default::default()
    };
    let mut ticks = 0;
    let degrees = |a: Vec3, b: Vec3| {
        a.normalize()
            .dot(b.normalize())
            .clamp(-1.0, 1.0)
            .acos()
            .to_degrees()
    };
    for (at, want_pos, want_up, want_fwd) in ALONG {
        while ticks < at {
            race.tick(&PlayerInputs::none());
            race.core.sim.world.detonator.live_mines = 0;
            ticks += 1;
        }
        let round = race.core.sim.world.detonator.cannon.rounds[0];
        assert!(round.live, "the round died by tick {at}");
        let off = round.pos.distance(Vec3::from_array(want_pos));
        let up_err = degrees(round.normal, Vec3::from_array(want_up));
        let fwd_err = degrees(round.heading, Vec3::from_array(want_fwd));
        eprintln!(
            "tick {at}: pos {:?} off {off:.2} up {up_err:.2} deg fwd {fwd_err:.2} deg",
            round.pos
        );
        assert!(off < 6.0, "tick {at}: {off} units from the original");
        assert!(up_err < 3.0, "tick {at}: up {up_err} degrees off");
        assert!(fwd_err < 3.0, "tick {at}: heading {fwd_err} degrees off");
    }
}
