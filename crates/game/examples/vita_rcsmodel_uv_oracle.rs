//! Scratch probe: does 2048's declared `Uv1` attribute decode as two
//! little-endian `f16`s, against the same **index-exact** oracle that
//! settled the normal encoding (`vita_rcsmodel_exact.rs`)?
//!
//! Byte budget already argues for 2 x f16 (`Uv1@stride-8` to
//! `lightmapUV@stride-4` is 4 bytes on the common 28-byte layout, and this
//! reading already decodes `lightmapUV` as f16 pairs). What this probe adds:
//! **content** agreement against Wipeout HD's own diffuse UV, on the 9
//! same-export submeshes `vita_rcsmodel_exact.rs` found (1,504 vertices, zero
//! correspondence ambiguity) - the sharpest oracle available, the same one
//! that scored the normal decode at 100% and ruled out three wrong
//! candidates before it.
//!
//! Also tests the discriminator `f16` and `unorm16` cannot be told apart by
//! range alone: whether the decoded values ever leave `[0, 1]`, which only a
//! float encoding (not a normalised unsigned short) can represent - real on
//! a track that tiles a road texture.
//!
//! ```sh
//! cargo run -q -p oag-game --example vita_rcsmodel_uv_oracle
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

const POSITION_HASH: u32 = 0xb9d3_1b0a;
const UV1_HASH: u32 = 0x4272_14fc;

struct HdSubmesh {
    positions: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
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
            let Ok(uvs) = mesh.texcoords(&blob, submesh, stride) else {
                continue;
            };
            out.push(HdSubmesh { positions, uvs });
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
    let sum: f32 = a
        .iter()
        .zip(b)
        .map(|(p, q)| (0..3).map(|i| (p[i] - q[i]).powi(2)).sum::<f32>())
        .sum();
    sum / a.len() as f32
}

/// `Uv1`'s declared offset for one stride, the same anchor-scan
/// `declared_normal_offset` uses.
fn declared_uv1_offset(blob: &[u8], section: psp2::Section, stride: usize) -> Option<u8> {
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
            if name_hash == UV1_HASH {
                return Some(bytes[base + 7]);
            }
        }
    }
    None
}

fn f16_pair(bytes: &[u8], at: usize) -> [f32; 2] {
    let word = |o: usize| u16::from_le_bytes([bytes[at + o], bytes[at + o + 1]]);
    [
        rcsmodel::unpack_half(word(0)),
        rcsmodel::unpack_half(word(2)),
    ]
}

fn unorm16_pair(bytes: &[u8], at: usize) -> [f32; 2] {
    let word = |o: usize| u16::from_le_bytes([bytes[at + o], bytes[at + o + 1]]);
    [f32::from(word(0)) / 65535.0, f32::from(word(2)) / 65535.0]
}

fn main() -> anyhow::Result<()> {
    let mut f16_dist_sum = 0f64;
    let mut unorm_dist_sum = 0f64;
    let mut n = 0usize;
    let mut f16_finite = 0usize;
    let mut f16_nan = 0usize;
    let mut f16_out_of_unit_range = 0usize;
    let mut hd_out_of_unit_range = 0usize;
    let mut samples: Vec<([f32; 2], [f32; 2], [f32; 2])> = Vec::new(); // (f16, unorm, hd)

    for &(archive, hd_env, name) in PAIRS {
        let hd = match hd_submeshes(archive, hd_env) {
            Ok(v) => v,
            Err(e) => {
                println!("{name}: HD .rcsmodel unavailable: {e}");
                continue;
            }
        };
        let Some(blob) = vita_blob(name) else {
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

            let Some(uv1_off) = declared_uv1_offset(&blob, cpu, submesh.stride) else {
                continue;
            };
            let uv1_off = usize::from(uv1_off);

            let base = cpu.at + submesh.record;
            let vertex_ptr =
                u32::from_le_bytes(blob[base + psp2::VERTEX_POINTER..][..4].try_into().unwrap());
            let vertex_at = gpu.at + vertex_ptr as usize;
            let stride = submesh.stride;

            for (v, hd_uv) in hd[hi].uvs.iter().enumerate() {
                let at = vertex_at + v * stride + uv1_off;
                if at + 4 > blob.len() {
                    continue;
                }
                let f16 = f16_pair(&blob, at);
                let unorm = unorm16_pair(&blob, at);
                n += 1;
                let f16_dist2 = (f16[0] - hd_uv[0]).powi(2) + (f16[1] - hd_uv[1]).powi(2);
                if f16_dist2.is_finite() {
                    f16_dist_sum += f64::from(f16_dist2);
                    f16_finite += 1;
                } else {
                    f16_nan += 1;
                }
                unorm_dist_sum +=
                    f64::from((unorm[0] - hd_uv[0]).powi(2) + (unorm[1] - hd_uv[1]).powi(2));
                if f16[0] < 0.0 || f16[0] > 1.0 || f16[1] < 0.0 || f16[1] > 1.0 {
                    f16_out_of_unit_range += 1;
                }
                if hd_uv[0] < 0.0 || hd_uv[0] > 1.0 || hd_uv[1] < 0.0 || hd_uv[1] > 1.0 {
                    hd_out_of_unit_range += 1;
                }
                if samples.len() < 30 {
                    samples.push((f16, unorm, *hd_uv));
                }
            }
            println!(
                "{name}: matched submesh (dist2={d:.6}), {} vertices checked so far",
                hd[hi].uvs.len()
            );
        }
    }

    println!("=== UV1 oracle results, n={n} ===");
    if n > 0 {
        println!(
            "  f16 decode:    mean squared dist to HD's own UV over {f16_finite} finite pair(s) = {:.6} ({f16_nan} NaN)",
            f16_dist_sum / f16_finite.max(1) as f64
        );
        println!(
            "  unorm16 decode: mean squared dist to HD's own UV = {:.6}",
            unorm_dist_sum / n as f64
        );
        println!("  f16 decode values outside [0,1]:    {f16_out_of_unit_range} / {n}");
        println!("  HD's own UV values outside [0,1]:   {hd_out_of_unit_range} / {n}");
        println!("  sample (f16, unorm16, HD):");
        for (f16, unorm, hd) in &samples {
            println!(
                "    f16=({:+.4},{:+.4}) unorm16=({:+.4},{:+.4}) hd=({:+.4},{:+.4})",
                f16[0], f16[1], unorm[0], unorm[1], hd[0], hd[1]
            );
        }
    } else {
        println!("  no index-exact submesh pairs with a declared Uv1 found");
    }

    Ok(())
}
