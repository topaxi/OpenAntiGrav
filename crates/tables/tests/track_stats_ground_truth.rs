//! Validates [`track_stats`](oag_tables::track_stats) against the real disc:
//! `PSP_GAME/USRDIR/FEData.wad`'s own `Data\Environments\16_Track\stats.xml`
//! on the USA PSP pressing.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! The test skips with a printed message when the disc image is absent. Set
//! `OAG_REQUIRE_GAME_DATA=1` to turn absence into a failure.
//!
//! # What this is for
//!
//! `docs/formats/race-setup.md`'s own live capture measured `16_Track`'s
//! `stats.xml` directly: `Physical Length="5178"`, and `SkillScaleValue`
//! `0.9` at `Easy`/`Venom`. This is the check that [`oag_tables::track_stats`]
//! reproduces those two numbers off the disc, and that the entry path this
//! reader assumes (`<the catalogue's own environment directory>\stats.xml`)
//! actually resolves - confirmed once already by hash, `oag-wad hash`
//! against `oag-wad list`'s own output, before this reader existed.

use std::path::PathBuf;

use oag_assets::Archive;
use oag_tables::handling::SpeedClass;

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

const ARCHIVE: &str = "PSP_GAME/USRDIR/FEData.wad";
const STATS_ENTRY: &str = r"Data\Environments\16_Track\stats.xml";

fn archive() -> Option<Archive> {
    let path = image("pulse-psp-usa.chd")?;
    let spec = format!("{}:{}", path.display(), ARCHIVE);
    Some(Archive::open(&spec).unwrap_or_else(|e| panic!("{spec}: {e}")))
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn sixteen_track_stats_match_the_live_capture() {
    let Some(mut archive) = self::archive() else {
        return;
    };

    let blob = archive
        .read_name(STATS_ENTRY)
        .unwrap_or_else(|e| panic!("{STATS_ENTRY}: {e}"));
    let stats =
        oag_tables::track_stats::from_blob(&blob).unwrap_or_else(|e| panic!("{STATS_ENTRY}: {e}"));

    assert_eq!(
        stats.length,
        Some(5178),
        "docs/formats/race-setup.md's own PPSSPP capture measured this exactly"
    );
    assert_eq!(
        stats.skill_curve(SpeedClass::Venom)[0],
        0.9,
        "Easy/Venom SkillScaleValue, docs/formats/race-setup.md's own example"
    );

    println!(
        "16_Track: length {:?}, Venom skill curve {:?}",
        stats.length,
        stats.skill_curve(SpeedClass::Venom)
    );
}

/// A second circuit, to check the archive path convention
/// (`Data\Environments\<id>\stats.xml`, the same directory a `track.vex`
/// lives in) is not a `16_Track`-only coincidence.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_second_track_also_parses() {
    let Some(mut archive) = self::archive() else {
        return;
    };

    let entry = r"Data\Environments\01_Track\stats.xml";
    let blob = archive
        .read_name(entry)
        .unwrap_or_else(|e| panic!("{entry}: {e}"));
    let stats =
        oag_tables::track_stats::from_blob(&blob).unwrap_or_else(|e| panic!("{entry}: {e}"));
    println!("01_Track: length {:?}", stats.length);
}
