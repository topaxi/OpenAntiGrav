//! A track file's art (ADR-0076) as what the renderer takes, and a disc
//! circuit's track model back into art for the local exporter.
//!
//! The renderer's inputs are the spec: an [`Art`] mesh becomes one [`DrawCall`]
//! over its own block of [`GpuVertex`]es, in the draw list its blend class
//! names, so `oag-mesh` and the render-side crates learn nothing of the format
//! (`just check-deps`, rule 3).
//!
//! **A texture that does not resolve draws nothing.** A host reference names a
//! `Texture` node of a circuit on the host title's disc; if the entry or the
//! name is not there, or an image beside the manifest will not decode, the
//! meshes that name it are left out and the report says which (CLAUDE.md,
//! "Never invent what the assets already author").

use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Arc;

use oag_assets::Archives;
use oag_mesh::mesh::{self, Bounds, DrawCall, GpuVertex, Model, ModelTexture, TextureSlots};
use oag_mesh::mesh_render::{HULL_LIGHTS, HullLights, ge_byte};
use oag_trackfile::art::{
    Art, ArtMaterial, ArtMesh, Blend, LensFlare, Light, ShadowRole, Sun, TextureSource,
};
use oag_vex::vex::{self, BlendClass};

pub mod colour;

/// What of a track file's art is drawn beside its track model.
#[derive(Debug, Default)]
pub struct Extras {
    /// The circuit `.vex` entry whose `Skycube` is the sky.
    pub sky: Option<String>,
    /// Pure's sun flare along the track's sun.
    pub lens_flare: Option<oag_fx::lens_flare::Setup>,
    /// Towards the sun, when the track's sun may cast: the `mapped` shadow
    /// tier's direction in place of the host title's own axis.
    pub sun_shadow: Option<[f32; 3]>,
    /// The PSP hull rig from the ambient, the sun and further directionals:
    /// [`hull_lights`]. The loader uses it where the host title's own
    /// `hull_lights` rule applies.
    pub hull: Option<HullLights>,
}

/// A host circuit's texture table: one name and one decoded slot per
/// `Texture` node, in node order.
struct HostTable {
    names: Vec<Option<String>>,
    slots: TextureSlots,
}

/// Each `Texture` node's runtime asset path, in node order: the name a host
/// reference spells. `None` for a node that carries none.
///
/// # Errors
///
/// A file whose class table or node tree will not read.
pub fn texture_names(blob: &[u8]) -> anyhow::Result<Vec<Option<String>>> {
    let classes = vex::classes_of(blob)?;
    let nodes = vex::nodes(blob)?;
    Ok(nodes
        .iter()
        .filter(|n| Some(n.class_id) == classes.texture)
        .map(|n| blob.get(n.payload()).and_then(vex::texture_asset_path))
        .collect())
}

/// A circuit's decoded texture slots, the way the race loader builds them: the
/// embedded block, or on a PS2 file (whose block is empty) the texture set the
/// archive keeps before the entry.
fn texture_slots(
    archives: &mut Archives,
    entry: &str,
    blob: &[u8],
) -> anyhow::Result<TextureSlots> {
    let slots = mesh::embedded_texture_slots(blob)?;
    if slots.is_empty() || slots.iter().any(Option::is_some) {
        return Ok(slots);
    }
    let Some(set) = oag_livery::entry::ps2_texture_set(archives, entry) else {
        return Ok(slots);
    };
    let classes = vex::classes_of(blob)?;
    let nodes = vex::nodes(blob)?;
    Ok(mesh::resolve_texture_slots(
        blob,
        &nodes,
        classes.texture,
        &set,
    ))
}

/// Whether `reference` names `name`: the whole asset path without case, or its
/// file name alone when the reference carries no directory.
fn spells(reference: &str, name: &str) -> bool {
    if reference.contains(['\\', '/']) {
        return reference.eq_ignore_ascii_case(name);
    }
    name.rsplit(['\\', '/'])
        .next()
        .is_some_and(|file| file.eq_ignore_ascii_case(reference))
}

/// Resolves the texture rows of `art` a mesh or the flare names, positionally:
/// `None` where a named row does not resolve, with the reason in `report`, and
/// `None` without a word for a row nothing names.
pub fn resolve_textures(
    art: &Art,
    manifest_dir: &Path,
    archives: &mut Archives,
    report: &mut Vec<String>,
) -> TextureSlots {
    let mut hosts: BTreeMap<String, Option<HostTable>> = BTreeMap::new();
    let named: std::collections::BTreeSet<usize> = art
        .meshes
        .iter()
        .filter_map(|m| m.material.texture)
        .chain(art.lens_flare.and_then(|f| f.atlas))
        .collect();
    let mut slots = Vec::with_capacity(art.textures.len());
    for (i, source) in art.textures.iter().enumerate() {
        if !named.contains(&i) {
            slots.push(None);
            continue;
        }
        let slot = match source {
            TextureSource::Image(file) => match image(&manifest_dir.join(file)) {
                Ok(texture) => Some(Arc::new(texture)),
                Err(e) => {
                    report.push(format!("art texture {i}: {file}: {e}; draws nothing"));
                    None
                }
            },
            TextureSource::Host { entry, name } => {
                let table = hosts.entry(entry.clone()).or_insert_with(|| {
                    let read = archives
                        .read_name(entry)
                        .map_err(anyhow::Error::from)
                        .and_then(|blob| {
                            Ok(HostTable {
                                names: texture_names(&blob)?,
                                slots: texture_slots(archives, entry, &blob)?,
                            })
                        });
                    read.map_err(|e| {
                        report.push(format!(
                            "art: host circuit {entry} does not read on this disc ({e}); \
                             every texture named in it draws nothing"
                        ));
                    })
                    .ok()
                });
                let found = table.as_ref().and_then(|t| {
                    let mut hits = t
                        .names
                        .iter()
                        .enumerate()
                        .filter(|(_, n)| n.as_deref().is_some_and(|n| spells(name, n)))
                        .map(|(at, _)| at);
                    let first = hits.next()?;
                    let more = hits.count();
                    if more > 0 {
                        report.push(format!(
                            "art texture {i}: {name} names {} nodes of {entry}; the first is drawn",
                            more + 1
                        ));
                    }
                    t.slots.get(first).cloned().flatten()
                });
                if found.is_none() && table.is_some() {
                    report.push(format!(
                        "art texture {i}: {entry} has no decodable texture {name}; draws nothing"
                    ));
                }
                found
            }
        };
        slots.push(slot);
    }
    slots
}

/// An author's PNG as an RGBA8 texture whose mip chain the uploader derives.
fn image(path: &Path) -> anyhow::Result<ModelTexture> {
    let file = std::fs::File::open(path)?;
    let mut decoder = png::Decoder::new(std::io::BufReader::new(file));
    decoder.set_transformations(png::Transformations::normalize_to_color8());
    let mut reader = decoder.read_info()?;
    let mut buf = vec![0; reader.output_buffer_size().unwrap_or(0)];
    let frame = reader.next_frame(&mut buf)?;
    let pixels = &buf[..frame.buffer_size()];
    let rgba: Vec<u8> = match frame.color_type {
        png::ColorType::Rgba => pixels.to_vec(),
        png::ColorType::Rgb => pixels
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|&[r, g, b]| [r, g, b, 0xff])
            .collect(),
        png::ColorType::GrayscaleAlpha => pixels
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|&[g, a]| [g, g, g, a])
            .collect(),
        png::ColorType::Grayscale => pixels.iter().flat_map(|&g| [g, g, g, 0xff]).collect(),
        png::ColorType::Indexed => anyhow::bail!("an indexed PNG survived expansion"),
    };
    let label = path
        .file_name()
        .map_or_else(String::new, |f| f.to_string_lossy().into_owned());
    Ok(ModelTexture::rgba8(
        label,
        frame.width,
        frame.height,
        rgba,
        None,
    ))
}

/// The draw list a blend class goes in and the transparent equation it names.
fn list_and_class(blend: Blend) -> (usize, Option<BlendClass>) {
    match blend {
        Blend::Opaque => (0, None),
        Blend::Cutout => (1, None),
        Blend::Alpha => (2, Some(BlendClass::AlphaOver)),
        Blend::Additive => (2, Some(BlendClass::Additive)),
        Blend::Unblended => (2, Some(BlendClass::None)),
    }
}

/// The AABB's circumscribing sphere, the arithmetic `mesh::build_with_textures`
/// bounds a batch with.
fn bounds(positions: &[[f32; 3]]) -> Bounds {
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for p in positions {
        for k in 0..3 {
            lo[k] = lo[k].min(p[k]);
            hi[k] = hi[k].max(p[k]);
        }
    }
    let centre = [
        (lo[0] + hi[0]) * 0.5,
        (lo[1] + hi[1]) * 0.5,
        (lo[2] + hi[2]) * 0.5,
    ];
    let radius = (0..3)
        .map(|k| (hi[k] - centre[k]).powi(2))
        .sum::<f32>()
        .sqrt();
    Bounds { centre, radius }
}

/// One vertex as a Pulse or Pure track batch carries it: baked light in the
/// colour, nothing animated, the default material slots.
fn vertex(mesh: &ArtMesh, i: usize, glow: f32) -> GpuVertex {
    GpuVertex {
        position: mesh.positions[i],
        normal: mesh.normals[i],
        colour: colour::vertex(mesh.colours[i]),
        texcoord: mesh.texcoords[i],
        lit: if mesh.material.lit { 1.0 } else { 0.0 },
        anim: 0,
        lightmap_texcoord: [0.0, 0.0],
        xform: 0,
        sun_mask: 1.0,
        slots: mesh::slots::DEFAULT,
        specular_exponent: mesh::DEFAULT_SPECULAR_EXPONENT,
        glow,
        texcoord2: [0.0, 0.0],
        quake: 0.0,
        tangent: 0,
    }
}

/// The track model of `art`'s meshes, one draw each, over the resolved
/// `textures`: vertices in file order, each list's draws by layer and then by
/// file order (`Model::sort_by_layer`, a stable sort), colours converted to the
/// gamma bytes the renderer draws. A mesh whose texture did not resolve is left
/// out and counted in `report`.
#[must_use]
pub fn model(label: &str, art: &Art, textures: TextureSlots, report: &mut Vec<String>) -> Model {
    let mut model = Model::none(label);
    let mut skipped = 0usize;
    for m in &art.meshes {
        let texture = match m.material.texture {
            None => None,
            Some(t) if textures.get(t).is_some_and(Option::is_some) => Some(t),
            Some(_) => {
                skipped += 1;
                continue;
            }
        };
        let (list, blend) = list_and_class(m.material.blend);
        let glow = if list == 2 {
            0.0
        } else {
            f32::from(mesh::glow::BASE) / 255.0
        };
        let base = model.vertices.len() as u32;
        let first = model.indices.len() as u32;
        model
            .vertices
            .extend((0..m.positions.len()).map(|i| vertex(m, i, glow)));
        model.indices.extend(m.indices.iter().map(|&i| base + i));
        let draw = DrawCall {
            range: first..model.indices.len() as u32,
            texture,
            bounds: bounds(&m.positions),
            moving: false,
            culled: m.material.culled,
            blend,
            blend_state: None,
            layer: m.layer,
            depth_point: None,
            // The `.vex` path's expression (`oag_mesh::mesh`), so a disc
            // export's reference comes back bit for bit.
            alpha_test_ref: m.material.alpha_ref.map(|r| f32::from(r) / 255.0),
            writes_depth: false,
            node: None,
            chunk: None,
        };
        [
            &mut model.draws,
            &mut model.alpha_tested_draws,
            &mut model.transparent_draws,
        ][list]
            .push(draw);
        model.mesh_count += 1;
    }
    model.sort_by_layer();
    model.textures = textures;
    let whole = bounds(
        &model
            .vertices
            .iter()
            .map(|v| v.position)
            .collect::<Vec<_>>(),
    );
    model.centre = whole.centre;
    // `mesh::build_with_textures`'s model radius: the widest half-extent.
    model.radius = model
        .vertices
        .iter()
        .fold([0.0f32; 3], |r, v| {
            std::array::from_fn(|k| r[k].max((v.position[k] - whole.centre[k]).abs()))
        })
        .into_iter()
        .fold(0.0f32, f32::max)
        .max(0.001);
    report.push(format!(
        "art: {} mesh(es), {} triangle(s), {} texture(s){}",
        model.mesh_count,
        model.indices.len() / 3,
        model.textures.iter().filter(|t| t.is_some()).count(),
        if skipped == 0 {
            String::new()
        } else {
            format!("; {skipped} mesh(es) not drawn, their texture did not resolve")
        }
    ));
    model
}

/// Pure's flare from the track's sun and its atlas slot, or `None` with the
/// reason in `report`.
#[must_use]
pub fn lens_flare(
    art: &Art,
    textures: &TextureSlots,
    report: &mut Vec<String>,
) -> Option<oag_fx::lens_flare::Setup> {
    let flare = art.lens_flare?;
    let direction = flare.direction.or(art.sun.map(|s| s.direction))?;
    let linear = flare.colour.or(art.sun.map(|s| s.colour))?;
    let Some(texture) = flare.atlas.and_then(|a| textures.get(a).cloned().flatten()) else {
        report.push("art lens flare: its atlas did not resolve, none drawn".into());
        return None;
    };
    let levels = match &texture.texels {
        mesh::Texels::Chain(levels) => levels.clone(),
        mesh::Texels::Rgba8(base) => vec![base.clone()],
        _ => {
            report.push("art lens flare: the atlas is not an RGBA picture, none drawn".into());
            return None;
        }
    };
    report.push(format!(
        "art lens flare: along ({:.3}, {:.3}, {:.3}){}, atlas {}x{}",
        direction[0],
        direction[1],
        direction[2],
        if flare.direction.is_some() {
            ", its own direction"
        } else {
            ", the track's sun"
        },
        texture.width,
        texture.height
    ));
    Some(oag_fx::lens_flare::Setup {
        direction,
        // Linear in the file; the ring law steps the gamma colour.
        colour: linear.map(colour::to_gamma),
        atlas: oag_fx::lens_flare::Atlas {
            width: texture.width,
            height: texture.height,
            levels,
        },
    })
}

/// Everything of `art` beside the track model, reported part by part.
#[must_use]
pub fn extras(art: &Art, textures: &TextureSlots, report: &mut Vec<String>) -> Extras {
    if art.sky.is_none() {
        report.push("art: no sky named; the sky stays black".into());
    }
    let sun_shadow = art.sun.filter(|s| s.shadows).map(|s| s.direction);
    report.push(match art.sun {
        None => "art: no sun; shadows keep the host title's own axis".into(),
        Some(s) if !s.shadows => {
            "art: a sun that casts nothing; shadows keep the host title's own axis".into()
        }
        Some(s) => format!(
            "art: the sun ({:.3}, {:.3}, {:.3}) sets the mapped shadows' direction",
            s.direction[0], s.direction[1], s.direction[2]
        ),
    });
    let hull = hull_lights(art);
    report.push(match &hull {
        Some(h) => format!(
            "art hull lights: the PSP rig from the ambient, the sun and {} further \
             directional(s), ambient {:?}, diffuse {:?} (drawn where the host title lights hulls \
             this way)",
            art.lights.len(),
            h.ambient,
            h.diffuse
        ),
        None => {
            "art hull lights: no ambient, sun or directional; hulls keep the stand-in rig".into()
        }
    });
    Extras {
        sky: art.sky.clone(),
        lens_flare: lens_flare(art, textures, report),
        sun_shadow,
        hull,
    }
}

/// The GE light list Pulse and Pure on the PSP light a hull with
/// (`oag_mesh::mesh_render::HullLights`), from the track's light: the ambient,
/// then the sun as directional 0 and the directional `[[art.light]]` rows after
/// it, four at most. Each colour is linear times its intensity, converted to
/// the gamma byte and clipped (`colour::gamma_byte`), and built at the bare
/// scale `256` as `HullLights::from_track` builds a circuit's. A direction is
/// taken as written (the format holds it to unit length). `None` when the file
/// authors none of the three, as `from_track` answers a circuit with no light.
#[must_use]
pub fn hull_lights(art: &Art) -> Option<HullLights> {
    let directionals: Vec<([f32; 3], [f32; 3])> = art
        .sun
        .map(|s| (s.direction, s.colour.map(|c| c * s.intensity)))
        .into_iter()
        .chain(art.lights.iter().map(|l| match *l {
            Light::Directional {
                direction,
                colour,
                intensity,
            } => (direction, colour.map(|c| c * intensity)),
        }))
        .collect();
    if art.ambient.is_none() && directionals.is_empty() {
        return None;
    }
    let channel = |linear: f32| ge_byte(colour::gamma_byte(linear), 256);
    let ambient = art.ambient.unwrap_or([0.0; 3]).map(channel);
    let mut out = HullLights::OFF;
    out.ambient = [ambient[0], ambient[1], ambient[2], 1.0];
    for (slot, (d, c)) in directionals.into_iter().take(HULL_LIGHTS).enumerate() {
        out.direction[slot] = [d[0], d[1], d[2], 1.0];
        let c = c.map(channel);
        out.diffuse[slot] = [c[0], c[1], c[2], 0.0];
    }
    Some(out)
}

/// Which blend class a built draw is, from the list it is in.
fn blend_of(list: usize, draw: &DrawCall) -> Blend {
    match (list, draw.blend) {
        (0, _) => Blend::Opaque,
        (1, _) => Blend::Cutout,
        (_, Some(BlendClass::Additive)) => Blend::Additive,
        (_, Some(BlendClass::None)) => Blend::Unblended,
        _ => Blend::Alpha,
    }
}

/// A disc circuit's track model as art: one mesh per draw in vertex-buffer
/// order (the order the `.vex` build appends them), each with its own block of
/// vertices, its layer and its alpha-test reference, so the loader's stable
/// sort by layer gives back each list's dispatch order. Colours are written
/// linear; textures are host references to `entry`'s own `Texture` nodes by
/// `names`.
///
/// Geometry under an `Anim Transform` is placed at its time-zero pose (the
/// format carries no motion yet); a texture node with no name is written as a
/// reference that resolves to nothing. **Local use only**: what this returns is
/// disc content.
#[must_use]
pub fn from_model(model: &Model, entry: &str, names: &[Option<String>]) -> Art {
    let poses = model.sample_anim_nodes(0.0);
    let mut meshes = Vec::new();
    let mut draws: Vec<(usize, usize, &DrawCall)> = [
        &model.draws,
        &model.alpha_tested_draws,
        &model.transparent_draws,
    ]
    .into_iter()
    .enumerate()
    .flat_map(|(list, draws)| draws.iter().enumerate().map(move |(n, d)| (list, n, d)))
    .collect();
    draws.sort_by_key(|(_, _, d)| d.range.start);
    for (list, n, draw) in draws {
        let indices = &model.indices[draw.range.start as usize..draw.range.end as usize];
        let lo = indices.iter().copied().min().unwrap_or(0);
        let hi = indices.iter().copied().max().unwrap_or(0);
        let block = &model.vertices[lo as usize..=hi as usize];
        let placed = |v: &GpuVertex| {
            let pose = (v.xform as usize)
                .checked_sub(1)
                .and_then(|at| poses.get(at));
            match pose {
                None => (v.position, v.normal),
                Some(m) => {
                    let n = vex::transform_direction(m, v.normal);
                    let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
                    let n = if len > 0.0 { n.map(|c| c / len) } else { n };
                    (vex::transform_point(m, v.position), n)
                }
            }
        };
        let (positions, normals) = block.iter().map(placed).unzip();
        meshes.push(ArtMesh {
            name: format!("{} {n}", ["opaque", "cutout", "transparent"][list]),
            positions,
            normals,
            texcoords: block.iter().map(|v| v.texcoord).collect(),
            colours: block
                .iter()
                .map(|v| {
                    let [r, g, b, a] = v.colour;
                    [
                        colour::to_linear(r),
                        colour::to_linear(g),
                        colour::to_linear(b),
                        a,
                    ]
                })
                .collect(),
            indices: indices.iter().map(|&i| i - lo).collect(),
            material: ArtMaterial {
                texture: draw.texture,
                blend: blend_of(list, draw),
                culled: draw.culled,
                lit: block.first().is_some_and(|v| v.lit != 0.0),
                alpha_ref: draw
                    .alpha_test_ref
                    .map(|r| (r * 255.0).round().clamp(0.0, 255.0) as u8),
            },
            shadow: ShadowRole::Both,
            layer: draw.layer,
        });
    }
    let textures = (0..model.textures.len())
        .map(|i| TextureSource::Host {
            entry: entry.to_string(),
            name: names
                .get(i)
                .cloned()
                .flatten()
                .unwrap_or_else(|| format!("unnamed texture node {i}")),
        })
        .collect();
    Art {
        meshes,
        textures,
        ..Art::default()
    }
}

/// A circuit's own light nodes as the track's: the `AmbientLight`s summed as
/// `HullLights::from_track` sums them, the first `DirectionalLight` as the sun
/// and every further one as a directional `[[art.light]]` row. Each colour is
/// the GE byte the PSP packs (`min(255, trunc(c * 255))`), written linear, so
/// the rig [`hull_lights`] builds from the file equals the circuit's; each
/// direction is row 2 of the node's world matrix normalised as `from_track`
/// does it. The sun casts nothing: a disc's shadows follow the host title's own
/// axis, not its light nodes. A node whose row is zero length is skipped.
pub fn lights_from_track(art: &mut Art, blob: &[u8], nodes: &[vex::Node]) {
    use oag_vex::lighting;
    let byte = |c: f32| colour::to_linear(f32::from(((c * 255.0) as u32).min(255) as u8) / 255.0);
    let ambients = lighting::ambient_lights(blob, nodes);
    if !ambients.is_empty() {
        let sum = ambients.iter().fold([0.0f32; 3], |acc, light| {
            [
                acc[0] + light.colour[0],
                acc[1] + light.colour[1],
                acc[2] + light.colour[2],
            ]
        });
        art.ambient = Some(sum.map(byte));
    }
    for light in lighting::directional_lights(blob, nodes) {
        let row = [light.to_world[8], light.to_world[9], light.to_world[10]];
        let length = (row[0] * row[0] + row[1] * row[1] + row[2] * row[2]).sqrt();
        if length <= f32::EPSILON {
            continue;
        }
        let direction = [row[0] / length, row[1] / length, row[2] / length];
        let colour = light.colour.map(byte);
        if art.sun.is_none() {
            art.sun = Some(Sun {
                direction,
                colour,
                intensity: 1.0,
                angular_diameter: 0.0,
                shadows: false,
            });
        } else {
            art.lights.push(Light::Directional {
                direction,
                colour,
                intensity: 1.0,
            });
        }
    }
}

/// Pure's flare node as the track's flare, the atlas the circuit's own
/// `LensflareComponents.tga` slot. Along the sun when the circuit's first
/// directional is the flare's direction and colour; otherwise the flare carries
/// its own (both exact through the linear curve, `colour::exact_linear`). With
/// no sun yet, the flare becomes the sun, casting nothing (Pure's shadows are
/// not cast from it).
pub fn flare_as_sun(art: &mut Art, flare: &oag_fx::lens_flare::Setup, names: &[Option<String>]) {
    let atlas = names.iter().position(|n| {
        n.as_deref()
            .is_some_and(|n| spells("LensflareComponents.tga", n))
    });
    let linear = flare.colour.map(colour::exact_linear);
    let sun = *art.sun.get_or_insert(Sun {
        direction: flare.direction,
        colour: linear,
        intensity: 1.0,
        angular_diameter: 0.0,
        shadows: false,
    });
    art.lens_flare = Some(LensFlare {
        direction: (sun.direction != flare.direction).then_some(flare.direction),
        colour: (sun.colour.map(colour::to_gamma) != flare.colour).then_some(linear),
        atlas,
    });
}

#[cfg(test)]
mod tests;
