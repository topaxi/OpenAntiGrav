//! A track file's keyframed motion (ADR-0076 decision 10) as what the renderer
//! already plays: one [`AnimNode`] per moving mesh, its vertices tagged with
//! the node's slot ([`GpuVertex::xform`](oag_mesh::mesh::GpuVertex)), so the
//! per-frame node table, the unculled moving-draw path and the `mapped` tier's
//! moving casters all take it as they take a disc circuit's `Anim Transform`.
//! The sun turns through [`SunTurn`]. The clock is the scene's animation clock
//! (the race tick, or `--anim-seconds`), never the wall clock.

use oag_mesh::mesh::{AnimNode, KeyTrack, Keyed, Model, Motion, NODE_ANIM_LIMIT};
use oag_trackfile::art::Art;
use oag_trackfile::art::motion::{Interpolation, Keys, Repeat, Target, Trs};
use oag_vex::vex;

/// The tracks that drive `target`, with `rest` for what none drives.
fn keyed(art: &Art, target: Target, rest: Trs) -> Option<Keyed> {
    let mut out = Keyed {
        translation: rest.translation,
        rotation: rest.rotation,
        scale: rest.scale,
        translation_keys: None,
        rotation_keys: None,
        scale_keys: None,
    };
    let mut any = false;
    for anim in &art.animations {
        let period = anim.duration();
        let clamp = anim.repeat == Repeat::Clamp;
        for ch in anim.channels.iter().filter(|c| c.target == target) {
            any = true;
            let step = ch.interpolation == Interpolation::Step;
            let times = ch.times.clone();
            match &ch.keys {
                Keys::Translation(v) => {
                    out.translation_keys = Some(KeyTrack {
                        times,
                        values: v.clone(),
                        step,
                        period,
                        clamp,
                    });
                }
                Keys::Rotation(v) => {
                    out.rotation_keys = Some(KeyTrack {
                        times,
                        values: v.clone(),
                        step,
                        period,
                        clamp,
                    });
                }
                Keys::Scale(v) => {
                    out.scale_keys = Some(KeyTrack {
                        times,
                        values: v.clone(),
                        step,
                        period,
                        clamp,
                    });
                }
            }
        }
    }
    any.then_some(out)
}

/// Every time any track of `node` has a key, and zero.
fn key_times(node: &Keyed) -> Vec<f32> {
    let mut times = vec![0.0];
    for t in [
        node.translation_keys.as_ref().map(|k| &k.times),
        node.rotation_keys.as_ref().map(|k| &k.times),
        node.scale_keys.as_ref().map(|k| &k.times),
    ]
    .into_iter()
    .flatten()
    {
        times.extend(t);
    }
    times
}

/// The moving meshes' nodes, deduplicated (the add-on writes one art mesh per
/// material slot of an object, each with the same tracks), and each mesh's
/// slot: `0` for a mesh that stands still, else its node's index plus one.
/// Nodes past the table's ceiling stand still at their rest pose, reported.
pub fn nodes(art: &Art, report: &mut Vec<String>) -> (Vec<AnimNode>, Vec<u32>) {
    let mut nodes: Vec<AnimNode> = Vec::new();
    let mut slots = Vec::with_capacity(art.meshes.len());
    let mut frozen = 0usize;
    for (n, m) in art.meshes.iter().enumerate() {
        let Some(k) = keyed(art, Target::Mesh(n), m.transform.unwrap_or(Trs::IDENTITY)) else {
            slots.push(0);
            continue;
        };
        let motion = Motion::Keyed(Box::new(k));
        let at = nodes.iter().position(|node| node.transform == motion);
        let slot = match at {
            Some(at) => at,
            None if nodes.len() < NODE_ANIM_LIMIT - 1 => {
                nodes.push(AnimNode {
                    transform: motion,
                    static_above: vex::IDENTITY,
                    parent: None,
                });
                nodes.len() - 1
            }
            None => {
                frozen += 1;
                slots.push(0);
                continue;
            }
        };
        slots.push(slot as u32 + 1);
    }
    if !art.animations.is_empty() {
        let moving = slots.iter().filter(|&&s| s != 0).count();
        report.push(format!(
            "art motion: {} animation(s), {moving} mesh(es) on {} node(s){}",
            art.animations.len(),
            nodes.len(),
            if frozen == 0 {
                String::new()
            } else {
                format!("; {frozen} past the {NODE_ANIM_LIMIT}-node table stand still")
            }
        ));
    }
    (nodes, slots)
}

/// Each moving vertex at every key time of its node: where the model's
/// sphere (far plane, framing) must reach. Linear keys put a translation's
/// extremes on a key; a rotation's between keys are not chased (chosen).
pub fn swept_positions(model: &Model) -> Vec<[f32; 3]> {
    let mut out = Vec::new();
    for (slot, node) in model.anim_nodes.iter().enumerate() {
        let Motion::Keyed(k) = &node.transform else {
            continue;
        };
        let xform = slot as u32 + 1;
        for t in key_times(k) {
            let m = node.transform.sample(t);
            out.extend(
                model
                    .vertices
                    .iter()
                    .filter(|v| v.xform == xform)
                    .map(|v| vex::transform_point(&m, v.position)),
            );
        }
    }
    out
}

/// A turning sun: its direction at a time is the file's direction turned by
/// the rotation from the sun node's rest rotation to its rotation then, so
/// the lamp's own axis convention never enters.
#[derive(Debug, Clone, PartialEq)]
pub struct SunTurn {
    /// The file's `[art.sun] direction`, at the rest rotation.
    pub direction: [f32; 3],
    /// The sun node's rest rotation.
    pub rest: [f32; 4],
    /// Its rotation keys.
    pub keys: Box<Keyed>,
}

impl SunTurn {
    /// The turning sun of `art`, if an animation turns it.
    #[must_use]
    pub fn of(art: &Art) -> Option<Self> {
        let sun = art.sun?;
        let rest = art.sun_rotation?;
        let keys = keyed(
            art,
            Target::Sun,
            Trs {
                rotation: rest,
                ..Trs::IDENTITY
            },
        )?;
        Some(Self {
            direction: sun.direction,
            rest,
            keys: Box::new(keys),
        })
    }

    /// Towards the sun at `seconds`.
    #[must_use]
    pub fn direction_at(&self, seconds: f32) -> [f32; 3] {
        let q = self.keys.rotation_at(seconds);
        let turn = multiply(q, conjugate(self.rest));
        rotate(turn, self.direction)
    }
}

fn conjugate([x, y, z, w]: [f32; 4]) -> [f32; 4] {
    [-x, -y, -z, w]
}

/// `a * b`, Hamilton's product, `x, y, z, w`: `b` applied first.
fn multiply(a: [f32; 4], b: [f32; 4]) -> [f32; 4] {
    let [ax, ay, az, aw] = a;
    let [bx, by, bz, bw] = b;
    [
        aw * bx + ax * bw + ay * bz - az * by,
        aw * by - ax * bz + ay * bw + az * bx,
        aw * bz + ax * by - ay * bx + az * bw,
        aw * bw - ax * bx - ay * by - az * bz,
    ]
}

/// `v` turned by the unit quaternion `q`.
fn rotate(q: [f32; 4], v: [f32; 3]) -> [f32; 3] {
    let p = multiply(multiply(q, [v[0], v[1], v[2], 0.0]), conjugate(q));
    [p[0], p[1], p[2]]
}
