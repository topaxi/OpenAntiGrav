//! The PSP disc carries a Concept shield shell for every team, and a Concept
//! race loads it.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test psp_shield_model_ground_truth --run-ignored all
//! ```
//!
//! # What only real data can say here
//!
//! `ShipShield_Construct` formats the `FE_TeamModel` registry value into
//! `%s\%sshield.vex` (`docs/ghidra/functions/psp-pulse-usa/shield-pickup.md`,
//! "The PSP's shell follows the Concept model"). That is only worth drawing if
//! `extrashield.vex` is on the PSP disc under every team directory and is a
//! different file from `shipshield.vex`, which is what this reads.

use oag_game::race;

const TEAMS: [&str; 8] = [
    "AG_Systems",
    "Assegai",
    "EGX",
    "Feisar",
    "Goteki",
    "Piranha",
    "Qirex",
    "Triakis",
];

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn every_team_carries_a_concept_shell_distinct_from_its_ordinary_one() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let mut archives =
        oag_pulse::open(&image.display().to_string()).expect("opening the PSP archives");
    for team in TEAMS {
        let name = |model| {
            race::shield_entry_names(
                oag_title::race::SHIP_DIR,
                team,
                oag_assets::Platform::Psp,
                model,
            )[0]
            .clone()
        };
        let (ordinary, concept) = (name(None), name(Some("extra")));
        assert_eq!(ordinary, format!(r"Data\Ships\{team}\shipshield.vex"));
        assert_eq!(concept, format!(r"Data\Ships\{team}\extrashield.vex"));
        let ordinary = archives.read_name(&ordinary).expect("the ordinary shell");
        let concept = archives.read_name(&concept).expect("the Concept shell");
        assert_ne!(ordinary, concept, "{team}: two names, one file");
    }
}
