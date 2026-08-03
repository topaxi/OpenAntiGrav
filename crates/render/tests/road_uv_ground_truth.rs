//! How wide a painted road marking comes out, measured off every PSP circuit
//! that ships the shared road texture.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this crate:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all
//! ```
//!
//! # What this is for
//!
//! A user reported that Moa Therma's centre lines come out wiggly and too wide
//! on PSP assets while other circuits look fine, and the obvious suspect was the
//! PSP's 8-bit texture coordinates - `u8 / 128`, so 0.5 texels of a 64-texel
//! texture per step. This test is the measurement that decides it, and it says
//! the suspect is innocent at the place the artifact was photographed.
//!
//! The quantity is **world units per whole texture repeat**, per triangle, along
//! the direction the texture is stretched furthest: `1 / sigma_min` of the UV
//! Jacobian taken in the triangle's own tangent plane. That is the number a
//! marking's width is proportional to - the texture paints a one-texel line
//! `repeat / 64` world units wide - and it is the right instrument where a
//! whole-draw affine fit is not: a road strip curves, so a fit over one is
//! answering a different question and returns residuals of 30 to 200 stored
//! units that are geometry, not error.
//!
//! # What it does not establish
//!
//! That our render matches the original's. It measures the authored mapping and
//! our decode of it, both of which can be right while the picture is still
//! wrong; how wide a line *looks* also depends on render resolution, filtering
//! and mip selection, none of which this test can see. See `HANDOVER.md`.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use oag_formats::vex;
use oag_render::mesh::{self, Model};

fn image() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images/pulse-psp-usa.chd");

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

/// The road surface's texture, shared by every circuit that has one: a 64x64
/// cross-section of the road, uniform along its columns, carrying the dark
/// centre line in rows 3 to 7 and the broad tone bands below it.
const ROAD: &str = "_surface6_1.tga";

/// Moa Therma. Resolved through `oag_game::catalogue::tracks` plus the string
/// table, and the same directory on both discs -
/// `crates/render/tests/animated_uv_ground_truth.rs` implies `06_Track` for it,
/// which is in fact Vertica.
const MOA_THERMA: &str = "03";

/// Where the reported artifact was photographed, and the radius around it this
/// test calls "the corner". From the frames in the 2026-08-03 pass.
const CORNER: [f32; 3] = [-410.0, 4.0, -200.0];
const CORNER_RADIUS: f32 = 70.0;

fn track(archives: &mut oag_assets::pulse::Archives, circuit: &str) -> Option<(Vec<u8>, Model)> {
    let name = format!(r"Data\Environments\{circuit}_Track\track.vex");
    let blob = archives.read_name(&name).ok()?;
    let model = mesh::build(&name, &blob).ok()?;
    Some((blob, model))
}

/// Every PSP circuit whose `track.vex` decodes, in disc order.
fn circuits() -> Vec<String> {
    (1..=20u32).map(|n| format!("{n:02}")).collect()
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f64; 3] {
    [
        f64::from(a[0]) - f64::from(b[0]),
        f64::from(a[1]) - f64::from(b[1]),
        f64::from(a[2]) - f64::from(b[2]),
    ]
}
fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}
fn norm(a: [f64; 3]) -> f64 {
    dot(a, a).sqrt()
}

/// World units covered by one whole texture repeat across this triangle, along
/// whichever direction the texture is stretched furthest.
///
/// `None` for a triangle with no area, or one whose texture coordinates do not
/// vary at all - both of which say nothing about width rather than saying it is
/// infinite.
fn triangle_repeat(p: [[f32; 3]; 3], uv: [[f32; 2]; 3]) -> Option<f64> {
    let e1 = sub(p[1], p[0]);
    let e2 = sub(p[2], p[0]);
    let d1 = [
        f64::from(uv[1][0]) - f64::from(uv[0][0]),
        f64::from(uv[1][1]) - f64::from(uv[0][1]),
    ];
    let d2 = [
        f64::from(uv[2][0]) - f64::from(uv[0][0]),
        f64::from(uv[2][1]) - f64::from(uv[0][1]),
    ];

    // An orthonormal basis in the triangle's own plane: `e1` becomes `(l1, 0)`
    // and `e2` becomes `(proj, l2)`, so the Jacobian is the texture-coordinate
    // deltas times the inverse of that.
    let l1 = norm(e1);
    if l1 < 1e-6 {
        return None;
    }
    let proj = dot(e2, [e1[0] / l1, e1[1] / l1, e1[2] / l1]);
    let l2 = norm([
        e2[0] - e1[0] / l1 * proj,
        e2[1] - e1[1] / l1 * proj,
        e2[2] - e1[2] / l1 * proj,
    ]);
    if l2 < 1e-6 {
        return None;
    }
    let inv = [[1.0 / l1, -proj / (l1 * l2)], [0.0, 1.0 / l2]];
    let j = [
        [
            d1[0] * inv[0][0] + d2[0] * inv[1][0],
            d1[0] * inv[0][1] + d2[0] * inv[1][1],
        ],
        [
            d1[1] * inv[0][0] + d2[1] * inv[1][0],
            d1[1] * inv[0][1] + d2[1] * inv[1][1],
        ],
    ];

    // Singular values of the 2x2 `j`, via the eigenvalues of `j j^T`.
    let m00 = j[0][0] * j[0][0] + j[0][1] * j[0][1];
    let m01 = j[0][0] * j[1][0] + j[0][1] * j[1][1];
    let m11 = j[1][0] * j[1][0] + j[1][1] * j[1][1];
    let trace = m00 + m11;
    let det = m00 * m11 - m01 * m01;
    let spread = (trace * trace / 4.0 - det).max(0.0).sqrt();
    let smallest = (trace / 2.0 - spread).max(0.0).sqrt();
    (smallest > 1e-9).then(|| 1.0 / smallest)
}

fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    sorted[((sorted.len() - 1) as f64 * q).round() as usize]
}

/// One circuit's road triangles, bucketed by the vertex type of the batch they
/// came from: `0x139` stores texture coordinates as two `u8`, `0x13b` as two
/// `f32`.
#[derive(Default)]
struct Road {
    by_type: BTreeMap<u16, Vec<f64>>,
    /// Repeats for triangles within [`CORNER_RADIUS`] of [`CORNER`], and the
    /// vertex types those triangles came from.
    corner: Vec<f64>,
    corner_types: BTreeMap<u16, usize>,
}

fn measure(blob: &[u8], model: &Model) -> Road {
    let nodes = vex::nodes(blob).expect("decoding nodes");
    let world = vex::world_transforms(blob, &nodes);
    let mut road = Road::default();

    for (index, node) in nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.class_id == 0x125)
    {
        let payload = &blob[node.payload()];
        let materials = vex::mesh_materials(payload);
        let to_world = world[index];
        for list in [0u8, 1u8] {
            for batch in vex::mesh_batches(payload, list).expect("decoding batches") {
                let is_road = materials
                    .get(usize::from(batch.material_index))
                    .copied()
                    .flatten()
                    .map(|m| m.texture as usize)
                    .and_then(|t| model.textures.get(t))
                    .and_then(Option::as_ref)
                    .is_some_and(|t| t.label.eq_ignore_ascii_case(ROAD));
                if !is_road {
                    continue;
                }
                let positions: Vec<[f32; 3]> = batch
                    .vertices
                    .iter()
                    .map(|v| vex::transform_point(&to_world, v.position))
                    .collect();
                for tri in batch.triangles() {
                    let i = [tri[0] as usize, tri[1] as usize, tri[2] as usize];
                    let p = [positions[i[0]], positions[i[1]], positions[i[2]]];
                    let uv = i.map(|k| batch.vertices[k].texcoord.unwrap_or([0.0; 2]));
                    let Some(repeat) = triangle_repeat(p, uv) else {
                        continue;
                    };
                    road.by_type
                        .entry(batch.vertex_type)
                        .or_default()
                        .push(repeat);

                    let centre = [
                        (p[0][0] + p[1][0] + p[2][0]) / 3.0,
                        (p[0][1] + p[1][1] + p[2][1]) / 3.0,
                        (p[0][2] + p[1][2] + p[2][2]) / 3.0,
                    ];
                    let away = ((centre[0] - CORNER[0]).powi(2)
                        + (centre[1] - CORNER[1]).powi(2)
                        + (centre[2] - CORNER[2]).powi(2))
                    .sqrt();
                    if away < CORNER_RADIUS {
                        road.corner.push(repeat);
                        *road.corner_types.entry(batch.vertex_type).or_default() += 1;
                    }
                }
            }
        }
    }

    for reps in road.by_type.values_mut() {
        reps.sort_by(|a, b| a.partial_cmp(b).expect("no NaN repeats"));
    }
    road.corner
        .sort_by(|a, b| a.partial_cmp(b).expect("no NaN repeats"));
    road
}

/// The band the median road triangle's repeat has to fall in, on every circuit
/// that ships the texture.
///
/// The measured medians run 20.40 to 20.72 on six of them and 23.06 on Moa
/// Therma - so the authored road mapping is one scale, laid down once and
/// reused, and a one-texel line is about a third of a world unit wide
/// everywhere. That uniformity is the finding: it is what says the reported
/// "Moa Therma's markings are wider than other circuits'" cannot be the
/// authored mapping - certainly not the roughly threefold difference the frames
/// suggested.
///
/// Moa Therma's 23.06 is a real 13%, and it is worth being precise about where
/// it comes from, because it is the one place the census does separate this
/// circuit from the rest. It is **not** the `0x139` outlier tail: that
/// population's median is 20.87, ordinary. It is the `0x13b` road, median 23.85
/// against 20.33 to 20.71 everywhere else. But it is not at the reported corner
/// either - the corner's own `0x13b` median is 20.42 - so whatever is 16%
/// coarser sits on some other stretch of the circuit, and does not explain the
/// artifact.
///
/// Taken over each circuit's whole road, not per vertex type. Both encodings
/// agree wherever a circuit has enough of each to compare, but a circuit can
/// carry a sliver of one - `06_Track` has 126 `0x13b` triangles against 4,942
/// `0x139` ones - and a median over a sliver describes those few surfaces
/// rather than that circuit's road.
const REPEAT_BAND: std::ops::Range<f64> = 18.0..26.0;

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_circuit_lays_the_road_texture_down_at_the_same_scale() {
    let Some(image) = image() else { return };
    let mut archives =
        oag_assets::pulse::Archives::open(&image.display().to_string()).expect("opening archives");

    let mut measured = 0;
    for circuit in circuits() {
        let Some((blob, model)) = track(&mut archives, &circuit) else {
            continue;
        };
        let road = measure(&blob, &model);
        if road.by_type.is_empty() {
            continue;
        }
        measured += 1;
        for (vertex_type, reps) in &road.by_type {
            println!(
                "{circuit}_Track {vertex_type:#06x}: {} triangle(s), repeat p50 {:.2}, \
                 p95 {:.2}, max {:.2} world units",
                reps.len(),
                quantile(reps, 0.5),
                quantile(reps, 0.95),
                quantile(reps, 1.0)
            );
        }
        let mut all: Vec<f64> = road.by_type.values().flatten().copied().collect();
        all.sort_by(|a, b| a.partial_cmp(b).expect("no NaN repeats"));
        let median = quantile(&all, 0.5);
        println!(
            "{circuit}_Track: {} road triangle(s), repeat p50 {median:.2}",
            all.len()
        );
        assert!(
            REPEAT_BAND.contains(&median),
            "{circuit}_Track: median repeat {median:.2} is outside {REPEAT_BAND:?}, so this \
             circuit lays the shared road texture down at a different scale from the rest"
        );
    }
    assert!(
        measured >= 7,
        "only {measured} circuit(s) carried {ROAD}; the census lost its subject"
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn moa_therma_paints_the_reported_corner_from_f32_texture_coordinates() {
    let Some(image) = image() else { return };
    let mut archives =
        oag_assets::pulse::Archives::open(&image.display().to_string()).expect("opening archives");
    let (blob, model) = track(&mut archives, MOA_THERMA).expect("reading Moa Therma");
    let road = measure(&blob, &model);

    println!(
        "corner: {} road triangle(s) within {CORNER_RADIUS} units of {CORNER:?}, by vertex type \
         {:?}",
        road.corner.len(),
        road.corner_types
    );
    println!(
        "corner repeat: p05 {:.2}, p50 {:.2}, p95 {:.2}, max {:.2} world units",
        quantile(&road.corner, 0.05),
        quantile(&road.corner, 0.5),
        quantile(&road.corner, 0.95),
        quantile(&road.corner, 1.0)
    );

    assert!(
        !road.corner.is_empty(),
        "no road triangles near {CORNER:?}; the corner moved and this test is measuring nothing"
    );
    // The load-bearing one. `0x139` is the `u8 / 128` encoding whose 0.5-texel
    // step was the leading explanation for the reported wiggle; there is not one
    // such triangle anywhere near where the artifact was photographed, so
    // texture-coordinate quantisation cannot be what is seen there.
    assert_eq!(
        road.corner_types.keys().copied().collect::<Vec<_>>(),
        vec![0x13b],
        "the reported corner is no longer painted only from f32 texture coordinates"
    );
    let median = quantile(&road.corner, 0.5);
    assert!(
        REPEAT_BAND.contains(&median),
        "corner median repeat {median:.2} is outside {REPEAT_BAND:?}"
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_road_texture_is_one_shared_asset_not_a_per_circuit_variant() {
    let Some(image) = image() else { return };
    let mut archives =
        oag_assets::pulse::Archives::open(&image.display().to_string()).expect("opening archives");

    let mut reference: Option<(String, u32, u32, Vec<u8>)> = None;
    for circuit in circuits() {
        let Some((_, model)) = track(&mut archives, &circuit) else {
            continue;
        };
        let Some(texture) = model
            .textures
            .iter()
            .flatten()
            .find(|t| t.label.eq_ignore_ascii_case(ROAD))
        else {
            continue;
        };
        match &reference {
            None => {
                println!(
                    "{circuit}_Track: {ROAD} is {}x{}",
                    texture.width, texture.height
                );
                reference = Some((
                    circuit.clone(),
                    texture.width,
                    texture.height,
                    texture.rgba.clone(),
                ));
            }
            Some((first, width, height, rgba)) => assert!(
                (*width, *height, rgba) == (texture.width, texture.height, &texture.rgba),
                "{circuit}_Track's {ROAD} differs from {first}_Track's, so the circuits do not \
                 share one road texture after all"
            ),
        }
    }
    assert!(reference.is_some(), "no circuit carried {ROAD}");
}
