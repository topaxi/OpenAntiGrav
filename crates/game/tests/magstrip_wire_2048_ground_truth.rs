//! The magstrip effect in a real Wipeout 2048 race on `tower`, eight craft
//! flown by their drivers (the player's by `set_autopilot`), the over-strip
//! predicate read off the real collision, and the cue edges raised. 2048 plays
//! `WO_MAGSTRIP_*` (`.POB`) while a craft is over the strip and builds no arc
//! wake: a 2048 event's mode id is a CRC, `>= 0x17`.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(magstrip_wire_2048_ground_truth)'
//! ```
//!
//! What it pins that the unit tests in `oag-raceplay` cannot: that 2048's own
//! `track_col.col` reports a magstrip under a craft at all (the precondition of
//! the whole effect), that the `.pob` effect is up exactly on contact and
//! empty once its arcs have aged out, that every craft carries an
//! `arc_anchor_point`, that both `.gxt` decode, and that both `.POB` parse.

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;
use oag_sound::sfx::Cue;

/// Altima, the default circuit, has no surface-3 triangle at all; `tower` has
/// 1,542 (`vita_colcorpus`, 2026-10-05), the most of any base-package circuit.
const TRACK: &str = r"Data\art\published\environments\tower\track.vex";

const TICKS: u64 = 5600;

fn load() -> Option<race::Race> {
    load_mode(oag_race::Mode::SingleRace)
}

fn load_mode(mode: oag_race::Mode) -> Option<race::Race> {
    let image = oag_testdata::exact("data/extracted/vita/PCSF00007")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        mode,
        track: Some(TRACK.to_string()),
        ..race::Options::default()
    })
    .expect("loading the race");
    for line in loaded.report.iter().filter(|l| {
        l.starts_with("sfx")
            || l.contains("magstrip effect")
            || l.contains("MAGSTRIP")
            || l.contains("~magstrip01")
            || l.contains("electric_arc")
            || l.contains("ElectricArc")
    }) {
        println!("{line}");
    }
    assert!(
        loaded.magstrip_wake_textures.is_none(),
        "no arc textures are bound on a .pob title"
    );
    for name in ["WO_MAGSTRIP_SPARKS", "WO_MAGSTRIP_ZONE"] {
        assert!(
            loaded.setup.effects.get(name).is_some(),
            "{name} must load from Data/Particles2048"
        );
    }
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
    /// Ticks the `.pob` effect and the contact disagreed, and ticks any arc lived.
    pob_off_contact: usize,
    arcs_live: usize,
}

fn fly(drop_edges: bool, audio: Option<&mut oag_sound::Audio>) -> (Log, Vec<i16>) {
    let mut race = load().expect("the HD disc");
    assert!(!race.has_magstrip_wake(), "2048 builds no arc wake");
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
        for slot in 0..n {
            if race.magstrip_pob_of(slot).is_some() != race.over_magstrip(slot) {
                log.pob_off_contact += 1;
            }
            log.arcs_live += race.magstrip_wake_live(slot);
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
#[ignore = "needs the extracted Vita package in data/extracted/vita"]
fn every_craft_visits_a_strip_and_its_edges_match_its_cues() {
    if load().is_none() {
        return;
    }
    let (log, _) = fly(false, None);
    println!(
        "rising {:?}\nfalling {:?}\nstarts {:?}\nstops {:?}",
        log.rising, log.falling, log.starts, log.stops
    );
    let visited = (0..8)
        .filter(|&slot| log.rising.iter().any(|e| e.1 == slot))
        .count();
    assert!(
        log.rising.iter().any(|e| e.1 == 0) && visited >= 6,
        "the player and most of the grid reach a strip on tower: {visited} of 8"
    );
    for slot in 0..8 {
        let count = |v: &[(u64, usize)]| v.iter().filter(|e| e.1 == slot).count();
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
    assert_eq!(
        log.pob_off_contact, 0,
        "WO_MAGSTRIP_* plays exactly while a craft is over the strip"
    );
    assert_eq!(log.arcs_live, 0, "no arc on a .pob title");
}

/// **A 2048 event plays the `.pob` and builds no arc wake, in every mode.**
///
/// `FUN_81000930` is `mode id < 0x17`, and a campaign event's id is a CRC of
/// its own name: `0x026886dc` read live on Vita3K for Queens Mall's Time Trial
/// (`ships-effects.md`), so the `.POB` side (`WO_MAGSTRIP_SPARKS`/`_ZONE`) is
/// the one 2048 reaches and the arc wake is the HD-lineage named-mode side.
/// Putting the title's `magstrip_wake` entry back fails this.
#[test]
#[ignore = "needs the extracted Vita package in data/extracted/vita"]
fn every_2048_mode_plays_the_pob_and_builds_no_arc_wake() {
    for mode in [
        oag_race::Mode::SingleRace,
        oag_race::Mode::TimeTrial,
        oag_race::Mode::SpeedLap,
        oag_race::Mode::Eliminator,
        oag_race::Mode::Zone,
    ] {
        let Some(race) = load_mode(mode) else { return };
        assert!(!race.has_magstrip_wake(), "{mode:?}: no arc wake");
        assert!(
            race.magstrip_wake_vertices().0.is_empty(),
            "{mode:?}: no arc geometry"
        );
    }
}

/// The hum stays silent on 2048: `~magstrip01` is a cue in `shipHD.bnk`, the
/// ship bank this title still points at (HD's spelling, chosen), and the
/// general cue lookup does not read a Vita bank's names by hash. Reading every
/// 2048 cue by hash was tried on 2026-10-05 and played cues whose triggers were
/// never measured (a perfect-lap announcement mid-lap, and noise), so it was
/// scoped back to the crossfade engine (`Bank::cue_named_or_hashed`). When the
/// 2048 cue set and its banks are measured, this flips to the HD test's audio
/// A/B.
#[test]
#[ignore = "needs the extracted Vita package in data/extracted/vita"]
fn the_hum_is_silent_until_the_vita_cue_set_is_measured() {
    let Some(race) = load() else { return };
    let mut rng = oag_core::Rng::new(1);
    assert!(
        race.sounds().voices(Cue::Magstrip, &mut rng).is_none(),
        "~magstrip01 resolves on the Vita ship bank"
    );
}

/// A real 2048 race load hands its grid the crossfaded engine of the table the
/// title's `Crossfade` axis names: four layers in an ordinary race, Zone's own
/// three-layer `xfship_ZONE_` table in Zone. This goes through
/// `oag_raceplay::load`, so it fails if that load reads the plain `ship` bank
/// instead of the axis.
#[test]
#[ignore = "needs the extracted Vita package in data/extracted/vita"]
fn a_race_load_gives_the_player_its_teams_crossfade_table_in_both_modes() {
    for (mode, layers) in [(oag_race::Mode::SingleRace, 4), (oag_race::Mode::Zone, 3)] {
        let Some(race) = load_mode(mode) else { return };
        let team = race
            .sounds()
            .xfade_team(r"Feisar2048\3")
            .unwrap_or_else(|| panic!("{mode:?}: no crossfade table for the player's team"));
        assert_eq!(team.table().layer_count(), layers, "{mode:?}");
        assert_eq!(team.playable(), 3, "{mode:?}: looping layers that resolved");
    }
}

/// `WO_MAGSTRIP_ZONE` and `WO_MAGSTRIP_SPARKS` ship in the base package's
/// `Data/Particles2048` and parse. They are what a 2048 craft plays over a strip: no named mode reaches
/// them (see [`every_2048_mode_plays_the_pob_and_builds_no_arc_wake`]).
#[test]
#[ignore = "needs the extracted Vita package in data/extracted/vita"]
fn the_two_magstrip_pobs_ship_and_parse() {
    let Some(root) = oag_testdata::exact("data/extracted/vita/PCSF00007") else {
        return;
    };
    let mut archive =
        oag_assets::psarc::Archive::open(&root.join("base/PSP2/data.psarc").display().to_string())
            .expect("the base archive");
    for name in ["WO_MAGSTRIP_ZONE", "WO_MAGSTRIP_SPARKS"] {
        let blob = archive
            .read_path(&format!("data/particles2048/{name}.pob"))
            .unwrap_or_else(|why| panic!("{name}: {why}"));
        let system = oag_pob::ParticleSystem::parse(&blob)
            .unwrap_or_else(|why| panic!("{name} does not parse: {why}"));
        println!("{name}: {} bytes, parses as {}", blob.len(), system.name);
    }
}
