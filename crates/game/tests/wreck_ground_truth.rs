//! A Pulse craft's wreck on a real disc image.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test wreck_ground_truth --run-ignored all
//! ```
//!
//! # What only real data can say here
//!
//! `Ship_SetState`'s case 5 makes `shipwreck.vex` the live model
//! (`docs/ghidra/functions/psp-pulse-usa/ship-wreck-model.md`). This proves the
//! loader finds every team's wreck under the name the original assembles, that
//! none of the eight authors the `0x2000` flag a hull's meshes carry (so the
//! wreck has no extra pass), and that the swap reaches the frame: a wrecked
//! opponent's frame submits exactly the wreck's triangles in place of the hull's
//! finest tier and extra pass, and the same frame during the explosion (state 4)
//! is the hull's, pixel for pixel.

use oag_livery::{self as livery, LoadContext};
use oag_race::Mode;

/// Every Pulse PSP team, by the directory the disc keeps it in.
const TEAMS: [&str; 8] = [
    "Assegai",
    "Qirex",
    "Feisar",
    "AG_Systems",
    "EGX",
    "Goteki",
    "Triakis",
    "Piranha",
];

fn load(wreck: bool, mode: Mode) -> Option<(Vec<livery::Livery>, Vec<String>)> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let mut archives =
        oag_assets::Archives::open(&image.to_string_lossy(), oag_pulse::TITLE).expect("archives");
    let teams: Vec<String> = TEAMS.iter().map(|t| (*t).to_string()).collect();
    let mut report = Vec::new();
    let liveries = livery::load(
        &mut archives,
        &teams,
        &LoadContext {
            race: oag_pulse::TITLE.race,
            mode,
            flare: oag_pulse::TITLE.flare,
            hull_overlay: false,
            hull_shine: true,
            hull_wreck: wreck,
            absorb_shell: false,
            zone_liveries: &[],
        },
        None,
        None,
        &mut report,
    )
    .expect("the liveries load");
    Some((liveries, report))
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn every_teams_wreck_loads_and_has_no_extra_pass() {
    let Some((liveries, _)) = load(true, Mode::SingleRace) else {
        return;
    };
    assert_eq!(liveries.len(), TEAMS.len());
    for (team, livery) in TEAMS.iter().zip(&liveries) {
        let wreck = livery
            .wreck
            .as_ref()
            .unwrap_or_else(|| panic!("{team}: no wreck was loaded"));
        assert!(
            wreck.model.label.ends_with("shipwreck.vex"),
            "{team}: {}",
            wreck.model.label
        );
        assert!(!wreck.model.indices.is_empty(), "{team}: an empty wreck");
        assert_ne!(
            wreck.model.indices.len(),
            livery.hull.indices.len(),
            "{team}"
        );
        // The hull's meshes carry 0x2000 and the wreck's do not, so there is
        // no extra pass to build and none is drawn.
        assert!(wreck.model.shine_draws.is_empty(), "{team}");
    }
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn a_context_that_does_not_ask_for_the_wreck_loads_none() {
    let Some((liveries, _)) = load(false, Mode::SingleRace) else {
        return;
    };
    assert!(liveries.iter().all(|l| l.wreck.is_none()));
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn a_zone_race_loads_the_zone_wreck_where_the_disc_has_one() {
    let Some((liveries, report)) = load(true, Mode::Zone) else {
        return;
    };
    for (team, livery) in TEAMS.iter().zip(&liveries) {
        let name = format!(r"Data\Ships\{team}\zonewreck.vex");
        match &livery.wreck {
            Some(wreck) => assert_eq!(wreck.model.label, name),
            None => assert!(
                report
                    .iter()
                    .any(|l| l.contains(&name) && l.contains("not in")),
                "{team}: no zone wreck and no report line saying so"
            ),
        }
    }
}

/// The team `oag-game --race` flies on grid slot `slot`: the title's default
/// team for the player, the rest by `livery::teams_for_slots`'s roster draw
/// over the disc's own definition, off the default seed - the same inputs the
/// game's own `--race` run below resolves.
fn team_on_slot(slot: usize) -> String {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd").expect("the image");
    let mut archives =
        oag_assets::Archives::open(&image.to_string_lossy(), oag_pulse::TITLE).expect("archives");
    let blob = archives
        .read_name(oag_pulse::TITLE.plugin_definition)
        .expect("the plugin definition");
    let xml = oag_tables::fexml::text(&blob).expect("the definition is text");
    let available: Vec<String> = oag_raceplay::catalogue::teams(&xml)
        .into_iter()
        .map(|team| team.id)
        .collect();
    livery::teams_for_slots(
        oag_pulse::TITLE.race.team,
        &available,
        oag_gameplay::MAX_SHIPS,
        oag_raceplay::SEED,
    )
    .swap_remove(slot)
}

/// One frame of the grid with slot 7's craft destroyed at the end of tick 40:
/// its pixels, and the triangle count the frame's own log line reports.
fn frame(
    image: &std::path::Path,
    scratch: &std::path::Path,
    name: &str,
    ticks: &str,
    flag: &[&str],
) -> (Vec<u8>, u64) {
    let out = scratch.join(format!("{name}.png"));
    let run = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(image)
        .args(["--race", "--no-audio", "--size", "480x272"])
        .args(["--render-scale", "100", "--msaa", "off"])
        .args(["--screen-filter", "off", "--anisotropy", "off"])
        .args(["--motion-blur", "off", "--opponents", "--ticks", ticks])
        .args(["--force-wreck", "40:7"])
        .args(flag)
        .arg("--screenshot")
        .arg(&out)
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"))
        .output()
        .expect("running oag-game");
    assert!(run.status.success(), "oag-game {name}");
    let log = String::from_utf8_lossy(&run.stderr);
    let triangles = log
        .lines()
        .find(|line| line.contains("frame at tick"))
        .and_then(|line| line.split(", ").nth(2))
        .and_then(|part| part.split(' ').next())
        .and_then(|count| count.parse().ok())
        .unwrap_or_else(|| panic!("{name}: no frame statistics in the log:\n{log}"));
    (
        std::fs::read(&out).expect("reading the screenshot"),
        triangles,
    )
}

fn triangles(model: &oag_mesh::mesh::Model, nearest_only: bool) -> u64 {
    model
        .draws
        .iter()
        .chain(&model.alpha_tested_draws)
        .chain(&model.transparent_draws)
        .filter(|draw| !nearest_only || model.lod_groups.shows_nearest(draw))
        .map(|draw| u64::from(draw.range.end - draw.range.start) / 3)
        .sum()
}

/// Slot 7 is destroyed at the end of tick 40; which team flies it is the
/// roster draw's ([`team_on_slot`]). Tick 60 is inside its
/// half-second explosion (state 4, which keeps the hull) and tick 100 is past
/// the edge into state 5, where the wreck is the live model.
///
/// The frame's triangle count is what pins the draw: with the wreck the frame
/// loses the hull's finest tier and its extra pass and gains the wreck, exactly.
/// Dropping the loader, the swap, the hull skip, the pass skip or the wreck's
/// own draw each moves that number. (The pixels alone cannot say: a frame with
/// the hull skipped and no wreck drawn still differs from one with the hull.)
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and a GPU adapter"]
fn the_wreck_replaces_the_hull_from_state_5_and_not_before() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let Some((liveries, _)) = load(true, Mode::SingleRace) else {
        return;
    };
    let on_seven = team_on_slot(7);
    let victim = liveries
        .iter()
        .find(|l| l.team == on_seven)
        .unwrap_or_else(|| panic!("{on_seven} is not among the loaded teams"));
    let hull = triangles(&victim.hull, true);
    let pass = victim
        .shine
        .as_ref()
        .map_or(0, |model| triangles(model, false));
    let wreck = triangles(&victim.wreck.as_ref().expect("a wreck").model, false);

    let scratch = std::env::temp_dir().join(format!("oag-wreck-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("creating the scratch directory");
    let (exploding, exploding_tris) = frame(&image, &scratch, "exploding", "60", &[]);
    let (exploding_hull, exploding_hull_tris) = frame(
        &image,
        &scratch,
        "exploding-hull",
        "60",
        &["--no-hull-wreck"],
    );
    let (wrecked, wrecked_tris) = frame(&image, &scratch, "wrecked", "100", &[]);
    let (_, hulled_tris) = frame(
        &image,
        &scratch,
        "wrecked-hull",
        "100",
        &["--no-hull-wreck"],
    );
    std::fs::remove_dir_all(&scratch).ok();

    assert_eq!(
        exploding, exploding_hull,
        "the wreck replaced the hull during the explosion"
    );
    assert_eq!(exploding_tris, exploding_hull_tris);
    assert_eq!(
        hulled_tris, exploding_hull_tris,
        "the hull frame moved between ticks 60 and 100"
    );
    assert_eq!(
        hulled_tris - wrecked_tris,
        hull + pass - wreck,
        "a wrecked craft draws its wreck in place of its hull and the hull's extra pass"
    );
    assert_ne!(wrecked, exploding, "the wreck drew nothing in state 5");
}
