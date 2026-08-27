//! Scratch probe: exhaustive candidate search for 2048's `.rcsmodel` `normal`
//! encoding (declaration type `5`, 3 components, `+0x0c`), against the
//! index-exact 1,504-vertex oracle `vita_rcsmodel_exact.rs` built.
//!
//! # Why a per-byte scheme is the principled next guess, not just another one
//!
//! `normal@0x0c` (3 declared components) and `tangent@0x10` (4 declared
//! components) share the same type code (`5`), and `tangent`'s byte budget
//! (`0x10` to `Uv1@0x14`) is exactly 4 bytes for 4 components - one byte
//! each, no padding needed. If type `5` is a fixed **1 byte per component**
//! type, `normal`'s 3 components would take 3 bytes and the 4th byte to
//! `tangent@0x10` is alignment padding, not a fourth used byte - which is a
//! completely different shape of encoding than the packed 32-bit word this
//! session tried and ruled out three times over. This is the natural
//! candidate the byte budget itself argues for.
//!
//! # What this searches
//!
//! - **Per-byte**: every assignment of 3 of the 4 bytes to x/y/z (4 choices
//!   of which byte is padding x 6 orderings = 24), x 2 signedness
//!   conventions (raw two's-complement `i8/127`, and HD's own
//!   unsigned-biased `byte/127.5 - 1`) = 48 candidates.
//! - **Packed word, field-to-axis permuted**: HD's exact 11:11:10 bit split
//!   was only ever tested with HD's own field-to-axis assignment (low 11 =
//!   x, mid 11 = y, top 10 = z) at every *offset*, never at other
//!   assignments of the same three fields - x 6 permutations x 2 byte
//!   orders = 12 more, cheap to rule out alongside the per-byte family.
//!
//! ```sh
//! cargo run -q -p oag-game --example vita_rcsmodel_bytesearch
//! ```

use std::collections::HashMap;

use oag_formats::rcsmodel::{self, psp2};

const HD_DIR: &str = "data/extracted/ps3/hdfury-eu/PS3_GAME/USRDIR";
const VITA_DLC: [&str; 2] = [
    "data/extracted/vita/PCSF00007/dlc1/PSP2/dlc1.psarc",
    "data/extracted/vita/PCSF00007/dlc2/PSP2/dlc2.psarc",
];

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

const MEAN_DIST2_BAR: f32 = 0.0025; // 5 cm

struct HdSubmesh {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
}

fn hd_submeshes(archive: &str, environment: &str) -> anyhow::Result<Vec<HdSubmesh>> {
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
            out.push(HdSubmesh { positions, normals });
        }
    }
    Ok(out)
}

fn vita_blob(name: &str) -> Option<Vec<u8>> {
    for psarc in VITA_DLC {
        let Ok(mut archive) = oag_assets::psarc::Archive::open(psarc) else {
            continue;
        };
        let entry = format!("data/art/published/DLC1/environments/{name}/track.rcsmodel");
        if let Ok(blob) = archive.read_path(&entry) {
            return Some(blob);
        }
    }
    None
}

fn index_agreement(a: &[[f32; 3]], b: &[[f32; 3]]) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    let sum: f32 = a
        .iter()
        .zip(b)
        .map(|(p, q)| (0..3).map(|i| (p[i] - q[i]).powi(2)).sum::<f32>())
        .sum();
    sum / a.len() as f32
}

/// `normal`'s declared offset for one stride - same anchor scan
/// `vita_rcsmodel_rosetta.rs`/`vita_rcsmodel_exact.rs` use.
fn declared_normal_offset(blob: &[u8], section: psp2::Section, stride: usize) -> Option<u8> {
    const POSITION_HASH: u32 = 0xb9d3_1b0a;
    const NORMAL_HASH: u32 = 0xde7a_971b;
    let bytes = &blob[section.at..section.at + section.len];
    let anchor = POSITION_HASH.to_le_bytes();
    for (i, _) in bytes.windows(4).enumerate().filter(|(_, w)| *w == anchor) {
        let header_at = i.checked_sub(4)?;
        let count = usize::from(*bytes.get(header_at)?);
        let decl_stride = usize::from(*bytes.get(header_at + 1)?);
        if decl_stride != stride || count == 0 || count > 16 {
            continue;
        }
        for k in 0..count {
            let base = header_at + 4 + k * 8;
            if base + 8 > bytes.len() {
                break;
            }
            let record_stride = u16::from_be_bytes([bytes[base + 4], bytes[base + 5]]) as usize;
            if record_stride != stride {
                break;
            }
            let name_hash = u32::from_le_bytes(bytes[base..base + 4].try_into().unwrap());
            if name_hash == NORMAL_HASH {
                return Some(bytes[base + 7]);
            }
        }
    }
    None
}

/// One paired vertex: the raw 4 bytes at `normal`'s declared offset, and
/// HD's ground-truth unit normal.
struct Pair {
    raw: [u8; 4],
    hd: [f32; 3],
}

fn collect_pairs() -> anyhow::Result<Vec<Pair>> {
    let mut out = Vec::new();
    for &(archive, hd_env, name) in PAIRS {
        let Ok(hd) = hd_submeshes(archive, hd_env) else {
            continue;
        };
        let Some(blob) = vita_blob(name) else {
            continue;
        };
        let Ok(model) = psp2::parse(&blob) else {
            continue;
        };
        let Some(&gpu) = model.sections.get(1) else {
            continue;
        };
        let cpu = model.sections[0];

        let mut by_count: HashMap<usize, Vec<usize>> = HashMap::new();
        for (i, s) in hd.iter().enumerate() {
            by_count.entry(s.positions.len()).or_default().push(i);
        }

        for submesh in &model.submeshes {
            let Some(candidates) = by_count.get(&submesh.positions.len()) else {
                continue;
            };
            if submesh.positions.len() < 8 {
                continue;
            }
            let mut best: Option<(f32, usize)> = None;
            for &hi in candidates {
                let d = index_agreement(&submesh.positions, &hd[hi].positions);
                if best.is_none_or(|(bd, _)| d < bd) {
                    best = Some((d, hi));
                }
            }
            let Some((d, hi)) = best else { continue };
            if d >= MEAN_DIST2_BAR {
                continue;
            }

            let base = cpu.at + submesh.record;
            let vertex_ptr =
                u32::from_le_bytes(blob[base + psp2::VERTEX_POINTER..][..4].try_into().unwrap());
            let vertex_at = gpu.at + vertex_ptr as usize;
            let stride = submesh.stride;
            let Some(normal_off) = declared_normal_offset(&blob, cpu, stride) else {
                continue;
            };
            let normal_off = usize::from(normal_off);

            for (v, hd_normal) in hd[hi].normals.iter().enumerate() {
                let at = vertex_at + v * stride + normal_off;
                if at + 4 > blob.len() {
                    continue;
                }
                out.push(Pair {
                    raw: blob[at..at + 4].try_into().unwrap(),
                    hd: *hd_normal,
                });
            }
        }
    }
    Ok(out)
}

/// All six orderings of three items.
fn permutations3<T: Copy>(items: [T; 3]) -> [[T; 3]; 6] {
    let [a, b, c] = items;
    [
        [a, b, c],
        [a, c, b],
        [b, a, c],
        [b, c, a],
        [c, a, b],
        [c, b, a],
    ]
}

struct Score {
    label: String,
    n: usize,
    within_18: usize,
    mean_dot: f64,
}

fn score(label: String, pairs: &[Pair], decode: impl Fn(&[u8; 4]) -> [f32; 3]) -> Score {
    let mut sum = 0f64;
    let mut within_18 = 0usize;
    for p in pairs {
        let candidate = decode(&p.raw);
        let dot: f32 = (0..3).map(|a| candidate[a] * p.hd[a]).sum();
        sum += f64::from(dot);
        if dot > 0.951 {
            within_18 += 1;
        }
    }
    Score {
        label,
        n: pairs.len(),
        within_18,
        mean_dot: sum / pairs.len() as f64,
    }
}

fn main() -> anyhow::Result<()> {
    let pairs = collect_pairs()?;
    println!(
        "{} index-exact vertex pairs with a declared normal offset",
        pairs.len()
    );
    if pairs.is_empty() {
        return Ok(());
    }

    let mut scores = Vec::new();

    // Per-byte family: which of the four raw bytes is padding, and which
    // ordering the other three take as (x, y, z), under both signedness
    // conventions.
    for pad in 0..4usize {
        let rest: Vec<usize> = (0..4).filter(|&i| i != pad).collect();
        let rest: [usize; 3] = [rest[0], rest[1], rest[2]];
        for order in permutations3(rest) {
            for signed in [true, false] {
                let label = format!(
                    "byte[{}]=x byte[{}]=y byte[{}]=z ({}), pad=byte[{pad}]",
                    order[0],
                    order[1],
                    order[2],
                    if signed { "i8/127" } else { "u8/127.5-1" }
                );
                let decode = move |raw: &[u8; 4]| -> [f32; 3] {
                    std::array::from_fn(|a| {
                        let b = raw[order[a]];
                        if signed {
                            (b as i8) as f32 / 127.0
                        } else {
                            f32::from(b) / 127.5 - 1.0
                        }
                    })
                };
                scores.push(score(label, &pairs, decode));
            }
        }
    }

    // Packed-word family: HD's 11:11:10 split, every assignment of the three
    // fields to x/y/z, both byte orders.
    let fields = ["low11", "mid11", "high10"];
    for perm in permutations3([0usize, 1, 2]) {
        for be in [false, true] {
            let label = format!(
                "packed 11:11:10, x={} y={} z={} ({})",
                fields[perm[0]],
                fields[perm[1]],
                fields[perm[2]],
                if be { "BE" } else { "LE" }
            );
            let decode = move |raw: &[u8; 4]| -> [f32; 3] {
                let word = if be {
                    u32::from_be_bytes(*raw)
                } else {
                    u32::from_le_bytes(*raw)
                };
                let unpacked = rcsmodel::unpack_normal(word);
                std::array::from_fn(|a| unpacked[perm[a]])
            };
            scores.push(score(label, &pairs, decode));
        }
    }

    // The winning candidate's 4th byte, not used by the decode - is it
    // padding (constant, or noise) or something meaningful?
    let pad: Vec<i8> = pairs.iter().map(|p| p.raw[3] as i8).collect();
    let zero = pad.iter().filter(|&&b| b == 0).count();
    let min = *pad.iter().min().unwrap();
    let max = *pad.iter().max().unwrap();
    println!(
        "byte[3] (unused by the winning decode): {zero}/{} are exactly 0, range {min}..={max}",
        pad.len()
    );
    let mut hist: std::collections::BTreeMap<i8, usize> = Default::default();
    for &b in &pad {
        *hist.entry(b).or_default() += 1;
    }
    let mut hist: Vec<(i8, usize)> = hist.into_iter().collect();
    hist.sort_by_key(|&(_, count)| std::cmp::Reverse(count));
    println!("  most common values: {:?}", &hist[..hist.len().min(8)]);

    scores.sort_by(|a, b| b.mean_dot.partial_cmp(&a.mean_dot).unwrap());
    println!(
        "\n=== top 15 of {} candidates, by mean dot ===",
        scores.len()
    );
    for s in scores.iter().take(15) {
        println!(
            "  mean dot {:+.3}  within 18deg {:5.1}%  n={}  {}",
            s.mean_dot,
            100.0 * s.within_18 as f64 / s.n as f64,
            s.n,
            s.label
        );
    }
    println!("\n=== bottom 5, for a sanity floor ===");
    for s in scores.iter().rev().take(5) {
        println!(
            "  mean dot {:+.3}  within 18deg {:5.1}%  n={}  {}",
            s.mean_dot,
            100.0 * s.within_18 as f64 / s.n as f64,
            s.n,
            s.label
        );
    }

    Ok(())
}
