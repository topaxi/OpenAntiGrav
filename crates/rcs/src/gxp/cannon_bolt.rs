//! 2048's `hd_detonator_cannonbolt_halo` fragment programs: the Detonator cannon's bolt.
//!
//! HD's arithmetic (`docs/rendering/hd-unlit-programs.md`, "The Detonator cannon bolt") with
//! two differences read off the code. The noise tap is prefetched (the fragment input records
//! name it as a texel of `LightningTexture` at interpolant 2, `Uv1_Uv2.zw`), the ring is
//! prefetched as a texel of `Texture1` at interpolant 1 (`.xy`), and the one `smp` is the colour
//! tap. And the program squares the colour tap itself (`mad.f16 pa0, pa6, pa6, pa0`, the texel
//! register read twice). The literals are the tint's half pair, `1`, the gain `5`, the noise
//! displacement `0.8`, the clock rate and exponent `5` and `1`.
//! `docs/rendering/hd-unlit-programs.md`, "The Detonator cannon bolt on 2048 and Omega".

use super::operands::{Bank, Tables, Word, field, sample, src0_bank, src12_bank, tables};
use super::{Category, Program, usse};

/// The picture of the noise and colour taps.
pub const LIGHTNING: &str = "LightningTexture";
/// The ring strip.
pub const RING: &str = "Texture1";
/// The interpolants of `Uv1_Uv2.zw` and `.xy`.
const NOISE_INTERPOLANT: u32 = 2;
const RING_INTERPOLANT: u32 = 1;

/// An IEEE half as `f32`, for the normal values a literal holds.
fn half(bits: u16) -> f32 {
    let exponent = u32::from(bits >> 10) & 0x1f;
    let mantissa = u32::from(bits) & 0x3ff;
    if exponent == 0 {
        return 0.0;
    }
    f32::from_bits((u32::from(bits >> 15) << 31) | ((exponent + 112) << 23) | (mantissa << 13))
}

/// The rim's tint, when the literal records hold exactly the bolt's.
fn literals(t: &Tables) -> Option<[f32; 3]> {
    let mut floats = Vec::new();
    let mut pairs = Vec::new();
    for &(_, bits) in &t.literals {
        let value = f32::from_bits(bits);
        if value == 0.8 || value == 5.0 || value == 1.0 {
            floats.push(value);
        } else {
            pairs.push((half(bits as u16), half((bits >> 16) as u16)));
        }
    }
    floats.sort_by(f32::total_cmp);
    (floats == [0.8, 1.0, 5.0]).then_some(())?;
    // Two `(1, 0)` halves (the tint's blue and the shift of the rim), one `(5, 0)` (the alpha
    // gain), and the tint's red and green.
    let ones = pairs.iter().filter(|&&p| p == (1.0, 0.0)).count();
    let gains = pairs.iter().filter(|&&p| p == (5.0, 0.0)).count();
    let rest: Vec<_> = pairs
        .iter()
        .filter(|&&p| p != (1.0, 0.0) && p != (5.0, 0.0))
        .collect();
    let &[&(red, green)] = rest.as_slice() else {
        return None;
    };
    (ones == 2 && gains == 1).then_some([red, green, 1.0])
}

/// Whether `word` is a half-float `mad` whose first two sources are the same `xyzw` lanes of
/// primary register `register`: its square, plus the third source.
fn squares(word: u64, register: u32) -> bool {
    const XYZW: u32 = 4;
    usse::classify(word) == Some(usse::Class::Mad2)
        && field(word, 58, 1) == 1
        // An f16 write to a register bank covers halves: bit 0 the pair `xy`, bit 2 `zw`.
        && field(word, 39, 4) == 0b0101
        && src0_bank(field(word, 34, 1)) == Bank::Primary
        && src12_bank(field(word, 30, 2), field(word, 49, 1)) == Bank::Primary
        && field(word, 12, 6) * 2 == register
        && field(word, 6, 6) * 2 == register
        && field(word, 18, 2) | (field(word, 53, 1) << 2) == XYZW
        && field(word, 20, 2) | (field(word, 44, 1) << 2) == XYZW
}

/// The rim's tint of one fragment program (`bytes` its own bytes, `p` it parsed), when it has
/// the bolt's interface and shape; `None` for any other.
#[must_use]
pub fn program(bytes: &[u8], p: &Program) -> Option<[f32; 3]> {
    let resource = |category: Category, name: &str| {
        p.parameters
            .iter()
            .find(|q| q.category == category && q.name == name)
            .map(|q| q.resource)
    };
    resource(Category::Uniform, "time")?;
    resource(Category::Uniform, "fresnelBase")?;
    let lightning = resource(Category::Sampler, LIGHTNING)?;
    let ring = resource(Category::Sampler, RING)?;
    let t = tables(bytes)?;
    let prefetched = |resource: u32, interpolant: u32| {
        t.texels
            .iter()
            .any(|x| x.resource == resource && x.interpolant == interpolant)
    };
    (prefetched(lightning, NOISE_INTERPOLANT) && prefetched(ring, RING_INTERPOLANT))
        .then_some(())?;
    let code: Vec<u64> = usse::words(bytes, &p.primary).collect();
    let taps: Vec<(Word, Word, u32)> = code.iter().filter_map(|&w| sample(w)).collect();
    let &[(Word(Bank::Primary, register), _, state)] = taps.as_slice() else {
        return None;
    };
    t.samplers
        .iter()
        .any(|&(resource, sa)| resource == lightning && t.base + sa == state)
        .then_some(())?;
    code.iter().any(|&w| squares(w, register)).then_some(())?;
    literals(&t)
}

/// The tint of a `.rcsmaterial` file, where every fragment program that declares `fresnelBase`
/// has the bolt's shape and they agree; `None` for any other file.
#[must_use]
pub fn cannon_bolt(data: &[u8]) -> Option<[f32; 3]> {
    let mut found: Option<[f32; 3]> = None;
    for (at, parsed) in super::programs(data) {
        let Ok(p) = parsed else { continue };
        let lit = p.is_fragment()
            && p.parameters
                .iter()
                .any(|q| q.category == Category::Uniform && q.name == "fresnelBase");
        if !lit {
            continue;
        }
        let read = program(&data[at..at + p.size], &p)?;
        if found.is_some_and(|f| f != read) {
            return None;
        }
        found = Some(read);
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_colour_tap_is_read_and_squared() {
        // `smp2d.f16.f32 pa6.x, pa4.xy, sa6`, then `mad.f16 pa0.xyzw, pa6.xyzw, pa6.xyzw,
        // pa0.xyzw`, off `data2.psarc`'s program #1.
        assert_eq!(
            sample(0xe041_8484_e0c0_8180).map(|(dst, _, state)| (dst, state)),
            Some((Word(Bank::Primary, 6), 6))
        );
        assert!(squares(0x04a0_9286_a000_30c0, 6));
        assert!(!squares(0x04a0_9286_a000_30c0, 4));
        // `mad.f16 pa0.xyzw, pa4.xxxx, pa0.xyzw, sa2.xyzw` squares nothing.
        assert!(!squares(0x0480_9286_b000_2001, 4));
    }

    #[test]
    fn the_tint_halves_decode() {
        assert!((half(0x2bed) - 0.061_92).abs() < 1.0e-5);
        assert!((half(0x39f5) - 0.744_6).abs() < 1.0e-3);
        assert_eq!(half(0x3c00), 1.0);
        assert_eq!(half(0x4500), 5.0);
        assert_eq!(half(0), 0.0);
    }
}
