//! Omega's `electricity.rcsmaterial` pixel program: the Detonator mine's visualiser ring.
//!
//! The program is HD's (`docs/ghidra/functions/ps3-hdfury-eu/detonator-mine-draw.md`, "The
//! visualiser, wired") with the three pictures named by sampler instead of by unit: two taps of
//! `Bass_Treble_HitsTexture`, one of `PitchTexture` and two of `ElectricityTexture`, the last
//! of which reads its second coordinate at `uv1.y + 0.3 m`. The literal is read off the code
//! by data flow (`v_madmk_f32` whose result addresses the colour tap), never typed by a caller.
//! `docs/ghidra/functions/ps4-omega-eu/detonator-hull.md`, "The visualiser".

use super::isa::{Encoding, Operand};
use super::{Program, Stage};

/// The picture of the distortion and colour taps (HD's unit 2).
pub const ELECTRICITY_TEXTURE: &str = "ElectricityTexture";
/// The pitch picture (HD's unit 1).
pub const PITCH_TEXTURE: &str = "PitchTexture";
/// The bass and treble picture (HD's unit 0).
pub const BASS_TREBLE_TEXTURE: &str = "Bass_Treble_HitsTexture";

/// What the program's code says beyond the names it declares.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ElectricityRead {
    /// How far the colour tap's `v` moves per unit of the music's distortion `m`
    /// (`v_madmk_f32 .., m, K, uv1.y`).
    pub colour_tap_shift: f32,
}

impl Program {
    /// The reading, for a pixel program that has this one's shape: exactly two taps of the bass
    /// and treble picture, one of the pitch picture and two of the electricity picture, with
    /// the shift literal feeding the last electricity tap's coordinate. `None` for any other.
    #[must_use]
    pub fn electricity(&self) -> Option<ElectricityRead> {
        if self.stage != Stage::Pixel {
            return None;
        }
        let reflection = self.reflection.as_ref().ok()?;
        let name_of = |slot: u8| {
            reflection
                .resources
                .iter()
                .find(|r| r.size == 0 && r.slot == u32::from(slot))
                .and_then(|r| r.name.strip_suffix("Texture").map(|_| r.name.as_str()))
        };
        let mut taps = [0usize; 3];
        let mut shift = None;
        for (k, sample) in self.instructions.iter().enumerate() {
            let is_sample = sample.encoding == Encoding::Mimg
                && sample.name.is_some_and(|name| name.contains("sample"));
            let (true, Some(address), Some(Operand::Sgpr { first: t, .. })) =
                (is_sample, sample.src[0], sample.src[1])
            else {
                continue;
            };
            let Some(name) = self.texture_slot(k, t).and_then(name_of) else {
                continue;
            };
            let which = match name {
                BASS_TREBLE_TEXTURE => 0,
                PITCH_TEXTURE => 1,
                ELECTRICITY_TEXTURE => 2,
                _ => continue,
            };
            taps[which] += 1;
            if which != 2 {
                continue;
            }
            let Operand::Vgpr { first, .. } = address else {
                continue;
            };
            // The coordinate's second register is written by a `v_madmk_f32` on the colour tap
            // and by a `v_mul_f32` on the distortion tap.
            let y = first + 1;
            let writer = self.instructions[..k].iter().rev().find(|w| {
                matches!(w.dst[0], Some(Operand::Vgpr { first: f, count }) if f <= y && y < f + count)
            });
            if let Some(w) = writer.filter(|w| w.name == Some("v_madmk_f32")) {
                shift = w.literal().map(f32::from_bits);
            }
        }
        (taps == [2, 1, 2]).then_some(())?;
        Some(ElectricityRead {
            colour_tap_shift: shift?,
        })
    }
}

/// The reading of a `.rcsmaterial` file, where every pixel program that declares the electricity
/// picture has this one's shape and they agree; `None` for a file with none.
#[must_use]
pub fn electricity(file: &[u8]) -> Option<ElectricityRead> {
    let mut found: Option<ElectricityRead> = None;
    for (_, program) in super::scan(file).iter() {
        let Ok(program) = program else { continue };
        let declares = program.stage == Stage::Pixel
            && program
                .reflection
                .as_ref()
                .is_ok_and(|r| r.resources.iter().any(|r| r.name == ELECTRICITY_TEXTURE));
        if !declares {
            continue;
        }
        let read = program.electricity()?;
        if found.is_some_and(|f| f != read) {
            return None;
        }
        found = Some(read);
    }
    found
}
