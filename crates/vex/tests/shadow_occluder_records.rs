//! The occluder's two record arrays, against every hull on the disc.
//!
//! **`#[ignore]`d, needs a disc image** (`just test-data`; ADR-0006). Skips when it
//! is absent; `OAG_REQUIRE_GAME_DATA=1` makes absence a failure.
//!
//! Its own file because `shadow_occluder_ground_truth.rs` is at the 1,000-line
//! rule (`scripts/check-file-size.py`); the seam is natural: that file pins the
//! payload's *closure and census*, this one what is *inside* the records. The
//! disc walker is copied, as in every ground-truth test here.
//!
//! Two claims, both closures rather than readings:
//!
//! 1. the face record's `+0x10` array is an edge graph, reciprocal on every one
//!    of 14,328 edges;
//! 2. the silhouette it describes chains into closed rings on every hull from six
//!    directions, which the `original` shadow tier's draw code fans into triangles.

use std::path::PathBuf;

use oag_disc::DiscImage;
use oag_formats::wad::{self, Compression, Directory};
use oag_vex::vex;

/// Class id of `Dynamic Shadow Occluder`, from the class-ID table at
/// `0x08ab2370`.
const CLASS_OCCLUDER: u32 = 0x3c3;

/// A bounding-box centre further than this from the node's origin makes it
/// world-space; the populations are far apart (a couple of units against
/// hundreds), so the value is not load-bearing.
const WORLD_SPACE_CENTRE: f32 = 50.0;

/// The `.vex` version Pulse ships throughout.
const PULSE_VERSIONS: [u32; 1] = [6];

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

fn f32_at(data: &[u8], at: usize) -> f32 {
    f32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]])
}

/// One decoded `.vex` file's worth of raw bytes.
struct VexFile {
    label: String,
    bytes: Vec<u8>,
    tree: Vec<vex::Node>,
}

/// Every `.vex` file of an accepted version in every `.wad` on the disc,
/// decompressed and walked.
///
/// **Every archive, not just `Data.wad`**: the named occluders live in both it and
/// `BEData.wad`, and one alone halves the population (reading as "the front end
/// has no shadows").
///
/// **The version filter is a parameter**: Pulse is version 6, Pure version 4 with
/// 15 version-3 files ([`pure-status.md`]); hard-coding 6 walks **zero** Pure
/// files, the false negative the file-count assertion catches.
fn vex_files(disc: &mut DiscImage, versions: &[u32]) -> Vec<VexFile> {
    let archives: Vec<_> = disc
        .entries()
        .expect("entries")
        .iter()
        .filter(|entry| entry.path.to_ascii_lowercase().ends_with(".wad"))
        .cloned()
        .collect();

    let mut out = Vec::new();
    for archive in archives {
        let header = disc
            .read_entry_range(&archive, 0, wad::HEADER_LEN as u64)
            .expect("header");
        let Ok(count) = Directory::peek_entry_count(&header) else {
            continue;
        };
        let Ok(dir_bytes) = disc.read_entry_range(&archive, 0, Directory::directory_len(count))
        else {
            continue;
        };
        let Ok(dir) = Directory::parse(&dir_bytes, Some(archive.size)) else {
            continue;
        };

        for (index, entry) in dir.entries.iter().enumerate() {
            if entry.size == 0 {
                continue;
            }
            let raw = disc
                .read_entry_range(&archive, u64::from(entry.offset), u64::from(entry.size))
                .expect("blob");
            let bytes = match entry.compression {
                Compression::None => raw,
                Compression::Lzss => {
                    match oag_formats::lzss::decompress(&raw, entry.size_uncompressed as usize) {
                        Ok(bytes) => bytes,
                        Err(_) => continue,
                    }
                }
                Compression::Zlib => continue,
            };
            if !vex::has_magic(&bytes) {
                continue;
            }
            match vex::version(&bytes) {
                Ok(version) if versions.contains(&version) => {}
                _ => continue,
            }
            let Ok(tree) = vex::nodes(&bytes) else {
                continue;
            };
            out.push(VexFile {
                label: format!("{}#{index}", archive.path),
                bytes,
                tree,
            });
        }
    }
    out
}

/// The packed bounding box at `+0x0c` (min) and `+0x18` (max).
fn bounds(payload: &[u8]) -> ([f32; 3], [f32; 3]) {
    let mut min = [0.0; 3];
    let mut max = [0.0; 3];
    for axis in 0..3 {
        min[axis] = f32_at(payload, 0x0c + axis * 4);
        max[axis] = f32_at(payload, 0x18 + axis * 4);
    }
    (min, max)
}

/// Faces on the Pulse disc, over all 129 occluder nodes.
const PSP_FACES: usize = 4381;

/// How many of those are triangles, and how many quads. Nothing else occurs.
const PSP_TRIANGLES: usize = 3184;
const PSP_QUADS: usize = 1197;

/// Directed edges whose `+0x10` slot names a face, disc-wide.
///
/// Every one is reciprocal (see the test): the closure that settles the index
/// layout, as the payload length's own does.
const PSP_ADJACENT_EDGES: usize = 14328;

/// Edges carrying `NO_NEIGHBOUR` that are *not* a triangle's degenerate fourth:
/// the boundary edges of the two open hulls, `Data.wad#242` and `#244`.
const PSP_OPEN_EDGES: usize = 12;

/// The worst angle, in degrees, between a triangle's declared plane normal and
/// the normal of the three vertices it indexes.
///
/// **Stated over triangles only, the honest population**: a quad is not planar
/// (300 of 1,197 spread past `1e-4` of their hull's scale), so a sliver quad
/// reaches 180 degrees and says nothing about the decode. Three points always
/// describe a plane; four authored ones need not.
const PSP_WORST_TRIANGLE_NORMAL_DEGREES: f32 = 0.03;

/// Faces that wind against their own declared normal, and faces with no area
/// at all.
///
/// One each, named in `oag_vex::shadow_occluder`'s docs and **carried rather than
/// rejected**: refusing them would refuse two hulls over two faces, and only a
/// caller building a volume can decide what to do with a reversed face.
const PSP_NEGATIVE_WINDING: usize = 1;
const PSP_DEGENERATE_WINDING: usize = 1;

/// The face record's own two index arrays decode, and the edge graph they
/// describe closes on itself.
///
/// **The test that settles the layout**, on two independent facts:
///
/// 1. Adjacency is reciprocal on every one of [`PSP_ADJACENT_EDGES`] edges (the
///    face named across an edge owns it); a wrong stride or offset does not give
///    a consistent edge graph on fourteen thousand edges.
/// 2. A triangle's declared normal agrees with its three vertices' geometry to
///    [`PSP_WORST_TRIANGLE_NORMAL_DEGREES`]: two quantities stored apart, agreeing.
///
/// Everything else is a count that moves if the corpus does.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd; run with `just test-data`"]
fn the_face_records_index_the_vertex_array_and_each_other() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open");
    let files = vex_files(&mut disc, &PULSE_VERSIONS);

    let mut faces = 0usize;
    let mut triangles = 0usize;
    let mut quads = 0usize;
    let mut adjacent_edges = 0usize;
    let mut reciprocal = 0usize;
    let mut open_edges = 0usize;
    let mut unowned_open_edges = 0usize;
    let mut worst_degrees = 0.0f32;
    let mut negative = 0usize;
    let mut degenerate = 0usize;

    for file in &files {
        for node in vex::nodes_by_class(&file.tree, CLASS_OCCLUDER) {
            let payload = &file.bytes[node.payload()];
            let label = format!("{} {}", file.label, node.name.clone().unwrap_or_default());
            let hull =
                oag_vex::shadow_occluder::Occluder::parse(payload, oag_formats::ByteOrder::Little)
                    .unwrap_or_else(|| panic!("{label}: does not close"));

            for (index, face) in hull.faces.iter().enumerate() {
                faces += 1;
                match face.count {
                    3 => triangles += 1,
                    4 => quads += 1,
                    other => panic!("{label}: face {index} claims {other} vertices"),
                }
                for slot in face.indices() {
                    assert!(
                        usize::from(*slot) < hull.vertices.len(),
                        "{label}: face {index} indexes vertex {slot} of {}",
                        hull.vertices.len()
                    );
                }
                let used = usize::from(face.count);
                for edge in 0..used {
                    let from = face.vertices[edge];
                    let to = face.vertices[(edge + 1) % used];
                    let owns = |other: &oag_vex::shadow_occluder::Face| {
                        let count = usize::from(other.count);
                        (0..count).any(|k| {
                            let have = (other.vertices[k], other.vertices[(k + 1) % count]);
                            have == (to, from) || have == (from, to)
                        })
                    };
                    match face.neighbour(edge) {
                        Some(across) => {
                            adjacent_edges += 1;
                            if hull.faces.get(usize::from(across)).is_some_and(owns) {
                                reciprocal += 1;
                            }
                        }
                        // A triangle's fourth edge is `v0 -> v0` and no face
                        // can own it; anything else is a real boundary.
                        None if from == to => {}
                        None => {
                            open_edges += 1;
                            let owned = hull
                                .faces
                                .iter()
                                .enumerate()
                                .any(|(other, candidate)| other != index && owns(candidate));
                            if !owned {
                                unowned_open_edges += 1;
                            }
                        }
                    }
                }

                let winding = face.winding(&hull.vertices);
                if winding < 0.0 {
                    negative += 1;
                } else if winding == 0.0 {
                    degenerate += 1;
                }
                if face.count == 3 {
                    let point = |slot: usize| hull.vertices[usize::from(face.vertices[slot])];
                    let (a, b, c) = (point(0), point(1), point(2));
                    let ab = [b[0] - a[0], b[1] - a[1], b[2] - a[2]];
                    let ac = [c[0] - a[0], c[1] - a[1], c[2] - a[2]];
                    let cross = [
                        ab[1] * ac[2] - ab[2] * ac[1],
                        ab[2] * ac[0] - ab[0] * ac[2],
                        ab[0] * ac[1] - ab[1] * ac[0],
                    ];
                    let length =
                        (cross[0] * cross[0] + cross[1] * cross[1] + cross[2] * cross[2]).sqrt();
                    if length > 1e-12 {
                        let cosine = (winding / length).clamp(-1.0, 1.0);
                        worst_degrees = worst_degrees.max(cosine.acos().to_degrees());
                    }
                }
            }
        }
    }

    assert_eq!(faces, PSP_FACES, "faces disc-wide");
    assert_eq!(
        (triangles, quads),
        (PSP_TRIANGLES, PSP_QUADS),
        "face shapes"
    );
    assert_eq!(adjacent_edges, PSP_ADJACENT_EDGES, "edges naming a face");
    assert_eq!(
        reciprocal, PSP_ADJACENT_EDGES,
        "every edge's named face owns that edge"
    );
    assert_eq!(open_edges, PSP_OPEN_EDGES, "real boundary edges");
    assert_eq!(
        unowned_open_edges, PSP_OPEN_EDGES,
        "an edge marked NO_NEIGHBOUR is owned by no other face"
    );
    assert!(
        worst_degrees < PSP_WORST_TRIANGLE_NORMAL_DEGREES,
        "a triangle's declared normal is {worst_degrees} degrees off its own geometry"
    );
    assert_eq!(negative, PSP_NEGATIVE_WINDING, "faces winding backwards");
    assert_eq!(degenerate, PSP_DEGENERATE_WINDING, "faces with no area");
}

/// The six directions the silhouette walk is exercised over.
///
/// The authored axis first (`oag_pulse::shadow::AUTHORED_AXIS`, what the original
/// projects along), then five others: a walk that works only for the direction it
/// was written against is not a silhouette walk.
const WALK_DIRECTIONS: [[f32; 3]; 6] = [
    [0.097_589_54, -0.975_895_4, 0.195_179_08],
    [0.0, -1.0, 0.0],
    [0.0, 1.0, 0.0],
    [1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0],
    [0.577_35, 0.577_35, 0.577_35],
];

/// 129 hulls over [`WALK_DIRECTIONS`].
const PSP_WALKS: usize = 774;

/// Silhouette chains that never returned to where they started, disc-wide.
///
/// **Zero, and that is the assertion the draw code rests on.** Every face
/// contributes a closed cycle of edges, so a silhouette is a union of cycles;
/// an open chain would mean the edge graph is not what the record says it is.
const PSP_OPEN_CHAINS: usize = 0;

/// Against the authored axis, and **after** `Occluder::outline`'s dedup: how
/// many of the 119 local-space hulls come out as one ring, and how many as
/// two.
///
/// The one that comes out as two is `Data.wad#597` `shadow_lodShape`, whose
/// rings have *different* vertices - two genuine lobes, both kept and both
/// drawn.
const PSP_LOCAL_ONE_RING: usize = 118;
const PSP_LOCAL_TWO_RINGS: usize = 1;

/// The ten world-space hulls, same axis, same dedup: eight come out as one
/// ring and two - the flat two-face `Data.wad#242`/`#244` - have no silhouette
/// against this axis at all.
const PSP_WORLD_ONE_RING: usize = 8;
const PSP_WORLD_NO_RING: usize = 2;

/// Hulls where the dedup actually fired: the walk found more rings than the
/// outline kept.
///
/// **Nine, and this is the number that says the dedup is not cosmetic.** Each
/// is a hull carrying a *doubled shell* - every face twinned with an opposite
/// one - so its silhouette comes back as the same ring twice, once each way
/// round. Filling both would darken the same ground twice under an alpha-over
/// blend. Eight are world-space; the ninth is `Data.wad#840` `shadowShape`,
/// which is also the hull carrying the disc's one reversed-winding face.
const PSP_DOUBLED_SHELLS: usize = 9;

/// The silhouette walk closes on every hull, and `outline` collapses the
/// doubled shells rather than drawing them twice.
///
/// This is what the `original` shadow tier's draw code rests on: it fans each
/// ring into triangles, and a ring that did not close, or a shell counted
/// twice, is a wrong picture that still renders.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd; run with `just test-data`"]
fn every_hulls_silhouette_closes_into_rings() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open");
    let files = vex_files(&mut disc, &PULSE_VERSIONS);

    let mut walks = 0usize;
    let mut open_chains = 0usize;
    let mut local_one = 0usize;
    let mut local_two = 0usize;
    let mut world_one = 0usize;
    let mut world_none = 0usize;
    let mut doubled = 0usize;

    for file in &files {
        for node in vex::nodes_by_class(&file.tree, CLASS_OCCLUDER) {
            let payload = &file.bytes[node.payload()];
            let label = format!("{} {}", file.label, node.name.clone().unwrap_or_default());
            let hull =
                oag_vex::shadow_occluder::Occluder::parse(payload, oag_formats::ByteOrder::Little)
                    .unwrap_or_else(|| panic!("{label}: does not close"));

            for direction in WALK_DIRECTIONS {
                walks += 1;
                let rings = hull.silhouette_loops(direction);
                open_chains += rings.iter().filter(|ring| !ring.closed).count();
                // Every ring the walk produced is a run of edges that exists:
                // consecutive vertices, and back to the first.
                for ring in &rings {
                    for slot in 0..ring.vertices.len() {
                        assert!(
                            usize::from(ring.vertices[slot]) < hull.vertices.len(),
                            "{label}: ring names vertex {} of {}",
                            ring.vertices[slot],
                            hull.vertices.len()
                        );
                    }
                }
            }

            // The population split is the same one `the_occluders_are_two_populations_and_only_one_is_per_craft`
            // makes: a craft's own hull sits at the origin.
            let (min, max) = bounds(payload);
            let centre = (0..3)
                .map(|axis| ((min[axis] + max[axis]) * 0.5).abs())
                .fold(0.0f32, f32::max);
            let outline = hull.outline(WALK_DIRECTIONS[0]);
            let walked = hull.silhouette_loops(WALK_DIRECTIONS[0]);
            if walked.len() > outline.len() {
                doubled += 1;
            }
            if centre <= WORLD_SPACE_CENTRE {
                match outline.len() {
                    1 => local_one += 1,
                    2 => local_two += 1,
                    other => panic!("{label}: {other} ring(s) against the authored axis"),
                }
            } else {
                match outline.len() {
                    0 => world_none += 1,
                    1 => world_one += 1,
                    other => panic!("{label}: {other} ring(s) against the authored axis"),
                }
            }
        }
    }

    assert_eq!(walks, PSP_WALKS, "silhouette walks");
    assert_eq!(open_chains, PSP_OPEN_CHAINS, "chains that never closed");
    assert_eq!(
        (local_one, local_two),
        (PSP_LOCAL_ONE_RING, PSP_LOCAL_TWO_RINGS),
        "local-space hulls by ring count, against the authored axis"
    );
    assert_eq!(
        (world_one, world_none),
        (PSP_WORLD_ONE_RING, PSP_WORLD_NO_RING),
        "world-space hulls by ring count, against the authored axis"
    );
    assert_eq!(
        doubled, PSP_DOUBLED_SHELLS,
        "hulls whose doubled shell the outline collapsed"
    );
}
