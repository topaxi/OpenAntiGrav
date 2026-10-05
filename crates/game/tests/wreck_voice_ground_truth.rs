//! A wrecked opponent's `cont_elim`, raised on its tick and rendered to a WAV.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! `docs/ghidra/functions/psp-pulse-usa/zone-rest.md`: in a Single Race an
//! opponent's destruction is voiced by the announcer line `cont_elim` and by
//! nothing else, 2.3 s after the craft is down. This wrecks an opponent at tick
//! 300, past `ready` and `go`, and renders speech alone: the third run of speech
//! must start where the cue is raised, and there must be exactly one cue.

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;
use oag_sound::Volume;
use oag_sound::sfx::Cue;

const WRECKED_AT: u64 = 300;
const RENDER_TICKS: u64 = 520;
const FRAMES_PER_TICK: usize = (oag_sound::DUMP_SAMPLE_RATE / 60) as usize;
const RUN_GAP: usize = (oag_sound::DUMP_SAMPLE_RATE / 5) as usize;
const SILENCE: i16 = 8;

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_wrecked_opponent_voices_cont_elim_in_a_single_race() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-usa.chd") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    let wav = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/shots/wreck-voice-single-race.wav");
    let mut audio = oag_sound::Audio::open(
        &oag_sound::settings::Settings {
            music_volume: Volume::OFFERED[0],
            sfx_volume: Volume::OFFERED[0],
            ..Default::default()
        },
        Some(wav.clone()),
        None,
        oag_audio::MIN_BUFFER,
        false,
    );
    let mut raised = Vec::new();
    for tick in 0..RENDER_TICKS {
        if tick == WRECKED_AT {
            race.sim.world.ships[3].physics.craft_state = oag_physics::CraftState::Eliminated;
        }
        race.tick(&PlayerInputs::none());
        let stepped = race.sim.world.tick - 1;
        for event in race.pending_cues() {
            if event.cue == Cue::ContElim {
                raised.push((stepped, event.slot));
            }
        }
        oag_game::sound::race_tick(&mut audio, &mut race);
        audio.tick();
    }
    audio.finish().expect("writing the dump");
    let file = std::fs::read(&wav).expect("the dump");
    let pcm = &file[44..];
    let mut runs: Vec<(usize, usize)> = Vec::new();
    for frame in 0..pcm.len() / 4 {
        let at = frame * 4;
        let l = i16::from_le_bytes([pcm[at], pcm[at + 1]]).unsigned_abs();
        let r = i16::from_le_bytes([pcm[at + 2], pcm[at + 3]]).unsigned_abs();
        if l.max(r) <= SILENCE as u16 {
            continue;
        }
        match runs.last_mut() {
            Some(run) if frame - run.1 <= RUN_GAP => run.1 = frame,
            _ => runs.push((frame, frame)),
        }
    }
    println!("raised {raised:?}, runs {runs:?}");
    assert_eq!(raised.len(), 1, "one wreck, one cue: {raised:?}");
    let cue_tick = raised[0].0;
    assert!((WRECKED_AT + 136..=WRECKED_AT + 142).contains(&cue_tick));
    assert_eq!(runs.len(), 3, "ready, go and cont_elim: {runs:?}");
    let onset = runs[2].0 / FRAMES_PER_TICK;
    assert!(
        (cue_tick as usize..=cue_tick as usize + 2).contains(&onset),
        "cont_elim is audible from tick {onset}, raised on {cue_tick}"
    );
}
