//! Wipeout HD's Missile explosion draws, on a real disc: `HD_missile_explosion`
//! (core, rays and shockwave rings) written and drawn when a Missile reaches a
//! craft, on the clock `oag_raceplay::missile_blast` plays.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game hd_missile_blast_ground_truth:: --run-ignored all
//! ```
//!
//! # What only a frame can say here
//!
//! The ageing and the pool are pinned in `missile_blast`'s own tests; what a
//! frame adds is that the model loads, its three programs run and reach the
//! screen. The scenario is `--force-missile-hit`'s: the player fires at tick
//! 286, rival 1 is put on the missile at the end of tick 289 and the
//! simulation's own hit test takes the explosion on tick 290. The control is
//! tick 289, one tick before; a dropped write or draw leaves what the psys
//! stage alone draws.
//!
//! The streaks are seen from a camera 130 units to the side and 60 above the
//! blast, where the player's own chase camera is inside the white-out.

const SIZE: (usize, usize) = (960, 544);

/// Every pixel of a PNG this project's own writer made, as `[r, g, b]`.
///
/// That writer stores its deflate blocks uncompressed and filters nothing
/// (`oag_texture::png`), so a reader needs only the chunk and block framing.
fn rgb_of(png: &[u8]) -> Vec<[u8; 3]> {
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
        .flat_map(|row| {
            row[1..]
                .as_chunks::<4>()
                .0
                .iter()
                .map(|p| [p[0], p[1], p[2]])
        })
        .collect()
}

/// The scene at `ticks`: Talon's Junction, the player firing one Missile,
/// rival 1 onto it after tick 289, from the default chase camera.
fn frame_of(ticks: u32) -> Option<Vec<[u8; 3]>> {
    frame_with(ticks, &[])
}

/// [`frame_of`] with more arguments for `oag-game`.
fn frame_with(ticks: u32, extra: &[String]) -> Option<Vec<[u8; 3]>> {
    let image = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")?;
    let scratch = std::env::temp_dir().join(format!(
        "oag-hd-missile-blast-{}-{ticks}",
        std::process::id()
    ));
    std::fs::create_dir_all(&scratch).expect("creating the scratch directory");
    let out = scratch.join("frame.png");
    let script = scratch.join("fire.inputs");
    std::fs::write(&script, "285 none\n1 square\n400 none\n").expect("the input script");
    let status = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(image)
        .args([
            "--race",
            "--no-audio",
            "--no-soundtrack",
            "--no-effects",
            "--size",
            "960x544",
            "--opponents",
        ])
        .args(["--render-scale", "100", "--msaa", "off"])
        .args(["--screen-filter", "off", "--anisotropy", "off"])
        .args(["--motion-blur", "off", "--ticks", &ticks.to_string()])
        .args(["--give", "missile", "--force-missile-hit", "289:1"])
        .args(extra)
        .arg("--input-script")
        .arg(&script)
        .arg("--screenshot")
        .arg(&out)
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"))
        .status()
        .expect("running oag-game");
    assert!(status.success());
    let frame = rgb_of(&std::fs::read(&out).expect("the frame"));
    std::fs::remove_dir_all(&scratch).ok();
    Some(frame)
}

/// Pixels of `frame` for which `keep` holds.
fn count(frame: &[[u8; 3]], keep: impl Fn([u8; 3]) -> bool) -> usize {
    frame.iter().filter(|&&p| keep(p)).count()
}

/// The core's cream-white and the rings' yellow: red and green high.
fn bright([r, g, b]: [u8; 3]) -> bool {
    r > 235 && g > 200 && b > 120
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU adapter"]
fn a_missile_that_reaches_a_craft_draws_its_explosion() {
    // Bright pixels of 522,240. The control: 36,393. At 0.1 s: 200,364 drawn,
    // 38,411 with the model draw dropped (the psys stage's own burst remains).
    // At 0.2 s: 299,761 drawn, 32,539 dropped.
    let at = |ticks| count(&frame_of(ticks).expect("image"), bright);
    let (before, early, late) = (at(289), at(296), at(302));
    println!("bright pixels: before {before}, 0.1 s {early}, 0.2 s {late}");
    assert!(before < 60_000, "the control already glows: {before}");
    assert!(
        early > 120_000,
        "the explosion is missing at 0.1 s: {early}"
    );
    assert!(
        late > 200_000 && late > early,
        "the blast does not grow: {early} then {late}"
    );
}

/// Mean saturation `(max - min) / max` of the frame, in percent.
fn saturation(frame: &[[u8; 3]]) -> f64 {
    let total: f64 = frame
        .iter()
        .map(|p| {
            let (hi, lo) = (
                p.iter().max().copied().unwrap(),
                p.iter().min().copied().unwrap(),
            );
            f64::from(hi - lo) / f64::from(hi.max(1))
        })
        .sum();
    100.0 * total / frame.len() as f64
}

/// The core and the rays read their colour tap decoded on fetch, the shockwave ring raw.
///
/// Unit 0's gamma nibble on a live blast, two shots (`docs/rendering/hd-unlit-programs.md`,
/// "The weapon units, read live"): core and rays 7, ring 0. At 0.2 s from the chase camera the
/// frame's mean saturation is 15.5 % decoded and 8.8 % with the core and rays raw (pale cream
/// where the original's is saturated orange).
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU adapter"]
fn the_explosions_core_and_rays_read_their_taps_decoded() {
    let Some(frame) = frame_of(302) else {
        return;
    };
    let saturation = saturation(&frame);
    println!("mean saturation at 0.2 s: {saturation:.2} %");
    assert!(
        saturation > 12.0,
        "the core and rays read their colour taps raw: {saturation:.2} %"
    );
}

/// Where the blast is on Talon's Junction in the scenario above, and a camera looking at
/// it from 130 units along +z and 60 above: `eye,forward,up` for `--camera-pose`.
fn side_camera() -> String {
    let blast = [-121.987f32, -50.615, -175.324];
    let (up, back) = (60.0f32, 130.0f32);
    let length = up.hypot(back);
    format!(
        "--camera-pose={},{},{},0,{},{},0,1,0",
        blast[0],
        blast[1] + up,
        blast[2] + back,
        -up / length,
        -back / length
    )
}

/// Near-white pixels: the streaks are thin additive lines that burn out to white on the
/// road and the walls behind them.
fn white(&[r, g, b]: &[u8; 3]) -> bool {
    r > 240 && g > 240 && b > 240
}

/// White pixels of the side camera's frame at `ticks`.
fn streak_pixels(ticks: u32) -> Option<usize> {
    frame_with(ticks, &[side_camera()]).map(|frame| frame.iter().filter(|p| white(p)).count())
}

/// The sixteen `ThickLine_bighalo` streaks draw, and outlive the blast: three frames, one
/// test each, near-white pixels of 522,240 from the side camera. With the streak draw
/// dropped the frame reads 3,635 at tick 335 and 2,548 at tick 365; drawn, 13,658 and
/// 9,524. At tick 410, a second after the blast retired (age 1.0 at tick 350) and past the
/// 1.2 s of its last node, 2,567: every streak has died.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU adapter"]
fn the_explosions_streaks_draw_at_0_75_s() {
    let Some(burst) = streak_pixels(335) else {
        return;
    };
    println!("white pixels: burst {burst}");
    assert!(burst > 9_000, "the streaks are missing at 0.75 s: {burst}");
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU adapter"]
fn the_explosions_streaks_outlive_the_blast() {
    let Some(after) = streak_pixels(365) else {
        return;
    };
    println!("white pixels: after the blast {after}");
    assert!(
        after > 6_000,
        "the streaks died with the blast instead of outliving it: {after}"
    );
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU adapter"]
fn the_explosions_streaks_all_die() {
    let Some(gone) = streak_pixels(410) else {
        return;
    };
    println!("white pixels: all dead {gone}");
    assert!(gone < 4_500, "a streak never dies: {gone}");
}

/// The log's own count for the frame at `ticks`: `(draws submitted, triangles)`,
/// for a Missile fired at tick 286 and left in flight.
fn submitted_at(ticks: u32) -> Option<(u32, u32)> {
    let image = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")?;
    let scratch = std::env::temp_dir().join(format!(
        "oag-hd-missile-head-{}-{ticks}",
        std::process::id()
    ));
    std::fs::create_dir_all(&scratch).expect("creating the scratch directory");
    let script = scratch.join("fire.inputs");
    std::fs::write(&script, "285 none\n1 square\n400 none\n").expect("the input script");
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"))
        .arg(image)
        .args([
            "--race",
            "--no-audio",
            "--no-soundtrack",
            "--no-effects",
            "--size",
            "960x544",
            "--opponents",
        ])
        .args(["--render-scale", "100", "--msaa", "off"])
        .args(["--screen-filter", "off", "--anisotropy", "off"])
        .args(["--motion-blur", "off", "--ticks", &ticks.to_string()])
        .args(["--give", "missile"])
        .arg("--input-script")
        .arg(&script)
        .arg("--screenshot")
        .arg(scratch.join("frame.png"))
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"))
        .output()
        .expect("running oag-game");
    assert!(output.status.success());
    std::fs::remove_dir_all(&scratch).ok();
    let log = String::from_utf8_lossy(&output.stderr).into_owned()
        + &String::from_utf8_lossy(&output.stdout);
    let line = log
        .lines()
        .find(|line| line.contains(&format!("frame at tick {ticks}:")))
        .unwrap_or_else(|| panic!("no frame line in the log:\n{log}"));
    let number = |after: &str, before: &str| -> u32 {
        let from = line.find(after).map_or(0, |at| at + after.len());
        let tail = &line[from..];
        tail[..tail.find(before).expect("the line's shape")]
            .trim()
            .parse()
            .expect("a count")
    };
    Some((
        number(&format!("frame at tick {ticks}:"), " draw(s)"),
        number("culled, ", " triangle(s)"),
    ))
}

/// A Missile in flight draws its two heads: `HD_missile_ball_bloomring`'s two
/// 400-triangle draws each, in place of the glow billboard.
///
/// The frame's own count is the check, not a pixel region: the heads sit under
/// the `WO_MISSILE_HEAD` particle balls and change about 4 % of the pixels around
/// them, which no threshold separates across adapters, while the draw and
/// triangle counts are exact. Fired at tick 286: 902 draws and 448,723
/// triangles at tick 280 and at 292 with the head draw dropped; 906 and
/// 450,323 with it.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU adapter"]
fn a_missile_in_flight_draws_its_two_heads() {
    let (Some((before_draws, before_triangles)), Some((flying_draws, flying_triangles))) =
        (submitted_at(280), submitted_at(292))
    else {
        return;
    };
    println!(
        "draws {before_draws} -> {flying_draws}, triangles {before_triangles} -> {flying_triangles}"
    );
    assert_eq!(
        flying_draws - before_draws,
        4,
        "two heads of two draws each"
    );
    assert_eq!(
        flying_triangles - before_triangles,
        1600,
        "two heads of 800 triangles each"
    );
}

/// Pixels of 522,240 the forced hit changes at the least. The volley left to fly strikes a craft
/// of its own, whose blast's `glow_longer` washes the top of the frame, so brightness alone does
/// not tell the two apart.
const FORCED_BLAST_PIXELS: usize = 10_000;

/// Rival 1 put on one of the volley's Rockets after tick 289.
const FORCED_HIT: [&str; 2] = ["--force-rocket-hit", "289:1"];

/// Talon's Junction, the player firing one volley of Rockets, with `forced`'s flags.
fn rocket_frame(forced: &[&str]) -> Option<Vec<[u8; 3]>> {
    static RUNS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    let image = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")?;
    let scratch = std::env::temp_dir().join(format!(
        "oag-hd-rocket-blast-{}-{}",
        std::process::id(),
        RUNS.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&scratch).expect("creating the scratch directory");
    let out = scratch.join("frame.png");
    let script = scratch.join("fire.inputs");
    std::fs::write(&script, "285 none\n1 square\n400 none\n").expect("the input script");
    let mut game = std::process::Command::new(env!("CARGO_BIN_EXE_oag-game"));
    game.arg(image)
        .args(["--race", "--no-audio", "--no-soundtrack", "--no-effects"])
        .args(["--size", "960x544", "--opponents"])
        .args(["--render-scale", "100", "--msaa", "off"])
        .args(["--screen-filter", "off", "--anisotropy", "off"])
        .args(["--motion-blur", "off", "--ticks", "296", "--give", "rocket"]);
    game.args(forced);
    let status = game
        .arg("--input-script")
        .arg(&script)
        .arg("--screenshot")
        .arg(&out)
        .env("XDG_CONFIG_HOME", scratch.join("config"))
        .env("XDG_DATA_HOME", scratch.join("data"))
        .env("XDG_STATE_HOME", scratch.join("state"))
        .status()
        .expect("running oag-game");
    assert!(status.success());
    let frame = rgb_of(&std::fs::read(&out).expect("the frame"));
    std::fs::remove_dir_all(&scratch).ok();
    Some(frame)
}

/// The flag lands the hit at its tick: 0.1 s later the frame differs from the same volley left
/// to fly by the forced blast.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU adapter"]
fn force_rocket_hit_blows_a_rocket_on_the_named_craft_at_its_tick() {
    let forced = rocket_frame(&FORCED_HIT).expect("image");
    let free = rocket_frame(&[]).expect("image");
    let changed = forced
        .iter()
        .zip(&free)
        .filter(|(a, b)| a.iter().zip(b.iter()).any(|(x, y)| x.abs_diff(*y) > 64))
        .count();
    println!(
        "at tick 296: {changed} pixels changed; bright forced {}, left to fly {}",
        count(&forced, bright),
        count(&free, bright)
    );
    assert!(
        changed > FORCED_BLAST_PIXELS,
        "no blast from the forced hit: {changed} pixels changed"
    );
}

/// The velocity flag reaches the craft the forced hit strikes: the blast that carries it, and the
/// craft itself, stand elsewhere 7 ticks later. The two runs differ in nothing else.
#[cfg(feature = "dev-cli")]
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU adapter"]
fn force_rocket_hit_velocity_moves_what_the_forced_hit_draws() {
    let own = rocket_frame(&FORCED_HIT).expect("image");
    let thrown = rocket_frame(&[
        FORCED_HIT[0],
        FORCED_HIT[1],
        "--force-rocket-hit-velocity=-300,0,0",
    ])
    .expect("image");
    let changed = own
        .iter()
        .zip(&thrown)
        .filter(|(a, b)| a.iter().zip(b.iter()).any(|(x, y)| x.abs_diff(*y) > 64))
        .count();
    println!("at tick 296: {changed} pixels changed by the velocity");
    assert!(
        changed > FORCED_BLAST_PIXELS,
        "the velocity changed only {changed} pixels"
    );
}

/// `--force-rocket-hit-at` plays its blasts at its point from its tick: given at 289 the frame at
/// 296 holds them, given at a tick the run never reaches it is the frame of the volley left to fly.
/// The point is 15 units ahead of the player's craft at tick 289.
#[cfg(feature = "dev-cli")]
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso and a GPU adapter"]
fn force_rocket_hit_at_plays_its_blasts_from_its_tick() {
    let free = rocket_frame(&[]).expect("image");
    let forced =
        rocket_frame(&["--force-rocket-hit-at", "289:-116.8,-52,-175.4:3"]).expect("image");
    let never = rocket_frame(&["--force-rocket-hit-at", "400:-116.8,-52,-175.4:3"]).expect("image");
    let changed = forced
        .iter()
        .zip(&free)
        .filter(|(a, b)| a.iter().zip(b.iter()).any(|(x, y)| x.abs_diff(*y) > 64))
        .count();
    println!("at tick 296: {changed} pixels changed by the forced blasts");
    assert!(
        changed > FORCED_BLAST_PIXELS,
        "no blast at the forced point: {changed} pixels changed"
    );
    assert!(
        never == free,
        "a flag whose tick never comes changed the frame"
    );
}
