//! Scratch probe: print a Pulse circuit's authored `AmbientLight` and
//! `DirectionalLight` nodes - colour, intensity and the direction the
//! original hands the GE (`-row 2` of the node's world matrix) - to compare
//! against the lighting list a live PPSSPP race builds for a hull.
//!
//! ```sh
//! cargo run -q -p oag-vex --example light_rig_probe -- data/images/pulse-psp-usa.chd 16_Track
//! ```

use oag_vex::{lighting, vex};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let image = args
        .next()
        .ok_or("usage: light_rig_probe IMAGE TRACK_DIR")?;
    let track = args.next().unwrap_or_else(|| "16_Track".to_string());
    let mut archives = oag_pulse::open(&image)?;
    let name = if track.contains('\\') {
        track.clone()
    } else {
        format!(r"Data\Environments\{track}\track.vex")
    };
    let blob = archives.read_name(&name)?;
    let nodes = vex::nodes(&blob)?;
    for light in lighting::ambient_lights(&blob, &nodes) {
        println!(
            "ambient colour {:?} intensity {} -> x255 {:?}",
            light.colour,
            light.intensity,
            light.colour.map(|c| c * light.intensity * 255.0)
        );
    }
    for light in lighting::directional_lights(&blob, &nodes) {
        let m = light.to_world;
        println!(
            "directional colour {:?} intensity {} -> x255 {:?}\n    rows {:?} {:?} {:?} {:?}",
            light.colour,
            light.intensity,
            light.colour.map(|c| c * light.intensity * 255.0),
            &m[0..4],
            &m[4..8],
            &m[8..12],
            &m[12..16]
        );
    }
    glow_census(&blob)?;
    if let Some(node) = oag_vex::track::find_node(&blob, &nodes) {
        let track = oag_vex::track::parse(&blob[node.payload()])?;
        println!("track version {:#x}", track.version);
        let mut histogram = std::collections::BTreeMap::<[u8; 4], usize>::new();
        for (index, path) in track.paths.iter().enumerate() {
            for point in &path.points {
                *histogram.entry(point.light_scale).or_default() += 1;
            }
            let near = path.points.iter().min_by(|a, b| {
                let d = |p: &oag_vex::track::SplinePoint| {
                    let q = [p.pos[0] - 104.8, p.pos[1] + 48.4, p.pos[2] + 203.7];
                    q[0] * q[0] + q[1] * q[1] + q[2] * q[2]
                };
                d(a).total_cmp(&d(b))
            });
            if let Some(p) = near {
                println!(
                    "path {index}: nearest point to the grid slot {:?} scale {:?}",
                    p.pos, p.light_scale
                );
            }
        }
        println!("light_scale histogram (top): {:?}", {
            let mut v: Vec<_> = histogram.into_iter().collect();
            v.sort_by_key(|(_, n)| std::cmp::Reverse(*n));
            v.truncate(12);
            v
        });
    }
    Ok(())
}

/// Census of batches whose `pass_mask & 0xc0` is set - the ones
/// `FUN_089307b4` re-points the stencil reference for - per texture.
fn glow_census(blob: &[u8]) -> Result<(), Box<dyn std::error::Error>> {
    let classes = vex::classes_of(blob)?;
    let nodes = vex::nodes(blob)?;
    let tex_nodes: Vec<_> = nodes
        .iter()
        .filter(|n| Some(n.class_id) == classes.texture)
        .collect();
    let mut counts = std::collections::BTreeMap::<(u16, u32), (usize, usize)>::new();
    let mut total = 0usize;
    for node in nodes.iter().filter(|n| Some(n.class_id) == classes.mesh) {
        let payload = &blob[node.payload()];
        let materials = vex::mesh_materials(payload);
        for list in 0..2u8 {
            for batch in vex::mesh_batches(payload, list)? {
                total += 1;
                let bits = batch.pass_mask & 0xc0;
                let texture = materials
                    .get(usize::from(batch.material_index))
                    .copied()
                    .flatten()
                    .map_or(u32::MAX, |m| m.texture);
                let entry = counts.entry((bits, texture)).or_default();
                entry.0 += 1;
                entry.1 += usize::from(batch.is_transparent());
            }
        }
    }
    println!("{total} batches");
    for ((bits, texture), (n, transparent)) in counts {
        if bits == 0 {
            continue;
        }
        let name = tex_nodes
            .get(texture as usize)
            .and_then(|n| vex::texture_asset_path(&blob[n.payload()]))
            .unwrap_or_default();
        let head = tex_nodes
            .get(texture as usize)
            .map(|n| blob[n.payload()][..0x38.min(n.payload().len())].to_vec())
            .unwrap_or_default();
        println!(
            "bits {bits:#04x} texture {texture} {name}: {n} batches ({transparent} transparent) head {head:02x?}"
        );
    }
    Ok(())
}
