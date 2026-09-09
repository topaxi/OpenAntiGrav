//! Why Moa Therma's magstrip lines wobble on PSP assets: the strip is drawn
//! twice, and the two copies disagree.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//! One test additionally needs the PS2 disc and skips without it.
//!
//! ```sh
//! just test-data
//! # or only this crate:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all
//! ```
//!
//! # What this is for
//!
//! The user-reported artifact on Moa Therma's loop - magstrip lines chopped
//! into segments that step sideways, at native 480x272, where the PS2 build
//! draws them smooth through the same renderer - turned out not to be the u8
//! texture-coordinate quantisation this file's sibling
//! (`road_uv_ground_truth.rs`) ruled out at the photographed corner. It is a
//! z-fight between two coincident copies of the strip:
//!
//! - The **base strip**, pass `0x1021`/`0x1001`, maps the texture `2/128 ..
//!   125/128` on both axes, ping-ponging U (the along-track axis) every tile.
//! - One **overlay copy**, pass `0x0021`, maps the *same* geometry `0 .. 1`
//!   (`0/128 .. 128/128`), coarser: one quad across where the base has five
//!   rows.
//!
//! At every shared position the two mappings disagree by 2-3 u8 steps, which
//! at the strip's V scale (~33 world units per repeat, V runs *across* the
//! road here) is ~0.5-0.8 world units of sideways texture shift. Drawn as
//! ordinary coincident opaque geometry, the depth-test winner alternates per
//! screen region and every line steps sideways where it flips. Skipping the
//! `0x0021` batches removes the artifact entirely; quantising the PS2 build's
//! f32 texture coordinates onto the PSP's u8 grid does *not* reproduce it
//! (sub-pixel line shifts only). Both were established by A/B captures at
//! `--pose 88.2,97.1,-462.3` - see `HANDOVER.md`.
//!
//! # Why the original never shows it
//!
//! The overlay is not an effect: it is the track's **far-LOD copy**, and the
//! original hides it with the authored PVS. The copy's meshes live in a
//! sibling group governed by their `section` node
//! ([`oag_formats::pvs::governing_sections`]) - id 62 on this track, named
//! `_59_TRACK_07_LOD` - and no section the craft races through lists 62 in
//! its visibility mask; only four distant vantage sections do. Our renderer
//! used to place draw calls in sections *geometrically*, and the copy sits
//! inside the racing sections' boxes, so it could never be hidden that way;
//! placement by the authored group reproduces the original's behaviour, and
//! [`far_lod_sections`] pins the data this rests on. The
//! quantisation-vs-master truncation measured below is real but sub-pixel -
//! the honest residual difference between the builds, not the artifact.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use oag_formats::vex;
use oag_render::mesh::{self, Model};

/// The magstrip surface texture, by artist-given label. PSP embeds labels;
/// the PS2 build's textures are nameless (external texture sets), so PS2
/// magstrip geometry is identified positionally, never by this.
const MAGSTRIP: &str = "_magsurface3_1verb.tga";

/// Moa Therma - `Data\Environments\03_Track` on both discs.
const MOA_THERMA: &str = "03";

/// A point on the loop's driving surface, from the user's telemetry
/// (spline distance 12.5), and the radius this file calls "the loop".
const LOOP: [f32; 3] = [88.2, 97.1, -462.3];
const LOOP_RADIUS: f32 = 45.0;

/// The magstrip overlay's pass mask: list A (`0x1`) plus back-face culling
/// (`0x20`), and - uniquely among magstrip batches - no bit `0x1000`.
const OVERLAY_PASS: u16 = 0x0021;

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

fn open(image: &Path) -> oag_assets::Archives {
    oag_pulse::open(&image.display().to_string()).expect("opening archives")
}

/// Reads a circuit's track model, with the PS2 external-texture fallback the
/// game itself uses (`crates/game/src/race.rs`).
fn track(archives: &mut oag_assets::Archives, circuit: &str) -> Option<(Vec<u8>, Model)> {
    let name = format!(r"Data\Environments\{circuit}_Track\track.vex");
    let blob = archives.read_name(&name).ok()?;
    let mut model = mesh::build(&name, &blob).ok()?;
    if !model.textures.is_empty() && model.textures.iter().all(Option::is_none) {
        let set = archives.read_preceding(&name).ok()?;
        let external = mesh::Ps2TextureSet::parse(&set).ok()?;
        model = mesh::build_with_textures(&name, &blob, Some(&external), mesh::Lod::Both).ok()?;
    }
    Some((blob, model))
}

fn is_magstrip(model: &Model, texture: Option<usize>) -> bool {
    texture
        .and_then(|t| model.textures.get(t))
        .and_then(Option::as_ref)
        .is_some_and(|t| t.label.eq_ignore_ascii_case(MAGSTRIP))
}

/// Every magstrip batch of a track, with its owning node's world transform.
fn magstrip_batches(blob: &[u8], model: &Model) -> Vec<([f32; 16], vex::Batch)> {
    let nodes = vex::nodes(blob).expect("decoding nodes");
    let world = vex::world_transforms(blob, &nodes);
    let mut out = Vec::new();
    for (index, node) in nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.class_id == 0x125)
    {
        let payload = &blob[node.payload()];
        let materials = vex::mesh_materials(payload);
        for list in [0u8, 1u8] {
            for batch in vex::mesh_batches(payload, list).expect("decoding batches") {
                let texture = materials
                    .get(usize::from(batch.material_index))
                    .copied()
                    .flatten()
                    .map(|m| m.texture as usize);
                if is_magstrip(model, texture) {
                    out.push((world[index], batch));
                }
            }
        }
    }
    out
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

/// World units covered by one whole texture repeat along V, per triangle -
/// the same instrument as `road_uv_ground_truth.rs`, and the right one for
/// the same reason: the magstrip texture is uniform along its rows (12x more
/// variation along V than along U, asserted below), so only `|grad V|`
/// decides where a painted line lands.
///
/// The axes' *meanings* differ from the road, though: on the magstrip V runs
/// **across** the road and U along it, so this measures the across-road
/// scale, and a one-LSB V error moves a line sideways by `repeat / 128`.
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
    let j_v = [
        d1[1] * inv[0][0] + d2[1] * inv[1][0],
        d1[1] * inv[0][1] + d2[1] * inv[1][1],
    ];
    let gradient_v = (j_v[0] * j_v[0] + j_v[1] * j_v[1]).sqrt();
    (gradient_v > 1e-9).then(|| 1.0 / gradient_v)
}

fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    sorted[((sorted.len() - 1) as f64 * q).round() as usize]
}

fn sort(v: &mut [f64]) {
    v.sort_by(|a, b| a.partial_cmp(b).expect("no NaN repeats"));
}

/// The band every other circuit's magstrip median falls in: 15.30 (Talon's
/// Junction) to 25.24 (Vertica).
const SHARED_BAND: std::ops::Range<f64> = 14.0..27.0;

/// Moa Therma's magstrip median: 31.27 across the circuit, 32.76 at the
/// loop - the coarsest of any circuit, which is why one u8 LSB is worth the
/// most world units exactly where the artifact was reported (~0.26 across).
const MOA_BAND: std::ops::Range<f64> = 30.0..34.0;

#[test]
#[ignore = "needs a disc image in data/images/"]
fn moa_therma_lays_the_magstrip_coarsest_and_from_u8_texture_coordinates() {
    let Some(psp) = image("pulse-psp-usa.chd") else {
        return;
    };
    let mut archives = open(&psp);
    let mut measured = 0;
    for circuit in (1..=20u32).map(|n| format!("{n:02}")) {
        let Some((blob, model)) = track(&mut archives, &circuit) else {
            continue;
        };
        let batches = magstrip_batches(&blob, &model);
        if batches.is_empty() {
            continue;
        }
        measured += 1;

        let mut by_type: BTreeMap<u16, Vec<f64>> = BTreeMap::new();
        let mut loop_reps: Vec<f64> = Vec::new();
        for (to_world, batch) in &batches {
            let positions: Vec<[f32; 3]> = batch
                .vertices
                .iter()
                .map(|v| vex::transform_point(to_world, v.position))
                .collect();
            for tri in batch.triangles() {
                let i = [tri[0] as usize, tri[1] as usize, tri[2] as usize];
                let p = [positions[i[0]], positions[i[1]], positions[i[2]]];
                let uv = i.map(|k| batch.vertices[k].texcoord.unwrap_or([0.0; 2]));
                let Some(repeat) = triangle_repeat(p, uv) else {
                    continue;
                };
                by_type.entry(batch.vertex_type).or_default().push(repeat);
                let centre = [
                    (p[0][0] + p[1][0] + p[2][0]) / 3.0,
                    (p[0][1] + p[1][1] + p[2][1]) / 3.0,
                    (p[0][2] + p[1][2] + p[2][2]) / 3.0,
                ];
                let away = ((centre[0] - LOOP[0]).powi(2)
                    + (centre[1] - LOOP[1]).powi(2)
                    + (centre[2] - LOOP[2]).powi(2))
                .sqrt();
                if circuit == MOA_THERMA && away < LOOP_RADIUS {
                    loop_reps.push(repeat);
                }
            }
        }

        let mut all: Vec<f64> = by_type.values().flatten().copied().collect();
        sort(&mut all);
        let median = quantile(&all, 0.5);
        for (vertex_type, reps) in &mut by_type {
            sort(reps);
            println!(
                "{circuit}_Track {vertex_type:#06x}: {} triangle(s), repeat p50 {:.2}, p95 {:.2}",
                reps.len(),
                quantile(reps, 0.5),
                quantile(reps, 0.95)
            );
        }
        println!(
            "{circuit}_Track: {} magstrip triangle(s), repeat p50 {median:.2}",
            all.len()
        );

        let (band, whose) = if circuit == MOA_THERMA {
            (MOA_BAND, "Moa Therma's own")
        } else {
            (SHARED_BAND, "the shared")
        };
        assert!(
            band.contains(&median),
            "{circuit}_Track: magstrip median repeat {median:.2} outside {whose} band {band:?}"
        );

        if circuit == MOA_THERMA {
            // The u8 encoding dominates, so the quantisation question - ruled
            // out at the corner, where the road is f32 - was genuinely live
            // here and had to be answered by measurement, not carried over.
            let u8_triangles = by_type.get(&0x139).map_or(0, Vec::len);
            let f32_triangles = by_type.get(&0x13b).map_or(0, Vec::len);
            assert!(
                u8_triangles > 10 * f32_triangles,
                "Moa Therma's magstrip is no longer overwhelmingly u8-encoded \
                 ({u8_triangles} against {f32_triangles})"
            );
            sort(&mut loop_reps);
            let loop_median = quantile(&loop_reps, 0.5);
            println!(
                "loop: {} triangle(s), repeat p50 {loop_median:.2}",
                loop_reps.len()
            );
            assert!(
                !loop_reps.is_empty() && MOA_BAND.contains(&loop_median),
                "the loop's magstrip median {loop_median:.2} left {MOA_BAND:?}; the artifact \
                 site no longer measures as recorded"
            );
        }
    }
    assert!(
        measured >= 5,
        "only {measured} circuit(s) carried {MAGSTRIP}; the census lost its subject"
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_magstrip_texture_paints_its_features_across_v_not_along_u() {
    let Some(psp) = image("pulse-psp-usa.chd") else {
        return;
    };
    let mut archives = open(&psp);
    let (_, model) = track(&mut archives, MOA_THERMA).expect("reading Moa Therma");
    let texture = model
        .textures
        .iter()
        .flatten()
        .find(|t| t.label.eq_ignore_ascii_case(MAGSTRIP))
        .expect("magstrip texture");
    let (w, h) = (texture.width as usize, texture.height as usize);
    // A PSP `.vex` texture is always decoded RGBA8; only Wipeout HD's `.gtf`
    // keeps its blocks. See `oag_render::mesh::Texels`.
    let rgba = texture.rgba().expect("a .vex texture is decoded RGBA8");
    let luma = |x: usize, y: usize| {
        let o = (y * w + x) * 4;
        let p = &rgba[o..o + 3];
        (u32::from(p[0]) + u32::from(p[1]) + u32::from(p[2])) / 3
    };
    let mut along_u = 0u64;
    let mut along_v = 0u64;
    for y in 0..h {
        for x in 0..w {
            along_u += u64::from(luma((x + 1) % w, y).abs_diff(luma(x, y)));
            along_v += u64::from(luma(x, (y + 1) % h).abs_diff(luma(x, y)));
        }
    }
    println!("{MAGSTRIP}: {w}x{h}, variation along U {along_u}, along V {along_v}");
    // Rows are near-uniform, columns are not: the painted features (edge
    // bands, centre dashes, the diagonal) sit at fixed V. This is what makes
    // a V disagreement a *sideways* line shift on the mesh, where V runs
    // across the road - and it is why measuring V repeat is the right
    // instrument in the census above.
    assert!(
        along_v > 5 * along_u,
        "the magstrip texture no longer varies chiefly along V \
         ({along_v} against {along_u}); the sideways-step reasoning would not hold"
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn moa_therma_alone_draws_its_magstrip_twice_and_the_copies_disagree() {
    let Some(psp) = image("pulse-psp-usa.chd") else {
        return;
    };
    let mut archives = open(&psp);
    for circuit in (1..=20u32).map(|n| format!("{n:02}")) {
        let Some((blob, model)) = track(&mut archives, &circuit) else {
            continue;
        };
        let batches = magstrip_batches(&blob, &model);
        if batches.is_empty() {
            continue;
        }

        let overlay_batches = batches
            .iter()
            .filter(|(_, b)| b.pass_mask == OVERLAY_PASS)
            .count();

        // Vertices keyed by exact position bits; the overlay duplicates the
        // base's positions exactly (same s16 grid, same transforms), so
        // coincident copies collide without any tolerance.
        let mut by_position: BTreeMap<[u32; 3], Vec<(bool, [f32; 2])>> = BTreeMap::new();
        for (to_world, batch) in &batches {
            let overlay = batch.pass_mask == OVERLAY_PASS;
            for v in &batch.vertices {
                let p = vex::transform_point(to_world, v.position);
                let key = [p[0].to_bits(), p[1].to_bits(), p[2].to_bits()];
                let uv = v.texcoord.unwrap_or([0.0; 2]);
                let entry = by_position.entry(key).or_default();
                if !entry.contains(&(overlay, uv)) {
                    entry.push((overlay, uv));
                }
            }
        }

        // Coincident vertices whose texture coordinates differ by at least
        // one u8 step, split by whether the disagreement crosses the
        // overlay/base boundary.
        let mut across = 0usize;
        let mut within = 0usize;
        let mut residuals: Vec<f64> = Vec::new();
        for copies in by_position.values() {
            for (i, (overlay_a, uv_a)) in copies.iter().enumerate() {
                for (overlay_b, uv_b) in &copies[i + 1..] {
                    let du = f64::from(uv_a[0] - uv_b[0]).abs() * 128.0;
                    let dv = f64::from(uv_a[1] - uv_b[1]).abs() * 128.0;
                    if du.max(dv) < 1.0 {
                        continue;
                    }
                    if overlay_a == overlay_b {
                        within += 1;
                        continue;
                    }
                    across += 1;
                    // The two mappings are one affine relation apart: in u8
                    // steps, `base = overlay * 123/128 + 2`, with two
                    // wrinkles that are both texture-addressing equivalences
                    // rather than disagreements. The base ping-pongs U
                    // (mirrors the texture every tile along the track) while
                    // the overlay tiles it forward through the u8 encoding's
                    // 0..2.0 range and lets the sampler wrap, so a shared
                    // position may need `overlay mod 128` and may pair one
                    // copy's tile start with the other's tile end - the
                    // mirrored relation, `128 - overlay` in place of
                    // `overlay`. Truncation to the u8 grid allows about one
                    // step of residual on top.
                    let (base, overlay) = if *overlay_a {
                        (uv_b, uv_a)
                    } else {
                        (uv_a, uv_b)
                    };
                    for axis in 0..2 {
                        let base = f64::from(base[axis]) * 128.0;
                        let overlay = (f64::from(overlay[axis]) * 128.0).rem_euclid(128.0);
                        let direct = overlay * (123.0 / 128.0) + 2.0;
                        let mirrored = (128.0 - overlay) * (123.0 / 128.0) + 2.0;
                        residuals.push((base - direct).abs().min((base - mirrored).abs()));
                    }
                }
            }
        }
        println!(
            "{circuit}_Track: {} magstrip batch(es), {overlay_batches} in pass {OVERLAY_PASS:#06x}; \
             disagreeing coincident pairs: {across} overlay-vs-base, {within} within one copy",
            batches.len()
        );

        if circuit == MOA_THERMA {
            assert!(
                overlay_batches >= 5,
                "Moa Therma's magstrip overlay pass went missing \
                 ({overlay_batches} batches in {OVERLAY_PASS:#06x})"
            );
            assert!(
                across >= 50,
                "only {across} disagreeing coincident overlay/base pairs; the z-fight \
                 evidence this file documents is gone"
            );
            // The disagreement is one affine mapping shift, not noise: the
            // base maps the texture `2/128..125/128` where the overlay maps
            // `0..1`, so at the strip's edges - where the painted edge bands
            // sit - the copies are 2-3 u8 steps apart, which at the loop's
            // ~33-unit V repeat is ~0.5-0.8 world units of sideways line
            // shift.
            sort(&mut residuals);
            let worst = quantile(&residuals, 1.0);
            println!("worst affine residual across {across} pair(s): {worst:.2} u8 steps");
            assert!(
                worst <= 1.1,
                "an overlay/base pair strays {worst:.2} u8 steps from \
                 `base = overlay * 123/128 + 2/128`; the two copies no longer differ by \
                 the one affine relation this file documents"
            );
        } else {
            assert_eq!(
                (overlay_batches, across),
                (0, 0),
                "{circuit}_Track now also draws its magstrip twice; the \"only Moa Therma\" \
                 scoping in this file and HANDOVER.md is stale"
            );
        }
    }
}

/// The far-LOD story, pinned: the overlay's group is governed by a section
/// no racing section can see, while its geometry sits inside the racing
/// sections' boxes - which is exactly why placement must be authored, not
/// spatial.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn far_lod_sections() {
    let Some(psp) = image("pulse-psp-usa.chd") else {
        return;
    };
    let mut archives = open(&psp);
    let (blob, model) = track(&mut archives, MOA_THERMA).expect("reading Moa Therma");
    let nodes = vex::nodes(&blob).expect("decoding nodes");
    let pvs = oag_formats::pvs::TrackPvs::from_nodes(&blob, &nodes).expect("parsing sections");
    let governing = oag_formats::pvs::governing_sections(&blob, &nodes).expect("governance");

    // Every overlay batch's node is governed by one section; the base strip's
    // nodes are governed by others.
    let mut overlay_sections = std::collections::BTreeSet::new();
    let mut base_sections = std::collections::BTreeSet::new();
    for (index, node) in nodes
        .iter()
        .enumerate()
        .filter(|(_, n)| n.class_id == 0x125)
    {
        let payload = &blob[node.payload()];
        let materials = vex::mesh_materials(payload);
        for list in [0u8, 1u8] {
            for batch in vex::mesh_batches(payload, list).expect("decoding batches") {
                let texture = materials
                    .get(usize::from(batch.material_index))
                    .copied()
                    .flatten()
                    .map(|m| m.texture as usize);
                if !is_magstrip(&model, texture) {
                    continue;
                }
                if batch.pass_mask == OVERLAY_PASS {
                    overlay_sections.insert(governing[index]);
                } else {
                    base_sections.insert(governing[index]);
                }
            }
        }
    }
    println!("overlay groups governed by {overlay_sections:?}, base by {base_sections:?}");
    assert_eq!(
        overlay_sections.len(),
        1,
        "the far-LOD copy no longer sits in one authored group"
    );
    let far_lod = overlay_sections
        .into_iter()
        .next()
        .flatten()
        .expect("the far-LOD group has a governing section");
    assert!(pvs.declares(far_lod));
    assert!(
        !base_sections.contains(&Some(far_lod)),
        "the base strip shares the far-LOD group's section, so hiding one hides both"
    );

    // The trap that broke geometric placement: the far-LOD section's own box
    // *contains* the loop pose...
    let bounds = pvs.bounds_of(far_lod).expect("the far-LOD section's box");
    assert!(
        bounds.contains(LOOP),
        "the far-LOD box no longer overlaps the loop; the geometric-placement \
         trap this file documents is gone"
    );
    // ...while the section the craft is actually in cannot see it.
    let racing = pvs.section_at(LOOP).expect("a section at the loop pose");
    println!("racing section {racing}, far-LOD section {far_lod}");
    assert_ne!(racing, far_lod);
    assert_eq!(
        pvs.visible_from(racing) & (1u64 << far_lod),
        0,
        "the racing section now sees the far-LOD section, so the original would \
         z-fight too and this file's whole explanation is wrong"
    );

    // The straddle, on real data: the racing line crosses from section 51
    // (whose mask includes the far-LOD copy) into section 26 (whose mask
    // includes the detail) just before the loop, and while the craft leads
    // the camera across that boundary a plain union of their masks shows
    // both halves of the swap. The swap table keeps the craft's side.
    let governing = oag_formats::pvs::governing_sections(&blob, &nodes).expect("governance");
    let (sections, _) = oag_render::pvs::DrawSections::place(&model, &governing, &pvs);
    let swaps = oag_render::pvs::SwapConflicts::find(&pvs, &sections, &model);
    println!("{} LOD-swap pair(s) on this track", swaps.pair_count());
    assert!(
        swaps.partners_of(1u64 << far_lod) != 0,
        "the far-LOD section no longer registers as anyone's swap partner"
    );

    let (craft, camera) = (26u8, 51u8);
    assert!(pvs.visible_from(camera) & (1u64 << far_lod) != 0);
    let padding = oag_render::pvs::SectionPadding::default();
    let unfiltered = oag_render::pvs::VisibleSet::around(
        &pvs,
        &padding,
        &oag_render::pvs::SwapConflicts::none(),
        craft,
        camera,
    );
    let detail_bits: u64 = base_sections
        .iter()
        .flatten()
        .fold(0, |acc, &id| acc | (1u64 << id));
    assert!(
        unfiltered.allows(1u64 << far_lod) && unfiltered.allows(detail_bits),
        "the unfiltered straddle no longer shows both halves; if this stops \
         holding, the swap table may be dead weight"
    );
    let filtered = oag_render::pvs::VisibleSet::around(&pvs, &padding, &swaps, craft, camera);
    assert!(
        !filtered.allows(1u64 << far_lod),
        "the swap table no longer keeps the far-LOD copy out of the straddle set"
    );
    assert!(
        filtered.allows(detail_bits),
        "the craft's own detail must survive the filter"
    );
}

#[test]
#[ignore = "needs both discs in data/images/"]
fn psp_magstrip_texture_coordinates_are_the_floor_of_the_ps2_masters() {
    let (Some(psp), Some(ps2)) = (image("pulse-psp-usa.chd"), image("pulse-ps2-eu.chd")) else {
        return;
    };

    // PSP base-strip vertices near the loop (the overlay's mapping is a
    // different question, answered above).
    let mut archives = open(&psp);
    let (blob, model) = track(&mut archives, MOA_THERMA).expect("reading PSP Moa Therma");
    let mut psp_verts: Vec<([f32; 3], [f32; 2])> = Vec::new();
    for (to_world, batch) in &magstrip_batches(&blob, &model) {
        if batch.pass_mask == OVERLAY_PASS {
            continue;
        }
        for v in &batch.vertices {
            let p = vex::transform_point(to_world, v.position);
            let away =
                ((p[0] - LOOP[0]).powi(2) + (p[1] - LOOP[1]).powi(2) + (p[2] - LOOP[2]).powi(2))
                    .sqrt();
            let uv = v.texcoord.unwrap_or([0.0; 2]);
            if away < LOOP_RADIUS && !psp_verts.contains(&(p, uv)) {
                psp_verts.push((p, uv));
            }
        }
    }

    // Every PS2 vertex near the loop, from every texture: the PS2 build's
    // textures have no labels, so the magstrip is identified by coincidence
    // of position, and the texture ordinal of the matches is recorded to
    // check they all landed on one surface.
    let mut archives = open(&ps2);
    let (blob, _) = track(&mut archives, MOA_THERMA).expect("reading PS2 Moa Therma");
    let nodes = vex::nodes(&blob).expect("decoding nodes");
    let world = vex::world_transforms(&blob, &nodes);
    let mut ps2_verts: Vec<([f32; 3], [f32; 2], usize)> = Vec::new();
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
                let Some(texture) = materials
                    .get(usize::from(batch.material_index))
                    .copied()
                    .flatten()
                    .map(|m| m.texture as usize)
                else {
                    continue;
                };
                for v in &batch.vertices {
                    let p = vex::transform_point(&to_world, v.position);
                    let away = ((p[0] - LOOP[0]).powi(2)
                        + (p[1] - LOOP[1]).powi(2)
                        + (p[2] - LOOP[2]).powi(2))
                    .sqrt();
                    if away < LOOP_RADIUS {
                        ps2_verts.push((p, v.texcoord.unwrap_or([0.0; 2]), texture));
                    }
                }
            }
        }
    }

    // Match by position. The PSP quantises positions to an s16 grid (~0.065
    // units here), the PS2 stores f32, so the same authored vertex sits up to
    // ~0.056 apart; 0.05 accepts only the pairs whose quantisation offset
    // happened to be small, which is exactly the population where the UV
    // comparison is cleanest.
    const EPS: f32 = 0.05;
    let mut matched = 0usize;
    let mut textures: BTreeMap<usize, usize> = BTreeMap::new();
    let mut deltas: Vec<f64> = Vec::new();
    let mut floor_witnesses = 0usize;
    for (p, uv) in &psp_verts {
        let best = ps2_verts
            .iter()
            .map(|(q, w, t)| {
                let d =
                    ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2) + (p[2] - q[2]).powi(2)).sqrt();
                (d, w, t)
            })
            .min_by(|a, b| a.0.partial_cmp(&b.0).expect("no NaN"));
        let Some((d, w, t)) = best else { continue };
        if d >= EPS {
            continue;
        }
        matched += 1;
        *textures.entry(*t).or_default() += 1;
        for axis in 0..2 {
            let delta = f64::from(uv[axis] - w[axis]) * 128.0;
            deltas.push(delta);
            // The half-open bound is the point: a value like the PS2's 63.75
            // stored as 63 rather than 64 is truncation's signature, and
            // rounding's refutation.
            if delta <= -0.7 {
                floor_witnesses += 1;
            }
        }
    }
    println!(
        "{matched} of {} PSP base-strip vertices matched within {EPS} units, on PS2 texture(s) \
         {textures:?}",
        psp_verts.len()
    );
    sort(&mut deltas);
    println!(
        "PSP minus PS2, u8 steps: min {:.3}, p50 {:.3}, max {:.3}; {floor_witnesses} \
         axis value(s) truncated by more than 0.7 steps",
        quantile(&deltas, 0.0),
        quantile(&deltas, 0.5),
        quantile(&deltas, 1.0)
    );

    assert!(
        matched >= 10,
        "only {matched} cross-platform vertex matches; the comparison lost its subject"
    );
    assert_eq!(
        textures.len(),
        1,
        "matches landed on {textures:?} - more than one PS2 surface, so position \
         coincidence no longer isolates the magstrip"
    );
    // Truncation, not rounding, and no decode error: every PSP value sits at
    // or below its PS2 master, by less than one whole u8 step.
    let (lo, hi) = (quantile(&deltas, 0.0), quantile(&deltas, 1.0));
    assert!(
        lo > -1.0 && hi <= 0.0,
        "PSP-minus-PS2 texture-coordinate deltas span {lo:.3}..{hi:.3} u8 steps; they are \
         no longer uniformly floor(master)"
    );
    assert!(
        floor_witnesses > 0,
        "no matched value was truncated by more than 0.7 steps, so this data cannot \
         distinguish floor from round-to-nearest any more"
    );
}

/// **The magstrip does not animate.** Neither its material carries the
/// texture-transform gate (`flags & 0x10`, see
/// [`vex::mesh_materials`]) nor does either magstrip material have a
/// keyframe block at all - both come back `None` from
/// [`vex::mesh_tex_transforms`]. Closes the handover thread's "magstrip's own
/// animation, if any" question as a negative result: the mechanism exists in
/// the engine and elsewhere on this disc (`docs/formats/vex.md`'s
/// texture-transform keyframe block, already implemented as
/// `oag_render::mesh_render::TexAnims`), the magstrip's own two materials
/// just do not use it.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_magstrip_material_carries_no_texture_transform() {
    let Some(psp) = image("pulse-psp-usa.chd") else {
        return;
    };
    let mut archives = open(&psp);
    let (blob, model) = track(&mut archives, MOA_THERMA).expect("reading PSP Moa Therma");
    let nodes = vex::nodes(&blob).expect("decoding nodes");

    let mut checked = 0usize;
    for node in nodes.iter().filter(|n| n.class_id == 0x125) {
        let payload = &blob[node.payload()];
        let materials = vex::mesh_materials(payload);
        let transforms = vex::mesh_tex_transforms(payload);
        for list in [0u8, 1u8] {
            for batch in vex::mesh_batches(payload, list).expect("decoding batches") {
                let mat_index = usize::from(batch.material_index);
                let material = materials.get(mat_index).copied().flatten();
                let texture = material.map(|m| m.texture as usize);
                if !is_magstrip(&model, texture) {
                    continue;
                }
                checked += 1;
                let flags = material.map_or(0, |m| m.flags);
                assert_eq!(
                    flags & 0x10,
                    0,
                    "magstrip material {mat_index} (pass_mask {:#06x}) now carries the \
                     texture-transform flag - re-check whether it animates",
                    batch.pass_mask
                );
                assert!(
                    transforms.get(mat_index).cloned().flatten().is_none(),
                    "magstrip material {mat_index} (pass_mask {:#06x}) now has keyframe \
                     data despite lacking the flag",
                    batch.pass_mask
                );
            }
        }
    }
    assert!(
        checked >= 2,
        "only {checked} magstrip batch(es) checked - both the base strip and the \
         far-LOD overlay should be here"
    );
}
