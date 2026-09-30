//! What the discs say about `.pvs` in the 2048 lineage's layout, over every
//! shipped file of Wipeout: Omega Collection (PS4) and Wipeout 2048 (Vita)
//! rather than the one the reader was first written against.
//!
//! The claims `oag_rcs::hd_pvs`' "A second dialect" section rests on, made
//! executable: header word 2 is `1` little-endian, the bitmap is
//! `ceil(chunks / 8)` bytes and the file is *exactly* its own table, the
//! declared chunk count is the sibling `.rcsmodel`'s **mesh-object** count and
//! not its submesh count, the cell positions are the `.pvsxml`'s, and bit `k`
//! is mesh object `k` - the last one measured spatially, the way
//! `hd_pvs_ground_truth` measured HD's.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! One archive per test, so the sweep parallelises across tests.

use std::path::{Path, PathBuf};

use oag_rcs::hd_pvs::{Dialect, Pvs};
use oag_rcs::rcsmodel::psp2;

fn archive_path(dir: &str, name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted")
        .join(dir)
        .join(name);
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

/// One file's measurements.
#[derive(Debug)]
struct Measured {
    path: String,
    cells: usize,
    chunks: usize,
    /// The sibling model's mesh objects and submeshes.
    mesh_objects: usize,
    submeshes: usize,
    trailing: usize,
    /// Mean of (chunks set in a cell) / chunks, over every cell.
    mean_fraction: f64,
    /// The same fraction, weighted by each mesh object's triangles.
    mean_triangle_fraction: f64,
    /// Of the `.pvsxml`'s `<origin>`s, how many have no cell record within
    /// 0.01 of them, and how many origins there are; `None` when there is no
    /// `.pvsxml`.
    origin_disagreements: Option<(usize, usize)>,
    /// (near, far) fraction set at index offsets `-SHIFT..=SHIFT`.
    near_far: [(f64, f64); 2 * SHIFT as usize + 1],
    /// Pearson correlation between a cell's bit for mesh object `k + offset`
    /// and how near that cell is to mesh object `k`, at each offset.
    correlation: [f64; 2 * SHIFT as usize + 1],
    /// Set bits the last byte of a bitmap carries beyond the declared chunks.
    stray_padding_bits: usize,
}

/// The `<origin>x y z</origin>` values of a `.pvsxml`, in order.
fn pvsxml_origins(xml: &str) -> Vec<[f32; 3]> {
    let mut out = Vec::new();
    let mut rest = xml;
    while let Some(at) = rest.find("<origin>") {
        rest = &rest[at + "<origin>".len()..];
        let Some(end) = rest.find("</origin>") else {
            break;
        };
        let v: Vec<f32> = rest[..end]
            .split_whitespace()
            .filter_map(|t| t.parse().ok())
            .collect();
        if let [x, y, z] = v[..] {
            out.push([x, y, z]);
        }
        rest = &rest[end..];
    }
    out
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

fn distance(a: [f32; 3], b: [f32; 3]) -> f32 {
    ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
}

/// How many mesh objects a cell is compared against at each end.
const ENDS: usize = 50;

/// How far the index is shifted either way to show the peak is at zero.
const SHIFT: i64 = 3;

/// How far above the best shifted offset the correlation at zero must sit.
const MARGIN: f64 = 0.01;

/// Where offset zero sits in [`Measured::near_far`].
const ZERO: usize = SHIFT as usize;

fn measure(
    archive: &mut oag_assets::psarc::Archive,
    pvs_path: &str,
    blob: &[u8],
    model_blob: &[u8],
) -> Measured {
    let pvs = Pvs::parse_detect(blob).unwrap_or_else(|e| panic!("{pvs_path}: {e}"));
    let model = psp2::parse(model_blob).unwrap_or_else(|e| panic!("{pvs_path}: model: {e}"));
    let meshes = &model.scene.meshes;

    let mut triangles = vec![0usize; meshes.len()];
    let mut lo = vec![[f32::MAX; 3]; meshes.len()];
    let mut hi = vec![[f32::MIN; 3]; meshes.len()];
    let mut all_static = vec![true; meshes.len()];
    for sub in &model.submeshes {
        let Some(mesh) = sub.mesh.filter(|&m| m < meshes.len()) else {
            continue;
        };
        triangles[mesh] += sub.triangle_count();
        if sub.node.is_some() {
            all_static[mesh] = false;
            continue;
        }
        for p in &sub.positions {
            for a in 0..3 {
                lo[mesh][a] = lo[mesh][a].min(p[a]);
                hi[mesh][a] = hi[mesh][a].max(p[a]);
            }
        }
    }
    // A mesh object placed by a node is in that node's space, so its bounds
    // say nothing about where it is drawn: left out of the spatial test.
    let centres: Vec<Option<[f32; 3]>> = (0..meshes.len())
        .map(|m| {
            (all_static[m] && lo[m][0] <= hi[m][0])
                .then(|| std::array::from_fn(|a| (lo[m][a] + hi[m][a]) / 2.0))
        })
        .collect();
    let located: Vec<usize> = (0..meshes.len())
        .filter(|&m| centres[m].is_some())
        .collect();

    let total_triangles: usize = triangles.iter().sum();
    let mut sum_fraction = 0.0;
    let mut sum_triangle_fraction = 0.0;
    let mut stray = 0usize;
    for cell in 0..pvs.cells() {
        sum_fraction += f64::from(pvs.visible_count(cell)) / pvs.chunks() as f64;
        let seen: usize = (0..pvs.chunks().min(meshes.len()))
            .filter(|&k| pvs.visible(cell, k))
            .map(|k| triangles[k])
            .sum();
        sum_triangle_fraction += seen as f64 / total_triangles.max(1) as f64;
        let bits = pvs.cell_bits(cell).expect("a declared cell");
        let spare = bits.len() * 8 - pvs.chunks();
        if spare > 0 {
            let last = bits[bits.len() - 1];
            stray += (last >> (8 - spare)).count_ones() as usize;
        }
    }

    // Omega's `track.final.pvs` is `track.pvsxml` beside it: the `.final`
    // infix marks a baked output and the text source does not carry it.
    let origin_disagreements = [
        pvs_path.replace(".final.pvs", ".pvsxml"),
        pvs_path.replace(".pvs", ".pvsxml"),
    ]
    .iter()
    .filter(|name| name.ends_with(".pvsxml"))
    .find_map(|name| archive.read_path(name).ok())
    .map(|xml| {
        let origins = pvsxml_origins(&String::from_utf8_lossy(&xml));
        // Each origin the text has, against every cell of the binary: the
        // text is often shorter and not always in the binary's order.
        let bad = origins
            .iter()
            .filter(|o| {
                pvs.nearest_cell(**o)
                    .is_none_or(|c| distance(pvs.position(c).expect("a declared cell"), **o) > 0.01)
            })
            .count();
        (bad, origins.len())
    });

    let step = (pvs.cells() / 40).max(1);
    let sampled: Vec<usize> = (0..pvs.cells()).step_by(step).collect();
    let mut near_far = [(0.0, 0.0); 2 * SHIFT as usize + 1];
    for (slot, offset) in (-SHIFT..=SHIFT).enumerate() {
        let (mut near_set, mut near_n, mut far_set, mut far_n) = (0usize, 0usize, 0usize, 0usize);
        for &cell in &sampled {
            let at = pvs.position(cell).expect("a declared cell");
            let mut order = located.clone();
            order.sort_by(|&a, &b| {
                distance(at, centres[a].unwrap())
                    .partial_cmp(&distance(at, centres[b].unwrap()))
                    .unwrap()
            });
            let take = ENDS.min(order.len() / 2);
            let probe = |set: &mut usize, n: &mut usize, mesh: usize| {
                let k = mesh as i64 + offset;
                if k >= 0 && (k as usize) < pvs.chunks() {
                    *n += 1;
                    *set += usize::from(pvs.visible(cell, k as usize));
                }
            };
            for &mesh in order.iter().take(take) {
                probe(&mut near_set, &mut near_n, mesh);
            }
            for &mesh in order.iter().rev().take(take) {
                probe(&mut far_set, &mut far_n, mesh);
            }
        }
        near_far[slot] = (
            near_set as f64 / near_n.max(1) as f64,
            far_set as f64 / far_n.max(1) as f64,
        );
    }

    let mut correlation = [0.0; 2 * SHIFT as usize + 1];
    for (slot, offset) in (-SHIFT..=SHIFT).enumerate() {
        let (mut xs, mut ys) = (Vec::new(), Vec::new());
        for &cell in &sampled {
            let at = pvs.position(cell).expect("a declared cell");
            for &mesh in &located {
                let k = mesh as i64 + offset;
                if k >= 0 && (k as usize) < pvs.chunks() {
                    let d = distance(at, centres[mesh].unwrap());
                    xs.push(f64::from(1.0 / (1.0 + d / 30.0)));
                    ys.push(f64::from(u8::from(pvs.visible(cell, k as usize))));
                }
            }
        }
        correlation[slot] = pearson(&xs, &ys);
    }

    Measured {
        path: pvs_path.to_string(),
        cells: pvs.cells(),
        chunks: pvs.chunks(),
        mesh_objects: meshes.len(),
        submeshes: model.submeshes.len(),
        trailing: pvs.trailing(),
        mean_fraction: sum_fraction / pvs.cells() as f64,
        mean_triangle_fraction: sum_triangle_fraction / pvs.cells() as f64,
        origin_disagreements,
        near_far,
        correlation,
        stray_padding_bits: stray,
    }
}

/// Every `.pvs` of an archive whose sibling model is in the same archive.
fn sweep(dir: &str, name: &str) -> Option<(Vec<Measured>, usize)> {
    let path = archive_path(dir, name)?;
    let mut archive = oag_assets::psarc::Archive::open_file(&path).expect("the archive opens");
    let paths: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.ends_with(".pvs"))
        .cloned()
        .collect();
    let mut out = Vec::new();
    let mut without_model = 0;
    for pvs_path in paths {
        let blob = archive.read_path(&pvs_path).expect("the .pvs reads");
        let Ok(model) = archive.read_path(&pvs_path.replace(".pvs", ".rcsmodel")) else {
            without_model += 1;
            continue;
        };
        out.push(measure(&mut archive, &pvs_path, &blob, &model));
    }
    Some((out, without_model))
}

/// The claims that hold for every file, and the numbers worth printing.
fn check(
    label: &str,
    files: &[Measured],
    without_model: usize,
    expect: usize,
    disagreeing: &[&str],
    files_with_origins: usize,
) {
    for m in files {
        println!(
            "{label} {}: {} cells x {} chunks ({} mesh objects, {} submeshes), sees {:.1}% of \
             chunks, {:.1}% of triangles; near/far at 0: {:.0}%/{:.0}%, at +-1: {:.0}%/{:.0}% \
             and {:.0}%/{:.0}%; correlation by offset {:?}",
            m.path,
            m.cells,
            m.chunks,
            m.mesh_objects,
            m.submeshes,
            m.mean_fraction * 100.0,
            m.mean_triangle_fraction * 100.0,
            m.near_far[ZERO].0 * 100.0,
            m.near_far[ZERO].1 * 100.0,
            m.near_far[ZERO - 1].0 * 100.0,
            m.near_far[ZERO - 1].1 * 100.0,
            m.near_far[ZERO + 1].0 * 100.0,
            m.near_far[ZERO + 1].1 * 100.0,
            m.correlation.map(|r| (r * 100.0).round() / 100.0),
        );
    }
    assert_eq!(
        files.len(),
        expect,
        "{label}: {} .pvs file(s) with a sibling model ({without_model} without one)",
        files.len()
    );
    let with_origins: Vec<&Measured> = files
        .iter()
        .filter(|m| m.origin_disagreements.is_some_and(|(_, n)| n > 0))
        .collect();
    assert_eq!(
        with_origins.len(),
        files_with_origins,
        "{label}: files whose .pvsxml carries <origin>s to compare"
    );
    let mismatched: Vec<&str> = files
        .iter()
        .filter(|m| m.chunks != m.mesh_objects)
        .map(|m| m.path.as_str())
        .collect();
    assert_eq!(
        mismatched, disagreeing,
        "{label}: the files whose declared chunk count is not their sibling model's mesh-object \
         count"
    );
    for m in files.iter().filter(|m| m.chunks == m.mesh_objects) {
        assert_ne!(
            m.chunks, m.submeshes,
            "{}: and it is not the submesh count",
            m.path
        );
        assert_eq!(
            m.trailing, 0,
            "{}: the file is exactly cells x (16 + ceil(chunks / 8)) + 16 bytes",
            m.path
        );
        assert_eq!(
            m.stray_padding_bits, 0,
            "{}: a bitmap's spare bits are clear",
            m.path
        );
        assert!(
            (0.02..0.85).contains(&m.mean_fraction),
            "{}: cells see {:.1}% of the circuit on average",
            m.path,
            m.mean_fraction * 100.0
        );
        // The `.pvsxml` is a text-authored precursor and is usually *shorter*
        // than the baked binary (Omega's Moa Therma reversed carries 445
        // origins beside 675 cells), and not always in its order. So it is a
        // second source for the positions it does carry - each origin is one
        // of the binary's cell records - and not for how many cells there are.
        // The ten zone-mode `trackzone.pvs` files have no `.pvsxml`, and of the
        // Vita's 28 only its DLC1 circuits (8 files) carry `<origin>`s.
        if let Some((bad, origins)) = m.origin_disagreements.filter(|&(_, n)| n > 0) {
            assert_eq!(
                bad, 0,
                "{}: {bad} of the .pvsxml's {origins} origin(s) are not a cell record",
                m.path
            );
            assert!(origins <= m.cells, "{}: more origins than cells", m.path);
        }
        // The near/far fractions are reported, and only loosely asserted:
        // a circuit whose cells see three quarters of everything (Sol 2)
        // leaves little room between the ends.
        let (near, _far) = m.near_far[ZERO];
        assert!(
            near > 0.6,
            "{}: at offset zero the nearest {ENDS} mesh objects are only {:.0}% visible",
            m.path,
            near * 100.0,
        );
        // A sharp peak at zero, not a plateau: of the offsets `-SHIFT..=SHIFT`
        // the correlation between a bit and its mesh object's nearness is
        // largest at zero. Adjacent mesh objects sit near each other, so a
        // shift of one keeps much of the signal - which is why the test is the
        // argmax and not an absolute.
        let best_elsewhere = m
            .correlation
            .iter()
            .enumerate()
            .filter(|&(i, _)| i != ZERO)
            .map(|(_, r)| *r)
            .fold(f64::MIN, f64::max);
        assert!(
            m.correlation[ZERO] > best_elsewhere + MARGIN,
            "{}: the correlation by index offset {:?} does not peak at zero",
            m.path,
            m.correlation.map(|r| (r * 100.0).round() / 100.0)
        );
    }
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu"]
fn omega_data00() {
    if let Some((files, without)) = sweep("ps4/omega-eu/uroot", "data00.psarc") {
        check("omega data00", &files, without, 4, &[], 2);
    }
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu"]
fn omega_data01() {
    if let Some((files, without)) = sweep("ps4/omega-eu/uroot", "data01.psarc") {
        check("omega data01", &files, without, 8, &[], 8);
    }
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu"]
fn omega_data02() {
    if let Some((files, without)) = sweep("ps4/omega-eu/uroot", "data02.psarc") {
        check("omega data02", &files, without, 16, &[], 16);
    }
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu"]
fn omega_data04() {
    if let Some((files, without)) = sweep("ps4/omega-eu/uroot", "data04.psarc") {
        check("omega data04", &files, without, 16, &[], 8);
    }
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn vita_2048_base() {
    if let Some((files, without)) = sweep("vita/PCSF00007/base/PSP2", "data.psarc") {
        check(
            "2048 base",
            &files,
            without,
            28,
            &[
                "data/art/published/DLC1/environments/Anulpha_Pass/track_reversed.pvs",
                "data/art/published/DLC1/environments/Chenghou_Project/track_reversed.pvs",
                "data/art/published/DLC1/environments/Moa_Therma/track_reversed.pvs",
                "data/art/published/DLC1/environments/Vineta_K/track_reversed.pvs",
            ],
            8,
        );
    }
}

/// Header word 2 tells the dialects apart on a whole disc, which is what
/// [`Pvs::parse_detect`] leans on: every Omega and Vita file is
/// [`Dialect::Psp2`], and none of them parses as HD's.
#[test]
#[ignore = "needs data/extracted/ps4/omega-eu"]
fn every_omega_file_is_the_psp2_dialect_and_none_parses_as_hds() {
    let Some(path) = archive_path("ps4/omega-eu/uroot", "data02.psarc") else {
        return;
    };
    let mut archive = oag_assets::psarc::Archive::open_file(&path).expect("the archive opens");
    let paths: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.ends_with(".pvs"))
        .cloned()
        .collect();
    assert!(!paths.is_empty());
    for path in paths {
        let blob = archive.read_path(&path).expect("reads");
        assert_eq!(
            Pvs::parse_detect(&blob).expect("parses").dialect(),
            Dialect::Psp2,
            "{path}"
        );
        assert!(Pvs::parse(&blob).is_err(), "{path}: HD's reader refuses it");
    }
}
