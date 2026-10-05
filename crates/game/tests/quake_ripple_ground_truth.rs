//! Holds the Quake ripple's placement against where the road actually is.
//!
//! The ripple draws a bump at the simulation wave's course distance into every
//! road vertex whose own course distance is near it
//! (`oag_game::race::place_spans`, `oag_render::ripple`). A vertex's distance
//! is *predicted* from its span's place on its AiTrack path and its own
//! parameter along the span. Two things are measured here instead of trusted:
//!
//! 1. **Where the vertex really is.** Every movable road vertex is located on
//!    the course directly and compared with its prediction. A span read at the
//!    wrong path, a path mapped to the wrong ring points, or a `t` that is not
//!    what the mapping assumes all show up as residuals that grow along a path.
//! 2. **Whether neighbouring spans agree.** The span table's own links say, to
//!    0.01 units, where each neighbour starts; two spans placed inconsistently
//!    would draw the bump at two different distances either side of the seam
//!    between them, which reads as a step in the road.
//!
//! **`#[ignore]`d and never run in CI**: it needs `pulse-psp-usa.chd`. Run
//! with `just test-data`.

use std::path::PathBuf;

use oag_core::math::Vec3;
use oag_game::race::SpanPlaces;
use oag_race::Course;
use oag_vex::quake::Span;
use oag_vex::{quake, track, vex};

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

/// How far from the centre line a vertex may be and still count as road.
///
/// Measured on 02_Track, where two long spans' outer vertices, 73 to 80 units
/// out, locate 300 units back down the track onto the far side of a hairpin:
/// past a road's width, "nearest ring point" measures the locator, not the
/// placement.
const ROAD_DECK: f32 = 30.0;

const DIRS: &[&str] = &[
    "01_Track", "02_Track", "03_Track", "04_Track", "05_Track", "06_Track", "07_Track", "09_Track",
    "10_Track", "13_Track", "14_Track", "16_Track",
];

/// One circuit file, decoded as far as the check needs.
struct Circuit {
    blob: Vec<u8>,
    course: Course,
    spans: Vec<Span>,
    places: SpanPlaces,
}

fn circuit(archives: &mut oag_assets::Archives, name: &str) -> Option<Circuit> {
    let blob = archives.read_name(name).ok()?;
    let nodes = vex::nodes(&blob).expect("nodes");
    let classes = vex::classes_of(&blob).expect("classes");
    let wo_track = track::find_node(&blob, &nodes).expect("a WO Track node");
    let ai = track::parse(&blob[wo_track.payload()]).expect("the spline graph");
    let start = nodes
        .iter()
        .find(|n| Some(n.class_id) == classes.start_position)
        .and_then(|n| track::start_position(&blob[n.payload()], vex::byte_order(&blob)))
        .map(|s| Vec3::from(s.position));
    let course = Course::from_track(&ai, start).expect("a closed course");
    let node = vex::nodes_by_class(&nodes, quake::CLASS_QUAKE)
        .next()
        .expect("a Quake node");
    let spans = quake::spans(&blob, node).expect("spans");
    let places = SpanPlaces::new(&spans, &ai, &course);
    Some(Circuit {
        blob,
        course,
        spans,
        places,
    })
}

/// Absolute residuals, in course units, of every road-deck vertex on a path
/// the lap drives.
fn residuals(c: &Circuit) -> Vec<f32> {
    let nodes = vex::nodes(&c.blob).expect("nodes");
    let classes = vex::classes_of(&c.blob).expect("classes");
    // Every mesh-shaped batch's decoded vertices and baking matrix, by header.
    let mut batches = std::collections::BTreeMap::new();
    for pick in [
        (|c| c.mesh) as fn(vex::classes::Classes) -> Option<u32>,
        |c| c.speedup_pad,
        |c| c.weapon_pad,
    ] {
        let placed = oag_mesh::mesh::batch_placements(&c.blob, pick).expect("placements");
        let class = pick(classes).expect("class id");
        let decoded = nodes.iter().filter(|n| n.class_id == class).flat_map(|n| {
            let payload = &c.blob[n.payload()];
            [0u8, 1]
                .into_iter()
                .flat_map(move |list| vex::mesh_batches(payload, list).expect("batches"))
        });
        for (placement, batch) in placed.into_iter().zip(decoded) {
            batches.insert(placement.header, (placement.to_world, batch));
        }
    }
    let driven = c.course.path_order();
    let ring = c.course.length();
    let mut out = Vec::new();
    for (i, span) in c.spans.iter().enumerate() {
        if !driven.iter().any(|&p| i32::from(p) == i32::from(span.path)) {
            continue;
        }
        let (to_world, batch) = &batches[&span.batch];
        let mut hint = None;
        for (p, vertex) in span.parameters.iter().zip(&batch.vertices) {
            let Some(p) = p else { continue };
            let world = vex::transform_point(to_world, vertex.position);
            let Some(located) = c.course.locate(Vec3::from(world), hint) else {
                continue;
            };
            hint = Some(located.index);
            if located.offset > ROAD_DECK {
                continue;
            }
            let predicted = c.places.distance(i, *p).expect("placed");
            let error = (located.progress - predicted + ring * 0.5).rem_euclid(ring) - ring * 0.5;
            out.push(error.abs());
        }
    }
    out.sort_by(f32::total_cmp);
    out
}

/// One seam's step, in course units, whether both sides lie on a path the lap
/// drives, and which link it is.
struct Seam {
    step: f32,
    driven: bool,
    which: String,
}

/// For every link in the span table, how far apart the two spans' places put
/// the point they share, in course units - the step a seam would show.
///
/// Skips the table's zero-length spans (`t_start == t_end`, `length` 0), whose
/// links carry an offset against nothing: a parameter along a span with no
/// length has no meaning to compare.
fn seams(c: &Circuit) -> Vec<Seam> {
    let ring = c.course.length();
    let driven = c.course.path_order();
    let on_ring = |span: &Span| driven.iter().any(|&p| i32::from(p) == i32::from(span.path));
    let real = |span: &Span| span.t_end > span.t_start && span.length > 0.0;
    let mut out = Vec::new();
    for (i, span) in c.spans.iter().enumerate() {
        if !real(span) {
            continue;
        }
        // Each link compared at the seam itself: a forward neighbour's start,
        // and a backward neighbour's end, against this span at the same point.
        // Comparing a backward neighbour's *start* instead extrapolates this
        // span's mapping one whole neighbour-length past its own end.
        let forward = span.forward.iter().flatten().map(|&(n, at)| (n, at, 0.0));
        let backward = span
            .backward
            .iter()
            .flatten()
            .map(|&(n, gap)| (n, span.length + gap, 1.0));
        for (n, at, seam) in forward.chain(backward) {
            let other = &c.spans[usize::from(n)];
            if !real(other) {
                continue;
            }
            let (Some(from), Some(to)) = (
                c.places.distance(i, at / span.length),
                c.places.distance(usize::from(n), seam),
            ) else {
                continue;
            };
            out.push(Seam {
                step: ((to - from + ring * 0.5).rem_euclid(ring) - ring * 0.5).abs(),
                driven: on_ring(span) && on_ring(other),
                which: format!("span {i} (path {}) -> {n} (path {})", span.path, other.path),
            });
        }
    }
    out.sort_by(|a, b| a.step.total_cmp(&b.step));
    out
}

fn each_circuit(mut check: impl FnMut(&str, &Circuit)) {
    let Some(image) = image() else { return };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("mounting the disc");
    let mut checked = 0;
    for dir in DIRS {
        for file in ["track.vex", "track_reversed.vex"] {
            let name = format!("Data\\Environments\\{dir}\\{file}");
            let Some(c) = circuit(&mut archives, &name) else {
                continue;
            };
            assert_eq!(c.places.unplaced(), 0, "{name}: spans with no place");
            check(&format!("{dir}/{file}"), &c);
            checked += 1;
        }
    }
    assert_eq!(checked, 24);
}

/// Every movable road vertex sits where its span's place says, on every
/// circuit forwards and reversed.
///
/// The bound is against a bump 50 to 150 units wide. The median is the
/// locator's own half-sample; the tail is path ends, where two paths' samples
/// overlap on the ring.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_road_vertex_sits_where_its_span_places_it() {
    each_circuit(|name, c| {
        let r = residuals(c);
        let at = |q: f32| r[((r.len() - 1) as f32 * q) as usize];
        println!(
            "{name}: {} road vertices, median {:.2}, p99 {:.2}, max {:.2} units off",
            r.len(),
            at(0.5),
            at(0.99),
            at(1.0)
        );
        assert!(r.len() > 10_000, "{name}: only {} road vertices", r.len());
        assert!(at(0.5) < 1.0, "{name}: median residual {}", at(0.5));
        assert!(at(0.99) < 8.0, "{name}: p99 residual {}", at(0.99));
    });
}

/// Neighbouring spans put the bump at the same distance either side of their
/// seam, so the road does not step - on the paths the lap drives and on the
/// branch of a split alike, each measured and bounded on its own.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn neighbouring_spans_agree_across_every_seam() {
    each_circuit(|name, c| {
        let all = seams(c);
        for (kind, driven) in [("driven", true), ("branch", false)] {
            let s: Vec<&Seam> = all.iter().filter(|seam| seam.driven == driven).collect();
            let Some(worst) = s.last() else { continue };
            let p99 = s[((s.len() - 1) as f32 * 0.99) as usize].step;
            println!(
                "{name}: {} {kind} seams, p99 {p99:.2}, worst {:.2} at {}",
                s.len(),
                worst.step,
                worst.which
            );
            assert!(
                worst.step < 3.0,
                "{name}: a {kind} seam steps {:.2} at {}",
                worst.step,
                worst.which
            );
        }
    });
}

/// `oag_mesh::mesh::batch_placements` walks the batches in exactly the order
/// the model builder lays their vertices down: its ranges tile each node's own
/// range in the built model, end to end, and reach the model's last vertex.
/// The load path refuses a model whose total disagrees; this is the stronger,
/// per-node statement, on the road and both pad models of two circuits.
type Build = fn(&str, &[u8]) -> oag_mesh::mesh::Model;
type Pick = fn(vex::classes::Classes) -> Option<u32>;

#[test]
#[ignore = "needs a disc image in data/images/"]
fn batch_placements_tile_each_node() {
    let Some(image) = image() else { return };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("mounting the disc");
    for dir in ["01_Track", "16_Track"] {
        let name = format!("Data\\Environments\\{dir}\\track.vex");
        let blob = archives.read_name(&name).expect("the circuit");
        let builds: [(&str, Build, Pick); 3] = [
            (
                "road",
                |n, b| oag_mesh::mesh::build_with_textures(n, b, None).expect("road"),
                |c| c.mesh,
            ),
            (
                "speedup pads",
                |n, b| oag_mesh::mesh::build_pads(n, b, None).expect("pads"),
                |c| c.speedup_pad,
            ),
            (
                "weapon pads",
                |n, b| oag_mesh::mesh::build_weapon_pads(n, b, None).expect("pads"),
                |c| c.weapon_pad,
            ),
        ];
        for (what, build, pick) in builds {
            let model = build(&name, &blob);
            let placements = oag_mesh::mesh::batch_placements(&blob, pick).expect("placements");
            let mut next = placements.iter().peekable();
            for node in &model.node_vertex_ranges {
                let mut at = node.start;
                while let Some(p) = next.next_if(|p| p.vertices.start < node.end) {
                    assert_eq!(p.vertices.start, at, "{dir} {what}: a gap inside a node");
                    at = p.vertices.end;
                }
                assert_eq!(
                    at, node.end,
                    "{dir} {what}: the batches stop short of the node's end"
                );
            }
            assert!(
                next.next().is_none(),
                "{dir} {what}: batches past the last node"
            );
            assert_eq!(
                placements.last().map_or(0, |p| p.vertices.end as usize),
                model.vertices.len(),
                "{dir} {what}"
            );
        }
    }
}

/// Level road rises in world space. A span whose authored "down" axis is
/// `(0, -1, 0)` at both ends, pushed through its batch's own matrix, has to
/// point into world `+y` - PPSSPP measured the original's road moving upward
/// on every span a whole Quake armed on Talon's Junction. A level span here
/// moving any other way would be this port applying the matrix wrongly.
///
/// **Not every span is level, and the others are not asserted.** Barrier and
/// bank spans author a "down" that is nearly horizontal - `(0.43, 0.13,
/// -0.89)` on `01_Track` - so the original bends them sideways, and a few
/// author one pointing up. Those move the way their data says; the count is
/// printed.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn level_road_rises_in_world_space() {
    each_circuit(|name, c| {
        let (mut level, mut other) = (0usize, 0usize);
        for pick in [
            (|c| c.mesh) as fn(vex::classes::Classes) -> Option<u32>,
            |c| c.speedup_pad,
            |c| c.weapon_pad,
        ] {
            let placed = oag_mesh::mesh::batch_placements(&c.blob, pick).expect("placements");
            for span in &c.spans {
                let Some(batch) = placed.iter().find(|p| p.header == span.batch) else {
                    continue;
                };
                if span.down_start[1] > -0.99 || span.down_end[1] > -0.99 {
                    other += 1;
                    continue;
                }
                let m = &batch.to_world;
                for p in span.parameters.iter().flatten() {
                    let local = [0, 1, 2].map(|k| {
                        -(span.down_start[k] + (span.down_end[k] - span.down_start[k]) * p)
                    });
                    let y = local[0] * m[1] + local[1] * m[5] + local[2] * m[9];
                    // Not "close to 1": a batch under a tilted node carries
                    // its local up with it - 0.60 on one 14_Track reversed
                    // mesh, as the original's own local-space write does.
                    assert!(y > 0.0, "{name}: a level span's vertex sinks, world y {y}");
                }
                level += 1;
            }
        }
        println!(
            "{name}: {level} level span(s) rise; {other} bank or barrier span(s) move as authored"
        );
        assert!(level > 30, "{name}: only {level} level spans checked");
    });
}
