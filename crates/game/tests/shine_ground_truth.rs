//! A Pulse hull's `0x2000` extra pass on a real disc image.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test shine_ground_truth --run-ignored all
//! ```
//!
//! # What only real data can say here
//!
//! The original draws six batches of every team's hull twice - once under
//! environment mapping with the material's *second* texture - `envtest4bit.tga`,
//! or `envmap_stripe2.tga` on AG_Systems - which a recorded GE list on Talon's Junction showed under `TEXMAPMODE` 2,
//! lights 0 and 1, lighting off. This proves the loader finds exactly those
//! batches on every team's own `Ship.vex` and hands the second texture to the
//! second draw, so a loader that read the wrong bit, the wrong material word
//! or the wrong file comes back with another count or another texture and
//! fails here rather than drawing nothing quietly.

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

fn load(shine: bool) -> Option<Vec<livery::Livery>> {
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
            mode: Mode::SingleRace,
            flare: oag_pulse::TITLE.flare,
            hull_overlay: false,
            hull_shine: shine,
            hull_wreck: false,
            absorb_shell: false,
            zone_liveries: &[],
        },
        None,
        None,
        &mut report,
    )
    .expect("the liveries load");
    Some(liveries)
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn every_teams_hull_carries_six_extra_pass_batches_under_the_env_map() {
    let Some(liveries) = load(true) else { return };
    assert_eq!(liveries.len(), TEAMS.len());
    for (team, livery) in TEAMS.iter().zip(&liveries) {
        let shine = livery
            .shine
            .as_ref()
            .unwrap_or_else(|| panic!("{team}: no extra pass was built"));
        // shipShape's three blended batches, both airbrakes' one each and the
        // canopy's glass - the six the recorded GE list showed under mode 2.
        assert_eq!(shine.draws.len(), 6, "{team}");
        for draw in &shine.draws {
            let texture = shine.textures[draw.texture.expect("a texture")]
                .as_ref()
                .expect("a decoded texture");
            println!(
                "{team}: shine draw {:?} under {}",
                draw.range, texture.label
            );
            // `envtest4bit.tga` on seven teams, `envmap_stripe2.tga` on
            // AG_Systems: each is whatever the hull's own material names.
            assert!(
                texture.label.contains("env"),
                "{team}: a shine draw is under {}, not an environment map",
                texture.label
            );
            // The same vertices the hull's own draw of that batch uses.
            let hull = &livery.hull;
            assert!(
                hull.draws
                    .iter()
                    .chain(&hull.transparent_draws)
                    .chain(&hull.alpha_tested_draws)
                    .any(|own| own.range == draw.range),
                "{team}: a shine draw has no batch of the hull behind it"
            );
        }
        // Where a flap's own vertices carry an extra-pass batch, `shine::write`
        // swings it with the hull's base draw: the original draws both under the
        // flap's one node matrix. Every team's flap carries exactly one - and Feisar's
        // and Triakis's flap is two meshes (`AirBrake_*Shape` and the
        // `underbrake_flash*Shape` after it), which the span must cover whole.
        for (side, flap) in shine.airbrakes.iter().enumerate() {
            let flap = flap
                .as_ref()
                .unwrap_or_else(|| panic!("{team}: the shine model lost airbrake {side}"));
            let on_flap = shine
                .draws
                .iter()
                .filter(|d| {
                    let indices = &shine.indices[d.range.start as usize..d.range.end as usize];
                    !indices.is_empty() && indices.iter().all(|i| flap.vertices.contains(i))
                })
                .count();
            println!("{team}: {on_flap} shine batch(es) on airbrake {side}");
            assert_eq!(on_flap, 1, "{team}: airbrake {side}'s own batch");
        }
        // The hull itself is untouched: its draws name their own textures.
        assert!(
            livery
                .hull
                .draws
                .iter()
                .chain(&livery.hull.transparent_draws)
                .all(|d| d.texture.is_none_or(|t| livery.hull.textures[t]
                    .as_ref()
                    .is_none_or(|t| !t.label.contains("env")))),
            "{team}: the hull's own pass was handed the env texture"
        );
    }
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn a_context_that_does_not_ask_for_the_pass_builds_none() {
    let Some(liveries) = load(false) else { return };
    assert!(liveries.iter().all(|l| l.shine.is_none()));
}

fn grid_frame(
    image: &std::path::Path,
    scratch: &std::path::Path,
    name: &str,
    flag: &[&str],
) -> Vec<u8> {
    let out = scratch.join(format!("{name}.png"));
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(image)
        .args(["--race", "--no-audio", "--size", "480x272"])
        .args(["--render-scale", "100", "--msaa", "off"])
        .args(["--screen-filter", "off", "--anisotropy", "off"])
        .args(["--motion-blur", "off", "--ticks", "1"])
        .args(flag)
        .arg("--screenshot")
        .arg(&out)
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"))
        .status()
        .expect("running oag-game");
    assert!(status.success(), "oag-game {name}");
    std::fs::read(&out).expect("reading the screenshot")
}

/// The pass reaches the picture: the grid frame differs with it and without
/// it (`--no-hull-shine`). Dropping the pass from the draw, the loader or the
/// per-frame write leaves the two equal.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and a GPU adapter"]
fn the_extra_pass_changes_the_grid_frame() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let scratch = std::env::temp_dir().join(format!("oag-shine-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("creating the scratch directory");
    let with = grid_frame(&image, &scratch, "with", &[]);
    let without = grid_frame(&image, &scratch, "without", &["--no-hull-shine"]);
    std::fs::remove_dir_all(&scratch).ok();
    assert_ne!(with, without, "the hull's extra pass drew nothing");
}

/// A circuit's own pass reaches the frame: the grid frame of `07_Track`,
/// whose tunnel rims and walls carry chrome-mapped batches in view from the
/// grid, differs with and without it (`--no-track-shine`).
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and a GPU adapter"]
fn the_circuits_extra_pass_changes_the_grid_frame() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let scratch = std::env::temp_dir().join(format!("oag-track-shine-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("creating the scratch directory");
    let track = ["--track", r"Data\Environments\07_Track\track.vex"];
    let with = grid_frame(&image, &scratch, "with", &track);
    let without = grid_frame(
        &image,
        &scratch,
        "without",
        &["--track", track[1], "--no-track-shine"],
    );
    std::fs::remove_dir_all(&scratch).ok();
    assert_ne!(with, without, "the circuit's extra pass drew nothing");
}
