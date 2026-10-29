//! The glTF binary half: writer and reader.
//!
//! One scene, flat nodes, in this order: `oag/start`, `oag/spline/<i>` per path,
//! `oag/pads/speedup`, `oag/pads/weapon`, then one node per collision node with
//! an `oag_kind` extra. Every record is a mesh primitive; numbers beyond
//! positions are custom `_OAG_*` attributes, so a stock glTF viewer shows the
//! points and triangles. Coordinates are this engine's world: right-handed,
//! Y up, in world units (not metres).

use gltf::Semantic;
use gltf::accessor::{Item, Iter};
use oag_vex::collision::{
    CollisionGeometry, CollisionMesh, CollisionNode, HEADER_WORD, SurfaceKind,
};
use oag_vex::pads::PadVolume;
use oag_vex::track::{AiTrack, Path, SplinePoint, StartPosition};
use serde_json::{Value, json};

use crate::manifest::{Parsed, opt};
use crate::{Error, Loaded, Track};

const FLOAT: u32 = 5126;
const UBYTE: u32 = 5121;
const USHORT: u32 = 5123;
const UINT: u32 = 5125;
const ARRAY_BUFFER: u32 = 34962;
const ELEMENT_ARRAY_BUFFER: u32 = 34963;
const POINTS: u32 = 0;
const TRIANGLES: u32 = 4;

mod art;

const START: &str = "oag/start";
const SPEEDUP: &str = "oag/pads/speedup";
const WEAPON: &str = "oag/pads/weapon";
const SPLINE: &str = "oag/spline/";

const KINDS: [(SurfaceKind, &str); 6] = [
    (SurfaceKind::Floor, "floor"),
    (SurfaceKind::Wall, "wall"),
    (SurfaceKind::Reset, "reset"),
    (SurfaceKind::MagFloor, "mag_floor"),
    (SurfaceKind::Cage, "cage"),
    (SurfaceKind::TrackWall, "track_wall"),
];

fn kind_tag(kind: SurfaceKind) -> &'static str {
    KINDS
        .iter()
        .find(|(k, _)| *k == kind)
        .map_or("floor", |(_, t)| t)
}

struct Bin {
    data: Vec<u8>,
    views: Vec<Value>,
    accessors: Vec<Value>,
}

impl Bin {
    fn add(
        &mut self,
        bytes: &[u8],
        component: u32,
        ty: &str,
        count: usize,
        target: u32,
        minmax: Option<([f64; 3], [f64; 3])>,
    ) -> usize {
        while !self.data.len().is_multiple_of(4) {
            self.data.push(0);
        }
        self.views.push(json!({
            "buffer": 0, "byteOffset": self.data.len(),
            "byteLength": bytes.len(), "target": target,
        }));
        self.data.extend_from_slice(bytes);
        let mut acc = json!({
            "bufferView": self.views.len() - 1, "componentType": component,
            "count": count, "type": ty,
        });
        if let Some((lo, hi)) = minmax {
            acc["min"] = json!(lo);
            acc["max"] = json!(hi);
        }
        self.accessors.push(acc);
        self.accessors.len() - 1
    }

    fn floats<const N: usize>(&mut self, rows: &[[f32; N]], ty: &str) -> usize {
        let bytes: Vec<u8> = rows
            .iter()
            .flatten()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        self.add(&bytes, FLOAT, ty, rows.len(), ARRAY_BUFFER, None)
    }

    fn positions(&mut self, rows: &[[f32; 3]]) -> usize {
        let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
        for r in rows {
            for i in 0..3 {
                lo[i] = lo[i].min(f64::from(r[i]));
                hi[i] = hi[i].max(f64::from(r[i]));
            }
        }
        let bytes: Vec<u8> = rows
            .iter()
            .flatten()
            .flat_map(|v| v.to_le_bytes())
            .collect();
        self.add(
            &bytes,
            FLOAT,
            "VEC3",
            rows.len(),
            ARRAY_BUFFER,
            Some((lo, hi)),
        )
    }

    fn scalars(&mut self, v: &[f32]) -> usize {
        let bytes: Vec<u8> = v.iter().flat_map(|x| x.to_le_bytes()).collect();
        self.add(&bytes, FLOAT, "SCALAR", v.len(), ARRAY_BUFFER, None)
    }

    fn bytes4(&mut self, rows: &[[u8; 4]]) -> usize {
        let bytes: Vec<u8> = rows.iter().flatten().copied().collect();
        self.add(&bytes, UBYTE, "VEC4", rows.len(), ARRAY_BUFFER, None)
    }
}

fn finite(record: &str, field: &str, values: impl IntoIterator<Item = f32>) -> Result<(), Error> {
    if values.into_iter().all(f32::is_finite) {
        Ok(())
    } else {
        Err(Error::NonFinite {
            record: record.to_string(),
            field: field.to_string(),
        })
    }
}

pub(crate) fn write(track: &Track) -> Result<Vec<u8>, Error> {
    let mut bin = Bin {
        data: Vec::new(),
        views: Vec::new(),
        accessors: Vec::new(),
    };
    let mut nodes: Vec<Value> = Vec::new();
    let mut meshes: Vec<Value> = Vec::new();
    let mut push = |name: &str, prims: Vec<Value>, extras: Option<Value>| {
        let mut node = json!({ "name": name });
        if !prims.is_empty() {
            meshes.push(json!({ "name": name, "primitives": prims }));
            node["mesh"] = json!(meshes.len() - 1);
        }
        if let Some(e) = extras {
            node["extras"] = e;
        }
        nodes.push(node);
    };

    let s = &track.start;
    finite(
        START,
        "frame",
        s.position
            .iter()
            .chain(&s.left)
            .chain(&s.up)
            .chain(&s.forward)
            .copied(),
    )?;
    let attrs = json!({
        "POSITION": bin.positions(&[s.position]),
        "_OAG_LEFT": bin.floats(&[s.left], "VEC3"),
        "_OAG_UP": bin.floats(&[s.up], "VEC3"),
        "_OAG_FORWARD": bin.floats(&[s.forward], "VEC3"),
    });
    push(
        START,
        vec![json!({ "attributes": attrs, "mode": POINTS })],
        None,
    );

    for (i, path) in track.ai.paths.iter().enumerate() {
        if path.points.is_empty() {
            continue;
        }
        let name = format!("{SPLINE}{i}");
        let p = &path.points;
        for pt in p {
            finite(
                &name,
                "point",
                pt.pos
                    .iter()
                    .chain(&pt.tangent)
                    .chain(&pt.down)
                    .chain(&pt.lateral)
                    .chain(&[
                        pt.progress,
                        pt.half_width_left,
                        pt.half_width_right,
                        pt.ai_bound_left,
                        pt.ai_bound_right,
                        pt.racing_line,
                    ])
                    .copied(),
            )?;
        }
        let col3 = |f: fn(&SplinePoint) -> [f32; 3]| p.iter().map(f).collect::<Vec<_>>();
        let mut attrs = json!({
            "POSITION": bin.positions(&col3(|q| q.pos)),
            "_OAG_TANGENT": bin.floats(&col3(|q| q.tangent), "VEC3"),
            "_OAG_DOWN": bin.floats(&col3(|q| q.down), "VEC3"),
            "_OAG_LATERAL": bin.floats(&col3(|q| q.lateral), "VEC3"),
            "_OAG_PROGRESS": bin.scalars(&p.iter().map(|q| q.progress).collect::<Vec<_>>()),
            "_OAG_HALF_WIDTH": bin.floats(
                &p.iter().map(|q| [q.half_width_left, q.half_width_right]).collect::<Vec<_>>(), "VEC2"),
            "_OAG_AI_BOUND": bin.floats(
                &p.iter().map(|q| [q.ai_bound_left, q.ai_bound_right]).collect::<Vec<_>>(), "VEC2"),
            "_OAG_RACING_LINE": bin.scalars(&p.iter().map(|q| q.racing_line).collect::<Vec<_>>()),
            "_OAG_SECTION_FLAGS": bin.bytes4(
                &p.iter().map(|q| [q.section_id, q.flags, 0, 0]).collect::<Vec<_>>()),
            "_OAG_LIGHT_SCALE": bin.bytes4(&p.iter().map(|q| q.light_scale).collect::<Vec<_>>()),
        });
        // Optional: written only when a point carries the original's AI bytes.
        if p.iter().any(|q| q.unk_0x5c.is_some()) {
            let bytes = p
                .iter()
                .map(|q| match (q.unk_0x5c, q.unk_0x5d, q.unk_0x66) {
                    (Some(lo), Some(hi), Some(floor)) => [lo, hi, floor, 1],
                    _ => [0; 4],
                })
                .collect::<Vec<_>>();
            attrs["_OAG_AI_BYTES"] = json!(bin.bytes4(&bytes));
        }
        // Optional: the collision cage's eight signed bytes, as two u8 VEC4s,
        // written only when a point authors any.
        if p.iter().any(|q| q.cage != [0; 8]) {
            let half = |from: usize| {
                p.iter()
                    .map(|q| std::array::from_fn(|k| q.cage[from + k] as u8))
                    .collect::<Vec<[u8; 4]>>()
            };
            attrs["_OAG_CAGE_A"] = json!(bin.bytes4(&half(0)));
            attrs["_OAG_CAGE_B"] = json!(bin.bytes4(&half(4)));
        }
        push(
            &name,
            vec![json!({ "attributes": attrs, "mode": POINTS })],
            None,
        );
    }

    for (name, pads) in [(SPEEDUP, &track.speedup_pads), (WEAPON, &track.weapon_pads)] {
        if pads.is_empty() {
            continue;
        }
        for pad in pads {
            finite(
                name,
                "pad",
                pad.to_world
                    .iter()
                    .chain(&pad.min)
                    .chain(&pad.max)
                    .chain(&[pad.disabled])
                    .copied(),
            )?;
        }
        let centre: Vec<[f32; 3]> = pads
            .iter()
            .map(|p| [p.to_world[12], p.to_world[13], p.to_world[14]])
            .collect();
        let attrs = json!({
            "POSITION": bin.positions(&centre),
            "_OAG_TO_WORLD": bin.floats(&pads.iter().map(|p| p.to_world.map(|v| v)).collect::<Vec<_>>(), "MAT4"),
            "_OAG_MIN": bin.floats(&pads.iter().map(|p| p.min).collect::<Vec<_>>(), "VEC3"),
            "_OAG_MAX": bin.floats(&pads.iter().map(|p| p.max).collect::<Vec<_>>(), "VEC3"),
            "_OAG_DISABLED": bin.scalars(&pads.iter().map(|p| p.disabled).collect::<Vec<_>>()),
        });
        push(
            name,
            vec![json!({ "attributes": attrs, "mode": POINTS })],
            None,
        );
    }

    for (n, node) in track.nodes.iter().enumerate() {
        let name = format!("oag/collision/{n}");
        let mut prims = Vec::new();
        for (m, mesh) in node.geometry.meshes.iter().enumerate() {
            let what = format!("{name} mesh {m}");
            if mesh.triangles.is_empty() || mesh.vertices.is_empty() {
                return Err(Error::Unwritable(format!("{what} is empty")));
            }
            if mesh.vertex_scalars.len() != mesh.vertices.len() {
                return Err(Error::Unwritable(format!(
                    "{what}: one scalar per vertex is required"
                )));
            }
            finite(&what, "vertices", mesh.vertices.iter().flatten().copied())?;
            finite(&what, "vertex scalars", mesh.vertex_scalars.iter().copied())?;
            if mesh
                .triangles
                .iter()
                .flatten()
                .any(|&i| usize::from(i) >= mesh.vertices.len())
            {
                return Err(Error::Unwritable(format!(
                    "{what}: a triangle index is out of range"
                )));
            }
            let idx: Vec<u8> = mesh
                .triangles
                .iter()
                .flatten()
                .flat_map(|i| i.to_le_bytes())
                .collect();
            let attrs = json!({
                "POSITION": bin.positions(&mesh.vertices),
                "_OAG_SCALAR": bin.scalars(&mesh.vertex_scalars),
            });
            let indices = bin.add(
                &idx,
                USHORT,
                "SCALAR",
                mesh.triangles.len() * 3,
                ELEMENT_ARRAY_BUFFER,
                None,
            );
            prims.push(json!({ "attributes": attrs, "indices": indices, "mode": TRIANGLES }));
        }
        let mut extras =
            json!({ "oag_kind": kind_tag(node.kind), "oag_version": node.geometry.version });
        if let Some(label) = &node.name {
            extras["oag_name"] = json!(label);
        }
        push(&name, prims, Some(extras));
    }

    if let Some(art) = &track.art {
        art::write(&mut bin, art, &mut push)?;
    }

    while !bin.data.len().is_multiple_of(4) {
        bin.data.push(0);
    }
    let count = nodes.len();
    let doc = json!({
        "asset": { "version": "2.0", "generator": "oag-trackfile 1.0" },
        "scene": 0,
        "scenes": [{ "nodes": (0..count).collect::<Vec<_>>() }],
        "nodes": nodes, "meshes": meshes,
        "buffers": [{ "byteLength": bin.data.len() }],
        "bufferViews": bin.views, "accessors": bin.accessors,
    });
    let mut text = serde_json::to_vec(&doc).map_err(|e| Error::Unwritable(e.to_string()))?;
    while !text.len().is_multiple_of(4) {
        text.push(b' ');
    }
    let total = 12 + 8 + text.len() + 8 + bin.data.len();
    let mut out = Vec::with_capacity(total);
    let word = |out: &mut Vec<u8>, v: usize| out.extend_from_slice(&(v as u32).to_le_bytes());
    out.extend_from_slice(b"glTF");
    word(&mut out, 2);
    word(&mut out, total);
    word(&mut out, text.len());
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(&text);
    word(&mut out, bin.data.len());
    out.extend_from_slice(b"BIN\0");
    out.extend_from_slice(&bin.data);
    Ok(out)
}

fn ai_present(bytes: &Option<Vec<[u8; 4]>>, i: usize) -> Option<[u8; 4]> {
    bytes.as_ref().map(|b| b[i]).filter(|b| b[3] == 1)
}

fn items<T: Item>(
    record: &str,
    attribute: &str,
    prim: &gltf::Primitive<'_>,
    blob: &[u8],
    expect: &str,
) -> Result<Vec<T>, Error> {
    let accessor = prim
        .get(&Semantic::Extras(
            attribute.trim_start_matches('_').to_string(),
        ))
        .ok_or_else(|| Error::accessor(record, attribute, "missing"))?;
    Iter::<T>::new(accessor, |_| Some(blob))
        .map(Iterator::collect)
        .ok_or_else(|| Error::accessor(record, attribute, format!("not {expect}")))
}

fn positions(
    record: &str,
    prim: &gltf::Primitive<'_>,
    blob: &[u8],
) -> Result<Vec<[f32; 3]>, Error> {
    let accessor = prim
        .get(&Semantic::Positions)
        .ok_or_else(|| Error::accessor(record, "POSITION", "missing"))?;
    Iter::<[f32; 3]>::new(accessor, |_| Some(blob))
        .map(Iterator::collect)
        .ok_or_else(|| Error::accessor(record, "POSITION", "not float VEC3"))
}

fn same_len(record: &str, attribute: &str, got: usize, want: usize) -> Result<(), Error> {
    if got == want {
        Ok(())
    } else {
        Err(Error::accessor(
            record,
            attribute,
            format!("{got} elements, POSITION has {want}"),
        ))
    }
}

fn extras_of(node: &gltf::Node<'_>) -> Option<Value> {
    let raw = node.extras().as_ref()?;
    serde_json::from_str(raw.get()).ok()
}

fn identity(node: &gltf::Node<'_>) -> bool {
    let m = node.transform().matrix();
    (0..4).all(|c| (0..4).all(|r| m[c][r].to_bits() == f32::from(u8::from(c == r)).to_bits()))
}

fn single_prim<'a>(record: &str, node: &gltf::Node<'a>) -> Result<gltf::Primitive<'a>, Error> {
    if !identity(node) {
        return Err(Error::Transformed {
            record: record.to_string(),
        });
    }
    let mesh = node
        .mesh()
        .ok_or_else(|| Error::Glb(format!("{record}: the node has no mesh")))?;
    let mut prims = mesh.primitives();
    match (prims.next(), prims.next()) {
        (Some(p), None) => Ok(p),
        _ => Err(Error::Glb(format!(
            "{record}: expected exactly one primitive"
        ))),
    }
}

fn walk<'a>(node: gltf::Node<'a>, out: &mut Vec<gltf::Node<'a>>) {
    out.push(node.clone());
    for c in node.children() {
        walk(c, out);
    }
}

pub(crate) fn read(m: Parsed, bytes: &[u8]) -> Result<Loaded, Error> {
    let g = gltf::Gltf::from_slice(bytes).map_err(|e| Error::Glb(e.to_string()))?;
    let blob = g
        .blob
        .as_deref()
        .ok_or_else(|| Error::Glb("no BIN chunk".into()))?;
    let scene = g
        .document
        .default_scene()
        .or_else(|| g.document.scenes().next())
        .ok_or_else(|| Error::Glb("no scene".into()))?;
    let mut all = Vec::new();
    for n in scene.nodes() {
        walk(n, &mut all);
    }

    let mut unread = m.unread.clone();
    let mut start = None;
    let mut spline: Vec<Option<Vec<SplinePoint>>> = vec![None; m.paths.len()];
    let (mut speedup, mut weapon) = (Vec::new(), Vec::new());
    let mut nodes = Vec::new();
    let mut art_meshes: Vec<(usize, crate::art::ArtMesh)> = Vec::new();
    let mut other = 0usize;

    for node in &all {
        let name = node.name().unwrap_or("");
        let extras = extras_of(node);
        if let Some(tag) = extras.as_ref().and_then(|e| e.get("oag_kind")) {
            let tag = tag.as_str().unwrap_or("");
            let kind = KINDS
                .iter()
                .find(|(_, t)| *t == tag)
                .map(|(k, _)| *k)
                .ok_or_else(|| Error::UnknownKind {
                    node: nodes.len(),
                    tag: tag.to_string(),
                })?;
            nodes.push(read_collision(
                node,
                kind,
                extras.as_ref(),
                blob,
                nodes.len(),
            )?);
        } else if name == START {
            start = Some(read_start(node, blob)?);
        } else if let Some(i) = name.strip_prefix(SPLINE) {
            let i: usize = i
                .parse()
                .map_err(|_| Error::Glb(format!("{name}: bad path index")))?;
            let slot = spline.get_mut(i).ok_or_else(|| Error::Range {
                what: name.to_string(),
                why: "the manifest has no such path".into(),
            })?;
            *slot = Some(read_spline(name, node, blob)?);
        } else if let Some(i) = name.strip_prefix(art::PREFIX) {
            let i: usize = i
                .parse()
                .map_err(|_| Error::Glb(format!("{name}: bad art mesh index")))?;
            art_meshes.push((i, art::read(i, node, blob)?));
        } else if name == SPEEDUP {
            speedup = read_pads(name, node, blob)?;
        } else if name == WEAPON {
            weapon = read_pads(name, node, blob)?;
        } else if node.mesh().is_some() {
            other += 1;
        }
    }
    if other > 0 {
        unread.push(format!(
            "{other} mesh node(s) without an oag name or oag_kind tag (visual meshes are not part of format 1.0)"
        ));
    }

    let start = start.ok_or_else(|| Error::Glb(format!("no {START} node")))?;
    let mut paths = Vec::new();
    for (i, row) in m.paths.iter().enumerate() {
        let points = spline[i].take().unwrap_or_default();
        same_len(
            &format!("spline path {i}"),
            "points",
            points.len(),
            row.points,
        )
        .map_err(|_| Error::CountMismatch {
            what: format!("spline path {i} points"),
            manifest: row.points,
            glb: points.len(),
        })?;
        paths.push(Path {
            points,
            max_spacing: f32::from_bits(row.max_spacing_bits),
            entry: opt(row.entry),
            exit: opt(row.exit),
        });
    }
    for (what, manifest, glb) in [
        ("collision nodes", m.counts.collision_nodes, nodes.len()),
        ("speedup pads", m.counts.speedup_pads, speedup.len()),
        ("weapon pads", m.counts.weapon_pads, weapon.len()),
        ("art meshes", m.counts.art_meshes, art_meshes.len()),
    ] {
        if manifest != glb {
            return Err(Error::CountMismatch {
                what: what.to_string(),
                manifest,
                glb,
            });
        }
    }
    art_meshes.sort_by_key(|(i, _)| *i);
    if art_meshes.iter().enumerate().any(|(n, (i, _))| n != *i) {
        return Err(Error::Glb("the art meshes are not numbered 0..n".into()));
    }
    let art = if m.art.is_some() || !art_meshes.is_empty() {
        let art = m
            .art
            .unwrap_or_default()
            .into_art(art_meshes.into_iter().map(|(_, a)| a).collect());
        art.validate()?;
        Some(art)
    } else {
        None
    };
    let mut notes = m.notes;
    notes.shrink_to_fit();
    Ok(Loaded {
        track: Track {
            id: m.id,
            title: m.title,
            host: Some(m.host),
            ai: AiTrack {
                version: m.point_version,
                paths,
                junctions: m.junctions,
            },
            start,
            nodes,
            speedup_pads: speedup,
            weapon_pads: weapon,
            art,
            notes,
        },
        unread,
    })
}

fn read_start(node: &gltf::Node<'_>, blob: &[u8]) -> Result<StartPosition, Error> {
    let prim = single_prim(START, node)?;
    let one = |attribute: &str| -> Result<[f32; 3], Error> {
        let v = items::<[f32; 3]>(START, attribute, &prim, blob, "float VEC3")?;
        match v.as_slice() {
            [x] => Ok(*x),
            _ => Err(Error::accessor(START, attribute, "expected one element")),
        }
    };
    let position = positions(START, &prim, blob)?;
    let [position] = position.as_slice() else {
        return Err(Error::accessor(START, "POSITION", "expected one element"));
    };
    let frame = StartPosition {
        position: *position,
        left: one("_OAG_LEFT")?,
        up: one("_OAG_UP")?,
        forward: one("_OAG_FORWARD")?,
    };
    finite(
        START,
        "frame",
        frame
            .position
            .iter()
            .chain(&frame.left)
            .chain(&frame.up)
            .chain(&frame.forward)
            .copied(),
    )?;
    Ok(frame)
}

fn read_spline(name: &str, node: &gltf::Node<'_>, blob: &[u8]) -> Result<Vec<SplinePoint>, Error> {
    let prim = single_prim(name, node)?;
    let pos = positions(name, &prim, blob)?;
    let n = pos.len();
    let v3 = |a: &str| -> Result<Vec<[f32; 3]>, Error> {
        let v = items::<[f32; 3]>(name, a, &prim, blob, "float VEC3")?;
        same_len(name, a, v.len(), n)?;
        Ok(v)
    };
    let v2 = |a: &str| -> Result<Vec<[f32; 2]>, Error> {
        let v = items::<[f32; 2]>(name, a, &prim, blob, "float VEC2")?;
        same_len(name, a, v.len(), n)?;
        Ok(v)
    };
    let s1 = |a: &str| -> Result<Vec<f32>, Error> {
        let v = items::<f32>(name, a, &prim, blob, "float SCALAR")?;
        same_len(name, a, v.len(), n)?;
        Ok(v)
    };
    let b4 = |a: &str| -> Result<Vec<[u8; 4]>, Error> {
        let v = items::<[u8; 4]>(name, a, &prim, blob, "unsigned byte VEC4")?;
        same_len(name, a, v.len(), n)?;
        Ok(v)
    };
    let (tangent, down, lateral) = (v3("_OAG_TANGENT")?, v3("_OAG_DOWN")?, v3("_OAG_LATERAL")?);
    let (progress, line) = (s1("_OAG_PROGRESS")?, s1("_OAG_RACING_LINE")?);
    let (half, bound) = (v2("_OAG_HALF_WIDTH")?, v2("_OAG_AI_BOUND")?);
    let (sf, light) = (b4("_OAG_SECTION_FLAGS")?, b4("_OAG_LIGHT_SCALE")?);
    let ai_bytes = if prim
        .get(&Semantic::Extras("OAG_AI_BYTES".to_string()))
        .is_some()
    {
        Some(b4("_OAG_AI_BYTES")?)
    } else {
        None
    };
    let cage = if prim
        .get(&Semantic::Extras("OAG_CAGE_A".to_string()))
        .is_some()
    {
        Some((b4("_OAG_CAGE_A")?, b4("_OAG_CAGE_B")?))
    } else {
        None
    };
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        if sf[i][2] != 0 || sf[i][3] != 0 {
            return Err(Error::accessor(
                name,
                "_OAG_SECTION_FLAGS",
                "padding bytes are not zero",
            ));
        }
        let p = SplinePoint {
            pos: pos[i],
            tangent: tangent[i],
            down: down[i],
            lateral: lateral[i],
            progress: progress[i],
            half_width_left: half[i][0],
            half_width_right: half[i][1],
            ai_bound_left: bound[i][0],
            ai_bound_right: bound[i][1],
            racing_line: line[i],
            section_id: sf[i][0],
            flags: sf[i][1],
            light_scale: light[i],
            unk_0x5c: ai_present(&ai_bytes, i).map(|b| b[0]),
            unk_0x5d: ai_present(&ai_bytes, i).map(|b| b[1]),
            unk_0x66: ai_present(&ai_bytes, i).map(|b| b[2]),
            cage: cage.as_ref().map_or([0; 8], |(a, b)| {
                std::array::from_fn(|k| if k < 4 { a[i][k] } else { b[i][k - 4] } as i8)
            }),
        };
        finite(
            name,
            "point",
            p.pos
                .iter()
                .chain(&p.tangent)
                .chain(&p.down)
                .chain(&p.lateral)
                .chain(&[
                    p.progress,
                    p.half_width_left,
                    p.half_width_right,
                    p.ai_bound_left,
                    p.ai_bound_right,
                    p.racing_line,
                ])
                .copied(),
        )?;
        out.push(p);
    }
    Ok(out)
}

fn read_pads(name: &str, node: &gltf::Node<'_>, blob: &[u8]) -> Result<Vec<PadVolume>, Error> {
    let prim = single_prim(name, node)?;
    let n = positions(name, &prim, blob)?.len();
    let matrix = items::<[[f32; 4]; 4]>(name, "_OAG_TO_WORLD", &prim, blob, "float MAT4")?;
    let min = items::<[f32; 3]>(name, "_OAG_MIN", &prim, blob, "float VEC3")?;
    let max = items::<[f32; 3]>(name, "_OAG_MAX", &prim, blob, "float VEC3")?;
    let disabled = items::<f32>(name, "_OAG_DISABLED", &prim, blob, "float SCALAR")?;
    same_len(name, "_OAG_TO_WORLD", matrix.len(), n)?;
    same_len(name, "_OAG_MIN", min.len(), n)?;
    same_len(name, "_OAG_MAX", max.len(), n)?;
    same_len(name, "_OAG_DISABLED", disabled.len(), n)?;
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let mut to_world = [0.0f32; 16];
        for (r, row) in matrix[i].iter().enumerate() {
            to_world[r * 4..r * 4 + 4].copy_from_slice(row);
        }
        let pad = PadVolume {
            to_world,
            min: min[i],
            max: max[i],
            disabled: disabled[i],
        };
        finite(
            name,
            "pad",
            pad.to_world
                .iter()
                .chain(&pad.min)
                .chain(&pad.max)
                .chain(&[pad.disabled])
                .copied(),
        )?;
        out.push(pad);
    }
    Ok(out)
}

fn read_collision(
    node: &gltf::Node<'_>,
    kind: SurfaceKind,
    extras: Option<&Value>,
    blob: &[u8],
    index: usize,
) -> Result<CollisionNode, Error> {
    let record = format!("collision node {index}");
    let mut meshes = Vec::new();
    if let Some(mesh) = node.mesh() {
        if !identity(node) {
            return Err(Error::Transformed { record });
        }
        for (m, prim) in mesh.primitives().enumerate() {
            let what = format!("{record} mesh {m}");
            if prim.mode() != gltf::mesh::Mode::Triangles {
                return Err(Error::accessor(&what, "mode", "not TRIANGLES"));
            }
            let vertices = positions(&what, &prim, blob)?;
            let scalars = match prim.get(&Semantic::Extras("OAG_SCALAR".into())) {
                Some(_) => items::<f32>(&what, "_OAG_SCALAR", &prim, blob, "float SCALAR")?,
                None => vec![1.0; vertices.len()],
            };
            same_len(&what, "_OAG_SCALAR", scalars.len(), vertices.len())?;
            let indices = prim
                .indices()
                .and_then(|a| Iter::<u16>::new(a, |_| Some(blob)))
                .ok_or_else(|| {
                    Error::accessor(&what, "indices", "missing or not unsigned short SCALAR")
                })?
                .collect::<Vec<u16>>();
            if indices.len() % 3 != 0 {
                return Err(Error::accessor(&what, "indices", "not a multiple of 3"));
            }
            if indices.iter().any(|&i| usize::from(i) >= vertices.len()) {
                return Err(Error::accessor(
                    &what,
                    "indices",
                    "an index is past the vertices",
                ));
            }
            finite(&what, "vertices", vertices.iter().flatten().copied())?;
            finite(&what, "vertex scalars", scalars.iter().copied())?;
            meshes.push(CollisionMesh {
                vertices,
                triangles: indices.as_chunks::<3>().0.to_vec(),
                vertex_scalars: scalars,
                chunks: Vec::new(),
            });
        }
    }
    let label = extras
        .and_then(|e| e.get("oag_name"))
        .and_then(Value::as_str)
        .map(str::to_string);
    let version = extras
        .and_then(|e| e.get("oag_version"))
        .and_then(Value::as_u64)
        .and_then(|v| u32::try_from(v).ok())
        .unwrap_or(HEADER_WORD);
    Ok(CollisionNode {
        kind,
        node_index: index,
        name: label,
        geometry: CollisionGeometry { version, meshes },
    })
}
