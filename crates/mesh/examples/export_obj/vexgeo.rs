//! A `.vex` that carries its own geometry, read batch by batch so only authored
//! attributes come out.

use anyhow::{Context, Result};
use oag_assets::Container;
use oag_mesh::mesh;
use oag_vex::vex;

use crate::json::{Value, array, object, text};
use crate::{Corner, Decoded, Draw, Object, Pictures, colour_range, nearest_nodes, node_name};

/// The PS2's textures: a `.vex` that embeds none takes the nested WAD stored directly
/// before it in the archive (`docs/formats/ps2-texture.md`, "How a model finds its
/// texture set"). `None` for a file with pictures of its own.
fn texture_set(
    archive: &mut Container,
    entry: &str,
    data: &[u8],
) -> Result<Option<(usize, mesh::Ps2TextureSet)>> {
    if mesh::embedded_texture_slots(data)?
        .iter()
        .any(Option::is_some)
    {
        return Ok(None);
    }
    let wad = archive.as_wad_mut("the entry before a model")?;
    let before = wad
        .index_of_name(entry)
        .and_then(|i| i.checked_sub(1))
        .with_context(|| format!("nothing precedes {entry}"))?;
    let blob = wad.read(before)?;
    Ok(Some((before, mesh::Ps2TextureSet::parse(&blob)?)))
}

pub fn objects(
    archive: &mut Container,
    entry: &str,
    data: &[u8],
    pictures: &mut Pictures,
) -> Result<Decoded> {
    let nodes = vex::nodes(data).context("walking the node tree")?;
    let classes = vex::classes_of(data)?;
    let class = classes.mesh.context("no mesh class id")?;
    let world = vex::world_transforms(data, &nodes);
    let set = texture_set(archive, entry, data)?;
    let external = set.as_ref().map(|(_, set)| set);
    let textures = match external {
        Some(set) => mesh::resolve_texture_slots(data, &nodes, classes.texture, set),
        None => mesh::embedded_texture_slots(data)?,
    };
    let unresolved: Vec<Value> = nodes
        .iter()
        .filter(|n| Some(n.class_id) == classes.texture)
        .zip(&textures)
        .filter(|(_, slot)| slot.is_none())
        .map(|(n, _)| {
            vex::texture_asset_path(&data[n.payload()]).map_or(Value::null(), |p| text(&p))
        })
        .collect();
    let nearest = nearest_nodes(&mesh::build_with_textures(entry, data, external)?);
    let mut out = Vec::new();
    for (index, node) in nodes.iter().enumerate() {
        if node.class_id != class {
            continue;
        }
        let payload = &data[node.payload()];
        let to_world = &world[index];
        let materials = vex::mesh_materials(payload);
        let transforms = vex::mesh_tex_transforms(payload);
        let mut object = Object {
            name: node_name(&nodes, index),
            node: Some(index),
            nearest_lod: nearest.contains(&(index as u32)),
            corners: Vec::new(),
            draws: Vec::new(),
        };
        for list in [0u8, 1] {
            for batch in vex::mesh_batches(payload, list).context("decoding batches")? {
                let material = materials
                    .get(usize::from(batch.material_index))
                    .copied()
                    .flatten();
                let uv_transform = transforms
                    .get(usize::from(batch.material_index))
                    .and_then(Option::as_ref)
                    .map(|t| t.sample(0.0));
                let picture = |slot: u32| textures.get(slot as usize).and_then(Option::as_ref);
                let name = material
                    .and_then(|m| picture(m.texture))
                    .map_or_else(|| "untextured".to_owned(), |t| pictures.name(t));
                let second = material
                    .filter(|_| batch.pass_mask & 0x2000 != 0)
                    .and_then(|m| picture(m.second_texture))
                    .map(|t| pictures.name(t));
                let base = object.corners.len() as u32;
                object.corners.extend(batch.vertices.iter().map(|v| Corner {
                    position: vex::transform_point(to_world, v.position),
                    normal: v.normal.map(|n| vex::transform_direction(to_world, n)),
                    colour: v.colour.map(|c| c.map(|b| f32::from(b) / 255.0)),
                    uv: v.texcoord.map(|[s, t]| match uv_transform {
                        Some((scale, offset)) => {
                            [s * scale[0] + offset[0], t * scale[1] + offset[1]]
                        }
                        None => [s, t],
                    }),
                }));
                let pass = if batch.is_transparent() {
                    "blend"
                } else if batch.is_alpha_tested() {
                    "cutout"
                } else {
                    "opaque"
                };
                object.draws.push(Draw {
                    material: name,
                    triangles: batch
                        .triangles()
                        .into_iter()
                        .map(|t| t.map(|i| base + i))
                        .collect(),
                    degenerate: 0,
                    facts: vec![
                        ("pass", text(pass)),
                        ("blend", text(&format!("{:?}", batch.blend_class()))),
                        ("culled", Value::from(batch.is_culled())),
                        ("pass_mask", text(&format!("{:#06x}", batch.pass_mask))),
                        (
                            "second_pass_picture",
                            second.map_or(Value::null(), |n| text(&n)),
                        ),
                        ("animated_uv", Value::from(uv_transform.is_some())),
                        (
                            "vertex_colour_range",
                            colour_range(&object.corners[base as usize..]),
                        ),
                        (
                            "authored_normals",
                            Value::from(batch.vertices.iter().all(|v| v.normal.is_some())),
                        ),
                    ],
                });
            }
        }
        out.push(object);
    }
    let mut facts = vec![
        ("geometry", text("the .vex's own batches")),
        ("unresolved_textures", array(unresolved)),
    ];
    if let Some((index, set)) = &set {
        facts.push((
            "texture_set",
            object([
                ("archive_index", Value::from(*index)),
                ("entries", Value::from(set.entry_count())),
                ("decoded", Value::from(set.decoded_count())),
            ]),
        ));
    }
    Ok((out, facts))
}
