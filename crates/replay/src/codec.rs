//! The input stream's encoding: run-length over whole snapshots, and within a
//! run only the fields that changed.
//!
//! An [`InputSnapshot`] is eight 32-bit words - four button masks and four
//! axes. Consecutive ticks are usually identical (a held throttle, a centred
//! stick), so the stream is a sequence of **runs**: how many ticks the snapshot
//! stays the same, then which of the eight words differ from the previous run,
//! then those words. A keyboard lap is a few hundred runs; an analog stick that
//! moves every tick costs one run per tick and one or two words in each, which
//! is still under ten bytes a tick. Measured figures are in ADR-0055.
//!
//! **Lossless, bit for bit.** The axes are stored as their `f32` bit patterns,
//! never quantised: a replay's whole claim is that the simulation receives
//! exactly what it received the first time, and an axis rounded to a byte
//! would be a different input.

use oag_gameplay::InputSnapshot;
use oag_gameplay::input::Input;

/// A snapshot as the eight words it is stored as.
#[must_use]
pub fn words(snapshot: &InputSnapshot) -> [u32; 8] {
    let [held, held_last, released, pressed] = snapshot.buttons.masks();
    [
        held,
        held_last,
        released,
        pressed,
        snapshot.stick_x.to_bits(),
        snapshot.stick_y.to_bits(),
        snapshot.airbrake_left.to_bits(),
        snapshot.airbrake_right.to_bits(),
    ]
}

/// The inverse of [`words`].
#[must_use]
pub fn snapshot(words: [u32; 8]) -> InputSnapshot {
    InputSnapshot {
        buttons: Input::from_masks([words[0], words[1], words[2], words[3]]),
        stick_x: f32::from_bits(words[4]),
        stick_y: f32::from_bits(words[5]),
        airbrake_left: f32::from_bits(words[6]),
        airbrake_right: f32::from_bits(words[7]),
    }
}

/// Appends `value` as an unsigned LEB128 varint.
pub fn put_varint(out: &mut Vec<u8>, mut value: u64) {
    loop {
        let byte = (value & 0x7f) as u8;
        value >>= 7;
        if value == 0 {
            out.push(byte);
            return;
        }
        out.push(byte | 0x80);
    }
}

/// A cursor over a byte slice, for the decoders here and in [`crate::file`].
#[derive(Debug)]
pub struct Cursor<'a> {
    bytes: &'a [u8],
    at: usize,
}

/// A read ran off the end of its input, or a varint did not fit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("truncated or malformed data at byte {0}")]
pub struct Truncated(pub usize);

impl<'a> Cursor<'a> {
    /// A cursor at the start of `bytes`.
    #[must_use]
    pub fn new(bytes: &'a [u8]) -> Self {
        Self { bytes, at: 0 }
    }

    /// Whether every byte has been read.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.at >= self.bytes.len()
    }

    /// Where the cursor is.
    #[must_use]
    pub fn position(&self) -> usize {
        self.at
    }

    /// The next `n` bytes.
    ///
    /// # Errors
    ///
    /// [`Truncated`] when fewer than `n` remain.
    pub fn take(&mut self, n: usize) -> Result<&'a [u8], Truncated> {
        let end = self.at.checked_add(n).ok_or(Truncated(self.at))?;
        let slice = self.bytes.get(self.at..end).ok_or(Truncated(self.at))?;
        self.at = end;
        Ok(slice)
    }

    /// One byte.
    ///
    /// # Errors
    ///
    /// [`Truncated`] at the end of the input.
    pub fn u8(&mut self) -> Result<u8, Truncated> {
        Ok(self.take(1)?[0])
    }

    /// A little-endian `u16`.
    ///
    /// # Errors
    ///
    /// [`Truncated`] when fewer than two bytes remain.
    pub fn u16(&mut self) -> Result<u16, Truncated> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    /// A little-endian `u32`.
    ///
    /// # Errors
    ///
    /// [`Truncated`] when fewer than four bytes remain.
    pub fn u32(&mut self) -> Result<u32, Truncated> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// A little-endian `u64`.
    ///
    /// # Errors
    ///
    /// [`Truncated`] when fewer than eight bytes remain.
    pub fn u64(&mut self) -> Result<u64, Truncated> {
        let b = self.take(8)?;
        let mut word = [0; 8];
        word.copy_from_slice(b);
        Ok(u64::from_le_bytes(word))
    }

    /// An unsigned LEB128 varint of at most ten bytes.
    ///
    /// # Errors
    ///
    /// [`Truncated`] at the end of the input or on an overlong encoding.
    pub fn varint(&mut self) -> Result<u64, Truncated> {
        let start = self.at;
        let mut value = 0u64;
        for shift in (0..70).step_by(7) {
            let byte = self.u8()?;
            let bits = u64::from(byte & 0x7f);
            if shift == 63 && bits > 1 {
                return Err(Truncated(start));
            }
            value |= bits << shift;
            if byte & 0x80 == 0 {
                return Ok(value);
            }
        }
        Err(Truncated(start))
    }
}

/// Encodes one slot's snapshots, one per tick.
#[must_use]
pub fn encode_inputs(snapshots: &[InputSnapshot]) -> Vec<u8> {
    let mut out = Vec::new();
    put_varint(&mut out, snapshots.len() as u64);
    let mut previous = [0u32; 8];
    let mut i = 0;
    while i < snapshots.len() {
        let current = words(&snapshots[i]);
        let run = snapshots[i..]
            .iter()
            .take_while(|s| words(s) == current)
            .count();
        put_varint(&mut out, run as u64);
        let mut mask = 0u8;
        for (bit, (now, before)) in current.iter().zip(previous.iter()).enumerate() {
            if now != before {
                mask |= 1 << bit;
            }
        }
        out.push(mask);
        for (bit, word) in current.iter().enumerate() {
            if mask & (1 << bit) != 0 {
                out.extend_from_slice(&word.to_le_bytes());
            }
        }
        previous = current;
        i += run;
    }
    out
}

/// Decodes what [`encode_inputs`] wrote.
///
/// # Errors
///
/// [`Truncated`] on input that ends early, a zero-length run, or runs that do
/// not add up to the stated tick count.
pub fn decode_inputs(bytes: &[u8]) -> Result<Vec<InputSnapshot>, Truncated> {
    let mut cursor = Cursor::new(bytes);
    let total = usize::try_from(cursor.varint()?).map_err(|_| Truncated(0))?;
    // Bounded by what the bytes could possibly describe, so a corrupt count
    // cannot ask for a huge allocation up front.
    let mut out = Vec::with_capacity(total.min(bytes.len().saturating_mul(64)));
    let mut previous = [0u32; 8];
    while out.len() < total {
        let at = cursor.position();
        let run = usize::try_from(cursor.varint()?).map_err(|_| Truncated(at))?;
        if run == 0 || out.len().saturating_add(run) > total {
            return Err(Truncated(at));
        }
        let mask = cursor.u8()?;
        for (bit, word) in previous.iter_mut().enumerate() {
            if mask & (1 << bit) != 0 {
                *word = cursor.u32()?;
            }
        }
        let snapshot = snapshot(previous);
        out.extend(std::iter::repeat_n(snapshot, run));
    }
    if !cursor.is_empty() {
        return Err(Truncated(cursor.position()));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_gameplay::input::Button;

    fn held(mask: u32, previous: &mut Input) -> InputSnapshot {
        previous.begin_frame(mask);
        InputSnapshot {
            buttons: *previous,
            ..InputSnapshot::EMPTY
        }
    }

    #[test]
    fn varints_round_trip_at_every_width() {
        for value in [
            0,
            1,
            127,
            128,
            300,
            16_383,
            16_384,
            u64::from(u32::MAX),
            u64::MAX,
        ] {
            let mut out = Vec::new();
            put_varint(&mut out, value);
            let mut cursor = Cursor::new(&out);
            assert_eq!(cursor.varint(), Ok(value));
            assert!(cursor.is_empty());
        }
    }

    #[test]
    fn a_stream_round_trips_bit_for_bit_including_negative_zero_and_edges() {
        let mut input = Input::new();
        let mut stream = Vec::new();
        for tick in 0..500u32 {
            let mask = if tick % 97 < 60 {
                Button::Cross.bit()
            } else {
                0
            };
            let mut snapshot = held(mask, &mut input);
            snapshot.stick_x = if tick % 50 < 25 { -0.0 } else { 0.25 };
            snapshot.airbrake_left = (tick % 7) as f32 / 7.0;
            stream.push(snapshot);
        }
        // A consumed press: the edge a replay must keep rather than rebuild.
        stream[3].buttons.consume_press(Button::Cross);
        let decoded = decode_inputs(&encode_inputs(&stream)).expect("decodes");
        assert_eq!(decoded.len(), stream.len());
        for (a, b) in decoded.iter().zip(&stream) {
            assert_eq!(words(a), words(b));
        }
    }

    #[test]
    fn a_held_throttle_costs_a_handful_of_bytes() {
        let mut input = Input::new();
        let stream: Vec<_> = (0..3_000)
            .map(|_| held(Button::Cross.bit(), &mut input))
            .collect();
        let bytes = encode_inputs(&stream);
        // First tick (pressed edge), second (held_last catches up), the rest.
        assert!(bytes.len() < 40, "{} bytes", bytes.len());
    }

    #[test]
    fn corrupt_streams_are_refused_rather_than_misread() {
        let stream = vec![InputSnapshot::EMPTY; 10];
        let bytes = encode_inputs(&stream);
        assert!(decode_inputs(&bytes[..bytes.len() - 1]).is_err());
        let mut long = bytes.clone();
        long.push(0);
        assert!(decode_inputs(&long).is_err(), "trailing bytes");
        // A zero-length run would loop forever if it were accepted.
        assert!(decode_inputs(&[1, 0, 0]).is_err());
    }
}
