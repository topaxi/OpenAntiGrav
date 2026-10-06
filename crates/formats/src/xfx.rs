//! Wipeout HD's `.xfx` crossfade tables: `data/sound/xfship_<team>.xfx`.
//!
//! A table is a handful of **input channels** and **layers**. The game writes
//! one number per channel per tick; each layer reads one channel, looks the
//! smoothed number up in two 512-entry curves, and drives one named sound's
//! gain and pitch. The curves are authored, so a team's engine note is data.
//! See `docs/formats/hd-xfx.md` for the evidence and confidence per field.
//!
//! The reader mirrors the game's loader (`XFadeSystem_AddCrossFader`,
//! `0x00312660` on the EU build): magic, version, target platform, then four
//! offsets turned into pointers. The game says *controller* and *element*;
//! this reader says channel and layer. Big-endian: the format ships on PS3.
//!
//! The layout, all offsets from the start of the file:
//!
//! | Offset | Field |
//! | --- | --- |
//! | `0x00` | magic `XFDX` |
//! | `0x04` | `0x02060000`: target platform `2` in the high byte, file version `0x060000` below it |
//! | `0x08` | relocation flag, `0` on disc (the loader sets it after fixing up offsets) |
//! | `0x0c` | channel (controller) count |
//! | `0x10` | layer (element) count |
//! | `0x14` | offset of the channel array, always `0x1c` |
//! | `0x18` | offset of the layer pointer table |
//!
//! Then `channels * 0x60` bytes of channels, each followed in the file by its
//! triggers (`0x40` bytes each), then `layers * 0x830` bytes of layers, then
//! the layer pointer table of `layers * 4` bytes, which is the end of the file.

use crate::byte_order::ByteOrder;
use crate::coverage::Coverage;

/// The first four bytes: `XFDX`.
pub const MAGIC: [u8; 4] = *b"XFDX";

/// The second word of a PS3 file, big-endian: target platform `2`, then file
/// version `0x060000`. The game's loader tests `word & 0x00ffffff == 0x060000`
/// ("Incorrect Crossfader file version") and `word >> 24 == 2` ("Incorrect
/// target platform") separately.
pub const VERSION_WORD: u32 = 0x0206_0000;

/// The second word of a Vita file, **little-endian**: target platform `0`, the
/// same file version `0x060000` (bytes on disc `00 00 06 00`). Only Wipeout
/// 2048's 23 tables carry it; the whole file is little-endian and differs from
/// HD's layout otherwise only by a fifth channel.
pub const VERSION_WORD_VITA: u32 = 0x0006_0000;

/// Bytes in the fixed header.
pub const HEADER_LEN: usize = 0x1c;

/// Bytes in one channel record.
pub const CHANNEL_LEN: usize = 0x60;

/// Bytes in one trigger record.
pub const TRIGGER_LEN: usize = 0x40;

/// Bytes in one layer record: a `0x30` head and two curves.
pub const LAYER_LEN: usize = 0x830;

/// Entries in each of a layer's two curves. A channel's smoothed value is
/// clamped to `0..=511` before it indexes one.
pub const CURVE_LEN: usize = 512;

/// The longest a layer name can be: the head holds the kind byte at `+0`, the
/// name from `+1`, and `+0x10` is the next field, so 15 bytes.
const NAME_LEN: usize = 15;

/// Something wrong with a `.xfx` table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Fewer bytes than the header needs.
    TooShort {
        /// Bytes supplied.
        got: usize,
    },
    /// The first four bytes were not `XFDX`.
    NotXfdx,
    /// The version word was not [`VERSION_WORD`].
    UnsupportedVersion {
        /// The word found.
        word: u32,
    },
    /// The relocation flag was set, so the offsets are already pointers.
    AlreadyRelocated,
    /// The channel array did not sit at `0x1c`.
    ChannelsMisplaced {
        /// The offset found.
        at: u32,
    },
    /// A record, array or curve runs past the end of the file.
    Truncated {
        /// What was being read.
        what: &'static str,
    },
    /// A layer's type byte (`+0x17`) was not the curve-pair type `0`. The
    /// loader's type `1` (a piecewise-linear pair via `+0x28`/`+0x2c`) is on no
    /// disc file, so it is refused rather than guessed at.
    UnsupportedLayerType {
        /// Index of the layer.
        layer: usize,
        /// The type byte found.
        kind: u8,
    },
    /// A layer named a channel the table does not have.
    ChannelOutOfRange {
        /// Index of the layer.
        layer: usize,
        /// The channel byte found.
        channel: u8,
    },
}

impl core::fmt::Display for Error {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::TooShort { got } => write!(f, "xfx: {got} bytes is shorter than the header"),
            Self::NotXfdx => write!(f, "xfx: magic is not XFDX"),
            Self::UnsupportedVersion { word } => write!(f, "xfx: version word {word:#010x}"),
            Self::AlreadyRelocated => write!(f, "xfx: relocation flag already set"),
            Self::ChannelsMisplaced { at } => write!(f, "xfx: channel array at {at:#x}, not 0x1c"),
            Self::Truncated { what } => write!(f, "xfx: {what} runs past the end of the file"),
            Self::UnsupportedLayerType { layer, kind } => {
                write!(f, "xfx: layer {layer} has type {kind}, only 0 is read")
            }
            Self::ChannelOutOfRange { layer, channel } => {
                write!(f, "xfx: layer {layer} reads channel {channel}")
            }
        }
    }
}

impl std::error::Error for Error {}

/// A parsed `.xfx` table, borrowing the file.
#[derive(Debug, Clone)]
pub struct Xfx<'a> {
    data: &'a [u8],
    order: ByteOrder,
    channels: Vec<Channel<'a>>,
    layers: Vec<Layer<'a>>,
}

/// One input channel: how a raw number is smoothed on its way to the curves.
/// The smoothing is `XFadeSystem_UpdateChannels` (`0x00313d10`): the target is
/// chased at a rate set by which of four bands the value sits in; zero means
/// no smoothing in that band.
#[derive(Debug, Clone, Copy)]
pub struct Channel<'a> {
    order: ByteOrder,
    raw: &'a [u8],
    triggers: &'a [u8],
}

/// One layer: a named sound whose gain and pitch follow a channel.
#[derive(Debug, Clone, Copy)]
pub struct Layer<'a> {
    order: ByteOrder,
    raw: &'a [u8],
    gain: &'a [u8],
    pitch: &'a [u8],
}

impl<'a> Xfx<'a> {
    /// Parse a table.
    /// # Errors
    ///
    /// [`Error`] when the header fails the loader's checks, a record runs off
    /// the end of `data`, a layer is of an unread type, or a layer names a
    /// channel the table lacks.
    pub fn parse(data: &'a [u8]) -> Result<Self, Error> {
        if data.len() < HEADER_LEN {
            return Err(Error::TooShort { got: data.len() });
        }
        if data[..4] != MAGIC {
            return Err(Error::NotXfdx);
        }
        let word = ByteOrder::Big.u32(data, 4);
        let order = if word == VERSION_WORD {
            ByteOrder::Big
        } else if ByteOrder::Little.u32(data, 4) == VERSION_WORD_VITA {
            ByteOrder::Little
        } else {
            return Err(Error::UnsupportedVersion { word });
        };
        if order.u32(data, 8) != 0 {
            return Err(Error::AlreadyRelocated);
        }
        let channel_count = order.u32(data, 0x0c) as usize;
        let layer_count = order.u32(data, 0x10) as usize;
        let channel_at = order.u32(data, 0x14);
        let table_at = order.u32(data, 0x18) as usize;
        if channel_at as usize != HEADER_LEN {
            return Err(Error::ChannelsMisplaced { at: channel_at });
        }

        let mut channels = Vec::with_capacity(channel_count);
        for n in 0..channel_count {
            let at = HEADER_LEN + n * CHANNEL_LEN;
            let raw = data
                .get(at..at + CHANNEL_LEN)
                .ok_or(Error::Truncated { what: "channel" })?;
            let trigger_count = usize::try_from(order.i16(raw, 0x58)).unwrap_or(0);
            let trigger_at = order.u32(raw, 0x5c) as usize;
            let triggers = data
                .get(trigger_at..trigger_at + trigger_count * TRIGGER_LEN)
                .ok_or(Error::Truncated { what: "triggers" })?;
            channels.push(Channel {
                order,
                raw,
                triggers,
            });
        }

        let pointers = data
            .get(table_at..table_at + layer_count * 4)
            .ok_or(Error::Truncated {
                what: "layer pointer table",
            })?;
        let mut layers = Vec::with_capacity(layer_count);
        for n in 0..layer_count {
            let at = order.u32(pointers, n * 4) as usize;
            let raw = data
                .get(at..at + LAYER_LEN)
                .ok_or(Error::Truncated { what: "layer" })?;
            let kind = raw[0x17];
            if kind != 0 {
                return Err(Error::UnsupportedLayerType { layer: n, kind });
            }
            let channel = raw[0x14];
            if usize::from(channel) >= channel_count {
                return Err(Error::ChannelOutOfRange { layer: n, channel });
            }
            let curve = |field: usize, what: &'static str| {
                let at = order.u32(raw, field) as usize;
                data.get(at..at + CURVE_LEN * 2)
                    .ok_or(Error::Truncated { what })
            };
            let gain = curve(0x1c, "gain curve")?;
            let pitch = curve(0x20, "pitch curve")?;
            layers.push(Layer {
                order,
                raw,
                gain,
                pitch,
            });
        }
        Ok(Self {
            data,
            order,
            channels,
            layers,
        })
    }

    /// The byte order the file is in: big on PS3, little on the Vita.
    #[must_use]
    pub fn byte_order(&self) -> ByteOrder {
        self.order
    }

    /// The input channels, in file order.
    #[must_use]
    pub fn channels(&self) -> &[Channel<'a>] {
        &self.channels
    }

    /// The layers, in file order.
    #[must_use]
    pub fn layers(&self) -> &[Layer<'a>] {
        &self.layers
    }

    /// The layers that read channel `channel`.
    pub fn layers_on(&self, channel: usize) -> impl Iterator<Item = &Layer<'a>> {
        self.layers
            .iter()
            .filter(move |l| usize::from(l.channel()) == channel)
    }

    /// Bytes the header, channels, triggers, layers and pointer table add up to.
    /// On every disc file this equals the file's length: no padding or tail.
    #[must_use]
    pub fn accounted_bytes(&self) -> usize {
        HEADER_LEN
            + self.channels.len() * CHANNEL_LEN
            + self
                .channels
                .iter()
                .map(|c| c.triggers.len())
                .sum::<usize>()
            + self.layers.len() * LAYER_LEN
            + self.layers.len() * 4
    }

    /// The whole file.
    #[must_use]
    pub fn bytes(&self) -> &'a [u8] {
        self.data
    }

    /// The ranges of the file this reader reaches, each claimed under its name.
    /// [`Self::accounted_bytes`] adds the sizes; this places them. Overlapping
    /// or hole-leaving pieces would still sum to the file length but not tile
    /// it, so a test asserts both.
    #[must_use]
    pub fn coverage(&self) -> Coverage {
        let base = self.data.as_ptr() as usize;
        let at = |part: &[u8]| part.as_ptr() as usize - base;
        let mut coverage = Coverage::new(self.data.len());
        coverage.claim(0, HEADER_LEN, "header");
        for channel in &self.channels {
            coverage.claim(at(channel.raw), CHANNEL_LEN, "channel");
            coverage.claim(at(channel.triggers), channel.triggers.len(), "triggers");
        }
        for layer in &self.layers {
            coverage.claim(at(layer.raw), LAYER_LEN, "layer");
        }
        let table = self.order.u32(self.data, 0x18) as usize;
        coverage.claim(table, self.layers.len() * 4, "layer pointer table");
        coverage
    }
}

impl<'a> Channel<'a> {
    /// The four band edges, in whole input counts.
    /// Band 0 while the value is below the first edge, band 1 below the second,
    /// band 2 below the third, band 3 otherwise (`0x00313d10`).
    #[must_use]
    pub fn band_edges(&self) -> [u16; 4] {
        std::array::from_fn(|i| self.order.u16(self.raw, i * 2))
    }

    /// Per-band rates for a value that is rising, `16.16` counts per millisecond.
    ///
    /// Zero means the value snaps to its target in that band.
    #[must_use]
    pub fn rise_rates(&self) -> [i32; 4] {
        std::array::from_fn(|i| self.order.u32(self.raw, 0x08 + i * 4) as i32)
    }

    /// Per-band rates for a value that is falling or holding, same unit.
    #[must_use]
    pub fn fall_rates(&self) -> [i32; 4] {
        std::array::from_fn(|i| self.order.u32(self.raw, 0x18 + i * 4) as i32)
    }

    /// The multiplier `XFadeSystem_SetInput` (`0x00314618`) applies to a value
    /// written to this channel, `16.16`.
    #[must_use]
    pub fn input_scale(&self) -> i32 {
        self.order.u32(self.raw, 0x50) as i32
    }

    /// The bias added after the multiply, in whole counts (shifted up `16`).
    #[must_use]
    pub fn input_bias(&self) -> u16 {
        self.order.u16(self.raw, 0x54)
    }

    /// The raw `0x60`-byte record, for fields this reader does not name.
    /// The raw `0x60`-byte record, for unnamed fields. The jitter fields at
    /// `+0x28..+0x50` (the smoother wobbles a channel with them) are zero on
    /// every disc channel except the ones the test pins.
    #[must_use]
    pub fn raw(&self) -> &'a [u8] {
        self.raw
    }

    /// How many triggers the channel carries.
    #[must_use]
    pub fn trigger_count(&self) -> usize {
        self.triggers.len() / TRIGGER_LEN
    }

    /// The raw `0x40`-byte trigger records.
    pub fn triggers(&self) -> impl Iterator<Item = &'a [u8]> {
        (0..self.trigger_count()).map(|n| &self.triggers[n * TRIGGER_LEN..(n + 1) * TRIGGER_LEN])
    }
}

impl<'a> Layer<'a> {
    /// The kind byte at `+0`; the game accepts `0..=2` and every shipped layer
    /// is `0`.
    #[must_use]
    pub fn kind(&self) -> u8 {
        self.raw[0]
    }

    /// The sound's name, which the game looks up in the ship sound bank.
    #[must_use]
    pub fn name(&self) -> &'a str {
        let field = &self.raw[1..=NAME_LEN];
        let end = field.iter().position(|&b| b == 0).unwrap_or(NAME_LEN);
        std::str::from_utf8(&field[..end]).unwrap_or("")
    }

    /// The channel this layer reads (`+0x14`).
    #[must_use]
    pub fn channel(&self) -> u8 {
        self.raw[0x14]
    }

    /// The flag word at `+0x18`; bit `0x1000` is tested by the layer update.
    #[must_use]
    pub fn flags(&self) -> u32 {
        self.order.u32(self.raw, 0x18)
    }

    /// The cue index at `+0x18` (a `u16`), naming the sound when the name is
    /// empty: `Crossfader` start (`FUN_8125d4b6` on the Vita build) plays by
    /// name if the first byte is set, else by this index, so the `*2048.xfx`
    /// files (all names empty) address their bank by index alone. On HD's and
    /// the older Vita files the high half of [`Self::flags`] is zero here.
    #[must_use]
    pub fn cue_index(&self) -> u16 {
        self.order.u16(self.raw, 0x18)
    }

    /// The element a kind-2 layer modulates (`+0x12`, a signed byte).
    /// A kind-2 layer plays no sound: the update writes its gain and pitch into
    /// the slots of the layer it links to.
    #[must_use]
    pub fn link(&self) -> i8 {
        self.raw[0x12] as i8
    }

    /// The gain curve: `0x400` is unity.
    pub fn gain(&self) -> impl Iterator<Item = i16> + 'a {
        curve(self.order, self.gain)
    }

    /// The pitch curve: `0x200` is neutral.
    pub fn pitch(&self) -> impl Iterator<Item = i16> + 'a {
        curve(self.order, self.pitch)
    }

    /// The gain at channel value `x`, clamped to `0..=511`.
    #[must_use]
    pub fn gain_at(&self, x: usize) -> i16 {
        self.order.i16(self.gain, x.min(CURVE_LEN - 1) * 2)
    }

    /// The pitch value at channel value `x`, clamped to `0..=511`.
    #[must_use]
    pub fn pitch_at(&self, x: usize) -> i16 {
        self.order.i16(self.pitch, x.min(CURVE_LEN - 1) * 2)
    }

    /// The raw `0x830`-byte record.
    #[must_use]
    pub fn raw(&self) -> &'a [u8] {
        self.raw
    }

    /// Where the gain and pitch curves sit, as offsets from this layer's record:
    /// `(0x30, 0x430)` on every disc file, the record's own tail.
    #[must_use]
    pub fn curve_offsets(&self) -> (usize, usize) {
        let base = self.raw.as_ptr() as usize;
        (
            (self.gain.as_ptr() as usize).wrapping_sub(base),
            (self.pitch.as_ptr() as usize).wrapping_sub(base),
        )
    }
}

fn curve(order: ByteOrder, bytes: &[u8]) -> impl Iterator<Item = i16> + '_ {
    (0..bytes.len() / 2).map(move |n| order.i16(bytes, n * 2))
}

#[cfg(test)]
mod tests;
