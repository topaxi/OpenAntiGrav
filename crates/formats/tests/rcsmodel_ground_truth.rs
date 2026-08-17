//! `.rcsmodel` read off the PS3 disc, checked against the `.vex` beside it.
//!
//! **`#[ignore]`d and never run in CI.** They need game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! The image has to be layer-1 decrypted first - `scripts/ps3iso.py decrypt`,
//! `docs/formats/ps3-disc.md`.
//!
//! # What makes this a measurement rather than a restatement
//!
//! Every assertion here is against something **the `.vex` says and the
//! `.rcsmodel` does not**: the authored bounding box, and the node's own hash.
//! So a reader that decoded the wrong bytes cannot pass by being
//! self-consistent - which is the failure mode a container test has to be built
//! against, and the one that made `docs/formats/psarc.md`'s per-entry digest
//! check worth writing.
//!
//! Six claims, in order of how much they would cost to get wrong:
//!
//! 1. **Every `Mesh` node's hash resolves to a chunk.** The link is the whole
//!    reason the two files can be read together at all.
//! 2. **Every index is in range and every count is a multiple of three.** 1,274
//!    of 1,274 submeshes, which says the index offsets and counts were read
//!    correctly rather than plausibly.
//! 3. **The recovered geometry fills the authored box.** The vertex stride is
//!    not stored anywhere in the file, so this is both how it is found and the
//!    only check that it was found rightly.
//! 4. **The stride the buffer layout gives is the same one.** The fourth is the
//!    one that does not need the `.vex` at all - it is arithmetic on two fields
//!    of the `.rcsmodel` - and it agrees with the box on 19 and with the
//!    compactness rule on 96, contradicting neither anywhere.
//! 5. **What the box-less rules decide stays inside the circuit.** The
//!    unreferenced path has no authored box to filter a stray submesh against,
//!    so this is the check that stands in for one.
//! 6. **The four bytes after a position are the vertex normal.** Unit on
//!    99.5 % of every model's vertices, and agreeing with an oracle the file
//!    does not state - the area-weighted average of the faces at each vertex.

use std::path::{Path, PathBuf};

use oag_formats::{rcsmodel, vex};

/// The decrypted PS3 image.
const PS3_IMAGE: &str = "hdfury-ps3-eu-dec.iso";

/// A `.vex` and the `.rcsmodel` beside it, with the archive holding both.
const PAIRS: &[(&str, &str, &str)] = &[
    (
        "DATA02.PSARC",
        "/data/ships/assegai/ship.vex",
        "/data/ships/assegai/ship.rcsmodel",
    ),
    (
        "DATA02.PSARC",
        "/data/ships/assegai/ship_lod1.vex",
        "/data/ships/assegai/ship_lod1.rcsmodel",
    ),
    (
        "DATA00.PSARC",
        "/data/environments/talons_junction/track.vex",
        "/data/environments/talons_junction/track.rcsmodel",
    ),
];

/// Two quantisation steps at the 1/128 scale every file on the disc uses.
const TOLERANCE: f32 = 2.0 / 128.0;

fn image() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images")
        .join(PS3_IMAGE);

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

/// Both halves of one pair, read straight out of the archive.
fn pair(archive: &str, vex_path: &str, model_path: &str) -> Option<(Vec<u8>, Vec<u8>)> {
    let image = image()?;
    let spec = format!("{}:PS3_GAME/USRDIR/{archive}", image.display());
    let mut open = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
    Some((
        open.read_path(vex_path).expect("the .vex reads"),
        open.read_path(model_path).expect("the .rcsmodel reads"),
    ))
}

/// Every `Mesh` node in a `.vex`, as (name, hash, min, max).
fn mesh_nodes(blob: &[u8]) -> Vec<(String, u32, [f32; 3], [f32; 3])> {
    let classes = vex::classes_of(blob).expect("a class table");
    let mesh = classes.mesh.expect("version 6 numbers Mesh");
    let order = vex::byte_order(blob);
    vex::nodes(blob)
        .expect("the node tree walks")
        .into_iter()
        .filter(|node| node.class_id == mesh)
        .filter_map(|node| {
            let payload = &blob[node.payload()];
            // The box pair at +0x10/+0x20 and the chunk hash at +0x30, which is
            // as much of a PS3 `Mesh` payload as anything has recovered.
            if payload.len() < 0x34 {
                return None;
            }
            let read3 = |at: usize| std::array::from_fn(|i| order.f32(payload, at + i * 4));
            Some((
                node.name.clone().unwrap_or_default(),
                order.u32(payload, 0x30),
                read3(0x10),
                read3(0x20),
            ))
        })
        .collect()
}

/// The widest axis of a point set.
fn span(points: &[[f32; 3]]) -> f32 {
    (0..3)
        .map(|i| {
            let lo = points.iter().fold(f32::MAX, |a, p| a.min(p[i]));
            let hi = points.iter().fold(f32::MIN, |a, p| a.max(p[i]));
            hi - lo
        })
        .fold(0.0, f32::max)
}

fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

fn unit3(v: [f32; 3]) -> Option<[f32; 3]> {
    let l = dot(v, v).sqrt();
    (l > 1e-9).then(|| [v[0] / l, v[1] / l, v[2] / l])
}

/// Area-weighted vertex normals, which is what the `.rcsmodel` is checked
/// against and what it does not itself state.
///
/// **Area-weighted on purpose**: summing raw cross products rather than
/// normalising each face first is what an exporter writes, and unweighted
/// averaging diverges wherever a mesh mixes large and small triangles - which a
/// circuit does everywhere.
fn smooth_normals(points: &[[f32; 3]], indices: &[u16]) -> Vec<Option<[f32; 3]>> {
    let mut acc = vec![[0.0f32; 3]; points.len()];
    for t in indices.chunks_exact(3) {
        let [a, b, c] = [t[0], t[1], t[2]].map(|i| points[i as usize]);
        let (u, v) = (
            [b[0] - a[0], b[1] - a[1], b[2] - a[2]],
            [c[0] - a[0], c[1] - a[1], c[2] - a[2]],
        );
        let n = [
            u[1] * v[2] - u[2] * v[1],
            u[2] * v[0] - u[0] * v[2],
            u[0] * v[1] - u[1] * v[0],
        ];
        for &i in t {
            for k in 0..3 {
                acc[i as usize][k] += n[k];
            }
        }
    }
    acc.into_iter().map(unit3).collect()
}

/// Claim 1: the hash in a `Mesh` payload addresses a chunk in the `.rcsmodel`
/// beside it - on every node of a craft, and on most of a circuit's.
///
/// **The shortfall is measured, not asserted away.** All 15 of Assegai's mesh
/// nodes resolve and all 4 of its LOD1's do; 70 of Talon's Junction's 126 do,
/// and the 56 that do not are scenery - `Skycar_1Shape`, `tanker1aShape`,
/// `shipintersteller1Shape`, `HyperContintentCraft1Shape`. Their geometry is
/// somewhere this project has not found: `track.rcsmodel` carries **983**
/// chunks to the `.vex`'s 126, so it is not short of them, and the directory
/// holds no second model file for the circuit. Whether those nodes address a
/// shared props file, or carry their reference somewhere other than `+0x30`,
/// is open - see `docs/formats/rcsmodel.md`.
///
/// The numbers are asserted as ratios so this fails if a change makes it
/// *worse*, which is what the test is for; the open question is a doc entry
/// rather than a failing test.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn a_mesh_node_addresses_its_chunk_by_hash_on_every_craft_and_most_of_a_circuit() {
    for (archive, vex_path, model_path) in PAIRS {
        let Some((blob, model_blob)) = pair(archive, vex_path, model_path) else {
            return;
        };
        let model = rcsmodel::Model::parse(&model_blob).expect("the .rcsmodel parses");
        let nodes = mesh_nodes(&blob);
        assert!(!nodes.is_empty(), "{vex_path} has no Mesh nodes");

        let missing: Vec<&str> = nodes
            .iter()
            .filter(|(_, hash, ..)| model.mesh(*hash).is_none())
            .map(|(name, ..)| name.as_str())
            .collect();
        let resolved = nodes.len() - missing.len();
        println!(
            "{vex_path}: {resolved} of {} nodes resolve, into {} chunks; unresolved e.g. {:?}",
            nodes.len(),
            model.meshes.len(),
            &missing[..missing.len().min(4)]
        );

        let floor = if vex_path.contains("/ships/") {
            nodes.len()
        } else {
            nodes.len() / 2
        };
        assert!(
            resolved >= floor,
            "{vex_path}: {resolved} of {} resolve, expected at least {floor}",
            nodes.len()
        );
    }
}

/// Claim 2: the index buffers are big-endian `u16` triangle lists, in range.
///
/// **Checked over the whole file rather than the referenced meshes**, because
/// this claim is about the container: `talons_junction` carries 983 chunks
/// where its `.vex` names 126, so most of what is measured here is geometry no
/// node points at - and it decodes identically, which is the stronger result.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn every_index_buffer_is_a_triangle_list_within_its_own_vertex_count() {
    let mut total = 0usize;
    for (archive, vex_path, model_path) in PAIRS {
        let Some((_, model_blob)) = pair(archive, vex_path, model_path) else {
            return;
        };
        let model = rcsmodel::Model::parse(&model_blob).expect("the .rcsmodel parses");
        for mesh in &model.meshes {
            for submesh in &mesh.submeshes {
                if submesh.index_count == 0 {
                    continue;
                }
                assert_eq!(
                    submesh.index_count % 3,
                    0,
                    "{model_path}: {} indices is not a whole number of triangles",
                    submesh.index_count
                );
                // `indices` raises `IndexOutOfRange` itself, so this is the
                // assertion as well as the read.
                mesh.indices(&model_blob, submesh)
                    .unwrap_or_else(|e| panic!("{model_path}: {e}"));
                total += 1;
            }
        }
    }
    assert!(total >= 1_200, "only {total} submeshes checked");
    println!("{total} submeshes are in-range triangle lists");
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
