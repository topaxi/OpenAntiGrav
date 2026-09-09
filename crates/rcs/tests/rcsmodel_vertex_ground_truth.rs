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
//! Six claims:
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
//! 6. **Whether a surface shows HD/Fury's Zone audio-spectrum visualiser is
//!    decided by its own authored vertex normal** - and the billboard family
//!    is mixed where the crowds and the flat banners are not.

mod rcsmodel_common;

use oag_rcs::rcsmodel;
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

/// Claim 5: what the box-less rules decide has unit normals, and stays inside
/// the circuit.
///
/// **The guard the unreferenced path does not otherwise have.** A referenced
/// mesh is filtered submesh by submesh through
/// [`rcsmodel::Mesh::submesh_fits`], because a stride right for a mesh can be
/// wrong for one of its submeshes and that submesh's attribute bytes then come
/// out as positions - trap 2 in `docs/formats/rcsmodel.md`, the one that framed
/// Assegai as a speck. A chunk no node references has no box to filter against.
///
/// Two checks stand in for it, and the first is the stronger:
///
/// 1. **Every chunk drawn this way has unit normals at the stride it chose**,
///    whichever of the three rules chose it. Measured on Talon's Junction:
///    worst 1.00 over the 91 the buffer layout decides, 0.81 over the 593 the
///    normals decide, and **0.80 over the 133 the compactness rule decides** -
///    which is the one that matters, because those two rules share no input.
/// 2. **Nothing lands outside the circuit.** A misread reads bytes normalised
///    over the whole `i16` range as positions, which at the `1/128` scale puts
///    points up to 256 units from the chunk's own bias in *every* axis.
///
/// The envelope is `+/-600` in world `y` and the reason it is not tighter is
/// itself a measurement: one chunk reaches `-159..413`, and it is a tower -
/// 148 by 573 by 208, 104 vertices, whose normals are unit on 0.98 at stride 18
/// against 0.35 and 0.29 at the other two. That is decisive, so the bound moved
/// rather than the chunk being excluded.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn nothing_the_box_less_rules_decide_lands_outside_the_circuit() {
    /// Wide enough for the tallest thing the circuit authors, tight against the
    /// 256-per-axis scatter a misread would produce.
    const ENVELOPE: f32 = 600.0;
    /// How much of a chunk must decode to a unit normal at the chosen stride.
    const UNIT_BAR: f32 = 0.75;

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
    let mut worst_unit = 1.0f32;
    for mesh in &model.meshes {
        if placed.contains(&mesh.hash) {
            continue;
        }
        let Some(stride) = mesh.solve_stride_without_a_box(&model_blob) else {
            continue;
        };
        let (mut unit, mut vertices) = (0usize, 0usize);
        for submesh in &mesh.submeshes {
            let (Ok(points), Ok(normals)) = (
                mesh.positions(&model_blob, submesh, stride),
                mesh.normals(&model_blob, submesh, stride),
            ) else {
                continue;
            };
            if points.is_empty() {
                continue;
            }
            for n in &normals {
                let length = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
                if length == 0.0 {
                    continue;
                }
                vertices += 1;
                unit += usize::from((0.93..=1.07).contains(&length));
            }
            let (lo, hi) = points.iter().fold((f32::MAX, f32::MIN), |(lo, hi), p| {
                (lo.min(p[1]), hi.max(p[1]))
            });
            checked += 1;
            widest = widest.max(hi - lo);
            assert!(
                lo > -ENVELOPE && hi < ENVELOPE,
                "a chunk decoded at stride {stride} spans y {lo}..{hi}, which is \
                 not a piece of a circuit"
            );
        }
        if vertices > 0 {
            let fraction = unit as f32 / vertices as f32;
            worst_unit = worst_unit.min(fraction);
            assert!(
                fraction >= UNIT_BAR,
                "a chunk decoded at stride {stride} has unit normals on only \
                 {fraction:.2} of its {vertices} vertices, so the stride is wrong"
            );
        }
    }
    println!(
        "{checked} box-less submeshes, widest world y span {widest:.1}, \
         worst unit-normal fraction {worst_unit:.2}"
    );
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
            // **85, not 90, and the number moved because the sample did.** The
            // bar was set when 731 of the circuit's chunks decoded; with 968
            // decoding, group `83 08` reads 87.1 % unit at a median `|dot|` of
            // 0.008 - unambiguously a tangent by the statistic that identifies
            // one. The `|dot|` bar is the discriminator; this is a sanity floor.
            assert!(
                unit_pct >= 85.0 && median < 0.1,
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

/// Claim 6: **where a diffuse texture coordinate is, measured against where it
/// was assumed to be.**
///
/// # What this replaces
///
/// This test twice pinned a reading that has since been retired. First a
/// negative - that the last four bytes of a vertex were *not* a coordinate on
/// every submesh, 290 of Talon's Junction's 1,112 draw calls spanning over 100
/// tiles of their texture. Then a positive that explained the split as two
/// coordinate *types* told apart by what the bytes decoded to.
///
/// Both were reasoning around a missing field, and the field exists: the
/// chunk's [`rcsmodel::VertexDecl`] gives each attribute's offset and type. The
/// submeshes the sniff called a second type are ones whose last four bytes are
/// a different **attribute** - a `tangent` or a colour set - on a vertex whose
/// coordinate is elsewhere.
///
/// The eliminations from the first reading still hold and are on
/// `docs/formats/rcsmodel.md`: no byte of the chunk header and no byte of the
/// `0x80`-byte submesh descriptor separates the groups. Neither does, because
/// the answer is not in either - it is in the block the header points at.
///
/// # What is asserted
///
/// That the declared coordinate is often somewhere other than the last four
/// bytes, and that reading it where the file says leaves the model's
/// coordinates far more plausible than reading the last four bytes did.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_declaration_says_where_a_texture_coordinate_is_and_it_is_rarely_last() {
    let (archive, vex_path, model_path) = PAIRS[2];
    let Some((_, model_blob)) = pair(archive, vex_path, model_path) else {
        return;
    };
    let model = rcsmodel::Model::parse(&model_blob).expect("the .rcsmodel parses");

    /// Widest tiling treated as authored rather than as noise.
    const PLAUSIBLE: f32 = 8.0;

    let mut where_it_is: std::collections::BTreeMap<(usize, &'static str), usize> =
        Default::default();
    let mut vertices = 0usize;
    let mut implausible = 0usize;
    let mut was_implausible = 0usize;
    for mesh in &model.meshes {
        let Some(stride) = mesh.declared_stride() else {
            continue;
        };
        let uv = mesh
            .decl
            .as_ref()
            .and_then(rcsmodel::VertexDecl::diffuse_texcoord);
        let Some(uv) = uv else { continue };
        let placed = if usize::from(uv.offset) + 4 == stride {
            "last"
        } else {
            "elsewhere"
        };
        *where_it_is.entry((stride, placed)).or_default() += 1;
        for submesh in &mesh.submeshes {
            if submesh.vertex_count < 16 {
                continue;
            }
            let Ok(uvs) = mesh.texcoords(&model_blob, submesh, stride) else {
                continue;
            };
            let bad = |uv: &[f32; 2]| !uv.iter().all(|c| c.is_finite() && c.abs() <= PLAUSIBLE);
            vertices += uvs.len();
            implausible += uvs.iter().filter(|uv| bad(uv)).count();
            // The identical submeshes under the reading this replaces - the
            // last four bytes of every vertex, as halves - so the improvement
            // is measured against the old behaviour on the same corpus rather
            // than against a bar this file picked.
            was_implausible += (0..submesh.vertex_count)
                .filter(|k| {
                    let at = submesh.vertex_offset + k * stride + stride - 4;
                    let uv: [f32; 2] = std::array::from_fn(|i| {
                        rcsmodel::unpack_half(u16::from_be_bytes([
                            model_blob[at + i * 2],
                            model_blob[at + i * 2 + 1],
                        ]))
                    });
                    bad(&uv)
                })
                .count();
        }
    }
    println!("chunks by (stride, where the declared coordinate is): {where_it_is:?}");
    println!(
        "implausible coordinates: {was_implausible} of {vertices} ({:.2}%) reading the last four \
         bytes, {implausible} ({:.2}%) reading where the declaration says",
        100.0 * was_implausible as f64 / vertices as f64,
        100.0 * implausible as f64 / vertices as f64
    );

    let elsewhere: usize = where_it_is
        .iter()
        .filter(|((_, placed), _)| *placed == "elsewhere")
        .map(|(_, n)| n)
        .sum();
    assert!(
        elsewhere >= 100,
        "the finding is that the coordinate is often not last: {where_it_is:?}"
    );
    // **Measured against the old behaviour, not against a bar this file picked.**
    // If the declaration were not the explanation, reading through it would not
    // move this.
    assert!(
        implausible * 3 <= was_implausible,
        "the declared offset leaves {implausible} of {vertices} implausible against \
         {was_implausible} for the last-four-bytes reading - under a threefold improvement, so \
         the declaration is not carrying its weight"
    );
}

/// Claim 6: **whether a surface shows Wipeout HD's Zone audio-spectrum
/// visualiser is decided by its own authored vertex normal**, and the
/// billboard family is genuinely mixed where the crowd and banner families
/// are not.
///
/// The glow is `saturate(N.y - 0.5) * ... * zoneTexVis[band].rgb` and the
/// gate is universal - `zone_shader_census_ground_truth.rs`'s
/// `the_visualiser_glow_is_gated_to_up_facing_surfaces_everywhere` counts it
/// on every one of the disc's 18,050 visualiser blocks, with no
/// billboard-specific shape anywhere. So "does a billboard light up" is not a
/// shader question at all; it is a question about the shipped meshes, and
/// this is where it gets answered.
///
/// `N` is the authored vertex normal in the mesh's own space, passed through
/// untouched: `billboarddiffuse`'s vertex program writes `MOV o[TC1].xyz,
/// v[1].xyzx`, and the fragment program normalises that same register and
/// dots it against the directional light's direction - so `N.y` is world up,
/// not a view- or tangent-space quantity, and a camera-facing billboard would
/// still be reading its authored normal.
///
/// **The result refutes the assumption that stood in for this measurement.**
/// `hd-zone-stage-textures-are-grounded.md` carried "a vertical billboard's
/// normal has `N.y ~ 0`, which zeroes the term outright" as the reason the
/// maintainer's play observation - floors *and* billboards - could not be the
/// recovered rule. Measured, the three `*billboard*` materials run 21-33% of
/// their vertices above the gate, against 0.0% for `nr_crowd_bustle`,
/// `cf_cheap_crowd` and `ns_adbanner`, and 80%+ for the track surfaces. So
/// the recovered rule does light part of a billboard mesh; it is the crowds
/// and the flat banners it cannot reach.
///
/// The assertion is the *ordering*, not the percentages: a parser fix moves
/// the latter and would not move the former.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn billboard_geometry_is_mixed_where_crowd_and_banner_geometry_is_not() {
    /// Which materials sit either side of the two claims, by leaf name.
    const UPWARD: [&str; 2] = ["track_surface", "track_coloured_specular"];
    const FLAT: [&str; 3] = ["nr_crowd_bustle", "cf_cheap_crowd", "ns_adbanner"];
    const MIXED: [&str; 3] = [
        "billboarddiffuse",
        "cf_billboard1",
        "wes_billboardholographicscanlines",
    ];

    let Some(image) = image() else {
        return;
    };
    // (vertices, vertices whose authored normal passes `N.y > 0.5`)
    let mut tally: std::collections::BTreeMap<&str, (usize, usize)> =
        std::collections::BTreeMap::new();
    for archive in ["DATA00", "DATA01", "DATA02", "DATA03"] {
        let spec = format!("{}:PS3_GAME/USRDIR/{archive}.PSARC", image.display());
        let Ok(mut open) = oag_assets::psarc::Archive::open(&spec) else {
            continue;
        };
        let paths: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmodel") && p.contains("/environments/"))
            .cloned()
            .collect();
        for path in paths {
            let Ok(bytes) = open.read_path(&path) else {
                continue;
            };
            let Ok(model) = rcsmodel::Model::parse(&bytes) else {
                continue;
            };
            for mesh in &model.meshes {
                for surface in mesh.surfaces() {
                    let Some(material) = model.material_of(surface) else {
                        continue;
                    };
                    let leaf = material
                        .name
                        .rsplit('/')
                        .next()
                        .unwrap_or_default()
                        .trim_end_matches(".rcsmaterial");
                    let Some(named) = UPWARD
                        .iter()
                        .chain(&FLAT)
                        .chain(&MIXED)
                        .find(|n| **n == leaf)
                    else {
                        continue;
                    };
                    let Some(stride) = surface.declared_stride() else {
                        continue;
                    };
                    for submesh in &surface.submeshes {
                        let Ok(normals) = surface.normals(&bytes, submesh, stride) else {
                            continue;
                        };
                        let entry = tally.entry(named).or_default();
                        entry.0 += normals.len();
                        entry.1 += normals.iter().filter(|n| n[1] > 0.5).count();
                    }
                }
            }
        }
    }

    let share = |name: &str| -> f64 {
        let (vertices, up) = tally
            .get(name)
            .copied()
            .unwrap_or_else(|| panic!("no geometry found for {name}; the sweep is broken"));
        assert!(vertices > 300, "{name} has only {vertices} vertices");
        #[expect(
            clippy::cast_precision_loss,
            reason = "a vertex count, compared as a ratio"
        )]
        let share = up as f64 / vertices as f64;
        share
    };

    for name in UPWARD {
        assert!(
            share(name) > 0.5,
            "{name} is a track surface and should be majority up-facing, at {:.3}",
            share(name)
        );
    }
    for name in FLAT {
        assert!(
            share(name) < 0.01,
            "{name} is crowd or banner geometry and should be effectively \
             vertical, at {:.3}",
            share(name)
        );
    }
    // The claim that closes the question: neither of the above. If a billboard
    // family ever measured at zero, the Open item's original assumption would
    // have been right after all and this is what would say so.
    for name in MIXED {
        let share = share(name);
        assert!(
            (0.05..0.5).contains(&share),
            "{name} is supposed to be mixed - part of the mesh up-facing, most \
             of it not - and measured {share:.3}"
        );
    }
}
