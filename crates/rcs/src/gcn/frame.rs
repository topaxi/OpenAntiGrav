//! Which vector a lit pixel program weights by its normal map's `z` lane.
//!
//! A program that perturbs the surface normal with a tangent-space map builds
//! `N = x * A + y * B + z * C` from three interpolants and the map's `2t - 1` lanes. The usual
//! frame is `A = T`, `B = cross(N0, T) * w`, `C = N0`. Omega's crowd sprites weight `z` by the
//! interpolant that the vertex program fills with `eyePositionWorldSpace - position`
//! (`docs/formats/omega-status.md`, "The crowd sprite's frame"): the normal that the program
//! lights is the figure turned to the camera, not the card's own normal.
//!
//! [`third_axis_is_eye`] reads it off the file: the pixel program's `z` consumer names an
//! interpolant; the interpolant's slot names the vertex program's output; the output's export
//! is `eye - position` when every lane is a `v_sub_f32` of an `eyePositionWorldSpace`
//! component. Nothing is assumed about which export is the normal.
//!
//! **The shadow-receiving permutations are left out.** A file carries a program per light
//! variant. The ones that read `shadowLightDirection` (the track's shadow map, `novaScaleBiasPower`)
//! weight `z` by another interpolant; they are what the engine picks for a surface that
//! receives the circuit's shadow, and the crowd draws, whose shadow is the `Shadowed` constant
//! of the `_shadow` materials, are not read to.

use super::isa::{Detail, Instruction, Operand};
use super::{Program, Stage};

/// What [`third_axis_is_eye`] counted in one material file.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct EyeFrame {
    /// Vertex and pixel program pairs (the pixel program directly after its vertex program,
    /// not reading `shadowLightDirection`) whose normal map's `z` lane the pixel program
    /// weights by a named interpolant.
    pub pairs: usize,
    /// Of those, pairs where that interpolant is the vertex program's `eye - position`.
    pub eye: usize,
}

impl EyeFrame {
    /// At least one pair was read and every pair read takes the eye vector.
    #[must_use]
    pub fn is_eye(&self) -> bool {
        self.pairs > 0 && self.eye == self.pairs
    }
}

/// The constant `eye - position` is built from.
const EYE: &str = "eyePositionWorldSpace";

/// The constant a shadow-receiving permutation reads.
const SHADOW_LIGHT: &str = "shadowLightDirection";

impl Program {
    /// The pixel interpolant (`attrN`, `N` the index into the program's input list) that the
    /// `z` lane of a `2t - 1` decoded sample is multiplied by, when one multiply names it.
    #[must_use]
    pub fn normal_z_attribute(&self) -> Option<u8> {
        for (k, sample) in self.instructions.iter().enumerate() {
            let is_sample = sample.encoding == super::isa::Encoding::Mimg
                && sample.name.is_some_and(|n| n.contains("sample"));
            let Some(Operand::Vgpr { first, count }) = sample.dst[0] else {
                continue;
            };
            if !is_sample || count < 3 {
                continue;
            }
            let z = first + 2;
            let Some(decoded) = self.instructions[k + 1..].iter().position(|i| {
                i.name == Some("v_mad_f32")
                    && i.src[0] == Some(Operand::Float(2.0))
                    && i.src[2] == Some(Operand::Float(-1.0))
                    && matches!(i.src[1], Some(Operand::Vgpr { first: f, count: 1 }) if f == z)
            }) else {
                continue;
            };
            let decode = k + 1 + decoded;
            let Some(Operand::Vgpr { first: lane, .. }) = self.instructions[decode].dst[0] else {
                continue;
            };
            return self.weighted_attribute(decode + 1, lane);
        }
        None
    }

    /// The interpolant multiplied by vector register `lane` in the first `mul`, `mac` or `mad`
    /// after instruction `from` that reads it, while `lane` still holds the decoded value.
    fn weighted_attribute(&self, from: usize, lane: u8) -> Option<u8> {
        for (k, i) in self.instructions.iter().enumerate().skip(from) {
            let name = i.name.unwrap_or("");
            let product = ["mul", "mac", "mad"].iter().any(|p| name.contains(p));
            if product {
                let other = match (i.src[0], i.src[1]) {
                    (Some(Operand::Vgpr { first: a, count: 1 }), Some(b)) if a == lane => Some(b),
                    (Some(a), Some(Operand::Vgpr { first: b, count: 1 })) if b == lane => Some(a),
                    _ => None,
                };
                if let Some(Operand::Vgpr { first, count: 1 }) = other
                    && let Some((_, writer)) = self.writer_of(k, first)
                    && let Detail::Interp { attr, .. } = writer.detail
                {
                    return Some(attr);
                }
            }
            if matches!(i.dst[0], Some(Operand::Vgpr { first, count }) if (first..first + count).contains(&lane))
            {
                return None;
            }
        }
        None
    }

    /// Whether the vertex output in `slot` is exported as `eyePositionWorldSpace - position`:
    /// each of its three lanes is a `v_sub_f32` whose first source is a component of that
    /// constant.
    #[must_use]
    pub fn output_is_eye_vector(&self, slot: u8) -> bool {
        let Ok(reflection) = &self.reflection else {
            return false;
        };
        let Some(output) = reflection
            .outputs
            .iter()
            .find(|a| a.slot() == slot && a.semantic == "TEXCOORD")
        else {
            return false;
        };
        let Some(target) = output
            .name
            .strip_prefix("__atg_param")
            .and_then(|n| n.parse::<u8>().ok())
        else {
            return false;
        };
        let Some((k, export)) = self.instructions.iter().enumerate().find(
            |(_, i)| matches!(i.detail, Detail::Export { target: t, .. } if t == 32 + target),
        ) else {
            return false;
        };
        let lanes: Vec<u8> = export
            .src
            .iter()
            .flatten()
            .filter_map(|s| match *s {
                Operand::Vgpr { first, count: 1 } => Some(first),
                _ => None,
            })
            .take(3)
            .collect();
        lanes.len() == 3
            && lanes.iter().all(|&lane| {
                self.writer_of(k, lane)
                    .is_some_and(|(at, w)| self.subtracts_eye(at, w))
            })
    }

    fn subtracts_eye(&self, at: usize, writer: &Instruction) -> bool {
        writer.name == Some("v_sub_f32")
            && matches!(writer.src[0], Some(Operand::Sgpr { first, .. })
                if self.constant_loaded(at, first) == Some(EYE))
    }

    /// The constant scalar register `sgpr` holds at instruction `k`, for a vertex program that
    /// may read two constant buffers. The header's usage records of a vertex program are
    /// unread, so a load's buffer is told from its descriptor register: the distinct descriptor
    /// registers the program loads through, ascending, are its constant buffers by API slot,
    /// ascending (the patch's compile splits `world` and `ViewDep`; confidence 70, and refused
    /// when the counts differ).
    fn constant_loaded(&self, k: usize, sgpr: u8) -> Option<&str> {
        let reflection = self.reflection.as_ref().ok()?;
        let (first, load) = self.instructions[..k]
            .iter()
            .rev()
            .find_map(|i| match i.dst[0] {
                Some(Operand::Sgpr { first, count }) if (first..first + count).contains(&sgpr) => {
                    Some((first, i))
                }
                _ => None,
            })?;
        if !load.name.is_some_and(|n| n.starts_with("s_buffer_load")) {
            return None;
        }
        let (Some(Operand::Sgpr { first: base, .. }), Some(Operand::Imm(dword))) =
            (load.src[0], load.src[1])
        else {
            return None;
        };
        let mut bases: Vec<u8> = self
            .instructions
            .iter()
            .filter(|i| i.name.is_some_and(|n| n.starts_with("s_buffer_load")))
            .filter_map(|i| match i.src[0] {
                Some(Operand::Sgpr { first, .. }) => Some(first),
                _ => None,
            })
            .collect();
        bases.sort_unstable();
        bases.dedup();
        let mut buffers: Vec<(u32, usize)> = reflection
            .resources
            .iter()
            .enumerate()
            .filter(|(_, r)| r.size > 0)
            .map(|(k, r)| (r.slot, k))
            .collect();
        buffers.sort_unstable();
        if bases.len() != buffers.len() {
            return None;
        }
        let (_, buffer) = buffers[bases.iter().position(|&b| b == base)?];
        reflection
            .constant_at(buffer, dword + u32::from(sgpr - first))
            .map(|(constant, _)| constant.name.as_str())
    }
}

/// Reads a `.rcsmaterial` file's lit programs for the vector their normal map's `z` lane
/// weights. A file lists each vertex program directly before the pixel program it feeds.
#[must_use]
pub fn third_axis_is_eye(file: &[u8]) -> EyeFrame {
    let mut found = EyeFrame::default();
    let mut vertex: Option<&Program> = None;
    for (_, program) in super::scan(file).iter() {
        let Ok(program) = program else {
            vertex = None;
            continue;
        };
        match program.stage {
            Stage::Vertex => vertex = Some(program),
            Stage::Pixel => {
                let Some(vertex) = vertex.take() else {
                    continue;
                };
                if program.names_constant(SHADOW_LIGHT) {
                    continue;
                }
                let Some(attr) = program.normal_z_attribute() else {
                    continue;
                };
                let Some(input) = program
                    .reflection
                    .as_ref()
                    .ok()
                    .and_then(|r| r.inputs.get(usize::from(attr)))
                else {
                    continue;
                };
                found.pairs += 1;
                found.eye += usize::from(vertex.output_is_eye_vector(input.slot()));
            }
            Stage::Other(_) => {}
        }
    }
    found
}
