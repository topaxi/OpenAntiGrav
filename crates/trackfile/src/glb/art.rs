//! The art meshes of a glb: `oag/art/<n>` nodes, one `TRIANGLES` primitive each,
//! with the material and shadow role as node extras (ADR-0076).

use gltf::accessor::Iter;
use gltf::mesh::Semantic;
use serde_json::{Value, json};

use super::{Bin, ELEMENT_ARRAY_BUFFER, Error, TRIANGLES, UINT, extras_of, identity, same_len};
use crate::art::{Art, ArtMaterial, ArtMesh, Blend, ShadowRole};

pub(super) const PREFIX: &str = "oag/art/";

pub(super) fn write(
    bin: &mut Bin,
    art: &Art,
    push: &mut impl FnMut(&str, Vec<Value>, Option<Value>),
) -> Result<(), Error> {
    art.validate()?;
    for (n, m) in art.meshes.iter().enumerate() {
        let idx: Vec<u8> = m.indices.iter().flat_map(|i| i.to_le_bytes()).collect();
        let attrs = json!({
            "POSITION": bin.positions(&m.positions),
            "NORMAL": bin.floats(&m.normals, "VEC3"),
            "TEXCOORD_0": bin.floats(&m.texcoords, "VEC2"),
            "COLOR_0": bin.floats(&m.colours, "VEC4"),
        });
        let indices = bin.add(
            &idx,
            UINT,
            "SCALAR",
            m.indices.len(),
            ELEMENT_ARRAY_BUFFER,
            None,
        );
        let mut extras = json!({
            "oag_name": m.name,
            "oag_blend": m.material.blend.tag(),
            "oag_culled": m.material.culled,
            "oag_lit": m.material.lit,
            "oag_shadow": m.shadow.tag(),
        });
        if let Some(t) = m.material.texture {
            extras["oag_texture"] = json!(t);
        }
        if let Some(r) = m.material.alpha_ref {
            extras["oag_alpha_ref"] = json!(r);
        }
        if m.layer != 0 {
            extras["oag_layer"] = json!(m.layer);
        }
        push(
            &format!("{PREFIX}{n}"),
            vec![json!({ "attributes": attrs, "indices": indices, "mode": TRIANGLES })],
            Some(extras),
        );
    }
    Ok(())
}

pub(super) fn read(index: usize, node: &gltf::Node<'_>, blob: &[u8]) -> Result<ArtMesh, Error> {
    let record = format!("{PREFIX}{index}");
    if !identity(node) {
        return Err(Error::Transformed { record });
    }
    let mesh = node
        .mesh()
        .ok_or_else(|| Error::Glb(format!("{record}: the node has no mesh")))?;
    let mut prims = mesh.primitives();
    let (Some(prim), None) = (prims.next(), prims.next()) else {
        return Err(Error::Glb(format!(
            "{record}: expected exactly one primitive"
        )));
    };
    if prim.mode() != gltf::mesh::Mode::Triangles {
        return Err(Error::accessor(&record, "mode", "not TRIANGLES"));
    }
    let floats = |semantic: Semantic, name: &str| -> Result<gltf::Accessor<'_>, Error> {
        prim.get(&semantic)
            .ok_or_else(|| Error::accessor(&record, name, "missing"))
    };
    let positions: Vec<[f32; 3]> =
        Iter::new(floats(Semantic::Positions, "POSITION")?, |_| Some(blob))
            .map(Iterator::collect)
            .ok_or_else(|| Error::accessor(&record, "POSITION", "not float VEC3"))?;
    let normals: Vec<[f32; 3]> = Iter::new(floats(Semantic::Normals, "NORMAL")?, |_| Some(blob))
        .map(Iterator::collect)
        .ok_or_else(|| Error::accessor(&record, "NORMAL", "not float VEC3"))?;
    let texcoords: Vec<[f32; 2]> = Iter::new(floats(Semantic::TexCoords(0), "TEXCOORD_0")?, |_| {
        Some(blob)
    })
    .map(Iterator::collect)
    .ok_or_else(|| Error::accessor(&record, "TEXCOORD_0", "not float VEC2"))?;
    let colours: Vec<[f32; 4]> = Iter::new(floats(Semantic::Colors(0), "COLOR_0")?, |_| Some(blob))
        .map(Iterator::collect)
        .ok_or_else(|| Error::accessor(&record, "COLOR_0", "not float VEC4"))?;
    for (name, len) in [
        ("NORMAL", normals.len()),
        ("TEXCOORD_0", texcoords.len()),
        ("COLOR_0", colours.len()),
    ] {
        same_len(&record, name, len, positions.len())?;
    }
    let indices: Vec<u32> = prim
        .indices()
        .and_then(|a| Iter::<u32>::new(a, |_| Some(blob)))
        .ok_or_else(|| Error::accessor(&record, "indices", "missing or not unsigned int SCALAR"))?
        .collect();

    let extras = extras_of(node).unwrap_or(Value::Null);
    let text = |key: &str| extras.get(key).and_then(Value::as_str);
    let flag =
        |key: &str, default: bool| extras.get(key).and_then(Value::as_bool).unwrap_or(default);
    let blend = match text("oag_blend") {
        None => Blend::Opaque,
        Some(tag) => Blend::from_tag(tag).ok_or_else(|| {
            Error::accessor(&record, "oag_blend", format!("unknown blend {tag:?}"))
        })?,
    };
    let shadow = match text("oag_shadow") {
        None => ShadowRole::Both,
        Some(tag) => ShadowRole::from_tag(tag).ok_or_else(|| {
            Error::accessor(
                &record,
                "oag_shadow",
                format!("unknown shadow role {tag:?}"),
            )
        })?,
    };
    let texture = extras
        .get("oag_texture")
        .map(|v| {
            v.as_u64()
                .and_then(|t| usize::try_from(t).ok())
                .ok_or_else(|| Error::accessor(&record, "oag_texture", "not an index"))
        })
        .transpose()?;
    let alpha_ref = extras
        .get("oag_alpha_ref")
        .map(|v| {
            v.as_u64()
                .and_then(|r| u8::try_from(r).ok())
                .ok_or_else(|| Error::accessor(&record, "oag_alpha_ref", "not in 0..=255"))
        })
        .transpose()?;
    let layer = extras
        .get("oag_layer")
        .map(|v| {
            v.as_u64()
                .and_then(|l| u32::try_from(l).ok())
                .ok_or_else(|| {
                    Error::accessor(&record, "oag_layer", "not an unsigned 32-bit integer")
                })
        })
        .transpose()?
        .unwrap_or(0);
    Ok(ArtMesh {
        name: text("oag_name").unwrap_or("").to_string(),
        positions,
        normals,
        texcoords,
        colours,
        indices,
        material: ArtMaterial {
            texture,
            blend,
            culled: flag("oag_culled", true),
            lit: flag("oag_lit", false),
            alpha_ref,
        },
        shadow,
        layer,
    })
}
