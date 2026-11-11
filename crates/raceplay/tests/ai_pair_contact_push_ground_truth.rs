//! Two opponents on the original driver forced side by side part their lanes:
//! each pushes its lane target away from the other, the one with its rival on
//! its left toward `+`, the other toward `-` (`Ai_PlanLane`'s contact push,
//! kind-`1` records only; `docs/gameplay/ai.md`, "The two rules").
//!
//! The same race is run twice from the same seed: once with the second craft
//! moved into the first's flank on the side it already rides, once left alone.
//! The lane targets' movement over the next half second is compared between
//! the two, so the round robin's own lane choice cancels out.

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

const SEED: u64 = 3;
/// Ticks after GO before the contact is forced: the field has strung out.
const SETTLE: u64 = 600;
/// Ticks the targets are watched for after the contact.
const WATCH: u64 = 30;
/// How far apart the two centres are put, across the track: well inside the
/// two hulls' half-widths, so the overlap survives the rail's pull on the
/// first step.
const OFFSET: f32 = 0.3;
/// How fast the moved craft closes on the other, across the track: faster than
/// the rails pull the two apart on the first step, or the resolver's separating
/// gate (`oag_physics::pair::SEPARATING_LIMIT`) lets the pair go untouched.
const CLOSING: f32 = 15.0;

fn start() -> Option<race::Race> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "venom".to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Novice,
        seed: Some(SEED),
        ..race::Options::default()
    })
    .expect("loading");
    let mut race = race::Race::start(loaded.setup);
    assert!(race.use_original_ai(None, 0));
    let mut moving = 0;
    for _ in 0..SETTLE + 2000 {
        race.tick(&PlayerInputs::none());
        let racing = (1..race.ship_count() as usize).any(|s| {
            race.sim.world.ships[s]
                .physics
                .body
                .linear_velocity
                .length()
                > 5.0
        });
        moving += u64::from(racing || moving > 0);
        if moving == SETTLE {
            return Some(race);
        }
    }
    panic!("the field never left the grid");
}

fn targets(race: &race::Race, a: usize, b: usize) -> (f32, f32) {
    let field = race.original_ai_field().expect("original field");
    (
        field.drivers[a].planner.target,
        field.drivers[b].planner.target,
    )
}

/// Moves `b` into `a`'s flank on the side `b` already rides, at `a`'s pose and
/// speed, closing. `+1.0` when that is `a`'s left.
fn force_side_by_side(race: &mut race::Race, a: usize, b: usize) -> f32 {
    let body = race.sim.world.ships[a].physics.body;
    let left = -body.right();
    let across = left.dot(race.sim.world.ships[b].physics.body.position - body.position);
    let side = if across >= 0.0 { 1.0 } else { -1.0 };
    let ship = &mut race.sim.world.ships[b];
    ship.physics.body.position = body.position + left * (side * OFFSET);
    ship.physics.body.orientation = body.orientation;
    ship.physics.body.linear_velocity = body.linear_velocity - left * (side * CLOSING);
    ship.physics.body.angular_velocity = body.angular_velocity;
    side
}

#[test]
#[ignore = "needs a Pulse PSP disc image under data/images/"]
fn two_opponents_forced_side_by_side_push_their_lanes_apart() {
    let Some(mut forced) = start() else {
        return;
    };
    let mut control = start().expect("second load");
    let (a, b) = (1, 2);
    let before = targets(&forced, a, b);
    assert_eq!(before, targets(&control, a, b), "the two runs diverged");
    let contacts_before = forced.pair_contact_ticks_of(a);

    let side = force_side_by_side(&mut forced, a, b);
    for _ in 0..WATCH {
        forced.tick(&PlayerInputs::none());
        control.tick(&PlayerInputs::none());
    }
    let contact_ticks = forced.pair_contact_ticks_of(a) - contacts_before;
    assert!(
        contact_ticks > 0,
        "the forced placement never made a pair contact"
    );
    let (fa, fb) = targets(&forced, a, b);
    let (ca, cb) = targets(&control, a, b);
    // Each target's move beyond the undisturbed run's, signed so that away
    // from the rival is positive: `a`'s rival is on `side`, `b`'s on the other.
    let a_away = side * ((fa - before.0) - (ca - before.0));
    let b_away = -side * ((fb - before.1) - (cb - before.1));
    println!(
        "rival on a's {}: a {a} target {:.3} -> {fa:.3} (control {ca:.3}), b {b} {:.3} -> {fb:.3} \
         (control {cb:.3}), {contact_ticks} contact ticks",
        if side > 0.0 { "left" } else { "right" },
        before.0,
        before.1,
    );
    assert!(
        a_away > 0.1,
        "craft {a} did not push its lane away from its rival: {a_away:+.3}"
    );
    assert!(
        b_away > 0.1,
        "craft {b} did not push its lane away from its rival: {b_away:+.3}"
    );
}
