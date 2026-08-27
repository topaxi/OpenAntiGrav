//! Scratch probe: does an **index-exact** correspondence exist between an HD
//! submesh and a 2048 submesh, as opposed to the spatial-nearest-neighbour
//! one `vita_rcsmodel_rosetta.rs` used?
//!
//! That probe's spatial match is inherently noisy at hard edges (several
//! vertices at the same position, different normals) and dropped 1,894,162 of
//! ~2.4M candidates as ambiguous for exactly that reason - which is a
//! plausible cause of its inconclusive content-decode result even where the
//! offset is now known to be right. If some submeshes were re-exported with
//! the **same vertex count and order** on both platforms - same source mesh,
//! same triangulation, only the container format changed - then vertex `v` of
//! a 2048 submesh is vertex `v` of the matching HD submesh with **no spatial
//! search and no ambiguity at all**, which is a much sharper oracle: exactly
//! the shape of correspondence that settled the `WO Track` tail.
//!
//! # What this checks
//!
//! For every 2048 submesh, every HD submesh with the *same vertex count* is a
//! candidate. A candidate is accepted only if comparing position `v` to
//! position `v` **directly by index** (no search) agrees closely for nearly
//! every vertex - which a coincidental count match could not produce, since
//! two unrelated submeshes of the same size read as noise at every index.
//!
//! # The method works; the result is still negative
//!
//! **The correspondence itself is real, and rare**: only 9 of 7,749
//! same-count candidates (25,712 2048 submeshes total, across the twelve
//! HD-ported circuits) pass a 5 cm index-agreement bar - most of a track was
//! genuinely re-tessellated on the way to the Vita, and the few that weren't
//! give 1,504 vertices with **zero correspondence ambiguity**, unlike the
//! spatial match this probe replaces.
//!
//! Against that clean sample, every encoding hypothesis tried still fails:
//! HD's packed word read little-endian, read big-endian (the byte order the
//! declaration's own stride field uses - a real possibility, not a stretch),
//! and a reordered-fields-plus-derived-z scheme two near-planar submeshes
//! appeared to confirm by eye all score at or below chance. **The two-example
//! match was very likely small-sample overfitting, not signal**: both
//! examples happen to be simple, close-to-axis-aligned directions, which many
//! wrong candidate decodings can fit by chance - and the same scheme reaches
//! only 20.5% within 18 degrees (mean dot 0.382) over the full 1,504-vertex
//! sample, nowhere near a real decode. The lesson is the one
//! `docs/reverse-engineering/methodology.md` already argues for: a fit on a
//! couple of hand-picked examples is not evidence until it is checked at
//! scale, and this project's own `HANDOVER.md` has a matching trap entry
//! about a heuristic that looked confirmed until the corpus disagreed.
//!
//! What this probe leaves behind that is worth keeping: the 1,504-vertex
//! index-exact corpus is a clean, reusable oracle for the *next* encoding
//! hypothesis, cheaper to test against than rebuilding spatial matching each
//! time.
//!
//! ```sh
//! cargo run -q -p oag-game --example vita_rcsmodel_exact
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

/// How close direct-index positions must land, on average, to call the
/// submeshes the same export rather than a coincidental count match.
const MEAN_DIST2_BAR: f32 = 0.0025; // 5 cm

/// One HD submesh, kept separate rather than flattened - this probe needs the
/// index boundary a flat vertex pool throws away.
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

/// Mean squared distance comparing position `v` to position `v`, no search.
fn index_agreement(a: &[[f32; 3]], b: &[[f32; 3]]) -> f32 {
    debug_assert_eq!(a.len(), b.len());
    let sum: f32 = a
        .iter()
        .zip(b)
        .map(|(p, q)| (0..3).map(|i| (p[i] - q[i]).powi(2)).sum::<f32>())
        .sum();
    sum / a.len() as f32
}

fn main() -> anyhow::Result<()> {
    let mut total_2048_submeshes = 0usize;
    let mut had_count_candidate = 0usize;
    let mut accepted = 0usize;
    let mut accepted_vertices = 0usize;

    // Normal test, but only over index-exact-paired vertices - no spatial
    // ambiguity, so any real signal should be far cleaner than the sweep in
    // vita_rcsmodel_rosetta.rs. Tested both ways the 4-byte word could be
    // assembled: the container reads little-endian everywhere checked so
    // far, EXCEPT the declaration's own repeated-stride field, which stays
    // big-endian - so a packed-normal word inheriting HD's original byte
    // order rather than the port's is a real possibility, not a stretch.
    let mut exact_dot: HashMap<&'static str, (f64, usize, usize)> = HashMap::new();
    let mut low_field_sign_matches = 0usize;
    let mut low_field_sign_total = 0usize;

    // A handful of raw byte groups from near-planar submeshes, printed for
    // direct inspection - the same move that pinned HD's own 11:11:10 split.
    let mut planar_samples: Vec<(String, [f32; 3], Vec<[u8; 4]>)> = Vec::new();

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

        let mut track_accepted = 0usize;
        let mut track_candidates = 0usize;
        for submesh in &model.submeshes {
            total_2048_submeshes += 1;
            let Some(candidates) = by_count.get(&submesh.positions.len()) else {
                continue;
            };
            if submesh.positions.len() < 8 {
                continue; // too small to tell a real match from luck
            }
            track_candidates += 1;
            had_count_candidate += 1;

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
            accepted += 1;
            track_accepted += 1;
            accepted_vertices += submesh.positions.len();

            let base = cpu.at + submesh.record;
            let vertex_ptr =
                u32::from_le_bytes(blob[base + psp2::VERTEX_POINTER..][..4].try_into().unwrap());
            let vertex_at = gpu.at + vertex_ptr as usize;
            let stride = submesh.stride;

            // Declared normal offset for this stride, if this track's
            // declarations named one - reuse the same anchor-scan
            // vita_rcsmodel_rosetta.rs uses, but only need `normal`'s offset
            // for this stride, once, so do it inline rather than importing.
            let normal_off = declared_normal_offset(&blob, cpu, stride);

            let Some(normal_off) = normal_off else {
                continue;
            };
            let normal_off = usize::from(normal_off);
            let mut sum = [0f32; 3];
            for (v, hd_normal) in hd[hi].normals.iter().enumerate() {
                let at = vertex_at + v * stride + normal_off;
                if at + 4 > blob.len() {
                    continue;
                }
                let raw: [u8; 4] = blob[at..at + 4].try_into().unwrap();
                for (label, word) in [
                    ("LE", u32::from_le_bytes(raw)),
                    ("BE", u32::from_be_bytes(raw)),
                ] {
                    let candidate = rcsmodel::unpack_normal(word);
                    let dot: f32 = (0..3).map(|a| candidate[a] * hd_normal[a]).sum();
                    let e = exact_dot.entry(label).or_default();
                    e.0 += f64::from(dot);
                    e.1 += 1;
                    if dot > 0.951 {
                        e.2 += 1;
                    }
                }

                // Two near-planar submeshes both show the LE low-11-bit
                // field matching no HD component, while the other two fields
                // (reordered) line up with HD's x/y and the missing
                // magnitude matches a unit-sphere completion for z almost
                // exactly - test that shape at scale: x = field1, y = field2,
                // z = +-sqrt(1 - x^2 - y^2), sign taken from HD (oracle-
                // assisted, so this confirms the x/y/magnitude relationship,
                // not a full decode) - and separately, whether the low field
                // correlates with that sign at all.
                let le = rcsmodel::unpack_normal(u32::from_le_bytes(raw));
                let (x, y) = (le[1], le[2]);
                let z_mag = (1.0 - x * x - y * y).max(0.0).sqrt();
                let z = z_mag.copysign(hd_normal[2]);
                let derived = [x, y, z];
                let dot: f32 = (0..3).map(|a| derived[a] * hd_normal[a]).sum();
                let e = exact_dot.entry("derived-z, oracle sign").or_default();
                e.0 += f64::from(dot);
                e.1 += 1;
                if dot > 0.951 {
                    e.2 += 1;
                }
                low_field_sign_total += 1;
                if le[0].signum() == hd_normal[2].signum() {
                    low_field_sign_matches += 1;
                }

                for a in 0..3 {
                    sum[a] += hd_normal[a];
                }
            }

            // Planar check: HD's own average direction, normalised, and how
            // tightly every HD normal in this submesh agrees with it.
            let len = (sum[0] * sum[0] + sum[1] * sum[1] + sum[2] * sum[2]).sqrt();
            if len < 1e-6 {
                continue;
            }
            let mean_dir = [sum[0] / len, sum[1] / len, sum[2] / len];
            let min_dot = hd[hi]
                .normals
                .iter()
                .map(|n| (0..3).map(|a| n[a] * mean_dir[a]).sum::<f32>())
                .fold(1.0f32, f32::min);
            if min_dot > 0.99 && planar_samples.len() < 12 {
                let words: Vec<[u8; 4]> = (0..submesh.positions.len().min(6))
                    .filter_map(|v| {
                        let at = vertex_at + v * stride + normal_off;
                        (at + 4 <= blob.len()).then(|| blob[at..at + 4].try_into().unwrap())
                    })
                    .collect();
                planar_samples.push((name.to_string(), mean_dir, words));
            }
        }
        println!(
            "{name}: {track_candidates} 2048 submesh(es) had a same-count HD candidate, \
{track_accepted} accepted (index agreement < {:.2} units)",
            MEAN_DIST2_BAR.sqrt()
        );
    }

    println!(
        "\n{accepted}/{had_count_candidate} count-matched submeshes accepted as index-exact \
({total_2048_submeshes} 2048 submeshes total), {accepted_vertices} vertices"
    );

    println!("\n=== normal at declared offset, index-exact pairs only, both word orders ===");
    if exact_dot.is_empty() {
        println!("  no index-exact submesh had a declared normal offset for its stride");
    }
    for label in ["LE", "BE", "derived-z, oracle sign"] {
        if let Some(&(sum, n, within18)) = exact_dot.get(label) {
            println!(
                "  {label}: n={n} within 18deg {:.1}% mean dot {:.3}",
                100.0 * within18 as f64 / n as f64,
                sum / n as f64,
            );
        }
    }
    if low_field_sign_total > 0 {
        println!(
            "  low LE field's sign == hd_z's sign: {:.1}% of {low_field_sign_total} (50% is chance)",
            100.0 * low_field_sign_matches as f64 / low_field_sign_total as f64,
        );
    }

    println!("\n=== raw normal bytes on near-planar index-exact submeshes ===");
    println!("(HD's own mean direction, then up to six raw byte groups from the same offset in");
    println!(" the matching 2048 submesh - look for a value constant across them, either order)");
    for (name, dir, words) in &planar_samples {
        let raw: Vec<String> = words
            .iter()
            .map(|w| format!("{:02x}{:02x}{:02x}{:02x}", w[0], w[1], w[2], w[3]))
            .collect();
        let unique: std::collections::BTreeSet<[u8; 4]> = words.iter().copied().collect();
        let decoded: Vec<String> = unique
            .iter()
            .map(|w| {
                let le = rcsmodel::unpack_normal(u32::from_le_bytes(*w));
                let be = rcsmodel::unpack_normal(u32::from_be_bytes(*w));
                format!(
                    "LE-as-11:11:10=({:+.4},{:+.4},{:+.4}) BE-as-11:11:10=({:+.4},{:+.4},{:+.4})",
                    le[0], le[1], le[2], be[0], be[1], be[2]
                )
            })
            .collect();
        println!(
            "  {name}: HD dir ({:+.3}, {:+.3}, {:+.3}) -> bytes {raw:?}\n    {}",
            dir[0],
            dir[1],
            dir[2],
            decoded.join(" | ")
        );
    }

    Ok(())
}

/// `normal`'s declared offset for one stride, found the same way
/// `vita_rcsmodel_rosetta.rs::find_declarations` does, trimmed to just the
/// one field this probe needs.
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
