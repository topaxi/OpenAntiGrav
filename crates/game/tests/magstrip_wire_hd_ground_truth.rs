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
//!     -E 'test(/^magstrip_wire_hd_ground_truth::/)'
//! ```
//!
//! What it pins that the unit tests in `oag-raceplay` cannot: that HD's own
//! collision reports a magstrip under a craft at all (the precondition of the
//! whole effect), that the wake's draw list is non-empty on contact and empty
//! once its arcs have aged out, that every craft carries an `arc_anchor_point`,
//! that `~magstrip01` loads from `shiphd.bnk`, and that dropping the cue edges
//! changes the rendered audio only inside the strip.

use oag_gameplay::PlayerInputs;
use oag_gameplay::cue::Cue;
use oag_raceplay as race;

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

#[derive(Default)]
struct Pad {
    sets: Vec<(usize, oag_input::rumble::Motors)>,
}

impl oag_input::rumble::MotorOutput for Pad {
    fn set(&mut self, seat: usize, motors: oag_input::rumble::Motors) {
        self.sets.push((seat, motors));
    }
    fn silence(&mut self) {}
}

fn count(fired: &[(&'static str, u32)], name: &str) -> u32 {
    fired
        .iter()
        .find(|(n, _)| *n == name)
        .map_or(0, |(_, c)| *c)
}

/// What the local player's pad did over `ticks` ticks of the autopiloted field:
/// the pad's writes, the rumbler's tally, and the player's own strip and pad
/// edges for comparison.
struct Run {
    pad: Pad,
    fired: Vec<(&'static str, u32)>,
    rises: u32,
    falls: u32,
    pad_cues: u32,
    bot_pad_cues: u32,
}

fn rumble_run(token: &str, paused_from: Option<u64>, give_autopilot: bool) -> Option<Run> {
    let mut race = load()?;
    let mut rumbler = oag_game::rumble::Rumbler::new(&race, token, oag_hd::TITLE.rumble);
    if token == "none" {
        assert!(rumbler.is_none(), "RUMBLE none builds no rumbler");
    }
    if give_autopilot {
        race.sim.world.ships[0].autopilot_timer = 4.0;
    }
    let mut run = Run {
        pad: Pad::default(),
        fired: Vec::new(),
        rises: 0,
        falls: 0,
        pad_cues: 0,
        bot_pad_cues: 0,
    };
    let mut was = false;
    for tick in 0..TICKS {
        race.tick(&PlayerInputs::none());
        for event in race.pending_cues() {
            if event.cue == Cue::SpeedupPad {
                if event.slot == 0 {
                    run.pad_cues += 1;
                } else {
                    run.bot_pad_cues += 1;
                }
            }
        }
        let over = race.over_magstrip(0);
        run.rises += u32::from(over && !was);
        run.falls += u32::from(!over && was);
        was = over;
        if let Some(rumbler) = &mut rumbler {
            rumbler.tick(
                &race,
                paused_from.is_some_and(|from| tick >= from),
                &mut run.pad,
            );
        }
        race.drain_cues();
    }
    if let Some(rumbler) = &rumbler {
        run.fired = rumbler.fired();
    }
    Some(run)
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_players_pad_plays_the_discs_edge_patterns_and_a_bots_events_reach_no_motor() {
    let Some(run) = rumble_run("ps3", None, false) else {
        return;
    };
    println!("fired {:?}", run.fired);
    assert!(run.rises >= 1, "the player reaches a strip");
    assert_eq!(count(&run.fired, "enter_mag_rumble"), run.rises);
    assert_eq!(count(&run.fired, "exit_mag_rumble"), run.falls);
    assert_eq!(count(&run.fired, "travel_mag_rumble"), run.rises);
    assert_eq!(
        count(&run.fired, "pad_speed"),
        run.pad_cues,
        "one pad_speed per speed pad the player crossed; seven bots crossed {} more",
        run.bot_pad_cues
    );
    assert!(run.pad_cues >= 1, "the player crosses a speed pad");
    assert!(run.pad.sets.iter().any(|(_, m)| m.large > 0), "a motor ran");
    assert!(
        run.pad.sets.iter().all(|(seat, _)| *seat == 0),
        "only the player's seat is written"
    );
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_none_setting_fires_nothing_and_a_pause_silences_the_motors() {
    let Some(none) = rumble_run("none", None, false) else {
        return;
    };
    assert!(none.fired.is_empty() && none.pad.sets.is_empty());

    let Some(paused) = rumble_run("ps3", Some(0), false) else {
        return;
    };
    assert!(
        paused
            .pad
            .sets
            .iter()
            .all(|(_, m)| m.large == 0 && m.small == 0),
        "paused from the first tick, nothing is felt"
    );
    assert!(
        count(&paused.fired, "enter_mag_rumble") >= 1,
        "voices still start while paused: the patterns run on"
    );
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn a_held_autopilot_loops_until_the_pickup_runs_out_then_finishes() {
    let Some(run) = rumble_run("ps3", None, true) else {
        return;
    };
    assert_eq!(count(&run.fired, "autopilot_active_rumble"), 1);
    let running = run
        .pad
        .sets
        .iter()
        .take(4 * 60)
        .filter(|(_, m)| m.large > 0)
        .count();
    assert!(
        running > 3 * 60,
        "held for the 4 s, not a one-shot: {running} ticks"
    );
}

fn flown_into_the_wall(stick: f32) -> Option<(Run2, race::Race)> {
    let image = oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    let mut rumbler = oag_game::rumble::Rumbler::new(&race, "ps3", oag_hd::TITLE.rumble)
        .expect("HD builds a rumbler");
    let mut buttons = oag_gameplay::input::Input::EMPTY;
    buttons.begin_frame(oag_gameplay::input::Button::Cross.bit());
    let inputs = PlayerInputs::single(oag_gameplay::input::InputSnapshot {
        buttons,
        stick_x: stick,
        ..oag_gameplay::input::InputSnapshot::EMPTY
    });
    let mut run = Run2 {
        pad: Pad::default(),
        contact_ticks: 0,
        fired: Vec::new(),
    };
    for _ in 0..3600u64 {
        race.tick(&inputs);
        run.contact_ticks += u32::from(race.wall_contact_of(0).is_some());
        rumbler.tick(&race, false, &mut run.pad);
        race.drain_cues();
    }
    run.fired = rumbler.fired();
    Some((run, race))
}

struct Run2 {
    pad: Pad,
    contact_ticks: u32,
    fired: Vec<(&'static str, u32)>,
}

#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn a_scrape_down_a_wall_plays_the_collision_bands_the_contact_size_calls_for() {
    let Some((run, _)) = flown_into_the_wall(1.0) else {
        return;
    };
    println!("fired {:?}", run.fired);
    let started = |prefix: &str| {
        run.fired
            .iter()
            .filter(|(n, _)| n.starts_with(prefix))
            .map(|(_, c)| *c)
            .sum::<u32>()
    };
    assert!(run.contact_ticks > 0, "the ship reaches a wall");
    assert!(
        started("collision_heavy") >= 1,
        "pressed hard against the wall"
    );
    assert!(
        started("collision_right") + started("collision_left") >= 1,
        "a glancing scrape decays through the lighter bands"
    );
    assert!(
        started("collision_") <= run.contact_ticks,
        "no voice starts without a contact tick"
    );
    assert!(run.pad.sets.iter().any(|(_, m)| m.large > 0));
}
