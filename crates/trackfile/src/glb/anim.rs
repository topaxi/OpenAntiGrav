//! The art's motion in the glb (ADR-0076 decision 10): a moving mesh node's
//! rest pose as its glTF `translation`, `rotation` and `scale`, the sun as an
//! `oag/sun` node carrying only a rest rotation, and glTF `animations` whose
//! samplers are float accessors, read and written bit for bit. `oag_loop`
//! (`loop`, `clamp`) is an extra on the animation. Nothing is sampled here.

use gltf::accessor::{DataType, Dimensions, Item, Iter};
use gltf::animation::{Interpolation as GltfInterpolation, Property};
use serde_json::{Value, json};

use super::{ARRAY_BUFFER, Bin, Error, FLOAT};
use crate::art::Art;
use crate::art::motion::{Animation, Channel, Interpolation, Keys, Repeat, Target, Trs};

/// The sun's node: no mesh, a rest rotation.
pub(super) const SUN: &str = "oag/sun";

/// The rest pose on `node`, glTF's decomposed form; identity parts are left out.
pub(super) fn set_trs(node: &mut Value, trs: &Trs) {
    if trs.translation != Trs::IDENTITY.translation {
        node["translation"] = json!(trs.translation);
    }
    if trs.rotation != Trs::IDENTITY.rotation {
        node["rotation"] = json!(trs.rotation);
    }
    if trs.scale != Trs::IDENTITY.scale {
        node["scale"] = json!(trs.scale);
    }
}

/// A node's decomposed transform, or `None` for one given as a matrix that is
/// not the identity (refused by the caller).
pub(super) fn trs_of(node: &gltf::Node<'_>) -> Option<Trs> {
    match node.transform() {
        gltf::scene::Transform::Decomposed {
            translation,
            rotation,
            scale,
        } => Some(Trs {
            translation,
            rotation,
            scale,
        }),
        gltf::scene::Transform::Matrix { .. } => None,
    }
}

/// The glTF `animations` array for `art`, with `mesh_nodes[i]` the glb node of
/// art mesh `i` and `sun_node` the sun's. Empty when nothing moves, so a file
/// with no motion is byte for byte what it was before motion existed.
pub(super) fn write(
    bin: &mut Bin,
    art: &Art,
    mesh_nodes: &[usize],
    sun_node: Option<usize>,
) -> Vec<Value> {
    let mut out = Vec::new();
    for anim in &art.animations {
        let mut samplers = Vec::new();
        let mut channels = Vec::new();
        for ch in &anim.channels {
            let node = match ch.target {
                Target::Mesh(m) => mesh_nodes[m],
                Target::Sun => sun_node.expect("validated: a turned sun has a node"),
            };
            let lo = f64::from(ch.times[0]);
            let hi = f64::from(*ch.times.last().expect("validated: keys"));
            let bytes: Vec<u8> = ch.times.iter().flat_map(|t| t.to_le_bytes()).collect();
            let input = bin.add(&bytes, FLOAT, "SCALAR", ch.times.len(), ARRAY_BUFFER, None);
            bin.accessors[input]["min"] = json!([lo]);
            bin.accessors[input]["max"] = json!([hi]);
            let output = match &ch.keys {
                Keys::Translation(k) | Keys::Scale(k) => bin.floats(k, "VEC3"),
                Keys::Rotation(k) => bin.floats(k, "VEC4"),
            };
            samplers.push(json!({
                "input": input, "output": output,
                "interpolation": ch.interpolation.tag(),
            }));
            channels.push(json!({
                "sampler": samplers.len() - 1,
                "target": { "node": node, "path": ch.keys.path() },
            }));
        }
        out.push(json!({
            "name": anim.name,
            "channels": channels,
            "samplers": samplers,
            "extras": { "oag_loop": anim.repeat.tag() },
        }));
    }
    out
}

fn floats<T: Item>(
    what: &str,
    accessor: gltf::Accessor<'_>,
    dims: Dimensions,
    blob: &[u8],
) -> Result<Vec<T>, Error> {
    if accessor.data_type() != DataType::F32 || accessor.dimensions() != dims {
        return Err(Error::accessor(
            what,
            "sampler",
            format!("not float {dims:?} (normalised integer keys are not read)"),
        ));
    }
    Iter::<T>::new(accessor, |_| Some(blob))
        .map(Iterator::collect)
        .ok_or_else(|| Error::accessor(what, "sampler", "unreadable accessor"))
}

/// Every animation of the document, its targets resolved through
/// `target_of` (a glb node index to an art mesh or the sun; `None` for any
/// other node, which is refused: only art and the sun move in 1.x).
pub(super) fn read(
    doc: &gltf::Document,
    blob: &[u8],
    target_of: impl Fn(usize) -> Option<Target>,
) -> Result<Vec<Animation>, Error> {
    let mut out = Vec::new();
    for (a, anim) in doc.animations().enumerate() {
        let name = anim.name().unwrap_or("").to_string();
        let what = format!("animation {a} ({name})");
        let extras: Option<Value> = anim
            .extras()
            .as_ref()
            .and_then(|raw| serde_json::from_str(raw.get()).ok());
        let repeat = match extras.as_ref().and_then(|e| e.get("oag_loop")) {
            None => Repeat::Loop,
            Some(v) => v.as_str().and_then(Repeat::from_tag).ok_or_else(|| {
                Error::accessor(
                    &what,
                    "oag_loop",
                    format!("{v} is not \"loop\" or \"clamp\""),
                )
            })?,
        };
        let mut channels = Vec::new();
        for (c, ch) in anim.channels().enumerate() {
            let what = format!("{what} channel {c}");
            let node = ch.target().node();
            let target = target_of(node.index()).ok_or_else(|| Error::Range {
                what: what.clone(),
                why: format!(
                    "it moves node {:?}; only art meshes and the sun move (collision, pads and \
                     the spline never do)",
                    node.name().unwrap_or("")
                ),
            })?;
            let sampler = ch.sampler();
            let interpolation = match sampler.interpolation() {
                GltfInterpolation::Linear => Interpolation::Linear,
                GltfInterpolation::Step => Interpolation::Step,
                GltfInterpolation::CubicSpline => {
                    return Err(Error::accessor(
                        &what,
                        "interpolation",
                        "CUBICSPLINE is not read; sample it to LINEAR keys",
                    ));
                }
            };
            let times: Vec<f32> = floats(&what, sampler.input(), Dimensions::Scalar, blob)?;
            let output = sampler.output();
            let keys = match ch.target().property() {
                Property::Translation => {
                    Keys::Translation(floats(&what, output, Dimensions::Vec3, blob)?)
                }
                Property::Rotation => {
                    Keys::Rotation(floats(&what, output, Dimensions::Vec4, blob)?)
                }
                Property::Scale => Keys::Scale(floats(&what, output, Dimensions::Vec3, blob)?),
                Property::MorphTargetWeights => {
                    return Err(Error::accessor(&what, "path", "morph weights are not read"));
                }
            };
            channels.push(Channel {
                target,
                interpolation,
                times,
                keys,
            });
        }
        out.push(Animation {
            name,
            repeat,
            channels,
        });
    }
    Ok(out)
}
