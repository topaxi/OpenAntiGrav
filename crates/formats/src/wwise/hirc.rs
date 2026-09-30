//! The `HIRC` chunk's objects, bank generator version 118.
//!
//! Framing is `{ u8 type, u32 size, u32 id, body }`; [`index`] walks it and
//! the typed readers below take a body. Every layout here was fitted against
//! all 107 banks and is asserted over them in
//! `crates/formats/tests/wwise_ground_truth.rs`, with the counts in
//! `docs/formats/wwise.md`.

use std::ops::Range;

use super::Error;

/// A `HIRC` object's type byte.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Settings,
    Sound,
    Action,
    Event,
    RanSeqContainer,
    SwitchContainer,
    ActorMixer,
    Bus,
    LayerContainer,
    MusicSegment,
    MusicTrack,
    MusicSwitch,
    MusicRanSeq,
    /// A type this crate does not read.
    Other(u8),
}

impl Kind {
    /// The type byte's meaning.
    #[must_use]
    pub fn from_byte(byte: u8) -> Self {
        match byte {
            1 => Self::Settings,
            2 => Self::Sound,
            3 => Self::Action,
            4 => Self::Event,
            5 => Self::RanSeqContainer,
            6 => Self::SwitchContainer,
            7 => Self::ActorMixer,
            8 => Self::Bus,
            9 => Self::LayerContainer,
            10 => Self::MusicSegment,
            11 => Self::MusicTrack,
            12 => Self::MusicSwitch,
            13 => Self::MusicRanSeq,
            other => Self::Other(other),
        }
    }

    /// Whether this kind is a node whose base parameters carry a parent id,
    /// which [`parent`] can read.
    #[must_use]
    pub fn has_parent(self) -> bool {
        matches!(
            self,
            Self::Sound
                | Self::RanSeqContainer
                | Self::SwitchContainer
                | Self::ActorMixer
                | Self::LayerContainer
        )
    }
}

/// One object: its type, its id and where its body is in the bank file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Object {
    pub kind: Kind,
    pub id: u32,
    /// The bytes after the id.
    pub body: Range<usize>,
}

/// Walks a `HIRC` chunk's objects.
pub(super) fn index(data: &[u8], chunk: Range<usize>) -> Result<Vec<Object>, Error> {
    let bad = |at: usize| Error::BadHierarchy { at };
    let body = &data[chunk.clone()];
    if body.len() < 4 {
        return Err(bad(chunk.start));
    }
    let count = u32::from_le_bytes(body[..4].try_into().expect("four bytes")) as usize;
    let mut objects = Vec::with_capacity(count.min(body.len() / 9));
    let mut at = 4;
    for _ in 0..count {
        if at + 9 > body.len() {
            return Err(bad(chunk.start + at));
        }
        let size =
            u32::from_le_bytes(body[at + 1..at + 5].try_into().expect("four bytes")) as usize;
        let end = (at + 5)
            .checked_add(size)
            .filter(|&end| size >= 4 && end <= body.len())
            .ok_or_else(|| bad(chunk.start + at))?;
        objects.push(Object {
            kind: Kind::from_byte(body[at]),
            id: u32::from_le_bytes(body[at + 5..at + 9].try_into().expect("four bytes")),
            body: chunk.start + at + 9..chunk.start + end,
        });
        at = end;
    }
    if at != body.len() {
        return Err(bad(chunk.start + at));
    }
    Ok(objects)
}

/// A cursor over a body that refuses to run off its end.
struct Reader<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8], at: usize) -> Self {
        Self { bytes, at }
    }

    fn take(&mut self, n: usize) -> Option<&'a [u8]> {
        let end = self.at.checked_add(n)?;
        let out = self.bytes.get(self.at..end)?;
        self.at = end;
        Some(out)
    }

    fn u8(&mut self) -> Option<u8> {
        self.take(1).map(|b| b[0])
    }

    fn u16(&mut self) -> Option<u16> {
        self.take(2)
            .map(|b| u16::from_le_bytes(b.try_into().expect("two bytes")))
    }

    fn u32(&mut self) -> Option<u32> {
        self.take(4)
            .map(|b| u32::from_le_bytes(b.try_into().expect("four bytes")))
    }

    /// A property bundle: `u8 n`, `n` ids, then `n` values of `width` bytes.
    fn bundle(&mut self, width: usize) -> Option<()> {
        let n = usize::from(self.u8()?);
        self.take(n)?;
        self.take(n.checked_mul(width)?)?;
        Some(())
    }

    fn done(&self) -> bool {
        self.at == self.bytes.len()
    }
}

/// An event: the actions it runs.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    /// Action object ids, in order.
    pub actions: Vec<u32>,
}

impl Event {
    /// Reads an event body: `u32 n`, then `n` action ids, exactly.
    #[must_use]
    pub fn parse(body: &[u8]) -> Option<Self> {
        let mut r = Reader::new(body, 0);
        let n = r.u32()? as usize;
        let actions = (0..n).map(|_| r.u32()).collect::<Option<Vec<_>>>()?;
        r.done().then_some(Self { actions })
    }
}

/// What a `Play` action carries after its property bundles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Play {
    pub fade_curve: u8,
    /// The bank the target is said to live in: **always the action's own
    /// bank** (2,183 of 2,183), including for the 710 whose target is not in it.
    pub bank_id: u32,
}

/// An action: what it does and to which object.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Action {
    /// `0x0403` is `Play`; the others seen (`0x2103`, `0x1204`, `0x1e03`,
    /// `0x1901`, ...) are not read.
    pub kind: u16,
    /// The object the action acts on.
    pub target: u32,
    /// The byte after the target id; `0` on every `Play`.
    pub target_flags: u8,
    /// Set for a `Play` whose property bundles, fade curve and bank id account
    /// for the body exactly.
    pub play: Option<Play>,
}

impl Action {
    /// The `Play` action type.
    pub const PLAY: u16 = 0x0403;

    /// Reads an action body.
    #[must_use]
    pub fn parse(body: &[u8]) -> Option<Self> {
        let mut r = Reader::new(body, 0);
        let kind = r.u16()?;
        let target = r.u32()?;
        let target_flags = r.u8()?;
        let play = (kind == Self::PLAY)
            .then(|| {
                r.bundle(4)?;
                r.bundle(8)?;
                let fade_curve = r.u8()?;
                let bank_id = r.u32()?;
                r.done().then_some(Play {
                    fade_curve,
                    bank_id,
                })
            })
            .flatten();
        Some(Self {
            kind,
            target,
            target_flags,
            play,
        })
    }
}

/// How a source's media reaches the player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamType {
    /// In this or another bank's `DATA`, found by `DIDX`.
    Embedded,
    /// A loose `<media id>.wem`, read as it plays.
    Streamed,
    /// A loose `<media id>.wem`; in the census none is in any `DIDX` either.
    Prefetch,
    Other(u8),
}

impl StreamType {
    fn from_byte(byte: u8) -> Self {
        match byte {
            0 => Self::Embedded,
            1 => Self::Streamed,
            2 => Self::Prefetch,
            other => Self::Other(other),
        }
    }
}

/// A source's plugin id, `AKMAKECLASSID(type, company, number)`:
/// `type | company << 4 | number << 16`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Plugin(pub u32);

/// A codec, by the plugin number a codec-class plugin id carries.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Codec {
    /// Plugin number 1: 10 sounds, and every one's media is `fmt ` tag `0xFFFE`
    /// (`WAVE_FORMAT_EXTENSIBLE`, which is how Wwise writes PCM).
    Pcm,
    /// Plugin number 12: **all 6,626 of the base banks' codec sounds bar the
    /// PCM ones**, and every one's media is `fmt ` tag `0xFFFC`. Sony's
    /// ATRAC9 - see `docs/formats/wwise.md` for the identification.
    Atrac9,
    Other(u16),
}

impl Plugin {
    /// The class: `1` codec, `2` source, `3` effect.
    #[must_use]
    pub fn class(self) -> u32 {
        self.0 & 0xf
    }

    /// The plugin's number within its class and company.
    #[must_use]
    pub fn number(self) -> u16 {
        (self.0 >> 16) as u16
    }

    /// The codec, when this is a codec-class plugin (`0x000N0001`).
    #[must_use]
    pub fn codec(self) -> Option<Codec> {
        (self.class() == 1 && (self.0 >> 4) & 0x3f == 0).then(|| match self.number() {
            1 => Codec::Pcm,
            12 => Codec::Atrac9,
            other => Codec::Other(other),
        })
    }
}

/// A source: one piece of media a sound or a music track plays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Source {
    pub plugin: Plugin,
    pub stream: StreamType,
    /// The media id, which is the `DIDX` id and the loose file's name.
    pub media_id: u32,
    /// The media's size in memory: the `DIDX` size for an embedded source
    /// (5,892 of 5,892), the prefetched head for a prefetched one.
    pub in_memory_size: u32,
    /// Bit 0 is *language specific*: the loose file is under the language's
    /// directory (`English(US)/`), 614 of 614; clear, it is in the root, 222
    /// of 222.
    pub bits: u8,
}

/// Bytes of a source record.
const SOURCE_LEN: usize = 14;

impl Source {
    fn read(r: &mut Reader<'_>) -> Option<Self> {
        Some(Self {
            plugin: Plugin(r.u32()?),
            stream: StreamType::from_byte(r.u8()?),
            media_id: r.u32()?,
            in_memory_size: r.u32()?,
            bits: r.u8()?,
        })
    }

    /// Whether the media lives under the language's directory.
    #[must_use]
    pub fn language_specific(&self) -> bool {
        self.bits & 1 != 0
    }
}

/// A sound: one source, under a parent node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sound {
    pub source: Source,
    /// The node this sound sits under; `None` for the root.
    pub parent: Option<u32>,
}

impl Sound {
    /// Reads a sound body: a source, then the node's base parameters.
    #[must_use]
    pub fn parse(body: &[u8]) -> Option<Self> {
        let mut r = Reader::new(body, 0);
        let source = Source::read(&mut r)?;
        Some(Self {
            source,
            parent: parent_at(body, SOURCE_LEN),
        })
    }
}

/// A music track: the sources it plays.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MusicTrack {
    pub sources: Vec<Source>,
}

impl MusicTrack {
    /// Reads the head of a music track body: `u8 flags`, `u32 n`, `n` sources.
    /// What follows (playlist, clip automation) is not read.
    #[must_use]
    pub fn parse(body: &[u8]) -> Option<Self> {
        let mut r = Reader::new(body, 1);
        let n = r.u32()? as usize;
        let sources = (0..n)
            .map(|_| Source::read(&mut r))
            .collect::<Option<Vec<_>>>()?;
        Some(Self { sources })
    }
}

/// The parent id at the head of a node's base parameters, beginning `at`.
///
/// The base parameters open with an effect list (`u8 override`, `u8 n`, and
/// when `n > 0` a bypass byte and `n` seven-byte records), a `u8`, the override
/// bus id and then the parent. Everything after that - positioning, sends,
/// state, RTPCs - varies and is not read, which is why this is the whole of
/// what a node contributes. **Fitted, not documented**: the parent it yields
/// is an object in the same bank or `0` for 6,868 + 463 + 525 + 133 + 1,789 of
/// the 9,785 nodes measured, plus 110 roots and one parent (a sound in
/// `Ship_NGP`) in no bank.
fn parent_at(body: &[u8], at: usize) -> Option<u32> {
    let mut r = Reader::new(body, at);
    r.u8()?;
    let effects = usize::from(r.u8()?);
    if effects > 0 {
        r.u8()?;
        r.take(effects.checked_mul(7)?)?;
    }
    r.u8()?;
    r.u32()?;
    r.u32().filter(|&parent| parent != 0)
}

/// The parent of a node object, or `None` for a root, a kind with no node
/// parameters read here, or a body too short to hold the parameters.
#[must_use]
pub fn parent(kind: Kind, body: &[u8]) -> Option<u32> {
    match kind {
        Kind::Sound => parent_at(body, SOURCE_LEN),
        Kind::RanSeqContainer | Kind::SwitchContainer | Kind::ActorMixer | Kind::LayerContainer => {
            parent_at(body, 0)
        }
        _ => None,
    }
}
