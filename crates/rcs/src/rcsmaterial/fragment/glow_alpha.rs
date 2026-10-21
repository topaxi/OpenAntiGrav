//! What a fragment program writes into the output's alpha lane.
//!
//! Wipeout HD's bloom reads the scene target's alpha as its glow mask
//! (`FunkLayerBloom`'s gate multiplies the frame's colour by it), and the
//! engine's programs write that lane themselves: a constant, a texture's
//! channel times a constant, or an expression this reading does not follow.
//! A live frame of Metropia shows it plainly - the "Be" neon sign writes 255,
//! the sky a flat grey, the track's pad stripes a graded value, the lit road
//! zero - and the programs end `MOV H0.w, <constant>` or `MOV H0.w, H1` after
//! the colour lanes (`docs/rendering/hd-glow-mask.md`).

use super::{Instruction, Program, Source};

/// The value of one lane as `scale * texel + bias`, or a plain constant, or
/// nothing this reading can follow.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum Lane {
    /// The program never writes this lane (or the register was never set).
    #[default]
    Unwritten,
    /// A value that does not depend on any texture.
    Constant(f32),
    /// `scale * (channel of unit's sample) + bias`.
    Texel {
        /// The texture unit sampled.
        unit: u8,
        /// Which channel of the sample.
        channel: u8,
        /// The factor the sample is multiplied by.
        scale: f32,
        /// The constant added after.
        bias: f32,
    },
    /// An expression of anything else (an interpolator, a dot product, a
    /// product of two samples).
    Other,
}

impl Lane {
    fn times(self, other: Self) -> Self {
        match (self, other) {
            (Self::Constant(a), Self::Constant(b)) => Self::Constant(a * b),
            (
                Self::Constant(k),
                Self::Texel {
                    unit,
                    channel,
                    scale,
                    bias,
                },
            )
            | (
                Self::Texel {
                    unit,
                    channel,
                    scale,
                    bias,
                },
                Self::Constant(k),
            ) => Self::Texel {
                unit,
                channel,
                scale: scale * k,
                bias: bias * k,
            },
            _ => Self::Other,
        }
    }

    fn plus(self, other: Self) -> Self {
        match (self, other) {
            (Self::Constant(a), Self::Constant(b)) => Self::Constant(a + b),
            (
                Self::Constant(k),
                Self::Texel {
                    unit,
                    channel,
                    scale,
                    bias,
                },
            )
            | (
                Self::Texel {
                    unit,
                    channel,
                    scale,
                    bias,
                },
                Self::Constant(k),
            ) => Self::Texel {
                unit,
                channel,
                scale,
                bias: bias + k,
            },
            _ => Self::Other,
        }
    }

    fn negated(self) -> Self {
        match self {
            Self::Constant(a) => Self::Constant(-a),
            Self::Texel {
                unit,
                channel,
                scale,
                bias,
            } => Self::Texel {
                unit,
                channel,
                scale: -scale,
                bias: -bias,
            },
            other => other,
        }
    }

    fn scaled(self, factor: f32) -> Self {
        self.times(Self::Constant(factor))
    }
}

/// The multiplier an instruction's output-scale code applies, `1.0` for none.
fn scale_factor(code: u8) -> f32 {
    match code {
        1 => 2.0,
        2 => 4.0,
        3 => 8.0,
        5 => 0.5,
        6 => 0.25,
        7 => 0.125,
        _ => 1.0,
    }
}

impl Program {
    /// The expression the program's output alpha (lane `w` of the register
    /// the end instruction writes) evaluates to.
    ///
    /// `parameter` resolves a patched constant's name hash to the material's
    /// value (`None` keeps the literal the file carries, which the engine
    /// would have overwritten). Registers are tracked as two files, half and
    /// full, as [`Program::output_texels`] does and for the same measured
    /// reason; straight-line code is assumed as everywhere in this module.
    /// A saturated write clamps a constant to `[0, 1]`; a texel expression
    /// is left unclamped and the caller clamps the sum it draws.
    #[must_use]
    pub fn output_alpha(&self, parameter: impl Fn(u32) -> Option<[f32; 4]>) -> Lane {
        let mut file = [[[Lane::Unwritten; 4]; 64]; 2];
        let mut output = (0usize, 0usize);
        for insn in &self.instructions {
            let dst = (usize::from(insn.dst_half), usize::from(insn.dst) & 63);
            let mut result = [Lane::Other; 4];
            if insn.is_texture() {
                for (i, lane) in result.iter_mut().enumerate() {
                    *lane = Lane::Texel {
                        unit: insn.unit,
                        channel: i as u8,
                        scale: 1.0,
                        bias: 0.0,
                    };
                }
            } else {
                let read = |slot: usize, lane: usize| -> Lane {
                    let pick = usize::from(insn.swizzles[slot][lane]);
                    let value = match insn.sources[slot] {
                        Source::Register { index, half } => {
                            file[usize::from(half)][usize::from(index) & 63][pick]
                        }
                        Source::Constant => insn.constant.map_or(Lane::Other, |literal| {
                            let patched = insn.const_slot.and_then(|slot| {
                                self.parameter_patches
                                    .iter()
                                    .find(|&&(at, _)| at == slot)
                                    .and_then(|&(_, hash)| parameter(hash))
                            });
                            Lane::Constant(patched.unwrap_or(literal)[pick])
                        }),
                        Source::Input | Source::Unknown => Lane::Other,
                    };
                    if insn.negate[slot] {
                        value.negated()
                    } else {
                        value
                    }
                };
                for (i, out) in result.iter_mut().enumerate() {
                    *out = match insn.name() {
                        Some("MOV") => read(0, i),
                        Some("MUL") => read(0, i).times(read(1, i)),
                        Some("ADD") => read(0, i).plus(read(1, i)),
                        Some("MAD") => read(0, i).times(read(1, i)).plus(read(2, i)),
                        _ => Lane::Other,
                    };
                    *out = out.scaled(scale_factor(insn.scale));
                    if insn.saturate
                        && let Lane::Constant(v) = *out
                    {
                        *out = Lane::Constant(v.clamp(0.0, 1.0));
                    }
                }
            }
            write_lanes(&mut file[dst.0][dst.1], insn, result);
            if insn.end {
                output = dst;
            }
        }
        file[output.0][output.1][3]
    }
}

fn write_lanes(register: &mut [Lane; 4], insn: &Instruction, result: [Lane; 4]) {
    for (i, slot) in register.iter_mut().enumerate() {
        if insn.mask & (1 << i) != 0 {
            *slot = result[i];
        }
    }
}

#[cfg(test)]
mod tests;
