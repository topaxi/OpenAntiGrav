//! The Vita's reading of the shield shell's pixel program, whose PS4 twin
//! [`crate::gcn::lightning_rates`] reads by data flow.
//!
//! The Vita's USSE operands are only listed in Python (`scripts/vita-usse.py`), so this reads
//! the literal records at the word indices one program was disassembled to use: the unlit
//! fragment program of `2048_shipshield_old` (`hd_shipshield_old`'s is the same instruction
//! for instruction), `docs/ghidra/functions/vita-2048-eu-v104/shield.md`, "The lattice
//! scrolls". Word 10 is `B`, word 11 the `v` rate before `B` scales it and word 12 `A`. The
//! lit variants carry the same three values at other indices, so only the unlit programs
//! (no lighting uniform) are read.

use super::{Program, programs};
use crate::gcn::LightningRates;

const NOISE_SCALE: u32 = 10;
const NOISE_DRIFT: u32 = 11;
const DIFFUSE_SCROLL: u32 = 12;

fn is_unlit_shield(program: &Program) -> bool {
    program.is_fragment()
        && program.parameter("LightningTexture").is_some()
        && program.parameter("time").is_some()
        && program.parameter("liveLighting0diffuse").is_none()
        && program.parameter("ibl").is_none()
}

fn rates_of(program: &Program) -> Option<LightningRates> {
    let literal = |word: u32| {
        program
            .literals
            .iter()
            .find(|&&(at, _)| at == word)
            .map(|&(_, bits)| f32::from_bits(bits))
    };
    let noise_scale = literal(NOISE_SCALE)?;
    Some(LightningRates {
        diffuse_scroll: literal(DIFFUSE_SCROLL)?,
        noise_scale,
        // The program scales `v + drift * time` by `B`, the PS4's the other way round.
        noise_rate: literal(NOISE_DRIFT)? * noise_scale,
    })
}

/// The rates of a `.rcsmaterial` file whose unlit shield programs all agree; `None` for a
/// file with none, with a literal missing, or whose programs differ.
#[must_use]
pub fn lightning_rates(file: &[u8]) -> Option<LightningRates> {
    let mut found: Option<LightningRates> = None;
    for (_, program) in programs(file) {
        let Ok(program) = program else {
            continue;
        };
        if !is_unlit_shield(&program) {
            continue;
        }
        let rates = rates_of(&program)?;
        if found.is_some_and(|seen| seen != rates) {
            return None;
        }
        found = Some(rates);
    }
    found
}
