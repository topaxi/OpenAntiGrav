//! Wipeout HD's crossfaded engine, played off the real disc.
//!
//! **`#[ignore]`d and never run in CI.** They need game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test sfx_xfade_ground_truth --run-ignored all
//! # and keep the WAVs:
//! OAG_XFADE_WAV_DIR=data/scratch/hd-engine-wire OAG_REQUIRE_GAME_DATA=1 \
//!   cargo nextest run -p oag-game --test sfx_xfade_ground_truth --run-ignored all --no-capture
//! ```
//!
//! What it proves: every team's table resolves on the disc (each layer a
//! looping cue in `shiphd.bnk`), a craft at a standstill and a craft at speed
//! sound different through a real mixer, and the throttle layer is silent
//! without throttle. What it does not prove: that the result matches the
//! original by ear, which needs an RPCS3 capture.

use std::path::Path;
use std::sync::Arc;

use oag_audio::{Listener, Mixer};
use oag_sound::sfx::{Banks, XfadeCraft, XfadeInputs, XfadeTeam};

const SAMPLE_RATE: u32 = 48_000;
const TICK_HZ: u32 = 60;

/// The thirteen tables, as the slot-team strings that select them.
const TEAMS: [&str; 13] = [
    "ag_systems",
    "assegai",
    "auricom",
    "detonator",
    "egx",
    "feisar",
    "goteki",
    "harimau",
    "icaras",
    "mirage",
    "piranha",
    "qirex",
    "triakis",
];

fn load(teams: &[String]) -> Option<(Banks, Vec<String>)> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images/hdfury-ps3-eu-dec.iso");
    if !path.exists() {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but {} is missing",
            path.display()
        );
        println!("skipping: {} not present", path.display());
        return None;
    }
    let opened =
        oag_source::title::open_source(&path.display().to_string(), Vec::new(), Vec::new())
            .expect("opening the source");
    let sounds = opened.title.race.sounds;
    let mut archives = opened.archives;
    let mut banks = Banks::load(
        &mut archives,
        sounds,
        false,
        oag_title::SequenceTick::Unknown,
    );
    let mut report = Vec::new();
    banks.load_xfade(
        &mut archives,
        oag_sound::sfx::XfadeSource {
            bank: sounds.ship,
            infix: "",
        },
        teams,
        &mut report,
    );
    Some((banks, report))
}

fn all_teams() -> Vec<String> {
    TEAMS.iter().map(|t| (*t).to_string()).collect()
}

/// A listener at the origin, and the craft on top of it: full gain, no pan.
fn ears() -> Listener {
    Listener {
        position: [0.0, 0.0, 0.0],
        right: [1.0, 0.0, 0.0],
    }
}

/// Runs one craft through `speeds_kmh` (one value per tick), rendering through
/// a real mixer, and returns the interleaved stereo samples.
fn render(team: Arc<XfadeTeam>, speeds_kmh: &[f32], throttle: &[f32]) -> (Vec<f32>, Vec<usize>) {
    let mut mixer = Mixer::new(SAMPLE_RATE);
    let mut craft = XfadeCraft::new(team);
    let mut out = Vec::new();
    let mut open = Vec::new();
    for (tick, speed) in speeds_kmh.iter().enumerate() {
        craft.tick(
            &mut mixer,
            XfadeInputs {
                speed_field: speed * 1.5,
                throttle: Some(throttle[tick]),
            },
            true,
            [0.0, 0.0, 0.0],
            &ears(),
            false,
            1.0 / TICK_HZ as f32,
        );
        mixer.render_tick(TICK_HZ, &mut out);
        open.push(craft.open_voices());
    }
    (out, open)
}

/// The mean frequency, weighted by magnitude, of a stretch of mono samples.
fn centroid(mono: &[f32]) -> f32 {
    // A plain DFT over a decimated grid of bins: enough to see a note move.
    let n = mono.len().min(8192);
    let x = &mono[..n];
    let mut weighted = 0.0f64;
    let mut total = 0.0f64;
    for bin in (4..n / 2).step_by(2) {
        let (mut re, mut im) = (0.0f64, 0.0f64);
        for (i, s) in x.iter().enumerate() {
            let w = 0.5 - 0.5 * (2.0 * std::f64::consts::PI * i as f64 / n as f64).cos();
            let a = 2.0 * std::f64::consts::PI * bin as f64 * i as f64 / n as f64;
            re += f64::from(*s) * w * a.cos();
            im -= f64::from(*s) * w * a.sin();
        }
        let mag = (re * re + im * im).sqrt();
        weighted += mag * (bin as f64 * f64::from(SAMPLE_RATE) / n as f64);
        total += mag;
    }
    (weighted / total.max(1e-12)) as f32
}

fn mono(stereo: &[f32]) -> Vec<f32> {
    stereo.chunks(2).map(|f| 0.5 * (f[0] + f[1])).collect()
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_team_table_resolves_to_looping_cues() {
    let Some((banks, report)) = load(&all_teams()) else {
        return;
    };
    for line in &report {
        println!("{line}");
    }
    for team in TEAMS {
        let table = banks
            .xfade_team(team)
            .unwrap_or_else(|| panic!("{team}: no table loaded"));
        assert_eq!(
            table.playable(),
            table.table().layer_count(),
            "{team}: a layer did not resolve to a looping cue"
        );
        let expected = if team == "feisar" { 8 } else { 9 };
        assert_eq!(table.table().layer_count(), expected, "{team}");
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_craft_at_speed_sounds_different_from_one_at_rest() {
    let Some((banks, _)) = load(&["goteki".to_string()]) else {
        return;
    };
    let team = banks.xfade_team("goteki").expect("goteki table");

    // Two seconds at rest, then a ten second climb to 400 km/h at full throttle,
    // then two seconds held.
    let rest = 2 * TICK_HZ as usize;
    let climb = 10 * TICK_HZ as usize;
    let hold = 2 * TICK_HZ as usize;
    let mut speeds = vec![0.0; rest];
    speeds.extend((0..climb).map(|i| 400.0 * i as f32 / climb as f32));
    speeds.extend(vec![400.0; hold]);
    let mut throttle = vec![0.0; rest];
    throttle.extend(vec![100.0; climb + hold]);

    // The layer table at three speeds, straight off the smoother.
    for (label, speed, thr) in [
        ("rest", 0.0, 0.0),
        ("200 km/h", 200.0, 100.0),
        ("400 km/h", 400.0, 100.0),
    ] {
        let mut mixer = Mixer::new(SAMPLE_RATE);
        let mut c = XfadeCraft::new(Arc::clone(&team));
        for _ in 0..240 {
            c.tick(
                &mut mixer,
                XfadeInputs {
                    speed_field: speed * 1.5,
                    throttle: Some(thr),
                },
                true,
                [0.0; 3],
                &ears(),
                false,
                1.0 / TICK_HZ as f32,
            );
        }
        let row: Vec<String> = (0..team.table().layer_count())
            .map(|n| {
                let l = c.level(n).unwrap();
                format!(
                    "{} g{:.2} b{}",
                    team.table().layer_name(n).unwrap(),
                    l.gain,
                    l.bend
                )
            })
            .collect();
        println!("{label}: {}", row.join(" | "));
    }

    let (stereo, open) = render(Arc::clone(&team), &speeds, &throttle);
    let m = mono(&stereo);
    let per_tick = (SAMPLE_RATE / TICK_HZ) as usize;
    let window = |from_tick: usize| &m[from_tick * per_tick..(from_tick + 30) * per_tick];

    let rms = |x: &[f32]| (x.iter().map(|s| s * s).sum::<f32>() / x.len() as f32).sqrt();
    let at_rest = window(30);
    let at_speed = window(rest + climb + 20);
    let (c_rest, c_speed) = (centroid(at_rest), centroid(at_speed));
    println!(
        "rest: rms {:.4} centroid {c_rest:.0} Hz, voices {}",
        rms(at_rest),
        open[40]
    );
    println!(
        "400 km/h: rms {:.4} centroid {c_speed:.0} Hz, voices {}",
        rms(at_speed),
        open[rest + climb + 40]
    );
    for second in 0..12 {
        let w = &m[second * SAMPLE_RATE as usize..(second + 1) * SAMPLE_RATE as usize];
        println!(
            "t={second:>2}s rms {:.4} centroid {:.0} Hz voices {}",
            rms(w),
            centroid(w),
            open[second * TICK_HZ as usize + 30]
        );
    }

    if let Some(dir) = std::env::var_os("OAG_XFADE_WAV_DIR") {
        let path = Path::new(&dir).join("xfade-goteki-ramp.wav");
        std::fs::write(&path, oag_audio::wav::from_samples(&stereo, SAMPLE_RATE)).expect("wav");
        println!("wrote {}", path.display());
    }

    assert!(rms(at_rest) > 0.001, "the standing craft is silent");
    assert!(rms(at_speed) > 0.001, "the fast craft is silent");
    assert!(
        (c_speed - c_rest).abs() > 50.0,
        "the note did not move: {c_rest:.0} Hz at rest, {c_speed:.0} Hz at speed"
    );
    assert!(open.iter().all(|&n| n <= team.table().layer_count()));
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_afterburner_layer_follows_the_throttle() {
    let Some((banks, _)) = load(&["goteki".to_string()]) else {
        return;
    };
    let team = banks.xfade_team("goteki").expect("goteki table");
    let mut mixer = Mixer::new(SAMPLE_RATE);
    let mut craft = XfadeCraft::new(team);
    let mut run = |throttle: f32, ticks: usize| {
        for _ in 0..ticks {
            craft.tick(
                &mut mixer,
                XfadeInputs {
                    speed_field: 300.0,
                    throttle: Some(throttle),
                },
                true,
                [0.0, 0.0, 0.0],
                &ears(),
                false,
                1.0 / TICK_HZ as f32,
            );
        }
        // The last layer on goteki is channel 3's `~afterburner`.
        craft.level(8).expect("nine layers")
    };
    let off = run(0.0, 120);
    let on = run(100.0, 120);
    println!("afterburner layer: off {off:?}, on {on:?}");
    assert_eq!(off.gain, 0.0, "the afterburner sounds with no throttle");
    assert!(on.gain > 0.5, "the afterburner is silent at full throttle");
}

/// Eight craft on the measured grid, one team each, all at rest and then all at
/// speed: the engines alone must neither clip nor take the whole voice pool.
///
/// The distances are the ones the eight `slot+4` words were read at (8.7 to 148
/// units behind or ahead of the listener), so the original's own grid decides
/// how many layers are audible at once. Every other cue of a race (the
/// ambience, the countdown, a collision) needs a voice too, so the engines get
/// to take no more than 96 of the 128 an HD race has.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_full_grid_neither_clips_nor_starves_the_voice_pool() {
    let Some((banks, _)) = load(&all_teams()) else {
        return;
    };
    let distances = [8.7, 34.4, 47.5, 69.8, 86.4, 109.2, 126.8, 147.6];
    let mut mixer = Mixer::new(SAMPLE_RATE);
    mixer.grow_pool(oag_audio::mixer::HD_VOICES);
    let mut crafts: Vec<XfadeCraft> = TEAMS
        .iter()
        .take(8)
        .map(|t| XfadeCraft::new(banks.xfade_team(t).expect("table")))
        .collect();
    let mut peak = 0.0f32;
    let mut most_voices = 0;
    let mut out = Vec::new();
    for (label, speed, throttle) in [("rest", 0.0, 0.0), ("400 km/h", 400.0, 100.0)] {
        for tick in 0..240 {
            for (slot, craft) in crafts.iter_mut().enumerate() {
                craft.tick(
                    &mut mixer,
                    XfadeInputs {
                        speed_field: speed * 1.5,
                        throttle: (slot == 0).then_some(throttle),
                    },
                    true,
                    [distances[slot], 0.0, 0.0],
                    &ears(),
                    false,
                    1.0 / TICK_HZ as f32,
                );
            }
            out.clear();
            mixer.render_tick(TICK_HZ, &mut out);
            if tick >= 60 {
                peak = peak.max(out.iter().fold(0.0f32, |m, s| m.max(s.abs())));
            }
            most_voices =
                most_voices.max(crafts.iter().map(XfadeCraft::open_voices).sum::<usize>());
        }
        println!("{label}: peak so far {peak:.3}, most voices {most_voices}");
    }
    assert!(peak < 1.0, "eight engines alone clip: peak {peak}");
    assert!(
        most_voices <= 96,
        "eight engines hold {most_voices} of the 128 voices"
    );
}
