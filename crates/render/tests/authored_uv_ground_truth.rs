//! The authored texture-transform tracks a circuit carries, and that the
//! renderer reaches them.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this crate:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-render --run-ignored all
//! ```
//!
//! # What this is for
//!
//! `animated_uv_ground_truth.rs`, this file's predecessor, measured *which*
//! surfaces look like they should animate, from banded texture rows and
//! narrow-V-band geometry, because at the time nothing on the disc was known to
//! say so. Something does: every animated material carries a keyframed
//! `TEXSCALE`/`TEXOFFSET` block after the mesh's material array - key times in
//! 60 Hz frames, values in 1/256 units, an authored loop period and a step
//! flag. `TexAnim_UpdateTransform` (`0x08927204`) replays it, and so does
//! `oag_mesh::mesh_render::TexAnims`. See
//! `docs/ghidra/functions/psp-pulse-usa/texture-animation.md`.
//!
//! That earlier file is kept: its survey is still the only record of which
//! surfaces the *geometry* singles out, and it is a useful cross-check. This
//! one pins the mechanism that actually draws.

use std::path::PathBuf;

use oag_mesh::mesh::{self, AnimTrack, Model};
use oag_mesh::mesh_render::TexAnims;
use oag_vex::vex;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

/// Every track `mesh::build`/`mesh::build_sky` produce is a Pulse/Pure
/// `TEXOFFSET`/`TEXSCALE` block - the PS3 variant only ever comes out of
/// `mesh::rcs::build*`, which nothing in this file calls - so every assertion
/// below reads straight through to it rather than repeat the match at each
/// call site.
fn psp(track: &AnimTrack) -> Option<&vex::TexTransform> {
    match track {
        AnimTrack::Psp(t) => Some(t),
        AnimTrack::Rcs(_) | AnimTrack::Scroll(_) => None,
    }
}

fn track(archives: &mut oag_assets::Archives, circuit: &str) -> Model {
    let name = format!(r"Data\Environments\{circuit}_Track\track.vex");
    let blob = archives.read_name(&name).expect("reading track.vex");
    mesh::build(&name, &blob).expect("decoding track.vex")
}

/// Talon's Junction's turn arrows animate, and the way they animate is a V
/// scroll of exactly one tile per 40 frames.
///
/// The surface a player sees this on: the chevrons on the second turn, which
/// the original runs and this project drew frozen until the authored blocks
/// were read. `16_Track` is Talon's Junction White - the mapping is not
/// guessable from the directory number, see `oag_pulse::race::DEFAULT_TRACK`.
///
/// Three meshes carry `col_arrows1_GLOW_ADD` and all three author the
/// identical track, so this does not depend on picking the right one.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn talons_junction_arrows_author_a_one_tile_v_scroll_every_forty_frames() {
    let Some(image) = image() else { return };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening archives");
    let model = track(&mut archives, "16");

    let arrows: Vec<_> = model
        .anim_tracks
        .iter()
        .filter_map(psp)
        .filter(|t| t.offset.times == vec![0, 40])
        .collect();
    assert!(
        !arrows.is_empty(),
        "no 40-frame track on 16_Track - the arrows' block is not being read"
    );
    for track in &arrows {
        assert_eq!(
            track.offset.values,
            vec![(0, 0), (0, 256)],
            "V by one whole tile, and forwards - the opposite of the ships' blink"
        );
        assert_eq!(track.scale.values, vec![(256, 256)], "scale is identity");
        assert!(!track.step, "a continuous scroll, not a stepped flicker");
        // 0.6667 s. The loop is the track's own span here, which is *not*
        // true in general - the flicker sequences below end at frame 12 and
        // loop at 50 - so it is asserted rather than assumed.
        assert!(
            (track.loop_seconds - 40.0 / 60.0).abs() < 1e-4,
            "loop {} s, expected 40 frames",
            track.loop_seconds
        );
        assert!((track.seconds_per_key - 1.0 / 60.0).abs() < 1e-6);

        // And it really sweeps: a quarter of the way through the loop is a
        // quarter of a tile along, and one whole loop is back at the start.
        let v = |frames: f32| track.sample(frames / 60.0).1[1];
        assert!((v(0.0) - 0.0).abs() < 1e-6);
        assert!((v(10.0) - 0.25).abs() < 1e-6, "{}", v(10.0));
        assert!((v(20.0) - 0.5).abs() < 1e-6, "{}", v(20.0));
        assert!((v(40.0) - v(0.0)).abs() < 1e-6, "wraps at its own period");
        assert!((v(50.0) - v(10.0)).abs() < 1e-6, "and keeps wrapping");
    }
}

/// The arrows reach the GPU: some vertex indexes the 40-frame track, and the
/// table the shader reads moves between two ticks 10 apart.
///
/// The half of the port a data-only assertion cannot cover - that the index
/// survives `mesh::build` and that `TexAnims::sample` writes the slot the
/// index points at.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_arrows_track_reaches_a_vertex_and_moves_between_ticks() {
    let Some(image) = image() else { return };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening archives");
    let model = track(&mut archives, "16");

    let slot = model
        .anim_tracks
        .iter()
        .position(|t| psp(t).is_some_and(|t| t.offset.times == vec![0, 40]))
        .expect("the arrows' track is in the table");
    let index = u32::try_from(slot + 1).expect("slot fits");
    let vertices = model.vertices.iter().filter(|v| v.anim == index).count();
    assert!(
        vertices > 0,
        "the arrows' track is in the table but no vertex selects it"
    );

    // The same tick-to-seconds conversion `race::Scene::draw` uses.
    let at = |tick: u64| TexAnims::sample(&model, tick as f32 / 60.0).transform[slot + 1];
    let (first, later) = (at(0), at(10));
    assert_eq!([first[0], first[1]], [1.0, 1.0], "scale stays identity");
    assert!(
        (later[3] - first[3]).abs() > 0.2,
        "the arrows' V offset did not move over 10 ticks: {first:?} -> {later:?}"
    );
    // Approximate rather than exact: the wrap is an `fmod` of a value the
    // conversion from ticks reached by division, so the residue is real.
    let wrapped = at(40);
    assert!(
        (wrapped[3] - first[3]).abs() < 1e-6,
        "did not wrap on the authored period: {wrapped:?}"
    );

    // Slot 0 is the identity every unanimated vertex reads, and nothing may
    // write it - a track landing there would animate the whole circuit.
    assert_eq!(
        TexAnims::sample(&model, 1.234).transform[0],
        [1.0, 1.0, 0.0, 0.0]
    );
}

/// Every `.vex` the disc ships fits inside `ANIM_TRACK_LIMIT`, and no
/// `Skycube` picks up a spurious animation.
///
/// The measurement behind the constant's headroom claim, which was written off
/// `16_Track` alone. Overflow is graceful - the surfaces past the ceiling draw
/// unanimated - but the ceiling should be a number somebody checked.
///
/// The sky half is the trap this closes. A `Skycube` payload **is** a `Mesh`
/// payload, so it goes through the same parser, and arbitrary bytes read as a
/// plausible keyframe block often enough to matter - a spuriously animated sky
/// slides the whole horizon. The `& 0x10` material gate is what stops it.
///
/// **Exactly one shipped sky animates, and it is real: `06_Track`'s**
/// (Vertica). It drifts one tile diagonally - `(0, 0)` to `(255, 255)` over
/// key times 0..1997 - on a **33.3 s** loop, over 71 of its 494 vertices. A
/// cloud layer, in other words, and by some margin the slowest track on the
/// disc. It is asserted by value rather than excluded, because "no sky
/// animates" was the natural expectation, is what this test first asserted,
/// and is wrong.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_shipped_model_fits_the_track_table_and_no_sky_animates() {
    let Some(image) = image() else { return };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening archives");

    // The four `.vex` files a track directory can hold, from the binary's
    // `%s\%strack%s.vex` template - measuring only `track.vex` would cover a
    // quarter of the disc's track files and miss every reversed circuit.
    const VARIANTS: &[&str] = &[
        "track.vex",
        "track_reversed.vex",
        "zone_track.vex",
        "zone_track_reversed.vex",
    ];
    const TEAMS: &[&str] = &[
        "Assegai",
        "AGSystems",
        "Feisar",
        "Goteki45",
        "Piranha",
        "Qirex",
        "Triakis",
        "EGX",
    ];

    let mut names: Vec<String> = Vec::new();
    for circuit in 1..=32 {
        for variant in VARIANTS {
            names.push(format!(r"Data\Environments\{circuit:02}_Track\{variant}"));
        }
    }
    for team in TEAMS {
        names.push(format!(r"Data\Ships\{team}\Ship.vex"));
        names.push(format!(r"Data\Ships\{team}\shipboost.vex"));
    }

    let mut worst = (0usize, String::new());
    let mut read = 0usize;
    let mut skies = 0usize;
    let mut drifting_skies: Vec<String> = Vec::new();
    for name in &names {
        let Ok(blob) = archives.read_name(name) else {
            continue;
        };
        let Ok(model) = mesh::build(name, &blob) else {
            continue;
        };
        read += 1;
        if model.anim_tracks.len() > worst.0 {
            worst = (model.anim_tracks.len(), name.clone());
        }

        // The sky, built the way `race::Scene` builds it: `Skycube` nodes
        // decoded through the same mesh path.
        let Ok(sky) = mesh::build_sky(name, &blob, None) else {
            continue;
        };
        if sky.mesh_count == 0 {
            continue;
        }
        skies += 1;
        for drift in sky.anim_tracks.iter().filter_map(psp) {
            drifting_skies.push(name.clone());
            // The one authored case is a 2000-frame drift. Anything an order
            // of magnitude faster is the failure mode this guards: a block
            // read out of bytes that are not one, which slides the horizon.
            assert!(
                drift.loop_seconds > 10.0,
                "{name}: its Skycube animates on a {} s loop - too fast for a \
                 cloud layer, so this is a block read out of bytes that are \
                 not one",
                drift.loop_seconds
            );
            assert_eq!(drift.offset.values, vec![(0, 0), (255, 255)]);
            assert!((drift.loop_seconds - 2000.0 / 60.0).abs() < 1e-2);
        }
    }

    assert!(
        read > 30,
        "only {read} models read - the sweep found nothing"
    );
    assert!(skies > 0, "no Skycube was reached, so nothing was checked");
    println!(
        "{read} models, {skies} skies, worst {} in {}, drifting skies {drifting_skies:?}",
        worst.0, worst.1
    );
    // Vertica's, and only Vertica's - both layouts of it, since the forward
    // and reversed files share the circuit's sky. More circuits would mean the
    // gate has stopped holding; none would mean the sky path no longer reaches
    // the blocks at all.
    assert!(
        !drifting_skies.is_empty() && drifting_skies.iter().all(|n| n.contains("06_Track")),
        "expected only Vertica's drifting sky, got {drifting_skies:?}"
    );
    assert!(
        worst.0 < mesh::ANIM_TRACK_LIMIT,
        "{} authors {} distinct tracks, past ANIM_TRACK_LIMIT ({}) - raise it, \
         or its surfaces past the ceiling draw unanimated",
        worst.1,
        worst.0,
        mesh::ANIM_TRACK_LIMIT
    );
}

/// `16_Track`'s flicker panels loop on the block's `+0x2c`, not on their last
/// key, and their siblings share that period at different key times.
///
/// The case that makes reading `+0x2c` mandatory rather than a refinement.
/// Three families of stepped panels end at frames 12, 18 and 24 and all three
/// author a **50-frame** loop; driven off their last key they would run at up
/// to four times speed and the phase interleave that makes them flicker
/// against each other would collapse into unison.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_flicker_panels_loop_on_the_authored_period_not_their_last_key() {
    let Some(image) = image() else { return };
    let mut archives = oag_pulse::open(&image.display().to_string()).expect("opening archives");
    let model = track(&mut archives, "16");

    let stepped: Vec<_> = model
        .anim_tracks
        .iter()
        .filter_map(psp)
        .filter(|t| t.step)
        .collect();
    assert!(
        stepped.len() >= 3,
        "expected the three interleaved flicker families, found {}",
        stepped.len()
    );
    for track in &stepped {
        // The point of the field: every one of these ends well before it
        // loops, so its last key time is not its period.
        assert!(
            track.loop_seconds * 60.0 > track.offset.period() + 1.0,
            "a stepped track ending at frame {} loops at {} s - if those were \
             the same, reading `+0x2c` would not matter",
            track.offset.period(),
            track.loop_seconds
        );
        // Stepped, so between two key pairs the value is held rather than
        // sliding. Every one of these families authors its keys in pairs one
        // frame apart, so a midpoint between pairs must equal the earlier key.
        let held = track.sample(f32::from(track.offset.times[1]) / 60.0).1;
        let later = track
            .sample((f32::from(track.offset.times[1]) + 0.5) / 60.0)
            .1;
        assert_eq!(held, later, "a stepped track must not interpolate");
    }

    // The interleave itself: three families are the same sequence of held
    // values at different key times over one shared 50-frame period, which is
    // what makes them flicker against one another instead of together.
    let fifty: Vec<_> = stepped
        .iter()
        .filter(|t| (t.loop_seconds - 50.0 / 60.0).abs() < 1e-4)
        .collect();
    let starts: std::collections::BTreeSet<u16> = fifty
        .iter()
        .filter_map(|t| t.offset.times.first().copied())
        .collect();
    assert!(
        starts.len() >= 3,
        "expected three 50-frame families at different key times, got {starts:?}"
    );
}
