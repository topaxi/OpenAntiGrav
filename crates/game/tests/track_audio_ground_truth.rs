//! What a circuit's own authored sound emitters do over a real lap.
//!
//! **`#[ignore]`d and never run in CI.** They need game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --test track_audio_ground_truth \
//!   --run-ignored all --no-capture
//! ```
//!
//! # What this is for
//!
//! `crates/formats/tests/sound_emitter_ground_truth.rs` proves the *decode*
//! against all 33 circuit files. This is the layer above, and it answers a
//! question nothing had answered: **how many of a circuit's emitters are inside
//! their own radius at the same time**, on a real lap, against the recovered
//! radius law. `oag_audio::mixer::MAX_VOICES` is 32 and `01_Track` authors 86
//! emitters, so "the budget question is real" was a prediction until this ran.

use std::path::{Path, PathBuf};

use oag_game::sound::listener_of;
use oag_gameplay::PlayerInputs;
use oag_raceplay as race;
use oag_sound::sfx::TrackEmitters;

/// `01_Track`, the circuit the thread names: 86 `sound` nodes and no cone.
const TRACK: &str = r"Data\Environments\01_Track\track.vex";

/// `14_Track`, the densest circuit on the disc: 97 `sound` nodes and 9 cones.
///
/// Measured as well as `01_Track` because a budget read off the *second* most
/// crowded circuit is not a budget. See `track-sound-emitters.md`'s census.
const DENSEST: &str = r"Data\Environments\14_Track\track.vex";

/// How long a measured lap is allowed to take before the run is called off.
///
/// Three minutes of simulated time. A Venom lap of Vineta K is well under one,
/// and a run that reaches this has gone wrong in a way a longer cap would hide.
const TICK_CAP: usize = 60 * 180;

fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::image(name)
}

/// One circuit, loaded, with the player flown by the racing line.
fn lap(image: &Path, track: &str) -> Option<(race::Race, TrackEmitters)> {
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        track: Some(track.to_string()),
        // The whole grid, because the listener is the camera and the camera is
        // the player's - a solo run and a full grid put the ears in the same
        // places, but a full grid is what a player actually races.
        opponents: true,
        ..race::Options::default()
    })
    .expect("loading the race");
    // Parsed by `race::load` itself, off the same `.vex` it builds the circuit
    // from - so this measures what a real race carries, not a second decode.
    let emitters = loaded.setup.track_emitters.clone();
    let mut race = race::Race::start(loaded.setup);
    // The player is flown by their own racing line, so the ears go all the way
    // round rather than into the first wall. See `Race::set_autopilot`.
    race.set_autopilot(true);
    Some((race, emitters))
}

/// Flies one lap and reports how many emitters were in range on each tick.
///
/// Returns the peak, because that is the number a voice budget is set from.
/// Everything else is printed: a bound that only ever appears inside an
/// `assert!` is a number nobody reads, and this one is the input to a design
/// decision - `oag_audio::mixer::MAX_VOICES` is 32.
fn measure(name: &str, race: &mut race::Race, emitters: &TrackEmitters) -> usize {
    let total_emitters = emitters.omni.len() + emitters.directional.len();
    let mut histogram = vec![0usize; total_emitters + 1];
    let mut ever = vec![false; total_emitters];
    let mut ticks = 0usize;
    while ticks < TICK_CAP && race.sim.world.primary_race().laps_completed() < 1 {
        race.tick(&PlayerInputs::none());
        let listener = listener_of(race);
        let mut live = 0;
        for (at, _) in emitters.placed(&listener) {
            ever[at] = true;
            live += 1;
        }
        histogram[live] += 1;
        ticks += 1;
    }
    assert!(
        ticks < TICK_CAP,
        "{name}: three minutes of autopilot and the lap never closed"
    );

    let peak = histogram
        .iter()
        .rposition(|&n| n > 0)
        .expect("at least one tick");
    let floor = histogram
        .iter()
        .position(|&n| n > 0)
        .expect("the same tick");
    let total: usize = histogram.iter().enumerate().map(|(n, c)| n * c).sum();
    #[expect(clippy::cast_precision_loss, reason = "counts, not money")]
    let mean = total as f32 / ticks as f32;
    let heard = ever.iter().filter(|&&e| e).count();

    println!(
        "{name}, one lap of {ticks} ticks: {floor}..={peak} emitters in range per tick, \
         mean {mean:.1}; {heard} of {total_emitters} were in range at some point"
    );
    for (count, ticks_at) in histogram.iter().enumerate() {
        if *ticks_at > 0 {
            println!("  {count:>3} in range: {ticks_at} tick(s)");
        }
    }
    peak
}

/// How many emitters are in range on each tick of one lap of `01_Track`.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_lap_of_vineta_k_says_how_many_emitters_want_a_voice_at_once() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let Some((mut race, emitters)) = lap(&path, TRACK) else {
        return;
    };
    for line in &emitters.report {
        println!("{line}");
    }
    assert_eq!(
        emitters.omni.len(),
        86,
        "01_Track authors 86 omnidirectional emitters"
    );
    assert_eq!(emitters.directional.len(), 0, "01_Track authors no cone");

    let peak = measure("01_Track", &mut race, &emitters);

    // The claim, and the reason the number was worth measuring: a circuit's
    // ambience fits in the voice pool with room for the race's own cues. If
    // this ever fails, the design question the thread raised is live again and
    // `Mixer::starved` is where it will show.
    assert!(
        peak < oag_audio::mixer::MAX_VOICES,
        "{peak} emitters want a voice at once and the pool holds {}",
        oag_audio::mixer::MAX_VOICES
    );
}

/// The same measurement on the circuit that authors the most.
///
/// `01_Track` alone would leave the budget resting on the circuit the thread
/// happened to name. `14_Track` is the disc's densest, and its nine cones are
/// the largest single-circuit set of them, so the budget question is at its
/// sharpest here now that both classes hold a voice.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_densest_circuit_on_the_disc_fits_in_the_pool_too() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let Some((mut race, emitters)) = lap(&path, DENSEST) else {
        return;
    };
    for line in &emitters.report {
        println!("{line}");
    }
    assert_eq!(emitters.omni.len(), 97, "14_Track authors 97 of them");
    assert_eq!(
        emitters.directional.len(),
        9,
        "and nine cones this port now plays too"
    );

    let peak = measure("14_Track", &mut race, &emitters);
    assert!(
        peak < oag_audio::mixer::MAX_VOICES,
        "{peak} emitters want a voice at once and the pool holds {}",
        oag_audio::mixer::MAX_VOICES
    );
}

/// The loader names both banks it opened and every reference it cannot resolve.
///
/// The dangling half is the point. Five references on the Pulse disc name a cue
/// or a bank that does not exist - the disc's own bugs, decoded in
/// `track-sound-emitters.md` - and the rule this project holds to is that an
/// asset which will not resolve plays nothing **and says so**. A silent drop
/// would make a later decode that fixed one of them invisible, which is exactly
/// what is still open: `Scream_FindSoundInBank`'s name comparison is unread, so
/// whether the original falls back to another bank is not settled.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn the_loader_names_its_banks_and_every_dangling_reference() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    for (track, dangling) in [
        (TRACK, &["basilic~groupcraft: 1 node(s) play nothing"][..]),
        (
            DENSEST,
            &[
                "fortcle~blueflashlight: 7 node(s) play nothing",
                "fortcle~RED_NEON_TUN: 1 node(s) play nothing",
            ][..],
        ),
    ] {
        let opened =
            oag_source::title::open_source(&path.display().to_string(), Vec::new(), Vec::new())
                .expect("opening the source");
        let mut archives = opened.archives;
        let blob = archives.read_name(track).expect("the circuit");
        let loaded = TrackEmitters::load(&mut archives, opened.title.race.sounds, track, &blob);
        for line in &loaded.report {
            println!("{line}");
        }
        let says = |what: &str| loaded.report.iter().any(|line| line.contains(what));

        // The shared bank is named by the executable; the circuit's own is
        // named by its own `trackstartup.xml` and sits beside it, which is the
        // half nothing had read before.
        assert!(
            says("generaltrack.bnk is bank \"gentrak\""),
            "{track}: the shared bank did not open"
        );
        assert!(
            says("_ENV.bnk is bank"),
            "{track}: the circuit's own bank did not open"
        );
        for reference in dangling {
            assert!(says(reference), "{track}: {reference} was not reported");
        }
        let playing = loaded.omni.iter().filter(|n| n.sound.is_some()).count();
        assert!(playing > 0, "{track}: nothing resolved at all");
        println!("{track}: {playing} of {} play", loaded.omni.len());
    }
}

/// A headless lap opens the circuit's voices, moves them and writes a WAV.
///
/// **This is the strongest evidence available here.** There is no windowed
/// session and nobody to listen, so `--dump-audio`'s own writer is what turns
/// "it should play" into a file: a real lap of a real circuit, rendered through
/// the real mixer with the null backend, out to 16-bit PCM. See
/// `oag_audio::wav`.
///
/// The dump is the whole race mix - the ambience is on `Bus::Sfx` beside the
/// engine, so it cannot be separated at the bus - which is why the voice count
/// is asserted as well as the samples: a WAV that was only the player's engine
/// would still be loud.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_headless_lap_sounds_the_circuit_and_writes_it_out() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let Some((mut race, emitters)) = lap(&path, TRACK) else {
        return;
    };
    let wav = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/shots")
        .join("track-ambience-01_track.wav");
    // `Some(dump)` forces the null backend, which is what makes this runnable
    // on a machine with no sound card - see `Audio::open`.
    let mut audio = oag_sound::Audio::open(
        &oag_sound::settings::Settings::default(),
        Some(wav.clone()),
        None,
        oag_audio::MIN_BUFFER,
        false,
    );

    // Half a minute of racing: long enough for the autopilot to leave the grid
    // and pass several of the circuit's emitters, short enough that the WAV is
    // a few megabytes rather than tens.
    const TICKS: usize = 30 * 60;
    let mut peak_voices = 0;
    let mut ever_moved = false;
    let mut last = None;
    for _ in 0..TICKS {
        race.tick(&PlayerInputs::none());
        oag_game::sound::race_tick(&mut audio, &mut race);
        audio.tick();
        let live = audio.ambient_voices();
        peak_voices = peak_voices.max(live);
        // A held voice whose gain follows the listener is the thing under test;
        // a count that never changes would also be produced by opening every
        // voice once and never touching it again.
        if last.is_some_and(|was| was != live) {
            ever_moved = true;
        }
        last = Some(live);
    }
    // Printed, not asserted, and worth watching. The mix already clipped
    // before this landed - 7,932 samples over the same 30 seconds with the
    // ambience switched off - and the circuit's own voices take it to 22,473,
    // about 0.8% of a 2.88M-sample render. Every emitter plays at the volume
    // `VexSound_Init` passes (`1.0`), so the headroom question is the sum's,
    // not this module's, and inventing a gain here to hide it would be exactly
    // the kind of plausible stand-in `CLAUDE.md` forbids. `starved` stays at
    // zero: the pool is never the constraint.
    let (clipped, starved) = audio
        .output()
        .with_mixer(|mixer| (mixer.clipped(), mixer.starved()));
    println!("clipped {clipped} sample(s), starved {starved} voice(s)");
    assert_eq!(
        starved, 0,
        "the voice pool ran out on one circuit's ambience"
    );
    audio.finish().expect("writing the dump");

    assert!(
        peak_voices > 0,
        "half a minute of Vineta K and not one of its {} emitters ever opened a voice",
        emitters.omni.len()
    );
    assert!(
        ever_moved,
        "the same {peak_voices} voice(s) for the whole run: the in-range latch never moved"
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
        "wrote {} - {} frames, peak sample {peak}, up to {peak_voices} of the circuit's own \
         voices open at once",
        wav.display(),
        pcm.len() / 4
    );
}

/// Every race circuit names a bank in its own manifest, beside its own `.vex`.
///
/// The sweep behind `docs/formats/psp-audio.md`'s per-circuit bank table, and
/// the reason that table is a reading rather than an extrapolation from the one
/// circuit this thread was written against. Three things have to hold together
/// on all twelve, and no two of them come from the same place:
///
/// 1. the circuit's `trackstartup.xml` names a bank,
/// 2. that name resolves **in the circuit's own directory** - `Data\Sound\` is
///    where every executable-named bank lives and it is not where these are,
/// 3. and the bank's own self-name is a label the circuit's emitters spell,
///    which is what the resolved count below stands in for.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn every_race_circuit_names_a_bank_beside_itself_and_its_nodes_spell_its_label() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let opened =
        oag_source::title::open_source(&path.display().to_string(), Vec::new(), Vec::new())
            .expect("opening the source");
    let mut archives = opened.archives;

    let mut found = 0;
    for number in 1..=16 {
        let track = format!(r"Data\Environments\{number:02}_Track\track.vex");
        let Ok(blob) = archives.read_name(&track) else {
            continue;
        };
        let loaded = TrackEmitters::load(&mut archives, opened.title.race.sounds, &track, &blob);
        assert!(
            !loaded.omni.is_empty(),
            "{track} is a race circuit and authors no emitter"
        );
        found += 1;

        // The bank line names the entry and the self-name together, so one
        // assertion covers both halves of the claim.
        let bank = loaded
            .report
            .iter()
            .find(|line| line.contains("circuit "))
            .unwrap_or_else(|| panic!("{track}: no circuit bank opened\n{:#?}", loaded.report));
        assert!(
            bank.contains(&format!("{number:02}_Track")),
            "{track}: its bank did not come from its own directory: {bank}"
        );

        // And the label really is what its nodes ask for. Taken off the bank's
        // own self-name rather than a table written here, so what is checked is
        // two sources agreeing rather than this file agreeing with itself.
        let label = bank
            .split('"')
            .nth(1)
            .unwrap_or_else(|| panic!("{track}: no self-name in {bank}"))
            .to_string();
        assert!(
            loaded
                .omni
                .iter()
                .any(|node| node.emitter.bank == label && node.sound.is_some()),
            "{track}: nothing it authors resolved in {label:?}, the bank its own \
             manifest names"
        );

        let playing = loaded
            .omni
            .iter()
            .filter(|node| node.sound.is_some())
            .count();
        println!("{bank}; {playing} of {} play", loaded.omni.len());
        for line in loaded.report.iter().filter(|l| l.contains("play nothing")) {
            println!("  {line}");
        }
    }
    assert_eq!(found, 12, "twelve race circuits, and this found {found}");
}
