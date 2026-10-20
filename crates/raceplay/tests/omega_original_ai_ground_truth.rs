//! The Omega Collection's campaign opponent (`oag_ai::original` with
//! `oag_omega::race::ORIGINAL_DRIVER`) flown on the extracted package pair.
//!
//! **Static only**: no PS4 emulator exists in this toolchain, so nothing here
//! is a figure the original was seen to produce. What is asserted is what
//! the executable and the disc say together
//! (`docs/ghidra/functions/ps4-omega-eu/ai-controls.md`): the tables the
//! HD-lineage driver reads, the two halved-lookahead circuits resolved
//! through Omega's own track definition, a campaign cell's skill off its
//! circuit's `stats.xml`, and the field finishing a race on four circuits the
//! campaign launches.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/^omega_original_ai_ground_truth::/)'
//! ```

use oag_gameplay::PlayerInputs;
use oag_tables::handling::SpeedClass;

fn source() -> Option<String> {
    Some(
        oag_testdata::exact("data/extracted/ps4")?
            .display()
            .to_string(),
    )
}

fn loaded(track: &str, class: &str) -> Option<oag_raceplay::Loaded> {
    let source = source()?;
    Some(
        oag_raceplay::load(&oag_raceplay::Options {
            source,
            class: class.to_string(),
            mode: oag_race::Mode::SingleRace,
            track: Some(track.to_string()),
            seed: Some(1),
            ..oag_raceplay::Options::default()
        })
        .unwrap_or_else(|e| panic!("loading {track}: {e:#}")),
    )
}

/// The base set an HD-lineage mode reads (`FUN_012c99b0` with its flag
/// clear) ships and reads whole for the four classes; the `*2048.xml` set a
/// 2048-lineage mode reads holds the same values (the control file adds a
/// Super Phantom row and nothing else).
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn the_hd_lineage_tables_ship_and_the_2048_set_holds_the_same_values() {
    let Some(source) = source() else {
        return;
    };
    let mut archives = oag_omega::open(&source).expect("open omega");
    let mut read = |name: &str| archives.read_name(name).expect(name);
    let squash = |blob: Vec<u8>| {
        oag_tables::fexml::text(&blob)
            .expect("expand")
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
    };
    let control = read(oag_tables::ai_control_stats::entry_name());
    let control_2048 = read(r"Data\XML\AIControlStats2048.xml");
    for class in SpeedClass::ALL {
        let base = read(&oag_tables::ai_race_stats::entry_name(class));
        assert!(oag_tables::ai_race_stats::full_from_blob(&base, class).is_ok());
        let own = read(&format!(
            r"Data\XML\AIRaceStats_{}2048.xml",
            class.as_str().to_ascii_lowercase()
        ));
        assert_eq!(squash(base), squash(own), "{class:?}");
        assert_eq!(
            oag_tables::ai_control_stats::from_blob(&control, class).expect("base row"),
            oag_tables::ai_control_stats::from_blob(&control_2048, class).expect("2048 row"),
            "{class:?}"
        );
    }
    let superphantom = squash(read(r"Data\XML\AIRaceStats_superphantom.xml"));
    assert_eq!(
        superphantom,
        squash(read(r"Data\XML\AIRaceStats_superphantom2048.xml"))
    );
}

/// The race's laws as the loader resolved them.
fn laws_line(track: &str, class: &str) -> Option<String> {
    let loaded = loaded(track, class)?;
    Some(
        loaded
            .report
            .iter()
            .find(|line| line.starts_with("original AI: this title's driver variant"))
            .unwrap_or_else(|| panic!("no laws line: {:#?}", loaded.report))
            .clone(),
    )
}

/// `05_Track` (Ubermall forward) halves its lookahead for a Phantom only,
/// `20_Track` (Amphiseum reversed) for every class: the ids are found in
/// Omega's `Data\Plugins\tracks\Definition.xml`, not its team definition.
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn the_halved_lookahead_resolves_through_omegas_own_track_definition() {
    let Some(phantom) = laws_line(r"Data\Environments\05_Ubermall\track.vex", "Phantom") else {
        return;
    };
    assert!(
        phantom.contains("halved_lookahead: Some((250.0, 4600.0))"),
        "{phantom}"
    );
    assert!(phantom.contains("coin_ordered_search: true"), "{phantom}");
    assert!(phantom.contains("every_tick: false"), "{phantom}");
    let venom = laws_line(r"Data\Environments\05_Ubermall\track.vex", "Venom").expect("loaded");
    assert!(venom.contains("halved_lookahead: None"), "{venom}");
    let reversed =
        laws_line(r"Data\Environments\amphiseum\track_reversed.vex", "Flash").expect("loaded");
    assert!(
        reversed.contains("halved_lookahead: Some((400.0, 5400.0))"),
        "{reversed}"
    );
}

/// The first cell of the first grid (Vineta K, Venom) resolves its skill off
/// `Data\Environments\01_Vineta_K\stats.xml`, read through the same
/// `Archives::open` the campaign's launch falls back to, as HD's cell does
/// (`AI_ResolveSkillScale`'s cell branch, inlined at the end of Omega's
/// `0x012c9650`).
#[test]
#[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
fn a_campaign_cell_resolves_its_skill_off_its_circuits_stats() {
    use oag_tables::race_campaign::{self, Difficulty};
    let Some(source) = source() else {
        return;
    };
    let mut archives = oag_assets::Archives::open(&source, oag_omega::TITLE).expect("open omega");
    let definition = archives
        .read_name(oag_omega::campaign::DEFINITION_ENTRY)
        .expect("grids definition");
    let definition = oag_tables::fexml::text(&definition).expect("expand");
    let first = race_campaign::definition_entries(&definition)
        .into_iter()
        .next()
        .expect("a grid");
    let grid = race_campaign::from_blob(&archives.read_name(&first).expect("grid")).expect("parse");
    let cell = grid.cells.first().expect("a cell");
    let stats = archives
        .read_name(r"Data\Environments\01_Vineta_K\stats.xml")
        .expect("stats.xml");
    let stats = oag_tables::track_stats::from_blob(&stats).expect("parse stats");
    let skills: Vec<f32> = [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard]
        .into_iter()
        .map(|rung| {
            oag_tables::track_stats::resolve_skill_scale(cell, rung, Some(&stats))
                .expect("the cell authors a skill")
        })
        .collect();
    println!("{} skills {skills:?}", cell.name);
    // `grid0_3_1`: Easy at the curve's bottom, Medium and Hard on it.
    let expected = [0.0, 1.225, 3.68];
    assert!(
        skills
            .iter()
            .zip(expected)
            .all(|(got, want)| (got - want).abs() < 1e-3),
        "{skills:?}"
    );
    assert_ne!(
        Some(skills[1]),
        oag_tables::track_stats::resolve_skill_scale(cell, Difficulty::Medium, None),
        "the circuit's own curve, not the default"
    );
}

fn seconds(race: &oag_raceplay::Race) -> f32 {
    race.sim
        .world
        .tick
        .saturating_sub(oag_race::COUNTDOWN_TICKS) as f32
        / 60.0
}

/// A shield loss of at least this much on a tick with no new wall contact is
/// read as a weapon hit, for [`fly`]'s respawn allowance.
const HIT: f32 = 5.0;

/// A respawn counts as weapon-caused when a hit came at most this many ticks
/// before it.
const HIT_WINDOW: u64 = 60;

/// The race with the player parked: every opponent finishes inside 300 s
/// with no respawn, or, where a circuit names its cause, at most
/// `field_respawns` in the whole field, each within [`HIT_WINDOW`] ticks of a
/// weapon hit on that craft. The skill is the race's ambient one. Shield is
/// read at each opponent's lap crossings (per lap) and at the end, reported
/// apart.
fn fly(track: &str, class: &str, field_respawns: u32, field_destroyed: u32) {
    let Some(loaded) = loaded(track, class) else {
        return;
    };
    let mut race = oag_raceplay::Race::start(loaded.setup);
    assert!(
        race.use_original_ai(None, 1),
        "Omega flies the original driver"
    );
    let mut lap = [1u32; 8];
    let mut per_lap: [Vec<f32>; 8] = Default::default();
    let mut shield = [0.0f32; 8];
    let mut walls = [0u32; 8];
    let mut last_hit = [None::<u64>; 8];
    let mut respawned = [0u32; 8];
    while seconds(&race) <= 300.0 && !(1..8).all(|s| race.sim.world.ships[s].standing.finished()) {
        race.tick(&PlayerInputs::none());
        let tick = race.sim.world.tick;
        for slot in 1..8 {
            let now_walls = race.wall_contact_ticks_of(slot);
            let ship = &race.sim.world.ships[slot];
            if shield[slot] - ship.physics.shield >= HIT && now_walls == walls[slot] {
                last_hit[slot] = Some(tick);
            }
            shield[slot] = ship.physics.shield;
            walls[slot] = now_walls;
            if race.respawns_of(slot) > respawned[slot] {
                respawned[slot] = race.respawns_of(slot);
                assert!(
                    field_respawns > 0
                        && last_hit[slot].is_some_and(|hit| tick - hit <= HIT_WINDOW),
                    "{track}: slot {slot} respawned at tick {tick}, last weapon hit {:?}",
                    last_hit[slot]
                );
            }
            if ship.standing.lap != lap[slot] {
                lap[slot] = ship.standing.lap;
                per_lap[slot].push(ship.physics.shield);
            }
        }
    }
    println!("{track} {class}: {:.1} s", seconds(&race));
    for (slot, shields) in per_lap.iter().enumerate().skip(1) {
        let ship = &race.sim.world.ships[slot];
        println!(
            "  slot {slot}: finished {} lap {} respawns {} per-lap shield {shields:?}, \
             end-of-run shield {:.1}",
            ship.standing.finished(),
            ship.standing.lap,
            race.respawns_of(slot),
            ship.physics.shield
        );
    }
    // A craft that did not finish must be a wreck (its pool emptied), and at most
    // `field_destroyed` of them.
    let mut destroyed = 0;
    for slot in 1..8 {
        let ship = &race.sim.world.ships[slot];
        if !ship.standing.finished() {
            assert!(
                ship.physics.shield <= 0.0,
                "{track}: slot {slot} did not finish"
            );
            destroyed += 1;
        }
    }
    assert!(
        destroyed <= field_destroyed,
        "{track}: {destroyed} opponent(s) destroyed"
    );
    let total: u32 = (1..8).map(|slot| race.respawns_of(slot)).sum();
    assert!(
        total <= field_respawns,
        "{track}: {total} respawns in the field"
    );
}

macro_rules! circuit {
    ($name:ident, $track:literal, $class:literal) => {
        circuit!($name, $track, $class, 0);
    };
    ($name:ident, $track:literal, $class:literal, $field_respawns:literal) => {
        circuit!($name, $track, $class, $field_respawns, 0);
    };
    ($name:ident, $track:literal, $class:literal, $field_respawns:literal, $field_destroyed:literal) => {
        #[test]
        #[ignore = "needs the decrypted PS4 package pair in data/extracted/ps4/"]
        fn $name() {
            fly($track, $class, $field_respawns, $field_destroyed);
        }
    };
}

circuit!(
    the_field_finishes_vineta_k,
    r"Data\Environments\01_Vineta_K\track.vex",
    "Venom"
);
// **One opponent may be destroyed.** The Turbo's running laws (`craft+0x1c0 & 0x200`: the stun escape and the pad fade,
// `docs/ghidra/functions/psp-pulse-usa/craft-flags.md`) moved it, judged by the noise method on
// 2026-10-20: failures among the 49 `*_original_ai_ground_truth` tests per airborne pitch-torque
// perturbation (-1e-2, -1e-3, -1e-4, -1e-5, 0, 1e-2, 1e-3, 1e-4, 1e-5), laws off `0 0 1 1 0 0 0 2 0`
// (median 0, max 2), laws on `1 2 0 1 4 2 0 0 0` (median 1, max 4): inside the bar.
// At `eps = 0` slot 1 is destroyed (per-lap shield `[80.0, 40.0]`, end-of-run shield
// `0.0`); before the laws every opponent finished.
circuit!(
    the_field_finishes_talons_junction,
    r"Data\Environments\talons_junction\track.vex",
    "Venom",
    0,
    1
);
// The two halved-lookahead circuits, in the class each one halves for.
circuit!(
    the_field_finishes_ubermall_as_phantoms,
    r"Data\Environments\05_Ubermall\track.vex",
    "Phantom"
);
// **Two respawns in the field, each after a weapon hit** (2026-10-19, once
// the field fires; asserted, not only stated). Slot 5 both times: hit at tick
// 5568 (shield 76 to 61 at speed 232, touching no wall), the wall at tick 5608
// at speed 24, off; and hit at tick 11163 (69.9 to 54.9 at speed 314), off by
// tick 11193. Every other circuit stays at none.
circuit!(
    the_field_finishes_amphiseum_reversed,
    r"Data\Environments\amphiseum\track_reversed.vex",
    "Flash",
    2
);
