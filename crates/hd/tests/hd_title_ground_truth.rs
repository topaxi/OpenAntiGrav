//! The `oag-hd` table, checked against the disc it was read off.
//!
//! **`#[ignore]`d and never run in CI.** They need game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! The image has to be layer-1 decrypted first - `scripts/ps3iso.py decrypt`,
//! `docs/formats/ps3-disc.md`.
//!
//! # What this is for
//!
//! `crates/hd/src/tests.rs` checks the table against itself: that the default
//! circuit is in the roster, that no archive is named twice. **Those are
//! consistency checks and every one of them passes on a table of invented
//! names.** This file is the other half: it re-derives the rosters and the
//! archive split from the manifest and compares, so a constant that drifted from
//! the disc says so.
//!
//! It also closes the seam the whole crate exists for - that `Archives::open`,
//! given HD's table, opens seven PSARC archives and answers a read by path -
//! which nothing above `oag-assets` had done before this crate existed.

use std::path::{Path, PathBuf};

use oag_hd::{TITLE, archives, names, race};

/// The decrypted PS3 image.
const PS3_IMAGE: &str = "hdfury-ps3-eu-dec.iso";

fn image() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images")
        .join(PS3_IMAGE);

    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

fn opened() -> Option<oag_assets::Archives> {
    let path = image()?;
    Some(oag_hd::open(path.to_str().expect("utf-8 path")).expect("the disc opens as HD"))
}

/// Every path any mounted archive holds, which is what the table is checked
/// against.
fn every_path(archives: &oag_assets::Archives) -> Vec<String> {
    let mut all = Vec::new();
    let mut take = |container: &oag_assets::Container| {
        if let oag_assets::Container::Psarc(psarc) = container {
            all.extend(psarc.paths().iter().cloned());
        }
    };
    take(&archives.data);
    if let Some(fe) = &archives.fe {
        take(fe);
    }
    for extra in &archives.extra {
        take(extra);
    }
    all
}

/// All seven archives open, as PSARCs, off the disc.
///
/// The claim the `extra` role was added for. Asserted on the *opened* archives
/// rather than on the layout, because a name that resolves and then fails to
/// open is the failure this would otherwise miss.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn all_seven_archives_open_as_psarcs_off_one_disc() {
    let Some(archives) = opened() else { return };

    let mut opened: Vec<&str> = std::iter::once(&archives.data)
        .chain(archives.fe.as_ref())
        .chain(&archives.extra)
        .map(|container| {
            assert!(
                matches!(container, oag_assets::Container::Psarc(_)),
                "{} did not open as a PSARC",
                container.label()
            );
            container.label()
        })
        .collect();
    assert_eq!(opened.len(), 7, "{opened:?}");
    opened.sort_unstable();

    let expected: Vec<String> = TITLE
        .archive_names()
        .iter()
        .map(|name| format!("{}:{name}", image().expect("checked above").display()))
        .collect();
    let mut expected: Vec<&str> = expected.iter().map(String::as_str).collect();
    expected.sort_unstable();
    assert_eq!(opened, expected);

    // 11,664 entries across the seven, from `docs/formats/psarc.md`. Counted
    // here so a table that mounted six would fail on the total as well as on
    // the count above.
    let total: usize = std::iter::once(&archives.data)
        .chain(archives.fe.as_ref())
        .chain(&archives.extra)
        .map(oag_assets::Container::entry_count)
        .sum();
    assert_eq!(total, 11_664);
}

/// The circuit roster is the disc's, not this crate's memory of it.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn every_environment_listed_has_a_track_vex_and_none_is_missing() {
    let Some(archives) = opened() else { return };
    let paths = every_path(&archives);

    let mut on_disc: Vec<&str> = paths
        .iter()
        .filter_map(|path| {
            path.strip_prefix("/data/environments/")?
                .strip_suffix("/track.vex")
        })
        .collect();
    on_disc.sort_unstable();

    let mut listed: Vec<&str> = names::ENVIRONMENTS.to_vec();
    listed.sort_unstable();

    assert_eq!(
        on_disc, listed,
        "the ENVIRONMENTS table and the disc disagree"
    );
}

/// So is the team roster, and the mode ships are separated the way the table
/// says.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_team_roster_matches_the_disc_and_the_variants_are_left_out() {
    let Some(archives) = opened() else { return };
    let paths = every_path(&archives);

    let ships: Vec<&str> = paths
        .iter()
        .filter_map(|path| {
            path.strip_prefix("/data/ships/")?
                .strip_suffix("/handlingstats.xml")
        })
        .collect();

    // 40 directories: 12 teams, 24 `_c1`/`_n1` variants, 4 mode ships. The
    // twelve teams have two variants each and the four mode ships have none.
    assert_eq!(ships.len(), 40, "{ships:?}");

    let mut base: Vec<&str> = ships
        .iter()
        .copied()
        .filter(|id| !id.ends_with("_c1") && !id.ends_with("_n1"))
        .collect();
    base.sort_unstable();

    let mut listed: Vec<&str> = names::TEAMS
        .iter()
        .chain(names::MODE_SHIPS)
        .copied()
        .collect();
    listed.sort_unstable();

    assert_eq!(
        base, listed,
        "teams plus mode ships should be every base id"
    );
}

/// `DATA03` carries four teams and no circuit, which is the row that forced the
/// `extra` role.
///
/// If this ever fails because `DATA03` gained a circuit, the design is still
/// right; if it fails because the four teams moved, the reasoning in
/// `ArchiveCandidates::extra`'s docs needs rewriting rather than the row.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_archive_that_forced_the_extra_role_still_holds_four_teams_and_no_circuit() {
    let Some(path) = image() else { return };
    let spec = format!("{}:{}", path.display(), archives::DATA03);
    let archive = oag_assets::psarc::Archive::open(&spec).expect("DATA03 opens");

    let teams: Vec<&String> = archive
        .paths()
        .iter()
        .filter(|p| p.ends_with("/handlingstats.xml"))
        .collect();
    assert_eq!(teams.len(), 4, "{teams:?}");

    assert!(
        !archive.paths().iter().any(|p| p.ends_with("/track.vex")),
        "DATA03 holds no circuit, which is why two archives were not enough"
    );
}

/// The whole point: the default circuit and the default team both read through
/// `Archives`, by the names the table gives, with no HD-specific spelling.
///
/// **The handling path is `oag_formats::handling::entry_name`'s Pulse-shaped
/// one**, backslashes and all, resolving on a PSARC because
/// `oag_assets::psarc` normalises before it looks up. That is the claim
/// `crates/hd/src/race.rs`'s module doc makes, and this is where it is checked.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_race_defaults_both_read_through_archives() {
    let Some(mut archives) = opened() else { return };

    let track = archives
        .read_name(race::DEFAULT_TRACK)
        .expect("the default circuit reads");
    assert_eq!(&track[12..16], b"XXEV", "a big-endian .vex");

    let stats_name = format!(r"Data\Ships\{}\handlingstats.xml", race::DEFAULT_TEAM);
    let stats = archives
        .read_name(&stats_name)
        .expect("the default team's handling reads, spelled Pulse's way");
    assert!(
        std::str::from_utf8(&stats)
            .expect("handlingstats.xml is text")
            .contains("<Stats"),
        "and it is the handling document rather than something that hashed the same"
    );

    // And the archive each came out of is reported, which is what tells a load
    // report which of the seven answered.
    assert!(
        archives
            .locate(race::DEFAULT_TRACK)
            .unwrap()
            .ends_with(archives::DATA00.rsplit('/').next().expect("a file name")),
        "Talon's Junction is in DATA00"
    );
}
