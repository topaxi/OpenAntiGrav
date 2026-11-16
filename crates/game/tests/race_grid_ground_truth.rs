//! Where the grid and the start pose sit on the real circuit, out of a real disc image.
//!
//! `#[ignore]`d and never run in CI; it needs a disc image. The shared loaders
//! live in `race_ground_truth.rs`.

use crate::race_ground_truth::{envelope, image, load, load_with_opponents};
use oag_core::math::Vec3;
use oag_gameplay::PlayerInputs;
use oag_raceplay::{self as race};

/// Where the original's own craft stands at the start of a Talon's Junction time
/// trial, read off `data/traces/talons-junction-standing-start.csv` at tick 0.
///
/// **Written in rather than read from the file, deliberately.** `data/` is
/// gitignored and derived captures under it have gone missing once already, taking
/// three documents' citations with them; a start pose is four numbers and the
/// point of this test is that they outlive the CSV. The heading is the recorded
/// craft basis, read left-up-forward - see `HANDOVER.md` on why the column names
/// cannot be taken at face value.
const CAPTURED_START: [f32; 3] = [6.07941, -50.06461, -195.9884];
const CAPTURED_FORWARD: [f32; 3] = [0.9998092, 0.004672341, -0.01896589];
const CAPTURED_LEFT: [f32; 3] = [-0.0188732, -0.01914353, -0.9996386];

/// The authored grid slot points where the original's craft points.
///
/// This is the check that says `Start Position` is read correctly rather than
/// merely parsed, and it is the *heading* that says it. Position cannot: the
/// authored slot and the pose a time trial starts from are 139.7 units apart, and
/// a distance that large is as consistent with a misread matrix as with a grid
/// laid out behind the line. A heading agreeing to a degree is not.
///
/// So the assertions are split by what each one can carry:
///
/// - **Heading, asserted tightly.** The slot's forward is within 1.2 degrees of
///   the craft's, and its left within 1.6 of the craft's left. Read the rows in
///   any other order, or take the recorded basis at its column names, and these
///   are tens of degrees out.
/// - **Position, asserted only in the slot's own frame**, where the separation
///   decomposes into 137.9 units *along* the slot's forward, 22.4 to its left and
///   0.8 up. Those are the three numbers a grid hypothesis has to explain; none
///   of them is asserted as a value, only in sign and rough scale, because what
///   lays out the other seven slots has not been read.
///
/// The vertical figure is the one to be most careful with: 0.8 is the gap between
/// two *authored* heights on a track that falls away, not a ride height. See
/// `oag_gameplay::spawn::Pose::from_start_position` for why the spawn takes its
/// height off the collision geometry instead.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_authored_slot_points_where_the_original_starts() {
    let Some(loaded) = load() else { return };
    let slot = loaded
        .setup
        .start_position
        .expect("16_Track authors a Start Position");

    let forward = Vec3::from_array(slot.forward);
    let left = Vec3::from_array(slot.left);
    let heading = forward
        .dot(Vec3::from_array(CAPTURED_FORWARD))
        .clamp(-1.0, 1.0)
        .acos()
        .to_degrees();
    let sideways = left
        .dot(Vec3::from_array(CAPTURED_LEFT))
        .clamp(-1.0, 1.0)
        .acos()
        .to_degrees();

    let offset = Vec3::from_array(CAPTURED_START) - Vec3::from_array(slot.position);
    println!(
        "the authored slot is {:.2} units from the original's start pose: {:.2} ahead, \
         {:.2} left, {:.2} up; heading agrees to {heading:.2} degrees and left to \
         {sideways:.2}",
        offset.length(),
        offset.dot(forward),
        offset.dot(left),
        offset.dot(Vec3::from_array(slot.up)),
    );

    assert!(
        heading < 3.0,
        "the authored slot faces {heading:.2} degrees away from where the original's craft \
         faces at the start line, so the matrix rows are not being read as left-up-forward"
    );
    assert!(
        sideways < 3.0,
        "the authored slot's left is {sideways:.2} degrees off the craft's left"
    );

    // Behind the line, not past it: the slot is upstream of where a time trial
    // begins, which is what a grid slot is and what a start-line marker is not.
    assert!(
        offset.dot(forward) > 50.0,
        "the original starts {:.2} units along the slot's own forward, which is not behind it",
        offset.dot(forward)
    );
    // And across the track from it, by about a track column rather than a
    // rounding error - the two are not the same slot.
    assert!(
        offset.dot(left).abs() > 5.0,
        "the original starts {:.2} units to the slot's side, so the two may be one slot",
        offset.dot(left)
    );
}

/// The reference capture is on `track.vex`, not `track_reversed.vex`.
///
/// **This is the `01_Track` trap one level down, and it had to be measured.** M3
/// settled the track by casting the recording's own positions against every
/// track's *collision geometry* and finding one where all of them land - see
/// `oag_raceplay::DEFAULT_TRACK`. A forward layout and its reverse share that
/// geometry, so that test could not tell the two apart, and something has to.
///
/// What made the question live rather than theoretical: `track_reversed.vex`'s
/// authored slot sits **2.2 units** from where the original's craft starts, `y`
/// agreeing to 0.0096, and **4.011 units** above its own collision surface
/// against a resting craft height of 4.002-4.009. Three coincidences pointing at
/// the reversed layout, against `track.vex`'s slot at 139.7 units. Only the
/// heading separates them, so the heading is what this asks.
///
/// It is not close: the recorded craft faces **along** `track.vex`'s tangent and
/// **against** the reversed variant's, by construction of the pair. So the
/// coincidences are exactly that - the two layouts' grids bracket one shared
/// start line, and the reversed grid's authored slot happens to land near the
/// forward grid's pole.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_captured_start_pose_faces_along_the_forward_layout() {
    let Some(image) = image() else { return };
    let forward = Vec3::from_array(CAPTURED_FORWARD);
    let at = Vec3::from_array(CAPTURED_START);

    let mut readings = Vec::new();
    for variant in ["track.vex", "track_reversed.vex"] {
        let name = format!(r"Data\Environments\16_Track\{variant}");
        let loaded = race::load(&race::Options {
            source: image.display().to_string(),
            track: Some(name.clone()),
            class: "VENOM".to_string(),
            ..race::Options::default()
        })
        .expect("loading the race");

        let (_, sample, _) = loaded
            .setup
            .spline
            .nearest(at)
            .expect("a resampled spline has samples");
        let tangent = Vec3::from_array(sample.tangent).normalize_or_zero();
        let along = forward.dot(tangent);
        println!("{variant}: the recorded craft faces the tangent at {along:.4}");
        readings.push((variant, along));
    }

    assert!(
        readings[0].1 > 0.9,
        "the capture faces {:.4} along track.vex's tangent, so the reference scenario is not          on the layout race::DEFAULT_TRACK names",
        readings[0].1
    );
    assert!(
        readings[1].1 < -0.9,
        "the capture faces {:.4} along track_reversed.vex's tangent, which should be the          negation of the forward layout's",
        readings[1].1
    );
}

/// A ship spawned on the authored slot is where the hover law wants it.
///
/// The spawn takes its height off the collision surface under the slot rather
/// than out of the slot's own `y`, and this is what that buys: both probes in
/// contact on the very first tick, on the track the reference scenario uses.
/// Asserting it here rather than trusting the arithmetic, because the authored
/// height ranges over six units across the shipped tracks and a spawn that only
/// works on one of them would still pass every unit test.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_ship_spawned_on_the_authored_slot_starts_in_contact() {
    let Some(loaded) = load() else { return };
    let handling = loaded.setup.handling;
    let colliders = loaded.setup.collision.clone();
    let slot = loaded
        .setup
        .start_position
        .expect("16_Track authors a Start Position");
    let mut race = race::Race::start(loaded.setup);

    let start = race.telemetry();
    println!(
        "spawned at {:?}, {:.3} above the slot's own authored y",
        start.position,
        start.position.y - slot.position[1]
    );

    // Facing where the slot says, still, after the pose has been through a
    // quaternion and back out of the body.
    let forward = race.ship().physics.body.forward();
    assert!(
        forward.dot(Vec3::from_array(slot.forward)) > 0.999,
        "the spawned ship faces {forward} and the slot says {:?}",
        slot.forward
    );

    // One tick, so the hover probes have run.
    let mut held = race::HeldButtons::new(0);
    race.tick(&PlayerInputs::single(held.snapshot()));
    assert_eq!(
        race.ship().physics.grounded,
        1.0,
        "both hover probes must reach the track from the authored slot, got {}: {}",
        race.ship().physics.grounded,
        race::describe(&race.telemetry())
    );

    // And the surface really is under it, at the height the spawn claims.
    let origin = start.position + Vec3::Y * 20.0;
    let hit = oag_physics::Raycaster::raycast(
        &colliders,
        oag_physics::Ray::new(origin, Vec3::NEG_Y, 80.0),
        None,
        false,
    )
    .expect("the authored slot has track under it");
    let above = start.position.y - hit.point.y;
    assert!(
        (above - race::spawn_height(&handling)).abs() < 0.01,
        "the ship spawned {above:.3} above the surface, wanted {:.3}",
        race::spawn_height(&handling)
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_whole_grid_lands_on_the_track() {
    let Some(loaded) = load_with_opponents() else {
        return;
    };
    let bound = envelope(&loaded);
    let colliders = loaded.setup.collision.clone();
    let race = race::Race::start(loaded.setup);

    assert_eq!(race.ship_count(), 8, "16_Track should field a full grid");
    let ships: Vec<_> = race.sim.world.ships.iter().filter(|s| s.active).collect();
    assert_eq!(ships.len(), 8);

    // The player is array index 0 and grid slot 8, so it is the rearmost. Every
    // opponent is ahead of it along its own forward axis.
    let player = ships[0].physics.body;
    let forward = player.forward();
    for (index, ship) in ships.iter().enumerate().skip(1) {
        let ahead = (ship.physics.body.position - player.position).dot(forward);
        assert!(
            ahead > 0.0,
            "ship {index} is {ahead:.2} along the player's forward, so it is behind them"
        );
    }

    // Two staggered columns, `GRID_COLUMN_OFFSET / 2` either side of the corridor
    // midpoint, the even slots on the node's side (`grid.md`). Two units: the
    // midpoint is read at the nearest resampled point, not the walked one.
    let off_midpoint = |p: Vec3| {
        let (_, s, _) = race.spline().nearest(p).expect("has samples");
        let lateral =
            (p - Vec3::from_array(s.pos)).dot(Vec3::from_array(s.lateral).normalize_or_zero());
        lateral - 0.5 * (s.ai_bound_left + s.ai_bound_right)
    };
    let side = off_midpoint(player.position).signum();
    for (index, ship) in ships.iter().enumerate() {
        let slot = if index == 0 { 8 } else { index as u8 };
        let want = if slot.is_multiple_of(2) { side } else { -side }
            * 0.5
            * oag_gameplay::GRID_COLUMN_OFFSET;
        let got = off_midpoint(ship.physics.body.position);
        assert!(
            (got - want).abs() < 2.0,
            "slot {slot} sits {got:.2} from the corridor midpoint, expected {want:.1}"
        );
    }

    // And every one of them is over the track rather than over a gap, inside a
    // wall or out in the scenery. Two independent checks, because either alone
    // passes somewhere a race never goes: near the driveable line, and with
    // collision geometry underneath.
    for (index, ship) in ships.iter().enumerate() {
        let position = ship.physics.body.position;
        let distance = race
            .spline()
            .distance_to(position)
            .expect("the track has samples");
        assert!(
            distance < bound,
            "ship {index} is {distance:.2} from the driveable line, \
             outside the {bound:.2} envelope"
        );
        let origin = position + Vec3::Y * 20.0;
        let ray = oag_physics::Ray::new(origin, Vec3::NEG_Y, 80.0);
        assert!(
            oag_physics::Raycaster::raycast(&colliders, ray, None, false).is_some(),
            "ship {index} has no collision surface under it"
        );
    }
}

/// Where the original's own eight craft sit on Talon's Junction's grid.
///
/// Read out of PPSSPP's memory on 2026-08-10 while the countdown held them in
/// place, slot 1 first: `psp-drive.py menu --single-race`, then the racer table
/// at `0x08b34420`. Fourteen samples agreed to a thousandth, so these are the
/// original's numbers rather than one noisy frame. Method and the rest of the
/// measurement: `docs/ghidra/functions/psp-pulse-usa/grid.md`.
///
/// Slot 1 is worth recognising: `(6.125, -50.064, -195.945)` is where a *time
/// trial* starts, which is the other end of the same grid.
const ORIGINAL_GRID: [[f32; 3]; 8] = [
    [6.125, -50.064, -195.945],
    [-13.537, -49.709, -175.897],
    [-33.428, -50.121, -195.817],
    [-53.161, -49.696, -175.727],
    [-72.998, -50.026, -195.615],
    [-92.777, -49.651, -175.494],
    [-112.565, -49.847, -195.362],
    [-132.304, -49.577, -175.223],
];

/// Our grid is the original's grid, slot for slot, on the same track.
///
/// The strongest check available on this layout, and much stronger than the
/// envelope test above: it compares against the thing itself rather than against
/// a bound. Every slot has to land, so a formation that is right on average and
/// wrong at one end fails here.
///
/// **One unit of tolerance, chosen, not measured**: the worst slot measures 0.62.
/// It was three, two of which were the raw node standing in for slot 8: the
/// original lays every slot `10` either side of the AI corridor's midpoint
/// (`grid.md`), and doing the same took the worst slot from 1.81 to 0.62.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn our_grid_is_the_originals_grid() {
    let Some(loaded) = load_with_opponents() else {
        return;
    };
    let race = race::Race::start(loaded.setup);
    assert_eq!(race.ship_count(), 8);

    // Array index 0 is the player and grid slot 8; indices 1..=7 are slots 1..=7.
    let slot_of = |index: usize| if index == 0 { 8 } else { index };
    let mut worst = 0.0f32;
    for (index, ship) in race
        .sim
        .world
        .ships
        .iter()
        .enumerate()
        .filter(|(_, ship)| ship.active)
    {
        let slot = slot_of(index);
        let want = Vec3::from_array(ORIGINAL_GRID[slot - 1]);
        let got = ship.physics.body.position;
        let error = (got - want).length();
        worst = worst.max(error);
        println!("slot {slot}: ours {got:?}, original {want:?}, {error:.3} apart");
        assert!(
            error < 1.0,
            "slot {slot} is {error:.3} from where the original puts it"
        );
    }
    println!("worst slot: {worst:.3} units");
}
