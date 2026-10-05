//! A craft on the far side of a fork reads lap progress on the ring's own
//! scale: driving any route from before its fork to past its rejoin advances
//! progress smoothly, by about the ring's own span, and never wraps a lap.
//!
//! **Why this is not automatic.** `Course` measures progress on the ring, and
//! a route strays up to 241 units from it on Pulse (`14_Track`) and 300 on
//! 2048 (`subway`) - `cargo run -p oag-game --example fork_survey`. Projected
//! onto the nearest ring point, a craft that far out loses its windowed fix,
//! falls back to a global scan, and can land a lap away. The original reads
//! progress off whichever path the craft is on (`Craft_UpdateLapProgress`
//! passes the AI's excluded path to its locator, `0x08842af4`), so a lap is
//! the same lap on either side. See
//! `docs/ghidra/functions/psp-pulse-usa/ai-branch-choice.md`.
//!
//! **`#[ignore]`d and never run in CI**: it needs the Pulse disc and the 2048
//! package. Run with `just test-data`.

use oag_core::math::Vec3;
use oag_race::Course;
use oag_raceplay::catalogue;
use oag_vex::{track, vex};

/// A circuit's course, or `None` when the file is not there.
fn course_of(archives: &mut oag_assets::Archives, entry: &str) -> Option<Course> {
    let file = archives.read_name(entry).ok()?;
    let nodes = vex::nodes(&file).ok()?;
    let node = track::find_node(&file, &nodes)?;
    let ai = track::parse(&file[node.payload()]).ok()?;
    Course::from_track(&ai, None)
}

/// The signed step from `from` to `to` round a lap of `length`, in
/// `(-length / 2, length / 2]`.
fn step(from: f32, to: f32, length: f32) -> f32 {
    let d = (to - from).rem_euclid(length);
    if d > length * 0.5 { d - length } else { d }
}

/// Drives every route of `course` sample by sample through `Course::locate`,
/// feeding each fix back as the next hint the way `oag_race::Standing` does,
/// and checks the progress it reads.
fn every_route_reads_progress_on_the_ring(id: &str, course: &Course) -> usize {
    let length = course.length();
    let count = course.len();
    for (k, route) in course.routes().iter().enumerate() {
        // From a stretch of ring before the fork, through the route, onto a
        // stretch of ring past the rejoin.
        let before = (route.split + count - 40) % count;
        let mut points: Vec<Vec3> = (0..40)
            .filter_map(|i| course.position((before + i) % count))
            .collect();
        points.extend(route.positions.iter().copied());
        points.extend((0..40).filter_map(|i| course.position((route.merge + i) % count)));

        let mut hint = Some(before);
        let mut last: Option<f32> = None;
        let mut travelled = 0.0f32;
        for (i, point) in points.iter().enumerate() {
            let located = course
                .locate(*point, hint)
                .unwrap_or_else(|| panic!("{id} route {k}: sample {i} not located"));
            hint = Some(located.index);
            if let Some(last) = last {
                let d = step(last, located.progress, length);
                // **Backward by up to 40 units, on 2048's `square` only.** Its
                // alternate forks twice more before it rejoins, so three routes
                // share their first path, and a shared sample is read through
                // whichever route claimed it first - a different length than the
                // one the craft turns out to be on, so the reading steps back where
                // they diverge. Half a lap is what the lap gate needs to be fooled
                // (`oag_race::RaceState`); forty units of a 7,890-unit lap is not.
                assert!(
                    (-45.0..=60.0).contains(&d),
                    "{id} route {k} (paths {:?}): progress jumped {d:.1} at sample {i} of {} \
                     ({last:.1} -> {:.1}), the fix landed {:.1} units off",
                    route.paths,
                    points.len(),
                    located.progress,
                    located.offset,
                );
                travelled += d;
            }
            last = Some(located.progress);
        }
        // The whole drive covers the ring's own split-to-merge span plus the
        // eighty ring samples either side, within a generous margin.
        let span = step(
            course.progress_at(route.split).unwrap_or(0.0),
            course.progress_at(route.merge).unwrap_or(0.0),
            length,
        )
        .rem_euclid(length);
        let ring_ends = step(
            course.progress_at(before).unwrap_or(0.0),
            course.progress_at(route.split).unwrap_or(0.0),
            length,
        ) + step(
            course.progress_at(route.merge).unwrap_or(0.0),
            course
                .progress_at((route.merge + 39) % count)
                .unwrap_or(0.0),
            length,
        );
        let expected = span + ring_ends;
        assert!(
            (expected * 0.8..=expected * 1.2).contains(&travelled),
            "{id} route {k} (paths {:?}): progress advanced {travelled:.0} over a stretch \
             whose ring span is {expected:.0}",
            route.paths
        );
    }
    course.routes().len()
}

#[test]
#[ignore = "needs the Pulse disc in data/images/"]
fn a_pulse_fork_reads_progress_on_the_ring() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("mounting the disc");
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .expect("the game plugin definition");
    let definition = oag_tables::fexml::expand(&blob).expect("expanding it");
    let mut routes = 0;
    for track in catalogue::tracks(&definition)
        .into_iter()
        .filter(|t| !t.reversed)
    {
        let course = course_of(&mut archives, &track.entry_name()).expect("every circuit loads");
        routes += every_route_reads_progress_on_the_ring(&track.id, &course);
    }
    // 05, 07 and 14 fork once each.
    assert_eq!(routes, 3, "Pulse's three forks were not all found");
}

#[test]
#[ignore = "needs the decrypted 2048 package in data/extracted/vita/"]
fn every_2048_fork_reads_progress_on_the_ring() {
    let Some(source) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    let mut archives = oag_2048::open(&source.display().to_string()).expect("mounting 2048");
    let mut routes = 0;
    for name in [
        "square",
        "park",
        "tower",
        "mall",
        "bridge",
        "arena",
        "subway",
        "cathedral",
        "sol",
        "altima",
    ] {
        let entry = format!(r"Data\art\published\environments\{name}\track.vex");
        let course = course_of(&mut archives, &entry).expect("every circuit loads");
        let found = every_route_reads_progress_on_the_ring(name, &course);
        assert!(found > 0, "{name} has forks and no route was found");
        routes += found;
    }
    assert!(
        routes >= 26,
        "only {routes} routes across 2048's ten circuits"
    );
}
