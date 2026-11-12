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

#[derive(Default, Clone, Debug, PartialEq)]
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
    /// Of those, the texcoords the reader returned differ from a fresh decode of the record's
    /// own diffuse attribute (or the reader read one where the record names none).
    /// Before 2026-10-27 the Vita reader looked the attribute up by stride and this was 56,948
    /// of the base archive's 170,164 submeshes (Empire Climb `track.rcsmodel`: 3,897 landed on
    /// the lightmap coordinate, a vertex colour, a tangent or no attribute).
    disagree: Area,
    /// Resolved submeshes whose `+0x28` word is a relocation site of the CPU section's table.
    relocated: usize,
    /// Of those, the own declaration has no `Uv1` at all.
    own_empty: Area,
    /// Reading `Uv1` through the own declaration, resolved submeshes only: all-zero.
    own_all_zero: Area,
    /// Of the submeshes the reader draws flat (empty or all-zero), those the own pointer
    /// resolves and reads a non-flat coordinate from.
    flat_fixed: Area,
    /// Resolved submeshes whose own diffuse attribute is a float pair (`2x9`), decoded as two
    /// `f32`s.
    own_float: Area,
    /// Resolved submeshes whose diffuse attribute is found by the confidence-70 rule (no
    /// `Uv1` named) rather than by name.
    own_fallback: Area,
    /// Resolved submeshes whose own diffuse attribute is of a type the reader does not decode
    /// (it draws nothing for them).
    own_undecoded: Area,
    /// Resolved submeshes whose declaration names `uv1_uvDirt` and no `Uv1`: the road families
    /// (`tarmac_*`) of Omega's `environments2048` circuits. Their diffuse coordinate is
    /// `uv1_uvDirt`'s first pair, not the first half pair in file order (`uvMarkings`).
    uv_dirt: Area,
    /// Of those, the ones whose coordinates the reader returned that differ from a fresh decode
    /// of `uv1_uvDirt`'s first two halves at the attribute's own offset, found by hash here.
    uv_dirt_differs: usize,
    /// Of those, the ones whose material is a `tarmac_*` family.
    uv_dirt_tarmac: usize,
    /// Per `tarmac_*` material name, the box (`[min u, min v]`, `[max u, max v]`) of the
    /// coordinates the reader returned, merged over the submeshes that draw it.
    tarmac_box: BTreeMap<String, CoordBox>,
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
    /// Resolved PS4 submeshes whose diffuse coordinate is the lower-case `uv1` as an `f32`
    /// pair, and of those the ones whose material names no diffuse-family sampler (the
    /// `fc02_effects_uscroll_vscroll_colour_emissive_alpha` family: an `EmissiveMap` only).
    uv1_float: (usize, usize),
    /// Sum over the `own_float` submeshes of their coordinate box (`min u + max u + min v + max v`):
    /// independent of vertex order, so a re-cooked sibling title's model can be compared on it.
    own_float_box: i64,
    /// The `own_empty_named_diffuse` submeshes by material path and declaration (stride, then
    /// each attribute as `hash:components x type @offset`), named in the failure message.
    named_no_uv: BTreeMap<String, Area>,
}

type CoordBox = ([f32; 2], [f32; 2]);

fn merge_box(a: Option<CoordBox>, b: Option<CoordBox>) -> Option<CoordBox> {
    match (a, b) {
        (Some((a, b)), Some((c, d))) => Some((
            [a[0].min(c[0]), a[1].min(c[1])],
            [b[0].max(d[0]), b[1].max(d[1])],
        )),
        (a, b) => a.or(b),
    }
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
        self.own_fallback.plus(o.own_fallback);
        self.own_undecoded.plus(o.own_undecoded);
        self.uv_dirt.plus(o.uv_dirt);
        self.uv_dirt_differs += o.uv_dirt_differs;
        self.uv_dirt_tarmac += o.uv_dirt_tarmac;
        for (k, b) in &o.tarmac_box {
            let merged = merge_box(self.tarmac_box.get(k).copied(), Some(*b));
            self.tarmac_box.extend(merged.map(|m| (k.clone(), m)));
        }
        self.relocated += o.relocated;
        self.lm_both += o.lm_both;
        self.lm_neither += o.lm_neither;
        self.lm_material_only += o.lm_material_only;
        self.lm_decl_only += o.lm_decl_only;
        self.own_empty_named_diffuse += o.own_empty_named_diffuse;
        self.own_uv_no_named_diffuse += o.own_uv_no_named_diffuse;
        self.unlocated += o.unlocated;
        self.own_float_box += o.own_float_box;
        self.uv1_float.0 += o.uv1_float.0;
        self.uv1_float.1 += o.uv1_float.1;
        for (k, a) in &o.named_no_uv {
            self.named_no_uv.entry(k.clone()).or_default().plus(*a);
        }
    }
    fn flat(&self) -> Area {
        Area {
            submeshes: self.empty.submeshes + self.all_zero.submeshes,
            triangles: self.empty.triangles + self.all_zero.triangles,
        }
    }
}

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
        let float = if ps4 {
            vertex_decl::PS4_FLOAT2
        } else {
            vertex_decl::FLOAT2
        };
        let own_attr = own.own_diffuse_texcoord(half, float).copied();
        if own_attr.is_some_and(|a| a.gxm_type == float) {
            out.own_float.add(tris);
            let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
            for uv in &s.texcoords {
                for k in 0..2 {
                    lo[k] = lo[k].min(uv[k]);
                    hi[k] = hi[k].max(uv[k]);
                }
            }
            if !s.texcoords.is_empty() {
                out.own_float_box +=
                    (f64::from(lo[0] + hi[0] + lo[1] + hi[1]) * 100.0).round() as i64;
            }
        }
        if own.diffuse_texcoord_is_fallback(half, float) {
            out.own_fallback.add(tris);
        }
        if own_attr.is_some_and(|a| a.gxm_type != half && a.gxm_type != float) {
            out.own_undecoded.add(tris);
        }
        if ps4
            && own_attr.is_some_and(|a| {
                a.name_hash == vertex_decl::LOWERCASE_UV1_HASH && a.gxm_type == float
            })
        {
            out.uv1_float.0 += 1;
            out.uv1_float.1 += usize::from(!named_diffuse);
        }
        match (own_attr.is_some(), named_diffuse) {
            (false, true) => {
                out.own_empty_named_diffuse += 1;
                let material = s.material.map_or("", |m| model.materials[m].name.as_str());
                let attributes: Vec<String> = own
                    .attributes
                    .iter()
                    .map(|a| {
                        format!(
                            "{:08x}:{}x{}@{}",
                            a.name_hash, a.components, a.gxm_type, a.offset
                        )
                    })
                    .collect();
                out.named_no_uv
                    .entry(format!(
                        "{material} stride {} [{}]",
                        own.stride,
                        attributes.join(" ")
                    ))
                    .or_default()
                    .add(tris);
            }
            (true, false) => out.own_uv_no_named_diffuse += 1,
            _ => {}
        }
        let own_off = own_attr.map(|a| a.offset);
        let width = if own_attr.is_some_and(|a| a.gxm_type == float) {
            8
        } else {
            4
        };
        let off = own_off.map(usize::from).filter(|&o| s.stride >= o + width);
        let decoded = ps4 || own_attr.is_some_and(|a| a.gxm_type == half || a.gxm_type == float);
        let Some(off) = off.filter(|_| decoded) else {
            out.own_empty.add(tris);
            if !s.texcoords.is_empty() {
                out.disagree.add(tris);
            }
            continue;
        };
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
        let mut differs = s.texcoords.len() != s.positions.len();
        for v in 0..s.positions.len() {
            let a = base + v * s.stride + off;
            let uv = if width == 8 {
                let f = |k: usize| f32::from_le_bytes(blob[a + k..a + k + 4].try_into().unwrap());
                [f(0), f(4)]
            } else {
                psp2::unpack_texcoord(blob[a..a + 4].try_into().unwrap())
            };
            let expect = if uv[0].is_finite() && uv[1].is_finite() {
                uv
            } else {
                [0.0, 0.0]
            };
            differs |= s.texcoords.get(v) != Some(&expect);
            if uv[0].is_finite() && uv[1].is_finite() {
                if !zero(&uv) {
                    all_zero = false;
                    finite_nonzero = true;
                }
            } else {
                // A non-finite value becomes the origin in the reader.
            }
        }
        if differs {
            out.disagree.add(tris);
        }
        if let Some(tiling) = own
            .attribute(vertex_decl::UV1_UVDIRT_HASH)
            .filter(|a| own.diffuse_texcoord().is_none() && a.components == 4 && a.gxm_type == half)
        {
            let mut uvs = Vec::with_capacity(s.positions.len());
            for v in 0..s.positions.len() {
                let a = base + v * s.stride + usize::from(tiling.offset);
                uvs.push(psp2::unpack_texcoord(blob[a..a + 4].try_into().unwrap()));
            }
            out.uv_dirt.add(tris);
            out.uv_dirt_differs += usize::from(uvs != s.texcoords);
            let material = s
                .material
                .and_then(|m| model.materials[m].name.rsplit('/').next())
                .map(|n| n.strip_suffix(".rcsmaterial").unwrap_or(n))
                .filter(|n| n.starts_with("tarmac_"));
            if let Some(material) = material {
                out.uv_dirt_tarmac += 1;
                let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
                for uv in &s.texcoords {
                    for k in 0..2 {
                        lo[k] = lo[k].min(uv[k]);
                        hi[k] = hi[k].max(uv[k]);
                    }
                }
                let merged = merge_box(out.tarmac_box.get(material).copied(), Some((lo, hi)));
                out.tarmac_box
                    .extend(merged.map(|m| (material.to_string(), m)));
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
        "sub {:>6} tri {:>8} | empty {:>5}/{:>7} zero {:>5}/{:>7} partly {:>5}/{:>7} nonfin {:>5}/{:>7} | resolved {:>6} disagree {:>5}/{:>7} own_empty {:>5} own_zero {:>5}/{:>7} fixed {:>5}/{:>7} float {:>5}/{:>7} lm {}/{}/{}/{} nodiffuse_uv {}/{} unloc {} fallback {}/{} undecoded {}/{}",
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
        c.own_fallback.submeshes,
        c.own_fallback.triangles,
        c.own_undecoded.submeshes,
        c.own_undecoded.triangles,
    )
}

fn report(name: &str, by_dir: &BTreeMap<String, Census>) -> Census {
    let mut total = Census::default();
    let sweep = std::env::var_os("OAG_SWEEP").is_some();
    for (dir, c) in by_dir {
        total.plus(c);
        if sweep && (c.flat().triangles > 200 || dir.contains("nvironments")) {
            println!(
                "{name} {dir}: {} box {} (hundredths)",
                line(c),
                c.own_float_box
            );
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
    /// Lower-case `uv1` `f32`-pair diffuse coordinates, and those on a material with no
    /// diffuse-family sampler. Omega only.
    uv1_float: (usize, usize),
    /// Submeshes whose diffuse coordinate is `uv1_uvDirt`'s tiling pair, and of those the `tarmac_*`
    /// ones. Omega's `environments2048` circuits only; zero on every Vita archive.
    uv_dirt: (usize, usize),
    /// Directories (full key) whose `own_float_box` is pinned: identical on 2048 and Omega for
    /// `tower`, `park`, `sol` and `square`, whose float-pair submesh sets are the same.
    boxes: &'static [(&'static str, i64)],
}

/// `own_float_box` of the circuits whose float-pair coordinate sets are identical in 2048 and
/// Omega (same submesh and triangle counts): Omega's `f32` `uv1` pair is 2048's float pair.
const VITA_BOXES: [(&str, i64); 4] = [
    ("environments/tower", -409_402),
    ("environments/park", -79_947),
    ("environments/sol", 9_848),
    ("environments/square", -372_134),
];
const OMEGA_DATA00_BOXES: [(&str, i64); 1] = [("Data/environments2048/tower", -409_402)];
const OMEGA_DATA04_BOXES: [(&str, i64); 3] = [
    ("Data/environments2048/park", -79_947),
    ("Data/environments2048/sol", 9_848),
    ("Data/environments2048/square", -372_134),
];

fn check(name: &str, by_dir: &BTreeMap<String, Census>, total: &Census, e: &Expect) {
    for (key, want) in e.boxes {
        assert_eq!(by_dir[*key].own_float_box, *want, "{name}: {key} box");
    }
    let got = Expect {
        files: total.files,
        submeshes: total.all.submeshes,
        flat: (total.flat().submeshes, total.flat().triangles),
        resolved: total.resolved.submeshes,
        disagree: total.disagree.submeshes,
        flat_fixed: (total.flat_fixed.submeshes, total.flat_fixed.triangles),
        empty_named_diffuse: total.own_empty_named_diffuse,
        uv1_float: total.uv1_float,
        uv_dirt: (total.uv_dirt.submeshes, total.uv_dirt_tarmac),
        boxes: &[],
    };
    if std::env::var_os("OAG_SWEEP").is_some() {
        println!(
            "{name} PIN files {} sub {} flat {:?} resolved {} disagree {} fixed {:?} uv1 {:?}",
            got.files,
            got.submeshes,
            got.flat,
            got.resolved,
            got.disagree,
            got.flat_fixed,
            got.uv1_float
        );
        println!("{name} UVDIRT {:?}", got.uv_dirt);
        for (k, c) in by_dir {
            for (m, b) in &c.tarmac_box {
                println!("{name} TARMAC {k} {m}: {b:?}");
            }
        }
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
            got.uv1_float,
            got.uv_dirt,
        ),
        (
            e.files,
            e.submeshes,
            e.flat,
            e.resolved,
            e.disagree,
            e.flat_fixed,
            e.empty_named_diffuse,
            e.uv1_float,
            e.uv_dirt,
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
    // A `uv1_uvDirt` road reads that attribute's first pair and no other set. The pair tiles the
    // road, u -54 to 57 over Rockway Stadium's road material, where `uvMarkings` (the set the
    // file-order rule used to take) spans u 0.79 to 0.99: a regression to it fails the width.
    assert_eq!(
        total.uv_dirt_differs, 0,
        "{name}: tiling pair read as declared"
    );
    for (key, c) in by_dir {
        let road = c
            .tarmac_box
            .get("tarmac_markings_dirt_markingsnormal_emis")
            .filter(|_| key.ends_with("environments2048/arena"));
        if let Some((lo, hi)) = road {
            assert!(
                hi[0] - lo[0] > 80.0,
                "{name}: {key} road box {lo:?} {hi:?} is not the tiling pair"
            );
        }
    }
    // A diffuse-family sampler always has a coordinate to sample with: no submesh names one
    // without the other.
    assert_eq!(
        total.named_no_uv,
        BTreeMap::new(),
        "{name}: diffuse sampler, no diffuse coordinate"
    );
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
                &by_dir,
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
    // Before the reader took the record's own declaration (2026-10-27): empty 685/59,631,
    // all-zero 78/1,449, partly-zero 1,760/209,561, non-finite 1,456/184,902, and 3,897
    // submeshes (353,130 triangles) reading a lightmap, vertex-colour, tangent or no attribute.
    assert_eq!(track.empty, area(22, 432));
    assert_eq!(track.all_zero, area(0, 0));
    assert_eq!(track.partly_zero, area(405, 37_903));
    assert_eq!(track.non_finite, area(0, 0));
    assert_eq!(track.disagree, area(0, 0));
    assert_eq!(track.own_empty.submeshes, 22);
    assert_eq!(track.own_float, area(139, 25_125));
    // The confidence-70 rule (no `Uv1` named) answers for nearly all of them.
    assert_eq!(track.own_fallback, area(4_761, 443_593));
    assert_eq!(track.flat_fixed, area(0, 0));
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

const BASE_ROWS: &[Row] = &[
    (
        "environments/altima",
        (2979, 434182),
        (0, 0),
        (0, 0),
        (72, 8161),
    ),
    (
        "environments/arena",
        (3849, 399934),
        (2, 92),
        (0, 0),
        (139, 9255),
    ),
    (
        "environments/bridge",
        (5974, 600577),
        (20, 1132),
        (0, 0),
        (192, 13828),
    ),
    (
        "environments/cathedral",
        (4995, 449440),
        (33, 888),
        (0, 0),
        (222, 17646),
    ),
    (
        "environments/mall",
        (7505, 506303),
        (15, 256),
        (0, 0),
        (354, 24243),
    ),
    (
        "environments/park",
        (5007, 381069),
        (4, 77),
        (0, 0),
        (242, 24268),
    ),
    (
        "environments/sol",
        (2846, 272600),
        (27, 320),
        (0, 0),
        (74, 5544),
    ),
    (
        "environments/square",
        (5886, 510592),
        (13, 324),
        (0, 0),
        (387, 19059),
    ),
    (
        "environments/subway",
        (7403, 663407),
        (8, 174),
        (0, 0),
        (533, 31554),
    ),
    (
        "environments/tower",
        (5126, 486724),
        (22, 432),
        (0, 0),
        (405, 37903),
    ),
];
const PATCH_ROWS: &[Row] = &[
    (
        "environments/arena [zone]",
        (6797, 617816),
        (2841, 287036),
        (0, 0),
        (11, 1293),
    ),
    (
        "environments/bridge",
        (5753, 580679),
        (20, 1132),
        (0, 0),
        (192, 13828),
    ),
    (
        "environments/park",
        (4829, 365249),
        (4, 77),
        (0, 0),
        (242, 24268),
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
        uv1_float: (0, 0),
        uv_dirt: (0, 0),
        boxes: &[],
    }
}

impl Expect {
    const fn with_boxes(mut self, boxes: &'static [(&'static str, i64)]) -> Self {
        self.boxes = boxes;
        self
    }

    const fn with_uv_dirt(mut self, all: usize, tarmac: usize) -> Self {
        self.uv_dirt = (all, tarmac);
        self
    }

    const fn with_uv1_float(mut self, all: usize, without_diffuse_sampler: usize) -> Self {
        self.uv1_float = (all, without_diffuse_sampler);
        self
    }
}

vita!(
    vita_base,
    "vita/PCSF00007/base/PSP2/data.psarc",
    pin(953, 170_164, (46_569, 4_655_125), 0, (0, 0), 0).with_boxes(&VITA_BOXES)
);
vita!(
    vita_dlc1,
    "vita/PCSF00007/dlc1/PSP2/dlc1.psarc",
    pin(20, 53_440, (515, 44_926), 0, (0, 0), 0)
);
vita!(
    vita_dlc2,
    "vita/PCSF00007/dlc2/PSP2/dlc2.psarc",
    pin(20, 23_932, (845, 891_166), 0, (0, 0), 0)
);
vita!(
    vita_patch_data1,
    "vita/PCSF00007/patch-v104/PSP2/data1.psarc",
    pin(7, 10_410, (6_790, 709_683), 0, (0, 0), 0)
);
vita!(
    vita_patch_data2,
    "vita/PCSF00007/patch-v104/PSP2/data2.psarc",
    pin(174, 18_293, (2_897, 303_563), 0, (0, 0), 0)
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
                &by_dir,
                &report(stringify!($test), &by_dir),
                &$expect,
            );
        }
    };
}

omega!(
    omega_data00,
    "ps4/omega-eu/uroot/data00.psarc",
    pin(625, 38_362, (14_438, 2_849_186), 0, (0, 0), 0)
        .with_uv1_float(235, 0)
        .with_uv_dirt(122, 122)
        .with_boxes(&OMEGA_DATA00_BOXES)
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
    pin(40, 108_417, (36_177, 4_678_390), 0, (0, 0), 0)
        .with_uv1_float(467, 35)
        .with_uv_dirt(539, 539)
        .with_boxes(&OMEGA_DATA04_BOXES)
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
