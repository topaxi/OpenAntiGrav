//! Writes one disc model as Wavefront OBJ, MTL, PNG textures and a node tree, as a
//! modelling reference.
//!
//! ```sh
//! cargo run -p oag-mesh --example export_obj -- <image>:<archive> <entry.vex> <out-dir> \
//!     [--locators <entry.vex>] [--swap <from>=<to>]
//! ```
//!
//! A `.vex` with its own geometry (PSP, PS2) is read batch by batch, so normals, colours
//! and coordinates are written only where the file authors them; a PS2 file's pictures
//! come from the texture set stored before it. A PS3 `.vex` is built from
//! the `.rcsmodel` beside it as the game builds it. Vertices are in model space at the rest
//! pose, in engine units with no axis swap; `vt` is `(u, 1 - v)` because OBJ's origin is
//! bottom left and the engine's is top left. `--swap` substitutes part of every texture
//! path (a livery), falling back to the authored path. `--locators` adds a second file's
//! nodes to `nodes.json`.
//!
//! **Local use only**: the output is content from the disc, so write it under `data/`
//! and never commit it.

use std::collections::HashSet;
use std::fmt::Write as _;
use std::path::Path;

use anyhow::{Context, Result};
use oag_assets::Container;
use oag_mesh::mesh::{self, ModelTexture, Texels, slots};
use oag_vex::vex;

mod json;
mod rcs;
mod vexgeo;

use json::{Value, array, object, text};

/// One vertex as the file has it; `None` is an attribute the file does not author.
#[derive(Debug, Clone, Copy)]
pub struct Corner {
    pub position: [f32; 3],
    pub normal: Option<[f32; 3]>,
    /// RGBA, `0..=1`.
    pub colour: Option<[f32; 4]>,
    /// Engine convention: `v` runs down the picture.
    pub uv: Option<[f32; 2]>,
}

/// One draw of one object.
#[derive(Debug)]
pub struct Draw {
    /// Material name: the picture's file name, or `untextured`.
    pub material: String,
    pub triangles: Vec<[u32; 3]>,
    /// Zero-area triangles left out: the stitching of the file's strips.
    pub degenerate: usize,
    /// Extra facts for `nodes.json`.
    pub facts: Vec<(&'static str, Value)>,
}

/// One OBJ object: a mesh node, or a PS3 chunk no node places.
#[derive(Debug)]
pub struct Object {
    pub name: String,
    pub node: Option<usize>,
    /// Whether the nearest level of detail shows it.
    pub nearest_lod: bool,
    pub corners: Vec<Corner>,
    pub draws: Vec<Draw>,
}

/// The smallest and largest of each vertex colour channel, `0..=1`.
pub fn colour_range(corners: &[Corner]) -> Value {
    let mut lo = [f32::MAX; 4];
    let mut hi = [f32::MIN; 4];
    for colour in corners.iter().filter_map(|c| c.colour) {
        for k in 0..4 {
            lo[k] = lo[k].min(colour[k]);
            hi[k] = hi[k].max(colour[k]);
        }
    }
    if lo[0] > hi[0] {
        return Value::null();
    }
    object([("min", json::numbers(&lo)), ("max", json::numbers(&hi))])
}

/// Leaves the zero-area triangles out of every draw, counting them.
fn drop_degenerate(objects: &mut [Object]) {
    for o in objects {
        for draw in &mut o.draws {
            let before = draw.triangles.len();
            draw.triangles.retain(|t| {
                let [a, b, c] = t.map(|i| o.corners[i as usize].position);
                let u: [f32; 3] = std::array::from_fn(|k| b[k] - a[k]);
                let v: [f32; 3] = std::array::from_fn(|k| c[k] - a[k]);
                let cross = [
                    u[1] * v[2] - u[2] * v[1],
                    u[2] * v[0] - u[0] * v[2],
                    u[0] * v[1] - u[1] * v[0],
                ];
                cross.iter().map(|x| x * x).sum::<f32>() > 1e-12
            });
            draw.degenerate = before - draw.triangles.len();
        }
    }
}

/// What a reader returns: the objects, and its own facts for `nodes.json`.
pub type Decoded = (Vec<Object>, Vec<(&'static str, Value)>);

/// The pictures written beside the OBJ, one PNG per distinct picture.
#[derive(Debug, Default)]
pub struct Pictures(pub Vec<(String, u32, u32, Vec<u8>)>);

impl Pictures {
    /// The material name of `texture`: its file name, suffixed when two different
    /// pictures share one.
    pub fn name(&mut self, texture: &ModelTexture) -> String {
        let label = texture
            .label
            .rsplit(['/', '\\'])
            .next()
            .unwrap_or("picture")
            .replace(' ', "_");
        let rgba = match &texture.texels {
            Texels::Rgba8(rgba) => rgba.clone(),
            Texels::Chain(levels) => levels.first().cloned().unwrap_or_default(),
            Texels::Blocks { format, levels } => levels
                .first()
                .and_then(|l| format.decode_level(l, texture.width, texture.height))
                .map(Vec::into_flattened)
                .unwrap_or_default(),
            Texels::Uploaded { .. } => Vec::new(),
        };
        let mut name = label.clone();
        for n in 2.. {
            match self.0.iter().find(|(seen, ..)| *seen == name) {
                Some((_, _, _, seen)) if *seen == rgba => return name,
                Some(_) => name = format!("{n}_{label}"),
                None => break,
            }
        }
        self.0
            .push((name.clone(), texture.width, texture.height, rgba));
        name
    }
}

/// A node's own matrix and its model-space one, row-major with the translation in
/// elements 12..14 (a point is a row vector on the left).
fn node_matrices(data: &[u8], nodes: &[vex::Node]) -> Vec<([f32; 16], [f32; 16])> {
    let world = vex::world_transforms(data, nodes);
    let classes = vex::classes_of(data).ok();
    nodes
        .iter()
        .enumerate()
        .map(|(i, node)| {
            let parent = node.parent.map_or(vex::IDENTITY, |p| world[p]);
            if classes.and_then(|c| c.anim_transform) == Some(node.class_id) {
                let local =
                    vex::anim_transform_of(data, node).map_or(vex::IDENTITY, |a| a.sample(0.0));
                return (local, world[i]);
            }
            // Transforms and locators carry a bare 64-byte matrix; the root's payload,
            // whatever its length, is the source file's path.
            let local = (node.parent.is_some() && node.data_size == 64)
                .then(|| vex::transform(&data[node.payload()], vex::byte_order(data)))
                .flatten()
                .unwrap_or(vex::IDENTITY);
            (local, vex::multiply(&local, &parent))
        })
        .collect()
}

fn matrix(m: &[f32; 16]) -> Value {
    array(m.chunks(4).map(json::numbers))
}

fn bounds(corners: &[Corner]) -> Value {
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for c in corners {
        for k in 0..3 {
            lo[k] = lo[k].min(c.position[k]);
            hi[k] = hi[k].max(c.position[k]);
        }
    }
    object([("min", json::numbers(&lo)), ("max", json::numbers(&hi))])
}

fn node_list(data: &[u8], objects: &[Object]) -> Result<Value> {
    let nodes = vex::nodes(data).context("walking the node tree")?;
    let matrices = node_matrices(data, &nodes);
    Ok(array(nodes.iter().enumerate().map(|(i, node)| {
        let (local, world) = &matrices[i];
        let mut fields = vec![
            ("index", Value::from(i)),
            ("name", text(node.name.as_deref().unwrap_or(""))),
            ("class", text(vex::class_name(node.class_id).unwrap_or("?"))),
            ("class_id", Value::from(node.class_id as usize)),
            ("parent", node.parent.map_or(Value::null(), Value::from)),
            ("local", matrix(local)),
            ("world", matrix(world)),
        ];
        if let Some(object) = objects.iter().find(|o| o.node == Some(i)) {
            fields.extend(object_facts(object));
        }
        json::object_of(fields)
    })))
}

fn object_facts(o: &Object) -> Vec<(&'static str, Value)> {
    vec![
        ("object", text(&o.name)),
        ("nearest_lod", Value::from(o.nearest_lod)),
        ("vertices", Value::from(o.corners.len())),
        (
            "triangles",
            Value::from(o.draws.iter().map(|d| d.triangles.len()).sum::<usize>()),
        ),
        ("bounds", bounds(&o.corners)),
        (
            "draws",
            array(o.draws.iter().map(|d| {
                let mut fields = vec![
                    ("material", text(&d.material)),
                    ("triangles", Value::from(d.triangles.len())),
                    ("degenerate_dropped", Value::from(d.degenerate)),
                ];
                fields.extend(d.facts.iter().cloned());
                json::object_of(fields)
            })),
        ),
    ]
}

fn write_obj(dir: &Path, stem: &str, objects: &[Object], pictures: &Pictures) -> Result<()> {
    let mut obj = format!("mtllib {stem}.mtl\n");
    let (mut v, mut vt, mut vn) = (1usize, 1usize, 1usize);
    let mut materials: Vec<&str> = Vec::new();
    for o in objects {
        writeln!(obj, "o {}", o.name)?;
        let coloured = o.corners.iter().any(|c| c.colour.is_some());
        let textured = o.corners.iter().any(|c| c.uv.is_some());
        let normals = o.corners.iter().any(|c| c.normal.is_some());
        for c in &o.corners {
            let [x, y, z] = c.position;
            if coloured {
                let [r, g, b, _] = c.colour.unwrap_or([1.0; 4]);
                writeln!(obj, "v {x} {y} {z} {r} {g} {b}")?;
            } else {
                writeln!(obj, "v {x} {y} {z}")?;
            }
            if textured {
                let [s, t] = c.uv.unwrap_or([0.0; 2]);
                writeln!(obj, "vt {s} {}", 1.0 - t)?;
            }
            if normals {
                let [a, b, d] = c.normal.unwrap_or([0.0, 1.0, 0.0]);
                writeln!(obj, "vn {a} {b} {d}")?;
            }
        }
        for draw in &o.draws {
            writeln!(obj, "usemtl {}", draw.material)?;
            if !materials.contains(&draw.material.as_str()) {
                materials.push(&draw.material);
            }
            for tri in &draw.triangles {
                obj.push('f');
                for &i in tri {
                    let i = i as usize;
                    match (textured, normals) {
                        (true, true) => write!(obj, " {}/{}/{}", v + i, vt + i, vn + i)?,
                        (true, false) => write!(obj, " {}/{}", v + i, vt + i)?,
                        (false, true) => write!(obj, " {}//{}", v + i, vn + i)?,
                        (false, false) => write!(obj, " {}", v + i)?,
                    }
                }
                obj.push('\n');
            }
        }
        v += o.corners.len();
        vt += if textured { o.corners.len() } else { 0 };
        vn += if normals { o.corners.len() } else { 0 };
    }
    let mut mtl = String::new();
    for name in materials {
        writeln!(mtl, "newmtl {name}\nKd 1 1 1")?;
        if pictures.0.iter().any(|(n, ..)| n == name) {
            writeln!(mtl, "map_Kd {name}.png")?;
        }
    }
    std::fs::write(dir.join(format!("{stem}.obj")), obj)?;
    std::fs::write(dir.join(format!("{stem}.mtl")), mtl)?;
    Ok(())
}

fn main() -> Result<()> {
    let mut args = std::env::args().skip(1);
    let mut positional = Vec::new();
    let mut locators = None;
    let mut swap = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--locators" => locators = args.next(),
            "--swap" => {
                swap = args
                    .next()
                    .and_then(|s| s.split_once('=').map(|(a, b)| (a.to_owned(), b.to_owned())));
            }
            _ => positional.push(arg),
        }
    }
    let [spec, entry, out] = positional.as_slice() else {
        anyhow::bail!(
            "usage: export_obj <image>:<archive> <entry.vex> <out-dir> \
             [--locators <entry.vex>] [--swap <from>=<to>]"
        );
    };
    let mut archive = Container::open(spec)?;
    let data = archive.read_entry(entry)?;
    let dir = Path::new(out);
    std::fs::create_dir_all(dir)?;
    let stem = entry
        .rsplit(['/', '\\'])
        .next()
        .and_then(|f| f.split('.').next())
        .context("the entry has no file name")?;

    let mut pictures = Pictures::default();
    let (mut objects, mut report) = if vex::geometry_is_external(&data) {
        rcs::objects(&mut archive, entry, &data, swap.as_ref(), &mut pictures)?
    } else {
        vexgeo::objects(&mut archive, entry, &data, &mut pictures)?
    };

    drop_degenerate(&mut objects);
    write_obj(dir, stem, &objects, &pictures)?;
    for (name, width, height, rgba) in &pictures.0 {
        anyhow::ensure!(
            rgba.len() == *width as usize * *height as usize * 4,
            "{name}: the picture did not decode"
        );
        let png = oag_texture::png::encode_rgba(*width, *height, rgba);
        std::fs::write(dir.join(format!("{name}.png")), png)?;
    }

    report.extend([
        ("entry", text(entry)),
        ("vex_version", Value::from(vex::version(&data)? as usize)),
        (
            "matrices",
            text("row-major, row vector on the left, translation in the fourth row"),
        ),
        (
            "pictures",
            array(pictures.0.iter().map(|(name, w, h, _)| {
                object([
                    ("name", text(name)),
                    ("width", Value::from(*w as usize)),
                    ("height", Value::from(*h as usize)),
                ])
            })),
        ),
        ("nodes", node_list(&data, &objects)?),
        (
            "unplaced_objects",
            array(
                objects
                    .iter()
                    .filter(|o| o.node.is_none())
                    .map(|o| json::object_of(object_facts(o))),
            ),
        ),
    ]);
    if let Some(entry) = locators {
        let data = archive.read_entry(&entry)?;
        report.push(("locators_entry", text(&entry)));
        report.push(("locators", node_list(&data, &[])?));
    }
    std::fs::write(dir.join("nodes.json"), json::object_of(report).to_string())?;

    let mut hidden: HashSet<&str> = HashSet::new();
    for o in &objects {
        if !o.nearest_lod {
            hidden.insert(&o.name);
        }
        println!(
            "{:<32} {:>6} vertices {:>6} triangles  {}",
            o.name,
            o.corners.len(),
            o.draws.iter().map(|d| d.triangles.len()).sum::<usize>(),
            o.draws
                .iter()
                .map(|d| d.material.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    println!("{} coarser-LOD object(s): {hidden:?}", hidden.len());
    println!("{} picture(s) written to {out}", pictures.0.len());
    Ok(())
}

/// The role bit that moves an HD material's colour to its second picture.
pub fn albedo_is_second(roles: u32) -> bool {
    roles & slots::ALBEDO_FROM_SECOND != 0
}

/// The node's name, or its index where the file names none.
pub fn node_name(nodes: &[vex::Node], index: usize) -> String {
    nodes[index]
        .name
        .clone()
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| format!("node{index}"))
}

/// The node indices whose draws survive the cut to the nearest level of detail.
pub fn nearest_nodes(model: &mesh::Model) -> HashSet<u32> {
    let mut model = model.clone();
    model.keep_nearest();
    model
        .draws
        .iter()
        .chain(&model.alpha_tested_draws)
        .chain(&model.transparent_draws)
        .filter_map(|d| d.node)
        .collect()
}
