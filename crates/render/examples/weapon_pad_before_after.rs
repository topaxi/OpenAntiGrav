//! Scratch capture: `talons_junction`'s `Weapon Pad` geometry, as authored and
//! with `oag_render::weapon_pad::ready_colour(0.0)` applied the way
//! `Drawable::tint_weapon_pads` does at the ready cycle's first keyframe,
//! side by side as two PNGs.

use oag_mesh::capture;
use oag_mesh::mesh;
use oag_mesh::mesh_render::Anisotropy;

/// A single node's own slice of `model`, rebased to its own vertex buffer,
/// keeping only the draws whose triangles fall entirely inside `range`.
fn isolate(model: &mesh::Model, range: std::ops::Range<u32>) -> mesh::Model {
    let mut out = model.clone();
    out.vertices = model.vertices[range.start as usize..range.end as usize].to_vec();
    let mut indices = Vec::new();
    let mut draws = Vec::new();
    for draw in model
        .draws
        .iter()
        .chain(&model.alpha_tested_draws)
        .chain(&model.transparent_draws)
    {
        let slice = &model.indices[draw.range.start as usize..draw.range.end as usize];
        if !slice.is_empty() && slice.iter().all(|i| range.contains(i)) {
            let start = indices.len() as u32;
            indices.extend(slice.iter().map(|i| i - range.start));
            draws.push(mesh::DrawCall {
                range: start..indices.len() as u32,
                ..draw.clone()
            });
        }
    }
    out.indices = indices;
    out.draws = draws;
    out.alpha_tested_draws = Vec::new();
    out.transparent_draws = Vec::new();
    out.node_vertex_ranges = Vec::new();
    let (centre, radius) = {
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for v in &out.vertices {
            for k in 0..3 {
                lo[k] = lo[k].min(v.position[k]);
                hi[k] = hi[k].max(v.position[k]);
            }
        }
        let centre = [
            (lo[0] + hi[0]) * 0.5,
            (lo[1] + hi[1]) * 0.5,
            (lo[2] + hi[2]) * 0.5,
        ];
        let radius = (0..3)
            .map(|k| (hi[k] - lo[k]) * 0.5)
            .fold(0.0f32, f32::max)
            .max(0.5);
        (centre, radius)
    };
    out.centre = centre;
    out.radius = radius;
    out
}

fn main() -> anyhow::Result<()> {
    let image = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "data/images/hdfury-ps3-eu-dec.iso".into());
    let spec = format!("{image}:PS3_GAME/USRDIR/DATA00.PSARC");
    let name = "/data/environments/talons_junction/track.vex";
    let data = mesh::read_blob(&spec, name)?;
    let geometry = mesh::rcs::sibling_geometry(&spec, name, &data)
        .ok_or_else(|| anyhow::anyhow!("no .rcsmodel sibling"))?;

    let (all_pads, report) = mesh::rcs::build_weapon_pads(name, &data, &geometry, &mut |path| {
        mesh::read_blob(&spec, path).ok()
    })?;
    println!("{}", report.describe());
    println!(
        "{} vertices, {} triangles, radius {:.2}",
        all_pads.vertices.len(),
        all_pads.indices.len() / 3,
        all_pads.radius
    );

    // Nine pads scattered around a 600+ unit circuit frame as nine specks from
    // any camera that fits them all. Isolate one node's own geometry - the
    // range `Drawable::tint_weapon_pads` would rewrite - so the close-up
    // shows what a player actually sees crossing a single plate.
    let one = all_pads
        .node_vertex_ranges
        .iter()
        .find(|r| !r.is_empty())
        .expect("at least one Weapon Pad node decoded")
        .clone();
    let before = isolate(&all_pads, one);

    let ready = oag_render::weapon_pad::ready_colour(0.0);
    let mut after = before.clone();
    for v in &mut after.vertices {
        v.colour = [ready[0], ready[1], ready[2], v.colour[3]];
    }

    let yaw = 0.6;
    let pitch = 0.9;
    capture::capture_from(
        &before,
        std::path::Path::new("/tmp/weapon_pad_before.png"),
        960,
        720,
        yaw,
        pitch,
        Anisotropy::Off,
        0.0,
    )?;
    capture::capture_from(
        &after,
        std::path::Path::new("/tmp/weapon_pad_after.png"),
        960,
        720,
        yaw,
        pitch,
        Anisotropy::Off,
        0.0,
    )?;
    println!("wrote /tmp/weapon_pad_before.png and /tmp/weapon_pad_after.png");
    Ok(())
}
