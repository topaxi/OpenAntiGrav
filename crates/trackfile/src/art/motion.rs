//! Keyframed motion of a track's art (ADR-0076 decision 10): glTF `animations`
//! whose channels move an art mesh node or turn the sun, played by the renderer
//! against the race clock. **Visual only**: nothing here reaches collision,
//! pads or the spline, and the simulation never reads it.
//!
//! The keys are stored as glTF carries them and never sampled here: sampling a
//! rotation is trigonometry, and this crate is in `check-determinism`'s scan
//! (decision 5). The renderer samples (`oag_mesh::mesh::Keyed`).

use crate::Error;

/// How far a rotation key may be from unit length before it is refused.
const UNIT_TOLERANCE: f32 = 1e-3;

/// One glTF animation: channels that share a clock.
#[derive(Debug, Clone, PartialEq)]
pub struct Animation {
    /// The author's name for it.
    pub name: String,
    /// What happens past its last key.
    pub repeat: Repeat,
    /// Its channels, in file order.
    pub channels: Vec<Channel>,
}

impl Animation {
    /// Its length in seconds: the latest key of any channel. A looping
    /// animation wraps on it, a clamped one holds there.
    #[must_use]
    pub fn duration(&self) -> f32 {
        self.channels
            .iter()
            .filter_map(|c| c.times.last().copied())
            .fold(0.0, f32::max)
    }
}

/// What an animation does past its last key: the `oag_loop` extra.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Repeat {
    /// Wraps on the animation's duration (`"loop"`, and absent).
    #[default]
    Loop,
    /// Holds the last key (`"clamp"`).
    Clamp,
}

impl Repeat {
    pub(crate) fn tag(self) -> &'static str {
        match self {
            Self::Loop => "loop",
            Self::Clamp => "clamp",
        }
    }

    pub(crate) fn from_tag(tag: &str) -> Option<Self> {
        [Self::Loop, Self::Clamp]
            .into_iter()
            .find(|r| r.tag() == tag)
    }
}

/// One channel: a sampler and the node property it drives.
#[derive(Debug, Clone, PartialEq)]
pub struct Channel {
    /// What it moves.
    pub target: Target,
    /// How it blends between keys.
    pub interpolation: Interpolation,
    /// Key times in seconds from the race clock's zero, strictly increasing.
    pub times: Vec<f32>,
    /// One key per time; the variant is the glTF `path`.
    pub keys: Keys,
}

/// The node a channel drives.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    /// An art mesh, by its index in `Art::meshes`.
    Mesh(usize),
    /// The sun (`oag/sun`): rotation only. Its direction at a time is the
    /// manifest's `direction` turned by the rotation from the node's rest
    /// rotation (`Art::sun_rotation`) to its rotation then.
    Sun,
}

/// glTF's `LINEAR` and `STEP`. `CUBICSPLINE` is refused by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Interpolation {
    /// Vectors blend linearly, rotations by spherical interpolation.
    #[default]
    Linear,
    /// Each key holds until the next.
    Step,
}

impl Interpolation {
    pub(crate) fn tag(self) -> &'static str {
        match self {
            Self::Linear => "LINEAR",
            Self::Step => "STEP",
        }
    }
}

/// The keys of a channel, by the property they drive.
#[derive(Debug, Clone, PartialEq)]
pub enum Keys {
    /// Translation, world units.
    Translation(Vec<[f32; 3]>),
    /// Rotation, a unit quaternion `x, y, z, w` as glTF orders it.
    Rotation(Vec<[f32; 4]>),
    /// Scale per axis.
    Scale(Vec<[f32; 3]>),
}

impl Keys {
    /// The glTF `path`.
    #[must_use]
    pub fn path(&self) -> &'static str {
        match self {
            Self::Translation(_) => "translation",
            Self::Rotation(_) => "rotation",
            Self::Scale(_) => "scale",
        }
    }

    fn len(&self) -> usize {
        match self {
            Self::Translation(k) | Self::Scale(k) => k.len(),
            Self::Rotation(k) => k.len(),
        }
    }

    fn values(&self) -> Vec<f32> {
        match self {
            Self::Translation(k) | Self::Scale(k) => k.iter().flatten().copied().collect(),
            Self::Rotation(k) => k.iter().flatten().copied().collect(),
        }
    }
}

/// A moving mesh node's rest pose, glTF's `translation`, `rotation` and
/// `scale`. Its vertices are in the node's own space; a property no channel
/// drives stays at this.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Trs {
    /// Translation, world units.
    pub translation: [f32; 3],
    /// Rotation, a unit quaternion `x, y, z, w`.
    pub rotation: [f32; 4],
    /// Scale per axis.
    pub scale: [f32; 3],
}

impl Trs {
    /// No transform at all.
    pub const IDENTITY: Self = Self {
        translation: [0.0; 3],
        rotation: [0.0, 0.0, 0.0, 1.0],
        scale: [1.0; 3],
    };
}

fn range(what: &str, why: impl Into<String>) -> Error {
    Error::Range {
        what: what.to_string(),
        why: why.into(),
    }
}

fn quaternion(what: &str, q: [f32; 4]) -> Result<(), Error> {
    let len = q.iter().map(|c| c * c).sum::<f32>().sqrt();
    if (len - 1.0).abs() > UNIT_TOLERANCE {
        return Err(range(what, format!("a rotation of length {len}, not 1")));
    }
    Ok(())
}

/// Checks a rest pose: finite, a unit rotation.
pub(crate) fn validate_trs(what: &str, trs: &Trs) -> Result<(), Error> {
    if !trs
        .translation
        .iter()
        .chain(&trs.rotation)
        .chain(&trs.scale)
        .all(|v| v.is_finite())
    {
        return Err(Error::NonFinite {
            record: what.to_string(),
            field: "transform".into(),
        });
    }
    quaternion(what, trs.rotation)
}

/// Checks the animations against the art they move: keys well formed, every
/// target there, each node property driven by one channel at most, the sun
/// turned only, a rest pose exactly where something moves.
pub(crate) fn validate(art: &crate::art::Art) -> Result<(), Error> {
    let mut driven: Vec<(Target, &'static str)> = Vec::new();
    for (a, anim) in art.animations.iter().enumerate() {
        for (c, ch) in anim.channels.iter().enumerate() {
            let what = format!("art animation {a} ({}) channel {c}", anim.name);
            if ch.times.is_empty() {
                return Err(range(&what, "no keys"));
            }
            if ch.keys.len() != ch.times.len() {
                return Err(range(
                    &what,
                    format!("{} keys for {} times", ch.keys.len(), ch.times.len()),
                ));
            }
            if !ch
                .times
                .iter()
                .chain(&ch.keys.values())
                .all(|v| v.is_finite())
            {
                return Err(Error::NonFinite {
                    record: what,
                    field: "keys".into(),
                });
            }
            if ch.times[0] < 0.0 || ch.times.windows(2).any(|w| w[0] >= w[1]) {
                return Err(range(
                    &what,
                    "key times must start at or after 0 and strictly increase",
                ));
            }
            if let Keys::Rotation(keys) = &ch.keys {
                for &q in keys {
                    quaternion(&what, q)?;
                }
            }
            match ch.target {
                Target::Mesh(m) if m >= art.meshes.len() => {
                    return Err(range(
                        &what,
                        format!("mesh {m}, the file has {}", art.meshes.len()),
                    ));
                }
                Target::Sun if art.sun.is_none() => {
                    return Err(range(&what, "it turns the sun and the file has no sun"));
                }
                Target::Sun if !matches!(ch.keys, Keys::Rotation(_)) => {
                    return Err(range(
                        &what,
                        format!("the sun takes rotation only, not {}", ch.keys.path()),
                    ));
                }
                _ => {}
            }
            let key = (ch.target, ch.keys.path());
            if driven.contains(&key) {
                return Err(range(
                    &what,
                    format!("a second channel drives {:?} {}", ch.target, ch.keys.path()),
                ));
            }
            driven.push(key);
        }
    }
    for (n, m) in art.meshes.iter().enumerate() {
        let moved = driven.iter().any(|(t, _)| *t == Target::Mesh(n));
        match (&m.transform, moved) {
            (Some(trs), true) => validate_trs(&format!("art mesh {n} ({})", m.name), trs)?,
            (Some(_), false) => {
                return Err(range(
                    &format!("art mesh {n} ({})", m.name),
                    "a transform on a mesh no animation moves; apply it before export",
                ));
            }
            (None, _) => {}
        }
    }
    let turned = driven.iter().any(|(t, _)| *t == Target::Sun);
    match (art.sun_rotation, turned) {
        (Some(q), true) => quaternion("art sun node", q)?,
        (None, true) => {
            return Err(range(
                "art sun node",
                "the sun turns and has no rest rotation",
            ));
        }
        (Some(_), false) => {
            return Err(range(
                "art sun node",
                "a rest rotation and no animation turns the sun",
            ));
        }
        (None, false) => {}
    }
    Ok(())
}
