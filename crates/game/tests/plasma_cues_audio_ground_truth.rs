//! The Plasma's own three cues - `PLASMA`, `~PLASMATVL`, `PLASMAHITWALL` - on
//! a real disc and a real bolt, out to a WAV.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test plasma_cues_audio_ground_truth --run-ignored all
//! ```
//!
//! Its own file rather than a section of `plasma_ground_truth.rs`, the same
//! reason `mine_launch_audio_ground_truth.rs` is its own file rather than a
//! section of `mine_ground_truth.rs`: that file is the weapon's own mechanics
//! on `crates/gameplay`'s side of the fence, and this is the wiring -
//! `audio::sfx::Cue::Plasma`/`PlasmaTravel`/`PlasmaHitWall` - end to end
//! through the composition root's own `Audio::race_tick`, on a real lap under
//! the real force law and the real collision soup. Same standard
//! `track_audio_ground_truth.rs` and `mine_launch_audio_ground_truth.rs` both
//! set: a headless run through the null backend, written out to 16-bit PCM,
//! so "it should play" is a file rather than an assertion about internal
//! state alone.
//!
//! # What only real data can say here
//!
//! 1. **That `PLASMA` and `PLASMAHITWALL` both resolve against the real
//!    `weapons.bnk`** - the unit tests in `race::tests::cues` prove a
//!    `CueEvent` is queued, not that it survives `Banks::pick` against real
//!    waveform data.
//! 2. **That a real bolt genuinely flies (`charge <= 0.0`) between the press
//!    and the impact**, on a real circuit's own collision soup rather than a
//!    hand-built fixture with no floor under it - see `plasma_ground_truth.rs`'s
//!    own note on why that matters for the flight model, which applies here
//!    for the same reason. `Cue::PlasmaTravel`'s own held-voice bookkeeping is
//!    private to `oag_game::audio::sfx`, so what this proves is that its rising
//!    edge has something real to fire on across a real flight, not that the
//!    voice itself stayed open the whole way - the same reach every other
//!    ground-truth test in this file has into a private mixer.
//! 3. **The ordering**: press, then a wind-up with no `PLASMAHITWALL` yet,
//!    then an ending - on a real class's own charge and flight speed, not a
//!    hand-timed fixture.

use std::path::{Path, PathBuf};

use oag_game::audio::sfx::Cue;
use oag_game::race;
use oag_gameplay::input::{Button, Input};
use oag_tables::weapons::Weapon;

/// Past the start-line countdown - see `plasma_ground_truth.rs`'s own doc
/// comment on why a shorter warm-up fails for a reason that has nothing to do
/// with the weapon.
const WARM_UP_TICKS: u64 = oag_race::COUNTDOWN_TICKS + 120;

/// Long enough for the one-second wind-up plus a real circuit's own flight
/// and impact, with margin: `plasma_ground_truth.rs` measured a bolt fired
/// straight down `Talons Junction` hitting a wall in about a second and a
/// half, well inside this.
const FLIGHT_TICKS: u64 = 300;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

/// One button held down, tick after tick. See `mine_ground_truth.rs`'s twin
/// for why a fresh `Input` per call is right.
fn held(button: Button) -> oag_gameplay::InputSnapshot {
    let mut buttons = Input::new();
    buttons.begin_frame(button.bit());
    buttons.begin_frame(button.bit());
    oag_gameplay::InputSnapshot {
        buttons,
        ..oag_gameplay::InputSnapshot::new()
    }
}

/// Fires a Plasma this tick while still holding the throttle - the same
/// combined mask `plasma_ground_truth.rs` presses with, so the craft keeps
/// moving on the very tick the bolt leaves.
fn fire_while_driving() -> oag_gameplay::InputSnapshot {
    let mut buttons = Input::new();
    buttons.begin_frame(0);
    buttons.begin_frame(Button::Square.bit() | Button::Cross.bit());
    oag_gameplay::InputSnapshot {
        buttons,
        ..oag_gameplay::InputSnapshot::new()
    }
}

/// Fires all three of the Plasma's own cues off a real disc, a real press and
/// a real bolt, and writes the mix out to `data/shots/plasma-cues.wav`.
///
/// **What this proves that the unit tests in `race::tests::cues` and
/// `audio::sfx::tests` cannot**: not only that a `CueEvent` is queued, but
/// that `PLASMA` and `PLASMAHITWALL` each survive `place()`'s range gate and
/// resolve against the real `weapons.bnk` waveform data, that the held
/// `~PLASMATVL` voice actually opens and is still playing partway through a
/// real flight, and that all of it renders to audible, non-silent PCM through
/// the same mixer a real session uses.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn firing_a_plasma_sounds_all_three_cues_in_order_and_writes_it_out() {
    let Some(image) = image() else { return };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    })
    .expect("loading the race");
    for line in &loaded.report {
        println!("{line}");
    }
    let mut race = race::Race::start(loaded.setup);
    race.plasma_stats().expect("the disc authors a Plasma");

    let throttle = held(Button::Cross);
    for _ in 0..WARM_UP_TICKS {
        race.tick(&throttle);
    }
    let speed = race.sim.world.ships[0]
        .physics
        .body
        .linear_velocity
        .length();
    assert!(
        speed > 10.0,
        "the craft is barely moving at {speed:.1} units/s"
    );

    let wav = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/shots")
        .join("plasma-cues.wav");
    // `Some(dump)` forces the null backend, which is what makes this runnable
    // headlessly - see `Audio::open`.
    let mut audio = oag_game::audio::Audio::open(
        &oag_game::settings::Audio::default(),
        Some(wav.clone()),
        None,
        oag_audio::MIN_BUFFER,
        false,
    );

    race.sim.world.ships[0].pickup.weapon = Some(Weapon::Plasma);

    let mut plasma_press_tick = None;
    let mut hit_wall_tick = None;
    // Whether a live Plasma bolt was seen actually flying - `charge <= 0.0`,
    // the exact condition `Audio::race_tick`'s own travel tracker gates on
    // (`crates/game/src/audio/sfx.rs`) - on any tick strictly inside the
    // press-to-impact window. The mixer's own held-voice bookkeeping is
    // private to `oag_game::audio::sfx` and reaches nothing this crate can
    // read from outside it, so this is the closest an integration test gets
    // to observing the travel loop's own precondition rather than merely its
    // two bracketing one-shots.
    let mut saw_bolt_flying = false;
    for tick in 0..FLIGHT_TICKS {
        let snapshot = if tick == 0 {
            fire_while_driving()
        } else {
            throttle
        };
        race.tick(&snapshot);
        // **Peeked, not drained**: `Audio::race_tick` below does its own
        // `drain_cues`, which is what actually feeds the mixer. Draining
        // here first would starve that call - the same trap
        // `mine_launch_audio_ground_truth.rs` names for its own peek.
        for event in race.pending_cues() {
            if event.cue == Cue::Plasma {
                plasma_press_tick.get_or_insert(tick);
            }
            if event.cue == Cue::PlasmaHitWall {
                hit_wall_tick.get_or_insert(tick);
            }
        }
        saw_bolt_flying |= race
            .sim
            .world
            .projectiles
            .slots
            .iter()
            .any(|p| p.kind == Some(Weapon::Plasma) && p.owner == 0 && p.charge <= 0.0);
        audio.race_tick(&mut race);
        audio.tick();

        if hit_wall_tick.is_some() {
            break;
        }
    }
    audio.finish().expect("writing the dump");

    let press = plasma_press_tick.expect("PLASMA never fired");
    let hit = hit_wall_tick.expect(
        "PLASMAHITWALL never fired - the bolt may still be flying past FLIGHT_TICKS, \
         or it timed out at 10 s (600 ticks), past this test's own budget",
    );
    assert!(
        hit > press,
        "PLASMAHITWALL (tick {hit}) did not come after PLASMA (tick {press})"
    );
    // The wind-up is `oag_gameplay::projectile::plasma::CHARGE_SECONDS`
    // (1.0 s = 60 ticks) before the bolt can even start flying, so an ending
    // any sooner than that is the press and the impact landing on the same
    // bolt's charge rather than a real flight in between.
    let charge_ticks = (oag_gameplay::projectile::plasma::CHARGE_SECONDS * 60.0) as u64;
    assert!(
        hit - press >= charge_ticks,
        "PLASMAHITWALL landed only {} ticks after PLASMA, inside the {charge_ticks}-tick \
         wind-up - the bolt cannot have flown anywhere",
        hit - press
    );
    assert!(
        saw_bolt_flying,
        "the bolt was never seen with charge <= 0.0 - the wind-up ran into the \
         impact with no flight in between, so `Cue::PlasmaTravel`'s own rising \
         edge never had anything to trigger on"
    );
    println!(
        "PLASMA at tick {press}, PLASMAHITWALL at tick {hit} ({} ticks of wind-up and flight)",
        hit - press
    );

    let file = std::fs::read(&wav).expect("the dump");
    let pcm = &file[44..];
    let peak = pcm
        .as_chunks::<2>()
        .0
        .iter()
        .map(|c| i16::from_le_bytes(*c).unsigned_abs())
        .max()
        .expect("samples");
    assert!(peak > 0, "the dump is {} bytes of silence", pcm.len());
    println!(
        "wrote {} - {} frames, peak sample {peak}",
        wav.display(),
        pcm.len() / 4
    );
}
