//! What the disc says about a chunk's surface table.
//!
//! The fourth `.rcsmodel` ground-truth binary, beside `rcsmodel_ground_truth`
//! (the container), `rcsmodel_material_ground_truth` (the material table) and
//! `rcsmodel_vertex_ground_truth` (what a vertex holds). Split for the same
//! reason they are: one file per claim family.
//!
//! **The claim being pinned is that a chunk header is 0x20 bytes and then a
//! surface record**, and that a quarter of the disc's chunks carry more of
//! them - which is 40% more geometry than every first surface holds between
//! them. See `docs/formats/rcsmodel.md`.
//!
//! `#[ignore]`d because it needs `data/images/hdfury-ps3-eu-dec.iso`; run with
//! `just test-data`.

mod rcsmodel_common;

use oag_rcs::rcsmodel;
use rcsmodel_common::image;

const ARCHIVES: &[&str] = &[
    "DATA00", "DATA01", "DATA02", "DATA03", "DATA04", "DATA05", "DATA06",
];

/// One model's totals.
#[derive(Default)]
struct Totals {
    models: usize,
    chunks: usize,
    multi_surface: usize,
    other_material: usize,
    submeshes_first: usize,
    submeshes_rest: usize,
    triangles_first: u64,
    triangles_rest: u64,
    /// Chunks whose surface table does not open at `chunk + 0x20`.
    misplaced_first: usize,
    /// Extra surfaces naming a material the model's table does not have.
    out_of_range: usize,
}

fn sweep() -> Totals {
    let image = image().expect("checked by the caller");
    let mut t = Totals::default();
    for archive in ARCHIVES {
        let spec = format!("{}:PS3_GAME/USRDIR/{archive}.PSARC", image.display());
        let mut open = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
        let paths: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmodel"))
            .cloned()
            .collect();
        for path in paths {
            let Ok(blob) = open.read_path(&path) else {
                continue;
            };
            let Ok(model) = rcsmodel::Model::parse(&blob) else {
                continue;
            };
            t.models += 1;
            let be32 = |at: usize| -> Option<u32> {
                blob.get(at..at + 4)
                    .map(|s| u32::from_be_bytes(s.try_into().unwrap()))
            };
            let chunk_table = be32(0x20).unwrap_or(0) as usize;
            for (index, chunk) in model.meshes.iter().enumerate() {
                t.chunks += 1;
                // The structural claim: the first entry of the chunk's own
                // surface table points at the record inside its header.
                if let (Some(at), Some(_)) = (be32(chunk_table + index * 4), be32(0x20)) {
                    let at = at as usize;
                    let surface_table = be32(at + 0x18).unwrap_or(0) as usize;
                    if be32(surface_table).map(|f| f as usize) != Some(at + 0x20) {
                        t.misplaced_first += 1;
                    }
                }
                let triangles = |m: &rcsmodel::Mesh| -> u64 {
                    m.submeshes.iter().map(|s| (s.index_count / 3) as u64).sum()
                };
                t.submeshes_first += chunk.submeshes.len();
                t.triangles_first += triangles(chunk);
                if chunk.extra_surfaces.is_empty() {
                    continue;
                }
                t.multi_surface += 1;
                if chunk
                    .extra_surfaces
                    .iter()
                    .any(|e| e.material != chunk.material)
                {
                    t.other_material += 1;
                }
                for extra in &chunk.extra_surfaces {
                    t.submeshes_rest += extra.submeshes.len();
                    t.triangles_rest += triangles(extra);
                    if extra.material as usize >= model.materials.len() {
                        t.out_of_range += 1;
                    }
                    assert!(
                        extra.extra_surfaces.is_empty(),
                        "{path} chunk {index}: a surface has surfaces of its own",
                    );
                }
            }
        }
    }
    t
}

/// **A chunk header is 0x20 bytes and then a surface record.**
///
/// The claim the whole reading rests on: if the first entry of a chunk's
/// surface table pointed anywhere else, the fields this module has always
/// called chunk fields would not be a surface's, and every later surface would
/// be being read against the wrong shape.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn every_chunks_first_surface_is_the_record_in_its_own_header() {
    if image().is_none() {
        return;
    }
    let t = sweep();
    assert_eq!(t.models, 643, "every .rcsmodel on the disc parses");
    assert_eq!(t.chunks, 41_861);
    assert_eq!(
        t.misplaced_first, 0,
        "{} of {} chunk(s) open their surface table somewhere other than +0x20",
        t.misplaced_first, t.chunks,
    );
}

/// **A quarter of the disc's chunks carry more than one surface, and every one
/// of those names another material.**
///
/// The measurement that turned `Mesh::material` from "the chunk's material"
/// into "the first surface's". Held to exact counts rather than a range: these
/// are properties of a fixed disc, and a change in them means the reader
/// changed, not the data.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn a_quarter_of_chunks_name_more_than_one_material() {
    if image().is_none() {
        return;
    }
    let t = sweep();
    assert_eq!(t.multi_surface, 9_891);
    assert_eq!(
        t.other_material, t.multi_surface,
        "every multi-surface chunk names a material other than its first's",
    );
    assert_eq!(
        t.out_of_range, 0,
        "{} surface(s) name a material the model's table does not have",
        t.out_of_range,
    );
}

/// **The surfaces past the first are 40% more geometry.**
///
/// Which is what makes this a rendering fix rather than a tidy-up: before the
/// surface table was read, 25,972 submeshes and 7.1 million triangles were
/// present in every model on the disc and drawn by nothing.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_surfaces_past_the_first_carry_forty_percent_more_geometry() {
    if image().is_none() {
        return;
    }
    let t = sweep();
    assert_eq!(t.submeshes_first, 50_873);
    assert_eq!(t.submeshes_rest, 25_972);
    assert_eq!(t.triangles_first, 17_505_484);
    assert_eq!(t.triangles_rest, 7_111_578);
}

/// **A chunk's `+0x07` byte says which space its positions are in.**
///
/// The field `rcsmodel` carried for months as "takes the values `01` and `02`
/// for a reason nothing here has distinguished", and which
/// `oag_mesh::mesh::rcs::is_world_baked` used to guess with an invented
/// one-unit tolerance. Three independent things are asserted here, because no
/// one of them would settle it alone:
///
/// - **the counts**, and that nothing on the disc carries a third value;
/// - **the categories**: every chunk of `data/ships` and `data/fe` is
///   node-local and every chunk of `data/pvsblocker` is world-baked, which is
///   what those directories mean;
/// - **the bias**, which is independent of the byte: a world-baked chunk's
///   bias is a world position and a node-local one's is near zero.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_space_byte_separates_world_baked_geometry_from_node_local() {
    if image().is_none() {
        return;
    }
    let image = image().expect("checked above");
    let (mut world, mut node, mut unknown) = (0usize, 0usize, 0usize);
    // Chunks of `data/environments` whose bias is under a unit, by space.
    let (mut world_at_origin, mut node_at_origin) = (0usize, 0usize);
    let (mut world_env, mut node_env) = (0usize, 0usize);
    for archive in ARCHIVES {
        let spec = format!("{}:PS3_GAME/USRDIR/{archive}.PSARC", image.display());
        let mut open = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
        let paths: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.ends_with(".rcsmodel"))
            .cloned()
            .collect();
        for path in paths {
            let Ok(blob) = open.read_path(&path) else {
                continue;
            };
            let Ok(model) = rcsmodel::Model::parse(&blob) else {
                continue;
            };
            for chunk in &model.meshes {
                match chunk.space {
                    rcsmodel::Space::World => world += 1,
                    rcsmodel::Space::Node => node += 1,
                    rcsmodel::Space::Unknown(_) => unknown += 1,
                }
                // What a directory means, asserted rather than assumed.
                if path.starts_with("/data/ships/") || path.starts_with("/data/fe/") {
                    assert_eq!(
                        chunk.space,
                        rcsmodel::Space::Node,
                        "{path}: a craft or front-end chunk is authored in its own space",
                    );
                }
                if path.starts_with("/data/pvsblocker/") {
                    assert_eq!(
                        chunk.space,
                        rcsmodel::Space::World,
                        "{path}: an occlusion blocker is world geometry",
                    );
                }
                if !path.starts_with("/data/environments/") {
                    continue;
                }
                let at_origin =
                    (chunk.bias[0].powi(2) + chunk.bias[1].powi(2) + chunk.bias[2].powi(2)).sqrt()
                        < 1.0;
                match chunk.space {
                    rcsmodel::Space::World => {
                        world_env += 1;
                        world_at_origin += usize::from(at_origin);
                    }
                    rcsmodel::Space::Node => {
                        node_env += 1;
                        node_at_origin += usize::from(at_origin);
                    }
                    rcsmodel::Space::Unknown(_) => {}
                }
            }
        }
    }
    assert_eq!((world, node, unknown), (33_088, 8_773, 0));

    // The separation, which is what makes the reading a reading rather than a
    // correlation: world-baked chunks essentially never sit at the origin, and
    // a third of node-local ones do.
    let world_share = world_at_origin as f64 / world_env as f64;
    let node_share = node_at_origin as f64 / node_env as f64;
    assert!(
        world_share < 0.001,
        "{world_at_origin} of {world_env} world-baked circuit chunk(s) have a bias under a unit",
    );
    assert!(
        node_share > 0.25,
        "only {node_at_origin} of {node_env} node-local circuit chunk(s) have a bias under a unit",
    );
}
