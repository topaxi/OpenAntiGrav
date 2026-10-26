//! Which space a `Mesh` node's vertex normals are authored in, measured against
//! the faces they belong to, on every `.vex` of each disc.
//!
//! **`#[ignore]`d, needs a disc image** (`just test-data`; ADR-0006). Skips when it
//! is absent; `OAG_REQUIRE_GAME_DATA=1` makes absence a failure.
//!
//! # What this is for
//!
//! A batch's positions are in its node's own space and the loader carries them
//! to the model's through the node's world matrix (`vex::anim_anchors`). The
//! original carries the normals through the same matrix: `Mesh_DrawBatchSet`
//! and `FUN_0890f288` write `*(mesh + 0x30)`, the node's 4x3 world matrix,
//! into GE `WORLDMATRIXDATA` words per mesh, and the GE takes a normal through
//! that matrix's 3x3 (`docs/ghidra/functions/psp-pulse-usa/mesh-draw.md`).
//! The loader did not, so a vertex normal under a rotated node stayed
//! node-local while its position was placed.
//!
//! The claim measured here does not depend on the loader's fix. For every
//! (vertex, triangle) pair of every `Mesh` node it asks whether the vertex
//! normal points to the side of the triangle's own face that the placed
//! positions say is outward, with the normal left as authored (`local`) and
//! carried through the node's rotation (`placed`). A mirrored node reverses a
//! face's winding, so its face normal is read with the determinant's sign.
//!
//! - **Under an identity rotation** the two are the same number, and they agree
//!   with the face on about 99.6% of pairs: the baseline an authored normal
//!   reaches (smoothed vertices, degenerate slivers).
//! - **Under a rotated node** `local` falls to about 75% on Pulse PSP and
//!   `placed` returns to the baseline. A normal in the node's space would
//!   score the baseline in both columns, so the normals are authored
//!   node-local, which is what the GE's world-matrix product expects.
//! - **Every rotated node is orthonormal** (worst deviation under 1e-5), so the
//!   GE's plain `n * M3x3` is also the inverse transpose.

use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

use oag_disc::DiscImage;
use oag_formats::wad::{self, Directory};
use oag_vex::vex;

use crate::wad_walk;

const PSP_DATA: &str = "PSP_GAME/USRDIR/Data.wad";
const PS2_DATA: &str = "54748/WADS2.WAD";

/// A rotation entry deviating from the orthonormal by more than this fails.
const ORTHONORMAL: f32 = 1.0e-5;

/// Pulse PSP's circuit directories, the twelve that ship a `track.vex`.
const CIRCUIT_DIRS: [&str; 16] = [
    "01_Track", "02_Track", "03_Track", "04_Track", "05_Track", "06_Track", "07_Track", "08_Track",
    "09_Track", "10_Track", "11_Track", "12_Track", "13_Track", "14_Track", "15_Track", "16_Track",
];

const CIRCUIT_FILES: [&str; 4] = [
    "track",
    "track_reversed",
    "zone_track",
    "zone_track_reversed",
];

/// One file's counts, or a whole corpus' when folded.
#[derive(Debug, Default, Clone, PartialEq)]
struct Survey {
    files: usize,
    /// Files whose batches or node tree the walk could not read at all.
    refused: usize,
    meshes: usize,
    /// `Mesh` nodes whose placing matrix has a rotation part other than the
    /// identity, and the subset that is a mirror (negative determinant).
    turned_meshes: usize,
    mirrored_meshes: usize,
    /// Vertices that author a normal.
    normals: usize,
    /// Of those, the ones under a turned node placed statically, and the ones
    /// under an `Anim Transform` (the matrix is the anchor-to-node one).
    turned_static: usize,
    turned_anchored: usize,
    /// Of the turned ones, those in a batch the shine pass redraws.
    turned_shine: usize,
    /// Of the turned ones, those that author no colour: the vertices a light
    /// rig, not a bake, shades by their normal.
    turned_lit: usize,
    /// Largest deviation of a turned node's 3x3 from an orthonormal basis.
    worst_skew: f32,
    /// `(vertex, face)` pairs under a turned node whose normal faces outward,
    /// left as authored and carried through the rotation.
    local_agree: usize,
    local_disagree: usize,
    placed_agree: usize,
    placed_disagree: usize,
    /// Normal-bearing vertices of batches the face test does not judge.
    unjudged: usize,
    /// The same pairs under an identity rotation.
    identity_agree: usize,
    identity_disagree: usize,
}

impl Survey {
    fn fold(&mut self, other: &Self) {
        self.files += other.files;
        self.refused += other.refused;
        self.meshes += other.meshes;
        self.turned_meshes += other.turned_meshes;
        self.mirrored_meshes += other.mirrored_meshes;
        self.normals += other.normals;
        self.turned_static += other.turned_static;
        self.turned_anchored += other.turned_anchored;
        self.turned_shine += other.turned_shine;
        self.turned_lit += other.turned_lit;
        self.worst_skew = self.worst_skew.max(other.worst_skew);
        self.local_agree += other.local_agree;
        self.local_disagree += other.local_disagree;
        self.placed_agree += other.placed_agree;
        self.placed_disagree += other.placed_disagree;
        self.unjudged += other.unjudged;
        self.identity_agree += other.identity_agree;
        self.identity_disagree += other.identity_disagree;
    }

    fn turned(&self) -> usize {
        self.turned_static + self.turned_anchored
    }

    /// The share of `agree + disagree` that agrees, in `0..=1`.
    fn share(agree: usize, disagree: usize) -> f64 {
        agree as f64 / (agree + disagree).max(1) as f64
    }

    fn report(&self, label: &str) {
        eprintln!(
            "{label}: {} mesh(es), {} turned ({} mirrored); {} normals, {} under a turned node \
             ({} static, {} anchored, {} in shine batches, {} lit); {} unjudged; skew {:.1e}; identity {:.4}, local \
             {:.4}, placed {:.4}",
            self.meshes,
            self.turned_meshes,
            self.mirrored_meshes,
            self.normals,
            self.turned(),
            self.turned_static,
            self.turned_anchored,
            self.turned_shine,
            self.turned_lit,
            self.unjudged,
            self.worst_skew,
            Self::share(self.identity_agree, self.identity_disagree),
            Self::share(self.local_agree, self.local_disagree),
            Self::share(self.placed_agree, self.placed_disagree),
        );
    }
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

/// The 3x3's determinant and its worst deviation from an orthonormal basis.
fn determinant_and_skew(m: &[f32; 16]) -> (f32, f32) {
    let det = m[0] * (m[5] * m[10] - m[6] * m[9]) - m[1] * (m[4] * m[10] - m[6] * m[8])
        + m[2] * (m[4] * m[9] - m[5] * m[8]);
    let row = |r: usize| [m[r * 4], m[r * 4 + 1], m[r * 4 + 2]];
    let mut skew = 0.0f32;
    for r in 0..3 {
        skew = skew.max((dot(row(r), row(r)).sqrt() - 1.0).abs());
        for s in r + 1..3 {
            skew = skew.max(dot(row(r), row(s)).abs());
        }
    }
    (det, skew)
}

/// One `.vex`'s counts, `None` when its class table or node tree is refused.
/// The face test runs only on the batches `judged` keeps; the counts of
/// vertices and nodes cover all of them.
fn survey_file(bytes: &[u8], judged: fn(&vex::Batch) -> bool) -> Option<Survey> {
    let nodes = vex::nodes(bytes).ok()?;
    let mesh_class = vex::classes_of(bytes).ok()?.mesh?;
    let anchors = vex::anim_anchors(bytes, &nodes);
    let mut out = Survey {
        files: 1,
        ..Survey::default()
    };
    for (index, node) in nodes.iter().enumerate() {
        if node.class_id != mesh_class || node.payload().end > bytes.len() {
            continue;
        }
        let payload = &bytes[node.payload()];
        let anchored = anchors[index];
        let m = anchored.local;
        let turned = !vex::leaves_directions(&m);
        let (det, skew) = determinant_and_skew(&m);
        out.meshes += 1;
        if turned {
            out.turned_meshes += 1;
            out.mirrored_meshes += usize::from(det < 0.0);
            out.worst_skew = out.worst_skew.max(skew);
        }
        let mesh_shines =
            payload.len() >= 2 && u16::from_le_bytes([payload[0], payload[1]]) & 0x2000 != 0;
        // A mirror reverses every winding the placed positions draw.
        let handed = if det < 0.0 { -1.0 } else { 1.0 };
        for list in [0u8, 1u8] {
            let Ok(batches) = vex::mesh_batches(payload, list) else {
                out.refused += 1;
                continue;
            };
            for batch in &batches {
                let shine = mesh_shines && batch.pass_mask & 0x2000 != 0;
                let placed: Vec<[f32; 3]> = batch
                    .vertices
                    .iter()
                    .map(|v| vex::transform_point(&m, v.position))
                    .collect();
                let normals = batch.vertices.iter().filter(|v| v.normal.is_some()).count();
                out.normals += normals;
                if turned {
                    if anchored.anchor.is_some() {
                        out.turned_anchored += normals;
                    } else {
                        out.turned_static += normals;
                    }
                    if shine {
                        out.turned_shine += normals;
                    }
                    out.turned_lit += batch
                        .vertices
                        .iter()
                        .filter(|v| v.normal.is_some() && v.colour.is_none())
                        .count();
                }
                if !judged(batch) {
                    out.unjudged += normals;
                    continue;
                }
                for tri in batch.triangles() {
                    let [a, b, c] = tri.map(|i| placed[i as usize]);
                    let face = cross(sub(b, a), sub(c, a));
                    if dot(face, face) < 1.0e-12 {
                        continue;
                    }
                    for i in tri {
                        let Some(n) = batch.vertices[i as usize].normal else {
                            continue;
                        };
                        // A normal with no length points nowhere.
                        if dot(n, n) < 0.5 {
                            continue;
                        }
                        let local = dot(n, face) * handed > 0.0;
                        let carried = dot(vex::transform_direction(&m, n), face) * handed > 0.0;
                        if turned {
                            out.local_agree += usize::from(local);
                            out.local_disagree += usize::from(!local);
                            out.placed_agree += usize::from(carried);
                            out.placed_disagree += usize::from(!carried);
                        } else {
                            out.identity_agree += usize::from(local);
                            out.identity_disagree += usize::from(!local);
                        }
                    }
                }
            }
        }
    }
    Some(out)
}

/// Entry index to name, for the names a walk should label.
fn names_by_index(
    disc: &mut DiscImage,
    archive_path: &str,
    names: &[String],
) -> HashMap<usize, String> {
    let archive = wad_walk::archive(disc, archive_path);
    let header = disc
        .read_entry_range(&archive, 0, wad::HEADER_LEN as u64)
        .expect("header");
    let count = Directory::peek_entry_count(&header).expect("entry count");
    let dir_bytes = disc
        .read_entry_range(&archive, 0, Directory::directory_len(count))
        .expect("directory");
    let dir = Directory::parse(&dir_bytes, Some(archive.size)).expect("parse directory");
    let by_hash: HashMap<u32, &String> = names.iter().map(|n| (wad::hash_name(n), n)).collect();
    dir.entries
        .iter()
        .enumerate()
        .filter_map(|(i, e)| by_hash.get(&e.name_hash).map(|n| (i, (*n).clone())))
        .collect()
}

/// Every `.vex` of `archive` at `version`, surveyed, with its label.
fn survey_archive(
    image: &str,
    archive: &str,
    labelled: &[String],
    version: u32,
    judged: fn(&vex::Batch) -> bool,
) -> Option<(Survey, BTreeMap<String, Survey>)> {
    let path: PathBuf = oag_testdata::image(image)?;
    let mut disc = DiscImage::open(&path).expect("open");
    let labels = names_by_index(&mut disc, archive, labelled);
    let blobs = wad_walk::blobs(&mut disc, archive, wad_walk::VEX_HEAD, |head| {
        vex::has_magic(head)
    });
    let mut total = Survey::default();
    let mut named = BTreeMap::new();
    for (index, bytes) in blobs {
        if vex::version(&bytes) != Ok(version) {
            continue;
        }
        let Some(file) = survey_file(&bytes, judged) else {
            total.refused += 1;
            continue;
        };
        total.fold(&file);
        if let Some(label) = labels.get(&index) {
            named.insert(label.clone(), file);
        }
    }
    Some((total, named))
}

fn circuit_names() -> Vec<String> {
    CIRCUIT_DIRS
        .iter()
        .flat_map(|dir| {
            CIRCUIT_FILES
                .iter()
                .map(move |f| format!("Data\\Environments\\{dir}\\{f}.vex"))
        })
        .collect()
}

fn assert_placed_is_what_the_faces_say(label: &str, s: &Survey) {
    let identity = Survey::share(s.identity_agree, s.identity_disagree);
    let local = Survey::share(s.local_agree, s.local_disagree);
    let placed = Survey::share(s.placed_agree, s.placed_disagree);
    assert!(
        identity > 0.99,
        "{label}: authored normals agree with their faces on {identity:.4} of pairs under an identity node"
    );
    assert!(
        placed > 0.99,
        "{label}: normals carried through the node agree on {placed:.4} of pairs"
    );
    assert!(
        local < placed - 0.05,
        "{label}: the normal as authored ({local:.4}) is not worse than carried ({placed:.4}); the \
         claim that it is node-local fails"
    );
    assert!(
        s.worst_skew < ORTHONORMAL,
        "{label}: a turned node is not orthonormal ({:e}), so the GE's product is not the inverse transpose",
        s.worst_skew
    );
}

/// The corpus counts a run is frozen against: `[meshes, turned meshes, mirrored
/// meshes, normals, turned static, turned anchored, turned in shine batches,
/// turned and lit, unjudged]`. A `Speedup Pad` or `Weapon Pad` payload is a
/// mesh payload too, and authors no normal at all, so it is left out.
fn counts(s: &Survey) -> [usize; 9] {
    [
        s.meshes,
        s.turned_meshes,
        s.mirrored_meshes,
        s.normals,
        s.turned_static,
        s.turned_anchored,
        s.turned_shine,
        s.turned_lit,
        s.unjudged,
    ]
}

/// Pulse PSP's twelve circuits' `track.vex`: `[normals, turned meshes, turned
/// vertices, of those in shine batches]`, the per-circuit census. The share is
/// 0 to 5.7% (`07_Track`), 1.1% over the disc.
const PULSE_PSP_CIRCUITS: [(&str, [usize; 4]); 12] = [
    ("01_Track", [174_297, 14, 936, 161]),
    ("02_Track", [175_699, 11, 1_347, 0]),
    ("03_Track", [164_774, 7, 193, 144]),
    ("04_Track", [186_818, 2, 181, 61]),
    ("05_Track", [195_197, 1, 116, 0]),
    ("06_Track", [196_220, 39, 4_285, 1_506]),
    ("07_Track", [181_114, 70, 10_344, 294]),
    ("09_Track", [199_997, 12, 1_862, 0]),
    ("10_Track", [180_159, 28, 2_803, 0]),
    ("13_Track", [169_620, 0, 0, 0]),
    ("14_Track", [186_014, 31, 4_485, 0]),
    ("16_Track", [179_648, 10, 771, 0]),
];

/// **Pulse PSP, the corpus the GE read was made against.**
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn pulse_psp_normals_are_node_local() {
    let names = circuit_names();
    let Some((total, named)) =
        survey_archive("data/images/pulse-psp-usa.chd", PSP_DATA, &names, 6, |_| {
            true
        })
    else {
        return;
    };
    for (name, s) in &named {
        s.report(name);
    }
    total.report("Pulse PSP, every .vex");
    assert_eq!(total.refused, 0, "a file or batch the walk could not read");
    assert_eq!(
        counts(&total),
        [21_055, 720, 17, 7_127_767, 70_561, 5_189, 7_898, 2_966, 0]
    );
    assert_placed_is_what_the_faces_say("Pulse PSP", &total);
    for (circuit, want) in PULSE_PSP_CIRCUITS {
        let s = &named[&format!("Data\\Environments\\{circuit}\\track.vex")];
        assert_eq!(
            [s.normals, s.turned_meshes, s.turned(), s.turned_shine],
            want,
            "{circuit}"
        );
    }
}

/// **Pulse PS2.** The VIF path hands the same `Batch` over, so the same
/// placement question has the same answer or it does not. The face test judges
/// the VIF batches only: the PSP-format batches this disc also carries (vertex
/// types `0x121` to `0x13f`) agree with their faces on about 52% of pairs
/// under an identity node, against 99.9% on PSP, so they say nothing about
/// placement and are counted as unjudged.
#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn pulse_ps2_normals_are_node_local() {
    let Some((total, _)) = survey_archive("data/images/pulse-ps2-eu.chd", PS2_DATA, &[], 6, |b| {
        vex::is_vif_batch(b.vertex_type)
    }) else {
        return;
    };
    total.report("Pulse PS2, every .vex");
    assert_eq!(total.refused, 0, "a file or batch the walk could not read");
    assert_eq!(
        counts(&total),
        [
            30_884, 1_439, 17, 2_041_273, 23_396, 32, 6_504, 740, 697_765
        ]
    );
    assert_placed_is_what_the_faces_say("Pulse PS2", &total);
}

/// **Pure PSP**, the other title that reads these files, at format version 4
/// (156 of its 171). Its 15 version-3 files break the batch header and are
/// not read here.
#[test]
#[ignore = "needs data/images/pure-psp-usa.chd"]
fn pure_psp_normals_are_node_local() {
    let Some((total, _)) =
        survey_archive("data/images/pure-psp-usa.chd", PSP_DATA, &[], 4, |_| true)
    else {
        return;
    };
    total.report("Pure PSP, every version-4 .vex");
    assert_eq!(total.refused, 0, "a file or batch the walk could not read");
    assert_eq!(
        counts(&total),
        [11_025, 771, 1, 2_344_382, 61_372, 2_942, 3_563, 4_418, 0]
    );
    assert_placed_is_what_the_faces_say("Pure PSP", &total);
}
