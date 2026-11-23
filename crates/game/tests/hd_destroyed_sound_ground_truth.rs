//! A destroyed craft on a real Wipeout HD disc plays the original's sounds: `EXPLBIG_PC` and the
//! looped `~SHIP_DESTROYED` for the local player, `EXPLBIG` for another craft, `~SHIP_ONFIRE` on
//! both, `~SHIP_DESTROYED` stopped at state 6 and `~SHIP_ONFIRE` at the craft's return
//! (`docs/ghidra/functions/ps3-hdfury-eu/ship-destroy.md`, rows 3, 12 and 15).
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not ship.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'test(/^hd_destroyed_sound_ground_truth::/)'
//! ```
//!
//! What it pins that the unit tests in `oag-raceplay` cannot: that `weapons.bnk` carries the four
//! cues, that the mixer opens a voice for each held loop and lets it go on the stop edge, and that
//! dropping the cues leaves the craft silent.

use oag_gameplay::PlayerInputs;
use oag_gameplay::cue::Cue;
use oag_physics::CraftState;
use oag_raceplay as race;

const PLAYER: usize = 0;
const OPPONENT: usize = 3;

fn load() -> Option<race::Race> {
    let image = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    })
    .expect("loading the race");
    for name in ["EXPLBIG_PC", "EXPLBIG", "~SHIP_DESTROYED", "~SHIP_ONFIRE"] {
        assert!(
            loaded
                .report
                .iter()
                .any(|line| line.starts_with(&format!("sfx: {name} -> "))),
            "{name} did not resolve in the HD banks"
        );
    }
    let mut race = race::Race::start(loaded.setup, loaded.view, loaded.modules);
    race.core.sim.set_autopilot(true);
    Some(race)
}

fn audio() -> oag_sound::Audio {
    oag_sound::Audio::open(
        &oag_sound::settings::Settings::default(),
        Some(std::path::PathBuf::from("/dev/null")),
        None,
        oag_audio::MIN_BUFFER,
        false,
    )
}

/// One tick with `slot` set to `state`, its cues returned as `(cue, slot)`, and the mixer's voice
/// count after the tick. `keep` filters the cues the audio hears.
fn step(
    race: &mut race::Race,
    audio: &mut oag_sound::Audio,
    slot: usize,
    state: CraftState,
    timer: f32,
    keep: impl Fn(&oag_gameplay::cue::CueEvent) -> bool,
) -> (Vec<(Cue, usize)>, usize) {
    let physics = &mut race.core.sim.world.ships[slot].physics;
    physics.craft_state = state;
    physics.state_timer = timer;
    race.tick(&PlayerInputs::none());
    let raised = race
        .core
        .sim
        .pending_cues()
        .iter()
        .filter(|event| {
            matches!(
                event.cue,
                Cue::ShipExplosionPlayer
                    | Cue::ShipExplosion
                    | Cue::ShipDestroyed
                    | Cue::ShipDestroyedStop
                    | Cue::ShipOnFire
                    | Cue::ShipOnFireStop
            )
        })
        .map(|event| (event.cue, usize::from(event.slot)))
        .collect();
    oag_game::sound::race_tick_keeping(audio, race, keep);
    audio.tick();
    let voices = audio.output().with_mixer(|mixer| mixer.active_voices());
    (raised, voices)
}

#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn a_destroyed_craft_blasts_burns_and_lets_its_loops_go_on_the_state_edges() {
    let Some(mut race) = load() else {
        return;
    };
    let mut audio = audio();
    let all = |_: &oag_gameplay::cue::CueEvent| true;
    for _ in 0..30 {
        race.tick(&PlayerInputs::none());
        race.core.sim.drain_cues();
    }

    let (raised, _) = step(
        &mut race,
        &mut audio,
        OPPONENT,
        CraftState::Destroyed,
        0.3,
        all,
    );
    assert_eq!(
        raised,
        [(Cue::ShipExplosion, OPPONENT), (Cue::ShipOnFire, OPPONENT)],
        "an opponent's blast and its fire, no `~SHIP_DESTROYED`"
    );
    for _ in 0..90 {
        step(
            &mut race,
            &mut audio,
            OPPONENT,
            CraftState::Eliminated,
            20.0,
            all,
        );
    }

    let (raised, _) = step(
        &mut race,
        &mut audio,
        PLAYER,
        CraftState::Destroyed,
        0.3,
        all,
    );
    assert_eq!(
        raised,
        [
            (Cue::ShipExplosionPlayer, PLAYER),
            (Cue::ShipDestroyed, PLAYER),
            (Cue::ShipOnFire, PLAYER)
        ],
        "the player's blast and both loops"
    );
    let (raised, _) = step(
        &mut race,
        &mut audio,
        PLAYER,
        CraftState::Eliminated,
        20.0,
        all,
    );
    assert_eq!(raised, [], "state 5 starts nothing");
    let (raised, _) = step(
        &mut race,
        &mut audio,
        PLAYER,
        CraftState::Eliminated,
        0.0,
        all,
    );
    assert_eq!(raised, [(Cue::ShipDestroyedStop, PLAYER)], "state 6");
    let (raised, _) = step(&mut race, &mut audio, PLAYER, CraftState::Racing, 0.0, all);
    assert_eq!(raised, [(Cue::ShipOnFireStop, PLAYER)], "back in the race");
}

/// The player's wreck, with and without the two held loops, at the same ticks: the gap is what
/// the loops add, so it is two voices or more while both are open, shrinks when state 6 stops
/// `~SHIP_DESTROYED` and closes when the fire stops. Fails if the loops are not wired through to
/// the mixer or a stop edge is dropped.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_held_loops_open_and_close_in_the_mixer() {
    let voices = |keep_loops: bool| {
        let mut race = load().expect("the HD disc");
        let mut audio = audio();
        for _ in 0..30 {
            race.tick(&PlayerInputs::none());
            race.core.sim.drain_cues();
        }
        let keep = |event: &oag_gameplay::cue::CueEvent| {
            keep_loops || !matches!(event.cue, Cue::ShipDestroyed | Cue::ShipOnFire)
        };
        let mut at = |state, timer, ticks| {
            (0..ticks)
                .map(|_| step(&mut race, &mut audio, PLAYER, state, timer, keep).1)
                .last()
                .expect("a tick")
        };
        [
            at(CraftState::Destroyed, 0.3, 1),
            at(CraftState::Eliminated, 20.0, 30),
            at(CraftState::Eliminated, 0.0, 2),
            at(CraftState::Racing, 0.0, 2),
        ]
    };
    let (with, without) = (voices(true), voices(false));
    let gap = |i: usize| with[i] as i64 - without[i] as i64;
    let note = format!("{with:?} against {without:?}");
    assert!(gap(1) >= 2, "both loops open in state 5: {note}");
    assert!(gap(2) < gap(1), "state 6 stops `~SHIP_DESTROYED`: {note}");
    assert!(gap(2) >= 1, "the fire burns on through state 6: {note}");
    assert_eq!(gap(3), 0, "the fire stops when the craft is back: {note}");
}
