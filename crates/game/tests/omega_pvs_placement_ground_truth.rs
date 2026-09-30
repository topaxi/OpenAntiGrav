//! The `.pvs` join, measured on what the **renderer** draws rather than on the
//! model's raw vertices - and split by how each draw is placed.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! `oag_rcs`' `psp2_pvs_ground_truth` proves that bit `k` of a cell is mesh
//! object `k` for the mesh objects authored in world space, and **has to leave
//! out every mesh object a node places**, because those vertices are in the
//! node's space. **The bits answer it directly: a node-placed mesh object is
//! set in every cell** (1,623 of 1,623 on `tech_de_ra`, none partial), so the
//! PVS never culls one - the original partitions only what is authored in world
//! space, 1,036 of that circuit's 2,659 mesh objects, of which a cell sets 17%
//! on average and 56 no cell sets at all. So there is nothing for the join to
//! get wrong on node-placed scenery, and the correlation there is zero because
//! the bit is constant, which is a different thing from a broken index.
//!
//! The world-placed half is measured here on what the **renderer** draws:
//! `psp2::build`'s bounds, the numbers `oag_render::pvs::ChunkSet` is actually
//! tested against, with the offset correlation peaking at zero.
//!
//! ```sh
//! just test-data
//! ```

use oag_rcs::hd_pvs::Pvs;
use oag_rcs::rcsmodel::psp2;
use oag_render::mesh::rcs::psp2 as build;

/// How far the chunk index is shifted either way.
const SHIFT: i64 = 3;

/// One circuit direction, from the base archive that holds it.
struct Circuit {
    archive: &'static str,
    model: &'static str,
    pvs: &'static str,
}

const CIRCUITS: &[Circuit] = &[
    Circuit {
        archive: "data02.psarc",
        model: "Data/environments/tech_de_ra/track.final.rcsmodel",
        pvs: "Data/environments/tech_de_ra/track.final.pvs",
    },
    Circuit {
        archive: "data02.psarc",
        model: "Data/environments/tech_de_ra/track_reversed.final.rcsmodel",
        pvs: "Data/environments/tech_de_ra/track_reversed.final.pvs",
    },
    Circuit {
        archive: "data02.psarc",
        model: "Data/environments/talons_junction/track.final.rcsmodel",
        pvs: "Data/environments/talons_junction/track.final.pvs",
    },
    Circuit {
        archive: "data04.psarc",
        model: "Data/environments2048/altima/track.final.rcsmodel",
        pvs: "Data/environments2048/altima/track.final.pvs",
    },
];

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

fn pearson(xs: &[f64], ys: &[f64]) -> f64 {
    let n = xs.len() as f64;
    let (mx, my) = (xs.iter().sum::<f64>() / n, ys.iter().sum::<f64>() / n);
    let (mut sxy, mut sxx, mut syy) = (0.0, 0.0, 0.0);
    for (x, y) in xs.iter().zip(ys) {
        sxy += (x - mx) * (y - my);
        sxx += (x - mx).powi(2);
        syy += (y - my).powi(2);
    }
    sxy / (sxx * syy).sqrt().max(f64::MIN_POSITIVE)
}

/// Correlation at each offset `-SHIFT..=SHIFT` over the draws `keep` selects.
fn by_offset(
    pvs: &Pvs,
    draws: &[(u32, [f32; 3])],
    keep: impl Fn(usize) -> bool,
) -> (usize, Vec<f64>) {
    let chosen: Vec<&(u32, [f32; 3])> = draws
        .iter()
        .enumerate()
        .filter(|&(i, _)| keep(i))
        .map(|(_, d)| d)
        .collect();
    let step = (pvs.cells() / 40).max(1);
    let mut out = Vec::new();
    for offset in -SHIFT..=SHIFT {
        let (mut xs, mut ys) = (Vec::new(), Vec::new());
        for cell in (0..pvs.cells()).step_by(step) {
            let at = pvs.position(cell).expect("a declared cell");
            for &&(chunk, centre) in &chosen {
                let k = i64::from(chunk) + offset;
                if k >= 0 && (k as usize) < pvs.chunks() {
                    xs.push(f64::from(1.0 / (1.0 + distance(at, centre) / 30.0)));
                    ys.push(f64::from(u8::from(pvs.visible(cell, k as usize))));
                }
            }
        }
        out.push(pearson(&xs, &ys));
    }
    (chosen.len(), out)
}

fn peaks_at_zero(label: &str, r: &[f64]) {
    let zero = r[SHIFT as usize];
    let best_elsewhere = r
        .iter()
        .enumerate()
        .filter(|&(i, _)| i != SHIFT as usize)
        .map(|(_, v)| *v)
        .fold(f64::MIN, f64::max);
    assert!(
        zero > best_elsewhere + 0.01,
        "{label}: the correlation by index offset {:?} does not peak at zero",
        r.iter()
            .map(|v| (v * 100.0).round() / 100.0)
            .collect::<Vec<_>>()
    );
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu"]
fn world_placed_draws_peak_at_offset_zero_and_node_placed_ones_are_never_culled() {
    for circuit in CIRCUITS {
        let Some(path) = oag_testdata::exact(&format!(
            "data/extracted/ps4/omega-eu/uroot/{}",
            circuit.archive
        )) else {
            return;
        };
        let mut archive = oag_assets::psarc::Archive::open_file(&path).expect("the archive opens");
        let blob = archive.read_path(circuit.model).expect("the model reads");
        let pvs = Pvs::parse_detect(&archive.read_path(circuit.pvs).expect("the pvs reads"))
            .expect("it parses");
        let decoded = psp2::parse(&blob).expect("it parses");
        let (model, _) =
            build::build(circuit.model, &blob, None, &mut |_| None).expect("it builds");
        assert_eq!(
            model.draws.len(),
            decoded.submeshes.len(),
            "{}: one draw per submesh, in submesh order",
            circuit.model
        );
        assert_eq!(
            pvs.chunks(),
            decoded.scene.meshes.len(),
            "{}",
            circuit.model
        );
        // (chunk, world centre) for every draw that has geometry and stays put.
        let draws: Vec<(u32, [f32; 3])> = model
            .draws
            .iter()
            .filter(|d| d.bounds.radius > 0.0 && !d.moving)
            .map(|d| {
                (
                    d.chunk.expect("every psp2 draw carries its mesh object"),
                    d.bounds.centre,
                )
            })
            .collect();
        let placed_by_node: Vec<bool> = model
            .draws
            .iter()
            .zip(&decoded.submeshes)
            .filter(|(d, _)| d.bounds.radius > 0.0 && !d.moving)
            .map(|(_, s)| s.node.is_some())
            .collect();
        let (n_world, world) = by_offset(&pvs, &draws, |i| !placed_by_node[i]);
        println!(
            "{}: {n_world} world-placed draws, correlation by offset {:?}",
            circuit.model,
            world
                .iter()
                .map(|v| (v * 100.0).round() / 100.0)
                .collect::<Vec<_>>(),
        );
        assert!(n_world > 100, "{}: world-placed draws", circuit.model);
        peaks_at_zero(&format!("{} world-placed", circuit.model), &world);
        // Node-placed mesh objects: set in (nearly) every cell, all of them.
        let mut node_objects = std::collections::BTreeSet::new();
        for s in decoded.submeshes.iter().filter(|s| s.node.is_some()) {
            node_objects.extend(s.mesh);
        }
        let partial = node_objects
            .iter()
            .filter(|&&k| {
                let set = (0..pvs.cells()).filter(|&c| pvs.visible(c, k)).count();
                (set as f64) < 0.99 * pvs.cells() as f64
            })
            .count();
        println!(
            "{}: {} node-placed mesh objects, {partial} set in fewer than 99% of cells",
            circuit.model,
            node_objects.len()
        );
        assert_eq!(
            partial, 0,
            "{}: the PVS partitions no node-placed mesh object",
            circuit.model
        );
    }
}

/// A mesh object no cell sets is one HD's engine never draws (its loader was
/// read to use one cell's bitmap as the frame's visible set), so it is
/// *inferred*, at about confidence 75, that Omega's does not either; the
/// renderer draws it only when the PVS is off.
///
/// `tech_de_ra` forward's mesh object 2412, `tracksurface:wohdtrack_0022Shape`,
/// is two triangles of `track_surface_displacement2out` in world space, is set
/// in **none of the circuit's 882 cells**, and is the one draw that separates
/// the `--pvs false` and `--pvs true` frames at tick 300 and 600 (a dark slab
/// on the right-hand wall). This pins those facts, so the difference between
/// the two pictures is a named object and not a mystery.
#[test]
#[ignore = "needs data/extracted/ps4/omega-eu"]
fn the_forward_frames_only_difference_is_a_mesh_object_no_cell_draws() {
    let Some(path) = oag_testdata::exact("data/extracted/ps4/omega-eu/uroot/data02.psarc") else {
        return;
    };
    let mut archive = oag_assets::psarc::Archive::open_file(&path).expect("the archive opens");
    let blob = archive
        .read_path("Data/environments/tech_de_ra/track.final.rcsmodel")
        .expect("the model reads");
    let pvs = Pvs::parse_detect(
        &archive
            .read_path("Data/environments/tech_de_ra/track.final.pvs")
            .expect("the pvs reads"),
    )
    .expect("it parses");
    let decoded = psp2::parse(&blob).expect("it parses");
    let k = 2412;
    let mesh = &decoded.scene.meshes[k];
    assert_eq!(mesh.name, "tracksurface:wohdtrack_0022Shape");
    let submeshes: Vec<_> = decoded
        .submeshes
        .iter()
        .filter(|s| s.mesh == Some(k))
        .collect();
    assert_eq!(submeshes.len(), 1);
    assert_eq!(submeshes[0].triangle_count(), 2);
    assert!(submeshes[0].node.is_none(), "authored in world space");
    let material = &decoded.materials[submeshes[0].material.expect("a material")];
    assert!(
        material
            .name
            .ends_with("track_surface_displacement2out.rcsmaterial")
    );
    let cells_that_set_it = (0..pvs.cells()).filter(|&c| pvs.visible(c, k)).count();
    assert_eq!((cells_that_set_it, pvs.cells()), (0, 882));
    // And it is not alone: some mesh objects are set by no cell at all.
    let never = (0..pvs.chunks())
        .filter(|&k| (0..pvs.cells()).all(|c| !pvs.visible(c, k)))
        .count();
    println!(
        "{never} of {} mesh objects are set by no cell",
        pvs.chunks()
    );
    assert!(never > 0);
}
