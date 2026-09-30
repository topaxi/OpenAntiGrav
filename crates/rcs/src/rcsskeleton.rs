//! The Vita's `.rcsskeleton`: the node hierarchy a `.rcsmodel`'s node-bound
//! meshes hang from, with each node's bind transform.
//!
//! Every 2048 circuit ships one beside its `track.rcsmodel` (and one more
//! for `trackZone` and each `trackpart_startgridanims`), in the same
//! [`container`](crate::rcsmodel::psp2::container) the model uses. Its one
//! section is a serialized node list:
//!
//! ```text
//! section, from its own start:
//!   +0x00  u32   2 - unread, constant on all 49 files
//!   +0x04  u32   node count
//!   +0x0c  u32   offset of u32[count]: node ids, the same values the
//!                model's node table and the clip carry - see
//!                [`crate::rcsmodel::psp2::nodes`]
//!   +0x10  u32   offset of u32[count]: each node's parent index, or the
//!                node's own index for a root
//!   +0x14  u32   offset of the property block: a u32 slot count (9), then
//!                per node { u32 offset of u32[9] property offsets; u32 9 }
//!   +0x18  u32   offset of f32[16][count]: for a root, the world matrix
//!                of whatever static transform sits above it - a group the
//!                table does not list; for a child, unread (it equals the
//!                parent's bind world on 89 of `altima`'s 112 and not on the
//!                rest, and nothing here needs it)
//!
//! one property:
//!   +0x00  u32   (type << 8) | slot in the low 16 bits; the high 16 vary
//!                per file and look uninitialised
//!   +0x04  u32   offset of the value
//! ```
//!
//! The nine slots are the same nine a `.rcsanimclip` channel names, and six
//! are read: `0` scale (vec3), `1` rotation (quaternion `x, y, z, w`), `2`
//! translation (vec3), `3` visibility (a u32 `0`/`1` here, one byte per key
//! in a clip), `4` the rotate/scale pivot (vec3) and `5` the pivot's
//! translate (vec3). Slots 6 to 8 are scalars 21 nodes across two
//! `startgridanims` files carry and nothing here interprets. A node's local
//! matrix composes as Maya composes a transform whose rotate pivot is set
//! and whose scale pivot is not:
//! `S * T(-pivot) * R * T(pivot) * T(pivot_translate) * T(translation)`,
//! row-vector order - see [`Node::local`] for why that and not the bare
//! `S * R * T` the model's own bind matrices were composed with.
//!
//! Confidence and the cross-title measurement behind each field are in
//! `docs/formats/2048-animation.md`.
//!
//! # The PS4's is the same file with 64-bit pointers
//!
//! Told apart the way the model is, by header word `+0x04`
//! ([`crate::rcsmodel::psp2::is_ps4`]). Every offset above is 8 bytes wide and
//! the header pointers move to `+0x10`, `+0x18`, `+0x20` and `+0x28` (the count
//! stays at `+0x04`); the property block opens with a 64-bit slot count and then
//! `{ u64 offset of u64[9]; u64 9 }` per node, and a property is
//! `{ u32 tag; u32 pad; u64 offset of the value }`. The id and parent arrays,
//! the matrices and every value stay 4-byte. Measured on
//! `tech_de_ra\track.final.rcsskeleton` (168 nodes: the arrays and the matrix
//! table close on the section's own length to the byte) and confirmed corpus-wide
//! by `crates/rcs/tests/omega_animation_ground_truth.rs`.

use crate::rcsmodel::psp2::container::{self, u32_at};
use crate::rcsmodel::psp2::{Error, Result};

/// The property slot a scale lives in.
pub const SLOT_SCALE: usize = 0;
/// The property slot a rotation lives in.
pub const SLOT_ROTATION: usize = 1;
/// The property slot a translation lives in.
pub const SLOT_TRANSLATION: usize = 2;
/// The property slot visibility lives in.
pub const SLOT_VISIBILITY: usize = 3;
/// The property slot the pivot lives in.
pub const SLOT_PIVOT: usize = 4;
/// The property slot the pivot translate lives in.
pub const SLOT_PIVOT_TRANSLATE: usize = 5;
/// How many property slots a node has.
pub const SLOTS: usize = 9;

/// How wide a container's offsets are: the Vita's are `u32` and the PS4's are
/// `u64`, and that is the whole difference between the two layouts of a
/// skeleton or a clip apart from where the fields sit.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Wire {
    /// Whether the file is a PS4 one.
    pub ps4: bool,
}

impl Wire {
    /// The layout of `file`, by the model's own discriminator.
    pub(crate) fn of(file: &[u8]) -> Self {
        Self {
            ps4: crate::rcsmodel::psp2::is_ps4(file),
        }
    }

    /// Bytes per offset.
    pub(crate) fn width(self) -> usize {
        if self.ps4 { 8 } else { 4 }
    }

    /// The offset stored at `at`.
    pub(crate) fn offset(self, section: &[u8], at: usize, what: &'static str) -> Result<usize> {
        let low = u32_at(section, at, what)? as usize;
        if !self.ps4 {
            return Ok(low);
        }
        // The high half is zero on every file measured; a value that does not
        // fit is refused rather than truncated into a plausible offset.
        match u32_at(section, at + 4, what)? {
            0 => Ok(low),
            high => Err(Error::OutOfBounds {
                what,
                end: (high as usize)
                    .saturating_mul(1 << 16)
                    .saturating_mul(1 << 16),
                len: section.len(),
            }),
        }
    }
}

/// A property's value type, the byte above the slot in its tag word.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// One `f32`.
    Scalar,
    /// Three `f32`.
    Vec3,
    /// Four `f32`, a quaternion as `x, y, z, w`.
    Quat,
    /// A boolean: a `u32` in the skeleton, one byte per key in a clip.
    Bool,
}

impl Kind {
    /// The kind a tag's type byte names, or `None` for one not seen on the
    /// disc.
    #[must_use]
    pub fn from_type(byte: u8) -> Option<Self> {
        match byte {
            0 => Some(Self::Scalar),
            1 => Some(Self::Vec3),
            3 => Some(Self::Quat),
            4 => Some(Self::Bool),
            _ => None,
        }
    }

    /// Bytes per key of this kind in a clip's key array.
    #[must_use]
    pub fn key_len(self) -> usize {
        match self {
            Self::Scalar => 4,
            Self::Vec3 => 12,
            Self::Quat => 16,
            Self::Bool => 1,
        }
    }
}

/// The identity matrix, row-major.
pub const IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

/// One node's bind pose.
#[derive(Debug, Clone, PartialEq)]
pub struct Node {
    /// The id the model's node table and the clip bind by.
    pub id: u32,
    /// Index of the parent in [`Skeleton::nodes`], `None` for a root.
    pub parent: Option<usize>,
    /// Scale, slot 0.
    pub scale: [f32; 3],
    /// Rotation as `x, y, z, w`, slot 1.
    pub rotation: [f32; 4],
    /// Translation, slot 2.
    pub translation: [f32; 3],
    /// Visibility, slot 3.
    pub visible: bool,
    /// The rotate and scale pivot, slot 4.
    pub pivot: [f32; 3],
    /// The pivot translate, slot 5.
    pub pivot_translate: [f32; 3],
    /// For a root, the world matrix of the static transform above it; the
    /// identity where there is none. Unread for a child - see the module
    /// doc.
    pub above: [f32; 16],
    /// Which slots the node's property block filled, by kind - the six
    /// above plus any scalar slot this reading carries no field for.
    pub kinds: [Option<Kind>; SLOTS],
}

impl Node {
    /// The node's local matrix from its own bind values, row-major with the
    /// translation in row 3.
    ///
    /// `S * T(-pivot) * R * T(pivot) * T(pivot_translate) * T(translation)`.
    /// **The pivot is load-bearing**: `altima`'s wind-turbine rotors carry
    /// their geometry 1,300 units from the node origin with the pivot at
    /// the geometry's centre, and without it each rotor orbits its tower
    /// at that radius instead of spinning on it. Measured against Wipeout
    /// HD on Anulpha Pass, where the same nodes are `Anim Transform`s with
    /// the pivot baked in: this form lands 76 of 86 shared nodes on HD's
    /// own time-zero world matrix, the bare `S * R * T` 63.
    #[must_use]
    pub fn local(&self) -> [f32; 16] {
        local_matrix(
            self.scale,
            self.rotation,
            self.translation,
            self.pivot,
            self.pivot_translate,
        )
    }
}

/// A decoded `.rcsskeleton`.
#[derive(Debug, Clone, PartialEq)]
pub struct Skeleton {
    /// Every node, in file order. A parent's index is always lower than its
    /// child's on 48 of 49 shipped files; [`Skeleton::order`] handles the
    /// one that is not.
    pub nodes: Vec<Node>,
}

impl Skeleton {
    /// Index of the node with `id`, if any.
    #[must_use]
    pub fn node_by_id(&self, id: u32) -> Option<usize> {
        self.nodes.iter().position(|n| n.id == id)
    }

    /// Node indices in an order where every parent precedes its children.
    #[must_use]
    pub fn order(&self) -> Vec<usize> {
        let n = self.nodes.len();
        let mut out = Vec::with_capacity(n);
        let mut placed = vec![false; n];
        // Each pass places every node whose parent is already placed; a
        // cycle (never observed) simply stops when a pass places nothing.
        loop {
            let before = out.len();
            for i in 0..n {
                if placed[i] {
                    continue;
                }
                if self.nodes[i].parent.is_none_or(|p| placed[p]) {
                    placed[i] = true;
                    out.push(i);
                }
            }
            if out.len() == before || out.len() == n {
                break;
            }
        }
        out
    }

    /// Every node's world matrix at bind, composed through the hierarchy:
    /// `local * parent_world`, with a root's parent being [`Node::above`].
    #[must_use]
    pub fn bind_world(&self) -> Vec<[f32; 16]> {
        let mut world = vec![IDENTITY; self.nodes.len()];
        for i in self.order() {
            let node = &self.nodes[i];
            let parent = node.parent.map_or(node.above, |p| world[p]);
            world[i] = multiply(&node.local(), &parent);
        }
        world
    }
}

/// Decodes a `.rcsskeleton`.
///
/// # Errors
///
/// The container's own refusals, plus any offset that runs past the section.
pub fn parse(file: &[u8]) -> Result<Skeleton> {
    let header = container::read(file)?;
    let section = header.section(file, 0).ok_or(Error::OutOfBounds {
        what: "skeleton section",
        end: 0,
        len: file.len(),
    })?;
    let wire = Wire::of(file);
    let w = wire.width();
    // The count stays at `+0x04`; the four offsets follow it in the Vita's
    // header at `+0x0c` and, 8 bytes wide, from `+0x10` in the PS4's.
    let first = if wire.ps4 { 0x10 } else { 0x0c };
    let count = u32_at(section, 0x04, "node count")? as usize;
    let ids_at = wire.offset(section, first, "node ids")?;
    let parents_at = wire.offset(section, first + w, "node parents")?;
    let props_at = wire.offset(section, first + 2 * w, "node properties")?;
    let above_at = wire.offset(section, first + 3 * w, "node parent matrices")?;
    let slots = wire.offset(section, props_at, "slot count")?;

    let mut nodes = Vec::with_capacity(count.min(section.len() / 8));
    for i in 0..count {
        let id = u32_at(section, ids_at + i * 4, "node id")?;
        let parent = u32_at(section, parents_at + i * 4, "node parent")? as usize;
        let parent = (parent != i && parent < count).then_some(parent);
        // `{ offset of the property table; slot count }` per node, after the
        // block's own slot count.
        let table = wire.offset(section, props_at + w + i * 2 * w, "property table")?;
        let above = matrix_at(section, above_at + i * 64, "parent matrix")?;
        let mut node = Node {
            id,
            parent,
            scale: [1.0; 3],
            rotation: [0.0, 0.0, 0.0, 1.0],
            translation: [0.0; 3],
            visible: true,
            pivot: [0.0; 3],
            pivot_translate: [0.0; 3],
            above,
            kinds: [None; SLOTS],
        };
        for slot in 0..slots.min(SLOTS) {
            let property = wire.offset(section, table + slot * w, "property")?;
            if property == 0 {
                continue;
            }
            let tag = u32_at(section, property, "property tag")?;
            // The value's offset follows the tag: at `+0x04` in the Vita's
            // property, at `+0x08` (8-aligned) in the PS4's.
            let value = wire.offset(section, property + w, "property value")?;
            let kind = Kind::from_type(((tag >> 8) & 0xff) as u8);
            node.kinds[slot] = kind;
            match (slot, kind) {
                (SLOT_SCALE, Some(Kind::Vec3)) => node.scale = vec3_at(section, value)?,
                (SLOT_ROTATION, Some(Kind::Quat)) => node.rotation = quat_at(section, value)?,
                (SLOT_TRANSLATION, Some(Kind::Vec3)) => node.translation = vec3_at(section, value)?,
                (SLOT_VISIBILITY, Some(Kind::Bool)) => {
                    node.visible = u32_at(section, value, "visibility")? != 0;
                }
                (SLOT_PIVOT, Some(Kind::Vec3)) => node.pivot = vec3_at(section, value)?,
                (SLOT_PIVOT_TRANSLATE, Some(Kind::Vec3)) => {
                    node.pivot_translate = vec3_at(section, value)?;
                }
                _ => {}
            }
        }
        nodes.push(node);
    }
    Ok(Skeleton { nodes })
}

/// `S * T(-pivot) * R * T(pivot) * T(pivot_translate) * T(translation)`, in
/// the row-vector convention every matrix in this crate uses: the scale is
/// about the node's origin and only the rotation is about the pivot.
///
/// Scaling about the pivot too - Maya's form when its scale pivot equals
/// its rotate pivot - fits every node with a unit scale identically and
/// misses Metropia's `pCylinder208_1` (scale 17.3, pivot 17.3 up, no
/// rotation) by 282 units where this form lands it on HD's matrix exactly.
#[must_use]
pub fn local_matrix(
    scale: [f32; 3],
    rotation: [f32; 4],
    translation: [f32; 3],
    pivot: [f32; 3],
    pivot_translate: [f32; 3],
) -> [f32; 16] {
    let rotate = quat_matrix(rotation);
    let mut m = rotate;
    // Scale, then rotate: scale each basis row.
    for row in 0..3 {
        for col in 0..3 {
            m[row * 4 + col] *= scale[row];
        }
    }
    // A translation composed on the right adds to row 3; the pivot's lead is
    // composed between the scale and the rotation, so it runs through the
    // rotation alone.
    let lead = [-pivot[0], -pivot[1], -pivot[2]];
    let led: [f32; 3] = std::array::from_fn(|col| {
        lead[0] * rotate[col] + lead[1] * rotate[4 + col] + lead[2] * rotate[8 + col]
    });
    for k in 0..3 {
        m[12 + k] = led[k] + pivot[k] + pivot_translate[k] + translation[k];
    }
    m
}

/// A unit quaternion `x, y, z, w` as a row-major rotation matrix.
#[must_use]
pub fn quat_matrix(q: [f32; 4]) -> [f32; 16] {
    let [x, y, z, w] = q;
    [
        1.0 - 2.0 * (y * y + z * z),
        2.0 * (x * y + z * w),
        2.0 * (x * z - y * w),
        0.0,
        2.0 * (x * y - z * w),
        1.0 - 2.0 * (x * x + z * z),
        2.0 * (y * z + x * w),
        0.0,
        2.0 * (x * z + y * w),
        2.0 * (y * z - x * w),
        1.0 - 2.0 * (x * x + y * y),
        0.0,
        0.0,
        0.0,
        0.0,
        1.0,
    ]
}

/// `a * b` for two row-major matrices: apply `a`, then `b`.
#[must_use]
pub fn multiply(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
    let mut out = [0.0f32; 16];
    for row in 0..4 {
        for col in 0..4 {
            let mut acc = 0.0f32;
            for k in 0..4 {
                acc += a[row * 4 + k] * b[k * 4 + col];
            }
            out[row * 4 + col] = acc;
        }
    }
    out
}

fn f32_at(section: &[u8], at: usize, what: &'static str) -> Result<f32> {
    u32_at(section, at, what).map(f32::from_bits)
}

fn vec3_at(section: &[u8], at: usize) -> Result<[f32; 3]> {
    Ok([
        f32_at(section, at, "vec3")?,
        f32_at(section, at + 4, "vec3")?,
        f32_at(section, at + 8, "vec3")?,
    ])
}

fn quat_at(section: &[u8], at: usize) -> Result<[f32; 4]> {
    Ok([
        f32_at(section, at, "quaternion")?,
        f32_at(section, at + 4, "quaternion")?,
        f32_at(section, at + 8, "quaternion")?,
        f32_at(section, at + 12, "quaternion")?,
    ])
}

fn matrix_at(section: &[u8], at: usize, what: &'static str) -> Result<[f32; 16]> {
    let mut out = [0.0f32; 16];
    for (k, slot) in out.iter_mut().enumerate() {
        *slot = f32_at(section, at + k * 4, what)?;
    }
    Ok(out)
}

#[cfg(test)]
pub(crate) mod tests;
