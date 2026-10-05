//! Where and when a field splits at a fork: for each fork of a circuit, the
//! tick at which the most craft are just past it with both sides in use, and
//! a `--camera-pose` that looks down on it. Scratch tool for the ai-forks lane.
//!
//! ```sh
//! cargo run -p oag-game --example fork_probe -- <source> <track entry> [ticks]
//! ```

use oag_core::math::Vec3;
use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

/// The best moment seen at one fork: score, tick, craft on the ring, on a route.
type Best = (usize, u32, usize, usize);

fn main() {
    let mut args = std::env::args().skip(1);
    let source = args.next().expect("source");
    let track = args.next().expect("track entry");
    let ticks: u32 = args.next().map_or(60 * 120, |t| t.parse().expect("ticks"));
    let loaded = race::load(&race::Options {
        source,
        mode: oag_race::Mode::SingleRace,
        track: Some(track),
        ..race::Options::default()
    })
    .expect("loading");
    let mut race = race::Race::start(loaded.setup);
    race.set_autopilot(true);
    let course = race.course().expect("a ring").clone();
    let count = race.ship_count() as usize;
    let n = course.len();
    // Per fork (pre-fork path, split): best (score, tick, ring, route).
    let mut best: Vec<(u16, usize, Best)> = Vec::new();
    for route in course.routes() {
        if best.iter().all(|(p, _, _)| *p != route.pre_fork) {
            best.push((route.pre_fork, route.split, (0, 0, 0, 0)));
        }
    }
    for tick in 0..ticks {
        race.tick(&PlayerInputs::none());
        for (_, split, top) in &mut best {
            let Some(at) = course.position(*split) else {
                continue;
            };
            let (mut ring, mut routed) = (0, 0);
            for slot in 0..count {
                let ship = &race.sim.world.ships[slot];
                if (ship.physics.body.position - at).length() > 500.0 {
                    continue;
                }
                // Past the fork: ahead of it along the ring's tangent there.
                let ahead = (ship.physics.body.position - at)
                    .dot(course.tangent(*split).unwrap_or(Vec3::X));
                if ahead < 20.0 {
                    continue;
                }
                if ship.driver.branching.route == 0 {
                    ring += 1;
                } else {
                    routed += 1;
                }
            }
            let score = ring.min(routed) * 10 + ring + routed;
            if ring > 0 && routed > 0 && score > top.0 {
                *top = (score, tick + 1, ring, routed);
            }
        }
    }
    for (pre, split, (score, tick, ring, routed)) in &best {
        let at = course.position(*split).unwrap_or(Vec3::ZERO);
        let tangent = course.tangent(*split).unwrap_or(Vec3::X);
        let route = course
            .routes()
            .iter()
            .find(|r| r.pre_fork == *pre)
            .expect("a route");
        // Over the first third of the split: the ring and the route there.
        let ring_mid = course
            .position((*split + route.len() / 4) % n)
            .unwrap_or(at);
        let route_mid = route.positions[route.len() / 4];
        let mid = (ring_mid + route_mid) * 0.5;
        let spread = (ring_mid - route_mid).length();
        let eye = mid + Vec3::Y * (250.0 + spread * 1.5) - tangent * 60.0;
        let fwd = (mid - eye).normalize_or_zero();
        let up = tangent;
        println!(
            "fork after path {pre}: split {split} at {at:?}; best tick {tick} with {ring} on the ring and {routed} on a route (score {score}), sides {spread:.0} apart"
        );
        println!(
            "  --camera-pose {:.1},{:.1},{:.1},{:.3},{:.3},{:.3},{:.3},{:.3},{:.3}",
            eye.x, eye.y, eye.z, fwd.x, fwd.y, fwd.z, up.x, up.y, up.z
        );
    }
}
