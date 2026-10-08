//! The HD-lineage magstrip arc wake on a real Wipeout HD disc: Talon's Junction,
//! eight craft flown by their drivers (the player's by `set_autopilot`), the
//! over-strip predicate read off the real collision, and the `~magstrip01` cue
//! raised and played.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(magstrip_wire_hd_ground_truth)'
//! ```
//!
//! What it pins that the unit tests in `oag-raceplay` cannot: that HD's own
//! collision reports a magstrip under a craft at all (the precondition of the
//! whole effect), that the wake's draw list is non-empty on contact and empty
//! once its arcs have aged out, that every craft carries an `arc_anchor_point`,
//! that `~magstrip01` loads from `shiphd.bnk`, and that dropping the cue edges
//! changes the rendered audio only inside the strip.

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;
use oag_sound::sfx::Cue;

const TICKS: u64 = 2400;
const FRAMES_PER_TICK: usize = (oag_sound::DUMP_SAMPLE_RATE / 60) as usize;

fn load() -> Option<race::Race> {
    let image = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    })
    .expect("loading the race");
    for line in loaded.report.iter().filter(|l| {
        l.contains("magstrip arc wake")
            || l.contains("~magstrip01")
            || l.contains("electric_arc")
            || l.contains("ElectricArc")
    }) {
        println!("{line}");
    }
    assert!(
        loaded.ribbon_textures.magstrip.is_some(),
        "HD_electric_arc_8x8 and HD_ElectricArc_Contact must both decode"
    );
    let mut race = race::Race::start(loaded.setup);
    race.set_autopilot(true);
    Some(race)
}

#[derive(Default)]
struct Log {
    /// `(tick, slot)` per rising and falling edge of `over_magstrip`.
    rising: Vec<(u64, usize)>,
    falling: Vec<(u64, usize)>,
    starts: Vec<(u64, usize)>,
    stops: Vec<(u64, usize)>,
    /// Ticks a slot was over the strip, and ticks it had arcs / vertices.
    over: Vec<(u64, usize)>,
    drawn_without_arcs: usize,
    arcs_without_vertices: usize,
}

fn fly(drop_edges: bool, audio: Option<&mut oag_sound::Audio>) -> (Log, Vec<i16>) {
    let mut race = load().expect("the HD disc");
    assert!(race.has_magstrip_wake(), "HD builds the wake");
    let mut audio = audio;
    let mut log = Log::default();
    let mut was = [false; 8];
    let n = usize::from(race.ship_count());
    for tick in 0..TICKS {
        race.tick(&PlayerInputs::none());
        let raised = race.pending_cues().to_vec();
        if audio.is_none() {
            race.drain_cues();
        }
        for event in &raised {
            match event.cue {
                Cue::Magstrip => log.starts.push((tick, usize::from(event.slot))),
                Cue::MagstripStop => log.stops.push((tick, usize::from(event.slot))),
                _ => {}
            }
        }
        #[allow(clippy::needless_range_loop)]
        for slot in 0..n {
            let over = race.over_magstrip(slot);
            if over {
                log.over.push((tick, slot));
            }
            match (was[slot], over) {
                (false, true) => log.rising.push((tick, slot)),
                (true, false) => log.falling.push((tick, slot)),
                _ => {}
            }
            was[slot] = over;
        }
        let (atlas, contact) = race.magstrip_wake_vertices();
        let live: usize = (0..n).map(|s| race.magstrip_wake_live(s)).sum();
        if live == 0 && (!atlas.is_empty() || !contact.is_empty()) {
            log.drawn_without_arcs += 1;
        }
        if live > 0 && atlas.is_empty() {
            log.arcs_without_vertices += 1;
        }
        if let Some(audio) = audio.as_deref_mut() {
            oag_game::sound::race_tick_keeping(audio, &mut race, |e| {
                !(drop_edges && matches!(e.cue, Cue::Magstrip | Cue::MagstripStop))
            });
            audio.tick();
        }
    }
    (log, Vec::new())
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn every_craft_visits_a_strip_and_its_edges_match_its_cues() {
    if load().is_none() {
        return;
    }
    let (log, _) = fly(false, None);
    println!(
        "rising {:?}\nfalling {:?}\nstarts {:?}\nstops {:?}",
        log.rising, log.falling, log.starts, log.stops
    );
    for slot in 0..8 {
        let count = |v: &[(u64, usize)]| v.iter().filter(|e| e.1 == slot).count();
        assert!(
            count(&log.rising) >= 1,
            "slot {slot} never reached a magstrip"
        );
        assert_eq!(
            count(&log.starts),
            count(&log.rising),
            "slot {slot}: one ~magstrip01 start per visit"
        );
        assert_eq!(
            count(&log.stops),
            count(&log.falling),
            "slot {slot}: one stop per departure"
        );
    }
    assert_eq!(log.drawn_without_arcs, 0, "geometry with no live arc");
    assert_eq!(log.arcs_without_vertices, 0, "live arcs that drew nothing");
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_hum_is_in_shiphd_bnk_and_changes_the_audio_only_on_the_strip() {
    let Some(race) = load() else { return };
    let mut rng = oag_core::Rng::new(1);
    let voices = race
        .sounds()
        .voices(Cue::Magstrip, &mut rng)
        .expect("~magstrip01 loads from shiphd.bnk");
    println!(
        "~magstrip01: {} voice(s), looping {:?}",
        voices.len(),
        voices.iter().map(|v| v.looping).collect::<Vec<_>>()
    );
    let mut seen = [0usize; 2];
    for _ in 0..200 {
        for v in race
            .sounds()
            .voices(Cue::Magstrip, &mut rng)
            .expect("loads")
        {
            seen[usize::from(v.looping)] += 1;
        }
    }
    println!(
        "~magstrip01 over 200 draws: {} one-shot, {} looping voices",
        seen[0], seen[1]
    );
    drop(race);

    let render = |drop_edges: bool| {
        let wav = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "../../data/scratch/magstrip-wire-hd/hum-{}.wav",
            if drop_edges { "off" } else { "on" }
        ));
        std::fs::create_dir_all(wav.parent().unwrap()).unwrap();
        let mut audio = oag_sound::Audio::open(
            &oag_sound::settings::Settings::default(),
            Some(wav.clone()),
            None,
            oag_audio::MIN_BUFFER,
            false,
        );
        let (log, _) = fly(drop_edges, Some(&mut audio));
        audio.finish().expect("writing the dump");
        let file = std::fs::read(&wav).expect("the dump");
        let pcm: Vec<i16> = file[44..]
            .as_chunks::<2>()
            .0
            .iter()
            .map(|b| i16::from_le_bytes(*b))
            .collect();
        (log, pcm)
    };
    let (log, on) = render(false);
    let (_, off) = render(true);
    let first_start = log.starts.iter().map(|e| e.0).min().expect("a start") as usize;
    let at = first_start * FRAMES_PER_TICK * 2;
    assert_eq!(on.len(), off.len());
    assert_eq!(
        on[..at],
        off[..at],
        "audio differs before any craft is on a strip"
    );
    let after = &on[at..];
    assert!(
        after.iter().zip(&off[at..]).any(|(a, b)| a != b),
        "dropping the hum changed nothing once a craft was on a strip"
    );
}
