//! Scratch board: how many ticks our AI field spends in craft-to-craft contact,
//! the number `docs/gameplay/ai.md` ("The field's contact time against the
//! original's") compares with `scripts/psp-ai-contact-field.py`'s capture.
//!
//! `#[ignore]`d and gated on `OAG_SWEEP`, printing rather than asserting.
//! Single Race, weapons on, the player parked (no input), seven opponents.
//! `OAG_AI=ours` flies this project's driver instead of the original's;
//! `OAG_LEVEL` (default `0`, the original's level for Novice), `OAG_DIFFICULTY`
//! (`novice`, default, or `ace`), `OAG_SEEDS` (default 4), `OAG_CLASSES`
//! (default `venom,rapier`), `OAG_TICKS` (default 5400 after GO).

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;
use oag_raceplay::catalogue;

fn image() -> Option<std::path::PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

fn entry_of(folder: &str, reversed: bool) -> Option<String> {
    let image = image()?;
    let mut archives = oag_pulse::open(&image.display().to_string()).ok()?;
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .ok()?;
    let definition = oag_tables::fexml::expand(&blob).ok()?;
    catalogue::tracks(&definition)
        .into_iter()
        .find(|track| track.location.ends_with(folder) && track.reversed == reversed)
        .map(|track| track.entry_name())
}

fn env_or<T: std::str::FromStr>(name: &str, default: T) -> T {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
}

/// Per opponent: contact ticks in the first 1,800 ticks after GO and over the
/// whole window.
fn run(entry: &str, class: &str, seed: u64) -> Option<(Vec<u32>, Vec<u32>)> {
    let image = image()?;
    let difficulty = match std::env::var("OAG_DIFFICULTY").as_deref() {
        Ok("ace") => oag_ai::Difficulty::Ace,
        _ => oag_ai::Difficulty::Novice,
    };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: class.to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty,
        track: Some(entry.to_string()),
        seed: Some(seed),
        ..race::Options::default()
    })
    .expect("loading");
    let mut race = race::Race::start(loaded.setup);
    if std::env::var("OAG_AI").as_deref() != Ok("ours") {
        assert!(race.use_original_ai(None, env_or("OAG_LEVEL", 0)));
    }
    let count = race.ship_count() as usize;
    let player = race.sim.world.human_slots().next().unwrap_or(0);
    let opponents: Vec<usize> = (0..count).filter(|s| *s != player).collect();
    let ticks: u64 = env_or("OAG_TICKS", 5400);
    let mut go = None;
    let mut at_30 = vec![0; count];
    for tick in 0..ticks + 2000 {
        race.tick(&PlayerInputs::none());
        if go.is_none()
            && opponents.iter().any(|s| {
                race.sim.world.ships[*s]
                    .physics
                    .body
                    .linear_velocity
                    .length()
                    > 5.0
            })
        {
            go = Some(tick);
            for s in 0..count {
                at_30[s] = race.pair_contact_ticks_of(s);
            }
        }
        if let Some(g) = go {
            if tick == g + 1800 {
                for (s, start) in at_30.iter_mut().enumerate() {
                    *start = race.pair_contact_ticks_of(s) - *start;
                }
            }
            if tick == g + ticks {
                break;
            }
        }
    }
    let first: Vec<u32> = opponents.iter().map(|s| at_30[*s]).collect();
    let whole: Vec<u32> = opponents
        .iter()
        .map(|s| race.pair_contact_ticks_of(*s))
        .collect();
    Some((first, whole))
}

#[test]
#[ignore = "scratch board: OAG_SWEEP=1 prints contact ticks per opponent"]
fn ai_contact_duration_board() {
    if std::env::var_os("OAG_SWEEP").is_none() || image().is_none() {
        return;
    }
    let seeds: u64 = env_or("OAG_SEEDS", 4);
    let classes = std::env::var("OAG_CLASSES").unwrap_or_else(|_| "venom,rapier".into());
    for (name, folder, reversed) in [
        ("talons-white", "16_Track", false),
        ("dekonstruct-white", "05_Track", true),
        ("dekonstruct-black", "05_Track", false),
    ] {
        let Some(entry) = entry_of(folder, reversed) else {
            continue;
        };
        for class in classes.split(',') {
            let (mut sum30, mut sum, mut n) = (0u32, 0u32, 0u32);
            for seed in 1..=seeds {
                let Some((first, whole)) = run(&entry, class, seed) else {
                    continue;
                };
                println!("ROW {name} {class} seed {seed} first30 {first:?} whole {whole:?}");
                sum30 += first.iter().sum::<u32>();
                sum += whole.iter().sum::<u32>();
                n += first.len() as u32;
            }
            println!(
                "MEAN {name} {class}: per opponent first30 {:.1} whole {:.1} over {n} craft",
                sum30 as f32 / n.max(1) as f32,
                sum as f32 / n.max(1) as f32
            );
        }
    }
}
