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
use oag_trackfile::art::{
    Art, ArtMaterial, ArtMesh, Blend, LensFlare, ShadowRole, Sun, TextureSource,
};
use oag_vex::vex::{self, BlendClass};

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
        colour: mesh.colours[i],
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

/// The track model of `art`'s meshes, one draw each in file order, over the
/// resolved `textures`. A mesh whose texture did not resolve is left out and
/// counted in `report`.
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
            layer: vex::LAYER_DEFAULT,
            depth_point: None,
            alpha_test_ref: None,
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
    let sun = art.sun?;
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
        "art lens flare: along the track's sun ({:.3}, {:.3}, {:.3}), atlas {}x{}",
        sun.direction[0], sun.direction[1], sun.direction[2], texture.width, texture.height
    ));
    Some(oag_fx::lens_flare::Setup {
        direction: sun.direction,
        colour: flare.colour.unwrap_or(sun.colour),
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
            "art: the sun ({:.3}, {:.3}, {:.3}) sets the mapped shadows' direction; the host's \
             own hull lighting is kept (the authored rig is HD's equation)",
            s.direction[0], s.direction[1], s.direction[2]
        ),
    });
    if art.ambient.is_some() {
        report.push(
            "art: an ambient colour, read and not drawn: the host's own rig lights the hulls"
                .into(),
        );
    }
    Extras {
        sky: art.sky.clone(),
        lens_flare: lens_flare(art, textures, report),
        sun_shadow,
    }
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

/// A disc circuit's track model as art: one mesh per draw in the order the
/// three lists dispatch them, its own block of vertices, textures as host
/// references to `entry`'s own `Texture` nodes by `names`.
///
/// Geometry under an `Anim Transform` is placed at its time-zero pose (the
/// format carries no motion yet); a texture node with no name is written as a
/// reference that resolves to nothing. **Local use only**: what this returns is
/// disc content.
#[must_use]
pub fn from_model(model: &Model, entry: &str, names: &[Option<String>]) -> Art {
    let poses = model.sample_anim_nodes(0.0);
    let mut meshes = Vec::new();
    for (list, draws) in [
        &model.draws,
        &model.alpha_tested_draws,
        &model.transparent_draws,
    ]
    .into_iter()
    .enumerate()
    {
        for (n, draw) in draws.iter().enumerate() {
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
                colours: block.iter().map(|v| v.colour).collect(),
                indices: indices.iter().map(|&i| i - lo).collect(),
                material: ArtMaterial {
                    texture: draw.texture,
                    blend: blend_of(list, draw),
                    culled: draw.culled,
                    lit: block.first().is_some_and(|v| v.lit != 0.0),
                },
                shadow: ShadowRole::Both,
            });
        }
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

/// Pure's flare node as the track's sun and its flare: the sun casts nothing
/// (Pure's shadows are not cast from it), and the atlas is the circuit's own
/// `LensflareComponents.tga` slot.
pub fn flare_as_sun(art: &mut Art, flare: &oag_fx::lens_flare::Setup, names: &[Option<String>]) {
    let atlas = names.iter().position(|n| {
        n.as_deref()
            .is_some_and(|n| spells("LensflareComponents.tga", n))
    });
    art.sun = Some(Sun {
        direction: flare.direction,
        colour: flare.colour,
        intensity: 1.0,
        angular_diameter: 0.0,
        shadows: false,
    });
    art.lens_flare = Some(LensFlare {
        colour: None,
        atlas,
    });
}

#[cfg(test)]
mod tests;
