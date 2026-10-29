//! ADR-0076's proof: the track model built from an exported circuit's art equals
//! the one built from its `.vex`.
//!
//! What is compared, draw by draw in each list's own order: the draw's vertices
//! (position, normal, colour, texture coordinate, the `lit` flag), its triangles
//! rebased to its first vertex, its texture slot and the texture that slot
//! resolves to, its blend class, its culled flag, its layer, its alpha-test
//! reference and its bounds. Colours go through the file linear and come back
//! as the circuit's own bytes. Beside the draws: the hull light rig built from
//! the file's ambient, sun and light rows against `HullLights::from_track` on
//! the circuit, and, where they agree, the whole vertex and index buffers.
//!
//! The reference is the circuit's own model with the finest tier of each
//! `LodGroup` (a track file has no level of detail), built here from `oag_mesh`
//! rather than through the exporter's own helper. Geometry under an
//! `Anim Transform` is compared at its time-zero pose, the pose the exporter
//! writes. Not compared, because the format carries none of it: texture
//! animation, the extra pass, depth points, glow.

use oag_mesh::mesh::{self, DrawCall, GpuVertex, Model, ModelTexture, Texels};
use oag_mesh::mesh_render::HullLights;
use oag_raceplay::foreign::art;
use oag_trackfile::Track;
use oag_vex::vex;

/// What the walk saw, summed over every circuit.
#[derive(Debug, Default)]
pub struct Tally {
    pub circuits: usize,
    pub draws: usize,
    pub vertices: usize,
    pub triangles: usize,
    /// Draws under an `Anim Transform`. The `.vex` build boxes them in anchor
    /// space and lifts only the box's centre by the anchor's time-zero matrix, a
    /// sphere that depends on the anchor's rotation (up to 8% wider or narrower
    /// on Pulse); the art boxes the placed vertices. Their bounds are held to the
    /// box sphere of the reference's placed vertices, bit for bit.
    pub moving_draws: usize,
    /// Texture slots some draw uses, each compared pixel for pixel.
    pub textures_compared: usize,
    /// Of those, slots whose asset path another node of the circuit shares.
    pub shared_names: usize,
    pub skies: usize,
    pub flares: usize,
    /// Of the flares, those whose direction is not the circuit's first
    /// directional light's, so the file carries the flare's own.
    pub flare_directions: usize,
    /// Circuits whose hull rig from the file equals the circuit's own.
    pub hull_rigs: usize,
    /// Circuits that author no light at all (no rig on either side).
    pub no_hull_rig: usize,
    /// Circuits whose whole vertex and index buffers equal the `.vex` build's
    /// as built.
    pub buffers_equal: usize,
    /// The others, by vertices: the `.vex` build keeps the coarser `LodGroup`
    /// tiers' vertices and indices after `keep_nearest` drops their draws. Each
    /// is held to buffer equality once those are dropped (`kept_only`).
    pub buffers_equal_without_lod_tiers: Vec<String>,
}

fn reference(archives: &mut oag_assets::Archives, track: &str, blob: &[u8]) -> Model {
    let mut model = mesh::build_with_textures(track, blob, None).expect("the circuit builds");
    if !model.textures.is_empty()
        && model.textures.iter().all(Option::is_none)
        && let Some(set) = oag_livery::entry::ps2_texture_set(archives, track)
    {
        model = mesh::build_with_textures(track, blob, Some(&set)).expect("the PS2 circuit builds");
    }
    model.keep_nearest();
    model
}

fn same_texels(a: &ModelTexture, b: &ModelTexture) -> bool {
    let texels = match (&a.texels, &b.texels) {
        (Texels::Rgba8(x), Texels::Rgba8(y)) => x == y,
        (Texels::Chain(x), Texels::Chain(y)) => x == y,
        _ => false,
    };
    texels
        && a.label == b.label
        && a.width == b.width
        && a.height == b.height
        && a.mip_count == b.mip_count
}

/// The reference vertex at its time-zero pose, as a track file holds it.
fn placed(v: &GpuVertex, poses: &[[f32; 16]]) -> ([f32; 3], [f32; 3]) {
    match (v.xform as usize)
        .checked_sub(1)
        .and_then(|at| poses.get(at))
    {
        None => (v.position, v.normal),
        Some(m) => {
            let n = vex::transform_direction(m, v.normal);
            let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
            let n = if len > 0.0 { n.map(|c| c / len) } else { n };
            (vex::transform_point(m, v.position), n)
        }
    }
}

/// `mesh::build_with_textures`'s per-batch sphere: the AABB's circumscribing
/// sphere.
fn box_sphere(points: &[[f32; 3]]) -> ([f32; 3], f32) {
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for p in points {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    let centre = std::array::from_fn(|k| (lo[k] + hi[k]) * 0.5);
    let radius = (0..3)
        .map(|k| (hi[k] - centre[k]).powi(2))
        .sum::<f32>()
        .sqrt();
    (centre, radius)
}

fn block(model: &Model, draw: &DrawCall) -> (u32, u32) {
    let indices = &model.indices[draw.range.start as usize..draw.range.end as usize];
    (
        indices.iter().copied().min().unwrap_or(0),
        indices.iter().copied().max().unwrap_or(0),
    )
}

fn same_draw(
    want: &Model,
    got: &Model,
    w: &DrawCall,
    g: &DrawCall,
    poses: &[[f32; 16]],
) -> Result<(), String> {
    let (wlo, whi) = block(want, w);
    let (glo, ghi) = block(got, g);
    if whi - wlo != ghi - glo {
        return Err(format!(
            "vertex count {} against {}",
            whi - wlo + 1,
            ghi - glo + 1
        ));
    }
    for (k, (wv, gv)) in want.vertices[wlo as usize..=whi as usize]
        .iter()
        .zip(&got.vertices[glo as usize..=ghi as usize])
        .enumerate()
    {
        let (position, normal) = placed(wv, poses);
        if position != gv.position
            || normal != gv.normal
            || wv.colour != gv.colour
            || wv.texcoord != gv.texcoord
            || wv.lit != gv.lit
        {
            return Err(format!("vertex {k} differs: {wv:?} against {gv:?}"));
        }
    }
    let rebased = |m: &Model, d: &DrawCall, lo: u32| -> Vec<u32> {
        m.indices[d.range.start as usize..d.range.end as usize]
            .iter()
            .map(|i| i - lo)
            .collect()
    };
    if rebased(want, w, wlo) != rebased(got, g, glo) {
        return Err("triangles differ".into());
    }
    if (w.texture, w.blend, w.culled, w.layer, w.alpha_test_ref)
        != (g.texture, g.blend, g.culled, g.layer, g.alpha_test_ref)
    {
        return Err(format!(
            "material: texture {:?} blend {:?} culled {} layer {:#x} alpha ref {:?} against \
             {:?} {:?} {} {:#x} {:?}",
            w.texture,
            w.blend,
            w.culled,
            w.layer,
            w.alpha_test_ref,
            g.texture,
            g.blend,
            g.culled,
            g.layer,
            g.alpha_test_ref
        ));
    }
    let bounds = if w.moving {
        let points: Vec<[f32; 3]> = want.vertices[wlo as usize..=whi as usize]
            .iter()
            .map(|v| placed(v, poses).0)
            .collect();
        box_sphere(&points)
    } else {
        (w.bounds.centre, w.bounds.radius)
    };
    if bounds != (g.bounds.centre, g.bounds.radius) {
        return Err(format!("bounds {bounds:?} against {:?}", g.bounds));
    }
    Ok(())
}

/// One circuit: the exported (and read back) `track`'s art against its `.vex`.
pub fn art_draws_as_the_circuit(
    archives: &mut oag_assets::Archives,
    entry: &str,
    track: &Track,
    tally: &mut Tally,
) -> Result<(), String> {
    let a = track.art.as_ref().ok_or("the export carries no art")?;
    let blob = archives.read_name(entry).map_err(|e| e.to_string())?;
    let want = reference(archives, entry, &blob);
    let mut report = Vec::new();
    let textures = art::resolve_textures(a, std::path::Path::new(""), archives, &mut report);
    let got = art::model(entry, a, textures, &mut report);
    let poses = want.sample_anim_nodes(0.0);

    for (list, (w, g)) in [
        (&want.draws, &got.draws),
        (&want.alpha_tested_draws, &got.alpha_tested_draws),
        (&want.transparent_draws, &got.transparent_draws),
    ]
    .into_iter()
    .enumerate()
    {
        if w.len() != g.len() {
            return Err(format!(
                "list {list}: {} draws against {}\n{report:#?}",
                w.len(),
                g.len()
            ));
        }
        for (n, (wd, gd)) in w.iter().zip(g).enumerate() {
            same_draw(&want, &got, wd, gd, &poses)
                .map_err(|e| format!("list {list} draw {n}: {e}"))?;
            tally.moving_draws += usize::from(wd.moving);
        }
        tally.draws += w.len();
    }
    tally.vertices += got.vertices.len();
    tally.triangles += got.indices.len() / 3;
    if buffers(&want, &got, &poses).is_ok() {
        tally.buffers_equal += 1;
    } else {
        let kept = kept_only(&want);
        buffers(&kept, &got, &poses).map_err(|e| format!("buffers, LOD tiers dropped: {e}"))?;
        tally.buffers_equal_without_lod_tiers.push(format!(
            "{entry}: {} of {} vertices",
            kept.vertices.len(),
            want.vertices.len()
        ));
    }

    let nodes = vex::nodes(&blob).map_err(|e| e.to_string())?;
    let rig = art::hull_lights(a);
    match (rig, HullLights::from_track(&blob, &nodes, 256)) {
        (None, None) => tally.no_hull_rig += 1,
        (got, want) if got == want => tally.hull_rigs += 1,
        (got, want) => return Err(format!("hull rig {got:?} against {want:?}")),
    }

    let names = art::texture_names(&blob).map_err(|e| e.to_string())?;
    let mut used: Vec<usize> = [
        &want.draws,
        &want.alpha_tested_draws,
        &want.transparent_draws,
    ]
    .into_iter()
    .flatten()
    .filter_map(|d| d.texture)
    .collect();
    used.sort_unstable();
    used.dedup();
    for slot in used {
        let (Some(w), Some(g)) = (&want.textures[slot], &got.textures[slot]) else {
            return Err(format!("texture slot {slot} resolves on one side only"));
        };
        if !same_texels(w, g) {
            return Err(format!(
                "texture slot {slot} ({:?}) resolves to other pixels",
                names[slot]
            ));
        }
        tally.textures_compared += 1;
        let shared = names[slot]
            .as_deref()
            .is_some_and(|n| names.iter().flatten().filter(|m| m.as_str() == n).count() > 1);
        tally.shared_names += usize::from(shared);
    }

    let sky = mesh::build_sky(entry, &blob, None).is_ok_and(|s| !s.indices.is_empty());
    if a.sky.is_some() {
        tally.skies += 1;
    } else if sky {
        return Err("the circuit authors a sky the export does not name".into());
    }

    if let Some(node) = oag_vex::lensflare::lensflares(&blob, &nodes).first()
        && a.lens_flare.is_some()
    {
        let textures = art::resolve_textures(a, std::path::Path::new(""), archives, &mut report);
        let flare =
            art::lens_flare(a, &textures, &mut report).ok_or("the art's flare does not draw")?;
        let atlas = vex::textures(&blob)
            .map_err(|e| e.to_string())?
            .into_iter()
            .flatten()
            .find(|t| {
                t.asset_path
                    .as_deref()
                    .is_some_and(|p| p.to_ascii_lowercase().ends_with("lensflarecomponents.tga"))
            })
            .ok_or("no atlas on the disc")?;
        if (flare.direction, flare.colour) != (node.direction, node.colour)
            || (flare.atlas.width, flare.atlas.height)
                != (u32::from(atlas.width), u32::from(atlas.height))
            || flare.atlas.levels != atlas.levels_rgba()
        {
            return Err("the flare differs from the circuit's own".into());
        }
        tally.flares += 1;
        tally.flare_directions += usize::from(a.lens_flare.is_some_and(|f| f.direction.is_some()));
    }
    tally.circuits += 1;
    Ok(())
}

/// The whole buffers: the art's vertices, placed at time zero as the file holds
/// them, against the `.vex` build's, and the two index buffers. `Err` names
/// the first thing that differs.
fn buffers(want: &Model, got: &Model, poses: &[[f32; 16]]) -> Result<(), String> {
    if want.vertices.len() != got.vertices.len() {
        return Err(format!(
            "{} vertices against {} (the art holds only what a kept draw reads)",
            want.vertices.len(),
            got.vertices.len()
        ));
    }
    if want.indices != got.indices {
        let at = want
            .indices
            .iter()
            .zip(&got.indices)
            .position(|(w, g)| w != g)
            .unwrap_or(want.indices.len().min(got.indices.len()));
        return Err(format!(
            "indices differ from {at} ({} against {})",
            want.indices.len(),
            got.indices.len()
        ));
    }
    for (k, (w, g)) in want.vertices.iter().zip(&got.vertices).enumerate() {
        let (position, normal) = placed(w, poses);
        if (position, normal, w.colour, w.texcoord, w.lit)
            != (g.position, g.normal, g.colour, g.texcoord, g.lit)
        {
            return Err(format!("vertex {k} differs"));
        }
    }
    Ok(())
}

/// `model` with only what its draws read: each draw's vertex block and indices,
/// in vertex-buffer order, rebased. What `keep_nearest` leaves behind of the
/// tiers it drops goes.
fn kept_only(model: &Model) -> Model {
    let mut draws: Vec<(usize, usize)> = [
        &model.draws,
        &model.alpha_tested_draws,
        &model.transparent_draws,
    ]
    .into_iter()
    .enumerate()
    .flat_map(|(list, draws)| (0..draws.len()).map(move |n| (list, n)))
    .collect();
    let draw = |m: &Model, (list, n): (usize, usize)| {
        [&m.draws, &m.alpha_tested_draws, &m.transparent_draws][list][n].clone()
    };
    draws.sort_by_key(|&at| draw(model, at).range.start);
    let mut out = model.clone();
    out.vertices.clear();
    out.indices.clear();
    for at in draws {
        let d = draw(model, at);
        let (lo, hi) = block(model, &d);
        let base = out.vertices.len() as u32;
        let first = out.indices.len() as u32;
        out.vertices
            .extend_from_slice(&model.vertices[lo as usize..=hi as usize]);
        out.indices.extend(
            model.indices[d.range.start as usize..d.range.end as usize]
                .iter()
                .map(|i| i - lo + base),
        );
        [
            &mut out.draws,
            &mut out.alpha_tested_draws,
            &mut out.transparent_draws,
        ][at.0][at.1]
            .range = first..out.indices.len() as u32;
    }
    out
}
