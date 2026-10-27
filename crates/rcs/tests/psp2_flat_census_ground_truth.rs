//! Census of 2048's (and Omega's) flat submeshes: which draw with empty or all-zero diffuse
//! texcoords, how many triangles that is, and whether the record's own declaration pointer
//! (`+0x28`, a `u32` on Vita) resolves where the stride lookup guessed.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not
//! ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! `OAG_SWEEP=1` prints the per-circuit table. One archive per test so the sweep parallelises.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use oag_rcs::rcsmodel::psp2::{self, vertex_decl};

fn extracted(rel: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted")
        .join(rel);
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

/// Submeshes and triangles in one class.
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq)]
struct Area {
    submeshes: usize,
    triangles: usize,
}

impl Area {
    fn add(&mut self, tris: usize) {
        self.submeshes += 1;
        self.triangles += tris;
    }
    fn plus(&mut self, other: Area) {
        self.submeshes += other.submeshes;
        self.triangles += other.triangles;
    }
}

#[derive(Default, Clone, Debug, PartialEq, Eq)]
struct Census {
    files: usize,
    all: Area,
    /// The reader as shipped: no `Uv1` in the declaration it chose.
    empty: Area,
    /// Every vertex decoded to (0,0).
    all_zero: Area,
    /// Some but not all vertices at (0,0).
    partly_zero: Area,
    /// At least one non-finite coordinate substituted (any of the classes above or none).
    non_finite: Area,
    /// The record's own pointer reaches a declaration header of the record's stride.
    resolved: Area,
    /// Of those, the declaration the reader used (the stride map's) puts `Uv1` elsewhere,
    /// or lacks it, or the own declaration does.
    disagree: Area,
    /// Of `disagree`, those where the reader's offset lands in the own declaration on the
    /// lightmap coordinate, a vertex colour, a tangent or no attribute: wrong whichever
    /// coordinate turns out to be the diffuse one / on another non-lightmap two-component
    /// attribute (depends on the fallback choice).
    disagree_not_a_coordinate: Area,
    disagree_other_coordinate: Area,
    /// Resolved submeshes whose `+0x28` word is a relocation site of the CPU section's table.
    relocated: usize,
    /// Of those, the own declaration has no `Uv1` at all.
    own_empty: Area,
    /// Reading `Uv1` through the own declaration, resolved submeshes only: all-zero.
    own_all_zero: Area,
    /// Of the submeshes the reader draws flat (empty or all-zero), those the own pointer
    /// resolves and reads a non-flat coordinate from.
    flat_fixed: Area,
    /// Resolved submeshes whose own diffuse attribute is a float pair, which no reader decodes.
    own_float: Area,
    /// Resolved submeshes whose material names a lightmap sampler / whose own declaration
    /// carries `lightmapUV`: both, neither, and the two off-diagonal cells.
    lm_both: usize,
    lm_neither: usize,
    lm_material_only: usize,
    lm_decl_only: usize,
    /// Resolved submeshes whose own declaration names no diffuse coordinate although the
    /// material names a diffuse-family sampler / the reverse: a coordinate, no such sampler.
    own_empty_named_diffuse: usize,
    own_uv_no_named_diffuse: usize,
    /// Resolved submeshes whose vertex buffer pointer could not be located for the re-read.
    unlocated: usize,
}

impl Census {
    fn plus(&mut self, o: &Census) {
        self.files += o.files;
        self.all.plus(o.all);
        self.empty.plus(o.empty);
        self.all_zero.plus(o.all_zero);
        self.partly_zero.plus(o.partly_zero);
        self.non_finite.plus(o.non_finite);
        self.resolved.plus(o.resolved);
        self.disagree.plus(o.disagree);
        self.own_empty.plus(o.own_empty);
        self.own_all_zero.plus(o.own_all_zero);
        self.flat_fixed.plus(o.flat_fixed);
        self.own_float.plus(o.own_float);
        self.disagree_not_a_coordinate
            .plus(o.disagree_not_a_coordinate);
        self.disagree_other_coordinate
            .plus(o.disagree_other_coordinate);
        self.relocated += o.relocated;
        self.lm_both += o.lm_both;
        self.lm_neither += o.lm_neither;
        self.lm_material_only += o.lm_material_only;
        self.lm_decl_only += o.lm_decl_only;
        self.own_empty_named_diffuse += o.own_empty_named_diffuse;
        self.own_uv_no_named_diffuse += o.own_uv_no_named_diffuse;
        self.unlocated += o.unlocated;
    }
    fn flat(&self) -> Area {
        Area {
            submeshes: self.empty.submeshes + self.all_zero.submeshes,
            triangles: self.empty.triangles + self.all_zero.triangles,
        }
    }
}

/// The diffuse attribute of a declaration known to be the submesh's own: `Uv1` where named,
/// otherwise the first two-component attribute that is not the lightmap's. `half` is the
/// container's two-half type nibble; a float pair (`2x9`) is returned as the second value of
/// the pair so the census can name it as not decoded.
fn own_diffuse(decl: &vertex_decl::VertexDecl, half: u8) -> Option<vertex_decl::Attribute> {
    decl.diffuse_texcoord().copied().or_else(|| {
        decl.attributes
            .iter()
            .find(|a| {
                a.name_hash != vertex_decl::LIGHTMAP_HASH
                    && a.components == 2
                    && (a.gxm_type == half || a.gxm_type == FLOAT)
            })
            .copied()
    })
}

/// The type nibble of an `f32` attribute (the position's `3x9`).
const FLOAT: u8 = 9;

fn census_of(blob: &[u8], ps4: bool) -> Census {
    let model = psp2::parse(blob).unwrap_or_else(|e| panic!("{e}"));
    let mut out = Census {
        files: 1,
        ..Census::default()
    };
    if model.submeshes.is_empty() {
        return out;
    }
    let (cpu, gpu) = (model.sections[0], model.sections[1]);
    let cpu_bytes = &blob[cpu.at..cpu.at + cpu.len];
    let sites: std::collections::HashSet<usize> = (0..cpu.entries)
        .map(|e| {
            let at = cpu.table + e * psp2::RELOCATION_LEN;
            u32::from_le_bytes(blob[at..at + 4].try_into().unwrap()) as usize
        })
        .collect();
    let by_header = vertex_decl::find_by_header(cpu_bytes);
    let by_stride = vertex_decl::find_by_stride(cpu_bytes);
    for s in &model.submeshes {
        let tris = s.triangle_count();
        out.all.add(tris);
        let zero = |uv: &[f32; 2]| uv[0] == 0.0 && uv[1] == 0.0;
        let zeroes = s.texcoords.iter().filter(|uv| zero(uv)).count();
        let flat = if s.texcoords.is_empty() {
            out.empty.add(tris);
            true
        } else if zeroes == s.texcoords.len() {
            out.all_zero.add(tris);
            true
        } else {
            if zeroes > 0 {
                out.partly_zero.add(tris);
            }
            false
        };
        if s.non_finite_texcoords > 0 {
            out.non_finite.add(tris);
        }
        let at = s.record + vertex_decl::PS4_DECLARATION_POINTER;
        let word = if ps4 {
            usize::try_from(u64::from_le_bytes(
                cpu_bytes[at..at + 8].try_into().unwrap(),
            ))
            .ok()
        } else {
            Some(u32::from_le_bytes(cpu_bytes[at..at + 4].try_into().unwrap()) as usize)
        };
        let Some(own) = word
            .and_then(|w| by_header.get(&w))
            .filter(|d| d.stride == s.stride)
        else {
            continue;
        };
        out.resolved.add(tris);
        out.relocated +=
            usize::from(sites.contains(&(s.record + vertex_decl::PS4_DECLARATION_POINTER)));
        if let Some(m) = s.material {
            let names = model.materials[m].lightmap.is_some()
                || model.materials[m]
                    .sampler(oag_rcs::rcsmaterial::LIGHTMAP_SAMPLER)
                    .is_some();
            match (names, own.lightmap_texcoord().is_some()) {
                (true, true) => out.lm_both += 1,
                (false, false) => out.lm_neither += 1,
                (true, false) => out.lm_material_only += 1,
                (false, true) => out.lm_decl_only += 1,
            }
        }
        let named_diffuse = s.material.is_some_and(|m| {
            psp2::material::DIFFUSE_SAMPLER_NAMES.iter().any(|n| {
                model.materials[m]
                    .sampler(oag_rcs::rcsmaterial::name_hash(n))
                    .is_some()
            })
        });
        let half = if ps4 { vertex_decl::PS4_HALF2 } else { 8 };
        let own_attr = own_diffuse(own, half);
        if own_attr.is_some_and(|a| a.gxm_type == FLOAT) {
            out.own_float.add(tris);
            continue;
        }
        let own_off = own_attr.map(|a| a.offset);
        match (own_attr.is_some(), named_diffuse) {
            (false, true) => out.own_empty_named_diffuse += 1,
            (true, false) => out.own_uv_no_named_diffuse += 1,
            _ => {}
        }
        let used_off = if ps4 {
            own_off
        } else {
            by_stride
                .get(&s.stride)
                .and_then(|d| d.diffuse_texcoord())
                .map(|a| a.offset)
        };
        if own_off != used_off {
            out.disagree.add(tris);
            let landed = used_off.and_then(|o| own.attributes.iter().find(|a| a.offset == o));
            if landed
                .is_some_and(|a| a.components == 2 && a.name_hash != vertex_decl::LIGHTMAP_HASH)
            {
                out.disagree_other_coordinate.add(tris);
            } else if used_off.is_some() {
                out.disagree_not_a_coordinate.add(tris);
            }
        }
        let Some(off) = own_off.map(usize::from) else {
            out.own_empty.add(tris);
            continue;
        };
        if s.stride < off + 4 {
            out.own_empty.add(tris);
            continue;
        }
        // The vertex pointer sits one known gap after the index pointer; the first position
        // read through it must be the decoded one.
        let mut vertex_ptr = None;
        for gap in [
            psp2::BUFFER_POINTER_GAP,
            psp2::SKY_BUFFER_POINTER_GAP,
            psp2::PS4_BUFFER_POINTER_GAP,
        ] {
            let width = if ps4 { 8 } else { 4 };
            let p = s.record + psp2::INDEX_POINTER + gap;
            let Some(w) = cpu_bytes.get(p..p + width) else {
                continue;
            };
            let ptr = if ps4 {
                u64::from_le_bytes(w.try_into().unwrap()) as usize
            } else {
                u32::from_le_bytes(w.try_into().unwrap()) as usize
            };
            let base = gpu.at + ptr;
            if let Some(b) = blob.get(base..base + 12) {
                let f = |k: usize| f32::from_le_bytes(b[k * 4..k * 4 + 4].try_into().unwrap());
                if [f(0), f(1), f(2)] == s.positions[0] {
                    vertex_ptr = Some(base);
                    break;
                }
            }
        }
        let Some(base) = vertex_ptr else {
            out.unlocated += 1;
            continue;
        };
        let mut all_zero = true;
        let mut finite_nonzero = false;
        for v in 0..s.positions.len() {
            let a = base + v * s.stride + off;
            let uv = psp2::unpack_texcoord(blob[a..a + 4].try_into().unwrap());
            if uv[0].is_finite() && uv[1].is_finite() {
                if !zero(&uv) {
                    all_zero = false;
                    finite_nonzero = true;
                }
            } else {
                // A non-finite value becomes the origin in the reader.
            }
        }
        let own_flat = all_zero && !finite_nonzero;
        if own_flat {
            out.own_all_zero.add(tris);
        }
        if flat && !own_flat {
            out.flat_fixed.add(tris);
        }
    }
    out
}

/// Per `(archive, directory)` census of one archive.
fn sweep(rel: &str, ps4: bool) -> Option<BTreeMap<String, Census>> {
    let path = extracted(rel)?;
    let mut archive = oag_assets::psarc::Archive::open(&path.display().to_string())
        .unwrap_or_else(|e| panic!("opening {}: {e}", path.display()));
    let mut entries: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with(".rcsmodel"))
        .cloned()
        .collect();
    entries.sort();
    let mut out: BTreeMap<String, Census> = BTreeMap::new();
    for entry in entries {
        let blob = archive
            .read_path(&entry)
            .unwrap_or_else(|e| panic!("reading {entry}: {e}"));
        let c = census_of(&blob, ps4);
        let dir = entry.rsplit_once('/').map_or("", |(d, _)| d);
        let dir = dir.strip_prefix("data/art/published/").unwrap_or(dir);
        let zone = entry.to_ascii_lowercase().ends_with("trackzone.rcsmodel");
        let key = if zone {
            format!("{dir} [zone]")
        } else {
            dir.to_string()
        };
        out.entry(key).or_default().plus(&c);
    }
    Some(out)
}

fn line(c: &Census) -> String {
    format!(
        "sub {:>6} tri {:>8} | empty {:>5}/{:>7} zero {:>5}/{:>7} partly {:>5}/{:>7} nonfin {:>5}/{:>7} | resolved {:>6} disagree {:>5}/{:>7} own_empty {:>5} own_zero {:>5}/{:>7} fixed {:>5}/{:>7} float {:>5}/{:>7} lm {}/{}/{}/{} nodiffuse_uv {}/{} unloc {}",
        c.all.submeshes,
        c.all.triangles,
        c.empty.submeshes,
        c.empty.triangles,
        c.all_zero.submeshes,
        c.all_zero.triangles,
        c.partly_zero.submeshes,
        c.partly_zero.triangles,
        c.non_finite.submeshes,
        c.non_finite.triangles,
        c.resolved.submeshes,
        c.disagree.submeshes,
        c.disagree.triangles,
        c.own_empty.submeshes,
        c.own_all_zero.submeshes,
        c.own_all_zero.triangles,
        c.flat_fixed.submeshes,
        c.flat_fixed.triangles,
        c.own_float.submeshes,
        c.own_float.triangles,
        c.lm_both,
        c.lm_neither,
        c.lm_material_only,
        c.lm_decl_only,
        c.own_empty_named_diffuse,
        c.own_uv_no_named_diffuse,
        c.unlocated,
    )
}

fn report(name: &str, by_dir: &BTreeMap<String, Census>) -> Census {
    let mut total = Census::default();
    let sweep = std::env::var_os("OAG_SWEEP").is_some();
    for (dir, c) in by_dir {
        total.plus(c);
        if sweep && (c.flat().triangles > 200 || dir.contains("nvironments")) {
            println!("{name} {dir}: {}", line(c));
        }
    }
    if sweep {
        println!("{name} TOTAL: {}", line(&total));
    }
    total
}

/// What a census must come to. Pinned from the first measured run; a change here is a change
/// of the corpus or of the reader, never a tolerance.
struct Expect {
    files: usize,
    submeshes: usize,
    /// Empty plus all-zero submeshes and their triangles, as the reader draws them today.
    flat: (usize, usize),
    /// Submeshes whose own pointer resolves.
    resolved: usize,
    /// Of those, where the reader's pick differs from the record's own declaration.
    disagree: usize,
    /// Of the flat ones, those the own declaration reads a real coordinate from.
    flat_fixed: (usize, usize),
    /// Submeshes with no diffuse coordinate in their own declaration whose material names a
    /// diffuse-family sampler. Zero on every Vita archive.
    empty_named_diffuse: usize,
}

fn check(name: &str, total: &Census, e: &Expect) {
    let got = Expect {
        files: total.files,
        submeshes: total.all.submeshes,
        flat: (total.flat().submeshes, total.flat().triangles),
        resolved: total.resolved.submeshes,
        disagree: total.disagree.submeshes,
        flat_fixed: (total.flat_fixed.submeshes, total.flat_fixed.triangles),
        empty_named_diffuse: total.own_empty_named_diffuse,
    };
    if std::env::var_os("OAG_SWEEP").is_some() {
        println!(
            "{name} PIN files {} sub {} flat {:?} resolved {} disagree {} fixed {:?}",
            got.files, got.submeshes, got.flat, got.resolved, got.disagree, got.flat_fixed
        );
    }
    assert_eq!(
        (
            got.files,
            got.submeshes,
            got.flat,
            got.resolved,
            got.disagree,
            got.flat_fixed,
            got.empty_named_diffuse,
        ),
        (
            e.files,
            e.submeshes,
            e.flat,
            e.resolved,
            e.disagree,
            e.flat_fixed,
            e.empty_named_diffuse,
        ),
        "{name}"
    );
    // The record's pointer is the submesh's own: it reaches a declaration of the record's
    // stride on every submesh, and its `lightmapUV` agrees with the material's lightmap
    // sampler in both directions (two independently read fields).
    assert_eq!(
        total.resolved.submeshes, total.all.submeshes,
        "{name}: pointer resolves"
    );
    assert_eq!(
        total.lm_material_only + total.lm_decl_only,
        0,
        "{name}: lightmap control group"
    );
    assert_eq!(total.unlocated, 0, "{name}: vertex buffer located");
}

macro_rules! vita {
    ($test:ident, $rel:expr, $expect:expr) => {
        #[test]
        #[ignore = "needs the extracted 2048 packages in data/extracted/vita/"]
        fn $test() {
            let Some(by_dir) = sweep($rel, false) else {
                return;
            };
            check(
                stringify!($test),
                &report(stringify!($test), &by_dir),
                &$expect,
            );
        }
    };
}

/// One entry of the base archive.
fn base_entry(entry: &str) -> Option<Census> {
    let path = extracted("vita/PCSF00007/base/PSP2/data.psarc")?;
    let mut archive = oag_assets::psarc::Archive::open(&path.display().to_string()).unwrap();
    let blob = archive
        .read_path(&format!("data/art/published/{entry}"))
        .unwrap_or_else(|e| panic!("{entry}: {e}"));
    Some(census_of(&blob, false))
}

/// Empire Climb (`environments/tower`), file by file. A normal race loads `track.rcsmodel`;
/// `trackZone.rcsmodel` (10,386 of the directory's 15,512 submeshes) is the Zone-mode circuit.
#[test]
#[ignore = "needs the extracted 2048 packages in data/extracted/vita/"]
fn empire_climb_is_pinned() {
    let area = |submeshes, triangles| Area {
        submeshes,
        triangles,
    };
    let Some(track) = base_entry("environments/tower/track.rcsmodel") else {
        return;
    };
    assert_eq!(track.all, area(4_957, 470_840));
    assert_eq!(track.resolved, track.all);
    assert_eq!(track.empty, area(685, 59_631));
    assert_eq!(track.all_zero, area(78, 1_449));
    assert_eq!(track.partly_zero, area(1_760, 209_561));
    assert_eq!(track.non_finite, area(1_456, 184_902));
    assert_eq!(track.disagree, area(4_477, 390_244));
    // The rest of `disagree` are the empty ones: the reader found no `Uv1`, the record has one.
    assert_eq!(track.disagree_not_a_coordinate, NOT_A_COORDINATE);
    assert_eq!(track.disagree_other_coordinate, OTHER_COORDINATE);
    assert_eq!(track.own_empty.submeshes, 22);
    assert_eq!(track.own_float, area(139, 25_125));
    assert_eq!(track.own_all_zero, area(0, 0));
    assert_eq!(track.flat_fixed, area(658, 38_563));
    assert_eq!((track.lm_both, track.lm_neither), (2_206, 2_751));
}

/// One circuit's row: submeshes and triangles, then empty, all-zero and partly-zero areas.
type Row = (
    &'static str,
    (usize, usize),
    (usize, usize),
    (usize, usize),
    (usize, usize),
);

fn rows(by_dir: &BTreeMap<String, Census>, keys: &[&'static str]) -> Vec<Row> {
    keys.iter()
        .map(|&k| {
            let c = &by_dir[k];
            (
                k,
                (c.all.submeshes, c.all.triangles),
                (c.empty.submeshes, c.empty.triangles),
                (c.all_zero.submeshes, c.all_zero.triangles),
                (c.partly_zero.submeshes, c.partly_zero.triangles),
            )
        })
        .collect()
}

const CIRCUITS: [&str; 10] = [
    "environments/altima",
    "environments/arena",
    "environments/bridge",
    "environments/cathedral",
    "environments/mall",
    "environments/park",
    "environments/sol",
    "environments/square",
    "environments/subway",
    "environments/tower",
];

/// Each base circuit without its Zone-mode `trackZone.rcsmodel` (which draws flat colour by
/// design): what a normal race loads, plus its skycube and start-grid parts.
#[test]
#[ignore = "needs the extracted 2048 packages in data/extracted/vita/"]
fn base_circuits_without_zone_are_pinned() {
    let Some(by_dir) = sweep("vita/PCSF00007/base/PSP2/data.psarc", false) else {
        return;
    };
    let got = rows(&by_dir, &CIRCUITS);
    if std::env::var_os("OAG_SWEEP").is_some() {
        for r in &got {
            println!("ROW {r:?},");
        }
    }
    assert_eq!(got, BASE_ROWS);
}

/// The v1.04 patch replaces three circuits' main model; a patched install loads these.
#[test]
#[ignore = "needs the extracted 2048 packages in data/extracted/vita/"]
fn patched_circuits_are_pinned() {
    let Some(by_dir) = sweep("vita/PCSF00007/patch-v104/PSP2/data2.psarc", false) else {
        return;
    };
    let got = rows(
        &by_dir,
        &[
            "environments/arena [zone]",
            "environments/bridge",
            "environments/park",
        ],
    );
    if std::env::var_os("OAG_SWEEP").is_some() {
        for r in &got {
            println!("PROW {r:?},");
        }
    }
    assert_eq!(got, PATCH_ROWS);
}

const NOT_A_COORDINATE: Area = Area {
    submeshes: 3_897,
    triangles: 353_130,
};
const OTHER_COORDINATE: Area = Area {
    submeshes: 0,
    triangles: 0,
};

const BASE_ROWS: &[Row] = &[
    (
        "environments/altima",
        (2979, 434182),
        (319, 33258),
        (0, 0),
        (120, 101577),
    ),
    (
        "environments/arena",
        (3849, 399934),
        (313, 32658),
        (4, 858),
        (157, 10341),
    ),
    (
        "environments/bridge",
        (5974, 600577),
        (5220, 511281),
        (0, 0),
        (96, 6488),
    ),
    (
        "environments/cathedral",
        (4995, 449440),
        (320, 30417),
        (0, 0),
        (252, 43860),
    ),
    (
        "environments/mall",
        (7505, 506303),
        (193, 15365),
        (0, 0),
        (435, 55027),
    ),
    (
        "environments/park",
        (5007, 381069),
        (827, 48429),
        (0, 0),
        (243, 24331),
    ),
    (
        "environments/sol",
        (2846, 272600),
        (597, 38476),
        (0, 0),
        (93, 44708),
    ),
    (
        "environments/square",
        (5886, 510592),
        (212, 18427),
        (0, 0),
        (521, 21375),
    ),
    (
        "environments/subway",
        (7403, 663407),
        (641, 63867),
        (2, 32),
        (629, 70607),
    ),
    (
        "environments/tower",
        (5126, 486724),
        (853, 75035),
        (78, 1449),
        (1760, 209561),
    ),
];
const PATCH_ROWS: &[Row] = &[
    (
        "environments/arena [zone]",
        (6797, 617816),
        (303, 13594),
        (2077, 252220),
        (765, 39065),
    ),
    (
        "environments/bridge",
        (5753, 580679),
        (5000, 492143),
        (0, 0),
        (96, 6488),
    ),
    (
        "environments/park",
        (4829, 365249),
        (650, 33089),
        (0, 0),
        (243, 24331),
    ),
];

const fn pin(
    files: usize,
    submeshes: usize,
    flat: (usize, usize),
    disagree: usize,
    flat_fixed: (usize, usize),
    empty_named_diffuse: usize,
) -> Expect {
    Expect {
        files,
        submeshes,
        flat,
        resolved: submeshes,
        disagree,
        flat_fixed,
        empty_named_diffuse,
    }
}

vita!(
    vita_base,
    "vita/PCSF00007/base/PSP2/data.psarc",
    pin(
        953,
        170_164,
        (36_441, 5_002_875),
        56_948,
        (10_659, 1_186_570),
        0
    )
);
vita!(
    vita_dlc1,
    "vita/PCSF00007/dlc1/PSP2/dlc1.psarc",
    pin(20, 53_440, (1_067, 82_198), 4_103, (689, 43_075), 0)
);
vita!(
    vita_dlc2,
    "vita/PCSF00007/dlc2/PSP2/dlc2.psarc",
    pin(20, 23_932, (1_190, 946_457), 3_096, (470, 61_568), 0)
);
vita!(
    vita_patch_data1,
    "vita/PCSF00007/patch-v104/PSP2/data1.psarc",
    pin(7, 10_410, (2_978, 555_137), 6_090, (8, 270), 0)
);
vita!(
    vita_patch_data2,
    "vita/PCSF00007/patch-v104/PSP2/data2.psarc",
    pin(174, 18_293, (8_083, 843_060), 8_448, (5_688, 564_951), 0)
);

macro_rules! omega {
    ($test:ident, $rel:expr, $expect:expr) => {
        #[test]
        #[ignore = "needs the extracted Omega packages in data/extracted/ps4/omega-eu/"]
        fn $test() {
            let Some(by_dir) = sweep($rel, true) else {
                return;
            };
            check(
                stringify!($test),
                &report(stringify!($test), &by_dir),
                &$expect,
            );
        }
    };
}

omega!(
    omega_data00,
    "ps4/omega-eu/uroot/data00.psarc",
    pin(625, 38_362, (14_673, 2_905_293), 0, (0, 0), 235)
);
omega!(
    omega_data01,
    "ps4/omega-eu/uroot/data01.psarc",
    pin(316, 27_942, (2_741, 1_478_396), 0, (0, 0), 0)
);
omega!(
    omega_data02,
    "ps4/omega-eu/uroot/data02.psarc",
    pin(32, 50_858, (7_361, 6_351_397), 0, (0, 0), 0)
);
omega!(
    omega_data03,
    "ps4/omega-eu/uroot/data03.psarc",
    pin(147, 802, (47, 61_711), 0, (0, 0), 0)
);
omega!(
    omega_data04,
    "ps4/omega-eu/uroot/data04.psarc",
    pin(40, 108_417, (36_645, 4_892_704), 0, (0, 0), 432)
);
omega!(
    omega_patch_data05,
    "ps4/omega-eu-patch/uroot/data05.psarc",
    pin(1, 1, (0, 0), 0, (0, 0), 0)
);
omega!(
    omega_patch_data07,
    "ps4/omega-eu-patch/uroot/data07.psarc",
    pin(1, 18, (0, 0), 0, (0, 0), 0)
);
omega!(
    omega_patch_data08,
    "ps4/omega-eu-patch/uroot/data08.psarc",
    pin(110, 12_708, (5_844, 577_626), 0, (0, 0), 0)
);
omega!(
    omega_patch_data09,
    "ps4/omega-eu-patch/uroot/data09.psarc",
    pin(0, 0, (0, 0), 0, (0, 0), 0)
);
