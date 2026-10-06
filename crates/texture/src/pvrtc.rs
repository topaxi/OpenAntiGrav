//! PowerVR texture compression, PVRTC-**II** at 4 bits per texel - the codec
//! `SceGxmTextureBaseFormat`'s `PVRTII4BPP` (format byte `0x83`) names, and
//! what almost every texture Wipeout 2048 ships is stored in.
//!
//! Lives beside [`crate::bcn`]: the block layout is Imagination Technologies'
//! hardware standard, not [`crate::gxt`]'s container's, so the container's
//! header rules stay there and the codec lives here.
//!
//! # Nothing about this codec is per-block
//!
//! BC expands one 4x4 block into sixteen texels. PVRTC does not, and a port that
//! treats it as a block codec produces a plausible-looking wrong picture:
//!
//! - A **word** is 8 bytes covering 4x4 texels but stores only *two* colours (A
//!   and B) plus sixteen 2-bit modulation values.
//! - Every output texel bilinearly interpolates the A colours of the **four**
//!   words around it, likewise B, then blends the two by its own modulation.
//! - Words are stored in **Morton order** ([`crate::gxt::twiddle`]), applied
//!   **once**, inside this codec.
//!
//! # PVRTC-II is not PVRTC-I
//!
//! The public PowerVR SDK decompressor (`PVRTDecompress.cpp`) implements PVRTC-**I**
//! only. Its bit layout is a near-lookalike, so it renders a `PVRTII4BPP`
//! payload as a wrong picture. Three differences, all implemented here:
//!
//! 1. **One opacity flag, not two.** PVRTC-I gives colour A its own opaque bit
//!    at bit 15 and colour B one at bit 31. PVRTC-II spends bit 15 on the
//!    hard-transition flag instead and lets **bit 31 answer for both**
//!    colours - see [`colour_a`].
//! 2. **A hard-transition mode.** With bit 15 set on the word to the
//!    north-west, the central 4x4 texels of a word quad stop interpolating and
//!    take one word's own colours flat (`+20` below), or take them through a
//!    local palette (`+30`) - see [`unpack_modulations`] and [`palette`].
//! 3. **Colour B's low alpha bit is forced to 1** in transparent mode, where
//!    PVRTC-I leaves it 0 - see [`colour_b`].
//!
//! # Where this reading comes from, and its confidence
//!
//! **Confidence 92.** The bit layout and decode arithmetic are a port of
//! **Vita3K**'s `vita3k/renderer/src/texture/pvrt-dec.cpp` (its PVRTC-II path
//! is credited there to the Vita3K team, as an addition to the Imagination SDK's
//! PVRTC-I decompressor): an emulator decoder real Vita titles run through, the
//! strongest external corroboration for a codec Sony never documented (the same
//! class as the `ClassiCube` finding behind `docs/formats/gxt.md`'s twiddle).
//!
//! **The 92 is the cross-title oracle's, not the reference's.** Checked here:
//!
//! - **Wipeout HD decodes the same art, and this agrees with it.** 2048's DLC
//!   re-ships HD/Fury's circuits and roster, so 2,284 textures exist as both a BC
//!   `.gtf` ([`crate::gtf`]) and a `PVRTII4BPP` `.gxt`. Median mean-absolute
//!   difference is **3.83** of 255, against 10.01 flipped, 34.60 in raster word
//!   order and 59.74 for a different texture (`docs/formats/gxt.md`,
//!   `crates/texture/tests/gxt_ground_truth.rs`).
//! - **The word size closes on the corpus**: all 10,204 `PVRTII4BPP` textures in
//!   the three EU packages match their mip chain at 8 bytes per word, floored at
//!   [`crate::gxt::MIN_LEVEL_LEN`].
//! - **A font atlas comes out legible**: `RussianHud.gxt` renders the full Latin
//!   and Cyrillic alphabets crisp and upright.
//! - **Synthetic words decode to the colours they name** (this module's tests).
//!
//! **What holds the 92 back**: the local-palette path (`+30`) needs the
//! hard-transition bit *and* modulation mode 1 on one quad, and **no texel in the
//! base package reaches it** (0 of 1,082,941,440). It is implemented from the
//! reference but unexercised, and the reference's palette index is transposed
//! relative to how the table reads - see [`palette`]. The hard-transition path
//! *is* exercised (7.1% of words set bit 15), so a PVRTC-I decoder would be wrong
//! on 7% of this corpus's words.

use crate::gxt::twiddle;

/// Bytes one word occupies: [`WORD_SIDE`] x [`WORD_SIDE`] texels at 4 bits each.
pub const WORD_LEN: usize = 8;

/// Texels a word spans, in each direction.
pub const WORD_SIDE: usize = 4;

/// Smallest surface this codec decodes, in texels, in each direction.
///
/// A word quad needs a 2x2 neighbourhood of words, so a smaller surface cannot
/// decode. A level below this decodes at this size with absent words read as
/// zero, then is cropped; 35 of the 10,204 `PVRTII4BPP` textures in the three
/// EU packages have a base level that needs this.
pub const MIN_SIDE: u32 = 8;

/// One 8-byte word: sixteen 2-bit modulation values, then the colour pair.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Word {
    modulation: u32,
    colour: u32,
}

/// A colour as the word stores it: five bits per channel, four of alpha.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct Colour {
    r: i32,
    g: i32,
    b: i32,
    a: i32,
}

impl Colour {
    fn map(self, f: impl Fn(i32) -> i32, alpha: impl Fn(i32) -> i32) -> Self {
        Self {
            r: f(self.r),
            g: f(self.g),
            b: f(self.b),
            a: alpha(self.a),
        }
    }

    fn zip(self, other: Self, f: impl Fn(i32, i32) -> i32) -> Self {
        Self {
            r: f(self.r, other.r),
            g: f(self.g, other.g),
            b: f(self.b, other.b),
            a: f(self.a, other.a),
        }
    }
}

/// Colour A, out of the word's colour half.
///
/// **The opacity flag is bit 31, not bit 15.** PVRTC-I reads colour A's opacity
/// at bit 15; PVRTC-II spends bit 15 on the hard-transition flag and lets bit
/// 31 answer for both colours. Reading it at 15 decodes half the words in the
/// wrong colour mode.
fn colour_a(data: u32) -> Colour {
    let d = data as i32;
    if data & 0x8000_0000 != 0 {
        // Opaque: RGB 5:5:4, blue's low bit replicated from its high one.
        Colour {
            r: (d & 0x7c00) >> 10,
            g: (d & 0x03e0) >> 5,
            b: (d & 0x001e) | ((d & 0x001e) >> 4),
            a: 0xf,
        }
    } else {
        // Transparent: ARGB 3:4:4:3, each channel widened to five bits and
        // alpha left with a zero in its low bit.
        Colour {
            r: ((d & 0x0f00) >> 7) | ((d & 0x0f00) >> 11),
            g: ((d & 0x00f0) >> 3) | ((d & 0x00f0) >> 7),
            b: ((d & 0x000e) << 1) | ((d & 0x000e) >> 2),
            a: (d & 0x7000) >> 11,
        }
    }
}

/// Colour B, out of the same word.
///
/// **Alpha's low bit is forced to 1** in transparent mode (PVRTC-I leaves it 0):
/// the third departure from PVRTC-I.
fn colour_b(data: u32) -> Colour {
    if data & 0x8000_0000 != 0 {
        // Opaque: RGB 5:5:5, no channel widening needed.
        Colour {
            r: ((data & 0x7c00_0000) >> 26) as i32,
            g: ((data & 0x03e0_0000) >> 21) as i32,
            b: ((data & 0x001f_0000) >> 16) as i32,
            a: 0xf,
        }
    } else {
        Colour {
            r: (((data & 0x0f00_0000) >> 23) | ((data & 0x0f00_0000) >> 27)) as i32,
            g: (((data & 0x00f0_0000) >> 19) | ((data & 0x00f0_0000) >> 23)) as i32,
            b: (((data & 0x000f_0000) >> 15) | ((data & 0x000f_0000) >> 19)) as i32,
            a: (((data & 0x7000_0000) >> 27) | 1) as i32,
        }
    }
}

/// Widens a stored colour to eight bits per channel without interpolating it -
/// what the hard-transition path uses in place of the bilinear result.
///
/// The reference's scaling, exact at both ends: a five-bit channel times 16 then
/// `(v >> 6) + (v >> 1)` maps 31 to 255; a four-bit alpha times 16 then
/// `(v >> 4) + v` maps 15 to 255.
fn expand(colour: Colour) -> Colour {
    colour
        .map(|v| v * 16, |v| v * 16)
        .map(|v| (v >> 6) + (v >> 1), |v| (v >> 4) + v)
}

/// Bilinearly interpolates one colour across the 4x4 texels a word quad
/// covers, from the four words' own colours.
///
/// `p` is the quad's top-left word, `q` its horizontal neighbour, `r` its
/// vertical one, `s` the diagonal. Indexed `[row][column]`, widened to eight
/// bits per channel as [`expand`] does.
fn interpolate(p: Colour, q: Colour, r: Colour, s: Colour) -> [[Colour; WORD_SIDE]; WORD_SIDE] {
    let q_minus_p = q.zip(p, |a, b| a - b);
    let s_minus_r = s.zip(r, |a, b| a - b);
    let mut hp = p.map(|v| v * WORD_SIDE as i32, |v| v * WORD_SIDE as i32);
    let mut hr = r.map(|v| v * WORD_SIDE as i32, |v| v * WORD_SIDE as i32);

    let mut out = [[Colour::default(); WORD_SIDE]; WORD_SIDE];
    for column in 0..WORD_SIDE {
        let mut result = hp.map(|v| v * 4, |v| v * 4);
        let step = hr.zip(hp, |a, b| a - b);
        for row in &mut out {
            row[column] = result.map(|v| (v >> 6) + (v >> 1), |v| (v >> 4) + v);
            result = result.zip(step, |a, b| a + b);
        }
        hp = hp.zip(q_minus_p, |a, b| a + b);
        hr = hr.zip(s_minus_r, |a, b| a + b);
    }
    out
}

/// The eight colours a word quad's local palette draws on, in the order
/// [`PALETTE`] names them: A and B of each of P, Q, R and S.
const PALETTE_SOURCES: usize = 8;

/// The local palette a hard-transition word in modulation mode 1 selects
/// from: for each of the sixteen texels, which of the quad's eight colours
/// each of the four modulation values picks.
///
/// Texel 0 is special-cased in [`palette`] (a four-step ramp between P's own two
/// colours), so its row here is a placeholder never read.
///
/// **The texel index is column-major** (`column * 4 + row`), as the reference
/// indexes it, though the table reads as if authored row-major: entry 1 names Q,
/// the *horizontal* neighbour, at what column-major indexing places one texel
/// *below* P. Ported as the reference computes it, the half with evidence behind
/// it; see the module doc's confidence note and `docs/formats/gxt.md` for how
/// often this path is reached.
const PALETTE: [[u8; 4]; 16] = [
    [0, 0, 0, 0], // texel 0: a P-only ramp, see `palette`
    [0, 1, 2, 3],
    [0, 1, 2, 3],
    [0, 1, 2, 3],
    [0, 1, 4, 5],
    [0, 1, 2, 5],
    [0, 1, 2, 3],
    [6, 1, 2, 3],
    [0, 1, 4, 5],
    [0, 1, 4, 5],
    [0, 7, 4, 3],
    [6, 7, 2, 3],
    [0, 1, 4, 5],
    [0, 7, 4, 5],
    [6, 7, 4, 5],
    [6, 7, 4, 3],
];

/// One entry of [`PALETTE`], resolved against a quad's eight expanded
/// colours.
fn palette(sources: &[Colour; PALETTE_SOURCES], texel: usize, modulation: usize) -> Colour {
    if texel == 0 {
        let (a, b) = (sources[0], sources[1]);
        return match modulation {
            0 => a,
            1 => a.zip(b, |x, y| (x * 5 + y * 3) / 8),
            2 => a.zip(b, |x, y| (x * 3 + y * 5) / 8),
            _ => b,
        };
    }
    sources[usize::from(PALETTE[texel][modulation])]
}

/// Writes one word's sixteen modulation values into the quad's 8x8 grid at
/// `(row_offset, column_offset)`.
///
/// `north_west` is the quad's own top-left word, whose bit 15 carries the
/// hard-transition flag for the whole quad.
///
/// The value written is a small tagged integer the blend step decodes, as in the
/// reference, not the raw 2-bit code:
///
/// | Written | Means |
/// | --- | --- |
/// | `0`, `3`, `5`, `8` | blend weight in eighths (mode 0's four steps) |
/// | `0`, `4`, `8` | blend weight in eighths (mode 1's opaque steps) |
/// | `14` | weight 4, and punch alpha through to zero |
/// | `+20` | do not interpolate: take one word's own colours flat |
/// | `+30` | take the quad's local palette instead |
fn unpack_modulations(
    word: Word,
    north_west: Word,
    row_offset: usize,
    column_offset: usize,
    out: &mut [[i32; 8]; 8],
) {
    let interpolated = word.colour & 1 != 0;
    let hard = north_west.colour & (1 << 15) != 0;
    let mut bits = word.modulation;

    for row in 0..WORD_SIDE {
        for column in 0..WORD_SIDE {
            let (r, c) = (row + row_offset, column + column_offset);
            // Only the centre 4x4 of the quad's 8x8 grid takes a hard transition,
            // and it is the part the blend step reads back.
            let central = (2..=5).contains(&r) && (2..=5).contains(&c);
            let code = (bits & 3) as i32;
            out[r][c] = if interpolated {
                if hard && central {
                    code + 30
                } else {
                    match code {
                        1 => 4,
                        // Weight 4 with alpha punched through to zero.
                        2 => 14,
                        3 => 8,
                        _ => 0,
                    }
                }
            } else {
                // 0, 3, 5, 8 - the four blend steps in eighths.
                let value = if code * 3 > 3 { code * 3 - 1 } else { code * 3 };
                if hard && central { value + 20 } else { value }
            };
            bits >>= 2;
        }
    }
}

/// Decodes the 4x4 texels one quad of words covers, indexed `[row][column]`.
///
/// The texels produced are **not** `p`'s own 4x4 but straddle all four words;
/// [`decode_ii_4bpp`]'s mapping step places them.
fn quad(p: Word, q: Word, r: Word, s: Word) -> [[[u8; 4]; WORD_SIDE]; WORD_SIDE] {
    let mut modulations = [[0i32; 8]; 8];
    unpack_modulations(p, p, 0, 0, &mut modulations);
    unpack_modulations(q, p, 0, WORD_SIDE, &mut modulations);
    unpack_modulations(r, p, WORD_SIDE, 0, &mut modulations);
    unpack_modulations(s, p, WORD_SIDE, WORD_SIDE, &mut modulations);

    let upscaled_a = interpolate(
        colour_a(p.colour),
        colour_a(q.colour),
        colour_a(r.colour),
        colour_a(s.colour),
    );
    let upscaled_b = interpolate(
        colour_b(p.colour),
        colour_b(q.colour),
        colour_b(r.colour),
        colour_b(s.colour),
    );
    let flat = [
        expand(colour_a(p.colour)),
        expand(colour_b(p.colour)),
        expand(colour_a(q.colour)),
        expand(colour_b(q.colour)),
        expand(colour_a(r.colour)),
        expand(colour_b(r.colour)),
        expand(colour_a(s.colour)),
        expand(colour_b(s.colour)),
    ];

    let mut out = [[[0u8; 4]; WORD_SIDE]; WORD_SIDE];
    for row in 0..WORD_SIDE {
        for column in 0..WORD_SIDE {
            let mut weight = modulations[row + 2][column + 2];
            let mut colours = (upscaled_a[row][column], upscaled_b[row][column]);
            let mut punch_through = false;
            let mut local = false;

            if weight >= 30 {
                local = true;
                weight -= 30;
            } else if weight >= 20 {
                // Which of the four words this texel's quadrant belongs to.
                let which =
                    usize::from(row >= WORD_SIDE / 2) * 2 + usize::from(column >= WORD_SIDE / 2);
                colours = (flat[which * 2], flat[which * 2 + 1]);
                weight -= 20;
            } else if weight > 10 {
                punch_through = true;
                weight -= 10;
            }

            let result = if local {
                palette(&flat, column * WORD_SIDE + row, weight as usize)
            } else {
                let (a, b) = colours;
                let mut blended = a.zip(b, |x, y| (x * (8 - weight) + y * weight) / 8);
                if punch_through {
                    blended.a = 0;
                }
                blended
            };
            out[row][column] = [
                result.r as u8,
                result.g as u8,
                result.b as u8,
                result.a as u8,
            ];
        }
    }
    out
}

/// Decodes one `PVRTII4BPP` surface to straight RGBA8, row-major.
///
/// `data` is the level's texels alone, [`WORD_LEN`] bytes per 4x4 word in
/// twiddled order. `None` only when the arithmetic overflows; short `data` reads
/// as zero words, which lets a level below [`MIN_SIDE`] decode.
///
/// # Panics
///
/// Never: every index derives from `width`/`height`, clamped to [`MIN_SIDE`].
#[must_use]
pub fn decode_ii_4bpp(data: &[u8], width: u32, height: u32) -> Option<Vec<[u8; 4]>> {
    let surface_w = width.max(MIN_SIDE) as usize;
    let surface_h = height.max(MIN_SIDE) as usize;
    let words_x = surface_w / WORD_SIDE;
    let words_y = surface_h / WORD_SIDE;
    let mut surface = vec![[0u8; 4]; surface_w.checked_mul(surface_h)?];

    let word_at = |x: usize, y: usize| -> Word {
        let index = twiddle(x as u32, y as u32, words_x as u32, words_y as u32) as usize;
        let at = index * WORD_LEN;
        let Some(bytes) = data.get(at..at + WORD_LEN) else {
            return Word::default();
        };
        Word {
            modulation: u32::from_le_bytes(bytes[0..4].try_into().expect("four bytes")),
            colour: u32::from_le_bytes(bytes[4..8].try_into().expect("four bytes")),
        }
    };

    // Each iteration decodes the 4x4 texels *between* four words, so the quad
    // grid is offset half a word from the word grid and wraps at both edges.
    for quad_y in 0..words_y {
        for quad_x in 0..words_x {
            let px = (quad_x + words_x - 1) % words_x;
            let py = (quad_y + words_y - 1) % words_y;
            let (qx, qy) = (quad_x, py);
            let (rx, ry) = (px, quad_y);
            let (sx, sy) = (quad_x, quad_y);
            let texels = quad(
                word_at(px, py),
                word_at(qx, qy),
                word_at(rx, ry),
                word_at(sx, sy),
            );

            let half = WORD_SIDE / 2;
            for row in 0..half {
                for column in 0..half {
                    let mut place = |wx: usize, wy: usize, dr: usize, dc: usize, texel| {
                        let y = wy * WORD_SIDE + row + dr;
                        let x = wx * WORD_SIDE + column + dc;
                        surface[y * surface_w + x] = texel;
                    };
                    place(px, py, half, half, texels[row][column]);
                    place(qx, qy, half, 0, texels[row][column + half]);
                    place(rx, ry, 0, half, texels[row + half][column]);
                    place(sx, sy, 0, 0, texels[row + half][column + half]);
                }
            }
        }
    }

    if surface_w == width as usize && surface_h == height as usize {
        return Some(surface);
    }
    let mut out = Vec::with_capacity((width as usize).checked_mul(height as usize)?);
    for y in 0..height as usize {
        out.extend_from_slice(&surface[y * surface_w..y * surface_w + width as usize]);
    }
    Some(out)
}

#[cfg(test)]
mod tests;
