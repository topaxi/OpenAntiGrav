//! A PS3 `.vex` and the `.rcsmodel` beside it, built as the game builds them.

use std::collections::BTreeMap;

use anyhow::{Context, Result};
use oag_assets::Container;
use oag_mesh::mesh::{self, DrawCall, Model};
use oag_vex::vex;

use crate::json::{Value, array, numbers, object, object_of, text};
use crate::{
    Corner, Decoded, Draw, Object, Pictures, albedo_is_second, colour_range, nearest_nodes,
    node_name,
};

/// Where one object's vertices come from: a mesh node, or a chunk no node places.
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Source {
    Node(u32),
    Chunk(u32),
}

pub fn objects(
    archive: &mut Container,
    entry: &str,
    data: &[u8],
    swap: Option<&(String, String)>,
    pictures: &mut Pictures,
) -> Result<Decoded> {
    let sibling = mesh::rcs::sibling_name(entry).context("not a .vex name")?;
    let geometry = archive.read_entry(&sibling)?;
    let mut read = Vec::new();
    let mut missing = Vec::new();
    let (model, report) = mesh::rcs::build_scene_with(
        entry,
        data,
        &geometry,
        &mut |path| {
            let swapped = swap.map(|(from, to)| path.replace(from.as_str(), to));
            let found = swapped
                .filter(|s| s != path)
                .and_then(|s| archive.read_entry(&s).ok().map(|blob| (s, blob)))
                .or_else(|| {
                    archive
                        .read_entry(path)
                        .ok()
                        .map(|blob| (path.to_owned(), blob))
                });
            if found.is_none() {
                missing.push(path.to_owned());
            }
            found.map(|(path, blob)| {
                read.push(path);
                blob
            })
        },
        Default::default(),
        Default::default(),
    )?;
    println!("{}", report.describe());
    read.sort();
    read.dedup();
    missing.sort();
    missing.dedup();

    let nodes = vex::nodes(data).context("walking the node tree")?;
    let rcsmodel = oag_rcs::rcsmodel::Model::parse(&geometry)
        .map_err(|e| anyhow::anyhow!("{sibling}: {e}"))?;
    let nearest = nearest_nodes(&model);
    let moved = model.sample_anim_nodes(0.0);

    let mut grouped: BTreeMap<Source, Vec<(&DrawCall, &'static str)>> = BTreeMap::new();
    for (list, pass) in [
        (&model.draws, "opaque"),
        (&model.alpha_tested_draws, "cutout"),
        (&model.transparent_draws, "blend"),
    ] {
        for draw in list {
            let source = match (draw.node, draw.chunk) {
                (Some(node), _) => Source::Node(node),
                (None, chunk) => Source::Chunk(chunk.unwrap_or(u32::MAX)),
            };
            grouped.entry(source).or_default().push((draw, pass));
        }
    }

    let mut out = Vec::new();
    for (source, draws) in grouped {
        let (name, node, nearest_lod) = match source {
            Source::Node(n) => (
                node_name(&nodes, n as usize),
                Some(n as usize),
                nearest.contains(&n),
            ),
            Source::Chunk(c) => (format!("chunk_{c}"), None, true),
        };
        let mut object = Object {
            name,
            node,
            nearest_lod,
            corners: Vec::new(),
            draws: Vec::new(),
        };
        for (draw, pass) in draws {
            object.draws.push(one_draw(
                &model,
                &rcsmodel,
                &moved,
                draw,
                pass,
                &mut object.corners,
                pictures,
            ));
        }
        // A chunk without a colour set has no baked light, so it reads as black beside
        // chunks that have one.
        if object.corners.iter().any(|c| c.colour.is_some()) {
            for corner in &mut object.corners {
                corner.colour.get_or_insert([0.0, 0.0, 0.0, 1.0]);
            }
        }
        out.push(object);
    }
    let facts = vec![
        ("geometry", text(&sibling)),
        ("loader_report", text(&report.describe())),
        ("files_read", array(read.iter().map(|p| text(p)))),
        ("files_not_found", array(missing.iter().map(|p| text(p)))),
        (
            "vertex_colour",
            text("HD's baked per-vertex light, which the shader adds; not paint"),
        ),
    ];
    Ok((out, facts))
}

fn one_draw(
    model: &Model,
    rcsmodel: &oag_rcs::rcsmodel::Model,
    moved: &[[f32; 16]],
    draw: &DrawCall,
    pass: &str,
    corners: &mut Vec<Corner>,
    pictures: &mut Pictures,
) -> Draw {
    // A draw's indices are one contiguous vertex run per submesh.
    let indices = &model.indices[draw.range.start as usize..draw.range.end as usize];
    let mut remap = BTreeMap::new();
    for &i in indices {
        remap.entry(i).or_insert(0u32);
    }
    let mut lit = false;
    for (&i, local) in &mut remap {
        let v = &model.vertices[i as usize];
        let matrix = (v.xform as usize)
            .checked_sub(1)
            .and_then(|slot| moved.get(slot));
        let uv = (v.anim as usize)
            .checked_sub(1)
            .and_then(|slot| model.anim_tracks.get(slot))
            .map_or(v.texcoord, |track| {
                let (scale, offset) = track.sample(0.0);
                [
                    v.texcoord[0] * scale[0] + offset[0],
                    v.texcoord[1] * scale[1] + offset[1],
                ]
            });
        lit |= v.colour[..3].iter().any(|&c| c != 0.0);
        *local = corners.len() as u32;
        corners.push(Corner {
            position: matrix.map_or(v.position, |m| vex::transform_point(m, v.position)),
            normal: Some(matrix.map_or(v.normal, |m| vex::transform_direction(m, v.normal))),
            colour: Some(v.colour),
            uv: Some(uv),
        });
    }
    // A chunk without a colour set reads as black light.
    let own: Vec<Corner> = remap.values().map(|&l| corners[l as usize]).collect();
    let light = if lit {
        colour_range(&own)
    } else {
        for &local in remap.values() {
            corners[local as usize].colour = None;
        }
        Value::null()
    };

    let slot = draw.texture;
    let picture = |slots: &mesh::TextureSlots| {
        slot.and_then(|s| slots.get(s))
            .and_then(Option::as_ref)
            .cloned()
    };
    let roles = slot
        .and_then(|s| model.material_slots.get(s))
        .copied()
        .unwrap_or(mesh::slots::DEFAULT);
    let albedo = if albedo_is_second(roles) {
        picture(&model.lightmaps)
    } else {
        picture(&model.textures)
    };
    let material = albedo.map_or_else(|| "untextured".to_owned(), |t| pictures.name(&t));
    let bound: Vec<(&str, Value)> = [
        ("unit0", &model.textures),
        ("unit1", &model.lightmaps),
        ("third", &model.pad_masks),
        ("fourth", &model.wave_maps),
        ("fifth", &model.fifth_maps),
        ("sixth", &model.sixth_maps),
    ]
    .into_iter()
    .filter_map(|(unit, slots)| picture(slots).map(|t| (unit, text(&pictures.name(&t)))))
    .collect();
    let authored = draw
        .chunk
        .and_then(|c| rcsmodel.meshes.get(c as usize))
        .and_then(|m| rcsmodel.material_of(m));
    let parameters = authored.map_or_else(Vec::new, |m| {
        m.parameters
            .iter()
            .map(|p| {
                object([
                    ("hash", text(&format!("{:#010x}", p.hash))),
                    ("value", numbers(&p.value)),
                ])
            })
            .collect()
    });
    Draw {
        material,
        triangles: indices
            .as_chunks::<3>()
            .0
            .iter()
            .map(|t| [remap[&t[0]], remap[&t[1]], remap[&t[2]]])
            .collect(),
        degenerate: 0,
        facts: vec![
            ("pass", text(pass)),
            ("vertex_colour_range", light),
            (
                "blend",
                text(&format!("{:?}", draw.blend_state.map(|b| b.color))),
            ),
            (
                "rcsmaterial",
                authored.map_or(Value::null(), |m| text(&m.name)),
            ),
            (
                "chunk",
                draw.chunk
                    .map_or(Value::null(), |c| Value::from(c as usize)),
            ),
            ("bound_pictures", object_of(bound)),
            (
                "authored_samplers",
                array(authored.into_iter().flat_map(|m| {
                    m.samplers
                        .iter()
                        .map(|(_, path)| path.as_deref().map_or(Value::null(), text))
                })),
            ),
            ("parameters", array(parameters)),
            ("roles", text(&format!("{roles:#010x}"))),
        ],
    }
}
