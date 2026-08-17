//! A `Weapon Pad`'s ready-to-collect colour cycle.
//!
//! Recovered from `WeaponPad_UpdateRefreshTimer` (`0x0892c034`,
//! `docs/ghidra/functions/psp-pulse-usa/pads.md`): while a pad is cooling
//! down after a hit it packs a flat grey into `pad+0x6c`, and once the timer
//! reaches zero it instead cross-fades `pad+0x6c` through a 6-entry colour
//! keyframe table at `1/3`-second steps. `pad+0x6c` is a full colour
//! *replacement*, not a modulation of the mesh's own baked vertex colour -
//! `Pad_Bind` calls `Mesh_Bind` first, which already builds the GE colour
//! commands from the file's own material colours, and the timer overwrites
//! that slot every tick regardless.
//!
//! # The table's address, and why it is trusted despite carrying no xref
//!
//! `WeaponPad_UpdateRefreshTimer` builds the table pointer from a bare
//! `lui`/`addiu` pair (`0x0892c0b0`/`0x0892c0b8`, immediate `0x002bc0c8`)
//! rather than a relocation Ghidra's loader resolved, so
//! `get_xrefs_to`/`get_xrefs_from` find nothing at either end - this
//! reference simply was not relocated when the ELF was imported. Adding the
//! image base (`0x08804000`, this program's own `min_address`) predicts
//! `0x08ac00c8`, inside `.data` (`08ab0600`-`08ad9717`); `read_memory` there
//! returns 72 bytes of clean, plausible `0..255`-range floats, and
//! `search_byte_patterns` for the first entry's exact 12 bytes
//! (`00 00 4c 43 00 00 cc 42 00 00 cc 42`) finds **exactly one** match in the
//! whole 70 MB image, at that address. An unrelated coincidence landing on a
//! predicted address, inside the right section, matching a hand-derived
//! stride and entry count, and unique across the image, is not a plausible
//! alternative reading. Confidence **85**: exact agreement with shipped data
//! and independent structural prediction, capped below what a live trace
//! would give because the relocation itself is inferred rather than read.
//!
//! # What is reproduced, and what is not
//!
//! The palette and the per-key duration are both real, recovered numbers.
//! What is **not** reproduced is *phase continuity*: the original only
//! advances a pad's own `pad+500` counter while that pad is ready, so a pad
//! resumes its cross-fade from wherever it left off the last time it was
//! collectable, and two pads that became ready at different moments are out
//! of phase with each other. This module has no per-pad state to resume
//! from - see `Drawable::tint_weapon_pads` - and samples every ready pad at
//! the same race clock instead, which is simpler and still cross-fades at
//! the right rate, but keeps every ready pad on the track in lockstep rather
//! than staggered. Worth revisiting if a per-pad timer is ever threaded
//! through render state for another reason.

/// `pad+0x6c` while cooling down: a flat `0x3f3f3f` grey, read directly off
/// the packed `0xff3f3f3f` constant in `WeaponPad_UpdateRefreshTimer`.
pub const COOLDOWN_COLOUR: [f32; 3] = [
    0x3f as f32 / 255.0,
    0x3f as f32 / 255.0,
    0x3f as f32 / 255.0,
];

/// The 6-entry `[r, g, b]` keyframe table at `0x08ac00c8`, `0..255` per
/// channel as authored - a rainbow cycle rather than anything track-themed.
pub const READY_KEYFRAMES: [[f32; 3]; 6] = [
    [204.0, 102.0, 102.0],
    [121.0, 210.0, 121.0],
    [121.0, 121.0, 210.0],
    [210.0, 210.0, 121.0],
    [140.0, 216.0, 216.0],
    [216.0, 140.0, 216.0],
];

/// Keys per second - `param+500` ramps at `dt * 3` in the original, so one
/// key every third of a second and a full 6-key cycle every 2 seconds.
pub const READY_KEYS_PER_SECOND: f32 = 3.0;

/// The ready-state colour at `seconds`, linearly cross-faded between
/// [`READY_KEYFRAMES`] the way `WeaponPad_UpdateRefreshTimer` interpolates
/// `pad+0x1f0`'s pair, wrapping the 6-entry table continuously.
///
/// `seconds` is not clamped to when a pad became ready - see this module's
/// own doc comment for what that costs.
#[must_use]
pub fn ready_colour(seconds: f32) -> [f32; 3] {
    let keys = READY_KEYFRAMES.len();
    let t = (seconds * READY_KEYS_PER_SECOND).rem_euclid(keys as f32);
    let i = t as usize % keys;
    let j = (i + 1) % keys;
    let frac = t - t.floor();
    let a = READY_KEYFRAMES[i];
    let b = READY_KEYFRAMES[j];
    [
        (a[0] * (1.0 - frac) + b[0] * frac) / 255.0,
        (a[1] * (1.0 - frac) + b[1] * frac) / 255.0,
        (a[2] * (1.0 - frac) + b[2] * frac) / 255.0,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_whole_key_lands_exactly_on_its_own_keyframe() {
        for (i, key) in READY_KEYFRAMES.iter().enumerate() {
            let seconds = i as f32 / READY_KEYS_PER_SECOND;
            let got = ready_colour(seconds);
            for c in 0..3 {
                assert!(
                    (got[c] - key[c] / 255.0).abs() < 1e-5,
                    "key {i} channel {c}: got {got:?}, wanted {key:?}"
                );
            }
        }
    }

    #[test]
    fn the_cycle_wraps_from_the_last_key_back_to_the_first() {
        let period = READY_KEYFRAMES.len() as f32 / READY_KEYS_PER_SECOND;
        let at_zero = ready_colour(0.0);
        let one_cycle_later = ready_colour(period);
        assert_eq!(at_zero, one_cycle_later);
    }

    #[test]
    fn every_channel_stays_in_the_unit_range() {
        let mut t = 0.0;
        while t < 10.0 {
            for c in ready_colour(t) {
                assert!((0.0..=1.0).contains(&c), "t={t} produced {c}");
            }
            t += 0.037;
        }
    }
}
