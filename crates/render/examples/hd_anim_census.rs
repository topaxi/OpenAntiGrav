//! Every `Anim Transform` node on the Wipeout HD disc, read big-endian.
//!
//! The predicate that turns four hand-read payloads into a measurement: each
//! decodes, its six key arrays tile contiguously from `0x50`, and its
//! `seconds_per_key` and `flags` match what Pulse's evaluators were read
//! against. A non-zero `flags` anywhere means the decode does not apply.

use oag_vex::vex;

/// All seven of the disc's archives - the same list `oag_hd::archives::ALL`
/// holds, spelled here because `oag-render` does not depend on `oag-hd`.
const ARCHIVES: &[&str] = &[
    "PS3_GAME/USRDIR/DATA00.PSARC",
    "PS3_GAME/USRDIR/DATA01.PSARC",
    "PS3_GAME/USRDIR/DATA02.PSARC",
    "PS3_GAME/USRDIR/DATA03.PSARC",
    "PS3_GAME/USRDIR/DATA04.PSARC",
    "PS3_GAME/USRDIR/DATA05.PSARC",
    "PS3_GAME/USRDIR/DATA06.PSARC",
];

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());

    let mut files = 0usize;
    let mut with_anim = 0usize;
    let mut nodes_total = 0usize;
    let mut decoded = 0usize;
    let mut contiguous = 0usize;
    let mut trailing_max = 0usize;
    let mut bad_rate = Vec::new();
    let mut bad_flags = Vec::new();
    let mut bad_flags_n = 0usize;
    let mut bad_flags_with_mesh = 0usize;
    let mut flag_files: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut gappy = Vec::new();
    let mut gappy_n = 0usize;
    let mut undecoded = Vec::new();
    let mut meshes_under = 0usize;
    let mut worst_file = (0usize, String::new());

    for archive in ARCHIVES {
        let spec = format!("{image}:{archive}");
        let Ok(mut open) = oag_assets::psarc::Archive::open(&spec) else {
            println!("skipping {archive}: does not open");
            continue;
        };
        let paths: Vec<String> = open
            .paths()
            .iter()
            .filter(|p| p.ends_with(".vex"))
            .cloned()
            .collect();
        let mut archive_nodes = 0usize;
        for path in paths {
            let Ok(blob) = open.read_path(&path) else {
                continue;
            };
            let Ok(classes) = vex::classes_of(&blob) else {
                continue;
            };
            let Some(anim_class) = classes.anim_transform else {
                continue;
            };
            let Ok(nodes) = vex::nodes(&blob) else {
                continue;
            };
            files += 1;
            let here: Vec<usize> = nodes
                .iter()
                .enumerate()
                .filter(|(_, n)| n.class_id == anim_class)
                .map(|(i, _)| i)
                .collect();
            if here.is_empty() {
                continue;
            }
            with_anim += 1;
            if here.len() > worst_file.0 {
                worst_file = (here.len(), path.clone());
            }
            archive_nodes += here.len();
            nodes_total += here.len();

            // How much geometry the class carries: every `Mesh` node whose
            // nearest anchor is one of these.
            let anchors = vex::anim_anchors(&blob, &nodes);
            if let Some(mesh_class) = classes.mesh {
                meshes_under += nodes
                    .iter()
                    .enumerate()
                    .filter(|(i, n)| n.class_id == mesh_class && anchors[*i].anchor.is_some())
                    .count();
            }

            for index in here {
                let node = &nodes[index];
                let payload = &blob[node.payload()];
                let Some(anim) = vex::anim_transform_of(&blob, node) else {
                    if undecoded.len() < 8 {
                        undecoded.push(format!("{path}:{:?}", node.name));
                    }
                    continue;
                };
                decoded += 1;
                if (anim.seconds_per_key - 1.0 / 60.0).abs() > 1e-6 && bad_rate.len() < 8 {
                    bad_rate.push(format!("{path}:{:?} = {}", node.name, anim.seconds_per_key));
                }
                let below = nodes
                    .iter()
                    .enumerate()
                    .filter(|(i, n)| {
                        Some(n.class_id) == classes.mesh && anchors[*i].anchor == Some(index)
                    })
                    .count();
                if anim.flags != 0 {
                    bad_flags_n += 1;
                    bad_flags_with_mesh += usize::from(below > 0);
                    flag_files.insert(path.clone());
                    if below > 0 && bad_flags.len() < 6 {
                        bad_flags.push(format!(
                            "{path}:{:?} = {:#x}, {below} meshes, base={:?} quantum={:?}",
                            node.name, anim.flags, anim.translation_base, anim.translation_quantum
                        ));
                    }
                }
                // The tiling check, in file order: the six arrays laid end to
                // end from 0x50, with only alignment padding left over.
                let order = vex::byte_order(&blob);
                let u16_at = |at: usize| usize::from(order.u16(payload, at));
                let u32_at = |at: usize| order.u32(payload, at) as usize;
                // The hypothesis the 97 outliers suggest: flags bit 0 widens a
                // translation value from an `s16` triple to an `f32` triple,
                // and bit 2 widens a rotation value to a whole `f32`
                // quaternion. Both are what a camera path wants precision for.
                let tvalue = if anim.flags & 1 != 0 { 12 } else { 6 };
                let rvalue = if anim.flags & 4 != 0 { 16 } else { 6 };
                let mut spans = [
                    (u32_at(0x0c), u16_at(0x02).max(1) * 2),
                    (u32_at(0x2c), u16_at(0x02).max(1) * tvalue),
                    (u32_at(0x08), u16_at(0x04).max(1) * 2),
                    (u32_at(0x1c), u16_at(0x04).max(1) * rvalue),
                    (u32_at(0x38), u16_at(0x06).max(1) * 2),
                    (u32_at(0x40), u16_at(0x06).max(1) * 6),
                ];
                spans.sort_unstable();
                let mut at = 0x50;
                let mut ok = true;
                for (start, len) in spans {
                    // HD pads a key array to a 4-byte boundary; Pulse leaves
                    // none. Anything wider than that is a misread, not padding.
                    if start < at || start - at > 3 {
                        ok = false;
                        break;
                    }
                    at = start + len;
                }
                if ok && at <= payload.len() {
                    contiguous += 1;
                    trailing_max = trailing_max.max(payload.len() - at);
                } else {
                    gappy_n += 1;
                    if gappy.len() < 6 {
                        gappy.push(format!(
                            "{path}:{:?} counts=({},{},{}) spans={spans:x?} len={:#x} {below} meshes",
                            node.name,
                            u16_at(0x02),
                            u16_at(0x04),
                            u16_at(0x06),
                            payload.len(),
                        ));
                    }
                }
            }
        }
        println!("{archive}: {archive_nodes} Anim Transform nodes");
    }

    println!("\n{files} .vex files with a class table, {with_anim} authoring the class");
    println!("{nodes_total} Anim Transform nodes, {meshes_under} Mesh nodes anchored to one");
    println!("busiest file: {} nodes in {}", worst_file.0, worst_file.1);
    println!("{decoded} decode, {contiguous} tile contiguously from 0x50");
    println!("worst trailing padding: {trailing_max} bytes");
    println!("seconds_per_key != 1/60: {bad_rate:?}");
    println!("flags != 0: {bad_flags_n} nodes, {bad_flags_with_mesh} with a mesh under them");
    for line in &bad_flags {
        println!("  {line}");
    }
    println!("files authoring one: {flag_files:#?}");
    println!("not contiguous under the widened reading: {gappy_n} nodes");
    for line in &gappy {
        println!("  {line}");
    }
    println!("undecoded: {undecoded:?}");
    Ok(())
}
