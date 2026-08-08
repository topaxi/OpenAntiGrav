//! Per-draw diagnostics and node isolation for `--mesh`, `--sky` and `--pads`.
//!
//! A screenshot says a surface looks wrong; it does not say which draw call
//! produced it. `--draws` prints the whole chain a batch goes through - list,
//! node, material, texture ordinal, decoded texture, pipeline, vertex colour -
//! so a wrong pixel can be attributed to a link in that chain instead of
//! guessed at from a texture name. `--only` then hides everything else, which
//! is what turns an attribution into a confirmation.

use oag_formats::vex;
use oag_render::mesh::{DrawCall, Model};

/// Which of [`Model`]'s three lists a draw came from, and therefore which
/// pipeline `mesh_render::build` draws it through.
const LISTS: [&str; 3] = ["opaque", "cutout", "blend"];

/// The three draw lists of a model, in the order `mesh_render` draws them.
fn lists(model: &Model) -> [&Vec<DrawCall>; 3] {
    [
        &model.draws,
        &model.alpha_tested_draws,
        &model.transparent_draws,
    ]
}

/// Mean RGBA of the vertices a draw references, on 0..1.
///
/// The reason this is here rather than left to the eye: a draw that resolved no
/// texture binds `mesh_render::build`'s white 1x1, so its on-screen colour is
/// the vertex colour alone. Printing both is what separates "the texture is
/// missing" from "the texture is fine and the vertices are bright".
fn mean_colour(model: &Model, draw: &DrawCall) -> [f32; 4] {
    let mut sum = [0.0f64; 4];
    let mut count = 0u32;
    for i in draw.range.clone() {
        let v = model.vertices[model.indices[i as usize] as usize];
        for (k, channel) in sum.iter_mut().enumerate() {
            *channel += f64::from(v.colour[k]);
        }
        count += 1;
    }
    if count == 0 {
        return [0.0; 4];
    }
    let n = f64::from(count);
    [
        (sum[0] / n) as f32,
        (sum[1] / n) as f32,
        (sum[2] / n) as f32,
        (sum[3] / n) as f32,
    ]
}

/// How many whole tiles of its texture a draw's vertices span, in U then V.
///
/// Under 1.0 means the draw shows part of one copy of the texture, which is
/// what a stretched sign does; well over 1.0 means it repeats, which is what a
/// swatch or a tiling wall does. The difference decides whether a small texture
/// on a large surface is a decode fault or the authored intent.
fn uv_span(model: &Model, draw: &DrawCall) -> [f32; 2] {
    let (mut lo, mut hi) = ([f32::MAX; 2], [f32::MIN; 2]);
    for i in draw.range.clone() {
        let v = model.vertices[model.indices[i as usize] as usize];
        for k in 0..2 {
            lo[k] = lo[k].min(v.texcoord[k]);
            hi[k] = hi[k].max(v.texcoord[k]);
        }
    }
    if lo[0] > hi[0] {
        return [0.0, 0.0];
    }
    [hi[0] - lo[0], hi[1] - lo[1]]
}

/// The label of the texture a draw binds, or why it binds none.
///
/// `-> white 1x1` is not decoration: it is the actual bind
/// `mesh_render::build` performs for `DrawCall::texture == None` (slot 0 of
/// `texture_binds`), and a draw reading that way paints a white quad modulated
/// by nothing but its vertex colour and the light rig.
fn texture_label(model: &Model, draw: &DrawCall) -> String {
    match draw.texture {
        None => "none -> white 1x1".to_string(),
        Some(slot) => match model.textures.get(slot) {
            Some(Some(t)) => format!("{slot}: {} {}x{}", t.label, t.width, t.height),
            _ => format!("{slot}: undecoded -> white 1x1"),
        },
    }
}

/// Names of a `.vex` file's nodes, indexed the way [`DrawCall::node`] is.
///
/// A track's `Mesh` nodes carry the artists' own names, which is the only
/// human-readable handle on a piece of scenery: `--only` matches against these
/// as well as against texture labels.
#[must_use]
pub fn node_names(data: &[u8]) -> Vec<String> {
    vex::nodes(data)
        .map(|nodes| {
            nodes
                .iter()
                .map(|n| n.name.clone().unwrap_or_default())
                .collect()
        })
        .unwrap_or_default()
}

/// Prints every draw call of `model`, grouped by list.
///
/// `data` is the `.vex` the model was built from, used for node names and for
/// the material-side half of the chain; pass an empty slice to print the
/// model-side half alone.
pub fn report(model: &Model, data: &[u8]) {
    let names = node_names(data);
    println!(
        "{}: {} opaque, {} cutout, {} blend draw(s), {} texture slot(s) ({} decoded)",
        model.label,
        model.draws.len(),
        model.alpha_tested_draws.len(),
        model.transparent_draws.len(),
        model.textures.len(),
        model.textures.iter().filter(|t| t.is_some()).count(),
    );

    for (slot, texture) in model.textures.iter().enumerate() {
        match texture {
            None => println!("  texture {slot:>3}: undecoded"),
            Some(t) => println!(
                "  texture {slot:>3}: {:<40} {}x{}",
                t.label, t.width, t.height
            ),
        }
    }

    for (list, draws) in LISTS.iter().zip(lists(model)) {
        for draw in draws {
            let node = draw.node.map_or(usize::MAX, |n| n as usize);
            let name = names.get(node).map_or("", String::as_str);
            let colour = mean_colour(model, draw);
            let uv = uv_span(model, draw);
            println!(
                "  {list:<6} node {node:>4} {name:<28} {:>6} tri  tex {:<34} \
                 uv {:.2}x{:.2}  rgba {:.2},{:.2},{:.2},{:.2}  centre {:.0},{:.0},{:.0}",
                (draw.range.end - draw.range.start) / 3,
                texture_label(model, draw),
                uv[0],
                uv[1],
                colour[0],
                colour[1],
                colour[2],
                colour[3],
                draw.bounds.centre[0],
                draw.bounds.centre[1],
                draw.bounds.centre[2],
            );
        }
    }

    // The unresolved set is the whole point of the report: each of these draws
    // paints white, whatever its material meant to paint.
    let unresolved: usize = lists(model)
        .iter()
        .map(|draws| draws.iter().filter(|d| d.texture.is_none()).count())
        .sum();
    println!("  {unresolved} draw(s) resolve no texture and therefore bind the white 1x1");

    if !data.is_empty() {
        report_materials(data);
    }
}

/// Prints the material side of the chain: which material each batch selects,
/// which texture ordinal that material names, and whether the ordinal is in
/// range of the file's own texture array.
///
/// Walked from the file rather than from [`Model`] because a `DrawCall` keeps
/// no material index: once the chain has been collapsed to
/// `Option<texture slot>`, a break in it is no longer attributable to a link.
fn report_materials(data: &[u8]) {
    let Ok(nodes) = vex::nodes(data) else { return };
    let Ok(textures) = vex::textures(data) else {
        return;
    };
    let decoded: Vec<bool> = textures.iter().map(Option::is_some).collect();
    println!(
        "  material chain ({} texture node(s), {} decoded):",
        decoded.len(),
        decoded.iter().filter(|d| **d).count()
    );

    for (index, node) in nodes.iter().enumerate() {
        if node.class_id != vex::CLASS_MESH
            && node.class_id != vex::CLASS_SKYCUBE
            && node.class_id != vex::CLASS_SPEEDUP_PAD
        {
            continue;
        }
        let payload = &data[node.payload()];
        let materials = vex::mesh_materials(payload);
        for list in [0u8, 1u8] {
            let Ok(batches) = vex::mesh_batches(payload, list) else {
                continue;
            };
            for batch in batches {
                let material = materials.get(usize::from(batch.material_index)).copied();
                let ordinal = material.flatten().map(|m| m.texture as usize);
                let state = match ordinal {
                    None => "no material".to_string(),
                    Some(t) if t >= decoded.len() => format!("ordinal {t} out of range"),
                    Some(t) if !decoded[t] => format!("ordinal {t} undecoded"),
                    Some(t) => format!("ordinal {t}"),
                };
                let pipeline = if batch.is_transparent() {
                    "blend"
                } else if batch.is_alpha_tested() {
                    "cutout"
                } else {
                    "opaque"
                };
                println!(
                    "    node {index:>4} {:<28} list {list} mat {:>3} flags {:#06x} \
                     pass {:#06x} hdr {:#04x} {pipeline:<6} {state}",
                    node.name.clone().unwrap_or_default(),
                    batch.material_index,
                    material.flatten().map_or(0, |m| m.flags),
                    batch.pass_mask,
                    batch.header_flags,
                );
            }
        }
    }
}

/// Keeps only the draws whose node name or texture label contains `needle`,
/// case-insensitively.
///
/// Geometry, textures and bounds are left alone: hiding a draw is dropping it
/// from a list, so the surviving draws index exactly the vertices and textures
/// they did before. `Model::radius`/`centre` are recomputed from the surviving
/// draws so the orbit camera frames what is left rather than the whole circuit,
/// which is the difference between an isolation view and a wide shot with one
/// object in it.
#[must_use]
pub fn isolate(mut model: Model, data: &[u8], needle: &str) -> Model {
    // `node:N` selects one scene-tree node by index, which is the only handle a
    // track's geometry has: every `Mesh` node in a track `.vex` carries an empty
    // name, so matching by text alone can never isolate one piece of scenery
    // from another sharing its texture.
    if let Some(index) = needle.strip_prefix("node:").and_then(|n| n.parse().ok()) {
        return isolate_node(model, index);
    }

    let names = node_names(data);
    let needle = needle.to_ascii_lowercase();
    // Snapshotted before the retains below, which borrow the lists mutably
    // while the predicate still needs to read a draw's texture label.
    let labels: Vec<String> = model
        .textures
        .iter()
        .map(|slot| {
            slot.as_ref()
                .map(|t| t.label.to_ascii_lowercase())
                .unwrap_or_default()
        })
        .collect();
    let keep = |draw: &DrawCall| {
        let node = draw.node.map_or(usize::MAX, |n| n as usize);
        names
            .get(node)
            .is_some_and(|n| n.to_ascii_lowercase().contains(&needle))
            || draw
                .texture
                .and_then(|slot| labels.get(slot))
                .is_some_and(|label| label.contains(&needle))
    };

    model.draws.retain(&keep);
    model.alpha_tested_draws.retain(&keep);
    model.transparent_draws.retain(&keep);
    reframe(model)
}

/// Keeps only the draws that came from one scene-tree node.
#[must_use]
fn isolate_node(mut model: Model, index: u32) -> Model {
    let keep = |draw: &DrawCall| draw.node == Some(index);
    model.draws.retain(keep);
    model.alpha_tested_draws.retain(keep);
    model.transparent_draws.retain(keep);
    reframe(model)
}

/// Recentres a model on the draws it still has.
///
/// Without this an isolated quad keeps the whole circuit's centre and radius and
/// the orbit camera frames it from a thousand units away, which is a wide shot
/// with one object in it rather than an isolation view.
fn reframe(mut model: Model) -> Model {
    let mut lo = [f32::MAX; 3];
    let mut hi = [f32::MIN; 3];
    for draws in lists(&model) {
        for draw in draws {
            for k in 0..3 {
                lo[k] = lo[k].min(draw.bounds.centre[k] - draw.bounds.radius);
                hi[k] = hi[k].max(draw.bounds.centre[k] + draw.bounds.radius);
            }
        }
    }
    if lo[0] <= hi[0] {
        model.centre = [
            (lo[0] + hi[0]) * 0.5,
            (lo[1] + hi[1]) * 0.5,
            (lo[2] + hi[2]) * 0.5,
        ];
        model.radius = (0..3)
            .map(|k| (hi[k] - lo[k]) * 0.5)
            .fold(0.0f32, f32::max)
            .max(0.001);
    }
    model
}
