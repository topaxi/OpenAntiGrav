//! Split screen on a real circuit: `--players N` draws `N` different cameras into
//! `N` rectangles of one frame.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project
//! does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test split_screen_ground_truth --run-ignored all
//! ```
//!
//! # What only a drawn frame can say here
//!
//! Every failure this guards against still draws *something* in every rectangle:
//!
//! - one encoder for all the views, so each rectangle draws with the last view's
//!   matrices: four copies of one picture;
//! - every view clearing the whole target, so only the last rectangle has
//!   anything in it;
//! - a view that follows the player's camera instead of its own craft;
//! - a post chain that runs once per view (HD's, Omega's, which 2048 shares with HD):
//!   its encode pass clears the target and samples the linear scene from the
//!   top-left, so the top view went black and a later one showed the first view's
//!   picture.
//!
//! So the check is on pixels: each rectangle holds a picture, no two rectangles
//! hold the same one, the gap between them is the clear colour, and `--players 1`
//! is the same frame as no flag at all.

use std::path::Path;

const SIZE: (usize, usize) = (960, 544);

/// The RGBA pixels of a PNG this project's own writer made: deflate blocks stored
/// uncompressed and nothing filtered (`oag_texture::png`), so the pixels are in
/// the file as written and a reader needs only the chunk and block framing.
fn rgba_of(png: &[u8]) -> Vec<[u8; 4]> {
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
    let stride = 1 + SIZE.0 * 4;
    assert_eq!(
        raw.len(),
        stride * SIZE.1,
        "an RGBA PNG of the frame's size"
    );
    raw.chunks_exact(stride)
        .flat_map(|row| row[1..].as_chunks::<4>().0.iter().copied())
        .collect()
}

fn frame(
    image: &Path,
    scratch: &Path,
    name: &str,
    players: Option<u8>,
    msaa: &str,
) -> Vec<[u8; 4]> {
    let out = scratch.join(format!("{name}.png"));
    let mut command = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"));
    command
        .arg(image)
        .args([
            "--race",
            "--no-audio",
            "--mode",
            "single_race",
            "--autopilot",
        ])
        .args(["--size", &format!("{}x{}", SIZE.0, SIZE.1)])
        .args(["--render-scale", "100", "--msaa", msaa])
        .args(["--screen-filter", "off", "--anisotropy", "off"])
        .args(["--motion-blur", "off", "--ticks", "600"])
        .arg("--screenshot")
        .arg(&out)
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"));
    if let Some(players) = players {
        command.args(["--players", &players.to_string()]);
    }
    assert!(command.status().expect("running oag-game").success());
    rgba_of(&std::fs::read(&out).expect("the frame"))
}

type Rect = (usize, usize, usize, usize);

fn pixels(frame: &[[u8; 4]], (x, y, w, h): Rect) -> Vec<[u8; 4]> {
    (y..y + h)
        .flat_map(|row| {
            frame[row * SIZE.0 + x..row * SIZE.0 + x + w]
                .iter()
                .copied()
        })
        .collect()
}

/// The fraction of `a`'s pixels that differ from `b`'s by more than noise.
fn differing(a: &[[u8; 4]], b: &[[u8; 4]]) -> f32 {
    let far = |p: &[u8; 4], q: &[u8; 4]| (0..3).any(|i| p[i].abs_diff(q[i]) > 12);
    a.iter().zip(b).filter(|(p, q)| far(p, q)).count() as f32 / a.len() as f32
}

/// The fraction of pixels that are not the clear colour.
fn drawn(rect: &[[u8; 4]]) -> f32 {
    rect.iter()
        .filter(|p| p[..3].iter().any(|&c| c > 8))
        .count() as f32
        / rect.len() as f32
}

/// Every view holds a picture, and no two hold the same one.
fn distinct_views(name: &str, frame: &[[u8; 4]], rects: &[Rect]) {
    let views: Vec<_> = rects.iter().map(|&rect| pixels(frame, rect)).collect();
    for (i, view) in views.iter().enumerate() {
        let filled = drawn(view);
        assert!(
            filled > 0.5,
            "{name}: view {i} is {filled:.2} drawn, a later view cleared it"
        );
    }
    for i in 0..views.len() {
        for j in i + 1..views.len() {
            let apart = differing(&views[i], &views[j]);
            assert!(
                apart > 0.15,
                "{name}: views {i} and {j} differ in {apart:.2} of their pixels, one camera drawn twice"
            );
        }
    }
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and a GPU adapter"]
fn players_draws_distinct_cameras_into_distinct_rectangles() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let scratch = std::env::temp_dir().join(format!("oag-split-screen-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("creating the scratch directory");

    let none = frame(&image, &scratch, "none", None, "off");
    let one = frame(&image, &scratch, "one", Some(1), "off");
    assert!(
        none == one,
        "--players 1 is the frame with no flag, to the byte"
    );

    let two = frame(&image, &scratch, "two", Some(2), "off");
    let four = frame(&image, &scratch, "four", Some(4), "off");
    std::fs::remove_dir_all(&scratch).ok();

    let (w, h) = SIZE;
    // `oag_render::viewport::layout`'s own cut, with its two-pixel gap.
    let half = (h - 2) / 2;
    let two_up: [Rect; 2] = [(0, 0, w, half), (0, h - half, w, half)];
    let col = (w - 2) / 2;
    let quads: [Rect; 4] = [
        (0, 0, col, half),
        (w - col, 0, col, half),
        (0, h - half, col, half),
        (w - col, h - half, col, half),
    ];

    distinct_views("two", &two, &two_up);
    distinct_views("four", &four, &quads);

    // The gap between two views is what the target was cleared to, and the split
    // really is two views and not the whole frame drawn as one.
    let gap = pixels(&two, (0, half, w, 2));
    assert!(
        drawn(&gap) < 0.01,
        "the gap between two views is clear colour"
    );
    let single = &none;
    assert!(
        differing(&pixels(single, (0, 0, w, half)), &pixels(&two, two_up[0])) > 0.05,
        "the two-up top view is a different picture from the full-screen one's top half"
    );
}

/// The window's default is multisampled, and a multisample resolve rewrites the
/// whole target: the later views have to load the multisampled colour, not only
/// the resolved one.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd and a GPU adapter"]
fn four_views_survive_a_multisample_resolve() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let scratch = std::env::temp_dir().join(format!("oag-split-msaa-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("creating the scratch directory");
    let four = frame(&image, &scratch, "four", Some(4), "4x");
    std::fs::remove_dir_all(&scratch).ok();
    let (w, h) = SIZE;
    let (half, col) = ((h - 2) / 2, (w - 2) / 2);
    let quads: [Rect; 4] = [
        (0, 0, col, half),
        (w - col, 0, col, half),
        (0, h - half, col, half),
        (w - col, h - half, col, half),
    ];
    distinct_views("four, msaa", &four, &quads);
}

/// Two views, top and bottom, each holding a picture of its own.
fn two_up_draws_two_pictures(source: &Path, tag: &str) {
    let scratch = std::env::temp_dir().join(format!("oag-split-{tag}-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("creating the scratch directory");
    let two = frame(source, &scratch, "two", Some(2), "off");
    std::fs::remove_dir_all(&scratch).ok();
    let (w, h) = SIZE;
    let half = (h - 2) / 2;
    distinct_views(tag, &two, &[(0, 0, w, half), (0, h - half, w, half)]);
}

/// HD runs its bloom chain over a linear float scene and encodes into the frame:
/// per view, that wiped every earlier view.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu.iso and a GPU adapter"]
fn hd_two_views_each_keep_their_picture() {
    let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu.iso") else {
        return;
    };
    two_up_draws_two_pictures(&image, "hd two");
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu.iso and a GPU adapter"]
fn hd_four_views_each_keep_their_picture() {
    let Some(image) = oag_testdata::image("data/images/hdfury-ps3-eu.iso") else {
        return;
    };
    let scratch = std::env::temp_dir().join(format!("oag-split-hd-four-{}", std::process::id()));
    std::fs::create_dir_all(&scratch).expect("creating the scratch directory");
    let four = frame(&image, &scratch, "four", Some(4), "off");
    std::fs::remove_dir_all(&scratch).ok();
    let (w, h) = SIZE;
    let (half, col) = ((h - 2) / 2, (w - 2) / 2);
    let quads: [Rect; 4] = [
        (0, 0, col, half),
        (w - col, 0, col, half),
        (0, h - half, col, half),
        (w - col, h - half, col, half),
    ];
    distinct_views("hd four", &four, &quads);
}

/// Omega's tone map is the other chain that encodes into the frame per call.
#[test]
#[ignore = "needs data/extracted/ps4 and a GPU adapter"]
fn omega_two_views_each_keep_their_picture() {
    let Some(source) = oag_testdata::exact("data/extracted/ps4") else {
        return;
    };
    two_up_draws_two_pictures(&source, "omega two");
}

/// 2048 draws through HD's chain, so it had the same black top view.
#[test]
#[ignore = "needs data/extracted/vita/PCSF00007 and a GPU adapter"]
fn vita_2048_two_views_each_keep_their_picture() {
    let Some(source) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    two_up_draws_two_pictures(&source, "2048 two");
}
