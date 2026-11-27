//! The Fury menu backdrop's camera against matrices read out of RPCS3.
//!
//! `scripts/hd-fury-backdrop-break.py` stops `BackgroundAnimFury_Render`
//! (`0x00183c88`) before its perspective helper and after `0x00182358`, and
//! reads the view matrix at `sp+0x430`, the projection at `sp+0x470`, the
//! clip's path, clock and constants. The numbers here are from its
//! 2026-09-14 runs at 1280x720 (`/tmp/hd-fury-backdrop-break3`, `..5`);
//! see docs/ghidra/functions/ps3-hdfury-eu/menu-backdrop.md, "Runtime
//! verification".
//!
//! The clip's clock was read a frame after the camera was built, so a
//! matrix is expected to agree to what the eye moves in one frame - a few
//! thousandths on a path a few units long over nine seconds.

use std::path::{Path, PathBuf};

use oag_game::boot;
use oag_ui::frontend;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")
}

fn fury(image: &Path) -> std::sync::Arc<boot::fury::FuryAssets> {
    let options = boot::Options {
        language: None,
        source: image.display().to_string(),
        dlc: Vec::new(),
        leg: frontend::Leg::LogoFmv,
        movie: None,
        cache: oag_testdata::scratch::cache("hd-fury-backdrop-ground-truth"),
        audio_cache: oag_source::cache::default_audio_cache_dir(),
        extent: oag_game::movie::Extent::Frames(oag_game::INTRO_FRAMES_NEEDED),
        no_video: true,
        refresh_video: false,
        prefer_av1_cache: false,
        find_texture_packs: None,
    };
    let (shell, _archives, _title) =
        boot::load_shell(&options).expect("HD's front end is wired; see ADR-0025");
    match &*shell
        .fury_backdrop
        .expect("the Fury style authors a BackgroundAnimFury widget")
    {
        boot::backdrop::MenuBackdrop::Fury(assets) => std::sync::Arc::clone(assets),
        boot::backdrop::MenuBackdrop::Scene(_) => panic!("the Fury style draws the Fury widget"),
    }
}

/// One RPCS3 frame: the static path and clock the clip held, the fovy the
/// camera returned, and the view matrix `Render` handed the point pass,
/// row-major as the RSX reads it (`c[0..3]`, row 3 the translation).
struct Captured {
    path: usize,
    frame: u32,
    fovy_deg: f32,
    world_view: [[f32; 4]; 4],
}

const CAPTURED: &[Captured] = &[
    Captured {
        path: 7,
        frame: 421,
        fovy_deg: 40.0,
        world_view: [
            [0.53755, 0.10270, -0.83696, 0.0],
            [0.00000, 0.99256, 0.12179, 0.0],
            [0.84323, -0.06547, 0.53354, 0.0],
            [1.82913, -0.37528, -4.41755, 1.0],
        ],
    },
    Captured {
        path: 6,
        frame: 372,
        fovy_deg: 30.0,
        world_view: [
            [0.98810, -0.06170, 0.14092, 0.0],
            [0.00000, 0.91604, 0.40108, 0.0],
            [-0.15384, -0.39631, 0.90514, 0.0],
            [0.06522, -1.54507, -3.34149, 1.0],
        ],
    },
];

/// The clip's clock is one frame ahead of the camera it was read beside.
const CLOCK_LAG: u32 = 1;

/// A frame's travel along the fastest captured path, with headroom.
const TOLERANCE: f32 = 0.02;

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_camera_is_the_one_rpcs3_builds() {
    let Some(image) = image() else {
        return;
    };
    let assets = fury(&image);
    for captured in CAPTURED {
        let mut model =
            oag_ui::backdrop::Fury::new(assets.settings.clone(), assets.first, boot::fury::SEED)
                .expect("static paths are authored");
        model
            .force_path(captured.path)
            .expect("the path is authored");
        for _ in 0..captured.frame - CLOCK_LAG {
            model.tick(&oag_ui::backdrop::SILENCE);
        }
        assert_eq!(model.path(), captured.path, "the clip outlived its path");
        let frame = model.frame(720.0, 16.0 / 9.0, [1.0; 4]);
        // `Frame::world_view` is column-major `[column][row]`; the capture's
        // rows are the RSX's `c[n]`, which is the same matrix transposed.
        for row in 0..4 {
            for col in 0..4 {
                let ours = frame.world_view[row][col];
                let theirs = captured.world_view[row][col];
                assert!(
                    (ours - theirs).abs() < TOLERANCE,
                    "path {} frame {}: worldView[{row}][{col}] {ours} against RPCS3's {theirs}",
                    captured.path,
                    captured.frame
                );
            }
        }
        let fovy = assets.settings.static_paths[captured.path].fovy;
        assert!((fovy - captured.fovy_deg).abs() < 1e-4);
        // At 720 lines the sprite scale is `2.26 - 0.0011667 * 720`, and the
        // clip block read `pointSize * 1.42` for both paths.
        let sprite_scale = 2.26 - 0.001_166_7 * 720.0;
        let expected = assets.settings.static_paths[captured.path].point_size * sprite_scale;
        assert!((frame.sprite_size - expected).abs() < 1e-5);
    }
}

/// The constants `RadioHead2_Update` fills, read off the uploaded block at
/// `RadioHead2_Upload` on path 6 at frame 118 and path 0 at frame 63, with
/// the music pulse at what silence reads.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_constants_are_the_ones_rpcs3_uploads() {
    let Some(image) = image() else {
        return;
    };
    let assets = fury(&image);
    assert!(
        assets.bloom.final_half_texel,
        "the backdrop ends in the final composite's quad, which reads half a texel off"
    );
    struct Uploaded {
        path: usize,
        frame: u32,
        fog: [f32; 4],
        ramp_factors: [f32; 4],
        dof_x: f32,
    }
    let cases = [
        Uploaded {
            path: 6,
            frame: 118,
            fog: [-20.0, -1.0 / 3.0, -23.0, 0.2],
            ramp_factors: [12.1333, 0.08, 0.5, 0.0],
            dof_x: -20.0,
        },
        Uploaded {
            path: 0,
            frame: 63,
            fog: [-3.0, -0.25, -7.0, 0.3],
            ramp_factors: [15.8, 0.08, 0.5, 0.0],
            dof_x: -3.0,
        },
    ];
    for Uploaded {
        path,
        frame: at,
        fog,
        ramp_factors,
        dof_x,
    } in cases
    {
        let mut model =
            oag_ui::backdrop::Fury::new(assets.settings.clone(), assets.first, boot::fury::SEED)
                .expect("static paths are authored");
        model.force_path(path).expect("the path is authored");
        for _ in 0..at - CLOCK_LAG {
            model.tick(&oag_ui::backdrop::SILENCE);
        }
        let frame = model.frame(720.0, 16.0 / 9.0, [1.0; 4]);
        for (ours, theirs) in frame.fog_factors.iter().zip(fog) {
            assert!(
                (ours - theirs).abs() < 1e-4,
                "path {path} fog {ours} against {theirs}"
            );
        }
        // `20 - 4 * seconds`, a frame's worth of slack.
        assert!(
            (frame.colour_ramp_factors[0] - ramp_factors[0]).abs() < 0.07,
            "path {path} crf.x"
        );
        for (ours, theirs) in frame.colour_ramp_factors.iter().zip(ramp_factors).skip(1) {
            assert!((ours - theirs).abs() < 1e-5);
        }
        assert_eq!(frame.colour_ramp_factors2, [4.0, -3.0, 6.0, 0.0]);
        assert!((frame.dof_factors[0] - dof_x).abs() < 1e-4);
        assert_eq!(frame.depth_fade_factors, [0.5, -0.5]);
        // `Particle Ramp Colour * resScale`, resScale `0.46` at 720 lines.
        for (ours, theirs) in frame.colour_ramp.iter().zip([5.4118, 1.4219, 0.1061]) {
            assert!(
                (ours - theirs).abs() < 1e-3,
                "colourRamp {ours} against {theirs}"
            );
        }
        // `Particle Colour * 0.7 * musicPulse * resScale` with the bands at
        // zero - the `0.040` RPCS3 reads between beats.
        for (ours, theirs) in frame.particle_colour.iter().zip([0.0401, 0.00174, 0.0]) {
            assert!(
                (ours - theirs).abs() < 5e-4,
                "particleColour {ours} against {theirs}"
            );
        }
    }
}

/// The block a screen with models loads is `fe.fury.EnvSettings`' own `HDR and Bloom`, and it
/// is what the running chain holds on the campaign's pages: the settings store read
/// `Tone adaption boost` 35, clamp 3, maximum 4 and `Bloom adaption rate` 0.015 on
/// `Campaign Selection`, both `Grid Selection`s and `Cell Selection`, where `Main Menu` read
/// the track file's 20, 3, 4 and 0.025. The gate's keys are the file's, and with them the
/// chain's law puts `Cell Selection`'s card, texel 63, at the level the frame of that read
/// shows, from the scene mean read beside it
/// (`docs/ghidra/functions/ps3-hdfury-eu/front-end-model-lighting.md`, "The flyer card is
/// drawn before the resolve").
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn a_model_screens_block_is_the_one_read_live_on_the_campaign_pages() {
    let Some(image) = image() else {
        return;
    };
    let assets = fury(&image);
    let tone = |block: &oag_post::hd_bloom::Params| {
        [
            block.tone_adaption_boost,
            block.tone_darkening_clamp,
            block.tone_maximum_brightness,
            block.adaption_rate,
        ]
    };
    let models = assets
        .model_bloom
        .expect("fe.fury.EnvSettings carries a whole HDR and Bloom block");
    assert_eq!(tone(&models), [35.0, 3.0, 4.0, 0.015], "the campaign pages");
    assert_eq!(tone(&assets.bloom), [20.0, 3.0, 4.0, 0.025], "Main Menu");
    assert_eq!(
        [
            models.alpha_contribution,
            models.frame_contribution,
            models.frame_exponent,
            models.adaption_boost,
        ],
        [0.75, 0.5, 0.0, 38.75]
    );
    // `Cell Selection` of `09_blitzed`: the ladder's mean read (34, 1, 1) of 255, the card's
    // flat red 104 on the frame grabbed at that read, its texel 63 and its scene alpha clear.
    let adapted = (0.3 * 34.0 + 0.59 + 0.11) / 255.0;
    let exposure = oag_post::hd_bloom::exposure(
        adapted,
        models.tone_adaption_boost,
        models.tone_darkening_clamp,
        models.tone_maximum_brightness,
    );
    let fade =
        1.0 - (adapted * models.adaption_boost * oag_post::hd_bloom::ADAPTION_FADE_SCALE).min(1.0);
    let store = oag_gpu::formats::SceneStore::Encoded;
    let card = store.texel_encode(
        store.texel_decode(63.0 / 255.0) * (exposure + models.frame_contribution * fade),
    ) * 255.0;
    assert!(
        (card - 104.0).abs() < 1.0,
        "exposure {exposure:.3} and a frame bloom of {:.3} put the card at {card:.1}; the frame shows 104",
        models.frame_contribution * fade
    );
}
