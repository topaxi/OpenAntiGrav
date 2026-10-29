//! ADR-0076's proof: the track model built from an exported circuit's art equals
//! the one built from its `.vex`.
//!
//! What is compared, draw by draw in each list's own order: the draw's vertices
//! (position, normal, colour, texture coordinate, the `lit` flag), its triangles
//! rebased to its first vertex, its texture slot and the texture that slot
//! resolves to, its blend class, its culled flag and its bounds. Each draw's
//! block sits elsewhere in the vertex buffer: the art keeps the order the lists
//! dispatch in, which the `.vex` build reaches by sorting on a render layer the
//! format does not carry.
//!
//! The reference is the circuit's own model with the finest tier of each
//! `LodGroup` (a track file has no level of detail), built here from `oag_mesh`
//! rather than through the exporter's own helper. Geometry under an
//! `Anim Transform` is compared at its time-zero pose, the pose the exporter
//! writes. Not compared, because the format carries none of it: texture
//! animation, the extra pass, layers, depth points, alpha-test references, glow.

use oag_mesh::mesh::{self, DrawCall, GpuVertex, Model, ModelTexture, Texels};
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
    if (w.texture, w.blend, w.culled) != (g.texture, g.blend, g.culled) {
        return Err(format!(
            "material: texture {:?} blend {:?} culled {} against {:?} {:?} {}",
            w.texture, w.blend, w.culled, g.texture, g.blend, g.culled
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

    let nodes = vex::nodes(&blob).map_err(|e| e.to_string())?;
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
    }
    tally.circuits += 1;
    Ok(())
}
