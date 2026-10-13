//! The `original` tier's per-tick hull gather against the per-frame one, on a real
//! circuit, between ticks.
//!
//! `#[ignore]`d: needs a disc image. `OAG_REQUIRE_GAME_DATA=1 cargo nextest run
//! -p oag-raceplay --run-ignored all hull_gather`.
//!
//! A draw between two ticks used to project the hull from the blended pose and cast
//! a ray per vertex from there. The cache takes the gather at each tick's own pose
//! and lerps the projected vertices (`oag_raceplay::hull_gather`), which is not the
//! same picture between ticks. This measures how far apart they are, in world
//! units, vertex by vertex, over three stretches of Zone's circuit at three alphas.

use oag_core::math::Vec3;
use oag_gameplay::PlayerInputs;
use oag_raceplay::hull_gather::HullGather;
use oag_raceplay::{self as race};

/// Ticks past the warm-up each window covers.
const WINDOW: u64 = 60;

/// Where each window starts, in ticks after the race began: Zone's start, a lap in
/// at Zone's own pace, and later still (the stretches `shadow_stability` uses).
const WARM: [u64; 3] = [900, 3000, 6000];

/// The alphas a 240 Hz window draws inside one tick.
const ALPHAS: [f32; 3] = [0.25, 0.5, 0.75];

/// How far the lerped hull may sit from the one projected at the blended pose, in
/// world units: 99 percent of vertices within [`P99_WITHIN`], every one within
/// [`WORST_WITHIN`].
///
/// **Chosen, not measured**, set past what Zone's circuit reads (2026-10-13:
/// median 0.0006, p90 0.013, p99 0.053, p99.9 0.147, worst 0.237 over 168,480
/// vertices). A craft is some 13 units long and the chase camera is about 20 away,
/// so a pixel is about 0.03 units: the p99 is under two pixels and the worst
/// vertex, a floor seam under one vertex's ray, under eight.
const P99_WITHIN: f32 = 0.1;
const WORST_WITHIN: f32 = 0.3;

#[test]
#[ignore = "needs a real disc image under data/images/"]
fn the_lerped_hull_agrees_with_the_one_projected_at_the_blended_pose() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::Zone,
        ..race::Options::default()
    })
    .expect("loading the race");
    let hulls = loaded.shadow_hulls.clone();
    let mut race = race::Race::start(loaded.setup);
    race.set_autopilot(true);

    let mut worst = 0.0f32;
    let mut errors: Vec<f32> = Vec::new();
    let (mut compared, mut unblendable) = (0usize, 0usize);
    let mut ticks = 0;
    for warm in WARM {
        while ticks < warm {
            race.tick(&PlayerInputs::none());
            ticks += 1;
        }
        for _ in 0..WINDOW {
            let previous = HullGather::at_tick(&race, &hulls);
            race.tick(&PlayerInputs::none());
            ticks += 1;
            let current = HullGather::at_tick(&race, &hulls);
            for alpha in ALPHAS {
                race.set_draw_alpha(alpha);
                let live = HullGather::at_draw(&race, &hulls);
                let blended = HullGather::blend(&race, &previous, &current, alpha);
                race.set_draw_alpha(1.0);
                let Some(blended) = blended else {
                    unblendable += 1;
                    continue;
                };
                if blended.len() != live.len() {
                    unblendable += 1;
                    continue;
                }
                compared += 1;
                for (a, b) in blended.iter().zip(&live) {
                    let error = Vec3::from_array(a.position).distance(Vec3::from_array(b.position));
                    errors.push(error);
                    worst = worst.max(error);
                }
            }
        }
    }
    errors.sort_by(f32::total_cmp);
    let at = |q: f32| errors[((errors.len() - 1) as f32 * q) as usize];
    println!(
        "vertex distance over {} vertices: median {:.4}, p90 {:.4}, p99 {:.4}, p99.9 {:.4}",
        errors.len(),
        at(0.5),
        at(0.9),
        at(0.99),
        at(0.999)
    );
    println!(
        "{compared} frames compared, {unblendable} drawn live instead; worst vertex distance {worst:.4}"
    );
    assert!(
        compared > 100,
        "only {compared} frames had a hull to compare"
    );
    assert!(
        at(0.99) <= P99_WITHIN,
        "p99 of the lerped hull's distance from the live one is {:.4}, past {P99_WITHIN}",
        at(0.99)
    );
    assert!(
        worst <= WORST_WITHIN,
        "the lerped hull is {worst:.4} units from the live one, past {WORST_WITHIN}"
    );
}
