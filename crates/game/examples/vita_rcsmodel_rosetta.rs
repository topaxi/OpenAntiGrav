//! Scratch probe, in two parts, for Wipeout 2048's unrecovered `.rcsmodel`
//! vertex fields (normal, lightmap texture coordinate).
//!
//! **Part one - now dead evidence, kept because the negative is real**: pair
//! a DLC circuit against Wipeout HD's own copy of it (the technique
//! `vita_rosetta.rs` used for the `WO Track` tail and `vita_surface.rs` used
//! for the collision surface byte - both titles' circuits sit in the same
//! world space, so a 2048 vertex and the HD vertex nearest it in space are
//! very likely the same authored vertex, and HD's already-decoded normal is
//! ground truth for 2048's undecoded one at the same location), then sweep
//! `unpack_normal` (HD's packed 11:11:10 word) over every word-aligned offset
//! of every stride. Every offset comes back at random-chance level - see the
//! "normal candidate" report below - which is a real finding, but not the one
//! it looks like: it says HD's *encoding* is not at any of these offsets, not
//! that the field is misplaced.
//!
//! **Part two - the real find**: section B (the object graph `psp2`'s reading
//! never walks) carries HD's own `~crc32` attribute-name hashes
//! (`position`/`normal`/`tangent`/`lightmapUV`), little-endian, in a
//! declaration structurally identical to HD's own
//! (`crates/formats/src/rcsmodel/vertex_decl.rs`): a 4-byte header then
//! 8-byte attribute records. **One field inside a record keeps HD's
//! big-endian byte order even though `name_hash` is little-endian** - the
//! record's own repeated stride at `+0x04..+0x06` only cross-checks against
//! the header's stride read big-endian, never little-endian. That is
//! consistent with a serializer that writes scalar fields through explicit
//! byte shifts (endian-independent) while `name_hash` is a straight
//! native-endian word copy that the port never adjusted.
//!
//! This decodes real, self-consistent offsets - `position@0`, `normal@12`,
//! `tangent@16`, `Uv1@20`, `lightmapUV@24` on the common 28-byte layout,
//! cross-checked the same way HD's own declaration is: every record's
//! restated stride must equal the header's. **What it does not decode is the
//! type nibble's meaning**: 2048 runs on the Vita's SceGxm, not the PS3's
//! RSX, so the numeric type codes are a different enum and HD's are not a
//! safe guess - `normal`'s declared type (5) tested as HD's packed word at
//! the now-*correct* offset still comes back at chance (see "declared normal
//! offset" below), and the byte budget between `normal@12` and `tangent@16`
//! is only 4 bytes for 3 declared components, which rules out at least one
//! plausible width per component. That is where this probe stops: the type
//! codes need either a verified SceGxm attribute-format reference or Ghidra
//! RE of the vertex-stream setup path, neither of which this probe attempts.
//!
//! `find_declarations` anchors on the `position` hash and assumes `position`
//! is always record 0 (`header_at = position_at - 4`) - true on every
//! declaration hand-checked, not independently verified against the full
//! corpus of chunks per circuit, so a declaration where `position` is not
//! first would be silently missed rather than misdecoded.
//!
//! ```sh
//! cargo run -q -p oag-game --example vita_rcsmodel_rosetta
//! ```

use std::collections::HashMap;

use oag_formats::rcsmodel::{self, psp2};

const HD_DIR: &str = "data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR";
const VITA_DLC: [&str; 2] = [
    "data/extracted/vita/PCSF00007/dlc1/PSP2/dlc1.psarc",
    "data/extracted/vita/PCSF00007/dlc2/PSP2/dlc2.psarc",
];

/// `(HD archive, HD environment, 2048 environment)`, the same pairing
/// `vita_surface.rs` uses.
const PAIRS: &[(&str, &str, &str)] = &[
    ("DATA02", "01_vineta_k", "Vineta_K"),
    ("DATA02", "05_ubermall", "Ubermall"),
    ("DATA02", "10_sebenco_climb", "Sebenco_Climb"),
    ("DATA02", "12_sol_2", "Sol_2"),
    ("DATA00", "amphiseum", "amphiseum"),
    ("DATA00", "modesto_heights", "modesto_heights"),
    ("DATA00", "talons_junction", "talons_junction"),
    ("DATA00", "tech_de_ra", "tech_de_ra"),
    ("DATA00", "zone_1", "zone_1"),
    ("DATA00", "zone_2", "zone_2"),
    ("DATA00", "zone_3", "zone_3"),
    ("DATA00", "zone_4", "zone_4"),
];

/// A cell this many units across - vertices at authored precision land in the
/// same or an adjacent cell.
const CELL: f32 = 0.25;
/// How close two positions have to be to call them the same authored vertex.
const MATCH_DIST2: f32 = 0.01; // 0.1 units

fn cell(p: [f32; 3]) -> (i32, i32, i32) {
    (
        (p[0] / CELL).floor() as i32,
        (p[1] / CELL).floor() as i32,
        (p[2] / CELL).floor() as i32,
    )
}

/// One HD vertex worth carrying into the match: its position and its normal.
/// A lightmap coordinate was carried here too until the texcoord test became
/// a plausibility check rather than a value match against HD - see the module
/// doc's "not a value-match test" note.
#[derive(Clone, Copy)]
struct HdVertex {
    pos: [f32; 3],
    normal: [f32; 3],
}

fn hd_vertices(archive: &str, environment: &str) -> anyhow::Result<Vec<HdVertex>> {
    let mut hd = oag_assets::psarc::Archive::open(&format!("{HD_DIR}/{archive}.PSARC"))?;
    let blob = hd.read_path(&format!("/data/environments/{environment}/track.rcsmodel"))?;
    let model = rcsmodel::Model::parse(&blob)?;
    let mut out = Vec::new();
    for mesh in &model.meshes {
        let Some(stride) = mesh.declared_stride() else {
            continue;
        };
        for submesh in &mesh.submeshes {
            let Ok(positions) = mesh.positions(&blob, submesh, stride) else {
                continue;
            };
            let Ok(normals) = mesh.normals(&blob, submesh, stride) else {
                continue;
            };
            for (pos, normal) in positions.into_iter().zip(normals) {
                out.push(HdVertex { pos, normal });
            }
        }
    }
    Ok(out)
}

fn vita_blob(name: &str, suffix: &str) -> Option<Vec<u8>> {
    for psarc in VITA_DLC {
        let Ok(mut archive) = oag_assets::psarc::Archive::open(psarc) else {
            continue;
        };
        let entry = format!("data/art/published/DLC1/environments/{name}/{suffix}");
        if let Ok(blob) = archive.read_path(&entry) {
            return Some(blob);
        }
    }
    None
}

/// `position`'s `~crc32` - HD's, from `crates/formats/src/rcsmodel/vertex_decl.rs`.
/// Used as the anchor to find a declaration: every one seen so far opens with
/// it, so a hit at `p` puts the 4-byte header at `p - HEADER_LEN`.
const POSITION_HASH: u32 = 0xb9d3_1b0a;
const HEADER_LEN: usize = 4;
const ATTRIBUTE_LEN: usize = 8;
const MAX_ATTRIBUTES: usize = 16;

/// One little-endian-decoded attribute record.
#[derive(Clone, Copy)]
struct Attr {
    name_hash: u32,
    components: u8,
    rsx_type: u8,
    offset: u8,
}

fn attr_name(hash: u32) -> Option<&'static str> {
    Some(match hash {
        0xb9d3_1b0a => "position",
        0xde7a_971b => "normal",
        0xdbe5_f417 => "tangent",
        0x26a7_b665 => "lightmapUV",
        0x4272_14fc => "Uv1",
        0xdb7b_4546 => "Uv2",
        0x7a3f_521c => "uv1",
        0x49f7_6806 => "Uvset1",
        0x2003_d7e6 => "map1",
        0xb90a_865c => "map2",
        0xce5c_d9d9 => "colorSet1",
        0x7493_d450 => "VertexColour1",
        _ => return None,
    })
}

/// One decoded declaration: the header's own stride, and its attributes.
struct Decl {
    stride: usize,
    attributes: Vec<Attr>,
}

/// Little-endian mirror of `VertexDecl::parse` - HD's is big-endian, this
/// tries the opposite byte order at every `position`-hash anchor found in
/// section B, with the same cross-check HD's reading uses: every record's own
/// repeated stride field must equal the header's, which a wrong anchor or a
/// wrong endianness essentially never passes by chance.
fn decode_declaration(bytes: &[u8], position_at: usize) -> Option<Decl> {
    let header_at = position_at.checked_sub(HEADER_LEN)?;
    let count = usize::from(*bytes.get(header_at)?);
    let stride = usize::from(*bytes.get(header_at + 1)?);
    if count == 0 || count > MAX_ATTRIBUTES || stride == 0 {
        return None;
    }
    if header_at + HEADER_LEN + count * ATTRIBUTE_LEN > bytes.len() {
        return None;
    }
    let mut attributes = Vec::with_capacity(count);
    for k in 0..count {
        let base = header_at + HEADER_LEN + k * ATTRIBUTE_LEN;
        // Big-endian, unlike name_hash: bytes[base+4..+6] read "00 1c" for a
        // stride of 28 on every record checked by hand, which is only right
        // read big-endian. Consistent with a serializer that writes this
        // scalar field through explicit byte shifts (host-endian-independent)
        // while name_hash is a straight native-endian word copy.
        let record_stride = u16::from_be_bytes([bytes[base + 4], bytes[base + 5]]) as usize;
        if record_stride != stride {
            return None;
        }
        let ty = bytes[base + 6];
        attributes.push(Attr {
            name_hash: u32::from_le_bytes(bytes[base..base + 4].try_into().unwrap()),
            components: ty >> 4,
            rsx_type: ty & 0xf,
            offset: bytes[base + 7],
        });
    }
    Some(Decl { stride, attributes })
}

/// Every declaration section B carries, found by anchoring on every
/// `position`-hash occurrence and decoding outward. Prints what it finds.
fn find_declarations(blob: &[u8], section: psp2::Section, name: &str) -> Vec<Decl> {
    let bytes = &blob[section.at..section.at + section.len];
    let anchor = POSITION_HASH.to_le_bytes();
    let mut out = Vec::new();
    for (i, _) in bytes.windows(4).enumerate().filter(|(_, w)| *w == anchor) {
        if let Some(decl) = decode_declaration(bytes, i) {
            out.push(decl);
        }
    }
    if out.is_empty() {
        println!("  {name}: no declaration decodes at any position-hash anchor");
    } else if let Some(first) = out.first() {
        let fields: Vec<String> = first
            .attributes
            .iter()
            .map(|a| {
                format!(
                    "{}@{}(c{} t{})",
                    attr_name(a.name_hash).unwrap_or("?"),
                    a.offset,
                    a.components,
                    a.rsx_type
                )
            })
            .collect();
        println!(
            "  {name}: {} declaration(s) decoded, e.g. stride {} - {}",
            out.len(),
            first.stride,
            fields.join(", ")
        );
    }
    out
}

fn f16_pair(b: &[u8], at: usize) -> [f32; 2] {
    let word = |o: usize| u16::from_le_bytes([b[at + o], b[at + o + 1]]);
    [
        rcsmodel::unpack_half(word(0)),
        rcsmodel::unpack_half(word(2)),
    ]
}

/// Bucketed dot-product agreement, the same shape HD's own normal recovery
/// reported it in.
#[derive(Default)]
struct DotStats {
    n: usize,
    within_18deg: usize, // dot > 0.951
    within_45deg: usize, // dot > 0.707
    sum_abs_dot: f64,
}

impl DotStats {
    fn add(&mut self, dot: f32) {
        self.n += 1;
        if dot > 0.951 {
            self.within_18deg += 1;
        }
        if dot > 0.707 {
            self.within_45deg += 1;
        }
        self.sum_abs_dot += f64::from(dot.abs());
    }

    fn report(&self, label: &str) {
        if self.n == 0 {
            println!("  {label}: no matched vertices");
            return;
        }
        println!(
            "  {label}: n={} within 18deg {:.1}% within 45deg {:.1}% mean|dot| {:.3}",
            self.n,
            100.0 * self.within_18deg as f64 / self.n as f64,
            100.0 * self.within_45deg as f64 / self.n as f64,
            self.sum_abs_dot / self.n as f64,
        );
    }
}

fn main() -> anyhow::Result<()> {
    // Every word-aligned offset in the vertex, grouped by stride - not a
    // handful of guesses. Keyed `(stride, offset)`.
    let mut normal_stats: HashMap<(usize, usize), DotStats> = HashMap::new();
    let mut lightmap_last4_total = 0usize;
    let mut lightmap_last4_plausible = 0usize;
    let mut lightmap_last4_nan = 0usize;
    let mut ambiguous = 0usize;
    let mut address_mismatches = 0usize;
    let mut total_checked = 0usize;
    let mut declared_normal = DotStats::default();
    let mut declared_lightmap_total = 0usize;
    let mut declared_lightmap_plausible = 0usize;
    let mut declared_lightmap_nan = 0usize;

    for &(archive, hd_env, name) in PAIRS {
        let hd = match hd_vertices(archive, hd_env) {
            Ok(v) => v,
            Err(e) => {
                println!("{name}: HD .rcsmodel unavailable: {e}");
                continue;
            }
        };
        let Some(blob) = vita_blob(name, "track.rcsmodel") else {
            println!("{name}: no 2048 .rcsmodel");
            continue;
        };
        let model = match psp2::parse(&blob) {
            Ok(m) => m,
            Err(e) => {
                println!("{name}: 2048 .rcsmodel does not parse: {e}");
                continue;
            }
        };
        let Some(&gpu) = model.sections.get(1) else {
            println!("{name}: 2048 .rcsmodel has no GPU section");
            continue;
        };
        let cpu = model.sections[0];
        let decls = find_declarations(&blob, cpu, name);
        let mut normal_offset_by_stride: HashMap<usize, u8> = HashMap::new();
        let mut lightmap_offset_by_stride: HashMap<usize, u8> = HashMap::new();
        for d in &decls {
            for a in &d.attributes {
                match attr_name(a.name_hash) {
                    Some("normal") => {
                        normal_offset_by_stride.entry(d.stride).or_insert(a.offset);
                    }
                    Some("lightmapUV") => {
                        lightmap_offset_by_stride
                            .entry(d.stride)
                            .or_insert(a.offset);
                    }
                    _ => {}
                }
            }
        }

        let mut grid: HashMap<(i32, i32, i32), Vec<usize>> = HashMap::new();
        for (i, v) in hd.iter().enumerate() {
            grid.entry(cell(v.pos)).or_default().push(i);
        }
        // Unique match only: if more than one HD vertex sits within the
        // threshold, the correspondence is ambiguous (a hard-edge split) and
        // is dropped rather than guessed, so a wrong pick cannot dilute the
        // signal this probe is trying to read.
        let mut nearest_unique = |p: [f32; 3]| -> Option<usize> {
            let (cx, cy, cz) = cell(p);
            let mut within: Vec<usize> = Vec::new();
            for dx in -1..=1 {
                for dy in -1..=1 {
                    for dz in -1..=1 {
                        for &i in grid.get(&(cx + dx, cy + dy, cz + dz)).into_iter().flatten() {
                            let d: f32 = (0..3).map(|a| (hd[i].pos[a] - p[a]).powi(2)).sum();
                            if d < MATCH_DIST2 {
                                within.push(i);
                            }
                        }
                    }
                }
            }
            match within.len() {
                0 => None,
                1 => Some(within[0]),
                _ => {
                    ambiguous += 1;
                    None
                }
            }
        };

        let mut matched_this_track = 0usize;
        for submesh in &model.submeshes {
            let base = cpu.at + submesh.record;
            let vertex_ptr =
                u32::from_le_bytes(blob[base + psp2::VERTEX_POINTER..][..4].try_into().unwrap());
            let vertex_at = gpu.at + vertex_ptr as usize;
            let stride = submesh.stride;
            for (v, &pos) in submesh.positions.iter().enumerate() {
                let Some(hi) = nearest_unique(pos) else {
                    continue;
                };
                matched_this_track += 1;
                let at = vertex_at + v * stride;

                // Sanity check: re-derive the position from raw bytes at `at`
                // and confirm it is the same one `psp2::parse` already
                // decoded, so a mismatch here means the address arithmetic
                // above is wrong rather than that the field hypotheses are.
                let reread: [f32; 3] = std::array::from_fn(|i| {
                    f32::from_le_bytes(blob[at + i * 4..][..4].try_into().unwrap())
                });
                total_checked += 1;
                if reread != pos {
                    address_mismatches += 1;
                }

                let mut off = 0usize;
                while off + 4 <= stride {
                    if at + off + 4 <= blob.len() {
                        let word = u32::from_le_bytes(blob[at + off..][..4].try_into().unwrap());
                        let candidate = rcsmodel::unpack_normal(word);
                        let dot: f32 = (0..3).map(|a| candidate[a] * hd[hi].normal[a]).sum();
                        normal_stats.entry((stride, off)).or_default().add(dot);
                    }
                    off += 4;
                }

                if let Some(&normal_off) = normal_offset_by_stride.get(&stride) {
                    let off = usize::from(normal_off);
                    if at + off + 4 <= blob.len() {
                        let word = u32::from_le_bytes(blob[at + off..][..4].try_into().unwrap());
                        let candidate = rcsmodel::unpack_normal(word);
                        let dot: f32 = (0..3).map(|a| candidate[a] * hd[hi].normal[a]).sum();
                        declared_normal.add(dot);
                    }
                }

                // NOT a value-match test: a lightmap atlas is baked per
                // platform, so 2048's lightmapUV has no reason to equal HD's
                // numerically even at the same authored vertex - only the
                // encoding (is it a half pair in a texcoord-shaped range at
                // all) is title-invariant. `PLAUSIBLE` mirrors
                // `rcsmodel_vertex_ground_truth.rs`'s own bar for this.
                if let Some(&lm_off) = lightmap_offset_by_stride.get(&stride) {
                    let off = usize::from(lm_off);
                    if at + off + 4 <= blob.len() {
                        let candidate = f16_pair(&blob, at + off);
                        declared_lightmap_total += 1;
                        const PLAUSIBLE: f32 = 8.0;
                        if candidate
                            .iter()
                            .all(|c| c.is_finite() && c.abs() <= PLAUSIBLE)
                        {
                            declared_lightmap_plausible += 1;
                        } else {
                            declared_lightmap_nan += 1;
                        }
                    }
                }

                // Same plausibility bar, against the naive pre-declaration
                // heuristic (last four bytes), for a direct comparison.
                if at + stride <= blob.len() {
                    const PLAUSIBLE: f32 = 8.0;
                    let last4 = f16_pair(&blob, at + stride - 4);
                    lightmap_last4_total += 1;
                    if last4.iter().all(|c| c.is_finite() && c.abs() <= PLAUSIBLE) {
                        lightmap_last4_plausible += 1;
                    } else {
                        lightmap_last4_nan += 1;
                    }
                }
            }
        }
        println!(
            "{name}: {} HD vertices, {} 2048 vertices matched uniquely within {:.2} units",
            hd.len(),
            matched_this_track,
            MATCH_DIST2.sqrt()
        );
    }
    println!("\n{ambiguous} 2048 vertices had more than one HD vertex within threshold, dropped");
    println!(
        "address sanity check: {address_mismatches}/{total_checked} re-read positions disagree \
with psp2::parse's own decode"
    );

    println!("\n=== normal at the DECLARED offset (from section B's own vertex declaration) ===");
    println!(
        "declaration structure itself: self-consistent (stride cross-check passes) on every \
decoded declaration - position@0, normal@12, tangent@16, Uv1@20, lightmapUV@24 on the common \
28-byte layout"
    );
    declared_normal.report("declared normal offset, decoded as HD's packed 11:11:10 word");

    println!(
        "\n=== lightmapUV plausibility at the DECLARED offset vs the naive last-4-bytes guess ==="
    );
    println!(
        "(NOT a value-match test: a lightmap atlas is baked per platform, so 2048's lightmapUV \
has no reason to equal HD's numerically at the same vertex - only whether it decodes to a \
half pair in a texcoord-shaped range at all is title-invariant)"
    );
    if declared_lightmap_total > 0 {
        println!(
            "  declared offset:   n={declared_lightmap_total} plausible {:.1}% ({declared_lightmap_nan} non-finite/implausible)",
            100.0 * declared_lightmap_plausible as f64 / declared_lightmap_total as f64,
        );
    } else {
        println!("  declared offset: no stride in the corpus had a decoded lightmapUV attribute");
    }
    if lightmap_last4_total > 0 {
        println!(
            "  last 4 bytes:      n={lightmap_last4_total} plausible {:.1}% ({lightmap_last4_nan} non-finite/implausible)",
            100.0 * lightmap_last4_plausible as f64 / lightmap_last4_total as f64,
        );
    }

    println!("\n=== normal candidate: unpack_normal(word at +off), best offset per stride ===");
    let mut strides: Vec<usize> = normal_stats.keys().map(|&(s, _)| s).collect();
    strides.sort_unstable();
    strides.dedup();
    for stride in strides {
        let mut offs: Vec<(usize, &DotStats)> = normal_stats
            .iter()
            .filter(|&(&(s, _), _)| s == stride)
            .map(|(&(_, o), stats)| (o, stats))
            .collect();
        offs.sort_by(|a, b| b.1.sum_abs_dot.partial_cmp(&a.1.sum_abs_dot).unwrap());
        println!("  stride {stride}:");
        for (off, stats) in offs.iter().take(3) {
            stats.report(&format!("    +{off:#04x}"));
        }
    }

    Ok(())
}
