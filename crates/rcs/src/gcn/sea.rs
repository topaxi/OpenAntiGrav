//! Omega's reflective sea (`reflectplane_mid` and its siblings), read from its pixel programs.
//!
//! The lit pixel programs declare two wave normal maps, `NormalMap` and `NormalMap1`, the
//! planar reflection `screenSpaceReflectionTex`, and the constants `time`, `Speed` and
//! `ReflectivityMin`, `ReflectivityScale`, `ReflectivityPower`, `ReflectivityStrength`. Each
//! fetches the two maps at
//!
//! ```text
//! s = time * Speed                          v_mul_f32 s, time, Speed
//! NormalMap  at (u + K * s, v)              v_madmk_f32 a, s, K, u
//! NormalMap1 at (K * s - u, v)              v_mad_f32   b, K, s, -u
//! ```
//!
//! `K` is a literal of the program, not a parameter of the material (`0.05` on every program
//! read). The rest of the law, the Fresnel weight and the highlight, is on
//! `docs/rendering/water.md`, "Omega (PS4)".
//!
//! **Roles come from data flow, not from the order the compiler scheduled the instructions
//! in**: the register that addresses a sample of each map is followed back to the instruction
//! that wrote it, and the product it scrolls by to the two constants it multiplies.

use super::isa::{Encoding, Instruction, Operand};
use super::{Program, Stage};

/// The two taps of one pixel program.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SeaTaps {
    /// `K`: how far both taps move along `u` per unit of `time * Speed`.
    pub rate: f32,
    /// The sign of `u` in `NormalMap`'s tap and in `NormalMap1`'s.
    pub signs: [f32; 2],
}

/// What a `.rcsmaterial` file's programs say of its sea.
#[derive(Clone, Debug, PartialEq)]
pub struct Sea {
    pub taps: SeaTaps,
    /// The constant the programs multiply the light by: `Watercolour`, or `Watercolour1` on the
    /// blended edge.
    pub colour: String,
    /// Whether the highlight is also times `SpecularStrength` (the pond and the tower's sea).
    pub specular_strength: bool,
    /// Whether a vertex program fetches `VertexColour1` and declares `Opacity`: the blended
    /// edge, whose alpha is `VertexColour1.y * Opacity`.
    pub opacity: bool,
    /// How many pixel programs carry the interface; all agree on the taps.
    pub programs: usize,
}

const NORMAL_MAP: &str = "NormalMapTexture";
const NORMAL_MAP1: &str = "NormalMap1Texture";
const REFLECTION: &str = "screenSpaceReflectionTexTexture";
const WEIGHTS: [&str; 4] = [
    "ReflectivityMin",
    "ReflectivityScale",
    "ReflectivityPower",
    "ReflectivityStrength",
];

impl Program<'_> {
    /// The last instruction before `k` that writes vector register `register`.
    fn writer_of(&self, k: usize, register: u8) -> Option<(usize, &Instruction)> {
        self.instructions[..k]
            .iter()
            .enumerate()
            .rev()
            .find(|(_, i)| matches!(i.dst[0], Some(d) if super::sample::overlaps(&d, register, 1)))
    }

    /// The name of the constant scalar register `sgpr` holds at instruction `k`: the member of
    /// the one constant buffer that the last `s_buffer_load` into it read.
    fn constant_in(&self, k: usize, sgpr: u8) -> Option<&str> {
        let reflection = self.reflection.as_ref().ok()?;
        let buffer = reflection.constant_buffer()?;
        let (load, first) = self.instructions[..k]
            .iter()
            .rev()
            .find_map(|i| match i.dst[0] {
                Some(Operand::Sgpr { first, count }) if (first..first + count).contains(&sgpr) => {
                    Some((i, first))
                }
                _ => None,
            })?;
        if !load.name.is_some_and(|n| n.starts_with("s_buffer_load")) {
            return None;
        }
        let Some(Operand::Imm(dword)) = load.src[1] else {
            return None;
        };
        reflection
            .constant_at(buffer, dword + u32::from(sgpr - first))
            .map(|(constant, _)| constant.name.as_str())
    }

    /// The constant a multiply's operand is: a scalar register, or a vector register a
    /// `v_mov_b32` filled from one.
    fn constant_operand(&self, k: usize, operand: Option<Operand>) -> Option<&str> {
        match operand? {
            Operand::Sgpr { first, .. } => self.constant_in(k, first),
            Operand::Vgpr { first, .. } => {
                let (at, mov) = self.writer_of(k, first)?;
                let Some(Operand::Sgpr { first: s, .. }) = mov.src[0] else {
                    return None;
                };
                (mov.name == Some("v_mov_b32"))
                    .then(|| self.constant_in(at, s))
                    .flatten()
            }
            _ => None,
        }
    }

    /// One tap: the literal and `u`'s sign in the instruction that wrote the sample's address
    /// register `address`, and the register it scrolls by. Either shape may feed either map.
    fn sea_tap(&self, k: usize, address: u8) -> Option<(f32, f32, u8, usize)> {
        let (at, writer) = self.writer_of(k, address)?;
        match writer.name? {
            "v_madmk_f32" => {
                let Some(Operand::Vgpr { first: scroll, .. }) = writer.src[0] else {
                    return None;
                };
                Some((f32::from_bits(writer.literal()?), 1.0, scroll, at))
            }
            "v_mad_f32" => {
                // The literal sits in a scalar register an `s_mov_b32` filled: an SGPR, or
                // `vcc_lo` where the compiler ran out of them.
                let (Some(rate), Some(Operand::Vgpr { first, .. })) =
                    (writer.src[0], writer.src[1])
                else {
                    return None;
                };
                let literal =
                    self.instructions[..at]
                        .iter()
                        .rev()
                        .find(|i| match (i.dst[0], rate) {
                            (
                                Some(Operand::Sgpr { first: s, count }),
                                Operand::Sgpr { first: r, .. },
                            ) => (s..s + count).contains(&r),
                            (Some(written), register) => written == register,
                            (None, _) => false,
                        })?;
                if literal.name != Some("s_mov_b32") {
                    return None;
                }
                let sign = if writer.neg & 0b100 == 0 { 1.0 } else { -1.0 };
                Some((f32::from_bits(literal.literal()?), sign, first, at))
            }
            _ => None,
        }
    }

    /// The two taps, for a pixel program that samples `NormalMap` and `NormalMap1` as the
    /// module docs describe; `None` for any other program, or one whose taps do not both scroll
    /// by the one product of `time` and `Speed` at the one literal.
    #[must_use]
    pub fn sea_taps(&self) -> Option<SeaTaps> {
        if self.stage != Stage::Pixel {
            return None;
        }
        let reflection = self.reflection.as_ref().ok()?;
        let slot_of = |name: &str| {
            reflection
                .resources
                .iter()
                .find(|r| r.size == 0 && r.name == name)
                .and_then(|r| u8::try_from(r.slot).ok())
        };
        let maps = [slot_of(NORMAL_MAP)?, slot_of(NORMAL_MAP1)?];
        let mut taps: [Option<(f32, f32, u8, usize)>; 2] = [None; 2];
        for (k, sample) in self.instructions.iter().enumerate() {
            let is_sample = sample.encoding == Encoding::Mimg
                && sample.name.is_some_and(|n| n.contains("sample"));
            let (
                true,
                Some(Operand::Vgpr { first: address, .. }),
                Some(Operand::Sgpr { first: t, .. }),
            ) = (is_sample, sample.src[0], sample.src[1])
            else {
                continue;
            };
            let Some(slot) = self.texture_slot(k, t) else {
                continue;
            };
            let Some(which) = maps.iter().position(|&m| m == slot) else {
                continue;
            };
            if taps[which].is_some() {
                return None;
            }
            taps[which] = Some(self.sea_tap(k, address)?);
        }
        let [
            Some((rate, first, scroll, at)),
            Some((rate1, second, scroll1, at1)),
        ] = taps
        else {
            return None;
        };
        if rate.to_bits() != rate1.to_bits() || scroll != scroll1 {
            return None;
        }
        // Both taps scroll by one register, the product of `time` and `Speed` in either order.
        let (at, product) = self.writer_of(at.min(at1), scroll)?;
        if product.name != Some("v_mul_f32") {
            return None;
        }
        let mut factors = [
            self.constant_operand(at, product.src[0])?,
            self.constant_operand(at, product.src[1])?,
        ];
        factors.sort_unstable();
        (factors == ["Speed", "time"]).then_some(SeaTaps {
            rate,
            signs: [first, second],
        })
    }
}

/// The sea a `.rcsmaterial` file's programs draw: every pixel program that declares the two
/// normal maps and the planar reflection also declares `time`, `Speed` and the four
/// `Reflectivity*` weights, and all agree on the taps. `None` for a file with no such program,
/// or whose programs differ.
#[must_use]
pub fn sea(file: &[u8]) -> Option<Sea> {
    let mut found: Option<Sea> = None;
    let mut opacity = false;
    for (_, program) in super::scan(file) {
        let Ok(program) = program else { continue };
        let Ok(reflection) = &program.reflection else {
            continue;
        };
        let declares = |name: &str| reflection.constants.iter().any(|c| c.name == name);
        if program.stage == Stage::Vertex {
            opacity |=
                declares("Opacity") && reflection.inputs.iter().any(|a| a.name == "VertexColour1");
            continue;
        }
        let has = |name: &str| reflection.resources.iter().any(|r| r.name == name);
        if !(has(NORMAL_MAP) && has(NORMAL_MAP1) && has(REFLECTION)) {
            continue;
        }
        if !WEIGHTS.iter().all(|name| declares(name)) {
            return None;
        }
        let colour = ["Watercolour", "Watercolour1"]
            .into_iter()
            .find(|name| declares(name))?;
        let this = Sea {
            taps: program.sea_taps()?,
            colour: colour.to_string(),
            specular_strength: declares("SpecularStrength"),
            opacity: false,
            programs: 1,
        };
        found = Some(match found {
            None => this,
            Some(seen)
                if seen.taps == this.taps
                    && seen.colour == this.colour
                    && seen.specular_strength == this.specular_strength =>
            {
                Sea {
                    programs: seen.programs + 1,
                    ..seen
                }
            }
            Some(_) => return None,
        });
    }
    found.map(|sea| Sea { opacity, ..sea })
}
