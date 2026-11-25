//! HD's Plasma and Rocket ride Moa Therma's steep bank the way the original's do: their floor
//! arm turns the velocity parallel to the floor, where Pulse's steers it at the ride point.
//!
//! **`#[ignore]`d and never run in CI**: needs `data/images/hdfury-ps3-eu-dec.iso`.
//!
//! The craft is posed on the racing line of spline sample 493, 3 units up, 140 units/s along
//! the sample's tangent, and held there until the shot flies. That is the pose the original was
//! fired from on RPCS3, where the Plasma rode the bank for 1.86 s and ended at
//! `(227.3, 87.8, -315.7)` (sample 774, lateral 29.2), and all three Rockets passed sample 668
//! (`docs/ghidra/functions/ps3-hdfury-eu/plasma.md`, "On Moa Therma's bank"). Steering at the
//! ride point instead slid the bolt down the bank and off its left edge by sample 590, and put
//! every Rocket into the bank's lower wall by sample 560.

use oag_core::math::Vec3;
use oag_gameplay::PlayerInputs;
use oag_gameplay::input::{Button, Input};
use oag_raceplay as race;
use oag_tables::weapons::Weapon;

const SAMPLE: usize = 493;

fn start() -> Option<race::Race> {
    let image = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        track: Some("/data/environments/03_track/track.vex".to_string()),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        opponent_teams: Vec::new(),
        ..race::Options::default()
    })
    .expect("loading Moa Therma");
    Some(race::Race::start(loaded.setup, loaded.view, loaded.modules))
}

/// Where one shot went: per slot, the furthest spline sample it reached while within the
/// track's edges, and where it ended.
struct Path {
    furthest: Vec<usize>,
    ends: Vec<Vec3>,
    left_the_track: bool,
}

fn fire_onto_the_bank(mut race: race::Race, weapon: Weapon) -> Path {
    let sample = *race.core.sim.spline().sample(SAMPLE).expect("sample 493");
    let pose = oag_gameplay::spawn::Pose::from_sample(&sample, sample.racing_line, 3.0);
    let place = |race: &mut race::Race| {
        let craft = &mut race.core.sim.world.ships[0];
        craft.place_at(pose);
        craft.physics.body.linear_velocity = craft.physics.body.forward() * 140.0;
    };
    place(&mut race);
    let mut input = Input::new();
    let mut seen: Vec<Option<(usize, Vec3)>> =
        vec![None; race.core.sim.world.projectiles.slots.len()];
    let mut furthest = vec![0; seen.len()];
    let mut left_the_track = false;
    for tick in 0..400 {
        let fire = tick == 2;
        if fire {
            race.core.sim.world.ships[0].pickup.weapon = Some(weapon);
        }
        input.begin_frame(Button::Cross.bit() | if fire { Button::Square.bit() } else { 0 });
        let snap = oag_gameplay::InputSnapshot {
            buttons: input,
            ..oag_gameplay::InputSnapshot::new()
        };
        race.tick(&PlayerInputs::single(snap));
        let slots = race.core.sim.world.projectiles.slots;
        if tick < 2
            || slots
                .iter()
                .any(|p| p.kind == Some(weapon) && p.charge > 0.0)
        {
            place(&mut race);
            continue;
        }
        let spline = race.core.sim.spline();
        for (slot, p) in slots.iter().enumerate() {
            if p.kind != Some(weapon) || p.owner != 0 {
                continue;
            }
            let (index, s, _) = spline.nearest(p.position).expect("a sample");
            let lateral = (p.position - Vec3::from_array(s.pos))
                .dot(Vec3::from_array(s.lateral).normalize_or_zero());
            if lateral < -s.half_width_left - 2.0 || lateral > s.half_width_right + 2.0 {
                left_the_track = true;
            }
            furthest[slot] = furthest[slot].max(index);
            seen[slot] = Some((index, p.position));
        }
        if tick > 10 && slots.iter().all(|p| p.kind != Some(weapon)) {
            break;
        }
    }
    let fired: Vec<usize> = (0..seen.len()).filter(|&i| seen[i].is_some()).collect();
    Path {
        furthest: fired.iter().map(|&i| furthest[i]).collect(),
        ends: fired.iter().map(|&i| seen[i].unwrap().1).collect(),
        left_the_track,
    }
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn an_hd_plasma_rides_moa_thermas_bank_to_where_the_originals_ended() {
    let Some(race) = start() else { return };
    let path = fire_onto_the_bank(race, Weapon::Plasma);
    assert_eq!(path.ends.len(), 1, "one bolt");
    assert!(
        !path.left_the_track,
        "the bolt left the track: {:?}",
        path.ends
    );
    let original = Vec3::new(227.3, 87.8, -315.7);
    assert!(
        path.furthest[0] >= 760 && (path.ends[0] - original).length() < 10.0,
        "the bolt reached sample {} and ended at {:?}; the original's rode to sample 774 and ended at {original:?}",
        path.furthest[0],
        path.ends[0]
    );
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hd_rockets_ride_moa_thermas_bank_past_sample_668() {
    let Some(race) = start() else { return };
    let path = fire_onto_the_bank(race, Weapon::Rocket);
    assert_eq!(path.ends.len(), 3, "a volley of three");
    for (furthest, end) in path.furthest.iter().zip(&path.ends) {
        assert!(
            *furthest >= 668,
            "a rocket ended at sample {furthest} ({end:?}); the original's three all passed 668"
        );
    }
}
