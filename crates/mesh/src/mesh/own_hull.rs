//! A hull of this project's own drawn on a host `.vex` model: our geometry, the host's
//! textures, materials and render state (`docs/formats/own-hull.md`).
//!
//! The host is built as usual ([`super::build_with_textures`]); [`substitute`] then replaces
//! the geometry of its mesh nodes with the parts an [`OwnHull`] names. A part is keyed by the
//! host mesh node's name, a surface by the host texture's file name. Everything a batch carries
//! besides geometry (list, blend, cull, lighting, glow stamp, texture animation, the `0x2000`
//! shine twin, the LOD node) is copied from the host draw the surface lands on.

use oag_core::math::{Mat3, Mat4, Vec3};
use oag_vex::vex;

use super::{Bounds, DrawCall, Flap, GpuVertex, LodGroups, Model};

mod read;
pub use read::{ReadError, parse};

/// A hull's geometry and anchors, in the host model's own space (Y up, `+Z` forward, engine
/// units).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OwnHull {
    /// One per host mesh node drawn.
    pub parts: Vec<Part>,
    /// Anchors that replace the host locator of the same name.
    pub locators: Vec<Locator>,
    /// The shadow occluder, triangles in model space, when the file authors one.
    pub shadow: Option<Shadow>,
    /// The images the file carries. The format allows them for a hull with textures of its own;
    /// none is read, and the host's textures paint every surface.
    pub images: usize,
}

/// One host mesh node's replacement geometry.
#[derive(Debug, Clone, PartialEq)]
pub struct Part {
    /// The host mesh node's name, e.g. `shipShape`.
    pub name: String,
    /// Model space from the part's own space (column-major, the layout a `.vex` matrix has).
    /// On an airbrake it is the flap's hinge.
    pub transform: [f32; 16],
    /// One per host texture the part is painted with.
    pub surfaces: Vec<Surface>,
}

/// Triangles painted with one host texture, in the part's own space.
#[derive(Debug, Clone, PartialEq)]
pub struct Surface {
    /// The host texture's file name, e.g. `texture1.tga`.
    pub texture: String,
    /// Vertex positions.
    pub positions: Vec<[f32; 3]>,
    /// Unit vertex normals, beside `positions`.
    pub normals: Vec<[f32; 3]>,
    /// Texture coordinates, `v` down, beside `positions`.
    pub uvs: Vec<[f32; 2]>,
    /// Vertex colours, beside `positions`, or `None` for the host's own tint.
    pub colours: Option<Vec<[u8; 4]>>,
    /// Counter-clockwise triangles.
    pub triangles: Vec<[u32; 3]>,
}

/// A named anchor.
#[derive(Debug, Clone, PartialEq)]
pub struct Locator {
    /// The host locator's name, e.g. `engine_flare`.
    pub name: String,
    /// Its pose in model space, laid out as a `.vex` locator matrix.
    pub transform: [f32; 16],
}

/// A shadow occluder's triangles, model space.
#[derive(Debug, Clone, PartialEq)]
pub struct Shadow {
    /// Vertex positions.
    pub positions: Vec<[f32; 3]>,
    /// Counter-clockwise triangles.
    pub triangles: Vec<[u32; 3]>,
}

impl OwnHull {
    /// The anchor named `name` (ignoring case), if the file places one.
    #[must_use]
    pub fn locator(&self, name: &str) -> Option<&Locator> {
        self.locators
            .iter()
            .find(|locator| fold(&locator.name) == fold(name))
    }

    /// The anchors whose name starts with `prefix` (ignoring case), in file order.
    pub fn locators_starting(&self, prefix: &str) -> impl Iterator<Item = &Locator> {
        let prefix = fold(prefix);
        self.locators
            .iter()
            .filter(move |locator| fold(&locator.name).starts_with(&prefix))
    }

    /// The vertex and triangle counts over every part.
    #[must_use]
    pub fn counts(&self) -> (usize, usize) {
        self.parts
            .iter()
            .flat_map(|part| &part.surfaces)
            .fold((0, 0), |(v, t), s| {
                (v + s.positions.len(), t + s.triangles.len())
            })
    }
}

/// Whether `hull` is meant for the `.vex` `blob` ([`misfits`] is empty).
#[must_use]
pub fn fits(blob: &[u8], hull: &OwnHull) -> bool {
    misfits(blob, hull).is_empty()
}

/// What keeps `hull` off the `.vex` `blob`, by name: a part naming none of its mesh nodes, or
/// a part none of whose surfaces names one of its textures. Empty where the file is meant for
/// the model. A single surface naming a missing texture does not keep the file off; it is not
/// drawn and [`substitute`] reports it. Pulse PS2's `Zone.vex` shares a few node names with the
/// PSP's and none of its textures, so the PSP file does not fit it.
#[must_use]
pub fn misfits(blob: &[u8], hull: &OwnHull) -> Vec<String> {
    let (Ok(nodes), Ok(classes)) = (vex::nodes(blob), vex::classes_of(blob)) else {
        return vec!["the model's node tree does not read".to_owned()];
    };
    let names_of = |class: Option<u32>| -> Vec<String> {
        nodes
            .iter()
            .filter(|node| Some(node.class_id) == class)
            .filter_map(|node| node.name.as_deref().map(fold))
            .collect()
    };
    let (meshes, textures) = (names_of(classes.mesh), names_of(classes.texture));
    if hull.parts.is_empty() {
        return vec!["no part".to_owned()];
    }
    let mut out = Vec::new();
    for part in &hull.parts {
        if !meshes.contains(&fold(&part.name)) {
            out.push(format!("no mesh node {}", part.name));
        } else if !part
            .surfaces
            .iter()
            .any(|surface| textures.contains(&fold(&surface.texture)))
        {
            let names: Vec<&str> = part.surfaces.iter().map(|s| s.texture.as_str()).collect();
            out.push(format!("{}: no texture {}", part.name, names.join(", ")));
        }
    }
    out
}

/// A name as the host is matched on it: no Blender `.NNN` suffix, no directory, lower case.
#[must_use]
pub fn fold(name: &str) -> String {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let base = match base.rsplit_once('.') {
        Some((stem, digits)) if digits.len() == 3 && digits.bytes().all(|b| b.is_ascii_digit()) => {
            stem
        }
        _ => base,
    };
    base.to_ascii_lowercase()
}

/// Which of a model's three draw lists a draw is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum List {
    Opaque,
    Cutout,
    Blended,
}

/// A host draw a surface can land on: its list, its state, one of its vertices (the per-batch
/// vertex fields are constant over a batch) and its shine twin's texture.
struct Template {
    list: List,
    draw: DrawCall,
    vertex: GpuVertex,
    shine: Option<usize>,
    flap: Option<usize>,
}

/// `host` (built from `blob`) with its mesh nodes' geometry replaced by `hull`'s parts, or
/// `None` when no part names a host mesh node: the file is for another model and the caller
/// keeps the host. Each part, surface or texture that does not resolve is left out and named
/// in `report`.
#[must_use]
pub fn substitute(
    host: &Model,
    blob: &[u8],
    hull: &OwnHull,
    report: &mut Vec<String>,
) -> Option<Model> {
    let nodes = vex::nodes(blob).ok()?;
    let classes = vex::classes_of(blob).ok()?;
    let texture_names: Vec<String> = nodes
        .iter()
        .filter(|node| Some(node.class_id) == classes.texture)
        .map(|node| fold(node.name.as_deref().unwrap_or_default()))
        .collect();
    let templates = templates(host);
    let node_of = |name: &str| {
        let name = fold(name);
        templates.iter().filter_map(|t| t.draw.node).find(|&n| {
            nodes
                .get(n as usize)
                .and_then(|node| node.name.as_deref())
                .is_some_and(|own| fold(own) == name)
        })
    };
    if !hull.parts.iter().any(|part| node_of(&part.name).is_some()) {
        report.push(format!(
            "{}: the own hull names none of this model's mesh nodes - the model's own hull draws",
            host.label
        ));
        return None;
    }

    let mut model = host.clone_without_geometry();
    let mut drawn = 0;
    let mut lod_part = false;
    for part in &hull.parts {
        let Some(node) = node_of(&part.name) else {
            report.push(format!(
                "{}: own hull part {} has no mesh node of that name - not drawn",
                host.label, part.name
            ));
            continue;
        };
        lod_part |= host.lod_groups.is_coarse_tier(node);
        let transform = Mat4::from_cols_array(&part.transform);
        let first_vertex = model.vertices.len() as u32;
        let mut flap_side = None;
        for surface in &part.surfaces {
            let wanted = fold(&surface.texture);
            let Some(slot) = texture_names.iter().position(|name| *name == wanted) else {
                report.push(format!(
                    "{}: own hull part {} surface {}: the model has no texture of that name - not drawn",
                    host.label, part.name, surface.texture
                ));
                continue;
            };
            let on_node: Vec<&Template> = templates
                .iter()
                .filter(|t| t.draw.node == Some(node) && t.draw.texture == Some(slot))
                .collect();
            let landing = if on_node.is_empty() {
                // Another node's draws with that texture, all of one node.
                let other = templates
                    .iter()
                    .find(|t| t.draw.texture == Some(slot))
                    .and_then(|t| t.draw.node);
                templates
                    .iter()
                    .filter(|t| t.draw.texture == Some(slot) && t.draw.node == other)
                    .collect()
            } else {
                on_node
            };
            if landing.is_empty() {
                report.push(format!(
                    "{}: own hull part {} surface {}: no draw of the model uses that texture - not drawn",
                    host.label, part.name, surface.texture
                ));
                continue;
            }
            for template in landing {
                flap_side = flap_side.or(template.flap);
                emit(&mut model, transform, node, surface, template);
            }
        }
        let span = first_vertex..model.vertices.len() as u32;
        if span.is_empty() {
            continue;
        }
        drawn += 1;
        model.node_vertex_ranges.push(span.clone());
        if let Some(side) = flap_side {
            Flap::record(&mut model.airbrakes[side], span, transform);
        }
    }
    if model.indices.is_empty() {
        report.push(format!(
            "{}: no surface of the own hull resolved - the model's own hull draws",
            host.label
        ));
        return None;
    }
    if !lod_part {
        model.lod_groups = LodGroups::default();
    }
    model.mesh_count = drawn;
    model.refit_bounds();
    model.sort_by_layer();
    crate::mesh_render::texlod::assign_lod_codes(
        &model.textures,
        &model.indices,
        &mut model.vertices,
        model
            .draws
            .iter()
            .chain(&model.alpha_tested_draws)
            .chain(&model.transparent_draws),
    );
    let (vertices, triangles) = hull.counts();
    report.push(format!(
        "{}: own hull drawn, {drawn} part(s), {vertices} vertex(es), {triangles} triangle(s){}{}",
        host.label,
        if hull.images == 0 {
            String::new()
        } else {
            format!(
                "; {} image(s) in the file not read, the host's textures paint it",
                hull.images
            )
        },
        if lod_part {
            ""
        } else {
            "; no far LOD part, the hull draws at every distance"
        }
    ));
    Some(model)
}

fn templates(host: &Model) -> Vec<Template> {
    let lists = [
        (List::Opaque, &host.draws),
        (List::Cutout, &host.alpha_tested_draws),
        (List::Blended, &host.transparent_draws),
    ];
    let mut out = Vec::new();
    for (list, draws) in lists {
        for draw in draws {
            let (Some(_), Some(_)) = (draw.node, draw.texture) else {
                continue;
            };
            let Some(vertex) = host
                .indices
                .get(draw.range.start as usize)
                .and_then(|&i| host.vertices.get(i as usize))
            else {
                continue;
            };
            let first = host.indices[draw.range.start as usize];
            let shine = host
                .shine_draws
                .iter()
                .find(|s| s.range == draw.range)
                .and_then(|s| s.texture);
            let flap = host.airbrakes.iter().position(|flap| {
                flap.as_ref()
                    .is_some_and(|flap| flap.vertices.contains(&first))
            });
            out.push(Template {
                list,
                draw: draw.clone(),
                vertex: *vertex,
                shine,
                flap,
            });
        }
    }
    out
}

/// Appends `surface`, placed by `transform`, as one draw in `template`'s state.
fn emit(model: &mut Model, transform: Mat4, node: u32, surface: &Surface, template: &Template) {
    let normals = Mat3::from_mat4(transform).inverse().transpose();
    let base = model.vertices.len() as u32;
    let first_index = model.indices.len() as u32;
    // A batch the host leaves uncoloured is lit and tinted by the builder's default; a surface
    // with no colours of its own takes that tint, and white on a batch the disc colours.
    let default_colour = if template.vertex.lit > 0.5 {
        template.vertex.colour
    } else {
        [1.0; 4]
    };
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for (i, &p) in surface.positions.iter().enumerate() {
        let position = transform.transform_point3(Vec3::from(p)).to_array();
        for k in 0..3 {
            lo[k] = lo[k].min(position[k]);
            hi[k] = hi[k].max(position[k]);
        }
        let normal = surface.normals.get(i).map_or([0.0, 0.0, 1.0], |&n| {
            (normals * Vec3::from(n)).normalize_or_zero().to_array()
        });
        let colour = surface
            .colours
            .as_ref()
            .and_then(|c| c.get(i))
            .map_or(default_colour, |c| c.map(|b| f32::from(b) / 255.0));
        model.vertices.push(GpuVertex {
            position,
            normal,
            colour,
            texcoord: surface.uvs.get(i).copied().unwrap_or([0.0, 0.0]),
            ..template.vertex
        });
    }
    for tri in &surface.triangles {
        if tri.iter().all(|&i| (i as usize) < surface.positions.len()) {
            model
                .indices
                .extend([base + tri[0], base + tri[1], base + tri[2]]);
        }
    }
    let last_index = model.indices.len() as u32;
    if last_index == first_index {
        return;
    }
    let centre = std::array::from_fn(|k| (lo[k] + hi[k]) * 0.5);
    let radius = (0..3)
        .map(|k| (hi[k] - centre[k]).powi(2))
        .sum::<f32>()
        .sqrt();
    let draw = DrawCall {
        range: first_index..last_index,
        bounds: Bounds { centre, radius },
        depth_point: template.draw.depth_point.map(|_| centre),
        node: Some(node),
        ..template.draw.clone()
    };
    if let Some(shine) = template.shine {
        model.shine_draws.push(DrawCall {
            texture: Some(shine),
            ..draw.clone()
        });
    }
    match template.list {
        List::Opaque => model.draws.push(draw),
        List::Cutout => model.alpha_tested_draws.push(draw),
        List::Blended => model.transparent_draws.push(draw),
    }
}

impl Model {
    /// This model's textures, materials and flags with no geometry, draws or flaps.
    fn clone_without_geometry(&self) -> Self {
        let mut model = Self {
            vertices: Vec::new(),
            indices: Vec::new(),
            draws: Vec::new(),
            alpha_tested_draws: Vec::new(),
            transparent_draws: Vec::new(),
            shine_draws: Vec::new(),
            node_vertex_ranges: Vec::new(),
            airbrakes: [None, None],
            ..self.clone()
        };
        model.caustic.draws.clear();
        model
    }

    /// Recomputes [`Self::centre`] and [`Self::radius`] from the vertices, as the builder does.
    fn refit_bounds(&mut self) {
        let mut lo = [f32::MAX; 3];
        let mut hi = [f32::MIN; 3];
        for v in &self.vertices {
            for i in 0..3 {
                lo[i] = lo[i].min(v.position[i]);
                hi[i] = hi[i].max(v.position[i]);
            }
        }
        self.centre = std::array::from_fn(|i| (lo[i] + hi[i]) * 0.5);
        self.radius = (0..3)
            .map(|i| (hi[i] - lo[i]) * 0.5)
            .fold(0.0f32, f32::max)
            .max(0.001);
    }
}

#[cfg(test)]
mod tests;
