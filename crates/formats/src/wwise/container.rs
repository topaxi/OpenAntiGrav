//! Sound containers: random/sequence (type 5) and switch (type 6) objects, the
//! two that decide which child of a target plays.
//!
//! A container body is the node's base parameters (the layout [`super::hirc::parent`]
//! reads the head of) and then what its kind adds. After the two property
//! bundles the base parameters continue:
//!
//! ```text
//! u8 positioning     bits 6 and 7 set; bits 3 or 4 set: u8 1, u32 attenuation follow
//! u8 sends           bit 3 set: four u32 bus ids follow
//! u8, u8, u16, u8, u8   advanced settings, unread
//! u32 groups, each { u32 id, u8, u16 n, n * { u32, u32 } }   state groups
//! u16 rtpc, each { u32, u8, u8, u8, u32, u8, u16 points, points * 12 bytes }
//! ```
//!
//! Only those shapes are read; any other positioning is refused by name
//! ([`ContainerError::BaseParams`]). The kind's own fields then follow and must
//! end the body exactly; the census is in `docs/formats/wwise.md`, "Sound
//! containers".
//!
//! ```text
//! ranseq  u16 loop, u16 loop min, u16 loop max, f32 transition, f32 min, f32 max,
//!         u16 avoid repeat, u8 transition mode, u8 random mode, u8 mode, u8 flags,
//!         u32 n, n child ids, u16 m, m * { u32 id, i32 weight }
//! switch  u8 group type, u32 group, u32 default, u8 continuous validation,
//!         u32 n, n child ids,
//!         u32 cases, cases * { u32 switch, u32 k, k ids },
//!         u32 params, params * { u32 id, u8, u8, i32, i32 }
//! ```

use oag_bytes::{Le, Reader, Truncated};

use super::hirc::{Kind, bundle};

/// Why a container was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContainerError {
    /// The body ends inside a field, at this offset.
    Truncated { at: usize },
    /// The wrong kind of object was handed to a reader.
    WrongKind(Kind),
    /// The positioning byte at this offset is not a shape read: bits 6 and 7
    /// clear, or a 3D block that opens a path.
    BaseParams { at: usize },
    /// The node binds RTPCs, whose records are unread.
    Rtpc { count: u16 },
    /// The container's own fields do not end the body, at this offset.
    TrailingBytes { at: usize },
    /// A count that the body cannot hold, or a playlist that names an id its
    /// child list does not.
    BadCount { at: usize },
}

impl std::fmt::Display for ContainerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Truncated { at } => write!(f, "container ends inside a field at {at:#x}"),
            Self::WrongKind(kind) => write!(f, "{kind:?} is not the container kind asked for"),
            Self::BaseParams { at } => {
                write!(
                    f,
                    "container base parameters at {at:#x} carry a positioning shape that is not read"
                )
            }
            Self::Rtpc { count } => write!(f, "{count} RTPC records, whose layout is unread"),
            Self::TrailingBytes { at } => write!(f, "container does not end at {at:#x}"),
            Self::BadCount { at } => write!(f, "container count at {at:#x} does not fit"),
        }
    }
}

impl std::error::Error for ContainerError {}

impl From<Truncated> for ContainerError {
    fn from(err: Truncated) -> Self {
        Self::Truncated { at: err.at }
    }
}

fn base(r: &mut Reader<'_, Le>) -> Result<(), ContainerError> {
    r.u8()?;
    let effects = usize::from(r.u8()?);
    if effects > 0 {
        r.u8()?;
        r.skip(effects * 7)?;
    }
    r.skip(1 + 4 + 4 + 1)?;
    bundle(r, 4)?;
    bundle(r, 8)?;
    let at = r.position();
    let positioning = r.u8()?;
    if positioning & 0xc0 != 0xc0 {
        return Err(ContainerError::BaseParams { at });
    }
    if positioning & 0x18 != 0 {
        // A 3D block: a `1`, then the attenuation id. A `0` there opens a path
        // (more fields) that is not read.
        if r.u8()? != 1 {
            return Err(ContainerError::BaseParams { at });
        }
        r.skip(4)?;
    }
    if r.u8()? & 0x08 != 0 {
        r.skip(16)?;
    }
    r.skip(1 + 1 + 2 + 1 + 1)?;
    let groups = r.u32()?;
    for _ in 0..groups {
        r.skip(4 + 1)?;
        let n = usize::from(r.u16()?);
        r.skip(n * 8)?;
    }
    let count = r.u16()?;
    for _ in 0..count {
        r.skip(4 + 1 + 1 + 1 + 4 + 1)?;
        let points = usize::from(r.u16()?);
        r.skip(points * 12)?;
    }
    Ok(())
}

fn ids(r: &mut Reader<'_, Le>, n: usize) -> Result<Vec<u32>, ContainerError> {
    if n.saturating_mul(4) > r.remaining() {
        return Err(ContainerError::BadCount { at: r.position() });
    }
    Ok((0..n).map(|_| r.u32()).collect::<Result<_, _>>()?)
}

fn finish(r: &Reader<'_, Le>) -> Result<(), ContainerError> {
    if r.is_empty() {
        Ok(())
    } else {
        Err(ContainerError::TrailingBytes { at: r.position() })
    }
}

/// One playlist item: a child and its weight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Item {
    pub id: u32,
    /// `50000` is Wwise's default weight of 50.0 (a thousandth of a unit).
    pub weight: i32,
}

/// A random/sequence container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RanSeq {
    /// The loop count; `0` is forever in Wwise, `1` on the missile's container.
    /// Read only on a [continuous](Self::is_continuous) container, whose
    /// playlist plays on: the held cues' containers loop at `0` (magstrip,
    /// autopilot, shuriken travel) or `200` (the player's autopilot).
    pub loop_count: u16,
    /// How many of the last plays a random pick must avoid.
    pub avoid_repeat: u16,
    pub transition_mode: u8,
    /// `0` normal, `1` shuffle.
    pub random_mode: u8,
    /// `0` random, `1` sequence.
    pub mode: u8,
    /// Bit 0 weights, 1 reset the playlist at each play, 2 restart backward,
    /// 3 continuous, 4 global. Fitted from the bit's names in public Wwise
    /// documentation, confidence 55; `0x12` on the missile's container.
    pub flags: u8,
    pub children: Vec<u32>,
    pub playlist: Vec<Item>,
}

impl RanSeq {
    pub const RANDOM: u8 = 0;
    pub const SEQUENCE: u8 = 1;

    /// Reads a type 5 body, to its last byte.
    ///
    /// # Errors
    ///
    /// A [`ContainerError`] naming what did not fit.
    pub fn parse(kind: Kind, body: &[u8]) -> Result<Self, ContainerError> {
        if kind != Kind::RanSeqContainer {
            return Err(ContainerError::WrongKind(kind));
        }
        let mut r = Reader::le(body);
        base(&mut r)?;
        let loop_count = r.u16()?;
        r.skip(2 + 2 + 4 + 4 + 4)?;
        let avoid_repeat = r.u16()?;
        let transition_mode = r.u8()?;
        let random_mode = r.u8()?;
        let mode = r.u8()?;
        let flags = r.u8()?;
        let n = r.u32()? as usize;
        let children = ids(&mut r, n)?;
        let m = usize::from(r.u16()?);
        if m * 8 > r.remaining() {
            return Err(ContainerError::BadCount { at: r.position() });
        }
        let playlist = (0..m)
            .map(|_| {
                Ok(Item {
                    id: r.u32()?,
                    weight: r.i32()?,
                })
            })
            .collect::<Result<Vec<_>, Truncated>>()?;
        finish(&r)?;
        if playlist.iter().any(|item| !children.contains(&item.id)) {
            return Err(ContainerError::BadCount { at: body.len() });
        }
        Ok(Self {
            loop_count,
            avoid_repeat,
            transition_mode,
            random_mode,
            mode,
            flags,
            children,
            playlist,
        })
    }

    /// Whether the container plays its playlist in order.
    #[must_use]
    pub fn is_sequence(&self) -> bool {
        self.mode == Self::SEQUENCE
    }

    /// Whether the container plays its playlist on, item after item (flag bit 3),
    /// rather than one item per post. `0x1a` on every held cue's looping
    /// container, `0x12` on every one-shot pick read.
    #[must_use]
    pub fn is_continuous(&self) -> bool {
        self.flags & 0x08 != 0
    }

    /// Whether the playlist's weights decide a random pick.
    #[must_use]
    pub fn uses_weights(&self) -> bool {
        self.flags & 1 != 0
    }
}

/// What a switch container does for one switch value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Case {
    pub switch: u32,
    pub children: Vec<u32>,
}

/// A per-child switch parameter record.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwitchParam {
    pub id: u32,
    /// The two bytes after the id: unknown, offsets +4 and +5 (`0, 1` on every
    /// one in the census).
    pub flags: [u8; 2],
    /// Two `i32` after the flags (fade times in ms; `0` on every one).
    pub fades: [i32; 2],
}

/// A switch container.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Switch {
    /// `0` a switch group, `1` a state group.
    pub group_type: u8,
    pub group: u32,
    pub default: u32,
    /// Unknown meaning (the byte after the default switch).
    pub continuous: u8,
    pub children: Vec<u32>,
    pub cases: Vec<Case>,
    pub params: Vec<SwitchParam>,
}

impl Switch {
    /// Reads a type 6 body, to its last byte.
    ///
    /// # Errors
    ///
    /// A [`ContainerError`] naming what did not fit.
    pub fn parse(kind: Kind, body: &[u8]) -> Result<Self, ContainerError> {
        if kind != Kind::SwitchContainer {
            return Err(ContainerError::WrongKind(kind));
        }
        let mut r = Reader::le(body);
        base(&mut r)?;
        let group_type = r.u8()?;
        let group = r.u32()?;
        let default = r.u32()?;
        let continuous = r.u8()?;
        let n = r.u32()? as usize;
        let children = ids(&mut r, n)?;
        let n = r.u32()? as usize;
        if n.saturating_mul(8) > r.remaining() {
            return Err(ContainerError::BadCount { at: r.position() });
        }
        let mut cases = Vec::new();
        for _ in 0..n {
            let switch = r.u32()?;
            let k = r.u32()? as usize;
            cases.push(Case {
                switch,
                children: ids(&mut r, k)?,
            });
        }
        let n = r.u32()? as usize;
        if n.saturating_mul(14) > r.remaining() {
            return Err(ContainerError::BadCount { at: r.position() });
        }
        let mut params = Vec::new();
        for _ in 0..n {
            params.push(SwitchParam {
                id: r.u32()?,
                flags: [r.u8()?, r.u8()?],
                fades: [r.i32()?, r.i32()?],
            });
        }
        finish(&r)?;
        Ok(Self {
            group_type,
            group,
            default,
            continuous,
            children,
            cases,
            params,
        })
    }

    /// The children that sound for `switch`, or for the default switch when
    /// `switch` has no case of its own.
    #[must_use]
    pub fn children_for(&self, switch: u32) -> &[u32] {
        let case = |id| self.cases.iter().find(|c| c.switch == id);
        case(switch)
            .or_else(|| case(self.default))
            .map_or(&[], |c| c.children.as_slice())
    }
}
