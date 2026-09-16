//! What the note-to-pitch port is asserted to reproduce: the pitch words
//! captured live at `__sceSasSetPitch` on `psp-pulse-usa`, and the two tables'
//! closed forms.
//!
//! The live values are the discriminating evidence: three different pitch
//! words out of one bank (`frontend.bnk`) falsify "one rate per bank", and
//! every one of them is what the descriptor bytes predict.

use super::*;

/// `(centre, fine) -> pitch` pairs read at `__sceSasSetPitch` (`0x08a76c7c`)
/// in the front end, 40 of 40 hits, 2026-09-16. Each is a `frontend.bnk`
/// descriptor played at the default note.
const FRONT_END_HITS: [(i8, i8, u16); 3] = [(-89, 66, 0x35c), (-86, 66, 0x400), (-95, 66, 0x260)];

#[test]
fn the_front_end_s_three_pitch_words_come_out_of_their_descriptors() {
    for (centre, fine, pitch) in FRONT_END_HITS {
        assert_eq!(
            sas_pitch(centre, fine, DEFAULT_NOTE, 0),
            pitch,
            "({centre}, {fine})"
        );
    }
}

#[test]
fn the_standard_rates_are_exact_pitch_words() {
    // `hud.bnk`, `frontend.bnk` and `weapons.bnk` are full of these three.
    assert_eq!(sas_pitch(-86, 66, DEFAULT_NOTE, 0), 0x400);
    assert_eq!(sas_pitch(-74, 66, DEFAULT_NOTE, 0), 0x800);
    assert_eq!(sas_pitch(-62, 66, DEFAULT_NOTE, 0), 0x1000);
    assert_eq!(sample_rate_hz(0x400), 11_025);
    assert_eq!(sample_rate_hz(0x800), 22_050);
    assert_eq!(sample_rate_hz(0x1000), 44_100);
}

#[test]
fn a_centre_fine_of_zero_is_a_semitone_flat_before_the_scale() {
    // `(-60, 0)`: the walk lands on `2^(-1/12)` and the scale lifts it to
    // `0x116f`, 48,051 Hz - the disc's "48 kHz" waveforms.
    assert_eq!(note_to_pitch(60, 0, DEFAULT_NOTE, 0), 0xf1a);
    assert_eq!(sas_pitch(-60, 0, DEFAULT_NOTE, 0), 0x116f);
    assert_eq!(sample_rate_hz(0x116f), 48_051);
}

#[test]
fn the_race_capture_s_modulated_plays_reproduce_too() {
    // Two rows of the 150-hit race capture with a non-zero pitch offset,
    // taken through `FUN_089950a0`'s note/fine split by hand: an offset of
    // `-1148` on note 60 is `60 * 128 - 1148 = 6532`, note 51 fine 4; an
    // offset of `7` is note 60 fine 7.
    assert_eq!(sas_pitch(-74, 66, 51, 4), 0x4c3);
    assert_eq!(sas_pitch(-89, 66, 60, 7), 0x35f);
}

#[test]
fn a_positive_centre_note_skips_the_scale() {
    assert_eq!(sas_pitch(62, 127, 62, 0), 0x1000);
    assert_eq!(sas_pitch(50, 127, 62, 0), 0x2000);
    assert_eq!(sas_pitch(74, 127, 62, 0), 0x800);
}

#[test]
fn the_semitone_table_is_two_to_the_i_over_twelve_truncated() {
    for (i, &entry) in SEMITONE_TABLE.iter().enumerate() {
        let expected = (32768.0 * 2f64.powf(i as f64 / 12.0)).floor() as u16;
        assert_eq!(entry, expected, "semitone {i}");
    }
}

#[test]
fn the_fine_table_is_two_to_the_i_over_1536_truncated() {
    for (i, &entry) in FINE_TABLE.iter().enumerate() {
        let expected = (32768.0 * 2f64.powf(i as f64 / 1536.0)).floor() as u16;
        assert_eq!(entry, expected, "fine {i}");
    }
}

#[test]
fn the_scale_is_not_a_rate_ratio_anyone_would_guess() {
    // Recorded so the number is not "corrected" to one of the obvious
    // candidates later: neither is what the binary carries.
    let from_48k = (65536.0 * 48_000.0 / 44_100.0) as u32;
    let from_48k_and_a_semitone = (65536.0 * 48_000.0 / 44_100.0 * 2f64.powf(1.0 / 12.0)) as u32;
    assert_ne!(NEGATIVE_CENTRE_SCALE, from_48k);
    assert_ne!(NEGATIVE_CENTRE_SCALE, from_48k_and_a_semitone);
    assert_eq!(from_48k_and_a_semitone, 0x12735);
}

/// Wipeout HD's own walk: same tables, `HD_NEGATIVE_CENTRE_SCALE` in place of
/// `NEGATIVE_CENTRE_SCALE`, `HD_SAMPLE_RATE` in place of `SAS_SAMPLE_RATE`.
/// Values cross-checked against `cargo run -p oag-formats --example
/// hd_rate_probe` on the real disc - see `docs/ghidra/functions/
/// ps3-hdfury-eu/sound.md`'s "The pitch" section.
#[test]
fn hd_s_own_scale_lands_its_largest_cluster_on_an_exact_rate() {
    // `(centre -60, fine 0)`, HD's single largest key-on cluster (1,803 of
    // 6,548 descriptors): 48,051 Hz under the PSP's borrowed scale, exactly
    // 48,000 under HD's own.
    assert_eq!(
        sas_pitch_scaled(-60, 0, DEFAULT_NOTE, 0, HD_NEGATIVE_CENTRE_SCALE),
        0x1000
    );
    assert_eq!(sample_rate_hz_at(0x1000, HD_SAMPLE_RATE), 48_000);
}

#[test]
fn hd_s_own_scale_moves_the_psp_s_exact_rates_off_by_about_a_tenth_of_a_percent() {
    // `(centre -62, fine 66)`, PSP's own "44,100 Hz exactly" descriptor -
    // HD's own scale puts it 0.11% off instead, inaudible and expected: the
    // two platforms' scales are close but not equal.
    assert_eq!(
        sas_pitch_scaled(-62, 66, DEFAULT_NOTE, 0, HD_NEGATIVE_CENTRE_SCALE),
        0xeaf
    );
    assert_eq!(sample_rate_hz_at(0xeaf, HD_SAMPLE_RATE), 44_051);
}

#[test]
fn the_hd_scale_is_close_to_but_not_a_semitone() {
    // Recorded so the number is not "corrected" to the semitone ratio it
    // resembles: `0x10f4a` is read off `lis r0,0x1; ori r0,r0,0xf4a` at
    // `0x0062ecb0`/`0x0062ecb8`, not derived.
    let semitone = (65536.0 * 2f64.powf(1.0 / 12.0)) as u32;
    assert_ne!(HD_NEGATIVE_CENTRE_SCALE, semitone);
}
