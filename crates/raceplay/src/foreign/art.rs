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
    Art, ArtMaterial, ArtMesh, Blend, LensFlare, Light, PadPlate, ShadowRole, Sun, TextureSource,
};
use oag_vex::vex::{self, BlendClass};

pub mod colour;
pub mod motion;

/// What of a track file's art is drawn beside its track model.
#[derive(Debug, Default)]
pub struct Extras {
    /// The circuit `.vex` entry whose `Skycube` is the sky.
    pub sky: Option<String>,
    /// The track's own sky (`[art.skybox]`), built: [`own_sky`].
    pub own_sky: Option<Model>,
    /// Pure's sun flare along the track's sun.
    pub lens_flare: Option<oag_fx::lens_flare::Setup>,
    /// Towards the sun, when the track's sun may cast: the `mapped` shadow
    /// tier's direction in place of the host title's own axis.
    pub sun_shadow: Option<[f32; 3]>,
    /// The PSP hull rig from the ambient, the sun and further directionals:
    /// [`hull_lights`]. The loader uses it where the host title's own
    /// `hull_lights` rule applies.
    pub hull: Option<HullLights>,
    /// The sun's rotation over time, when an animation turns it: the `mapped`
    /// shadows' direction and a flare that follows the sun turn with it.
    pub sun_turn: Option<motion::SunTurn>,
    /// Whether the flare follows the turning sun (it names no direction of
    /// its own).
    pub flare_turns: bool,
    /// The speed pads' plates (`oag_pad = "speedup"`): the loader's pad model.
    pub pads: Option<Model>,
    /// The weapon pads' plates (`oag_pad = "weapon"`): the loader's weapon pad
    /// model, which a mode without weapons does not draw.
    pub weapon_pads: Option<Model>,
    /// Whether these come from a track file's art section (default: not).
    pub from_art: bool,
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
        colour: if mesh.material.lit {
            colour::lit_vertex(mesh.colours[i])
        } else {
            colour::vertex(mesh.colours[i])
        },
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
///
/// Pad plates (`oag_pad`) are left out: they are [`pad_model`]'s.
#[must_use]
pub fn model(label: &str, art: &Art, textures: TextureSlots, report: &mut Vec<String>) -> Model {
    build(label, art, textures, None, report)
}

/// The plates of one pad kind as a pad model, built as [`model`] builds the
/// track's, or `None` when the art tags none.
#[must_use]
pub fn pad_model(
    label: &str,
    art: &Art,
    textures: TextureSlots,
    which: PadPlate,
    report: &mut Vec<String>,
) -> Option<Model> {
    art.meshes
        .iter()
        .any(|m| m.pad == Some(which))
        .then(|| build(label, art, textures, Some(which), report))
}

fn build(
    label: &str,
    art: &Art,
    textures: TextureSlots,
    plates: Option<PadPlate>,
    report: &mut Vec<String>,
) -> Model {
    let mut model = Model::none(label);
    let mut skipped = 0usize;
    // Pad plates never move (`oag_trackfile` refuses it), so only the track
    // model has nodes.
    let (anim_nodes, slots) = if plates.is_none() {
        motion::nodes(art, report)
    } else {
        (Vec::new(), vec![0; art.meshes.len()])
    };
    for (m, &xform) in art.meshes.iter().zip(&slots) {
        if m.pad != plates {
            continue;
        }
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
        // A moving mesh's vertices are in its node's space and ride its slot; one
        // the node table had no room for stands at its rest pose.
        let rest = m.transform.filter(|_| xform == 0).map(|t| {
            oag_rcs::rcsskeleton::local_matrix(
                t.scale,
                t.rotation,
                t.translation,
                [0.0; 3],
                [0.0; 3],
            )
        });
        model.vertices.extend((0..m.positions.len()).map(|i| {
            let mut v = vertex(m, i, glow);
            v.xform = xform;
            if let Some(rest) = &rest {
                v.position = vex::transform_point(rest, v.position);
                v.normal = vex::transform_direction(rest, v.normal);
            }
            v
        }));
        model.indices.extend(m.indices.iter().map(|&i| base + i));
        let draw = DrawCall {
            range: first..model.indices.len() as u32,
            texture,
            bounds: bounds(&m.positions),
            // Drawn unculled, as a disc circuit's moving draws are: its bounds
            // are in its node's space.
            moving: xform != 0,
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
    model.anim_nodes = anim_nodes;
    // Where the art reaches: what stands still, and what moves at each key.
    let reach: Vec<[f32; 3]> = model
        .vertices
        .iter()
        .filter(|v| v.xform == 0)
        .map(|v| v.position)
        .chain(motion::swept_positions(&model))
        .collect();
    let whole = bounds(&reach);
    model.centre = whole.centre;
    // `mesh::build_with_textures`'s model radius: the widest half-extent.
    model.radius = reach
        .iter()
        .fold([0.0f32; 3], |r, p| {
            std::array::from_fn(|k| r[k].max((p[k] - whole.centre[k]).abs()))
        })
        .into_iter()
        .fold(0.0f32, f32::max)
        .max(0.001);
    report.push(format!(
        "art{}: {} mesh(es), {} triangle(s), {} texture(s){}",
        match plates {
            None => "",
            Some(PadPlate::Speedup) => " speed pad plates",
            Some(PadPlate::Weapon) => " weapon pad plates",
        },
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
    let own_sky = own_sky(art, textures, report);
    if art.sky.is_none() && own_sky.is_none() {
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
    let sun_turn = motion::SunTurn::of(art);
    if sun_turn.is_some() {
        report.push(
            "art: the sun turns with its animation; the mapped shadows and a flare that follows              it turn too, the baked colours and the hull rig keep its pose at time zero"
                .into(),
        );
    }
    let lens_flare = lens_flare(art, textures, report);
    let pads = pad_model(
        "pad plates",
        art,
        textures.clone(),
        PadPlate::Speedup,
        report,
    );
    let weapon_pads = pad_model(
        "weapon pad plates",
        art,
        textures.clone(),
        PadPlate::Weapon,
        report,
    );
    Extras {
        from_art: true,
        pads,
        weapon_pads,
        sky: art.sky.clone(),
        own_sky,
        flare_turns: sun_turn.is_some()
            && lens_flare.is_some()
            && art.lens_flare.is_some_and(|f| f.direction.is_none()),
        lens_flare,
        sun_shadow,
        hull,
        sun_turn,
    }
}

/// The track's own sky (`[art.skybox]`, ADR-0076 decision 14) on the
/// camera-centred cube HD's `sky.gtf` draws on (`mesh::sky_cube::own`): each
/// face its texture row's picture, or the gradient from the file's linear
/// colours converted the way a vertex colour is ([`colour::vertex`]). `None`,
/// with the reason in `report`, when the file gives none, or when a face's
/// texture did not resolve and there is no gradient to stand in for it.
pub fn own_sky(art: &Art, textures: &TextureSlots, report: &mut Vec<String>) -> Option<Model> {
    let sky = art.skybox?;
    let gradient = sky.gradient().map(|(zenith, horizon)| {
        let display = |c: [f32; 3]| {
            let v = colour::vertex([c[0], c[1], c[2], 1.0]);
            [v[0], v[1], v[2]]
        };
        (display(zenith), display(horizon))
    });
    let mut missing = 0usize;
    let faces: [Option<std::sync::Arc<ModelTexture>>; 6] = std::array::from_fn(|i| {
        let picture = sky.faces[i].and_then(|t| textures.get(t).cloned().flatten());
        missing += usize::from(sky.faces[i].is_some() && picture.is_none());
        picture
    });
    if missing > 0 && gradient.is_none() {
        report.push(format!(
            "art skybox: {missing} face texture(s) did not resolve and there is no gradient; \
             the sky stays black"
        ));
        return None;
    }
    let pictures = faces.iter().filter(|f| f.is_some()).count();
    report.push(format!(
        "art skybox: the track's own sky, {pictures} face picture(s){}{} on a camera-centred cube",
        if gradient.is_some() {
            format!(", {} face(s) of zenith-to-horizon gradient", 6 - pictures)
        } else {
            String::new()
        },
        if missing > 0 {
            format!(" ({missing} face texture(s) did not resolve, drawn as the gradient)")
        } else {
            String::new()
        }
    ));
    Some(mesh::sky_cube::own("art skybox", faces, gradient))
}
/// The weapon pad plates for the loader's weapon pad model, reported as the
/// disc path reports its own: decoded either way, drawn only where the mode
/// has weapons (`Scene::new` holds them back otherwise).
pub fn weapon_pads(
    extras: &mut Extras,
    weapons_on: bool,
    report: &mut Vec<String>,
) -> Option<Model> {
    let pads = extras.weapon_pads.take();
    report.push(match &pads {
        Some(p) => format!(
            "{} the track file's weapon pad plates: {} triangle(s)",
            if weapons_on {
                "drawing"
            } else {
                "decoded but not drawing (weapons off in this mode)"
            },
            p.indices.len() / 3
        ),
        None if extras.from_art => {
            "the track file tags no weapon pad plate (oag_pad); none is drawn".into()
        }
        None => return None,
    });
    pads
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
                        colour::exact_linear(r),
                        colour::exact_linear(g),
                        colour::exact_linear(b),
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
            transform: None,
            pad: None,
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
