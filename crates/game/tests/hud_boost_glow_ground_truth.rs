//! A speed pad's boost on Pulse PSP's HUD, on a real disc: the speed bar glows and the widgets
//! smear, and at rest the HUD is the plain one.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not ship.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game hud_boost_glow_ground_truth:: --run-ignored all
//! ```
//!
//! A pickup absorb on the same HUD: the shield bar's fill stamps the glow mask for the whole
//! window, so the bar blooms in both halves of the 8 Hz blink.
//!
//! # What only real data can say here
//!
//! A PPSSPP frame at a boost's middle (`n = 7` of the 0.8 s timer) has the speed bar's fill washed
//! to near white with a halo, and its text ghosted: the HUD's own copies, and a glow mask stamped
//! under the fill that the bloom, which the frame has anyway, then blooms
//! (`docs/ghidra/functions/psp-pulse-usa/hud-boost-smear.md`). Dropping either half of the
//! wiring - the copies in `oag_hud::boost_smear`, or `Scene::stamp_glow` from
//! `oag_game::hud_views::draw` - leaves the bar crisp and mint, which this reads.

use std::path::Path;

/// The RGBA bytes of a PNG this project's own writer made (stored deflate blocks).
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

/// A posed Pulse PSP frame in `mode` with `extra` capture flags on the command line.
fn frame_in(image: &Path, scratch: &Path, name: &str, mode: &str, extra: &[&str]) -> Vec<u8> {
    let out = scratch.join(format!("{name}.png"));
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"));
    command
        .arg(image)
        .args([
            "--race",
            "--no-audio",
            "--no-soundtrack",
            "--no-effects",
            "--size",
            "480x272",
        ])
        .args([
            "--render-scale",
            "100",
            "--msaa",
            "off",
            "--screen-filter",
            "off",
        ])
        .args([
            "--anisotropy",
            "off",
            "--motion-blur",
            "off",
            "--ticks",
            "0",
        ])
        .args(["--mode", mode, "--class", "venom"])
        .args(["--track", r"Data\Environments\16_Track\track.vex"])
        .arg("--pose=-118.4,-49.2,-702.0,2.8")
        .args(["--hud-state", "5,2,29.4,39.3,562.9,99.4,56.34"])
        .arg("--screenshot")
        .arg(&out)
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"));
    command.args(extra);
    assert!(command.status().expect("running oag-game").success());
    rgba_of(&std::fs::read(&out).expect("the frame"), 480, 272)
}

fn frame(image: &Path, scratch: &Path, name: &str, boost: Option<&str>) -> Vec<u8> {
    match boost {
        Some(phase) => frame_in(
            image,
            scratch,
            name,
            "time_trial",
            &["--hud-boost-phase", phase],
        ),
        None => frame_in(image, scratch, name, "time_trial", &[]),
    }
}

/// The mean of channel `channel` over the rectangle `[x0, x1) x [y0, y1)`.
fn mean(rgba: &[u8], channel: usize, x: (usize, usize), y: (usize, usize)) -> f32 {
    let mut sum = 0u64;
    for row in y.0..y.1 {
        for col in x.0..x.1 {
            sum += u64::from(rgba[(row * 480 + col) * 4 + channel]);
        }
    }
    sum as f32 / ((x.1 - x.0) * (y.1 - y.0)) as f32
}

fn scratch() -> oag_testdata::scratch::TempDir {
    oag_testdata::scratch::dir("boost-glow")
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and a GPU adapter"]
fn a_boost_washes_the_speed_bar_and_rest_leaves_the_hud_plain() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let scratch = scratch();
    let plain = frame(&image, &scratch, "plain", None);
    let rest = frame(&image, &scratch, "rest", Some("0"));
    let boost = frame(&image, &scratch, "boost", Some("0.45"));
    std::fs::remove_dir_all(&scratch).ok();

    assert_eq!(
        plain, rest,
        "a boost timer of zero is the plain HUD, byte for byte"
    );
    // The speed bar's fill (x 306..430, y 222..244): mint at rest, washed toward white (the red
    // channel climbs from about 125) with the stamp's glow and the copies' own composite.
    let bar = |frame: &[u8]| mean(frame, 0, (310, 420), (224, 242));
    assert!(
        bar(&boost) > bar(&rest) + 40.0,
        "the bar's red went {} -> {}",
        bar(&rest),
        bar(&boost)
    );
    // The lap counter's corner is the same widgets with the same copies and no stamp: ghosted,
    // never brightened past what the copies' alpha leaves of the glyph.
    let corner = |frame: &[u8]| mean(frame, 1, (0, 60), (0, 40));
    assert!(
        (corner(&boost) - corner(&rest)).abs() < 25.0,
        "{} -> {}",
        corner(&rest),
        corner(&boost)
    );
}

/// A Pulse race frame at `ticks` with the held pickup absorbed at tick 100 (`--give mine`, circle),
/// or the same frame with no pickup and no press.
fn absorb_frame(image: &Path, scratch: &Path, name: &str, ticks: u32, absorb: bool) -> Vec<u8> {
    let out = scratch.join(format!("{name}.png"));
    let script = scratch.join("press.inputs");
    std::fs::write(&script, "100 none\n1 circle\n600 none\n").expect("the input script");
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"));
    command
        .arg(image)
        .args([
            "--race",
            "--no-audio",
            "--no-soundtrack",
            "--no-effects",
            "--size",
            "480x272",
        ])
        .args([
            "--render-scale",
            "100",
            "--msaa",
            "off",
            "--screen-filter",
            "off",
        ])
        .args(["--anisotropy", "off", "--motion-blur", "off"])
        .args([
            "--mode",
            "single_race",
            "--class",
            "venom",
            "--team",
            "Assegai",
        ])
        .args(["--track", r"Data\Environments\13_Track\track.vex"])
        .args(["--ticks", &ticks.to_string()])
        .arg("--screenshot")
        .arg(&out)
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"));
    if absorb {
        command
            .args(["--give", "mine", "--input-script"])
            .arg(&script);
    }
    assert!(command.status().expect("running oag-game").success());
    rgba_of(&std::fs::read(&out).expect("the frame"), 480, 272)
}

/// The mean Rec. 709 luma over the rectangle `[x0, x1) x [y0, y1)`.
fn luma(rgba: &[u8], x: (usize, usize), y: (usize, usize)) -> f32 {
    let channel = |c: usize| mean(rgba, c, x, y);
    0.2126 * channel(0) + 0.7152 * channel(1) + 0.0722 * channel(2)
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and a GPU adapter"]
fn an_absorb_lights_the_shield_bar_and_its_halo_in_both_blink_phases() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let scratch = scratch();
    let control = absorb_frame(&image, &scratch, "control", 108, false);
    let on = absorb_frame(&image, &scratch, "on", 104, true);
    let off = absorb_frame(&image, &scratch, "off", 108, true);
    std::fs::remove_dir_all(&scratch).ok();

    // The fill (rows 248 to 258, x 315 to 440) and the band under it, which only a bloom of the
    // bar reaches. PPSSPP: the fill reads 164 at rest, 207 in the blink's on half and 238 in its
    // off half, where the fill's alpha is zero and the stamped mask blooms the pale backdrop
    // (`docs/ghidra/functions/psp-pulse-usa/shield.md`, "The absorb flash at the draw call").
    let bar = |frame: &[u8]| luma(frame, (315, 440), (248, 258));
    let band = |frame: &[u8]| luma(frame, (330, 420), (259, 265));
    assert!(
        (bar(&control) - 164.0).abs() < 4.0,
        "the bar at rest: {}",
        bar(&control)
    );
    assert!(
        (190.0..215.0).contains(&bar(&on)),
        "the on half is bright cyan, not the unlit 164 or white: {}",
        bar(&on)
    );
    assert!(
        (232.0..250.0).contains(&bar(&off)),
        "the off half washes to the original's 237 to 242, which needs the scene under the backdrop in the bright pass: {}",
        bar(&off)
    );
    for (name, frame) in [("on", &on), ("off", &off)] {
        assert!(
            band(frame) > band(&control) + 25.0,
            "{name}: the halo under the bar, {} against {}",
            band(frame),
            band(&control)
        );
    }
}

/// The race HUD's marquee bar and head-to-head bar, drawn from the capture flags, on a posed single
/// race (the arcade layout is the one that authors `HeadToHeadBar`): the now-playing marquee's bar
/// darkens the track under it to about four fifths (alpha 0.2 at the end of its fade-in, drawn once), and the
/// head-to-head bar is a column of the HUD's mint (`0xFF7DEFC0`) the plain frame lacks. Dropping the flags' wiring in `oag_game::app` or `oag_raceplay` leaves both out.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and a GPU adapter"]
fn the_capture_flags_draw_the_marquee_bar_and_the_head_to_head_bar() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let scratch = scratch();
    let plain = frame_in(&image, &scratch, "bars-plain", "single_race", &[]);
    let bars = frame_in(
        &image,
        &scratch,
        "bars-on",
        "single_race",
        &[
            "--hud-song",
            "Test Title",
            "--hud-song-artist",
            "Some Artist",
            "--hud-head-to-head",
            "150,lead",
        ],
    );
    std::fs::remove_dir_all(&scratch).ok();

    // The marquee's bar (x 130..280, y 247..267) left of where the text has scrolled to: green
    // channel, whose track pixels are bright enough to read a ratio of.
    let bar = |frame: &[u8]| mean(frame, 1, (135, 215), (249, 265));
    let ratio = bar(&bars) / bar(&plain);
    assert!(
        (0.74..0.86).contains(&ratio),
        "the marquee bar leaves {ratio} of the track under it, not about 0.8 (about 0.2, drawn once)"
    );

    // The head-to-head bar: a column of the HUD's mint, 5 units wide at the right, and a few
    // stray pixels of the same colour in the plain frame.
    let mint = |frame: &[u8]| {
        (0..272usize)
            .flat_map(|y| (440..480usize).map(move |x| (x, y)))
            .filter(|&(x, y)| frame[(y * 480 + x) * 4..][..3] == [0x7D, 0xEF, 0xC0])
            .count()
    };
    assert!(
        mint(&plain) < 20,
        "{} mint pixels without the flag",
        mint(&plain)
    );
    assert!(
        mint(&bars) > 100,
        "the bar is {} mint pixels, expected a column",
        mint(&bars)
    );
}
