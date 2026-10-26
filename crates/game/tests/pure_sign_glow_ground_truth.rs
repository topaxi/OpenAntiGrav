//! Pure's neon signs glow on a real disc: the stencil-stamped mask and the bloom over it.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game pure_sign_glow_ground_truth:: --run-ignored all
//! ```
//!
//! # What only real data can say here
//!
//! A GE list of Pure on PPSSPP's software renderer, Modesto Heights' compare pose
//! (`docs/ghidra/functions/psp-pure-usa/bloom.md`), draws the bloom's four passes after
//! the scene, and the neon signs on the background building glow in it. Dropping the
//! composite alone leaves the crisp signs ours drew before the bloom was wired. Read out
//! of EDRAM, the frame's alpha is `4` on ordinary batches and the texture's own glow byte
//! on the `_GLOW` ones: `0x9e` over 3,452 pixels and `0xb7` over 346 in the scene rows.
//! The sign box (x 180-280, y 60-105) averages luma 111.1 in the original's frame and 75.5
//! in ours without the bloom.
//!
//! The pose is the compare pair's own one-row trace, embedded below.

use std::path::Path;

/// The compare pair's `trace.csv`: tick 0 of the Modesto pose.
const POSE: &str = "tick,dt,grounded,throttle,brake,steer,airbrake_l,airbrake_r,speed_cached,stun_timer,timer_2e0,shield,fov_intercept,fov_additive,controller_class,ss_tap_window_l,ss_tap_window_r,ss_shift_l,ss_shift_r,ss_lockout,steer_lean,cam_lean,cam_lean_follower,roll_phase,roll_eased,right_x,right_y,right_z,up_x,up_y,up_z,fwd_x,fwd_y,fwd_z,pos_x,pos_y,pos_z,vel_x,vel_y,vel_z,speed,avel_x,avel_y,avel_z,omega_x,omega_y,omega_z,cam_right_x,cam_right_y,cam_right_z,cam_up_x,cam_up_y,cam_up_z,cam_fwd_x,cam_fwd_y,cam_fwd_z,cam_pos_x,cam_pos_y,cam_pos_z\n0.0,0.016666666666666666,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,0.0,-0.9999909996986389,3.650621511042118e-05,-0.0042629120871424675,6.904728797962889e-05,0.9999709725379944,-0.007633633445948362,0.004262509755790234,-0.007633858826011419,-0.9999619722366333,-309.9035339355469,197.99696350097656,-429.9906005859375,7.055111382214818e-06,0.036113590002059937,-0.0007905989186838269,0.03612224355243877,0.0,0.0,0.0,0.0,0.0,0.0,0.9950284957885742,6.904609472258016e-05,-0.0042340513318777084,-3.6326931876828894e-05,0.9999708533287048,0.10709376633167267,0.0042413221672177315,-0.007633693516254425,0.9942399263381958,-309.9512634277344,201.08297729492188,-418.7639465332031\n";

/// A PNG this project's own writer made, as RGBA bytes.
///
/// That writer stores its deflate blocks uncompressed and filters nothing
/// (`oag_texture::png`), so the pixels are in the file as they were written and a reader
/// needs only the chunk and block framing - not a decoder.
fn rgba_of(png: &[u8], width: usize, height: usize) -> Vec<u8> {
    let mut at = 8;
    let mut zlib = Vec::new();
    while at + 8 <= png.len() {
        let len = u32::from_be_bytes(png[at..at + 4].try_into().unwrap()) as usize;
        if &png[at + 4..at + 8] == b"IDAT" {
            zlib.extend_from_slice(&png[at + 8..at + 8 + len]);
        }
        at += 12 + len;
    }
    let mut raw = Vec::new();
    let mut at = 2;
    loop {
        let last = zlib[at] & 1 == 1;
        let len = usize::from(u16::from_le_bytes([zlib[at + 1], zlib[at + 2]]));
        raw.extend_from_slice(&zlib[at + 5..at + 5 + len]);
        at += 5 + len;
        if last {
            break;
        }
    }
    let stride = 1 + width * 4;
    assert_eq!(
        raw.len(),
        stride * height,
        "an RGBA PNG of the frame's size"
    );
    raw.chunks_exact(stride)
        .flat_map(|row| row[1..].iter().copied())
        .collect()
}

/// The compare pair's run: its flags, its settings and its pose, writing the frame and the
/// glow mask under `scratch`.
fn run(image: &Path, scratch: &Path) -> (Vec<u8>, Vec<u8>) {
    std::fs::create_dir_all(scratch.join("cfg/oag")).expect("the profile directory");
    std::fs::write(
        scratch.join("cfg/oag/settings.toml"),
        "[display]\naspect = \"psp\"\nbrightness = 100\ngamma = 100\n\n\
         [graphics]\nanisotropy = \"off\"\nfov = 100\nboost_fov_kick = 0\n\n\
         [render_profiles.\"Wipeout Pure (PSP)\"]\nrender_scale = 100\n\
         reconstruction = \"off\"\nmsaa = \"off\"\nmotion_blur = \"off\"\n\
         shadows = \"original\"\nmodel_detail = \"original\"\n\
         texture_detail = \"original\"\ntrail_order = \"original\"\nscreen_filter = \"off\"\n",
    )
    .expect("writing the settings");
    let pose = scratch.join("pose.csv");
    std::fs::write(&pose, POSE).expect("writing the pose");
    let (frame, mask) = (scratch.join("frame.png"), scratch.join("mask.png"));
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(image)
        .args([
            "--race",
            "--track",
            r"Data\Environments\03_Modesto_Heights\track.vex",
        ])
        .arg("--pose-from")
        .arg(&pose)
        .args(["--pose-tick", "0", "--no-camera", "--pose-hold"])
        .args([
            "--camera-fov",
            "60",
            "--camera-view",
            "close",
            "--ticks",
            "1339",
        ])
        .args(["--no-audio", "--size", "480x272", "--render-scale", "100"])
        .args([
            "--reconstruction",
            "off",
            "--msaa",
            "off",
            "--motion-blur",
            "off",
        ])
        .args(["--lod", "original", "--texture-detail", "original"])
        .args(["--trail-order", "original", "--screen-filter", "off"])
        .args([
            "--anisotropy",
            "off",
            "--no-hull-shake",
            "--mode",
            "time_trial",
        ])
        .args(["--class", "vector", "--team", "Feisar"])
        .arg("--screenshot")
        .arg(&frame)
        .env("OAG_DUMP_GLOW_MASK", &mask)
        .env("XDG_CONFIG_HOME", scratch.join("cfg"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"))
        .status()
        .expect("running oag-game");
    assert!(status.success(), "oag-game");
    let read = |path: &Path| rgba_of(&std::fs::read(path).expect("a PNG"), 480, 272);
    (read(&frame), read(&mask))
}

#[test]
#[ignore = "needs data/images/pure-psp-usa.chd and a GPU adapter"]
fn modesto_s_neon_signs_glow_and_the_mask_holds_the_originals_glow_bytes() {
    let Some(image) = oag_testdata::image("data/images/pure-psp-usa.chd") else {
        return;
    };
    let scratch = std::env::temp_dir().join(format!("oag-pure-sign-glow-{}", std::process::id()));
    let (frame, mask) = run(&image, &scratch);
    std::fs::remove_dir_all(&scratch).ok();

    // The sign box's mean luma (Rec. 601). 111.1 in the original; 75.5 in ours without the
    // bloom; 111.3 with it.
    let (mut sum, mut n) = (0.0f32, 0u32);
    for y in 60..105 {
        for x in 180..280 {
            let at = (y * 480 + x) * 4;
            let (r, g, b) = (frame[at], frame[at + 1], frame[at + 2]);
            sum += 0.299 * f32::from(r) + 0.587 * f32::from(g) + 0.114 * f32::from(b);
            n += 1;
        }
    }
    let luma = sum / n as f32;
    println!("sign box luma {luma:.1}");
    assert!(
        (100.0..=125.0).contains(&luma),
        "the sign box averages luma {luma:.1}: the original's is 111.1, and a frame with no bloom reads 75.5"
    );

    // The scene rows' mask (the HUD's own alpha lands outside them): the original holds `0x9e`
    // over 3,452 pixels and `0xb7` over 346, and `4` over the rest.
    let count = |value: u8| {
        (8..230)
            .flat_map(|y| (0..480).map(move |x| (y * 480 + x) * 4))
            .filter(|&at| mask[at] == value)
            .count()
    };
    let (wall, neon) = (count(0x9e), count(0xb7));
    println!("mask: 0x9e x {wall}, 0xb7 x {neon}");
    assert!(
        (3100..=3800).contains(&wall),
        "{wall} pixels hold 0x9e, the original's 3,452: the batch's glow byte is not stamped"
    );
    assert!(
        (280..=400).contains(&neon),
        "{neon} pixels hold 0xb7, the original's 346: the neon batch's glow byte is not stamped"
    );
}
