//! Race Remix: a track from one title, a craft from another - and the
//! regression this must never become, a mixed load that silently drifts from
//! today's single-title one.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run --release -p oag-game \
//!     --run-ignored all -E 'binary(race_remix_ground_truth)'
//! ```
//!
//! Two properties, each its own test:
//!
//! 1. **No-op regression.** `craft_source: None` and `craft_source:
//!    Some(<source>)` (the same path, spelled out) must load the same race -
//!    same track geometry, same grid, same HUD - for every title that races
//!    today. This is what makes `crate::remix::Remix::Single` a true no-op
//!    rather than an accidental behaviour change dressed as a refactor.
//! 2. **A genuine remix.** Wipeout 2048's own circuit with Wipeout Pure's
//!    craft: the track loads off 2048, the grid and the HUD off Pure. This is
//!    the spec's own example (see `race_remix_prompt.md`, not committed).
//! 3. **The RACE REMIX menu page's own data source**, `remix::catalogue`,
//!    independent of the menus themselves - which need a window this suite
//!    cannot open.

use oag_game::{race, remix};
use std::path::{Path, PathBuf};

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn image(name: &str) -> Option<PathBuf> {
    let path = root().join(name);
    if path.exists() {
        Some(path)
    } else {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} is missing",
            path.display()
        );
        println!("skipping: {} not present", path.display());
        None
    }
}

fn load(options: race::Options) -> race::Loaded {
    let source = options.source.clone();
    race::load(&options).unwrap_or_else(|e| panic!("loading {source}: {e:#}"))
}

/// The measurements a no-op regression check compares: cheap to take, and
/// each one is a different layer of the loader, so a drift in any of them
/// names itself rather than reading as "the numbers differ somehow".
struct Shape {
    track_triangles: usize,
    hud_layout: Option<String>,
    grid: Vec<String>,
}

fn shape(loaded: &race::Loaded) -> Shape {
    Shape {
        track_triangles: loaded.track_model.indices.len(),
        hud_layout: loaded
            .hud
            .layout
            .as_ref()
            .map(|layout| format!("{layout:?}").chars().take(40).collect()),
        grid: loaded
            .liveries
            .iter()
            .map(|livery| format!("{:?}", livery.hull.indices.len()))
            .collect(),
    }
}

/// **The no-op regression.** For every title that races today, naming its own
/// source as `craft_source` too must change nothing - `crate::remix::Remix`
/// falls back to `Single` whenever the two paths are equal, and this is the
/// test that would fail if that fallback ever stopped being exact.
#[test]
#[ignore = "needs disc images in data/images/"]
fn naming_the_same_source_as_craft_source_changes_nothing() {
    let sources = [
        ("Pulse", "data/images/pulse-psp-eu.chd"),
        ("HD", "data/images/hdfury-ps3-eu-dec.iso"),
    ];
    for (label, path) in sources {
        let Some(image) = image(path) else { continue };
        let source = image.display().to_string();
        let plain = load(race::Options {
            source: source.clone(),
            ..race::Options::default()
        });
        let remixed = load(race::Options {
            source: source.clone(),
            craft_source: Some(source.clone()),
            ..race::Options::default()
        });
        assert_eq!(
            shape(&plain).track_triangles,
            shape(&remixed).track_triangles,
            "{label}: naming craft_source as the same source moved the track"
        );
        assert_eq!(
            shape(&plain).grid,
            shape(&remixed).grid,
            "{label}: naming craft_source as the same source moved the grid"
        );
        assert_eq!(
            shape(&plain).hud_layout,
            shape(&remixed).hud_layout,
            "{label}: naming craft_source as the same source moved the HUD"
        );
        assert!(
            !plain
                .report
                .iter()
                .any(|line| line.starts_with("craft from")),
            "{label}: a single-source race must not report a craft source"
        );
        assert!(
            !remixed
                .report
                .iter()
                .any(|line| line.starts_with("craft from")),
            "{label}: craft_source equal to source is still not a remix"
        );
    }
}

/// **A genuine remix**, and the spec's own example: a Wipeout 2048 circuit
/// with a Wipeout Pure craft. The track is 2048's own Altima; the grid, the
/// livery and the HUD are Pure's.
#[test]
#[ignore = "needs the extracted Vita package and a Pure disc image"]
fn a_2048_track_races_with_a_pure_craft() {
    let track_source = root().join("data/extracted/vita/PCSF00007");
    if !track_source.join("base/PSP2/data.psarc").exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but the 2048 package is not extracted"
        );
        println!("skipping: 2048 package not extracted under data/extracted/vita/");
        return;
    }
    let Some(craft_source) = image("data/images/pure-psp-eu.chd") else {
        return;
    };

    let loaded = load(race::Options {
        source: track_source.display().to_string(),
        craft_source: Some(craft_source.display().to_string()),
        mode: oag_race::Mode::TimeTrial,
        ..race::Options::default()
    });

    assert!(
        loaded
            .report
            .iter()
            .any(|line| line == "racing on Wipeout 2048"),
        "expected the track's title in the report; was {:#?}",
        loaded.report
    );
    assert!(
        loaded
            .report
            .iter()
            .any(|line| line == "craft from Wipeout Pure"),
        "expected the craft's title in the report; was {:#?}",
        loaded.report
    );
    assert!(
        loaded.track_model.indices.len() > 3,
        "2048's Altima decoded to no geometry"
    );
    assert!(
        loaded.liveries[0].hull.indices.len() > 3,
        "Pure's default hull decoded to no geometry"
    );
    // Pure's own default team, not 2048's - the roster followed the craft.
    assert!(
        loaded
            .report
            .iter()
            .any(|line| line.contains("grid liveries:") && line.contains(oag_pure::TITLE.race.team)),
        "expected Pure's roster, not 2048's; was {:#?}",
        loaded.report
    );
    // Pure's HUD layout entry, not 2048's - the HUD followed the craft.
    assert!(
        loaded
            .report
            .iter()
            .any(|line| line.starts_with("HUD ") && line.contains("_HUD.xml")),
        "expected a Pure-shaped HUD entry name; was {:#?}",
        loaded.report
    );
}

/// `remix::catalogue` - the RACE REMIX page's own reader - offers HD's real
/// circuits and roster, independent of the menu machinery that calls it.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_catalogue_offers_a_titles_real_tracks_and_roster() {
    let Some(image) = image("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let catalogue = remix::catalogue(&image.display().to_string())
        .unwrap_or_else(|e| panic!("cataloguing {}: {e:#}", image.display()));

    assert!(!catalogue.tracks.is_empty(), "HD declares circuits");
    assert!(!catalogue.teams.is_empty(), "HD declares a roster");
    // `Track::id` is the plugin's own race id (`16_Track`-shaped), not the
    // archive path `oag_hd::race::DEFAULT_TRACK` names - `Catalogue::track`
    // is exercised by id via whichever the disc's own first entry is
    // instead.
    let (first_track, _) = &catalogue.tracks[0];
    assert!(
        catalogue.track(&first_track.id).is_some(),
        "the catalogue should find its own first circuit by id"
    );
    // Same reasoning as the track above: `oag_hd::race::DEFAULT_TEAM` is
    // lowercased for archive lookups, which a PSARC case-folds onto anyway -
    // the plugin definition's own spelling, which is what `Team::id` reads,
    // need not agree. See `oag_title::RaceDefaults::team`'s own doc comment.
    assert!(
        catalogue.team(&catalogue.teams[0].value).is_some(),
        "the catalogue should find its own first team by id"
    );
    // Labels are the string table where it has one, never the bare id when
    // the table names something different - the same rule `Team::label`
    // documents. A team's label falling back to its id is not itself wrong
    // (a title can genuinely leave one out), but every label should at least
    // be non-empty.
    assert!(
        catalogue.teams.iter().all(|team| !team.label.is_empty()),
        "every team choice should carry a label"
    );
    // **The regression this test exists for.** HD's own string table is not
    // keyed by a track's plugin id (`01_Track`), so a direct
    // `strings.get(&track.id)` resolves nothing and every label fell back to
    // the raw id on this page until `crate::boot::load_circuit_names` was
    // wired into `remix::catalogue` - see its own doc comment. Talon's
    // Junction is this project's reference circuit and is always present.
    let talons_junction = catalogue
        .tracks
        .iter()
        .find(|(track, _)| track.id == "17_Track")
        .unwrap_or_else(|| {
            panic!(
                "HD's circuit list has no 17_Track; was {:#?}",
                catalogue.tracks
            )
        });
    assert_eq!(
        talons_junction.1.to_uppercase(),
        "TALON'S JUNCTION",
        "17_Track's label should be its real name, not the id"
    );
    assert!(
        catalogue
            .tracks
            .iter()
            .all(|(track, label)| label != &track.id),
        "no track's label should be its own bare id; was {:#?}",
        catalogue.tracks
    );
}
