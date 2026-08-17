//! What a `.rcsmodel` **vertex** holds, checked against the disc.
//!
//! **`#[ignore]`d and never run in CI.** They need game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! The container half is `rcsmodel_ground_truth.rs`; the two were one file
//! until it passed a thousand lines.
//!
//! The vertex stride is in no field of the file and the bytes after a position
//! were undecoded until 2026-08-17, so **every claim here is a reading checked
//! against something the `.rcsmodel` does not state** - the box the `.vex`
//! authors, or the geometry the triangles imply.
//!
//! Five claims:
//!
//! 1. **The recovered stride fills the authored box.** Both how it is found and
//!    the only check that it was found rightly.
//! 2. **The stride the buffer layout gives is the same one** - arithmetic on
//!    two fields of the `.rcsmodel`, needing no `.vex` at all.
//! 3. **What the box-less rules decide stays inside the circuit.** The
//!    unreferenced path has no box to filter a stray submesh against.
//! 4. **The four bytes after a position are the vertex normal.**
//! 5. **What a vertex carries follows the stride**, not the `83 XX` descriptor
//!    byte. A negative result: the byte was the obvious candidate for naming
//!    the field set and it names nothing.

mod rcsmodel_common;

use oag_formats::rcsmodel;
use rcsmodel_common::*;

/// Claim 3: the recovered vertex stride fills the box the `.vex` authors.
///
/// The number to watch is the unresolved count, not the resolved one: a mesh
/// whose stride no single value explains is reported and drawn as nothing, so
/// this test records how much of the disc that is rather than requiring zero.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_recovered_geometry_fills_the_box_the_vex_authors() {
    for (archive, vex_path, model_path) in PAIRS {
        let Some((blob, model_blob)) = pair(archive, vex_path, model_path) else {
            return;
        };
        let model = rcsmodel::Model::parse(&model_blob).expect("the .rcsmodel parses");
        let nodes = mesh_nodes(&blob);

        let mut solved = 0;
        let mut unresolved = Vec::new();
        let mut widths = std::collections::BTreeMap::new();
        let mut addressed = 0;
        for (name, hash, min, max) in &nodes {
            // The nodes claim 1 records as unresolved have no geometry here to
            // solve a stride for; they are that test's finding, not this one's.
            let Some(mesh) = model.mesh(*hash) else {
                continue;
            };
            addressed += 1;
            match mesh.solve_stride(&model_blob, (*min, *max), TOLERANCE) {
                Some(stride) => {
                    solved += 1;
                    *widths.entry(stride).or_insert(0usize) += 1;
                }
                None => unresolved.push(name.as_str()),
            }
        }

        println!(
            "{vex_path}: {solved} of {addressed} addressed meshes solved a stride, \
             widths {widths:?}, unsolved e.g. {:?}",
            &unresolved[..unresolved.len().min(4)]
        );
        assert!(
            solved * 4 >= addressed * 3,
            "{vex_path}: only {solved} of {addressed} meshes recovered a stride"
        );
        for stride in widths.keys() {
            assert!(
                (14..=22).contains(stride) && stride % 2 == 0,
                "{vex_path}: unexpected stride {stride}"
            );
        }
    }
}
/// Claim 4: the stride read out of the file's own buffer layout is the same one
/// the authored box gives, and the same one the compactness rule gives.
///
/// **This is the claim that turned the stride from a heuristic into a
/// reading.** [`rcsmodel::Mesh::solve_stride_by_layout`] measures the step from
/// one submesh's vertex buffer to the next and divides by the vertex count -
/// arithmetic on two numbers the file states outright, decoding nothing. So it
/// is independent of both of the other rules: of the box, which is in a
/// different file, and of the compactness rule, which is about what the bytes
/// look like once read.
///
/// Two agreements, and a **zero** that matters more than either: the rule must
/// never contradict an oracle. It does not.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_stride_the_buffer_layout_gives_is_the_one_the_box_and_the_span_give() {
    let (mut vs_box, mut vs_span, mut disagreements) = (0, 0, 0);
    for (archive, vex_path, model_path) in PAIRS {
        let Some((blob, model_blob)) = pair(archive, vex_path, model_path) else {
            return;
        };
        let model = rcsmodel::Model::parse(&model_blob).expect("the .rcsmodel parses");

        for (_, hash, min, max) in mesh_nodes(&blob) {
            let Some(mesh) = model.mesh(hash) else {
                continue;
            };
            let (Some(layout), Some(boxed)) = (
                mesh.solve_stride_by_layout(),
                mesh.solve_stride(&model_blob, (min, max), TOLERANCE),
            ) else {
                continue;
            };
            vs_box += 1;
            disagreements += usize::from(layout != boxed);
            assert_eq!(
                layout, boxed,
                "{vex_path}: the layout says {layout} where the authored box says {boxed}"
            );
        }

        // Over every chunk, referenced or not - which on a circuit is mostly
        // the unreferenced ones, and the whole reason either rule exists.
        for mesh in &model.meshes {
            let (Some(layout), Some(span)) = (
                mesh.solve_stride_by_layout(),
                mesh.solve_stride_by_extent(&model_blob),
            ) else {
                continue;
            };
            vs_span += 1;
            disagreements += usize::from(layout != span);
            assert_eq!(
                layout, span,
                "{model_path}: the layout says {layout} where the span says {span}"
            );
        }
        let decided = model
            .meshes
            .iter()
            .filter(|m| m.solve_stride_without_a_box(&model_blob).is_some())
            .count();
        let span_only = model
            .meshes
            .iter()
            .filter(|m| m.solve_stride_by_extent(&model_blob).is_some())
            .count();
        println!(
            "{model_path}: {decided} of {} chunks decide a stride, against {span_only} \
             for the span rule alone",
            model.meshes.len()
        );
        assert!(
            decided >= span_only,
            "{model_path}: adding the layout rule decided fewer chunks, not more"
        );
    }
    println!("the layout rule agrees with the box on {vs_box} and with the span on {vs_span}");
    assert_eq!(disagreements, 0);
    assert!(
        vs_box >= 15 && vs_span >= 90,
        "{vs_box} and {vs_span} is too few to mean anything"
    );
}

/// Claim 5: nothing the box-less rules decide lands outside the circuit.
///
/// **The guard the unreferenced path does not otherwise have.** A referenced
/// mesh is filtered submesh by submesh through
/// [`rcsmodel::Mesh::submesh_fits`], because a stride right for a mesh can be
/// wrong for one of its submeshes and that submesh's attribute bytes then come
/// out as positions - trap 2 in `docs/formats/rcsmodel.md`, the one that framed
/// Assegai as a speck. A chunk no node references has no box to filter against.
///
/// So this measures the shape of what those rules produce. A misread submesh
/// reads bytes normalised over the whole `i16` range as positions, which at the
/// `1/128` scale puts points up to 256 units from the chunk's own bias in
/// *every* axis - and a circuit is flat: Talon's Junction's floor collision
/// spans 194 units vertically over 1,476 by 1,088.
///
/// **Measured: not one of the 868 submeshes drawn this way leaves +/-400.** The
/// widest world-space `y` span is 306 units on a chunk only the layout rule
/// decides, 237 where both rules agree and 189 where only compactness does.
/// The layout-only chunks being the largest is expected rather than alarming -
/// the compactness rule needs the true reading to be *decisively* smaller than
/// the wrong one, and that margin is narrowest on a big chunk, so it declines
/// on exactly those.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn nothing_the_box_less_rules_decide_lands_outside_the_circuit() {
    /// Generous against the 306 measured, tight against the 256-per-axis
    /// scatter a misread would produce on a chunk anywhere but the middle.
    const ENVELOPE: f32 = 400.0;

    let Some((blob, model_blob)) = pair(
        "DATA00.PSARC",
        "/data/environments/talons_junction/track.vex",
        "/data/environments/talons_junction/track.rcsmodel",
    ) else {
        return;
    };
    let model = rcsmodel::Model::parse(&model_blob).expect("the .rcsmodel parses");
    let placed: std::collections::BTreeSet<u32> = mesh_nodes(&blob)
        .into_iter()
        .map(|(_, hash, ..)| hash)
        .collect();

    let (mut checked, mut widest) = (0usize, 0.0f32);
    for mesh in &model.meshes {
        if placed.contains(&mesh.hash) {
            continue;
        }
        let Some(stride) = mesh.solve_stride_without_a_box(&model_blob) else {
            continue;
        };
        for submesh in &mesh.submeshes {
            let Ok(points) = mesh.positions(&model_blob, submesh, stride) else {
                continue;
            };
            let (lo, hi) = points.iter().fold((f32::MAX, f32::MIN), |(lo, hi), p| {
                (lo.min(p[1]), hi.max(p[1]))
            });
            if points.is_empty() {
                continue;
            }
            checked += 1;
            widest = widest.max(hi - lo);
            assert!(
                lo > -ENVELOPE && hi < ENVELOPE,
                "a chunk decoded at stride {stride} spans y {lo}..{hi}, which is \
                 not a piece of a circuit"
            );
        }
    }
    println!("{checked} box-less submeshes, widest world y span {widest}");
    assert!(checked >= 800, "only {checked} submeshes reach this path");
}

/// Claim 6: the four bytes after a position are the vertex normal.
///
/// Two assertions, and the second is the one that makes it a normal rather than
/// merely a unit vector:
///
/// 1. **It is a unit vector**, on 99 % or more of every model's vertices. No
///    other reading of any offset in the vertex exceeds 51 %, which is what
///    located the field before anything checked what it meant.
/// 2. **It agrees with the geometry.** The oracle is the area-weighted average
///    of the faces touching each vertex - which the `.rcsmodel` does not state -
///    over meshes more than a unit across, where `1/128` quantisation cannot
///    make the triangles degenerate. The bar is 75 %; the measurement is 82 %,
///    and the shortfall is *expected*: a hard edge is exactly where the
///    exporter splits a vertex and authors a normal no smooth average has.
///
/// The tangent at `+10` is checked the same way and for the same reason - a
/// second unit field that is **perpendicular** to this one is what says the two
/// are not the same quantity read twice.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_four_bytes_after_a_position_are_the_vertex_normal() {
    /// Below this span, `1/128` quantisation makes the face normals noise.
    const MIN_SPAN: f32 = 1.0;

    for (archive, vex_path, model_path) in PAIRS {
        let Some((blob, model_blob)) = pair(archive, vex_path, model_path) else {
            return;
        };
        let model = rcsmodel::Model::parse(&model_blob).expect("the .rcsmodel parses");

        // Per stride, because the three widths are separate claims and stride
        // 14 rests on the least data of the three.
        let mut unit: std::collections::BTreeMap<usize, (usize, usize)> = Default::default();
        let mut agree: std::collections::BTreeMap<usize, (usize, usize)> = Default::default();
        let mut zeroes = 0usize;
        // Split by stride: what sits at `+10` is not the same field on all
        // three, and averaging over them would hide that.
        let mut tangent_dots: std::collections::BTreeMap<usize, Vec<f32>> = Default::default();
        for (_, hash, min, max) in mesh_nodes(&blob) {
            let Some(mesh) = model.mesh(hash) else {
                continue;
            };
            let Some(stride) = mesh.solve_stride(&model_blob, (min, max), TOLERANCE) else {
                continue;
            };
            for submesh in &mesh.submeshes {
                if !mesh.submesh_fits(&model_blob, submesh, stride, (min, max)) {
                    continue;
                }
                let (Ok(points), Ok(indices), Ok(normals)) = (
                    mesh.positions(&model_blob, submesh, stride),
                    mesh.indices(&model_blob, submesh),
                    mesh.normals(&model_blob, submesh, stride),
                ) else {
                    continue;
                };
                for n in &normals {
                    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
                    // **An all-zero word is not a misdecode, it is an unused
                    // vertex.** The records that carry one are zero from the
                    // position onward - `cockpit_screenShape` pads its buffer
                    // that way. `oag_render` reads it as "the file authored no
                    // normal here", derives one from the triangles, and does not
                    // count it as authored; counting it in the statistic below
                    // would be the same mistake in the other direction.
                    if len == 0.0 {
                        zeroes += 1;
                        continue;
                    }
                    let e = unit.entry(stride).or_default();
                    e.1 += 1;
                    if (0.93..=1.07).contains(&len) {
                        e.0 += 1;
                    }
                }
                if span(&points) < MIN_SPAN {
                    continue;
                }
                for (k, want) in smooth_normals(&points, &indices).iter().enumerate() {
                    let (Some(want), Some(got)) = (want, normals.get(k).copied().and_then(unit3))
                    else {
                        continue;
                    };
                    let e = agree.entry(stride).or_default();
                    e.1 += 1;
                    if dot(got, *want) > 0.95 {
                        e.0 += 1;
                    }
                    // The tangent, read the same way the recovery read it.
                    let at = submesh.vertex_offset + k * stride + 10;
                    if stride > 14 && at + 3 <= model_blob.len() {
                        let c = |o: usize| (f32::from(model_blob[at + o]) - 128.0) / 127.0;
                        if let Some(t) = unit3([c(0), c(1), c(2)]) {
                            tangent_dots
                                .entry(stride)
                                .or_default()
                                .push(dot(t, *want).abs());
                        }
                    }
                }
            }
        }

        let medians: std::collections::BTreeMap<usize, f32> = tangent_dots
            .into_iter()
            .map(|(stride, mut d)| {
                d.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
                (stride, d[d.len() / 2])
            })
            .collect();
        let total = |m: &std::collections::BTreeMap<usize, (usize, usize)>| {
            m.values().fold((0, 0), |(a, b), (c, d)| (a + c, b + d))
        };
        let (units, vertices) = total(&unit);
        let (agreed, scored) = total(&agree);
        println!(
            "{model_path}: {units} of {vertices} normals are unit {unit:?}, \
             {agreed} of {scored} within 18 degrees of the smooth average {agree:?}, \
             {zeroes} all-zero; median |dot| of the +10 field with the normal, \
             by stride: {medians:?}"
        );
        // **Per stride, not pooled.** Pooling would let stride 22's 23,000
        // vertices carry a claim about stride 14's few hundred.
        for (&stride, &(hits, n)) in &unit {
            // **90 per stride, 99 overall**, and the gap between the two bars
            // is the honest one. Pooled, 91,376 of 91,480 non-zero normals are
            // unit. Per stride the minority widths are thin and noisier -
            // stride 14 reads 97 % on 206 vertices of Assegai and 100 % on 186
            // of the circuit, stride 22 reads 99.9 % on 23,593 of Assegai and
            // 91.5 % on 437 of the circuit. Setting the per-stride bar by the
            // worst of those keeps the claim honest about which widths carry
            // real evidence and which carry a few hundred vertices.
            assert!(
                hits * 10 >= n * 9,
                "{model_path}: at stride {stride}, only {hits} of {n} decode to a unit vector"
            );
        }
        for (&stride, &(hits, n)) in &agree {
            assert!(
                hits * 4 >= n * 3,
                "{model_path}: at stride {stride}, only {hits} of {n} agree with the geometry"
            );
        }
        assert!(
            units * 100 >= vertices * 99,
            "{units} of {vertices} overall"
        );
        // **Nothing about `+10` is asserted, and that is the finding.** On
        // Assegai's hull it is perpendicular to the normal to a median `|dot|`
        // of 0.02-0.04 over 23,000 vertices, which is a tangent. On its LOD1 and
        // on a circuit the same bytes come out at 0.577 - which is `1/sqrt(3)`,
        // exactly what a *constant* `(-1,-1,-1)` direction scores against any
        // axis-aligned normal, so those records hold `00 00 00` there and the
        // field is something else. What `+10` is in general is unrecovered, and
        // the number is printed rather than asserted so a future reading has the
        // measurement to start from. See `docs/formats/rcsmodel.md`.
        let _ = &medians;
    }
}

/// Claim 7: what a vertex carries follows the **stride**, and the descriptor
/// byte does not name it.
///
/// The four bytes at `+0x0a` are the worked case, because they are two
/// different things on two widths:
///
/// - **On stride 22 they are a tangent.** Unit on 90 % or more of every group
///   and perpendicular to the vertex's own normal at a median `|dot|` under
///   0.01, over 78,456 vertices.
/// - **On stride 18 they are not.** Unit on 7 to 59 %, and a median `|dot|` of
///   0.52 to 0.57 - `1/sqrt(3)` is 0.577, which is what a constant `(-1,-1,-1)`
///   scores against any axis-aligned normal, so a large share of those records
///   are `00 00 00` there. What the field *is* on stride 18 is unrecovered.
///
/// **The point of grouping by `83 XX` is the negative result.** That byte runs
/// `07` to `0d` and was the obvious candidate for naming the field set, since
/// it is the only part of the descriptor that varies and is already known not
/// to determine the stride. It does not: every `XX` group at stride 22 behaves
/// like a tangent and every `XX` group at stride 18 does not, so the split is
/// the width's and the byte explains nothing. Naming what it *does* select is
/// still open - `docs/formats/rcsmodel.md`.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_field_after_the_normal_follows_the_stride_and_not_the_descriptor_byte() {
    /// Below this many vertices a group is noise rather than a measurement.
    const ENOUGH: usize = 500;

    // (stride, descriptor byte) -> (unit count, total, |dot| with the normal)
    let mut groups: std::collections::BTreeMap<(usize, u8), (usize, usize, Vec<f32>)> =
        Default::default();

    for (archive, vex_path, model_path) in PAIRS {
        let Some((blob, model_blob)) = pair(archive, vex_path, model_path) else {
            return;
        };
        let model = rcsmodel::Model::parse(&model_blob).expect("the .rcsmodel parses");
        let boxes: std::collections::BTreeMap<u32, ([f32; 3], [f32; 3])> = mesh_nodes(&blob)
            .into_iter()
            .map(|(_, hash, min, max)| (hash, (min, max)))
            .collect();

        for mesh in &model.meshes {
            let stride = match boxes.get(&mesh.hash) {
                Some(&bounds) => mesh.solve_stride(&model_blob, bounds, TOLERANCE),
                None => mesh.solve_stride_without_a_box(&model_blob),
            };
            // Stride 14 has no field here at all: its 8 attribute bytes are the
            // normal and the last four, with nothing between them.
            let Some(stride) = stride.filter(|&s| s > 14) else {
                continue;
            };
            for submesh in &mesh.submeshes {
                let Ok(normals) = mesh.normals(&model_blob, submesh, stride) else {
                    continue;
                };
                let entry = groups.entry((stride, submesh.format[1])).or_default();
                for (k, n) in normals.iter().enumerate() {
                    let at = submesh.vertex_offset + k * stride + 10;
                    let Some(raw) = model_blob.get(at..at + 3) else {
                        continue;
                    };
                    entry.1 += 1;
                    let c = |o: usize| (f32::from(raw[o]) - 128.0) / 127.0;
                    let v = [c(0), c(1), c(2)];
                    let len = dot(v, v).sqrt();
                    if !(0.93..=1.07).contains(&len) {
                        continue;
                    }
                    entry.0 += 1;
                    if let (Some(t), Some(n)) = (unit3(v), unit3(*n)) {
                        entry.2.push(dot(t, n).abs());
                    }
                }
            }
        }
    }

    let mut checked = 0;
    for ((stride, xx), (unit, total, mut dots)) in groups {
        if total < ENOUGH {
            continue;
        }
        dots.sort_by(|a, b| a.partial_cmp(b).expect("no NaN"));
        let median = dots.get(dots.len() / 2).copied().unwrap_or(f32::NAN);
        let unit_pct = 100.0 * unit as f32 / total as f32;
        println!(
            "stride {stride}, descriptor 83 {xx:02x}: {total:6} vertices, \
             {unit_pct:5.1} % unit, median |dot| with the normal {median:.3}"
        );
        checked += 1;
        if stride == 22 {
            assert!(
                unit_pct >= 90.0 && median < 0.1,
                "stride 22 group 83 {xx:02x} does not look like a tangent \
                 ({unit_pct} % unit, median |dot| {median})"
            );
        } else {
            assert!(
                median > 0.4,
                "stride 18 group 83 {xx:02x} looks like a tangent after all \
                 (median |dot| {median}), which would make the descriptor byte \
                 the thing that selects the field"
            );
        }
    }
    assert!(
        checked >= 8,
        "only {checked} groups were big enough to measure"
    );
}
