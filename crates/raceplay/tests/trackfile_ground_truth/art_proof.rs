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
//! The pad plates too: the speed and weapon pad models built from the meshes
//! tagged `oag_pad` against `mesh::build_pads` and `mesh::build_weapon_pads`,
//! draw for draw.
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
use oag_trackfile::art::PadPlate;
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
    /// Circuits whose speed pad plates and weapon pad plates (each, when the
    /// circuit authors them) equal the circuit's own, draw for draw.
    pub pad_models: [usize; 2],
    /// Their draws, summed.
    pub pad_draws: usize,
    /// Circuits whose speed pads and weapon pads (each, when the circuit authors
    /// them), drawn from the host's own art on the file's volumes alone
    /// (`art::host_pads`, no plates), are the circuit's first pad on each volume.
    pub host_pad_models: [usize; 2],
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

/// The circuit's pad plates of one kind as the race loader builds them.
fn reference_pads(
    archives: &mut oag_assets::Archives,
    track: &str,
    blob: &[u8],
    which: PadPlate,
) -> Model {
    let build = match which {
        PadPlate::Speedup => mesh::build_pads,
        PadPlate::Weapon => mesh::build_weapon_pads,
    };
    let mut model = build(track, blob, None).expect("the pads build");
    if !model.textures.is_empty()
        && model.textures.iter().all(Option::is_none)
        && let Some(set) = oag_livery::entry::ps2_texture_set(archives, track)
    {
        model = build(track, blob, Some(&set)).expect("the PS2 pads build");
    }
    model
}

/// Draw for draw, list by list.
fn same_lists(want: &Model, got: &Model, poses: &[[f32; 16]]) -> Result<usize, String> {
    let mut draws = 0;
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
                "list {list}: {} draws against {}",
                w.len(),
                g.len()
            ));
        }
        for (n, (wd, gd)) in w.iter().zip(g).enumerate() {
            same_draw(want, got, wd, gd, poses)
                .map_err(|e| format!("list {list} draw {n}: {e}"))?;
        }
        draws += w.len();
    }
    Ok(draws)
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
    let got = art::model(entry, a, textures.clone(), &mut report);
    let poses = want.sample_anim_nodes(0.0);

    // The host's pad art on the file's pad volumes, with no plate to say otherwise.
    let mut host = art::Extras::default();
    art::host_pads(
        &mut host,
        None,
        &track.speedup_pads,
        &track.weapon_pads,
        entry,
        archives,
        &mut report,
    );

    for (k, which) in [PadPlate::Speedup, PadPlate::Weapon]
        .into_iter()
        .enumerate()
    {
        let want = reference_pads(archives, entry, &blob, which);
        let got = art::pad_model(entry, a, textures.clone(), which, &mut report);
        match got {
            None if want.indices.is_empty() => {}
            None => {
                return Err(format!(
                    "{which:?} pad plates: the circuit has them, the file none"
                ));
            }
            Some(got) => {
                let draws = same_lists(&want, &got, &[])
                    .map_err(|e| format!("{which:?} pad plates: {e}"))?;
                // The plates' pictures are the circuit's own texture nodes.
                for d in got
                    .draws
                    .iter()
                    .chain(&got.alpha_tested_draws)
                    .chain(&got.transparent_draws)
                {
                    if let Some(slot) = d.texture {
                        match (&want.textures[slot], &got.textures[slot]) {
                            (Some(w), Some(g)) if same_texels(w, g) => {}
                            _ => return Err(format!("{which:?} pad plate texture {slot} differs")),
                        }
                    }
                }
                tally.pad_models[k] += 1;
                tally.pad_draws += draws;
            }
        }
        let (volumes, host_model) = match which {
            PadPlate::Speedup => (&track.speedup_pads, host.pads.as_ref()),
            PadPlate::Weapon => (&track.weapon_pads, host.weapon_pads.as_ref()),
        };
        if !want.indices.is_empty() {
            let m = host_model.ok_or_else(|| format!("{which:?}: no host pad art\n{report:#?}"))?;
            host_pads_are_the_circuits(&want, m, volumes)
                .map_err(|e| format!("{which:?} host pads: {e}"))?;
            tally.host_pad_models[k] += 1;
        } else if host_model.is_some() {
            return Err(format!(
                "{which:?}: host pads drawn from a circuit with none"
            ));
        }
    }

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

/// The host pads `got`, one per volume, against the circuit's own pad model `want`:
/// one vertex range per volume, the first pad's vertices equal in everything but
/// position (lifted along the volume's up by `art::HOST_PAD_LIFT`), and every
/// texture the pads use the circuit's pixels.
fn host_pads_are_the_circuits(
    want: &Model,
    got: &Model,
    volumes: &[oag_vex::pads::PadVolume],
) -> Result<(), String> {
    if got.node_vertex_ranges.len() != volumes.len() {
        return Err(format!(
            "{} vertex ranges for {} volumes",
            got.node_vertex_ranges.len(),
            volumes.len()
        ));
    }
    let (Some(w), Some(g)) = (
        want.node_vertex_ranges.first(),
        got.node_vertex_ranges.first(),
    ) else {
        return Err("no first pad".into());
    };
    let (w, g) = (
        &want.vertices[w.start as usize..w.end as usize],
        &got.vertices[g.start as usize..g.end as usize],
    );
    if w.len() != g.len() {
        return Err(format!(
            "first pad: {} vertices against {}",
            g.len(),
            w.len()
        ));
    }
    let m = volumes[0].to_world;
    let up = [m[4], m[5], m[6]];
    for (i, (w, g)) in w.iter().zip(g).enumerate() {
        let lifted: [f32; 3] = std::array::from_fn(|k| w.position[k] + up[k] * art::HOST_PAD_LIFT);
        if (0..3).any(|k| (lifted[k] - g.position[k]).abs() > 1e-2)
            || (w.normal, w.colour, w.texcoord, w.lit, w.anim)
                != (g.normal, g.colour, g.texcoord, g.lit, g.anim)
        {
            return Err(format!("first pad vertex {i}: {g:?} against {w:?}"));
        }
    }
    // The first pad's draws, list by list: the circuit's draws of its first pad
    // node against the host's draws whose first vertex lies in its first range.
    fn lists(m: &Model) -> [&Vec<DrawCall>; 3] {
        [&m.draws, &m.alpha_tested_draws, &m.transparent_draws]
    }
    let first_node = lists(want)
        .into_iter()
        .flatten()
        .filter_map(|d| d.node)
        .min()
        .ok_or("the circuit's pads have no draw")?;
    let range = got.node_vertex_ranges[0].clone();
    for (list, (wl, gl)) in lists(want).into_iter().zip(lists(got)).enumerate() {
        let w: Vec<&DrawCall> = wl.iter().filter(|d| d.node == Some(first_node)).collect();
        let g: Vec<&DrawCall> = gl
            .iter()
            .filter(|d| range.contains(&got.indices[d.range.start as usize]))
            .collect();
        if w.len() != g.len() {
            return Err(format!(
                "list {list}: {} draws against {}",
                g.len(),
                w.len()
            ));
        }
        for (n, (w, g)) in w.iter().zip(&g).enumerate() {
            let key = |d: &DrawCall| {
                (
                    d.texture,
                    d.blend,
                    d.culled,
                    d.layer,
                    d.alpha_test_ref,
                    d.range.len(),
                    d.moving,
                )
            };
            if key(w) != key(g) {
                return Err(format!(
                    "list {list} draw {n}: {:?} against {:?}",
                    key(g),
                    key(w)
                ));
            }
        }
        for (w, g) in w.iter().zip(&g) {
            if let Some(slot) = g.texture {
                match (&want.textures[slot], &got.textures[slot]) {
                    (Some(a), Some(b)) if same_texels(a, b) => {}
                    _ => return Err(format!("texture {slot} differs")),
                }
            }
            // Rebased triangles, and the texture animation the draw's vertices name.
            let rebase = |m: &Model, d: &DrawCall, first: u32| -> Vec<u32> {
                m.indices[d.range.start as usize..d.range.end as usize]
                    .iter()
                    .map(|i| i - first)
                    .collect()
            };
            let (wf, gf) = (
                want.node_vertex_ranges[0].start,
                got.node_vertex_ranges[0].start,
            );
            if rebase(want, w, wf) != rebase(got, g, gf) {
                return Err("a draw's triangles differ".into());
            }
            let track = |m: &Model, v: &GpuVertex| {
                (v.anim as usize)
                    .checked_sub(1)
                    .map(|at| format!("{:?}", m.anim_tracks[at]))
            };
            let first =
                |m: &Model, d: &DrawCall| m.vertices[m.indices[d.range.start as usize] as usize];
            if track(want, &first(want, w)) != track(got, &first(got, g)) {
                return Err("a draw's texture animation differs".into());
            }
        }
    }
    Ok(())
}
