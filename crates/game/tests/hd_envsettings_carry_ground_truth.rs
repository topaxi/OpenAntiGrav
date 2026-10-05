//! Whether `envsettings_bloom` builds the read `FunkLayerBloom` chain, over
//! every environment Wipeout HD/Fury ships.
//!
//! **`#[ignore]`d and never run in CI.** It needs the decrypted PS3 disc image
//! under `data/images/` - see
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(hd_envsettings_carry_ground_truth)'
//! ```
//!
//! # What this pins
//!
//! `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`'s "The resolve's Fury
//! circuits carry the front end's own Tone triple" section (2026-09-13) found
//! `envsettings_bloom` requiring the *circuit's own* `.envsettings` to author
//! the whole ten-key `HDR and Bloom` block, so `oag_post::hd_bloom::Chain`
//! was never built on eleven of the sixteen environments: the eight Fury/DLC
//! circuits, which author `Tone adaption boost` alone and never `Tone
//! darkening clamp`/`Tone maximum brightness`, and `zone_2`/`zone_3`/`zone_4`,
//! which ship no `.envsettings` at all. A live read on Sol 2 found the running
//! settings singleton holding all three Tone keys at the front end's own
//! `(20, 3, 4)` regardless - the registrar persists across a circuit's own
//! file, carrying forward whatever key the file does not declare.
//!
//! **Measured against the tree before this lane's
//! `crates/game/src/race/load/environment.rs::staged_envsettings` existed:
//! exactly 5 of these 16 built (`git stash` the module's carry and rerun to
//! reproduce it).** This test pins the fixed reading - all sixteen build -
//! rather than the gap itself, so it stays green rather than becoming a
//! permanently-failing record; the 5-of-16 count lives in `renderer.md` and
//! this lane's own report instead.
use oag_game::race;
use std::collections::BTreeSet;
use std::path::PathBuf;

/// The decrypted PS3 image.
const PS3_IMAGE: &str = "hdfury-ps3-eu-dec.iso";

fn image() -> Option<PathBuf> {
    oag_testdata::image(PS3_IMAGE)
}

/// Every environment on the disc, by its forward `track.vex` - the reversed
/// twin reads the same sibling `.envsettings`
/// (`crates/game/src/race/load/environment.rs::envsettings_name`), so it
/// answers `envsettings_bloom` identically and would not add a distinct case
/// here. The eight `01_..15_` entries are the Fury/DLC circuits (`DATA02`);
/// the plain names and `zone_1..4` are the base game's (`DATA00`) - see
/// `renderer.md`'s disc-wide survey table for which of the two ships a
/// complete `HDR and Bloom` block on its own.
const ENVIRONMENTS: &[&str] = &[
    "/data/environments/amphiseum/track.vex",
    "/data/environments/modesto_heights/track.vex",
    "/data/environments/talons_junction/track.vex",
    "/data/environments/tech_de_ra/track.vex",
    "/data/environments/zone_1/track.vex",
    "/data/environments/zone_2/track.vex",
    "/data/environments/zone_3/track.vex",
    "/data/environments/zone_4/track.vex",
    "/data/environments/01_vineta_k/track.vex",
    "/data/environments/02_track/track.vex",
    "/data/environments/03_track/track.vex",
    "/data/environments/04_chenghou_project/track.vex",
    "/data/environments/05_ubermall/track.vex",
    "/data/environments/10_sebenco_climb/track.vex",
    "/data/environments/12_sol_2/track.vex",
    "/data/environments/15_anulpha_pass/track.vex",
];

/// Before the carry, exactly these five built - every other row's own file
/// omits at least one `HDR and Bloom` key, or (the three `zone_*` rows) ships
/// no `.envsettings` at all. Kept as a named constant rather than folded into
/// the assertion below so a future regression names exactly which environments
/// it took bloom away from, against the reading this lane made and measured.
const BUILT_BEFORE_THE_CARRY: &[&str] = &[
    "/data/environments/amphiseum/track.vex",
    "/data/environments/modesto_heights/track.vex",
    "/data/environments/talons_junction/track.vex",
    "/data/environments/tech_de_ra/track.vex",
    "/data/environments/zone_1/track.vex",
];

/// **The reproduction and the fix, in one test.** Runs every environment
/// through the real loader and asserts today's answer - `16/16` - while
/// printing the built/skipped split so the report can quote it; the
/// `BUILT_BEFORE_THE_CARRY` constant is what makes clear, in a diff, that this
/// used to read `5/16`.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn every_environment_on_the_disc_builds_its_bloom_chain() {
    let Some(image) = image() else {
        return;
    };

    let mut built = Vec::new();
    let mut skipped = Vec::new();
    for track in ENVIRONMENTS {
        let loaded = race::load(&race::Options {
            source: image.display().to_string(),
            class: "VENOM".to_string(),
            track: Some((*track).to_string()),
            ..race::Options::default()
        })
        .unwrap_or_else(|e| panic!("{track}: loading the race: {e:#}"));

        if loaded.hd_bloom.is_some() {
            built.push(*track);
        } else {
            let bloom_lines: Vec<_> = loaded
                .report
                .iter()
                .filter(|line| line.contains("HDR and Bloom") || line.contains("bloom gate"))
                .collect();
            println!("{track}: no bloom chain, report: {bloom_lines:#?}");
            skipped.push(*track);
        }
    }
    println!(
        "{}/{} environments built the bloom chain\nbuilt: {built:#?}\nskipped: {skipped:#?}",
        built.len(),
        ENVIRONMENTS.len()
    );

    assert_eq!(
        skipped,
        Vec::<&str>::new(),
        "expected every environment to build its bloom chain now that the front end's own \
         values carry forward - see the module doc"
    );

    let before: BTreeSet<_> = BUILT_BEFORE_THE_CARRY.iter().collect();
    let after: BTreeSet<_> = ENVIRONMENTS.iter().collect();
    assert!(
        before.is_subset(&after) && before.len() == 5 && after.len() == 16,
        "BUILT_BEFORE_THE_CARRY/ENVIRONMENTS drifted from the 5-of-16 reading this test's own \
         doc comment names"
    );
}
