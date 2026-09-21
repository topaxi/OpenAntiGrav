//! Wipeout HD's built-in campaign, read through Pulse's own
//! `oag_tables::race_campaign` parser, extended additively for what this
//! title authors that Pulse does not - see `docs/formats/race-campaign.md`'s
//! HD section and `crates/hd/src/campaign.rs`'s own doc comment for the full
//! archive table this file measures.
//!
//! **`#[ignore]`d and never run in CI.** They need game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this is for
//!
//! The discriminating question the thread that wrote this file opened with:
//! does `race_campaign::from_blob` parse HD's grids unchanged, or does the
//! schema need extending? The answer is extending - a per-difficulty medal
//! target shape, two new mode spellings, and a handful of new grid-level
//! attributes, all added to `oag_tables::race_campaign` without changing
//! what any Pulse cell reads as (`race_campaign_ground_truth.rs` on that
//! crate still passes unchanged). This file is what proves the extended
//! parser against the real disc, both through this project's own
//! [`oag_assets::Archives`] precedence (what a normal read reaches) and by
//! opening every archive directly (what is actually authored, precedence
//! aside).

use std::collections::BTreeMap;
use std::path::PathBuf;

use oag_assets::Archives;
use oag_tables::race_campaign::{self, MedalTargets, Mode};

/// The decrypted PS3 image.
const PS3_IMAGE: &str = "hdfury-ps3-eu-dec.iso";

fn image() -> Option<PathBuf> {
    oag_testdata::image(PS3_IMAGE)
}

fn opened() -> Option<Archives> {
    let path = image()?;
    Some(oag_hd::open(path.to_str().expect("utf-8 path")).expect("the disc opens as HD"))
}

/// Reads every grid this project's own archive precedence reaches: whichever
/// copy [`Archives::read_name`] resolves to for each of the sixteen entry
/// names `Definition.xml` lists.
fn read_precedence_grids(archives: &mut Archives) -> Vec<race_campaign::Grid> {
    let definition = archives
        .read_name(oag_hd::campaign::DEFINITION_ENTRY)
        .expect("Definition.xml reads");
    let expanded = oag_tables::fexml::text(&definition).expect("plain or shortened, either way");
    let entries = race_campaign::definition_entries(&expanded);
    assert_eq!(
        entries.len(),
        16,
        "DATA00's own Definition.xml names 16 grids"
    );

    entries
        .iter()
        .map(|src| {
            let blob = archives
                .read_name(src)
                .unwrap_or_else(|e| panic!("{src}: {e}"));
            race_campaign::from_blob(&blob).unwrap_or_else(|e| panic!("{src}: {e}"))
        })
        .collect()
}

/// The precedence-resolved campaign parses whole, sixteen grids, and is
/// genuinely mixed-schema: `grid0`..`grid7` flat (the `DATA02.PSARC` copy),
/// `grid8`..`grid15` per-difficulty (`DATA00.PSARC`, Fury-only). See
/// `crates/hd/src/campaign.rs`'s module doc for the archive table this
/// measures.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_precedence_resolved_campaign_is_sixteen_grids_mixed_schema() {
    let Some(mut archives) = opened() else {
        return;
    };
    let grids = read_precedence_grids(&mut archives);
    assert_eq!(grids.len(), 16);

    let flat: Vec<&str> = grids
        .iter()
        .filter(|g| g.cells.iter().all(|c| c.difficulty_targets.is_none()))
        .map(|g| g.name.as_str())
        .collect();
    let per_difficulty: Vec<&str> = grids
        .iter()
        .filter(|g| g.cells.iter().any(|c| c.difficulty_targets.is_some()))
        .map(|g| g.name.as_str())
        .collect();

    assert_eq!(
        flat,
        [
            "grid0", "grid1", "grid2", "grid3", "grid4", "grid5", "grid6", "grid7"
        ],
        "the first eight grids resolve to DATA02's flat-schema copy"
    );
    assert_eq!(
        per_difficulty,
        [
            "grid8", "grid9", "grid10", "grid11", "grid12", "grid13", "grid14", "grid15"
        ],
        "the last eight resolve to DATA00's own Fury-only per-difficulty copy"
    );

    // grid4's own <Values> tag is missing a closing `>` on the disc, in all
    // three of its copies - see campaign.rs's module doc. Read structurally
    // that swallows every <PI_Cell> under the broken <Values> instead of
    // <PI_Grid>, so this project's own parser correctly reads zero cells for
    // a grid that authors five. Asserted here so a future fix to the XML
    // reader that started tolerating the break would have to notice this
    // assertion rather than silently start disagreeing with it.
    let grid4 = grids.iter().find(|g| g.name == "grid4").expect("grid4");
    assert_eq!(
        grid4.cells.len(),
        0,
        "grid4's own <Values> tag is broken on the disc; see campaign.rs"
    );

    let total_cells: usize = grids.iter().map(|g| g.cells.len()).sum();
    assert_eq!(
        total_cells, 157,
        "77 across DATA02's grid0..7 (6+8+10+10+0+12+14+17, grid4's own 0 included) \
         plus 80 across DATA00's grid8..15 (7+7+9+9+11+11+13+13)"
    );
}

/// Every grid archive on the disc, in the order `crate::archives::ALL`
/// declares - independent of any precedence, so this is what is actually
/// authored rather than what a normal read reaches.
fn read_every_grid_copy(
    archives: &mut Archives,
) -> BTreeMap<&'static str, Vec<race_campaign::Grid>> {
    let mut out = BTreeMap::new();
    for archive in oag_hd::archives::ALL.iter().copied() {
        let mut grids = Vec::new();
        for index in 0..oag_hd::campaign::GRID_COUNT {
            let name = oag_hd::campaign::entry_name(index);
            let matches = archives.read_every_name(&name);
            if let Some((_, blob)) = matches.iter().find(|(label, _)| label.ends_with(archive)) {
                grids.push(
                    race_campaign::from_blob(blob)
                        .unwrap_or_else(|e| panic!("{archive} {name}: {e}")),
                );
            }
        }
        if !grids.is_empty() {
            out.insert(archive, grids);
        }
    }
    out
}

/// Every one of the 32 grid files across the four archives that carry
/// `plugins/grids/` parses with the extended schema - the direct answer to
/// this thread's discriminating question, independent of which copy
/// [`Archives`]'s precedence happens to reach.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn every_grid_file_on_every_archive_parses() {
    let Some(mut archives) = opened() else {
        return;
    };
    let by_archive = read_every_grid_copy(&mut archives);

    assert_eq!(
        by_archive.len(),
        4,
        "grids/ is on DATA00, DATA02, DATA04 and DATA06"
    );
    assert_eq!(
        by_archive[oag_hd::archives::DATA00].len(),
        8,
        "grid_08..grid_15"
    );
    assert_eq!(
        by_archive[oag_hd::archives::DATA02].len(),
        8,
        "grid_00..grid_07"
    );
    assert_eq!(
        by_archive[oag_hd::archives::DATA04].len(),
        8,
        "grid_00..grid_07"
    );
    assert_eq!(
        by_archive[oag_hd::archives::DATA06].len(),
        8,
        "grid_00..grid_07"
    );

    let cells =
        |archive: &str| -> usize { by_archive[archive].iter().map(|g| g.cells.len()).sum() };
    assert_eq!(cells(oag_hd::archives::DATA00), 80);
    assert_eq!(cells(oag_hd::archives::DATA02), 77);
    assert_eq!(cells(oag_hd::archives::DATA04), 77);
    assert_eq!(cells(oag_hd::archives::DATA06), 77);

    // DATA04's and DATA06's copies of grid_00..07 carry the per-difficulty
    // schema too, unlike DATA02's flat one for the same eight grids - a real
    // difference between three copies of what Definition.xml treats as one
    // file. Not reached by this project's own read precedence (see the
    // module-level test above), but authored on the disc regardless.
    for archive in [oag_hd::archives::DATA04, oag_hd::archives::DATA06] {
        let any_per_difficulty = by_archive[archive]
            .iter()
            .flat_map(|g| &g.cells)
            .any(|c| c.difficulty_targets.is_some());
        assert!(
            any_per_difficulty,
            "{archive}'s own grid_00..07 should carry per-difficulty targets too"
        );
    }
}

/// The two mode spellings this table has no Pulse ordinal for -
/// `"NitroBattle"` and `"Detonator"` - are kept raw as [`Mode::Other`] rather
/// than rejected, and only appear on the Fury-only grids.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn nitro_battle_and_detonator_are_kept_raw_and_only_on_fury_grids() {
    let Some(mut archives) = opened() else {
        return;
    };
    let grids = read_precedence_grids(&mut archives);

    let mut other_modes: Vec<(String, String)> = Vec::new();
    for grid in &grids {
        for cell in &grid.cells {
            if let Mode::Other(name) = &cell.mode {
                other_modes.push((grid.name.clone(), name.clone()));
            }
        }
    }

    assert!(
        other_modes
            .iter()
            .all(|(grid, _)| grid.starts_with("grid") && {
                let n: u32 = grid.trim_start_matches("grid").parse().unwrap();
                n >= 8
            }),
        "an unrecognised mode should only be on the Fury-only grids: {other_modes:?}"
    );

    let mut names: Vec<&str> = other_modes.iter().map(|(_, name)| name.as_str()).collect();
    names.sort_unstable();
    names.dedup();
    assert_eq!(names, ["Detonator", "NitroBattle"]);
}

/// `skillMedium` is Wipeout HD's own name for Pulse's `skill` - read as an
/// alias, landing in the same field, so
/// `Cell::skill_for_difficulty`/`evaluate_medal` need no HD-specific caller
/// logic.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn skill_medium_lands_in_the_same_field_as_pulses_skill() {
    let Some(mut archives) = opened() else {
        return;
    };
    let grids = read_precedence_grids(&mut archives);

    let scaled_cells = grids
        .iter()
        .flat_map(|g| &g.cells)
        .filter(|c| c.ai_count.is_some())
        .count();
    let with_skill = grids
        .iter()
        .flat_map(|g| &g.cells)
        .filter(|c| c.ai_count.is_some() && c.skill.is_some())
        .count();

    assert!(scaled_cells > 0, "at least one cell scales AI by skill");
    assert_eq!(
        scaled_cells, with_skill,
        "every cell with opponents should read a skill value under one of its two names"
    );
}

/// `Elimination`/`NitroBattle` cells carry a *real* `nitro_elimination_targets`
/// triple and a *dummy* `difficulty_targets` (every rung `1`/`2`/`3`, the same
/// dummy shape a plain `Race` cell's *flat* `gold`/`silver`/`bronze` also
/// happens to take since `1`/`2`/`3` reads as a real finishing position there,
/// so this test does not compare against `Race`). `Detonator`, the one
/// other mode on the shipped grids carrying both fields, has it the other way
/// round: a real `difficulty_targets` (six-figure scores) and a dummy
/// `nitro_elimination_targets` (`1`/`1`/`1`). Confirmed directly on
/// `DATA00.PSARC`'s eight Fury grids, the only ones carrying
/// [`Cell::nitro_elimination_targets`] at all.
///
/// This is the data-side half of the evidence
/// `docs/ghidra/functions/ps3-hdfury-eu/race-campaign.md`'s medal-law section
/// cites: on the executable's own attribute table (`0x008ae898`),
/// `NitroElimNovice`/`Skilled`/`Elite` are `PI_Cell` attributes in their own
/// right, immediately after `EasyGold`..`HardBronze` - so a cell authoring
/// dummy `difficulty_targets` alongside a real nitro triple is the shape a
/// cell that is *read* through the nitro triple rather than the flat one
/// would take, not a coincidence of two independent XML attributes.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn eliminationfamily_cells_carry_a_real_nitro_triple_and_a_dummy_flat_one() {
    let Some(mut archives) = opened() else {
        return;
    };
    let by_archive = read_every_grid_copy(&mut archives);
    let fury_grids = &by_archive[oag_hd::archives::DATA00];

    let dummy_flat = MedalTargets {
        gold: 1,
        silver: 2,
        bronze: 3,
    };
    let mut checked_elimination_family = 0;
    let mut checked_detonator = 0;

    for grid in fury_grids {
        for cell in &grid.cells {
            let Some(dt) = &cell.difficulty_targets else {
                continue;
            };
            let Some(nitro) = cell.nitro_elimination_targets else {
                continue;
            };
            let flat_is_dummy = [dt.easy, dt.medium, dt.hard]
                .iter()
                .all(|t| *t == dummy_flat);
            let nitro_is_dummy = nitro == (1, 1, 1);

            let is_elimination_family = matches!(&cell.mode, Mode::Elimination)
                || matches!(&cell.mode, Mode::Other(name) if name == "NitroBattle");
            let is_detonator = matches!(&cell.mode, Mode::Other(name) if name == "Detonator");

            if is_elimination_family {
                assert!(
                    flat_is_dummy,
                    "{}: an Elimination/NitroBattle cell's difficulty_targets \
                     should be dummy 1/2/3 on every rung, got {dt:?}",
                    cell.name
                );
                assert!(
                    !nitro_is_dummy,
                    "{}: an Elimination/NitroBattle cell's nitro triple \
                     should be real, got {nitro:?}",
                    cell.name
                );
                checked_elimination_family += 1;
            } else if is_detonator {
                assert!(
                    nitro_is_dummy,
                    "{}: a Detonator cell's nitro triple should be dummy \
                     1/1/1, got {nitro:?}",
                    cell.name
                );
                assert!(
                    !flat_is_dummy,
                    "{}: a Detonator cell's difficulty_targets should be \
                     real, got {dt:?}",
                    cell.name
                );
                checked_detonator += 1;
            }
        }
    }

    assert!(
        checked_elimination_family > 0,
        "expected at least one Elimination/NitroBattle cell with both fields"
    );
    assert!(
        checked_detonator > 0,
        "expected at least one Detonator cell with both fields"
    );
}
