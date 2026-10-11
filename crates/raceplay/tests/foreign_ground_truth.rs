//! The foreign-track mapping on the Workshop tracks tagged `Original` (never
//! `Port/Remake`): the conventions measured on Pulse's `16_Track` hold on the
//! mapped output (`crates/game/tests/foreign_conventions_ground_truth.rs`),
//! and what is mapped is where the content put it. Disc-backed.

use oag_core::math::Vec3;
use oag_gameplay::collision::collision_world;
use oag_physics::{Ray, Raycaster};
use oag_race::Course;
use oag_raceplay::foreign;

fn originals() -> Option<Vec<std::path::PathBuf>> {
    let root = oag_testdata::exact("data/unity/workshop-473770")?;
    let mut out = Vec::new();
    let mut dirs: Vec<_> = std::fs::read_dir(root)
        .ok()?
        .map(|e| e.unwrap().path())
        .collect();
    dirs.sort();
    for d in dirs {
        for e in std::fs::read_dir(d).ok()? {
            let p = e.ok()?.path();
            if p.extension().and_then(|x| x.to_str()) != Some("wtrk") {
                continue;
            }
            let file = std::fs::read(&p).ok()?;
            let w = oag_ballisticng::wtrk::Wtrk::parse(&file).ok()?;
            if w.has_tag("Original") && !w.has_tag("Port/Remake") {
                out.push(p);
            }
        }
    }
    Some(out)
}

#[test]
#[ignore = "needs data/unity"]
fn the_originals_map_and_hold_the_pulse_conventions() {
    let Some(items) = originals() else { return };
    assert_eq!(items.len(), 27);
    let mut refused = Vec::new();
    for path in items {
        let t = match foreign::read(&path) {
            Ok(t) => t,
            Err(e) => {
                refused.push(format!("{}: {e}", path.display()));
                continue;
            }
        };
        let world = collision_world(&t.nodes);
        let pts = &t.ai.paths[0].points;
        let (mut hits, mut up_ok, mut line_ok) = (0usize, 0usize, 0usize);
        for p in pts {
            let (tan, down, lat) = (
                Vec3::from_array(p.tangent),
                Vec3::from_array(p.down),
                Vec3::from_array(p.lateral),
            );
            // The convention measured on Pulse: lateral = tangent x up.
            assert!((tan.cross(down).dot(lat) + 1.0).abs() < 1e-3, "{}", t.title);
            let from = Vec3::from_array(p.pos) - down * 3.0;
            if let Some(h) = world.raycast(Ray::new(from, down, 8.0), None, false)
                && (h.distance - 3.0).abs() < 1.5
            {
                hits += 1;
                up_ok += usize::from(h.normal.dot(-down) > 0.9);
            }
            line_ok += usize::from(
                p.racing_line.abs() < p.half_width_left.min(p.half_width_right)
                    && p.ai_bound_left < p.racing_line
                    && p.ai_bound_right > p.racing_line,
            );
        }
        let n = pts.len();
        // 27 of 27 tracks put floor under at least 88 percent of their ring
        // (21 of them under all but a handful); the rest are the authored
        // jumps and sections the floor does not span.
        assert!(
            hits * 100 >= n * 85,
            "{}: floor under {hits} of {n}",
            t.title
        );
        assert_eq!(up_ok, hits, "{}: floor normals face up", t.title);
        assert_eq!(line_ok, n, "{}: racing line inside the corridor", t.title);
        let start = Vec3::from_array(t.start.position);
        let course = Course::from_track(&t.ai, Some(start)).expect("the ring closes");
        assert!(course.length() > 100.0);
        let slot = world.raycast(
            Ray::new(start + Vec3::Y * 20.0, -Vec3::Y, 60.0),
            None,
            false,
        );
        assert!(slot.is_some(), "{}: floor under the start slot", t.title);
        for pad in t.speedup_pads.iter().chain(&t.weapon_pads) {
            assert!(
                pad.contains(pad.centre()),
                "{}: pad contains its centre",
                t.title
            );
        }
    }
    assert_eq!(refused, Vec::<String>::new());
}

#[test]
#[ignore = "needs data/unity"]
fn lions_den_maps_as_the_probe_read_it() {
    let Some(root) = oag_testdata::exact("data/unity/workshop-473770") else {
        return;
    };
    let path = std::fs::read_dir(root.join("1392209921"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let t = foreign::read(&path).expect("maps");
    for l in &t.notes {
        println!("{l}");
    }
    // The engine's own start-line rule (the slot advanced 154 units along its
    // straight tangent, projected onto the ring) against the authored line,
    // section 52: lap section 52 is the 53rd control point (the lap loop starts
    // at section 0 and runs 0..=305, 343).
    let course = oag_race::Course::from_track(&t.ai, Some(Vec3::from_array(t.start.position)))
        .expect("closes");
    let line = Vec3::from_array(t.ai.paths[0].points[52].pos);
    let missed = course
        .position(course.start_index())
        .unwrap()
        .distance(line);
    println!("start line misses the authored section 52 by {missed:.1} units");
    assert!(
        missed < MISSED_MAX,
        "start line {missed} units from section 52"
    );
    assert_eq!(t.ai.paths.len(), 1);
    assert_eq!(t.ai.paths[0].points.len(), 307);
    assert_eq!(t.speedup_pads.len(), 48);
    assert!(!t.weapon_pads.is_empty());
    assert!(t.notes.iter().any(|l| l.contains("jump(s)")));
}

/// Measured on Lion's Den: where the engine's start line lands against the
/// authored one: 5.0 units, one resampled step (control points are 20 apart).
const MISSED_MAX: f32 = 10.0;
