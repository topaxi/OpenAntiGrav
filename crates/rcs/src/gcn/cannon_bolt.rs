//! Omega's `hd_detonator_cannonbolt_halo` pixel program: the Detonator cannon's bolt.
//!
//! HD's arithmetic (`docs/rendering/hd-unlit-programs.md`, "The Detonator cannon bolt") with
//! the two pictures named by sampler: `LightningTexture` twice (the alpha noise, then the
//! scrolled colour), `Texture1` once (the ring strip), clock rate `5`, noise displacement `0.8`,
//! alpha gain `5`. The tint of the rim's `(1 - F)^3` is Omega's own (`v_madmk_f32` on red and
//! green; the blue lane adds `(1 - F)^3` with no literal, so `1`). The variants that declare no
//! `fresnelBase` (the glow-mask and depth passes) are other passes and are not read.
//! `docs/rendering/hd-unlit-programs.md`, "The Detonator cannon bolt on 2048 and Omega".

use super::isa::{Encoding, Operand};
use super::{Program, Stage};

/// The picture of the noise and colour taps.
pub const LIGHTNING_TEXTURE: &str = "LightningTextureTexture";
/// The ring strip.
pub const RING_TEXTURE: &str = "Texture1Texture";

/// The clock's rate, the noise's displacement and the alpha's gain, which the shader holds.
const TIME_RATE: f32 = 5.0;
const NOISE_SHIFT: f32 = 0.8;
const ALPHA_GAIN: f32 = 5.0;

impl Program {
    /// The rim's tint, for a pixel program with the bolt's shape; `None` for any other.
    #[must_use]
    pub fn cannon_bolt(&self) -> Option<[f32; 3]> {
        if self.stage != Stage::Pixel {
            return None;
        }
        let reflection = self.reflection.as_ref().ok()?;
        for name in ["time", "fresnelBase"] {
            reflection
                .constants
                .iter()
                .any(|c| c.name == name)
                .then_some(())?;
        }
        let name_of = |slot: u8| {
            reflection
                .resources
                .iter()
                .find(|r| r.size == 0 && r.slot == u32::from(slot))
                .map(|r| r.name.as_str())
        };
        let mut taps = [0usize; 2];
        for (k, sample) in self.instructions.iter().enumerate() {
            let is_sample = sample.encoding == Encoding::Mimg
                && sample.name.is_some_and(|name| name.contains("sample"));
            let (true, Some(Operand::Sgpr { first: t, .. })) = (is_sample, sample.src[1]) else {
                continue;
            };
            match self.texture_slot(k, t).and_then(name_of) {
                Some(LIGHTNING_TEXTURE) => taps[0] += 1,
                Some(RING_TEXTURE) => taps[1] += 1,
                _ => return None,
            }
        }
        (taps == [2, 1]).then_some(())?;
        let literal = |i: &super::Instruction| i.literal().map(f32::from_bits);
        let (mut clock, mut noise, mut tint) = (0, 0, Vec::new());
        for i in &self.instructions {
            if i.name != Some("v_madmk_f32") {
                continue;
            }
            match literal(i)? {
                v if v == TIME_RATE => clock += 1,
                v if v == NOISE_SHIFT => noise += 1,
                v => tint.push(v),
            }
        }
        let gain = self
            .instructions
            .iter()
            .any(|i| i.name == Some("v_mul_f32") && literal(i) == Some(ALPHA_GAIN));
        (clock == 2 && noise == 2 && gain).then_some(())?;
        let &[red, green] = tint.as_slice() else {
            return None;
        };
        Some([red, green, 1.0])
    }
}

/// The tint of a `.rcsmaterial` file, where every pixel program that declares `fresnelBase` has
/// the bolt's shape and they agree; `None` for any other file.
#[must_use]
pub fn cannon_bolt(file: &[u8]) -> Option<[f32; 3]> {
    let mut found: Option<[f32; 3]> = None;
    for (_, program) in super::scan(file).iter() {
        let Ok(program) = program else { continue };
        let lit = program.stage == Stage::Pixel
            && program
                .reflection
                .as_ref()
                .is_ok_and(|r| r.constants.iter().any(|c| c.name == "fresnelBase"));
        if !lit {
            continue;
        }
        let read = program.cannon_bolt()?;
        if found.is_some_and(|f| f != read) {
            return None;
        }
        found = Some(read);
    }
    found
}
