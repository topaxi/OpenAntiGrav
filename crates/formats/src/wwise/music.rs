//! The music objects of a `HIRC` chunk: segments (type 10), switches (12) and
//! random/sequence containers (13), plus the clip list of a music track (11).
//!
//! Read forward, field by field, and **every layout here refuses by name** on a
//! value it was not measured against. The census is in `docs/formats/wwise.md`,
//! "Music"; the ground truth is `crates/formats/tests/wwise_music_ground_truth.rs`.
//!
//! A music node is `{ u8 flags, base parameters, u32 n, n child ids, meter,
//! stingers }` and then what its kind adds:
//!
//! ```text
//! segment   f64 duration, u32 markers { u32 id, f64 position, u32 len, name }
//! switch    u32 rules, rules, u8 continue, u32 depth, depth * u32 group,
//!           depth * u8 type, u32 tree bytes, u8 mode, tree (12-byte nodes)
//! ranseq    u32 rules, rules, u32 items, items (30 bytes each, depth first)
//! ```

use super::hirc::Kind;

#[cfg(test)]
mod tests;

/// Why a music object was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MusicError {
    /// The body ends inside a field, at this offset.
    Truncated { at: usize },
    /// The wrong kind of object was handed to a reader.
    WrongKind(Kind),
    /// The state chunk lists state properties (`u32 n`, `n` != 0): their
    /// record layout has not been measured.
    StateProperties { count: u32 },
    /// The RTPC list is not empty: its record layout has not been measured.
    Rtpc { count: u16 },
    /// A music node has stinger records the layout does not describe here.
    Stingers { count: u32 },
    /// A segment's markers or a node's trailing fields do not end the body.
    TrailingBytes { at: usize },
    /// A switch tree or a playlist whose size and counts do not agree.
    BadTree,
    /// A transition rule with a flag value other than 0 or 1.
    BadRule { at: usize },
}

impl std::fmt::Display for MusicError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Truncated { at } => write!(f, "music object ends inside a field at {at:#x}"),
            Self::WrongKind(kind) => write!(f, "{kind:?} is not the music kind asked for"),
            Self::StateProperties { count } => {
                write!(f, "{count} state properties, whose layout is unread")
            }
            Self::Rtpc { count } => write!(f, "{count} RTPC records, whose layout is unread"),
            Self::Stingers { count } => write!(f, "{count} stingers, whose layout is unread"),
            Self::TrailingBytes { at } => write!(f, "music object does not end at {at:#x}"),
            Self::BadTree => write!(f, "a decision tree or playlist whose sizes disagree"),
            Self::BadRule { at } => write!(f, "a transition rule is malformed at {at:#x}"),
        }
    }
}

impl std::error::Error for MusicError {}

struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Cursor<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], MusicError> {
        let at = self.at;
        let end = at.checked_add(n).ok_or(MusicError::Truncated { at })?;
        let out = self
            .bytes
            .get(at..end)
            .ok_or(MusicError::Truncated { at })?;
        self.at = end;
        Ok(out)
    }

    fn u8(&mut self) -> Result<u8, MusicError> {
        Ok(self.take(1)?[0])
    }

    fn u16(&mut self) -> Result<u16, MusicError> {
        Ok(u16::from_le_bytes(self.take(2)?.try_into().expect("two")))
    }

    fn u32(&mut self) -> Result<u32, MusicError> {
        Ok(u32::from_le_bytes(self.take(4)?.try_into().expect("four")))
    }

    fn f64(&mut self) -> Result<f64, MusicError> {
        Ok(f64::from_le_bytes(self.take(8)?.try_into().expect("eight")))
    }

    fn bundle(&mut self, width: usize) -> Result<(), MusicError> {
        let n = usize::from(self.u8()?);
        self.take(n)?;
        self.take(n * width)?;
        Ok(())
    }

    fn rest(&self) -> &'a [u8] {
        &self.bytes[self.at..]
    }
}

/// A bound state group: its id, the change-occurs byte and its
/// `(state id, state instance id)` pairs.
pub type StateGroup = (u32, u8, Vec<(u32, u32)>);

/// What every music node carries before its own fields.
#[derive(Debug, Clone, PartialEq)]
pub struct MusicNode {
    pub parent: Option<u32>,
    /// The four bytes after the properties (positioning, auxiliary sends,
    /// advanced settings): `c0 00 00 01` on every node but one switch's
    /// `c3 00 00 01`. Kept raw.
    pub flags: [u8; 4],
    /// State groups the node binds: `(group id, change-occurs byte, states)`
    /// with each state `(state id, state instance id)`.
    pub state_groups: Vec<StateGroup>,
    pub children: Vec<u32>,
    /// Tempo in beats per minute, and the time signature.
    pub tempo: f32,
    pub beats: (u8, u8),
}

fn node(c: &mut Cursor<'_>) -> Result<MusicNode, MusicError> {
    c.u8()?;
    c.u8()?;
    let fx = usize::from(c.u8()?);
    if fx > 0 {
        c.u8()?;
        c.take(fx * 7)?;
    }
    c.u8()?;
    c.u32()?;
    let parent = Some(c.u32()?).filter(|&p| p != 0);
    c.u8()?;
    c.bundle(4)?;
    c.bundle(8)?;
    let flags: [u8; 4] = c.take(4)?.try_into().expect("four");
    let props = c.u32()?;
    if props != 0 {
        return Err(MusicError::StateProperties { count: props });
    }
    let groups = c.u32()?;
    let mut state_groups = Vec::new();
    for _ in 0..groups {
        let id = c.u32()?;
        let occurs = c.u8()?;
        let n = c.u16()?;
        let mut states = Vec::new();
        for _ in 0..n {
            states.push((c.u32()?, c.u32()?));
        }
        state_groups.push((id, occurs, states));
    }
    let rtpc = c.u16()?;
    if rtpc != 0 {
        return Err(MusicError::Rtpc { count: rtpc });
    }
    let n = c.u32()? as usize;
    let mut children = Vec::new();
    for _ in 0..n {
        children.push(c.u32()?);
    }
    c.f64()?;
    c.f64()?;
    let tempo = f32::from_le_bytes(c.take(4)?.try_into().expect("four"));
    let beats = (c.u8()?, c.u8()?);
    c.u8()?;
    let stingers = c.u32()?;
    if stingers != 0 {
        return Err(MusicError::Stingers { count: stingers });
    }
    Ok(MusicNode {
        parent,
        flags,
        state_groups,
        children,
        tempo,
        beats,
    })
}

fn expect(kind: Kind, want: Kind) -> Result<(), MusicError> {
    (kind == want)
        .then_some(())
        .ok_or(MusicError::WrongKind(kind))
}

/// A music segment: its child tracks and its markers.
#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub node: MusicNode,
    pub duration: f64,
    /// `(id, position in ms)`; the marker names are empty on every segment.
    pub markers: Vec<(u32, f64)>,
}

impl Segment {
    /// Reads a type 10 body, to its last byte.
    ///
    /// # Errors
    ///
    /// A [`MusicError`] naming what did not fit.
    pub fn parse(kind: Kind, body: &[u8]) -> Result<Self, MusicError> {
        expect(kind, Kind::MusicSegment)?;
        let mut c = Cursor { bytes: body, at: 0 };
        let node = node(&mut c)?;
        let duration = c.f64()?;
        let n = c.u32()?;
        let mut markers = Vec::new();
        for _ in 0..n {
            let id = c.u32()?;
            let at = c.f64()?;
            let len = c.u32()? as usize;
            c.take(len)?;
            markers.push((id, at));
        }
        if !c.rest().is_empty() {
            return Err(MusicError::TrailingBytes { at: c.at });
        }
        Ok(Self {
            node,
            duration,
            markers,
        })
    }
}

/// Skips `rules` transition rules: each is `u32 n, n ids, u32 m, m ids`, a
/// 21-byte source rule, a 24-byte destination rule and a flag byte that, when
/// `1`, precedes a 30-byte transition object. Nothing in them decides what
/// plays, so they are counted, not kept.
fn skip_rules(c: &mut Cursor<'_>, rules: u32) -> Result<(), MusicError> {
    for _ in 0..rules {
        for _ in 0..2 {
            let n = c.u32()? as usize;
            c.take(n.checked_mul(4).ok_or(MusicError::BadTree)?)?;
        }
        c.take(21 + 24)?;
        let at = c.at;
        match c.u8()? {
            0 => {}
            1 => {
                c.take(30)?;
            }
            _ => return Err(MusicError::BadRule { at }),
        }
    }
    Ok(())
}

/// One node of a switch's decision tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeNode {
    pub key: u32,
    /// An object id at a leaf, else `first child index | count << 16`.
    pub target: u32,
    pub weight: u16,
    pub probability: u16,
}

/// A music switch: which child plays for which state values.
#[derive(Debug, Clone, PartialEq)]
pub struct Switch {
    pub node: MusicNode,
    pub rules: u32,
    pub continue_playback: bool,
    /// The state or switch groups the tree is keyed on, outermost first, with
    /// their type byte (`1` is a state group on every switch).
    pub groups: Vec<(u32, u8)>,
    pub mode: u8,
    pub tree: Vec<TreeNode>,
}

impl Switch {
    /// Reads a type 12 body, to its last byte.
    ///
    /// # Errors
    ///
    /// A [`MusicError`] naming what did not fit.
    pub fn parse(kind: Kind, body: &[u8]) -> Result<Self, MusicError> {
        expect(kind, Kind::MusicSwitch)?;
        let mut c = Cursor { bytes: body, at: 0 };
        let node = node(&mut c)?;
        let rules = c.u32()?;
        skip_rules(&mut c, rules)?;
        let continue_playback = c.u8()? != 0;
        let depth = c.u32()? as usize;
        if depth == 0 || depth > 16 {
            return Err(MusicError::BadTree);
        }
        let ids = (0..depth).map(|_| c.u32()).collect::<Result<Vec<_>, _>>()?;
        let types = c.take(depth)?.to_vec();
        let size = c.u32()? as usize;
        let mode = c.u8()?;
        if !size.is_multiple_of(12) {
            return Err(MusicError::BadTree);
        }
        let raw = c.take(size)?;
        if !c.rest().is_empty() {
            return Err(MusicError::TrailingBytes { at: c.at });
        }
        let tree = raw
            .as_chunks::<12>()
            .0
            .iter()
            .map(|n| TreeNode {
                key: u32::from_le_bytes(n[0..4].try_into().expect("four")),
                target: u32::from_le_bytes(n[4..8].try_into().expect("four")),
                weight: u16::from_le_bytes(n[8..10].try_into().expect("two")),
                probability: u16::from_le_bytes(n[10..12].try_into().expect("two")),
            })
            .collect();
        Ok(Self {
            node,
            rules,
            continue_playback,
            groups: ids.into_iter().zip(types).collect(),
            mode,
            tree,
        })
    }

    /// Every leaf as `(keys from the root down, target object id)`.
    ///
    /// # Errors
    ///
    /// [`MusicError::BadTree`] for a child range outside the tree.
    pub fn leaves(&self) -> Result<Vec<(Vec<u32>, u32)>, MusicError> {
        let mut out = Vec::new();
        if self.tree.is_empty() {
            return Err(MusicError::BadTree);
        }
        self.walk(0, 0, &mut Vec::new(), &mut out)?;
        Ok(out)
    }

    fn walk(
        &self,
        index: usize,
        level: usize,
        keys: &mut Vec<u32>,
        out: &mut Vec<(Vec<u32>, u32)>,
    ) -> Result<(), MusicError> {
        let node = self.tree.get(index).ok_or(MusicError::BadTree)?;
        keys.push(node.key);
        if level == self.groups.len() {
            out.push((keys.clone(), node.target));
        } else {
            let first = (node.target & 0xffff) as usize;
            let count = (node.target >> 16) as usize;
            for child in first..first + count {
                self.walk(child, level + 1, keys, out)?;
            }
        }
        keys.pop();
        Ok(())
    }

    /// The target for one value per group, outermost first.
    #[must_use]
    pub fn select(&self, values: &[u32]) -> Option<u32> {
        self.leaves()
            .ok()?
            .into_iter()
            .find(|(keys, _)| keys[1..] == *values)
            .map(|(_, target)| target)
    }
}

/// One playlist item of a random/sequence container.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlaylistItem {
    pub segment: u32,
    pub item_id: u32,
    pub children: u32,
    pub kind: i32,
    pub loop_count: i16,
    pub loop_min: i16,
    pub loop_max: i16,
    pub weight: u32,
    pub avoid_repeat: u16,
    pub use_weight: bool,
    pub shuffle: bool,
}

/// A music random/sequence container.
#[derive(Debug, Clone, PartialEq)]
pub struct RanSeq {
    pub node: MusicNode,
    pub rules: u32,
    /// The items in file order: a root and its descendants, depth first.
    pub items: Vec<PlaylistItem>,
}

impl RanSeq {
    /// Reads a type 13 body, to its last byte.
    ///
    /// # Errors
    ///
    /// A [`MusicError`] naming what did not fit.
    pub fn parse(kind: Kind, body: &[u8]) -> Result<Self, MusicError> {
        expect(kind, Kind::MusicRanSeq)?;
        let mut c = Cursor { bytes: body, at: 0 };
        let node = node(&mut c)?;
        let rules = c.u32()?;
        skip_rules(&mut c, rules)?;
        let n = c.u32()? as usize;
        let mut items = Vec::new();
        for _ in 0..n {
            let segment = c.u32()?;
            let item_id = c.u32()?;
            let children = c.u32()?;
            let kind = c.u32()? as i32;
            let loop_count = c.u16()? as i16;
            let loop_min = c.u16()? as i16;
            let loop_max = c.u16()? as i16;
            let weight = c.u32()?;
            let avoid_repeat = c.u16()?;
            let use_weight = c.u8()? != 0;
            let shuffle = c.u8()? != 0;
            items.push(PlaylistItem {
                segment,
                item_id,
                children,
                kind,
                loop_count,
                loop_min,
                loop_max,
                weight,
                avoid_repeat,
                use_weight,
                shuffle,
            });
        }
        if !c.rest().is_empty() {
            return Err(MusicError::TrailingBytes { at: c.at });
        }
        // The root's descendants account for every item: a tree whose child
        // counts do not add up to the list is not this layout.
        if items.first().map(|root| count_below(&items, 0, root)) != Some(Some(items.len())) {
            return Err(MusicError::BadTree);
        }
        Ok(Self { node, rules, items })
    }

    /// The segments the playlist plays, in order.
    #[must_use]
    pub fn segments(&self) -> Vec<u32> {
        self.items
            .iter()
            .map(|item| item.segment)
            .filter(|&s| s != 0)
            .collect()
    }
}

/// Items in the subtree rooted at `items[at]`, counting it, or `None` when the
/// child counts run past the list.
fn count_below(items: &[PlaylistItem], at: usize, root: &PlaylistItem) -> Option<usize> {
    let mut next = at + 1;
    for _ in 0..root.children {
        let child = items.get(next)?;
        next += count_below(items, next, child)?;
    }
    Some(next - at)
}

/// A music track's clip: which source plays, where, and for how long.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Clip {
    pub track_id: u32,
    pub source_id: u32,
    /// Milliseconds: where the clip starts in the segment, the trims, and the
    /// source's own length.
    pub play_at: f64,
    pub begin_trim: f64,
    pub end_trim: f64,
    pub source_duration: f64,
}

/// Reads a music track's clips, which follow its sources.
///
/// # Errors
///
/// A [`MusicError`] for a body that ends inside them.
pub fn track_clips(body: &[u8]) -> Result<Vec<Clip>, MusicError> {
    let mut c = Cursor { bytes: body, at: 1 };
    let sources = c.u32()? as usize;
    c.take(sources.checked_mul(14).ok_or(MusicError::BadTree)?)?;
    let n = c.u32()?;
    let mut clips = Vec::new();
    for _ in 0..n {
        clips.push(Clip {
            track_id: c.u32()?,
            source_id: c.u32()?,
            play_at: c.f64()?,
            begin_trim: c.f64()?,
            end_trim: c.f64()?,
            source_duration: c.f64()?,
        });
    }
    Ok(clips)
}

/// A music track's type: the byte five from the end, before the look-ahead.
/// `0` is a normal track (every track in the census). Fitted from the tail, as
/// the track's own base parameters are unread.
#[must_use]
pub fn track_type(body: &[u8]) -> Option<u8> {
    body.len().checked_sub(5).map(|i| body[i])
}
