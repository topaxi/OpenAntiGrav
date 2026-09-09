//! Scratch probe: does a Wipeout 2048 track and craft decode with the existing
//! PSARC/vex/rcsmodel/collision/handling readers, unmodified?
//!
//! ```sh
//! cargo run -q -p oag-game --example vita_probe -- <psarc path>
//! ```

use oag_render::mesh;
use oag_tables::handling;
use oag_vex::{collision, vex};

fn main() -> anyhow::Result<()> {
    let path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/extracted/vita/PCSF00007/base/PSP2/data.psarc".to_string());
    let mut archive = oag_assets::psarc::Archive::open(&path)?;

    // Corpus check: does `HEADER_LEN + RESERVED_LEN + paths*PATH_LEN +
    // junctions*JUNCTION_LEN + total_points*96` land exactly on every
    // track's WO Track payload length, or only altima's?
    let mut track_paths: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with("/track.vex"))
        .cloned()
        .collect();
    track_paths.sort();
    println!(
        "=== corpus check: {} track.vex entries ===",
        track_paths.len()
    );
    for track_path in &track_paths {
        let Ok(blob) = archive.read_path(track_path) else {
            println!("{track_path}: read ERROR");
            continue;
        };
        let Ok(nodes) = vex::nodes(&blob) else {
            println!("{track_path}: node walk ERROR");
            continue;
        };
        let Some(node) = oag_vex::track::find_node(&blob, &nodes) else {
            println!("{track_path}: no WO Track node");
            continue;
        };
        let payload = &blob[node.payload()];
        if payload.len() < 0x10 {
            println!("{track_path}: payload too short ({} bytes)", payload.len());
            continue;
        }
        let u32_le = |at: usize| u32::from_le_bytes(payload[at..at + 4].try_into().unwrap());
        let version = u32_le(0x04);
        let path_count = u32_le(0x08) as usize;
        let junction_count = u32_le(0x0c) as usize;
        let paths_at = 0x40; // HEADER_LEN(0x20) + RESERVED_LEN(0x20) for version >= 0x101
        let fixed = paths_at + path_count * 0x20 + junction_count * 0x10;
        if fixed > payload.len() {
            println!(
                "{track_path}: version {version:#x}, path/junction header runs past payload ({fixed} > {})",
                payload.len()
            );
            continue;
        }
        let total_points: usize = (0..path_count)
            .map(|i| u32_le(paths_at + i * 0x20) as usize)
            .sum();
        let expected_96 = fixed + total_points * 96;
        let expected_112 = fixed + total_points * 112;
        println!(
            "{track_path}: version {version:#x}, {path_count} path(s), {junction_count} junction(s), \
             {total_points} point(s), payload {} byte(s) - stride 96 {} | stride 112 {}",
            payload.len(),
            if expected_96 == payload.len() {
                "MATCH"
            } else {
                "no match"
            },
            if expected_112 == payload.len() {
                "MATCH"
            } else {
                "no match"
            },
        );
    }

    let track = "data/art/published/environments/altima/track.vex";
    println!("--- {track} ---");
    match archive.read_path(track) {
        Ok(blob) => {
            println!("bytes: {}", blob.len());
            match vex::classes_of(&blob) {
                Ok(classes) => println!("classes_of: {classes:?}"),
                Err(e) => println!("classes_of: ERROR {e}"),
            }
            let external = mesh::geometry_is_external(&blob);
            println!("geometry_is_external: {external}");
            match mesh::build_with_textures(track, &blob, None, mesh::Lod::default()) {
                Ok(model) => println!(
                    "build_with_textures OK: {} triangle(s), radius {:.2}, {} texture(s)",
                    model.indices.len() / 3,
                    model.radius,
                    model.textures.len()
                ),
                Err(e) => println!("build_with_textures: ERROR {e}"),
            }
            if let Some(sibling) = mesh::rcs::sibling_name(track) {
                println!("sibling_name: {sibling}");
                match archive.read_path(&sibling) {
                    Ok(rcs_blob) => {
                        println!("rcsmodel bytes: {}", rcs_blob.len());
                        println!(
                            "first 64 bytes: {:02x?}",
                            &rcs_blob[..64.min(rcs_blob.len())]
                        );
                        // Live::Rcs::Model header, per PSP2/Psp2.RcsModelLoader.cpp
                        // (Ghidra, FUN_812f15b2): 0x60 bytes total (+0x10 states its
                        // own length), a "main memory" size at +0x24 and a GPU
                        // section size at +0x44.
                        let u32_le = |at: usize| {
                            u32::from_le_bytes(rcs_blob[at..at + 4].try_into().unwrap())
                        };
                        println!("full header (0x60 bytes): {:02x?}", &rcs_blob[..0x60]);
                        for at in [
                            0x00, 0x04, 0x08, 0x0c, 0x10, 0x14, 0x18, 0x1c, 0x20, 0x24, 0x28, 0x2c,
                            0x30, 0x34, 0x38, 0x3c, 0x40, 0x44, 0x48, 0x4c, 0x50, 0x54, 0x58, 0x5c,
                        ] {
                            println!("  +{at:#04x}: {:#010x} ({})", u32_le(at), u32_le(at));
                        }
                        let header_len = u32_le(0x10) as usize;
                        let main_mem = u32_le(0x24) as usize;
                        let gpu_size = u32_le(0x44) as usize;
                        println!(
                            "header {header_len} + main_mem {main_mem} + gpu {gpu_size} = {} \
                             vs file length {}",
                            header_len + main_mem + gpu_size,
                            rcs_blob.len()
                        );
                        match mesh::rcs::build_scene(track, &blob, &rcs_blob, &mut |p| {
                            archive.read_path(p).ok()
                        }) {
                            Ok((model, built)) => println!(
                                "build_scene OK: {} triangle(s), {}",
                                model.indices.len() / 3,
                                built.describe()
                            ),
                            Err(e) => println!("build_scene: ERROR {e}"),
                        }
                    }
                    Err(e) => println!("read sibling: ERROR {e}"),
                }
            } else {
                println!("sibling_name: None");
            }
            match collision::from_vex(&blob) {
                Ok(nodes) => println!("collision::from_vex: {} node(s)", nodes.len()),
                Err(e) => println!("collision::from_vex: ERROR {e}"),
            }
        }
        Err(e) => println!("read: ERROR {e}"),
    }

    if let Ok(blob) = archive.read_path(track) {
        let nodes = vex::nodes(&blob).expect("node walk");
        let node = oag_vex::track::find_node(&blob, &nodes).expect("WO Track node");
        let payload = &blob[node.payload()];
        let u32_le = |at: usize| u32::from_le_bytes(payload[at..at + 4].try_into().unwrap());
        let u32_be = |at: usize| u32::from_be_bytes(payload[at..at + 4].try_into().unwrap());
        println!("WO Track payload length: {}", payload.len());
        for at in [0x00, 0x04, 0x08, 0x0c, 0x10, 0x14, 0x18, 0x1c] {
            println!(
                "  +{at:#04x}: LE {:#010x} ({}) | BE {:#010x} ({})",
                u32_le(at),
                u32_le(at),
                u32_be(at),
                u32_be(at)
            );
        }
        let paths_at = 0x40; // HEADER_LEN(0x20) + RESERVED_LEN(0x20), version 0x107 >= 0x101
        let path_count = u32_le(0x08) as usize;
        let counts: Vec<usize> = (0..path_count)
            .map(|i| u32_le(paths_at + i * 0x20) as usize)
            .collect();
        let total_points: usize = counts.iter().sum();
        println!("per-path point counts: {counts:?}, total {total_points}");
        let junction_count_hdr = u32_le(0x0c) as usize;
        let fixed = paths_at + path_count * 0x20 + junction_count_hdr * 0x10;
        let overshoot = fixed + total_points * 112;
        let overshoot = overshoot.saturating_sub(payload.len());
        println!("overshoot against stride 0x70 (112): {overshoot} byte(s)");
        if total_points > 0 && overshoot.is_multiple_of(total_points) {
            println!(
                "overshoot / total_points = {} bytes/point (assumed stride 0x70 = 112)",
                overshoot / total_points
            );
        } else {
            println!("overshoot {overshoot} does not divide evenly by total_points {total_points}");
        }

        // Stride-detection by continuity: the first path's points sit at
        // `points_at`, back to back. For the right stride, consecutive
        // `pos` reads (the first f32x3 of each point) are close together in
        // world space; for the wrong one they are noise. Score every
        // candidate stride and print the smoothest few.
        let junction_count = u32_le(0x0c) as usize;
        let points_at = paths_at + path_count * 0x20 + junction_count * 0x10;
        let first_path_points = counts[0];
        let f32_le = |at: usize| f32::from_le_bytes(payload[at..at + 4].try_into().unwrap());
        let pos_at = |base: usize| [f32_le(base), f32_le(base + 4), f32_le(base + 8)];
        let mut scored: Vec<(usize, f32)> = Vec::new();
        for stride in 16..=160usize {
            if points_at + first_path_points * stride > payload.len() {
                continue;
            }
            let mut score = 0.0f32;
            for k in 0..first_path_points.saturating_sub(1) {
                let a = pos_at(points_at + k * stride);
                let b = pos_at(points_at + (k + 1) * stride);
                if a.iter().any(|v| !v.is_finite()) || b.iter().any(|v| !v.is_finite()) {
                    score += 1e9;
                    continue;
                }
                let d = (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2);
                score += d;
            }
            scored.push((stride, score));
        }
        scored.sort_by(|a, b| a.1.total_cmp(&b.1));
        println!(
            "best strides by continuity: {:?}",
            &scored[..10.min(scored.len())]
        );

        // With stride 96 confirmed (exact length match), find where each
        // unit-length vector (tangent/down/lateral) sits inside the record:
        // score every 4-byte offset by how close a [f32;3] there is to norm 1
        // across every point of every path.
        let stride = 96usize;
        let mut norm_scores = [0.0f32; 96 / 4];
        let mut samples = 0usize;
        let mut at = points_at;
        for &count in &counts {
            for k in 0..count {
                let base = at + k * stride;
                if base + 12 > payload.len() {
                    continue;
                }
                samples += 1;
                // `slot` doubles as an array index and a byte-offset
                // multiplier, so `enumerate()` doesn't fit here. Every slot
                // in the record, including ones whose 12-byte read spills
                // into the next point (or past the buffer, guarded below) -
                // narrowing this range previously left the last two slots
                // at their initialised 0.0 and made them look like a
                // perfect match by omission.
                #[allow(clippy::needless_range_loop)]
                for slot in 0..(96 / 4) {
                    let off = base + slot * 4;
                    if off + 12 > payload.len() {
                        continue;
                    }
                    let v = pos_at(off);
                    let norm = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
                    norm_scores[slot] += (norm - 1.0).abs();
                }
            }
            at += count * stride;
        }
        let mut ranked: Vec<(usize, f32)> = norm_scores
            .iter()
            .enumerate()
            .map(|(slot, &s)| (slot * 4, s / samples as f32))
            .collect();
        ranked.sort_by(|a, b| a.1.total_cmp(&b.1));
        println!(
            "offsets closest to a unit vector (mean |norm-1| over {samples} points): {:?}",
            &ranked[..12.min(ranked.len())]
        );

        // Half-widths and racing_line should be small, mildly varying floats
        // (single digits to low tens of world units); scan single f32 slots
        // for that shape too, as a secondary signal.
        let mut small_float_scores = [0.0f32; 96 / 4];
        let mut at = points_at;
        for &count in &counts {
            for k in 0..count {
                let base = at + k * stride;
                #[allow(clippy::needless_range_loop)]
                for slot in 0..(96 / 4) {
                    let off = base + slot * 4;
                    if off + 4 > payload.len() {
                        continue;
                    }
                    let v = f32_le(off);
                    // Penalise anything outside a plausible half-width/offset
                    // range or non-finite; reward small magnitude.
                    small_float_scores[slot] += if v.is_finite() && v.abs() < 200.0 {
                        v.abs()
                    } else {
                        1e6
                    };
                }
            }
            at += count * stride;
        }
        let mut ranked2: Vec<(usize, f32)> = small_float_scores
            .iter()
            .enumerate()
            .map(|(slot, &s)| (slot * 4, s / samples as f32))
            .collect();
        ranked2.sort_by(|a, b| a.1.total_cmp(&b.1));
        println!(
            "offsets with small, finite float values (candidates for half-width/racing_line): {:?}",
            &ranked2[..16.min(ranked2.len())]
        );

        // Eyeball the tail region [56, 96) of the first six points of path 0,
        // as floats and as raw bytes, to read off the real field layout by
        // hand rather than guessing further.
        for k in 0..6.min(counts[0]) {
            let base = points_at + k * stride;
            let floats: Vec<f32> = (56..96).step_by(4).map(|off| f32_le(base + off)).collect();
            let bytes = &payload[base + 88..base + 96];
            println!("point {k} tail[0x38..0x60) floats: {floats:?}");
            println!("point {k} tail[0x58..0x60) bytes: {bytes:02x?}");
        }

        // Hypothesis check: do the two monotonic tail fields at 0x44/0x4c
        // track pos.x/pos.z (offset 0x00/0x08) rather than being new scalar
        // fields, i.e. did the corridor/racing-line representation move from
        // lateral offsets to world-space points?
        for k in 0..6.min(counts[0]) {
            let base = points_at + k * stride;
            let pos = pos_at(base);
            let f44 = f32_le(base + 0x44);
            let f4c = f32_le(base + 0x4c);
            println!(
                "point {k}: pos.x {:.3} pos.z {:.3} | 0x44 {:.3} (diff {:.3}) | 0x4c {:.3} (diff {:.3})",
                pos[0],
                pos[2],
                f44,
                f44 - pos[0],
                f4c,
                f4c - pos[2]
            );
        }
    }

    match oag_render::track::load(&path, track) {
        Ok((ai, label)) => println!(
            "track_render::load OK ({label}): {} path(s), {} junction(s), {} point(s)",
            ai.paths.len(),
            ai.junctions.len(),
            ai.point_count()
        ),
        Err(e) => println!("track_render::load: ERROR {e}"),
    }

    let col_path = "data/art/published/environments/altima/track_col.col";
    match archive.read_path(col_path) {
        Ok(blob) => {
            println!("{col_path}: {} byte(s)", blob.len());
            println!("first 32 bytes: {:02x?}", &blob[..32.min(blob.len())]);
            let u32_le = |at: usize| u32::from_le_bytes(blob[at..at + 4].try_into().unwrap());

            // Structural walk per Ghidra FUN_8118d134
            // (Backend/General/Collision/KdTree.cpp): "kdtr" + 4-digit ASCII
            // version, then repeating [4-byte "----" tag][section], ending
            // in a KdTreeMeshShape.cpp-owned trailer this probe does not
            // attempt.
            let magic = std::str::from_utf8(&blob[0..4]).unwrap_or("?");
            let version = std::str::from_utf8(&blob[4..8]).unwrap_or("?");
            let tag1 = &blob[8..12];
            println!("magic {magic:?} version {version:?} tag1 {tag1:02x?}");
            let node_stride = u32_le(0x0c) as usize;
            let node_count = u32_le(0x10) as usize;
            let nodes_at = 0x14;
            let nodes_end = nodes_at + node_count * node_stride;
            println!(
                "node_stride {node_stride} (expect 24) node_count {node_count} -> nodes [{nodes_at}, {nodes_end})"
            );
            if nodes_end + 12 <= blob.len() {
                let tag2 = &blob[nodes_end..nodes_end + 4];
                let leaf_count = u32_le(nodes_end + 4) as usize;
                let leaf_at = nodes_end + 8;
                let leaf_end = leaf_at + leaf_count * 2;
                println!(
                    "tag2 {tag2:02x?} leaf_count {leaf_count} -> leaves [{leaf_at}, {leaf_end})"
                );
                if leaf_end + 4 + 24 <= blob.len() {
                    let tag3 = &blob[leaf_end..leaf_end + 4];
                    let bbox_at = leaf_end + 4;
                    let f32_le =
                        |at: usize| f32::from_le_bytes(blob[at..at + 4].try_into().unwrap());
                    let bbox_min = [f32_le(bbox_at), f32_le(bbox_at + 4), f32_le(bbox_at + 8)];
                    let bbox_max = [
                        f32_le(bbox_at + 12),
                        f32_le(bbox_at + 16),
                        f32_le(bbox_at + 20),
                    ];
                    let after_bbox = bbox_at + 24;
                    println!(
                        "tag3 {tag3:02x?} bbox_min {bbox_min:?} bbox_max {bbox_max:?} - {} byte(s) left for the mesh-shape trailer",
                        blob.len().saturating_sub(after_bbox)
                    );
                }
            }
        }
        Err(e) => println!("{col_path}: ERROR {e}"),
    }

    let stats_paths = [
        "data/art/published/hdships/AG_Systems/handlingstats.xml",
        "data/HandlingStats/ag_systems2048/1/handlingstats.xml",
        "data/xml/handlingstats.xml",
    ];
    for stats_path in stats_paths {
        println!("--- {stats_path} ---");
        match archive.read_path(stats_path) {
            Ok(blob) => {
                println!(
                    "bytes: {}, contains(): {}",
                    blob.len(),
                    archive.contains(stats_path)
                );
                if stats_path == "data/xml/handlingstats.xml" {
                    match handling::global_from_blob(&blob) {
                        Ok(global) => println!("handling::global_from_blob OK: {global:?}"),
                        Err(e) => println!("handling::global_from_blob: ERROR {e}"),
                    }
                } else {
                    match handling::from_blob(&blob) {
                        Ok(stats) => println!(
                            "handling::from_blob OK: {} class(es): {:?}",
                            stats.classes.len(),
                            stats
                                .classes
                                .iter()
                                .map(|c| &c.raw_name)
                                .collect::<Vec<_>>()
                        ),
                        Err(e) => println!("handling::from_blob: ERROR {e}"),
                    }
                }
            }
            Err(e) => println!("read: ERROR {e}"),
        }
    }

    // The multi-segment root check the ship_dir axis needs: does the literal
    // Data-spelling normalise onto the manifest's stored path?
    let spelled = r"Data\art\published\hdships\AG_Systems\handlingstats.xml";
    println!(
        "--- contains({spelled:?}) = {} ---",
        archive.contains(spelled)
    );

    Ok(())
}
