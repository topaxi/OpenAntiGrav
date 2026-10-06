//! A craft spawns facing the way its circuit runs, on every shipped track file.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(spawn_heading_ground_truth)'
//! ```
//!
//! The PS3 image has to be layer-1 decrypted first - `scripts/ps3iso.py
//! decrypt`, `docs/formats/ps3-disc.md`.
//!
//! # What is being established
//!
//! A `Start Position` node carries a heading, and this project prefers it to
//! the spline for the reason `oag_gameplay::spawn` gives: it is a recovered
//! authored value where a tangent is this crate's own resampling. That is right
//! on both PSP discs and **wrong nine times on Wipeout HD**, where the node was
//! moved for a reversed circuit and never turned round - its rotation rows are
//! byte-identical to the forward file's - or, once, never authored at all and
//! left as a bare identity matrix. It is wrong **once on Pulse's PS2
//! pressing** too, which is the case that corroborates the fix rather than only
//! needing it: see
//! `the_ps2s_stale_slot_is_corrected_to_what_the_psps_own_file_authors`.
//!
//! So there are two claims here and they are deliberately separate:
//!
//! 1. **The outcome.** Every craft on every shipped circuit spawns pointing the
//!    way the track runs, which is the bug a player sees.
//! 2. **The census.** Exactly which files needed the correction, so this fails
//!    the day the count moves rather than passing for having silently corrected
//!    something new. A drift here means either the disc is different or the
//!    `Start Position` parse changed.
//!
//! The second is what makes the first more than a tautology: the rule under
//! test believes the spline whenever the two disagree, so claim 1 alone would
//! pass even if the authored heading were being thrown away everywhere.

use std::path::{Path, PathBuf};

use oag_core::math::Vec3;
use oag_raceplay as race;

/// How far a craft's heading may be off the track's own before this calls it
/// wrong: `cos 60 degrees`, the same band
/// `oag_raceplay::spawn::SLOT_AGREES_WITH_THE_SPLINE` splits.
const AGREES: f32 = 0.5;

/// Every circuit file on the HD disc: 16 forward and 12 reversed.
///
/// The same list `hd_trackwall_ground_truth.rs` walks, and it is a list rather
/// than a directory scan for the same reason: the reversed files sit in a
/// *different archive* from their forward twin on the eight `DATA02` circuits,
/// so a list built from one archive comes back 12 short and looks complete.
const HD_CIRCUITS: &[(&str, &str)] = &[
    ("amphiseum", "track"),
    ("modesto_heights", "track"),
    ("talons_junction", "track"),
    ("tech_de_ra", "track"),
    ("zone_1", "track"),
    ("zone_2", "track"),
    ("zone_3", "track"),
    ("zone_4", "track"),
    ("amphiseum", "track_reversed"),
    ("modesto_heights", "track_reversed"),
    ("talons_junction", "track_reversed"),
    ("tech_de_ra", "track_reversed"),
    ("01_vineta_k", "track"),
    ("02_track", "track"),
    ("03_track", "track"),
    ("04_chenghou_project", "track"),
    ("05_ubermall", "track"),
    ("10_sebenco_climb", "track"),
    ("12_sol_2", "track"),
    ("15_anulpha_pass", "track"),
    ("01_vineta_k", "track_reversed"),
    ("02_track", "track_reversed"),
    ("03_track", "track_reversed"),
    ("04_chenghou_project", "track_reversed"),
    ("05_ubermall", "track_reversed"),
    ("10_sebenco_climb", "track_reversed"),
    ("12_sol_2", "track_reversed"),
    ("15_anulpha_pass", "track_reversed"),
];

/// The nine HD circuit files whose authored slot disagrees with their own
/// spline, measured on `hdfury-ps3-eu-dec.iso`.
///
/// **Eight are reversed circuits and one is not**, which is why this build
/// corrects a *disagreement* rather than a `Reversed="True"` flag: `zone_3`
/// forward is as stale as `01_vineta_k` reversed. `tech_de_ra` reversed is the
/// perpendicular one - a bare identity matrix, a slot nobody authored.
const HD_STALE_SLOTS: &[(&str, &str)] = &[
    ("zone_3", "track"),
    ("modesto_heights", "track_reversed"),
    ("tech_de_ra", "track_reversed"),
    ("01_vineta_k", "track_reversed"),
    ("04_chenghou_project", "track_reversed"),
    ("05_ubermall", "track_reversed"),
    ("10_sebenco_climb", "track_reversed"),
    ("12_sol_2", "track_reversed"),
    ("15_anulpha_pass", "track_reversed"),
];

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

/// Loads one circuit and reports `(every craft's agreement with the track, the
/// authored slot's own agreement with it)`.
///
/// The second is read straight off the loaded setup rather than out of the
/// spawn, so the census below measures the *disc* and the assertion above
/// measures the *fix*, and neither can quietly become the other.
fn agreement(source: &Path, track: &str) -> (Vec<f32>, f32) {
    let loaded = race::load(&race::Options {
        source: source.display().to_string(),
        class: "VENOM".to_string(),
        track: Some(track.to_string()),
        opponents: true,
        ..race::Options::default()
    })
    .unwrap_or_else(|e| panic!("{track}: loading the race: {e:#}"));

    let slot = loaded.setup.start_position;
    let spline = loaded.setup.spline.clone();
    let race = race::Race::start(loaded.setup);

    let tangent_at = |position: Vec3| {
        spline
            .nearest(position)
            .and_then(|(_, sample, _)| Vec3::from_array(sample.tangent).try_normalize())
            .unwrap_or_else(|| panic!("{track}: the spline has no direction at {position:?}"))
    };

    let craft = race
        .sim
        .world
        .ships
        .iter()
        .filter(|ship| ship.active)
        .map(|ship| {
            let body = &ship.physics.body;
            let forward = body.forward();
            // **Level, on every craft on every circuit.** The bind handler
            // forces the slot's up row to world `(0, 1, 0)`, so
            // `oag_vex::track::start_position` drops the authored
            // forward's `y` and every authored heading is exactly horizontal.
            // A spline tangent is not, so the correction has to flatten what it
            // substitutes or the nine circuits it fires on would start in a
            // different frame from the other 43. Asserted here rather than
            // trusted, because the agreement figure below is blind to pitch.
            //
            // **Or, on Pulse PSP alone, it carries the track's own pitch**, where the
            // grid is laid in the track's frame (`oag_gameplay::orientation_on_sample`,
            // 2026-10-01): the original's eight placed craft read forward `y` of
            // `+0.0037` to `-0.0040`, the track's slope. Every other source keeps the
            // bind's level frame and the strict bound, so an accidental pitch on HD,
            // the PS2 or Pure still fails. The Pulse PSP tolerance is `0.01`, not
            // `0.001`, because the slot's own sample is not the nearest one to the
            // craft, and `09_Track` reversed climbs at `0.29`.
            let tangent = tangent_at(body.position);
            let on_the_tracks_frame = source.to_string_lossy().contains("pulse-psp");
            let level = forward.y.abs() < 1.0e-3;
            let on_the_track = (forward.y - tangent.y).abs() < 1.0e-2;
            assert!(
                level || (on_the_tracks_frame && on_the_track),
                "{track}: a craft spawned pitched by {} (the track's own is {}), and {}",
                forward.y,
                tangent.y,
                if on_the_tracks_frame {
                    "neither the bind's level frame nor the track's pitch explains it"
                } else {
                    "this source's frame is level"
                }
            );
            forward.dot(tangent)
        })
        .collect();

    let authored = slot.map_or(1.0, |slot| {
        let position = Vec3::from_array(slot.position);
        Vec3::from_array(slot.forward).dot(tangent_at(position))
    });

    (craft, authored)
}

/// Claim 1 on HD, and claim 2: every craft faces the right way, and exactly the
/// known files needed help to get there - over the circuit files of one
/// direction.
///
/// # Direction by slice: eight tests, asserting what one loop did
///
/// The per-craft claim is per circuit, so any partition of [`HD_CIRCUITS`]
/// asserts the same 28 things. The list claim is `assert_eq!` against
/// [`HD_STALE_SLOTS`] *in file order*: each test keeps the circuits of its
/// direction whose index within that direction is `slice` modulo [`SLICES`],
/// and filters [`HD_STALE_SLOTS`] to exactly those circuits, which keeps that
/// order and keeps the assertion exact rather than merely equivalent. The
/// slices cover every circuit once, so the union of the eight lists is the
/// whole of [`HD_STALE_SLOTS`].
///
/// Direction is also the axis the finding itself is about: eight of the nine
/// stale slots are `track_reversed`, which is the pattern the correction was
/// derived from, so the two tests read as the two halves of the claim rather
/// than as an arbitrary chunking.
///
/// The reason to split is wall clock. `cargo nextest` parallelises across
/// tests, one process each, so all 28 circuit files ran on a single core:
/// **147 s** on 2026-09-09, in a suite whose whole wall clock was 587 s. The
/// two halves run side by side at the same total CPU, and each half is now four
/// slices (**161 s** and **147 s** under load on 2026-10-06).
const SLICES: usize = 4;

fn hd_craft_spawn_facing_their_circuit(file_kind: &str, slice: usize) {
    let Some(image) = image("hdfury-ps3-eu-dec.iso") else {
        return;
    };

    let mut stale = Vec::new();
    let mut seen = 0usize;
    let mine: Vec<(&str, &str)> = HD_CIRCUITS
        .iter()
        .filter(|(_, f)| *f == file_kind)
        .skip(slice)
        .step_by(SLICES)
        .copied()
        .collect();
    for (circuit, file) in &mine {
        seen += 1;
        let track = format!("/data/environments/{circuit}/{file}.vex");
        let (craft, authored) = agreement(&image, &track);
        assert_eq!(craft.len(), 8, "{circuit}/{file} should field a full grid");
        for (slot, dot) in craft.iter().enumerate() {
            assert!(
                *dot > AGREES,
                "{circuit}/{file}: slot {slot} faces {dot:+.3} along its own track"
            );
        }
        if authored <= AGREES {
            stale.push((*circuit, *file));
        }
    }
    // A renamed file kind would otherwise leave this passing over an empty
    // list, with an empty expectation to match it.
    assert!(
        seen > 0,
        "no circuit in HD_CIRCUITS is a {file_kind} file, so this asserted nothing"
    );

    let expected: Vec<(&str, &str)> = HD_STALE_SLOTS
        .iter()
        .filter(|pair| mine.contains(pair))
        .copied()
        .collect();
    assert_eq!(
        stale, expected,
        "the disc's own stale {file_kind} slice {slice} slots, in file order"
    );
}

/// Slice 0 of the forward circuit files - see [`hd_craft_spawn_facing_their_circuit`].
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_forward_hd_craft_slice_0_spawns_facing_the_way_its_circuit_runs() {
    hd_craft_spawn_facing_their_circuit("track", 0);
}

/// Slice 1 of the forward circuit files - see [`hd_craft_spawn_facing_their_circuit`].
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_forward_hd_craft_slice_1_spawns_facing_the_way_its_circuit_runs() {
    hd_craft_spawn_facing_their_circuit("track", 1);
}

/// Slice 2 of the forward circuit files - see [`hd_craft_spawn_facing_their_circuit`].
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_forward_hd_craft_slice_2_spawns_facing_the_way_its_circuit_runs() {
    hd_craft_spawn_facing_their_circuit("track", 2);
}

/// Slice 3 of the forward circuit files - see [`hd_craft_spawn_facing_their_circuit`].
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_forward_hd_craft_slice_3_spawns_facing_the_way_its_circuit_runs() {
    hd_craft_spawn_facing_their_circuit("track", 3);
}

/// Slice 0 of the reversed circuit files - see [`hd_craft_spawn_facing_their_circuit`].
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_reversed_hd_craft_slice_0_spawns_facing_the_way_its_circuit_runs() {
    hd_craft_spawn_facing_their_circuit("track_reversed", 0);
}

/// Slice 1 of the reversed circuit files - see [`hd_craft_spawn_facing_their_circuit`].
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_reversed_hd_craft_slice_1_spawns_facing_the_way_its_circuit_runs() {
    hd_craft_spawn_facing_their_circuit("track_reversed", 1);
}

/// Slice 2 of the reversed circuit files - see [`hd_craft_spawn_facing_their_circuit`].
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_reversed_hd_craft_slice_2_spawns_facing_the_way_its_circuit_runs() {
    hd_craft_spawn_facing_their_circuit("track_reversed", 2);
}

/// Slice 3 of the reversed circuit files - see [`hd_craft_spawn_facing_their_circuit`].
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_reversed_hd_craft_slice_3_spawns_facing_the_way_its_circuit_runs() {
    hd_craft_spawn_facing_their_circuit("track_reversed", 3);
}

/// One non-HD source in hand, swept the same way as the HD pair above.
///
/// **Pulse's PS2 pressing carries a misauthored slot the PSP's does not**, and
/// the pair is what makes this more than a repair. `09_Track`'s slot has a
/// **byte-identical position** on the two discs - `(-506.0834, 2.1623526,
/// 242.45824)` on both - and a completely different rotation: the PSP's forward
/// row runs straight down the track and the PS2's is 116 degrees off it. Same
/// spline, same slot, one rotation maintained and one not.
///
/// So the PSP's file says what the PS2's should have said, and it can be
/// checked: the heading this correction substitutes on the PS2 file agrees with
/// the PSP file's **authored** one to within **0.08 degrees**. That is the
/// closest thing to ground truth this project has for the branch - a case where
/// the right answer exists in another pressing and was never used to derive it.
///
/// Pure and the PSP's own 24 need no correction at all, by between `+0.920` and
/// `+1.000`, so nothing on either PSP disc moves.
///
/// # One test per source
///
/// The three sources were one loop over 64 circuit files and measured **106 s**
/// on 2026-09-09. Each source's claim is entirely its own - its circuit count
/// and its own stale list - so a test per source asserts exactly what the loop
/// did, and the PS2's 32 files on a 3.7 GB image no longer hold up the two PSP
/// discs behind them.
fn a_source_is_swept_for_stale_slots(name: &str, count: usize, known_stale: &[&str]) {
    let Some(image) = image(name) else { return };

    let (shell, _, _) = oag_game::boot::load_shell(&oag_game::boot::Options {
        source: image.display().to_string(),
        language: None,
        dlc: Vec::new(),
        leg: oag_ui::frontend::Leg::LogoFmv,
        movie: None,
        cache: std::env::temp_dir().join("oag-spawn-heading-ground-truth"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(1),
        // Every assertion here is about a spawn; nothing pays for a transcode.
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
    })
    .unwrap_or_else(|e| panic!("{name}: opening the front end: {e:#}"));

    // Swept off the source's own plugin rather than a numbered guess: the PS2
    // pressing ships eight circuit files the PSP EU disc does not, and a
    // hardcoded list would have missed them and looked complete.
    assert_eq!(shell.tracks.len(), count, "{name}'s circuit count");

    let mut stale = Vec::new();
    for track in &shell.tracks {
        let entry = track.entry_name();
        let (craft, authored) = agreement(&image, &entry);
        for (slot, dot) in craft.iter().enumerate() {
            assert!(
                *dot > AGREES,
                "{name} {entry}: slot {slot} faces {dot:+.3} along its own track"
            );
        }
        if authored <= AGREES {
            stale.push(entry);
        }
    }
    let stale: Vec<&str> = stale.iter().map(String::as_str).collect();
    assert_eq!(stale, known_stale, "{name}'s own stale slots");
}

/// The PSP EU pressing's 24 circuit files, none of them stale - see
/// [`a_source_is_swept_for_stale_slots`].
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_psp_pulse_pressing_carries_no_stale_slot() {
    a_source_is_swept_for_stale_slots("pulse-psp-eu.chd", 24, &[]);
}

/// Pure's eight, none of them stale - see [`a_source_is_swept_for_stale_slots`].
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_pure_pressing_carries_no_stale_slot() {
    a_source_is_swept_for_stale_slots("pure-psp-eu.chd", 8, &[]);
}

/// The PS2 pressing's 32, one of which is the stale slot that corroborates the
/// whole correction - see [`a_source_is_swept_for_stale_slots`].
#[test]
#[ignore = "needs a disc image in data/images/"]
fn only_the_ps2_pressing_carries_a_stale_slot() {
    a_source_is_swept_for_stale_slots(
        "pulse-ps2-eu.chd",
        32,
        &[r"Data\Environments\09_Track\track.vex"],
    );
}

/// The PS2's stale slot and the PSP's maintained one are the same slot, and the
/// correction lands on the value the PSP file states.
///
/// The check the sweep above cannot make, because it never has two pressings of
/// one circuit side by side. Both claims are asserted rather than described:
/// that the position is identical, so this really is one slot exported twice,
/// and that the substituted heading matches the maintained rotation.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_ps2s_stale_slot_is_corrected_to_what_the_psps_own_file_authors() {
    let (Some(ps2), Some(psp)) = (image("pulse-ps2-eu.chd"), image("pulse-psp-eu.chd")) else {
        return;
    };
    let entry = r"Data\Environments\09_Track\track.vex";

    let slot_of = |image: &Path| {
        race::load(&race::Options {
            source: image.display().to_string(),
            class: "VENOM".to_string(),
            track: Some(entry.to_string()),
            ..race::Options::default()
        })
        .unwrap_or_else(|e| panic!("{}: {e:#}", image.display()))
    };

    let on_ps2 = slot_of(&ps2);
    let on_psp = slot_of(&psp);
    let ps2_slot = on_ps2
        .setup
        .start_position
        .expect("the PS2 file has a slot");
    let psp_slot = on_psp
        .setup
        .start_position
        .expect("the PSP file has a slot");

    assert_eq!(
        ps2_slot.position, psp_slot.position,
        "one slot exported twice: the position is the same to the last bit"
    );
    assert!(
        Vec3::from_array(ps2_slot.forward).dot(Vec3::from_array(psp_slot.forward)) < AGREES,
        "and the rotation is not: {:?} against {:?}",
        ps2_slot.forward,
        psp_slot.forward
    );

    // What the PS2 craft now points along, against what the PSP file authors.
    let corrected = race::Race::start(on_ps2.setup)
        .sim
        .world
        .ships
        .iter()
        .find(|ship| ship.active)
        .expect("a craft on the grid")
        .physics
        .body
        .forward();
    let authored = Vec3::from_array(psp_slot.forward);
    let apart = corrected.dot(authored).clamp(-1.0, 1.0).acos().to_degrees();
    assert!(
        apart < 0.1,
        "the correction lands on the maintained heading: {apart:.3} degrees apart"
    );
}
