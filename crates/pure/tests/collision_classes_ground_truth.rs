//! Which of Pure's three collision-shaped classes is floor, wall and reset.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-pure --run-ignored all \
//!     -E 'binary(collision_classes_ground_truth)'
//! ```
//!
//! # The question
//!
//! `docs/formats/collision.md` found three class ids in Pure's version-4 `.vex`
//! files whose payloads decode exactly under the collision chunk walk - `0x36b`,
//! `0x36c` and `0x37f`, 16 nodes each, one per circuit - and could not say which
//! was which. The evidence available then was **object counts**: two large
//! classes and one small one, the same shape Pulse's floor/wall/reset have. That
//! is analogy, not evidence, so nothing was named and `vex::classes::V4` left all
//! five collision ids `None`. The consequence is concrete: `collision::from_vex`
//! returns nothing on a Pure track, so a ship falls through it.
//!
//! # The discriminator, and why it is not a count
//!
//! A floor and a wall differ in **where they face**, which is a property of the
//! geometry rather than of how much of it there is. For every triangle: take its
//! area-weighted normal, find the nearest sample of the track's own spline, and
//! take `|dot(normal, up)|` against that sample's local up axis. A floor's
//! triangles face along it; a wall's face across it.
//!
//! The spline is the right reference and world `+y` is not, because these
//! circuits bank, climb and in Pulse's case invert - a world-up test would call
//! a banked floor a wall. The spline axis is decoded from the track's own
//! `WO Track` payload, which reads on Pure already ([`oag_formats::track`]).
//!
//! # Calibrated before it is trusted
//!
//! [`the_discriminator_separates_pulses_known_classes`] runs the same statistic
//! over **Pulse**, where the answer is known from
//! `Collision_RegisterNodeClasses` at confidence 90, and asserts it puts floor
//! and wall on the right sides with a margin. A discriminator that cannot tell
//! apart two classes whose identities are already established has no business
//! being pointed at three whose identities are not. Only then does
//! [`pures_three_candidates_separate_the_same_way`] report Pure's.
//!
//! # This file names nothing
//!
//! It **measures**. Whether the separation it finds is enough to put ids into
//! `vex::classes::V4` is a judgement recorded in `docs/formats/collision.md`
//! with a confidence score, per `CLAUDE.md`'s rules - not something a test
//! decides. What this file guarantees is that the number behind that judgement
//! is reproducible and did not come from counting objects.

use std::path::{Path, PathBuf};

use oag_formats::{collision, track, vex};

/// The three Pure class ids whose payloads decode as collision geometry.
///
/// From `docs/formats/collision.md`'s own detector sweep: 16 nodes each, all
/// closing exactly, with no false positives elsewhere in the file set.
const PURE_CANDIDATES: [u32; 3] = [0x36b, 0x36c, 0x37f];

/// Pulse's, which are established at confidence 90 and are the calibration.
const PULSE_KNOWN: [(&str, u32); 5] = [
    ("floor", vex::CLASS_FLOOR_COLLISION),
    ("wall", vex::CLASS_WALL_COLLISION),
    ("reset", vex::CLASS_RESET_COLLISION),
    ("magfloor", vex::CLASS_MAG_FLOOR_COLLISION),
    ("cage", vex::CLASS_CAGE_COLLISION),
];

/// What the statistic says about one class on one circuit.
#[derive(Debug, Clone, Copy)]
struct Facing {
    /// Total triangle area, so classes can be weighted by how much there is.
    area: f32,
    /// Area-weighted mean of `|dot(normal, spline up)|`. 1.0 faces along the
    /// track's up axis, 0.0 faces across it.
    mean_abs_dot: f32,
    /// Fraction of area whose `|dot|` is above 0.7 - about 45 degrees of the up
    /// axis, so unambiguously a surface a ship rides on.
    floorlike: f32,
    /// Fraction of area whose `|dot|` is below 0.3 - within about 17 degrees of
    /// vertical, so unambiguously a surface a ship hits side-on.
    walllike: f32,
    /// How many objects the class carries on this circuit.
    objects: usize,
    /// Triangles across all its objects.
    triangles: usize,
    /// Mean triangle area per object, which is what isolates Pure's third class.
    ///
    /// Facing alone cannot: an enclosing shell averages "along" and "across" and
    /// lands between a floor and a wall without being either. **Scale can.** A
    /// floor object and a wall object are pieces of track surface and come out
    /// within a factor of two of each other on both discs; a shell is a different
    /// order of magnitude.
    area_per_object: f32,
    /// Objects that are exactly a closed box: 8 vertices and 12 triangles.
    ///
    /// The second, independent axis. A facing statistic cannot tell a *volume*
    /// from a *surface* - a box averages the two, which is why the third
    /// candidate sits between the other two rather than beside either - and this
    /// says outright how many of the objects are boxes.
    boxes: usize,
}

fn image(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(name);
    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

/// Densely sampled spline positions and up axes, from the track's own payload.
///
/// Sampled rather than taken at the control points because the nearest-point
/// lookup below is nearest-*sample*: control points on a long straight can be
/// tens of units apart, and a triangle beside the gap would take its up axis
/// from a point far along the curve. Four per segment matches what
/// `oag_game::race::Spline` builds for the same reason.
fn spline_frames(blob: &[u8]) -> Vec<([f32; 3], [f32; 3])> {
    // Found by the class table for the file's own version - `track::find_node`
    // is the call that already does this, and the reason a Pure track's spline
    // reads at all.
    let Ok(nodes) = vex::nodes(blob) else {
        return Vec::new();
    };
    let Some(node) = track::find_node(blob, &nodes) else {
        return Vec::new();
    };
    let Ok(ai) = track::parse(&blob[node.payload()]) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for path in &ai.paths {
        let segments = path.points.len();
        for segment in 0..segments {
            for step in 0..4 {
                let t = step as f32 / 4.0;
                let Some(sample) = path.sample(segment, t) else {
                    continue;
                };
                // `down` is what the payload carries; up is its negation. Not
                // renormalised by the decoder (it matches the original there), so
                // it is normalised here.
                let d = sample.down;
                let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
                if len > 1e-6 {
                    out.push((sample.pos, [-d[0] / len, -d[1] / len, -d[2] / len]));
                }
            }
        }
    }
    out
}

/// The facing statistic for one class id in one track file.
///
/// `None` when the class has no nodes here, which is how a candidate that is not
/// collision geometry at all reports rather than by producing a misleading zero.
fn facing_of(blob: &[u8], class_id: u32, frames: &[([f32; 3], [f32; 3])]) -> Option<Facing> {
    let nodes = vex::nodes(blob).ok()?;
    let mut area = 0.0f32;
    let mut weighted = 0.0f32;
    let mut floorlike = 0.0f32;
    let mut walllike = 0.0f32;
    let mut objects = 0usize;
    let mut triangles = 0usize;
    let mut boxes = 0usize;
    let mut found = false;

    for node in nodes.iter().filter(|n| n.class_id == class_id) {
        found = true;
        let Some(payload) = blob.get(node.payload()) else {
            continue;
        };
        let Ok(geometry) = collision::parse_chunks(payload) else {
            continue;
        };
        objects += geometry.meshes.len();
        for mesh in &geometry.meshes {
            triangles += mesh.triangles.len();
            if mesh.vertices.len() == 8 && mesh.triangles.len() == 12 {
                boxes += 1;
            }
            for tri in &mesh.triangles {
                let (Some(a), Some(b), Some(c)) = (
                    mesh.vertices.get(tri[0] as usize),
                    mesh.vertices.get(tri[1] as usize),
                    mesh.vertices.get(tri[2] as usize),
                ) else {
                    continue;
                };
                let ab = sub(*b, *a);
                let ac = sub(*c, *a);
                let n = cross(ab, ac);
                // Length of the cross product is twice the triangle's area, so
                // this weights by area without a separate normalise-then-measure
                // pass. A degenerate triangle contributes nothing, correctly.
                let two_area = norm(n);
                if two_area < 1e-9 {
                    continue;
                }
                let unit = [n[0] / two_area, n[1] / two_area, n[2] / two_area];
                let centre = [
                    (a[0] + b[0] + c[0]) / 3.0,
                    (a[1] + b[1] + c[1]) / 3.0,
                    (a[2] + b[2] + c[2]) / 3.0,
                ];
                let Some(up) = nearest_up(frames, centre) else {
                    continue;
                };
                // `abs`, because a collision surface's winding says which side is
                // solid and this question is only about the axis it lies on.
                let d = dot(unit, up).abs().min(1.0);
                let w = two_area * 0.5;
                area += w;
                weighted += w * d;
                if d > 0.7 {
                    floorlike += w;
                }
                if d < 0.3 {
                    walllike += w;
                }
            }
        }
    }

    if !found || area <= 0.0 {
        return None;
    }
    Some(Facing {
        area,
        mean_abs_dot: weighted / area,
        floorlike: floorlike / area,
        walllike: walllike / area,
        objects,
        triangles,
        boxes,
        area_per_object: area / objects.max(1) as f32,
    })
}

fn nearest_up(frames: &[([f32; 3], [f32; 3])], at: [f32; 3]) -> Option<[f32; 3]> {
    let mut best = f32::INFINITY;
    let mut up = None;
    for (pos, axis) in frames {
        let d = sub(*pos, at);
        let d2 = d[0] * d[0] + d[1] * d[1] + d[2] * d[2];
        if d2 < best {
            best = d2;
            up = Some(*axis);
        }
    }
    up
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn norm(a: [f32; 3]) -> f32 {
    dot(a, a).sqrt()
}

/// Every circuit a source's own plugin definition declares, as entry names.
///
/// Read off the disc rather than listed here, for the reason
/// `oag_game::catalogue` gives: a circuit list in this repository would be
/// shipped content, and a disc's own answer is the one that matches the
/// pressing in front of you.
fn circuits(archives: &mut oag_assets::Archives, definition: &str) -> Vec<String> {
    let Ok(blob) = archives.read_name(definition) else {
        return Vec::new();
    };
    let Ok(xml) = oag_formats::fexml::text(&blob) else {
        return Vec::new();
    };
    let root = oag_formats::fexml::parse(&xml);
    let mut out = Vec::new();
    collect_tracks(&root, &mut out);
    out.sort();
    out.dedup();
    out
}

fn collect_tracks(node: &oag_formats::fexml::Node, out: &mut Vec<String>) {
    if node.name.eq_ignore_ascii_case("PI_Track") {
        for child in &node.children {
            if let Some(location) = child.value("location") {
                out.push(format!(r"{location}\track.vex"));
            }
        }
    }
    for child in &node.children {
        collect_tracks(child, out);
    }
}

/// One row of the table this file exists to produce.
fn measure(
    archives: &mut oag_assets::Archives,
    circuits: &[String],
    classes: &[(String, u32)],
) -> Vec<(String, Facing)> {
    let mut totals: Vec<(String, Running)> = classes
        .iter()
        .map(|(label, _)| (label.clone(), Running::default()))
        .collect();

    for circuit in circuits {
        let Ok(blob) = archives.read_name(circuit) else {
            continue;
        };
        let frames = spline_frames(&blob);
        if frames.is_empty() {
            println!("  {circuit}: no spline, skipped");
            continue;
        }
        for (index, (_, class_id)) in classes.iter().enumerate() {
            let Some(f) = facing_of(&blob, *class_id, &frames) else {
                continue;
            };
            let running = &mut totals[index].1;
            running.area += f.area;
            running.weighted += f.mean_abs_dot * f.area;
            running.floorlike += f.floorlike * f.area;
            running.walllike += f.walllike * f.area;
            running.objects += f.objects;
            running.triangles += f.triangles;
            running.boxes += f.boxes;
        }
    }

    totals
        .into_iter()
        .filter(|(_, running)| running.area > 0.0)
        .map(|(label, running)| {
            let objects = running.objects.max(1) as f32;
            (
                label,
                Facing {
                    area: running.area,
                    mean_abs_dot: running.weighted / running.area,
                    floorlike: running.floorlike / running.area,
                    walllike: running.walllike / running.area,
                    objects: running.objects,
                    triangles: running.triangles,
                    boxes: running.boxes,
                    area_per_object: running.area / objects,
                },
            )
        })
        .collect()
}

/// One class's running totals while [`measure`] walks the circuits.
///
/// A named struct rather than a tuple only because the tuple had grown to eight
/// fields and every read of it was an index.
#[derive(Debug, Clone, Copy, Default)]
struct Running {
    area: f32,
    weighted: f32,
    floorlike: f32,
    walllike: f32,
    objects: usize,
    triangles: usize,
    boxes: usize,
}

fn print_table(title: &str, rows: &[(String, Facing)]) {
    println!("\n{title}");
    println!(
        "  {:<8} {:>10} {:>9} {:>9} {:>11} {:>8} {:>9} {:>12}",
        "class", "mean|dot|", "floor>.7", "wall<.3", "area", "objects", "tris/obj", "area/obj"
    );
    for (label, f) in rows {
        println!(
            "  {:<8} {:>10.3} {:>8.1}% {:>8.1}% {:>11.0} {:>8} {:>9.1} {:>12.0}",
            label,
            f.mean_abs_dot,
            f.floorlike * 100.0,
            f.walllike * 100.0,
            f.area,
            f.objects,
            f.triangles as f32 / f.objects.max(1) as f32,
            f.area_per_object,
        );
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_discriminator_separates_pulses_known_classes() {
    let Some(pulse) = image("data/images/pulse-psp-eu.chd") else {
        return;
    };
    let mut archives = oag_pulse::open(&pulse.display().to_string()).expect("opening Pulse");
    let circuits = circuits(&mut archives, oag_pulse::names::GAME_PLUGIN_DEFINITION);
    assert!(!circuits.is_empty(), "Pulse declares circuits");

    let classes: Vec<(String, u32)> = PULSE_KNOWN
        .iter()
        .map(|(label, id)| ((*label).to_string(), *id))
        .collect();
    let rows = measure(&mut archives, &circuits, &classes);
    print_table("Pulse (identities known, confidence 90)", &rows);

    let get = |name: &str| {
        rows.iter()
            .find(|(label, _)| label == name)
            .map(|(_, f)| *f)
            .unwrap_or_else(|| panic!("Pulse's {name} class produced no geometry"))
    };
    let floor = get("floor");
    let wall = get("wall");

    // The calibration. These bounds are deliberately loose - the claim being
    // tested is "this statistic separates the two at all", not any particular
    // value - but they are far enough apart that a discriminator which had
    // silently stopped working could not pass.
    assert!(
        floor.mean_abs_dot > 0.75,
        "a floor faces along the track's up axis; got {:.3}",
        floor.mean_abs_dot
    );
    assert!(
        wall.mean_abs_dot < 0.45,
        "a wall faces across it; got {:.3}",
        wall.mean_abs_dot
    );
    assert!(
        floor.mean_abs_dot - wall.mean_abs_dot > 0.35,
        "the two have to be separated by more than measurement noise; got floor \
         {:.3} against wall {:.3}",
        floor.mean_abs_dot,
        wall.mean_abs_dot
    );
    assert!(
        floor.floorlike > 0.6,
        "most of a floor's area is within 45 degrees of up; got {:.1}%",
        floor.floorlike * 100.0
    );
    assert!(
        wall.walllike > 0.5,
        "most of a wall's area is near vertical; got {:.1}%",
        wall.walllike * 100.0
    );
}

/// No `Cage` signature is measurable, and that is a finding rather than a gap.
///
/// `docs/formats/collision.md` censused **6** cage nodes on the whole PS2 disc
/// and **0** on the PSP one. This walks every circuit *either* disc declares as
/// raceable and finds none on any of them - so the six live somewhere the race
/// path never loads, and there is no reference signature for a cage to compare
/// Pure's third candidate against.
///
/// Recorded as a test rather than left unwritten because it is the reason that
/// candidate stays unnamed: it is not that nobody looked, it is that the only
/// Pulse class with no measurable signature is also the only one whose
/// signature would have settled the question.
#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn no_cage_geometry_is_reachable_from_either_pressings_race_circuits() {
    for (label, name) in [
        ("PS2", "data/images/pulse-ps2-eu.chd"),
        ("PSP", "data/images/pulse-psp-eu.chd"),
    ] {
        let Some(disc) = image(name) else { continue };
        let mut archives = oag_pulse::open(&disc.display().to_string()).expect("opening Pulse");
        let circuits = circuits(&mut archives, oag_pulse::names::GAME_PLUGIN_DEFINITION);
        assert!(!circuits.is_empty(), "{label}: declares circuits");
        let rows = measure(
            &mut archives,
            &circuits,
            &[("cage".to_string(), vex::CLASS_CAGE_COLLISION)],
        );
        assert!(
            rows.is_empty(),
            "{label}: a cage turned up on a race circuit, which would make a cage \
             signature measurable after all - re-open the third Pure candidate; got \
             {rows:#?}"
        );
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pures_three_candidates_separate_the_same_way() {
    let Some(pure) = image("data/images/pure-psp-eu.chd") else {
        return;
    };
    let Some(pulse) = image("data/images/pulse-psp-eu.chd") else {
        return;
    };

    let mut pulse_archives = oag_pulse::open(&pulse.display().to_string()).expect("opening Pulse");
    let pulse_circuits = circuits(
        &mut pulse_archives,
        oag_pulse::names::GAME_PLUGIN_DEFINITION,
    );
    let pulse_rows = measure(
        &mut pulse_archives,
        &pulse_circuits,
        &PULSE_KNOWN
            .iter()
            .map(|(label, id)| ((*label).to_string(), *id))
            .collect::<Vec<_>>(),
    );
    print_table("Pulse (identities known, confidence 90)", &pulse_rows);

    let mut pure_archives = oag_pure::open(&pure.display().to_string()).expect("opening Pure");
    let pure_circuits = circuits(&mut pure_archives, oag_pure::names::GAME_PLUGIN_DEFINITION);
    let pure_rows = measure(
        &mut pure_archives,
        &pure_circuits,
        &PURE_CANDIDATES
            .iter()
            .map(|id| (format!("{id:#05x}"), *id))
            .collect::<Vec<_>>(),
    );
    print_table("Pure (0x36b and 0x36c named; 0x37f not)", &pure_rows);

    assert_eq!(
        pure_rows.len(),
        PURE_CANDIDATES.len(),
        "all three candidates carry geometry somewhere; got {pure_rows:#?}"
    );

    let row = |rows: &[(String, Facing)], name: &str| -> Facing {
        rows.iter()
            .find(|(label, _)| label == name)
            .map(|(_, f)| *f)
            .unwrap_or_else(|| panic!("no {name} row in {rows:#?}"))
    };

    // **The two claims `vex::classes::V4` now makes**, asserted against the ids
    // it actually holds rather than against literals repeated here - so moving an
    // id in the table without re-measuring fails this.
    let v4 = vex::classes::V4;
    let named = |id: Option<u32>| -> String { format!("{:#05x}", id.expect("V4 names this")) };
    let floor = row(&pure_rows, &named(v4.floor_collision));
    let wall = row(&pure_rows, &named(v4.wall_collision));
    let pulse_floor = row(&pulse_rows, "floor");
    let pulse_wall = row(&pulse_rows, "wall");

    // Tolerances are the spread Pulse's own circuits show between its PSP and PS2
    // pressings (0.979 against 0.972 for a floor, 0.087 against 0.070 for a wall),
    // rounded outward. Tighter would be pinning noise; looser would stop
    // distinguishing a floor from a wall, which is the whole claim.
    assert!(
        (floor.mean_abs_dot - pulse_floor.mean_abs_dot).abs() < 0.10,
        "Pure's floor class should face the way Pulse's does; got {:.3} against \
         {:.3}",
        floor.mean_abs_dot,
        pulse_floor.mean_abs_dot
    );
    assert!(
        (wall.mean_abs_dot - pulse_wall.mean_abs_dot).abs() < 0.10,
        "Pure's wall class should face the way Pulse's does; got {:.3} against \
         {:.3}",
        wall.mean_abs_dot,
        pulse_wall.mean_abs_dot
    );
    assert!(
        floor.floorlike > 0.9 && wall.walllike > 0.8,
        "and each should be overwhelmingly one or the other; got floor {:.1}% \
         floor-facing, wall {:.1}% wall-facing",
        floor.floorlike * 100.0,
        wall.walllike * 100.0
    );

    // **And the claim it declines to make.** The third candidate matches none of
    // Pulse's four measurable classes on either axis. If a future change ever
    // makes it match one, that is the evidence needed to name it - and this
    // failing is how anyone would find out.
    let third = row(
        &pure_rows,
        &format!(
            "{:#05x}",
            PURE_CANDIDATES
                .iter()
                .find(|id| Some(**id) != v4.floor_collision && Some(**id) != v4.wall_collision)
                .expect("a third candidate")
        ),
    );
    for (label, known) in &pulse_rows {
        assert!(
            (third.mean_abs_dot - known.mean_abs_dot).abs() > 0.3
                || third.area_per_object > known.area_per_object * 4.0,
            "the third candidate now resembles Pulse's {label} ({:.3} / {:.0} against \
             {:.3} / {:.0}) - that is grounds to name it, so re-open \
             docs/formats/collision.md rather than relaxing this",
            third.mean_abs_dot,
            third.area_per_object,
            known.mean_abs_dot,
            known.area_per_object
        );
    }
}
