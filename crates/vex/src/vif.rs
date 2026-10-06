//! Walking a PlayStation 2 VIF1 packet.
//!
//! The PS2 has no interleaved vertex arrays. Geometry reaches the GS through
//! **VIF**, the VPU interface: a stream of 32-bit command words, each optionally
//! followed by data, which unpack attribute arrays into VU1 memory and run a
//! microprogram over them. Where a PSP `.vex` batch holds one interleaved
//! vertex array, its PS2 counterpart holds one of these packets, each attribute
//! in its **own** unpack at its own VU address.
//!
//! This is the general walker: it knows nothing about `.vex`, and what the
//! unpacked arrays mean is [`crate::vex`]'s business.
//!
//! This module is the general walker. It knows nothing about `.vex`; what the
//! unpacked arrays mean is [`crate::vex`]'s business.
//!
//! ```text
//! VIFcode:
//!   bits 31      interrupt
//!   bits 30-24   command
//!   bits 23-16   NUM
//!   bits 15-0    IMMEDIATE
//! ```
//!
//! # Only what the data contains
//!
//! Commands Pulse's packets do not use are still decoded where their length is
//! fixed and unambiguous, since skipping one with the wrong data length
//! desynchronises everything after it. Anything genuinely unknown is an
//! [`Error::UnknownCommand`], not a guess: a mis-stepped walk would produce
//! plausible garbage vertices.
//!
//! Evidence is in `docs/formats/vex.md`; a wrong reading fails to end the packet
//! exactly, which the ground-truth tests assert.

use std::fmt;
use std::ops::Range;

/// A VIF command word, decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Code {
    /// `NOP`, which is also how a packet pads to a quadword.
    Nop,
    /// `STCYCL`: the write cycle. `cl` quadwords are skipped for every `wl`
    /// written, which is what interleaves the attributes of one vertex.
    StCycl {
        /// Cycle length.
        cl: u8,
        /// Write length.
        wl: u8,
    },
    /// `OFFSET`, `BASE`, `ITOP`, `STMOD`, `MSKPATH3` or `MARK`: a register write
    /// with no data after it.
    Register {
        /// The command byte, so a caller can tell which.
        command: u8,
        /// The immediate field.
        immediate: u16,
    },
    /// One of the `FLUSH` family.
    Flush,
    /// `STMASK`, followed by one word of mask.
    StMask(u32),
    /// `STROW`, followed by four words of fill.
    StRow([u32; 4]),
    /// `STCOL`, followed by four words of fill.
    StCol([u32; 4]),
    /// `MSCAL`/`MSCALF`: start the microprogram at an address.
    Mscal(u16),
    /// `MSCNT`: continue the microprogram where it left off.
    ///
    /// What ends a batch of unpacks in Pulse's packets, so the boundary a caller
    /// groups attribute arrays on.
    Mscnt,
    /// `UNPACK`: an attribute array written into VU memory.
    Unpack(Unpack),
}

/// One `UNPACK` command and the bytes it carries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unpack {
    /// How many elements. A `NUM` of zero means 256, per the hardware.
    pub count: usize,
    /// Components per element: 1, 2, 3 or 4.
    pub components: usize,
    /// How each component is stored.
    pub format: Format,
    /// Whether the write is masked, so some components come from the `STROW` or
    /// `STCOL` registers instead of the data.
    pub masked: bool,
    /// Destination address in VU memory, in quadwords.
    pub address: u16,
    /// Whether the address is relative to the double-buffer base.
    pub relative: bool,
    /// Whether integer components are unsigned.
    pub unsigned: bool,
    /// Where the element data is, as a range into the walked slice.
    pub data: Range<usize>,
}

/// How an unpacked component is stored.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Format {
    /// 32 bits per component.
    Bits32,
    /// 16 bits per component.
    Bits16,
    /// 8 bits per component.
    Bits8,
    /// The packed `V4-5` form: one 16-bit word holding four 5-bit components.
    V4_5,
}

impl Format {
    /// Bytes one element occupies, given its component count.
    #[must_use]
    pub fn element_size(self, components: usize) -> usize {
        match self {
            Self::Bits32 => 4 * components,
            Self::Bits16 => 2 * components,
            Self::Bits8 => components,
            // Four components in one 16-bit word: the count does not enter in.
            Self::V4_5 => 2,
        }
    }
}

/// Why a packet did not walk.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A command word this module will not guess at.
    UnknownCommand {
        /// The command byte, with the interrupt bit already masked off.
        command: u8,
        /// Where it was, as an offset into the walked slice.
        at: usize,
    },
    /// A command's data runs past the end of the packet.
    Truncated {
        /// What was being read.
        what: &'static str,
        /// Where it would end.
        end: usize,
        /// How long the packet is.
        len: usize,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownCommand { command, at } => {
                write!(f, "unknown VIF command {command:#04x} at {at}")
            }
            Self::Truncated { what, end, len } => {
                write!(f, "{what} ends at {end} but the packet is {len} bytes")
            }
        }
    }
}

impl std::error::Error for Error {}

/// Result alias for this module.
pub type Result<T> = std::result::Result<T, Error>;

/// Walks a whole VIF packet.
///
/// `data` is the command stream only (no DMA tag, no framing); the ranges in
/// [`Unpack::data`] index it.
pub fn walk(data: &[u8]) -> Result<Vec<Code>> {
    let mut out = Vec::new();
    let mut at = 0usize;

    while at + 4 <= data.len() {
        let word = u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]]);
        // Bit 31 is the interrupt bit and does not change the command.
        let command = ((word >> 24) & 0x7f) as u8;
        let num = ((word >> 16) & 0xff) as usize;
        let immediate = (word & 0xffff) as u16;
        at += 4;

        let code = match command {
            NOP => Code::Nop,
            STCYCL => Code::StCycl {
                cl: (immediate & 0xff) as u8,
                wl: (immediate >> 8) as u8,
            },
            OFFSET | BASE | ITOP | STMOD | MSKPATH3 | MARK => Code::Register { command, immediate },
            FLUSHE | FLUSH | FLUSHA => Code::Flush,
            MSCAL | MSCALF => Code::Mscal(immediate),
            MSCNT => Code::Mscnt,
            STMASK => {
                let end = at + 4;
                let word = word_at(data, at, "STMASK")?;
                at = end;
                Code::StMask(word)
            }
            STROW | STCOL => {
                let end = at + 16;
                let mut fill = [0u32; 4];
                for (i, slot) in fill.iter_mut().enumerate() {
                    *slot = word_at(data, at + i * 4, "STROW/STCOL")?;
                }
                at = end;
                if command == STROW {
                    Code::StRow(fill)
                } else {
                    Code::StCol(fill)
                }
            }
            _ if command & UNPACK_MASK == UNPACK => {
                // The low nibble is the element description: two bits of
                // component count and two of storage width.
                let components = usize::from((command >> 2) & 3) + 1;
                let format = match command & 3 {
                    0 => Format::Bits32,
                    1 => Format::Bits16,
                    2 => Format::Bits8,
                    _ => Format::V4_5,
                };
                // NUM is 8 bits and zero means the full 256, per the hardware.
                let count = if num == 0 { 256 } else { num };
                let len = count * format.element_size(components);
                let end = at + len;
                if end > data.len() {
                    return Err(Error::Truncated {
                        what: "UNPACK data",
                        end,
                        len: data.len(),
                    });
                }
                let unpack = Unpack {
                    count,
                    components,
                    format,
                    masked: command & UNPACK_MASKED != 0,
                    address: immediate & 0x3ff,
                    relative: immediate & 0x8000 != 0,
                    unsigned: immediate & 0x4000 != 0,
                    data: at..end,
                };
                // Command words are word-aligned: a byte-sized unpack of an odd
                // length pads before the next.
                at = end.next_multiple_of(4);
                Code::Unpack(unpack)
            }
            _ => {
                return Err(Error::UnknownCommand {
                    command,
                    at: at - 4,
                });
            }
        };
        out.push(code);
    }

    Ok(out)
}

fn word_at(data: &[u8], at: usize, what: &'static str) -> Result<u32> {
    if at + 4 > data.len() {
        return Err(Error::Truncated {
            what,
            end: at + 4,
            len: data.len(),
        });
    }
    Ok(u32::from_le_bytes([
        data[at],
        data[at + 1],
        data[at + 2],
        data[at + 3],
    ]))
}

const NOP: u8 = 0x00;
const STCYCL: u8 = 0x01;
const OFFSET: u8 = 0x02;
const BASE: u8 = 0x03;
const ITOP: u8 = 0x04;
const STMOD: u8 = 0x05;
const MSKPATH3: u8 = 0x06;
const MARK: u8 = 0x07;
const FLUSHE: u8 = 0x10;
const FLUSH: u8 = 0x11;
const FLUSHA: u8 = 0x13;
const MSCAL: u8 = 0x14;
const MSCALF: u8 = 0x15;
const MSCNT: u8 = 0x17;
const STMASK: u8 = 0x20;
const STROW: u8 = 0x30;
const STCOL: u8 = 0x31;

/// The bit pattern every `UNPACK` command shares.
const UNPACK: u8 = 0x60;

/// The bits that identify an `UNPACK`, ignoring its element description and its
/// mask bit.
const UNPACK_MASK: u8 = 0x60;

/// The `m` bit of an `UNPACK` command.
const UNPACK_MASKED: u8 = 0x10;

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a command word the way the hardware lays one out.
    fn code(command: u8, num: u8, immediate: u16) -> [u8; 4] {
        (u32::from(command) << 24 | u32::from(num) << 16 | u32::from(immediate)).to_le_bytes()
    }

    fn packet(parts: &[&[u8]]) -> Vec<u8> {
        parts.concat()
    }

    #[test]
    fn an_empty_packet_walks_to_nothing() {
        assert_eq!(walk(&[]), Ok(Vec::new()));
    }

    #[test]
    fn the_interrupt_bit_does_not_change_the_command() {
        let stream = packet(&[&code(0x80, 0, 0)]);
        assert_eq!(walk(&stream), Ok(vec![Code::Nop]));
    }

    #[test]
    fn a_cycle_register_carries_its_two_halves() {
        let stream = packet(&[&code(STCYCL, 0, 0x0104)]);
        assert_eq!(walk(&stream), Ok(vec![Code::StCycl { cl: 4, wl: 1 }]));
    }

    #[test]
    fn a_mask_register_consumes_the_word_after_it() {
        let stream = packet(&[
            &code(STMASK, 0, 0),
            &0x8080_8080u32.to_le_bytes(),
            &code(MSCNT, 0, 0),
        ]);
        assert_eq!(
            walk(&stream),
            Ok(vec![Code::StMask(0x8080_8080), Code::Mscnt])
        );
    }

    #[test]
    fn a_row_register_consumes_four_words() {
        let stream = packet(&[
            &code(STROW, 0, 0),
            &1u32.to_le_bytes(),
            &2u32.to_le_bytes(),
            &3u32.to_le_bytes(),
            &4u32.to_le_bytes(),
            &code(NOP, 0, 0),
        ]);
        assert_eq!(
            walk(&stream),
            Ok(vec![Code::StRow([1, 2, 3, 4]), Code::Nop])
        );
    }

    /// The element description is the part that decides how far the walk steps,
    /// so every combination Pulse uses is pinned.
    #[test]
    fn unpack_lengths_follow_the_element_description() {
        // V3_32 of 4 elements: 4 * 3 * 4 bytes.
        let data = vec![0u8; 48];
        let stream = packet(&[&code(0x68, 4, 0xc004), &data, &code(MSCNT, 0, 0)]);
        let codes = walk(&stream).expect("walks");
        let Code::Unpack(unpack) = &codes[0] else {
            panic!("expected an unpack, got {:?}", codes[0]);
        };
        assert_eq!(unpack.components, 3);
        assert_eq!(unpack.format, Format::Bits32);
        assert_eq!(unpack.count, 4);
        assert_eq!(unpack.address, 4);
        assert!(unpack.relative, "bit 15 is the double-buffer flag");
        assert!(unpack.unsigned, "bit 14 is the sign flag");
        assert!(!unpack.masked);
        assert_eq!(unpack.data, 4..52);
        assert_eq!(codes[1], Code::Mscnt, "the walk landed on the next command");
    }

    #[test]
    fn a_masked_unpack_is_flagged_and_the_same_length() {
        let data = vec![0u8; 24];
        let stream = packet(&[&code(0x78, 2, 0), &data]);
        let codes = walk(&stream).expect("walks");
        let Code::Unpack(unpack) = &codes[0] else {
            panic!("expected an unpack");
        };
        assert!(unpack.masked);
        assert_eq!(unpack.components, 3);
        assert_eq!(unpack.data, 4..28);
    }

    #[test]
    fn a_byte_unpack_pads_to_the_next_command_word() {
        // V4_8 of 7 elements is 28 bytes, which is already word-aligned; a
        // single S_8 of 1 element is one byte and is not.
        let stream = packet(&[&code(0x62, 1, 0), &[0xaa, 0, 0, 0], &code(MSCNT, 0, 0)]);
        let codes = walk(&stream).expect("walks");
        assert_eq!(codes.len(), 2);
        assert_eq!(codes[1], Code::Mscnt);
    }

    #[test]
    fn a_num_of_zero_means_two_hundred_and_fifty_six() {
        let data = vec![0u8; 256 * 4];
        let stream = packet(&[&code(0x60, 0, 0), &data]);
        let codes = walk(&stream).expect("walks");
        let Code::Unpack(unpack) = &codes[0] else {
            panic!("expected an unpack");
        };
        assert_eq!(unpack.count, 256);
    }

    #[test]
    fn an_unpack_running_past_the_end_is_an_error_not_a_short_read() {
        let stream = packet(&[&code(0x68, 4, 0), &[0u8; 8]]);
        assert_eq!(
            walk(&stream),
            Err(Error::Truncated {
                what: "UNPACK data",
                end: 52,
                len: 12
            })
        );
    }

    #[test]
    fn an_unknown_command_stops_the_walk() {
        let stream = packet(&[&code(NOP, 0, 0), &code(0x4a, 1, 0)]);
        assert_eq!(
            walk(&stream),
            Err(Error::UnknownCommand {
                command: 0x4a,
                at: 4
            })
        );
    }
}
