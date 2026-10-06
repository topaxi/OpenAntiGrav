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
//! [`oag_assets::Layout`], which finds a source's bulk archive by name rather
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
//! Textures used to be a place a PS2 race was knowingly worse; the model-to-*set*
//! lookup (which archive entry holds a model's textures) is found by directory
//! position for both a ship and a track, and the *within-set* lookup (which of
//! that set's entries a given material means) is resolved by each `Texture`
//! node's own declared name - see `docs/formats/ps2-texture.md`. Both halves are
//! exact now: every slot on every one of the 32 track models decodes, and
//! [`every_ps2_track_s_texture_set_is_found_by_directory_position`] asserts it
//! stays that way. The load report still names any model whose slots did not
//! all fill rather than guessing at one, for whichever half regresses.

use std::path::PathBuf;

use oag_gameplay::input::Button;
use oag_pulse as pulse;

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

/// The PS2 release, Europe-only, `SCES-54748`.
const PS2_IMAGE: &str = "pulse-ps2-eu.chd";

/// The PSP release this project's reference scenario was captured on.
const PSP_IMAGE: &str = "pulse-psp-usa.chd";

/// One second at the fixed 60 Hz. Short on purpose: this file is about whether a
/// race *loads and steps* off a second source, and `race_ground_truth.rs` is where
/// how it flies is measured.
const TICKS: u32 = 60;

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

/// A race off `name`, with the defaults, and its report printed.
fn load(name: &str) -> Option<race::Loaded> {
    let image = image(name)?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
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
        let layout = oag_assets::Layout::resolve(&image.display().to_string(), oag_pulse::TITLE)
            .expect("a PSP layout");
        println!("{}", layout.describe());
        assert_eq!(layout.platform, oag_assets::Platform::Psp);
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
        let layout = oag_assets::Layout::resolve(&image.display().to_string(), oag_pulse::TITLE)
            .expect("a PS2 layout");
        println!("{}", layout.describe());
        assert_eq!(layout.platform, oag_assets::Platform::Ps2);
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
        !loaded.liveries[0].hull.indices.is_empty(),
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

    let mut held = race::HeldButtons::new(Button::Cross.bit());
    let mut grounded_ticks = 0u32;
    for tick in 0..TICKS {
        let snapshot = held.snapshot();
        race.tick(&PlayerInputs::single(snapshot));
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

/// Both exhaust textures decode off **both** discs, under one declared name.
///
/// The PS2 keeps them as `.pct` where the PSP keeps them as `.mip`, and the
/// executables name the same literal on both - so the rewrite in
/// [`oag_pulse::ps2_texture_name`] is the whole of the difference. Before it,
/// the PS2 trail and flare both fell back to a procedural glow and the load
/// report said `not in the archive set`, which is what this asserts is gone.
///
/// The dimensions are asserted, not just the fact of a decode: 64x64 and
/// 128x64 on both discs is a second, independent agreement that the `.pct`
/// entries really are the same two pictures, and `grabbedEngineFlare128x64x8`
/// says its own shape in its name.
///
/// **What this does not assert** is that the pixels match. The two builds
/// palette their textures differently (see `oag_texture::ps2_texture` on the
/// GS's 0-128 alpha), and comparing shipped art across the two releases is the
/// kind of thing `docs/comparisons/pulse-psp-vs-ps2.md` keeps out of the
/// repository on purpose.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_exhaust_textures_decode_off_both_discs() {
    for (name, expected) in [(PS2_IMAGE, ".pct"), (PSP_IMAGE, ".mip")] {
        let Some(loaded) = load(name) else {
            continue;
        };
        for (declared, width, height) in [
            (race::NOISE_TEXTURE, 64, 64),
            (race::FLARE_TEXTURE, 128, 64),
        ] {
            let line = loaded
                .report
                .iter()
                .find(|line| line.starts_with(declared))
                .unwrap_or_else(|| panic!("{name}: the report says nothing about {declared}"));
            assert!(
                line.contains(&format!("{width}x{height} {expected}")),
                "{name}: expected {declared} to decode as a {width}x{height} \
                 {expected}; the report line was:\n{line}"
            );
        }
    }
}

/// Each pressing hands the HUD **its own** grid, and the two are not the same.
///
/// The HUD renderer used to be told nothing and start at `Space::PSP`, so the
/// PS2's layouts - authored in a 640x448 grid, `docs/ui/hud.md` - were drawn a
/// third oversized and pushed off the right and bottom edges. The same defect
/// on HD's 1920x1080 was four times worse and is pinned in
/// `crates/game/tests/hd_hud_ground_truth.rs`; this is the two-pressing half of
/// it, and it is the one a Pulse-only reader would have called a non-issue.
///
/// Structural, like everything else in this file: it asserts which grid each
/// source names, not a single coordinate out of either layout.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn each_pressing_hands_the_hud_its_own_coordinate_grid() {
    for (source, expected) in [
        (PS2_IMAGE, oag_display::space::Space::PS2),
        (PSP_IMAGE, oag_display::space::Space::PSP),
    ] {
        let Some(loaded) = load(source) else {
            continue;
        };
        assert_eq!(
            loaded.hud.space, expected,
            "{source} should draw its HUD in its own grid"
        );
    }
}

/// The loading screen's glow strip is on the PS2 disc too, under the same rule.
///
/// Found while fixing the exhaust textures and fixed in the same change: this
/// entry was read with `read_name`, so on a PS2 source it missed and the wave
/// sampled `GlowStrip::placeholder` instead. `WADS2.WAD` entry 138 is the real
/// one, and `oag_render` decodes it now that it knows the PS2 format.
///
/// Asserted through [`oag_pulse::read_image`] and the decoder rather than
/// through `oag_game::loading::Assets`, because that path degrades to the
/// placeholder by design - a test that went through it would pass either way.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_loading_glow_strip_decodes_off_both_discs() {
    for source in [PS2_IMAGE, PSP_IMAGE] {
        let Some(image) = image(source) else {
            continue;
        };
        let mut archives =
            pulse::open(&image.display().to_string()).expect("the disc opens as Pulse");
        let blob = oag_pulse::read_image(&mut archives, pulse::loading::GLOW_STRIP_ENTRY)
            .unwrap_or_else(|e| {
                panic!(
                    "{source}: {} is not reachable ({e})",
                    pulse::loading::GLOW_STRIP_ENTRY
                )
            });
        let strip = oag_render::loading::GlowStrip::decode(&blob).unwrap_or_else(|e| {
            panic!(
                "{source}: {} does not decode ({e:#})",
                pulse::loading::GLOW_STRIP_ENTRY
            )
        });
        println!("{source}: glow strip {}x{}", strip.width, strip.height);
        assert!(
            strip.width > 0 && strip.height > 0,
            "{source}: the glow strip decoded to nothing"
        );
    }
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
/// [`oag_assets::Archives::read_preceding`] and
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
    //
    // **Twelve, not eleven.** The PS2 release bundles what the PSP sold as
    // four downloadable packs, and the twelfth was missed here for as long as
    // its id was unknown: the pack the packaging calls Mirage ships its ship
    // under `Data\Ships\Mantis`, so no spelling of `Mirage` ever resolved. See
    // `docs/formats/dlc-pack.md`.
    const TEAMS: [&str; 12] = [
        "AG_Systems",
        "Assegai",
        "Auricom",
        "EGX",
        "Feisar",
        "Goteki",
        "Harimau",
        "Icaras",
        "Mantis",
        "Piranha",
        "Qirex",
        "Triakis",
    ];

    for team in TEAMS {
        let loaded = race::load(&race::Options {
            source: image.display().to_string(),
            team: Some(team.to_string()),
            class: "VENOM".to_string(),
            ..race::Options::default()
        })
        .unwrap_or_else(|e| panic!("loading {team} off {PS2_IMAGE}: {e:#}"));

        let ship = &loaded.liveries[0].hull;
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

/// The archive-entry lookup is the same directory-position rule the ship uses,
/// checked independently for every circuit's own
/// `track.vex`/`track_reversed.vex` rather than extrapolated from the ship
/// result above - but finding the right *entry* is only half of it. 5 of the
/// 32 `<n>_Track` models have a nested set with fewer entries than the model
/// has `Texture` nodes, because two nodes there declare the same name and the
/// set holds one entry per unique name; see
/// `docs/formats/ps2-texture.md#the-original-never-suffers-this-collapse-it-resolves-every-texture-by-name-not-by-ordinal`.
/// Resolving each node by its own declared name rather than by position in
/// that set is what makes the duplicate harmless - a repeated name simply
/// resolves to the one entry it always named - so **every slot on every one
/// of the 32 models decodes**, no exceptions. Asserting the exact zero rather
/// than "no more than a handful" is deliberate: a regression that broke the
/// by-name lookup and fell back to position (or to nothing) would still pass
/// a looser bound.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_ps2_track_s_texture_set_is_found_by_directory_position() {
    let Some(image) = image(PS2_IMAGE) else {
        return;
    };

    for n in 1..=16u32 {
        for variant in ["track.vex", "track_reversed.vex"] {
            let track = format!(r"Data\Environments\{n:02}_Track\{variant}");
            let loaded = race::load(&race::Options {
                source: image.display().to_string(),
                track: Some(track.clone()),
                class: "VENOM".to_string(),
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
            assert_eq!(
                decoded,
                model.textures.len(),
                "{track}: only {decoded} of {} texture slot(s) decoded; by-name \
                 resolution should fill every slot now, including the ones a \
                 duplicate Texture node name used to leave unbound; report was:\n{}",
                model.textures.len(),
                loaded.report.join("\n")
            );
        }
    }
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
        // The PS2 disc bundles what the PSP sold as downloadable content, so
        // this one has a roster of twelve with nothing mounted.
        dlc: Vec::new(),
        leg: oag_ui::frontend::Leg::LogoFmv,
        movie: Some(oag_game::boot::DEFAULT_BOOT_MOVIE.to_string()),
        cache: oag_source::cache::default_cache_dir(),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(1),
        // No transcode: this is about what is found, not about ffmpeg.
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
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

/// **A transcode reports its frame counter while it runs**, which is what the
/// loading screen draws over the minute `ffmpeg` takes on `INTRO512.PSS`.
///
/// A transcode scratch directory this test does not share with anything.
///
/// **Both transcode tests in this file open the same `INTRO512.PSS` with the
/// same cache key and `refresh: true`, so one shared directory has them writing
/// and reading one set of files at the same time.** `nextest` runs them as
/// parallel processes, so the race is reachable in a single ordinary
/// `just test-data` with nothing else on the machine; a second run in another
/// worktree collides with both on top of that.
///
/// That is the mechanism behind the two symptoms `HANDOVER.md` recorded for
/// `an_uncapped_transcode_still_reports_a_total_to_divide_by` across three
/// separate days: `movie.frames == None`, the output clobbered mid-write, and
/// `ffprobe: Invalid data found`, the `.pss` read while the other test was
/// still extracting it. The failure correlated with machine load because load
/// changes the interleaving - the quick capped test stops reliably finishing
/// its extraction before the slow uncapped one reaches the same files - not
/// because anything was starving.
///
/// `CARGO_TARGET_TMPDIR` rather than `std::env::temp_dir()`: Cargo hands each
/// package its own directory under that worktree's `target/`, which makes the
/// path per-worktree for free and keeps this off the 32 GiB `/tmp` tmpfs, where
/// a 63 MiB reel per run is not free. The per-test leaf is what closes the race
/// *within* one run.
fn transcode_scratch(test: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join("ps2-progress-ground-truth")
        .join(test)
}

/// The plumbing under this is easy to break silently and impossible to notice
/// from a unit test: `movie::Watch` is threaded through five functions, the
/// counts come out of `ffmpeg -progress pipe:1` in a format this workspace does
/// not control, and every parse failure there is deliberately silent (see
/// `ffmpeg_progress` - a build of `ffmpeg` that spelled its keys differently
/// would simply stop reporting, and every other test would still pass).
///
/// **The PS2's loose movies rather than the PSP's reel, deliberately.** A `.PSS`
/// always goes through the AV1 cache; a `.PMF` on a `native-video` build is
/// decoded by GStreamer and never transcodes at all, so the PSP disc would make
/// this test pass or skip depending on which features it was built with.
///
/// `refresh: true` is the point of the setup: the assertion is about what a
/// *conversion* reports, and any earlier run would otherwise turn it into a
/// `Step::Cached` that reports once and says nothing about `ffmpeg`.
///
/// Capped to a couple of dozen frames to stay quick. The **uncapped** case has a
/// test of its own next door and costs a minute, because that is the one that
/// was actually broken.
///
/// Scratch space comes from [`transcode_scratch`], not a shared `/tmp` path -
/// see that function for the race it exists to prevent.
#[test]
#[ignore = "needs data/images/ and ffmpeg"]
fn a_transcode_reports_its_frames_as_it_encodes_them() {
    const FRAMES: usize = 24;

    let Some(image) = image(PS2_IMAGE) else {
        return;
    };
    if !std::process::Command::new("ffmpeg")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
    {
        println!("skipping: ffmpeg is not on PATH");
        return;
    }

    let source = image.display().to_string();
    let Some((path, blob)) =
        oag_assets::read_loose_file(&source, &["DATA/MOVIES/INTRO512.PSS"]).expect("reading it")
    else {
        panic!("the PS2 disc's loose INTRO512.PSS was not found");
    };

    let seen = std::sync::Mutex::new(Vec::new());
    let watch = |step: oag_game::movie::Step| seen.lock().expect("the watch's lock").push(step);
    let movie = oag_game::movie::open(
        &blob,
        &format!("{}-{}", path.replace(['/', '\\'], "_"), blob.len()),
        &transcode_scratch("a_transcode_reports_its_frames_as_it_encodes_them"),
        oag_game::movie::Extent::Frames(FRAMES),
        oag_game::movie::Decode {
            no_video: false,
            refresh: true,
            // The PS2's loose movies have no platform decoder, so this changes
            // nothing here; stated rather than defaulted because the assertion
            // below is about a transcode.
            prefer_cache: true,
        },
        Some(&watch),
    )
    .expect("transcoding the PS2's loose intro");
    assert!(
        movie.frames.is_some(),
        "no picture, so nothing transcoded: {:?}",
        movie.no_picture_reason
    );

    let seen = seen.into_inner().expect("the watch's lock");
    let counts: Vec<usize> = seen
        .iter()
        .filter_map(|step| match step {
            oag_game::movie::Step::Transcoding { done, .. } => Some(*done),
            _ => None,
        })
        .collect();

    assert!(
        !counts.is_empty(),
        "the transcode reported nothing at all: {seen:?}"
    );
    assert_eq!(
        counts.first(),
        Some(&0),
        "the first report is made before ffmpeg is spawned, so a screen says \
         \"transcoding\" for the whole wait rather than from the first frame: {counts:?}"
    );
    assert!(
        counts.windows(2).all(|pair| pair[0] <= pair[1]),
        "the frame counter went backwards: {counts:?}"
    );
    assert_eq!(
        counts.last(),
        Some(&FRAMES),
        "the last report should be the frame count asked for: {counts:?}"
    );
    assert!(
        !seen
            .iter()
            .any(|step| matches!(step, oag_game::movie::Step::Cached)),
        "refresh: true must convert rather than read the cache: {seen:?}"
    );
}

/// **An uncapped transcode reports a total, so the bar can move through it.**
///
/// The regression this pins, and it shipped: `Extent::Whole` became a bare
/// `None` that meant both "do not pass `-frames:v`" and "there is no total", so
/// the loading screen got frame numbers with nothing to divide them by and its
/// bar stood still through the longest wait in the boot. The container's length
/// was known the whole time - `probe` reads it as duration times frame rate,
/// 38.0 s at 25/1 for `INTRO512.PSS`.
///
/// **This is the expensive one**, about a minute of `libaom` on this workspace,
/// and it is uncapped deliberately: capping it is what made the sibling test
/// above pass while the real path was broken. The cheap half of the same
/// coverage is `an_uncapped_conversion_keeps_the_count_it_was_measured_at`, a
/// unit test on `Frames::plan`, which runs in CI where this never does.
#[test]
#[ignore = "needs data/images/ and ffmpeg; transcodes the whole reel, about a minute"]
fn an_uncapped_transcode_still_reports_a_total_to_divide_by() {
    let Some(image) = image(PS2_IMAGE) else {
        return;
    };
    if !std::process::Command::new("ffmpeg")
        .arg("-version")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
    {
        println!("skipping: ffmpeg is not on PATH");
        return;
    }

    let source = image.display().to_string();
    let Some((path, blob)) =
        oag_assets::read_loose_file(&source, &["DATA/MOVIES/INTRO512.PSS"]).expect("reading it")
    else {
        panic!("the PS2 disc's loose INTRO512.PSS was not found");
    };

    // **A prefix of the reel, not the whole of it.** An MPEG-2 program stream is
    // decodable from any pack boundary onward and ends cleanly where it is cut,
    // so the first quarter (in whole 2048-byte sectors) is a real, shorter
    // movie that still takes the uncapped `Extent::Whole` path - the one that
    // was broken. The whole reel was 81 s alone and 206 s under load; a quarter
    // is a fifth of that, and the assertions below are unchanged.
    let blob = &blob[..blob.len() / 4 / 2048 * 2048];

    let seen = std::sync::Mutex::new(Vec::new());
    let watch = |step: oag_game::movie::Step| seen.lock().expect("the watch's lock").push(step);
    let movie = oag_game::movie::open(
        blob,
        &format!("{}-{}", path.replace(['/', '\\'], "_"), blob.len()),
        &transcode_scratch("an_uncapped_transcode_still_reports_a_total_to_divide_by"),
        // The whole reel: the case that was broken.
        oag_game::movie::Extent::Whole,
        oag_game::movie::Decode {
            no_video: false,
            refresh: true,
            // The PS2's loose movies have no platform decoder, so this changes
            // nothing here; stated rather than defaulted because the assertion
            // below is about a transcode.
            prefer_cache: true,
        },
        Some(&watch),
    )
    .expect("transcoding the PS2's loose intro");

    let seen = seen.into_inner().expect("the watch's lock");
    let totals: Vec<Option<usize>> = seen
        .iter()
        .filter_map(|step| match step {
            oag_game::movie::Step::Transcoding { total, .. } => Some(*total),
            _ => None,
        })
        .collect();

    assert!(
        !totals.is_empty(),
        "the transcode reported nothing at all: {seen:?}"
    );
    assert!(
        totals.iter().all(Option::is_some),
        "an uncapped transcode reported no total, so a bar drawn from it cannot \
         move: {totals:?}"
    );
    // Not asserted as an exact figure: `probe` derives it from a duration, so it
    // is an estimate of what the encode produces, and the bar clamps rather than
    // depending on it being right. What matters is that it is in the right
    // order of magnitude rather than, say, the byte length.
    let total = totals[0].expect("checked above");
    let produced = movie.frames.as_ref().expect("a picture").len;
    assert!(
        total.abs_diff(produced) * 20 <= produced,
        "the stated total {total} is not within 5% of the {produced} frames \
         actually encoded, so it is measuring the wrong thing"
    );
}
