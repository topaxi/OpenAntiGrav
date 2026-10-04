//! Validates [`ai_race_stats`](oag_tables::ai_race_stats) against the real
//! disc: the four `Data\XML\AIRaceStats_<class>.xml` in the USA PSP pressing's
//! `Data.wad`, and the finished player's thrust they give.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this is for
//!
//! On 2026-10-04 PPSSPP read the finished player's thrust as `56.7` after
//! a first-place finish and `56.0` after a fourth, Venom, Easy, Talon's
//! Junction, Single Race (`docs/ghidra/functions/psp-pulse-usa/race-finish.md`).
//! Those are two numbers off a running original; this checks that the disc's
//! own tables, the track's own skill value and
//! [`AiRaceStats::finished_player_thrust`](oag_tables::ai_race_stats::AiRaceStats::finished_player_thrust)
//! reproduce both, so the port cannot drift from them without a red test.

use std::path::PathBuf;

use oag_assets::Archive;
use oag_tables::ai_race_stats;
use oag_tables::handling::SpeedClass;
use oag_tables::race_campaign::Difficulty;
use oag_tables::track_stats::{self, ModeTerm};

fn archive(name: &str) -> Option<Archive> {
    let path: PathBuf = oag_testdata::image("pulse-psp-usa.chd")?;
    let spec = format!("{}:PSP_GAME/USRDIR/{name}", path.display());
    Some(Archive::open(&spec).unwrap_or_else(|e| panic!("{spec}: {e}")))
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_class_file_parses_and_reads_eight_places() {
    let Some(mut data) = archive("Data.wad") else {
        return;
    };
    for class in SpeedClass::ALL {
        let entry = ai_race_stats::entry_name(class);
        let blob = data
            .read_name(&entry)
            .unwrap_or_else(|e| panic!("{entry}: {e}"));
        let stats =
            ai_race_stats::from_blob(&blob, class).unwrap_or_else(|e| panic!("{entry}: {e}"));
        assert!(
            stats.ai_thrust.iter().all(|&v| (50.0..=150.0).contains(&v)),
            "{entry}: {:?}",
            stats.ai_thrust
        );
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_finished_venom_player_flies_at_the_thrust_ppsspp_read() {
    let (Some(mut data), Some(mut fedata)) = (archive("Data.wad"), archive("FEData.wad")) else {
        return;
    };
    let entry = ai_race_stats::entry_name(SpeedClass::Venom);
    let stats =
        ai_race_stats::from_blob(&data.read_name(&entry).expect("venom"), SpeedClass::Venom)
            .expect("venom parses");
    let track = track_stats::from_blob(
        &fedata
            .read_name(r"Data\Environments\16_Track\stats.xml")
            .expect("16_Track stats"),
    )
    .expect("16_Track stats parse");
    // A Single Race, weapons on, eight craft: read live as g_game_mode 3,
    // g_weapons_enabled 1, eight racers, Easy.
    let skill = track_stats::ambient_skill_scale(
        Some(&track),
        SpeedClass::Venom,
        Difficulty::Easy,
        ModeTerm::Race {
            weapons: true,
            full_grid: true,
        },
    );
    assert!((skill - 0.9).abs() < 1e-6, "skill {skill}");
    let first = stats.finished_player_thrust(1, skill);
    let fourth = stats.finished_player_thrust(4, skill);
    assert!((first - 56.7).abs() < 0.05, "first place: {first}");
    assert!((fourth - 56.0).abs() < 0.05, "fourth place: {fourth}");
}
