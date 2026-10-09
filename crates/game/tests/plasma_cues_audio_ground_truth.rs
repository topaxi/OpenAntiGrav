//! The Plasma's own four cues - `PLASMA`, `~PLASMATVL`, `PLASMAHITWALL` and
//! `PLASMAHITSHIP` - on a real disc and a real bolt, out to a WAV.
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
//! `audio::sfx::Cue::Plasma`/`PlasmaTravel`/`PlasmaHitWall`/`PlasmaHitShip` -
//! end to end through the composition root's own `Audio::race_tick`, on a
//! real lap under the real force law and the real collision soup. Same
//! standard `track_audio_ground_truth.rs` and `mine_launch_audio_ground_truth.rs`
//! both set: a headless run through the null backend, written out to 16-bit
//! PCM, so "it should play" is a file rather than an assertion about internal
//! state alone.
//!
//! # What only real data can say here
//!
//! 1. **That `PLASMA`, `PLASMAHITWALL` and `PLASMAHITSHIP` all resolve
//!    against the real `weapons.bnk`** - the unit tests in `race::tests::cues`
//!    prove a `CueEvent` is queued, not that it survives `Banks::pick` against
//!    real waveform data. The second test in this file (craft hit) is what
//!    settles it for `PLASMAHITSHIP` specifically, since the first (wall hit)
//!    never reaches that branch.
//! 2. **That a real bolt genuinely flies (`charge <= 0.0`) between the press
//!    and the impact**, on a real circuit's own collision soup rather than a
//!    hand-built fixture with no floor under it - see `plasma_ground_truth.rs`'s
//!    own note on why that matters for the flight model, which applies here
//!    for the same reason. `Cue::PlasmaTravel`'s own held-voice bookkeeping is
//!    private to `oag_sound::sfx`, so what this proves is that its rising
//!    edge has something real to fire on across a real flight, not that the
//!    voice itself stayed open the whole way - the same reach every other
//!    ground-truth test in this file has into a private mixer.
//! 3. **The ordering**: press, then a wind-up with no ending cue yet, then an
//!    ending - on a real class's own charge and flight speed, not a
//!    hand-timed fixture.

use std::path::{Path, PathBuf};

use oag_gameplay::PlayerInputs;
use oag_gameplay::input::{Button, Input};
use oag_raceplay as race;
use oag_sound::sfx::Cue;
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

/// Fires the Plasma's own press/travel/ending cues off a real disc, a real
/// press and a real bolt, and writes the mix out to a `.wav` under `data/shots/`.
///
/// **What this proves that the unit tests in `race::tests::cues` and
/// `audio::sfx::tests` cannot**: not only that a `CueEvent` is queued, but
/// that `PLASMA` and the ending cue it reaches each survive `place()`'s range
/// gate and resolve against the real `weapons.bnk` waveform data, that the
/// held `~PLASMATVL` voice actually opens and is still playing partway
/// through a real flight, and that all of it renders to audible, non-silent
/// PCM through the same mixer a real session uses.
///
/// **The ending is whichever one a real race reaches, not `PLASMAHITWALL`
/// specifically** - see the assertion loop's own comment for why a
/// straight-ahead shot can meet a real opponent instead of a wall on this
/// exact fixture.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn firing_a_plasma_sounds_its_press_travel_and_ending_cues_and_writes_it_out() {
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
        race.tick(&PlayerInputs::single(throttle));
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
    let mut audio = oag_sound::Audio::open(
        &oag_sound::settings::Settings::default(),
        Some(wav.clone()),
        None,
        oag_audio::MIN_BUFFER,
        false,
    );

    race.sim.world.ships[0].pickup.weapon = Some(Weapon::Plasma);

    let mut plasma_press_tick = None;
    // **Either ending, not `PlasmaHitWall` specifically.** `Mode::SingleRace`
    // fields a full eight-craft grid unconditionally
    // (`oag_race::Mode::has_opponents`) whenever the track authors a `Start
    // Position`, which every shipped circuit does - `race::Options::opponents`
    // has no say in it. So a bolt fired straight ahead early in the race can
    // meet a real opponent instead of the wall it was originally assumed to
    // reach: measured on this exact fixture, it does, at tick 82, the tick
    // `PlasmaHitWall` used to be asserted at before `Cue::PlasmaHitShip`
    // existed to tell the two endings apart. Both are the recovered ending for
    // *some* case (`docs/ghidra/functions/psp-pulse-usa/plasma.md`'s "a craft
    // hit is the third ending"), so this only asserts that exactly one of the
    // two fires, not which - `firing_a_plasma_at_a_craft_sounds_plasmahitship_not_plasmahitwall`
    // below is what pins the craft-hit case down deterministically.
    let mut hit_tick = None;
    let mut hit_cue = None;
    // Whether a live Plasma bolt was seen actually flying - `charge <= 0.0`,
    // the exact condition `Audio::race_tick`'s own travel tracker gates on
    // (`crates/sound/src/sfx.rs`) - on any tick strictly inside the
    // press-to-impact window. The mixer's own held-voice bookkeeping is
    // private to `oag_sound::sfx` and reaches nothing this crate can
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
        race.tick(&PlayerInputs::single(snapshot));
        // **Peeked, not drained**: `Audio::race_tick` below does its own
        // `drain_cues`, which is what actually feeds the mixer. Draining
        // here first would starve that call - the same trap
        // `mine_launch_audio_ground_truth.rs` names for its own peek.
        for event in race.pending_cues() {
            if event.cue == Cue::Plasma {
                plasma_press_tick.get_or_insert(tick);
            }
            if event.cue == Cue::PlasmaHitWall || event.cue == Cue::PlasmaHitShip {
                hit_tick.get_or_insert(tick);
                hit_cue.get_or_insert(event.cue);
            }
        }
        saw_bolt_flying |= race
            .sim
            .world
            .projectiles
            .slots
            .iter()
            .any(|p| p.kind == Some(Weapon::Plasma) && p.owner == 0 && p.charge <= 0.0);
        oag_game::sound::race_tick(&mut audio, &mut race);
        audio.tick();

        if hit_tick.is_some() {
            break;
        }
    }
    audio.finish().expect("writing the dump");

    let press = plasma_press_tick.expect("PLASMA never fired");
    let hit = hit_tick.expect(
        "neither PLASMAHITWALL nor PLASMAHITSHIP ever fired - the bolt may still be \
         flying past FLIGHT_TICKS, or it timed out at 10 s (600 ticks), past this \
         test's own budget",
    );
    let cue = hit_cue.expect("hit_tick was set without hit_cue");
    assert!(
        hit > press,
        "{} (tick {hit}) did not come after PLASMA (tick {press})",
        cue.name()
    );
    // The wind-up is `oag_weapons::projectile::plasma::CHARGE_SECONDS`
    // (1.0 s = 60 ticks) before the bolt can even start flying, so an ending
    // any sooner than that is the press and the impact landing on the same
    // bolt's charge rather than a real flight in between.
    let charge_ticks = (oag_weapons::projectile::plasma::CHARGE_SECONDS * 60.0) as u64;
    assert!(
        hit - press >= charge_ticks,
        "{} landed only {} ticks after PLASMA, inside the {charge_ticks}-tick \
         wind-up - the bolt cannot have flown anywhere",
        cue.name(),
        hit - press
    );
    assert!(
        saw_bolt_flying,
        "the bolt was never seen with charge <= 0.0 - the wind-up ran into the \
         impact with no flight in between, so `Cue::PlasmaTravel`'s own rising \
         edge never had anything to trigger on"
    );
    println!(
        "PLASMA at tick {press}, {} at tick {hit} ({} ticks of wind-up and flight)",
        cue.name(),
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

/// The fourth Plasma cue, `PLASMAHITSHIP`, against the same real
/// `weapons.bnk` - a craft hit rather than a wall.
///
/// **What only real data can say here, on top of what the sibling test
/// above already covers**: that `PLASMAHITSHIP` itself resolves against the
/// disc's own bank data, not only that `Cue::PlasmaHitShip` is queued - the
/// unit test in `race::tests::cues`
/// (`a_plasma_that_hits_a_craft_raises_plasmahitship_and_not_plasmahitwall`)
/// proves the queueing on a synthetic fixture with no bank at all, which is
/// exactly the gap this project's own `oag-wad sounds` check cannot close by
/// itself: confirming a cue string is *in* `weapons.bnk` is not the same as
/// confirming `Banks::pick` resolves it in this engine's own load path.
///
/// **Slot 1 is a real opponent already, not a craft activated for this
/// test.** `Mode::SingleRace` fields a full eight-craft grid unconditionally
/// whenever the track authors a `Start Position` - see the sibling test
/// above's own doc comment, corrected the same way once this was measured
/// directly on that fixture. So this repurposes slot 1 as a deterministic
/// point-blank target exactly the way
/// `a_plasma_bolt_that_times_out_far_above_the_track_hurts_nobody_below_it`
/// (`plasma_ground_truth.rs`) repurposes it as a blast sentinel: teleported
/// every tick, before `Race::tick` runs, rather than left to its own AI
/// driving. It is re-pinned to `ships[0]`'s own `body.forward() * 15.0` -
/// ahead of the *moving* player rather than a fixed world point, since this
/// test drives forward the same way the sibling test does - so the
/// swept-sphere hull test in `oag_weapons::projectile::geometry::nearest_hit`
/// always has a target in its own flight path. Its `handling` is already the
/// grid's own `VENOM` copy (`Race::start` gives every opponent the player's
/// own handling), so nothing here has to set it.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn firing_a_plasma_at_a_craft_sounds_plasmahitship_not_plasmahitwall() {
    let Some(image) = image() else { return };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    race.plasma_stats().expect("the disc authors a Plasma");
    assert!(
        race.sim.world.ship_count > 1,
        "a Single Race needs at least one opponent to repurpose as a target"
    );
    assert!(
        race.sim.world.ships[1].active,
        "slot 1 is not an active opponent"
    );

    let throttle = held(Button::Cross);
    for _ in 0..WARM_UP_TICKS {
        race.tick(&PlayerInputs::single(throttle));
    }

    let wav = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/shots")
        .join("plasma-hitship.wav");
    let mut audio = oag_sound::Audio::open(
        &oag_sound::settings::Settings::default(),
        Some(wav.clone()),
        None,
        oag_audio::MIN_BUFFER,
        false,
    );

    race.sim.world.ships[0].pickup.weapon = Some(Weapon::Plasma);

    let mut plasma_press_tick = None;
    let mut hit_ship_tick = None;
    let mut hit_wall_tick = None;
    for tick in 0..FLIGHT_TICKS {
        // Re-pinned every tick, ahead of the moving player rather than a
        // fixed world point - see this test's own doc comment.
        let forward = race.sim.world.ships[0].physics.body.forward();
        race.sim.world.ships[1].physics.body.position =
            race.sim.world.ships[0].physics.body.position + forward * 15.0;
        race.sim.world.ships[1].physics.body.linear_velocity =
            race.sim.world.ships[0].physics.body.linear_velocity;

        let snapshot = if tick == 0 {
            fire_while_driving()
        } else {
            throttle
        };
        race.tick(&PlayerInputs::single(snapshot));
        for event in race.pending_cues() {
            if event.cue == Cue::Plasma {
                plasma_press_tick.get_or_insert(tick);
            }
            if event.cue == Cue::PlasmaHitShip {
                hit_ship_tick.get_or_insert(tick);
            }
            if event.cue == Cue::PlasmaHitWall {
                hit_wall_tick.get_or_insert(tick);
            }
        }
        oag_game::sound::race_tick(&mut audio, &mut race);
        audio.tick();

        if hit_ship_tick.is_some() || hit_wall_tick.is_some() {
            break;
        }
    }
    audio.finish().expect("writing the dump");

    let press = plasma_press_tick.expect("PLASMA never fired");
    assert!(
        hit_wall_tick.is_none(),
        "PLASMAHITWALL fired on a craft hit (tick {:?}) - the target craft was \
         missed and the bolt reached the real track's own geometry instead",
        hit_wall_tick
    );
    let hit = hit_ship_tick.expect(
        "PLASMAHITSHIP never fired - the bolt may have missed the target craft \
         within this test's own FLIGHT_TICKS budget",
    );
    let charge_ticks = (oag_weapons::projectile::plasma::CHARGE_SECONDS * 60.0) as u64;
    assert!(
        hit - press >= charge_ticks,
        "PLASMAHITSHIP landed only {} ticks after PLASMA, inside the {charge_ticks}-tick \
         wind-up - the bolt cannot have flown anywhere",
        hit - press
    );
    println!(
        "PLASMA at tick {press}, PLASMAHITSHIP at tick {hit} ({} ticks)",
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
