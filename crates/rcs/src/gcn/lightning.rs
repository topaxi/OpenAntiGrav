//! The rates of `2048_shipshield_old`'s pixel program, read from its code.
//!
//! The program samples the diffuse picture at `(u + A * time, v)` for its colour, and a
//! second picture, `LightningTexture`, twice: once for a noise at
//! `(u2, B * v2 + C * time)` (its alpha), and once for the colour at
//! `(u2 + time + noise, v2 + time + noise)`. `A`, `B` and `C` are not parameters of the
//! material; they are literals of the program, `v_madmk_f32 .., A, ..`, `v_mul_f32 C, ..` and
//! `v_madmk_f32 .., B, ..` in that order (`docs/ghidra/functions/ps4-omega-eu/shield.md`,
//! "The Lightning layer"). The rate of the second picture's own scroll, `1`, is a plain
//! `v_add_f32 time, uv`, not a literal.

use super::isa::{Encoding, Operand};
use super::{Program, Stage};

/// The three literals of the program.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LightningRates {
    /// `A`: how far the diffuse colour's `u` moves per second.
    pub diffuse_scroll: f32,
    /// `B`: the noise tap's `v` scale on the second coordinate set.
    pub noise_scale: f32,
    /// `C`: how far the noise tap's `v` moves per second.
    pub noise_rate: f32,
}

impl Program<'_> {
    /// The rates, for a pixel program that declares a `LightningTexture`; `None` for any other
    /// program or one whose literals do not feed the taps described above.
    ///
    /// **Roles come from data flow, not from the order the compiler scheduled the
    /// instructions in** (it varies across the program's variants): a `v_madmk_f32` whose result
    /// addresses a sample of the diffuse texture carries `A`; one that addresses a sample of the
    /// lightning texture carries `B`, and the register it adds in was last written by the
    /// `v_mul_f32` that carries `C`.
    #[must_use]
    pub fn lightning_rates(&self) -> Option<LightningRates> {
        if self.stage != Stage::Pixel {
            return None;
        }
        let reflection = self.reflection.as_ref().ok()?;
        let texture_named = |slot: u8| {
            reflection
                .resources
                .iter()
                .find(|r| r.size == 0 && r.slot == u32::from(slot))
                .map(|r| r.name.as_str())
        };
        let mut diffuse_scroll = None;
        let mut noise = None;
        for (k, madmk) in self.instructions.iter().enumerate() {
            if madmk.name != Some("v_madmk_f32") {
                continue;
            }
            let (Some(bits), Some(dst), Some(addend)) =
                (madmk.literal(), madmk.dst[0], madmk.src[2])
            else {
                continue;
            };
            let value = f32::from_bits(bits);
            let Some(slot) = self.instructions[k + 1..]
                .iter()
                .enumerate()
                .find_map(|(n, s)| {
                    let sample = s.encoding == Encoding::Mimg
                        && s.name.is_some_and(|name| name.contains("sample"));
                    let (true, Some(address), Some(Operand::Sgpr { first: t, .. })) =
                        (sample, s.src[0], s.src[1])
                    else {
                        return None;
                    };
                    let Operand::Vgpr { first, count } = dst else {
                        return None;
                    };
                    // A 2D sample's address is two consecutive registers; the decoder names the first.
                    let Operand::Vgpr { first: a, .. } = address else {
                        return None;
                    };
                    super::sample::overlaps(&Operand::Vgpr { first: a, count: 2 }, first, count)
                        .then(|| self.texture_slot(k + 1 + n, t))
                        .flatten()
                })
            else {
                continue;
            };
            match texture_named(slot) {
                Some("DiffuseTextureTexture") => diffuse_scroll = Some(value),
                Some("LightningTextureTexture") => {
                    let Operand::Vgpr { first, count } = addend else {
                        continue;
                    };
                    let writer = self.instructions[..k].iter().rev().find(|w| {
                        matches!(w.dst[0], Some(d) if super::sample::overlaps(&d, first, count))
                    })?;
                    if writer.name != Some("v_mul_f32") {
                        return None;
                    }
                    noise = Some((value, f32::from_bits(writer.literal()?)));
                }
                _ => {}
            }
        }
        let (noise_scale, noise_rate) = noise?;
        Some(LightningRates {
            diffuse_scroll: diffuse_scroll?,
            noise_scale,
            noise_rate,
        })
    }
}

/// The rates of a `.rcsmaterial` file, where every pixel program that declares the
/// `LightningTexture` agrees on them; `None` for a file with none, or whose programs differ.
#[must_use]
pub fn lightning_rates(file: &[u8]) -> Option<LightningRates> {
    let mut found: Option<LightningRates> = None;
    for (_, program) in super::scan(file) {
        let Ok(program) = program else { continue };
        let is_lightning = program.stage == Stage::Pixel
            && program.reflection.as_ref().is_ok_and(|r| {
                r.resources
                    .iter()
                    .any(|r| r.name == "LightningTextureTexture")
            });
        if !is_lightning {
            continue;
        }
        let rates = program.lightning_rates()?;
        if found.is_some_and(|f| f != rates) {
            return None;
        }
        found = Some(rates);
    }
    found
}
