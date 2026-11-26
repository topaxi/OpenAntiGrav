//! The colour-table laws against the worked values of `docs/ghidra/functions/ps3-hdfury-eu/ship-destroy.md`
//! ("`colour_mode` 3 is a blend toward the scene light"): HD's `WO_SHIP_EXPLOSION` root table.

use super::*;
use crate::psys::tests::effect;

/// The root's table: entry 0 white, entries 1 to 31 black (red 0), 32 to 40 a grey ramp whose
/// red is `0.10 .. 0.90`, then white.
fn hd_root_table() -> Box<[[f32; 4]; 256]> {
    let mut table = Box::new([[1.0; 4]; 256]);
    for entry in &mut table[1..32] {
        *entry = [0.0, 0.0, 0.0, 1.0];
    }
    for (i, entry) in table[32..41].iter_mut().enumerate() {
        let grey = (26 + 26 * i) as f32 / 255.0;
        *entry = [grey, grey, grey, 1.0];
    }
    table
}

const L: [f32; 3] = [51.0 / 255.0, 102.0 / 255.0, 153.0 / 255.0];

#[test]
fn light_blend_at_life_zero_is_the_light() {
    // Index 0 weighs entry 0 by its own red, 1.0.
    assert_eq!(
        light_blend(&hd_root_table(), 0.0, L),
        [L[0], L[1], L[2], 1.0]
    );
}

#[test]
fn light_blend_at_an_eighth_is_entry_zero_not_the_blacks() {
    // `trunc(0.125 * 255.99997) = 31`: red 0, so weight 0, so entry 0, white.
    assert_eq!(light_blend(&hd_root_table(), 0.125, L), [1.0; 4]);
}

#[test]
fn light_blend_at_half_is_the_light() {
    // Index 127: red 1.0.
    assert_eq!(
        light_blend(&hd_root_table(), 0.5, L),
        [L[0], L[1], L[2], 1.0]
    );
}

#[test]
fn light_blend_truncates_a_partial_weight() {
    // Index 36 has red 130; white toward (50, 100, 150): 255 + (50 - 255) * 130/255 = 150.49,
    // 255 + (100 - 255) * 130/255 = 175.98, 255 + (150 - 255) * 130/255 = 201.47. Truncated
    // 150, 175, 201; rounding would give 150, 176, 201.
    let light = [50.0 / 255.0, 100.0 / 255.0, 150.0 / 255.0];
    let out = light_blend(&hd_root_table(), 36.0 / 255.99997 + 1e-4, light);
    assert_eq!(out[..3], [150.0 / 255.0, 175.0 / 255.0, 201.0 / 255.0]);
}

#[test]
fn the_reversed_walk_reads_the_table_from_its_end() {
    let table = hd_root_table();
    // Age 0 is entry 255 (white), age 0.9 entry 25 (black), the last eighth is black.
    assert_eq!(ColourMode::ReverseLife.colour(&table, 0.0, 0), table[255]);
    assert_eq!(ColourMode::ReverseLife.colour(&table, 0.9, 0), table[25]);
    assert_eq!(
        ColourMode::ReverseLife.colour(&table, 0.9, 0),
        [0.0, 0.0, 0.0, 1.0]
    );
    assert_eq!(ColourMode::OverLife.colour(&table, 0.9, 0), table[230]);
}

fn with_word(word: u32, law: ColourLaw) -> ColourMode {
    let mut effect = (*effect("x", true, 10.0)).clone();
    let spec = &mut effect.emitters[0];
    spec.colour_mode = ColourMode::authored(word);
    spec.authored_colour_mode = word;
    effect.with_colour_law(law);
    effect.emitters[0].colour_mode
}

#[test]
fn a_laws_words() {
    use ColourLaw::{Forward, LightBlend, Reversed};
    for (law, word, want) in [
        (Forward, 3, ColourMode::OverLife),
        (Reversed, 0, ColourMode::OverLife),
        (Reversed, 1, ColourMode::ReverseLife),
        (Reversed, 3, ColourMode::ReverseLife),
        (Reversed, 2, ColourMode::RandomEntry),
        (LightBlend, 3, ColourMode::LightBlend),
        (LightBlend, 1, ColourMode::OverLife),
        (LightBlend, 0, ColourMode::OverLife),
        (LightBlend, 2, ColourMode::RandomEntry),
    ] {
        assert_eq!(with_word(word, law), want, "{law:?} word {word}");
    }
}

#[test]
fn a_law_applied_again_replaces_the_first() {
    let mut effect = (*effect("x", true, 10.0)).clone();
    effect.emitters[0].colour_mode = ColourMode::OverLife;
    effect.emitters[0].authored_colour_mode = 3;
    effect.with_colour_law(ColourLaw::LightBlend);
    assert_eq!(effect.emitters[0].colour_mode, ColourMode::LightBlend);
    effect.with_colour_law(ColourLaw::Forward);
    assert_eq!(effect.emitters[0].colour_mode, ColourMode::OverLife);
}

/// `DetonatorMine_Explode`'s entry for Detonator stage 1, `Sub Venom`, mine colour
/// `122 25 255 255`: `0.8` of the way to white is `228, 209, 255`, and the packing's shifts of
/// `9` and `1` move green and red one bit up, so red keeps its low seven bits, green takes red's
/// top bit and blue green's (`docs/ghidra/functions/ps3-hdfury-eu/detonator-mine.md`, "The
/// blast's colour").
#[test]
fn a_live_mine_blast_entry_is_the_executables_packing_not_a_plain_lerp() {
    assert_eq!(
        detonator_mine_blast_entry([122, 25, 255, 255]),
        [200, 163, 255, 255]
    );
    // Stage 2, `Venom`, `0 142 224`: `204, 232, 248`.
    assert_eq!(
        detonator_mine_blast_entry([0, 142, 224, 255]),
        [152, 209, 249, 255]
    );
    // The authored alpha byte rides through; it is never drawn.
    assert_eq!(detonator_mine_blast_entry([255, 255, 255, 0])[3], 0);
}

#[test]
fn one_colour_fills_every_emitters_whole_table() {
    let mut effect = (*effect("x", true, 10.0)).clone();
    effect.emitters.push(effect.emitters[0].clone());
    effect.with_one_colour([51, 102, 153, 255]);
    for spec in &effect.emitters {
        assert!(
            spec.palette
                .iter()
                .all(|entry| *entry == [0.2, 0.4, 0.6, 1.0]),
            "every entry of {} is the one colour",
            spec.name
        );
    }
}
