//! Boots a race off the **PS2** disc, and off the PSP disc through the same code.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project
//! does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all ps2_source
//! ```
//!
//! # What this is for
//!
//! `oag-game` used to spell the PSP's archive layout out, `PSP_GAME/USRDIR/Data.wad`,
//! and so refused the PS2 disc outright, even though every decoder underneath it
//! already handled both. What replaced that is
//! [`oag_assets::pulse::Layout`], which finds a source's bulk archive by name rather
//! than by platform, and the claim this file exists to check is a narrow one:
//!
//! **The archive layout was the whole of the difference.** Not the entry names, not
//! the handling schema, not the mesh or collision decoders. So the same
//! [`race::load`] call with the same [`race::DEFAULT_TRACK`] and
//! [`race::DEFAULT_TEAM`] should come back with a driveable race off either disc,
//! and [`both_sources_resolve_to_their_own_archives`] is what says the two really did
//! take different paths to get there rather than one of them quietly reading the
//! other's.
//!
//! # What it deliberately does not assert
//!
//! Nothing about *values*. Whether the two releases ship the same handling numbers or
//! the same spline is an open question that `docs/comparisons/pulse-psp-vs-ps2.md`
//! keeps open on purpose, and answering it here would mean putting a comparison of
//! shipped design data in the repository. Every assertion below is structural: a
//! track has paths, a hull has positive extents, a position is finite.
//!
//! Textures used to be a place a PS2 race was knowingly worse; the model-to-set
//! lookup is now recovered for both a ship and a track (directory position, not a
//! name - see `docs/formats/ps2-texture.md`), and
//! [`every_ps2_track_s_texture_set_is_found_by_directory_position`] asserts it stays
//! that way for tracks specifically. A handful of slots on 5 of 32 tracks still do
//! not decode (documented, not a regression to fix here), and the load report names
//! any model whose slots did not all fill rather than guessing at one.

use std::path::{Path, PathBuf};

use oag_assets::pulse;
use oag_gameplay::input::button;
use oag_physics::SpeedClass;

use oag_game::race;

/// The PS2 release, Europe-only, `SCES-54748`.
const PS2_IMAGE: &str = "pulse-ps2-eu.chd";

/// The PSP release this project's reference scenario was captured on.
const PSP_IMAGE: &str = "pulse-psp-usa.chd";

/// One second at the fixed 60 Hz. Short on purpose: this file is about whether a
/// race *loads and steps* off a second source, and `race_ground_truth.rs` is where
/// how it flies is measured.
const TICKS: u32 = 60;

fn image(name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images")
        .join(name);

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

/// A race off `name`, with the defaults, and its report printed.
fn load(name: &str) -> Option<race::Loaded> {
    let image = image(name)?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: SpeedClass::Venom,
        ..race::Options::default()
    })
    .unwrap_or_else(|e| panic!("loading a race off {name}: {e:#}"));
    println!("--- {name}");
    for line in &loaded.report {
        println!("{line}");
    }
    Some(loaded)
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn both_sources_resolve_to_their_own_archives() {
    if let Some(image) = image(PSP_IMAGE) {
        let layout = pulse::Layout::resolve(&image.display().to_string()).expect("a PSP layout");
        println!("{}", layout.describe());
        assert_eq!(layout.platform, pulse::Platform::Psp);
        assert!(
            layout.data.ends_with(pulse::archives::DATA),
            "the PSP disc resolved to {}",
            layout.data
        );
        assert_eq!(
            layout
                .fe
                .as_ref()
                .map(|fe| fe.ends_with(pulse::archives::FE)),
            Some(true),
            "the PSP disc's companion archive resolved to {:?}",
            layout.fe
        );
    }

    if let Some(image) = image(PS2_IMAGE) {
        let layout = pulse::Layout::resolve(&image.display().to_string()).expect("a PS2 layout");
        println!("{}", layout.describe());
        assert_eq!(layout.platform, pulse::Platform::Ps2);
        // Found by name, so nothing here may know the serial directory: what is
        // asserted is the archive's own name and that something preceded it.
        assert!(
            layout.data.ends_with(pulse::archives::ps2::DATA),
            "the PS2 disc resolved to {}",
            layout.data
        );
        assert_eq!(
            layout
                .fe
                .as_ref()
                .map(|fe| fe.ends_with(pulse::archives::ps2::FE)),
            Some(true),
            "the PS2 disc's companion archive resolved to {:?}",
            layout.fe
        );
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_ps2_disc_loads_a_driveable_race() {
    let Some(loaded) = load(PS2_IMAGE) else {
        return;
    };

    assert!(
        !loaded.setup.ai.paths.is_empty(),
        "the PS2 track decoded no spline paths"
    );
    assert!(
        !loaded.setup.spline.is_empty(),
        "the PS2 track resampled to nothing"
    );
    assert!(
        !loaded.setup.collision.colliders().is_empty(),
        "the PS2 track has no collision geometry, so a ship has nothing to hover on"
    );
    // Asserted rather than left to the spawn, because a PS2 track with no
    // `Start Position` node falls back to the spline *silently* - the race still
    // loads, the ship still flies, and the fallback would look exactly like the
    // recovered path from outside. The PS2 authors the same node as the PSP: it
    // reads byte-identical on `16_Track`, which is also the corroboration that
    // the two discs ship one set of authored track data.
    assert!(
        loaded.setup.start_position.is_some(),
        "the PS2 track carries no Start Position node, so the race is quietly spawning \
         on the spline instead"
    );
    assert!(
        !loaded.ship_model.indices.is_empty(),
        "the PS2 ship model decoded no triangles"
    );

    // The hull the wall constraint and the inertia tensor are both built from.
    let dimensions = loaded.setup.handling.dimensions;
    assert!(
        dimensions.width > 0.0 && dimensions.height > 0.0 && dimensions.length > 0.0,
        "the PS2 handling stats give a degenerate hull: {dimensions:?}"
    );
    assert!(
        loaded.setup.handling.physical.mass > 0.0,
        "the PS2 handling stats give a massless ship"
    );
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_ship_spawns_and_steps_on_the_ps2_disc() {
    let Some(loaded) = load(PS2_IMAGE) else {
        return;
    };
    let bound = loaded.setup.spline.max_half_width() * 2.0;
    let handling = loaded.setup.handling;
    let mut race = race::Race::start(loaded.setup);

    let start = race.telemetry();
    assert!(start.position.is_finite(), "{:?}", start.position);
    assert!(
        (start.height_above_spline - race::spawn_height(&handling)).abs() < 0.01,
        "spawned {} above the spline, wanted {}",
        start.height_above_spline,
        race::spawn_height(&handling)
    );
    assert!(
        race.ship().physics.body.up().y > 0.5,
        "the ship is not the right way up: {}",
        race.ship().physics.body.up()
    );

    let mut held = race::HeldButtons::new(1 << button::CROSS);
    let mut grounded_ticks = 0u32;
    for tick in 0..TICKS {
        let snapshot = held.snapshot();
        race.tick(&snapshot);
        let telemetry = race.telemetry();
        assert!(
            telemetry.position.is_finite(),
            "tick {tick}: {}",
            race::describe(&telemetry)
        );
        assert!(
            telemetry.spline_distance < bound,
            "tick {tick}: {bound:.1} units off the spline is off the track: {}",
            race::describe(&telemetry)
        );
        if race.ship().physics.grounded > 0.0 {
            grounded_ticks += 1;
        }
    }

    let end = race.telemetry();
    println!("after {TICKS} tick(s): {}", race::describe(&end));
    println!("grounded on {grounded_ticks} of {TICKS} tick(s)");

    // The probes finding the track at all is the composition claim: the spline
    // decode and the collision decode agree about where the surface is. How well it
    // then flies is `race_ground_truth.rs`'s subject, and the engine gap recorded
    // there applies to both discs equally.
    assert!(
        grounded_ticks > 0,
        "no probe found the PS2 track's collision geometry on any tick"
    );
    assert!(
        end.position != start.position,
        "the ship never moved under thrust"
    );
}

/// The PS2 ship declares texture slots that do not fill, and the report says so.
///
/// This asserts the *gap is reported*, not that it is closed. Closing it means
/// recovering the model-to-texture-set lookup, at which point this test should be
/// replaced by one that asserts a skin.
///
/// The PS2 model-to-texture-set lookup, for ships: the archive entry directly
/// before a `Ship.vex` decodes as its texture set, with exactly as many
/// entries as the model has `Texture` nodes. See
/// [`oag_assets::pulse::Archives::read_preceding`] and
/// `docs/formats/ps2-texture.md`.
///
/// Checked against **every** team, not just the default: a wrong or
/// coincidental adjacency would show up as one team fitting and its neighbour
/// not, since the rule is purely positional and every ship sits next to a
/// different texture set. The same rule, checked separately, also holds for
/// track models - see
/// [`every_ps2_track_s_texture_set_is_found_by_directory_position`].
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_ps2_ship_s_texture_set_is_found_by_directory_position() {
    let Some(image) = image(PS2_IMAGE) else {
        return;
    };

    // The full roster, not just `race::DEFAULT_TEAM` - the point of this test
    // is that the rule holds for each team independently, not for one team by
    // coincidence.
    const TEAMS: [&str; 11] = [
        "AG_Systems",
        "Assegai",
        "Auricom",
        "EGX",
        "Feisar",
        "Goteki",
        "Harimau",
        "Icaras",
        "Piranha",
        "Qirex",
        "Triakis",
    ];

    for team in TEAMS {
        let loaded = race::load(&race::Options {
            source: image.display().to_string(),
            team: team.to_string(),
            class: SpeedClass::Venom,
            ..race::Options::default()
        })
        .unwrap_or_else(|e| panic!("loading {team} off {PS2_IMAGE}: {e:#}"));

        let ship = &loaded.ship_model;
        let decoded = ship.textures.iter().filter(|t| t.is_some()).count();
        assert!(
            !ship.textures.is_empty(),
            "{team}'s PS2 ship model declares no texture slots at all, which \
             contradicts docs/formats/ps2-texture.md's one-entry-per-Texture-node \
             reading"
        );
        assert_eq!(
            decoded,
            ship.textures.len(),
            "{team}: only {decoded} of {} texture slot(s) decoded; report was:\n{}",
            ship.textures.len(),
            loaded.report.join("\n")
        );
    }
}

/// The same directory-position rule, checked independently for every
/// circuit's own `track.vex`/`track_reversed.vex`, not extrapolated from the
/// ship result above.
///
/// 27 of the 32 `<n>_Track` models on the disc fill every slot; the other 5
/// are short 1 or 2 slots each (never more), for a total of exactly 7 missing
/// slots across all 32 - see
/// `docs/formats/ps2-texture.md#how-a-model-finds-its-texture-set-directory-position-not-a-name`.
/// Asserting the exact total rather than "no more than a handful" is
/// deliberate: a regression that broke the lookup entirely (falling back to
/// the white texture for every slot) would still pass a looser bound.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_ps2_track_s_texture_set_is_found_by_directory_position() {
    let Some(image) = image(PS2_IMAGE) else {
        return;
    };

    let mut total_missing = 0usize;

    for n in 1..=16u32 {
        for variant in ["track.vex", "track_reversed.vex"] {
            let track = format!(r"Data\Environments\{n:02}_Track\{variant}");
            let loaded = race::load(&race::Options {
                source: image.display().to_string(),
                track: track.clone(),
                class: SpeedClass::Venom,
                ..race::Options::default()
            })
            .unwrap_or_else(|e| panic!("loading {track} off {PS2_IMAGE}: {e:#}"));

            let model = &loaded.track_model;
            assert!(
                !model.textures.is_empty(),
                "{track}'s PS2 track model declares no texture slots at all, which \
                 contradicts docs/formats/ps2-texture.md's one-entry-per-Texture-node \
                 reading"
            );
            let decoded = model.textures.iter().filter(|t| t.is_some()).count();
            let missing = model.textures.len() - decoded;
            assert!(
                missing <= 2,
                "{track}: {missing} of {} texture slot(s) undecoded, more than the \
                 documented 0-2 slack; report was:\n{}",
                model.textures.len(),
                loaded.report.join("\n")
            );
            total_missing += missing;
        }
    }

    assert_eq!(
        total_missing, 7,
        "expected exactly 7 undecoded texture slots total across all 32 track models \
         (see docs/formats/ps2-texture.md), got {total_missing}"
    );
}

/// Whatever the PS2 front end can do, its failures name the archives searched.
///
/// **This assertion has been turned around once, and the history is the point.**
/// It used to require `loaded.movie.is_none()`: the PS2 ships no `.PMF` in any
/// archive, so the intro degraded to no picture at all. It does not any more -
/// both the intro (`INTRO512.PSS`) and the backdrop (`BG512.IPF`) are loose in
/// the ISO filesystem and both now decode, so what this checks is that the PS2
/// answers with a movie that came from *there* rather than from a WAD: no PSMF
/// header, real metadata even under `--no-video` (which does not transcode a
/// single frame), and the report naming the loose file. See
/// `docs/formats/ipf.md` and `docs/ps2/pulse-disc-layout.md`.
///
/// Whether it gets that far depends on whether the front-end XML is in
/// `WADS2.WAD` under the name the PSP uses, so the outer shape is still "either
/// boots, or says which archive it could not find the front-end root in".
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_ps2_front_end_either_boots_or_says_what_it_could_not_find() {
    let Some(image) = image(PS2_IMAGE) else {
        return;
    };

    let options = oag_game::boot::Options {
        // No saved language: these boot a fresh install every time.
        language: None,
        source: image.display().to_string(),
        leg: oag_game::frontend::Leg::LogoFmv,
        movie: oag_game::boot::DEFAULT_BOOT_MOVIE.to_string(),
        cache: oag_game::boot::default_cache_dir(),
        extent: oag_game::movie::Extent::Frames(1),
        // No transcode: this is about what is found, not about ffmpeg.
        no_video: true,
    };

    match oag_game::boot::load(&options) {
        Ok(loaded) => {
            for line in &loaded.report {
                println!("{line}");
            }
            let movie = loaded
                .movie
                .as_ref()
                .expect("the PS2 disc's loose INTRO512.PSS/INTRO640.PSS was not found");
            assert!(
                movie.header.is_none(),
                "the PS2 disc produced a PSMF reel, which contradicts \
                 docs/ps2/pulse-disc-layout.md"
            );
            assert!(
                loaded
                    .report
                    .iter()
                    .any(|line| line.contains("MPEG-2 program stream")),
                "the report does not name the loose intro it loaded; it was:\n{}",
                loaded.report.join("\n")
            );
            assert_eq!(
                movie.no_picture_reason.as_deref(),
                Some("--no-video was given"),
                "a movie loaded, but not for the reason --no-video should have caused"
            );
        }
        Err(e) => {
            let message = format!("{e:#}");
            println!("the PS2 front end did not boot: {message}");
            assert!(
                message.contains(pulse::archives::ps2::DATA),
                "the failure does not name the archive it searched: {message}"
            );
        }
    }
}
