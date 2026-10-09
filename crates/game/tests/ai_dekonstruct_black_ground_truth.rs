//! The field on de Konstruct Black, which the lone-Ace gates cannot see.
//!
//! **Black is the forward `05_Track`** (`track.vex`) and White the reversed one,
//! `21_Track` (`track_reversed.vex`): the disc's own Track Select titles say so
//! and PPSSPP's start positions confirmed it live on 2026-10-03 (see
//! `docs/formats/track.md`, "White and Black"). The reversed layout (White) is
//! the control and has an own bound.
//!
//! The lone-Ace gates (`race_ground_truth.rs`, `ai_clean_lap_gate.rs`) race slot
//! 1 alone, and slot 1 is the pole position: the one craft that arrives at the
//! first jump fast enough. A field of seven reaches it behind a leader, a few
//! units a second short, and a craft that leaves the ground short of the lip is
//! destroyed or beached. That is what a player saw ("the AI really struggles
//! with de Konstruct Black") and what no lone row measured.
//!
//! Seven AI Aces, the player's slot parked, weapons as the mode ships them.
//! One `#[test]` per (speed class, seed), so the matrix is the test axis and a
//! failure names its cell. `BOUND` is a ceiling on **craft destroyed**, frozen
//! from `print_bounds` (`OAG_SWEEP=1`, release); it may only fall.

use oag_gameplay::PlayerInputs;
use oag_physics::CraftState;
use oag_raceplay as race;
use oag_raceplay::catalogue;

/// Five game-minutes, the window every other AI board uses: a later lap kills too.
const TICKS: u64 = 18_000;

fn entry(reversed: bool) -> Option<String> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let mut archives = oag_pulse::open(&image.display().to_string()).ok()?;
    let blob = archives
        .read_name(oag_pulse::names::GAME_PLUGIN_DEFINITION)
        .ok()?;
    let definition = oag_tables::fexml::expand(&blob).ok()?;
    catalogue::tracks(&definition)
        .into_iter()
        .find(|track| track.location.ends_with("05_Track") && track.reversed == reversed)
        .map(|track| track.entry_name())
}

/// How many of the seven craft ended the run in any state but racing.
fn destroyed(class: &str, seed: u64, reversed: bool) -> Option<u32> {
    let entry = entry(reversed)?;
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: class.to_string(),
        mode: oag_race::Mode::SingleRace,
        difficulty: oag_ai::Difficulty::Ace,
        track: Some(entry),
        seed: Some(seed),
        ..race::Options::default()
    })
    .unwrap_or_else(|error| panic!("loading 05_Track at {class}: {error}"));
    let mut race = race::Race::start(loaded.setup);
    race.sim.world.ships[0].active = false;
    let count = race.ship_count() as usize;
    let mut dead = vec![false; count];
    for _ in 0..TICKS {
        race.tick(&PlayerInputs::none());
        for (slot, flag) in dead.iter_mut().enumerate().skip(1) {
            if race.sim.world.ships[slot].physics.craft_state != CraftState::Racing {
                *flag = true;
            }
        }
    }
    Some(dead.iter().filter(|flag| **flag).count() as u32)
}

/// Ceilings on craft destroyed, `(class, seed, forward, reversed)`.
///
/// **Regenerated 2026-10-03 for the AI speed plan** (`oag_ai::SpeedPlan`): the
/// reversed control now drives a verified plan, Black does not (its plan does
/// not verify, so its column is unchanged). Reversed destroyed over the twelve
/// cells 11 -> 10: four cells fell by one, two rose by one (RAPIER seed 3,
/// PHANTOM seed 2), which is the per-seed spread of a field with weapons on,
/// not a column getting worse.
///
/// **Tightened 2026-10-03 again (lane `pulse-ai-05`)**: Black's plan verifies
/// now (its line leaves a magstrip on the first jump's run-up, the corridor
/// there stops short of the strip, and an opponent grounded well beneath its
/// line is rescued), so its column fell 49 -> 4 over the twelve cells. The
/// reversed column regenerated identical. Without the corridor bound alone
/// Black reads 6 (RAPIER seeds 2 and 3, PHANTOM seed 3 each one higher).
/// **Regenerated 2026-10-03 for the opponent fire law** (`oag_ai::weapon_ai`,
/// the original's `WeaponAi_DecideFireOrAbsorb`): opponents fire forward
/// weapons on a different schedule, so every race after the first shot is a
/// different race. Black 4 -> 5 and reversed 10 -> 10 over the twelve cells;
/// nine cells moved, up and down. Not weapon deaths: of the craft destroyed,
/// one was finished by a weapon blow under either law (shield lost in the 2 s
/// before, less wall charge, over 20), measured in release with both laws on
/// one binary.
///
/// **Regenerated 2026-10-05 for the fork choice** (`oag_ai::branch`, the
/// original's `Ai_ChooseBranch`): Black forks once, and opponents now take
/// either side, so every race past the fork is a different race. Black 5 -> 6
/// over the twelve cells; six cells moved, three down and three up, and the
/// reversed column regenerated identical. **No loss is on the route.** The
/// cells over the old bound were located (release probe, same seeds): VENOM
/// seed 2 lost one craft to a weapon on the ring (shield 57 to 0 in a tick, no
/// wall charge, ring sample 2115) and one finished at the ring's first-jump
/// lip already at 2.7 shield (ring sample 206); RAPIER seed 2 lost one to wall
/// attrition on the ring (72.5 charged, ring sample 2900, lap 5).
///
/// **Regenerated 2026-10-07 for the Cannon's straight flight and the craft-up
/// seed** (lane `weapon-tilt`): a round no longer snaps to a ride height under
/// world up, so every race after an opponent's first shot is a different race.
/// Five cells rose (FLASH 1 forward 0 -> 2 and reversed 0 -> 1, PHANTOM 1
/// reversed 0 -> 1, PHANTOM 2 1 -> 2 and 1 -> 3, PHANTOM 3 forward 0 -> 1,
/// RAPIER 2 reversed 1 -> 2) and two fell (RAPIER 1, VENOM 2); the forward
/// total went 6 -> 8 and the reversed one stayed 10.
///
/// **Cause separated 2026-10-07 (lane `ai-loss-split`,
/// `ai_loss_attribution_board`)**: the shield each destroyed craft lost, split
/// into wall charge, barrel-roll cost and the rest (every other drain of the
/// pool is a weapon: `apply_weapon` is called only by the Cannon, blast, Quake,
/// Leach Beam and Repulser). Of the 10 extra deaths in the raised cells, 7 were
/// weapon-dominant (a blast, Missile or Mine did more than the walls and rolls
/// put together): FLASH 1 (two forward, one reversed), PHANTOM 2 (one forward,
/// slots 4 and 7 reversed), RAPIER 2 reversed. Three were wall-dominant, a
/// craft at its usual scrape spots topped up by a hit: PHANTOM 1 reversed,
/// PHANTOM 2 reversed slot 1, PHANTOM 3 forward. The scrape spots and the wall
/// share per death are the same before and after (mean wall charge per death
/// 25.9 -> 19.3 forward, 38.5 -> 39.6 reversed), a Cannon hit applies no
/// impulse, and over seeds 4-23 (160 races) deaths were 110 before, 110 after.
/// The cells moved because the races diverged, not because the AI drives worse.
/// Read the cells as seed spread, as the 2026-10-05 note above does.
///
/// **Three cells raised 2026-10-08 for the Mine's six-tick drop** (lane
/// `pulse-mine-drop`; the field now lays a cluster every 6 ticks, not 7, as the
/// original does): RAPIER 1 reversed 0 -> 1, RAPIER 3 forward 1 -> 3, PHANTOM 3
/// forward 1 -> 3; only the exceeded half of a row moved, every other bound
/// stays (the other cells measured at or under theirs). Cause, per cell, with
/// the old spacing against the new: the deaths are weapon-dominant (PHANTOM 3
/// slots 5 and 6, RAPIER 1 reversed slot 6, RAPIER 3 slots 2, 3 and 4: weapon
/// shield loss over wall plus roll) apart from PHANTOM 3 slot 4 (wall 57, roll
/// 23, weapon 15). Mines are not shown to be the killer (a blast leaves no
/// projectile to attribute), and the spread is per-seed divergence: over
/// PHANTOM and RAPIER seeds 4-13 (80 races) deaths were 40 with the fix and 41
/// with the old spacing (forward 18 against 19, reversed 22 against 22).
///
/// **Four cells raised 2026-10-09 for holding the plan's line into a tight
/// corner** (lane `hd-ai-talon`; within 200 units of a sample the plan passed a
/// wall closer than a unit, a driver spends none of its level's plan slack, see
/// `docs/gameplay/ai.md`, "Tight corners"): FLASH 2 reversed 0 -> 1, FLASH 3
/// forward 0 -> 1, PHANTOM 1 reversed 1 -> 2, PHANTOM 2 forward 2 -> 3; only
/// the exceeded halves. Over seeds 1-23, every class, both layouts
/// (`ai_dekonstruct_black_board`, `OAG_SEEDS=23`), destroyed went 138 -> 142 and
/// wall-contact ticks 86,765 -> 84,889; by class VENOM 15 -> 10, FLASH 16 -> 15,
/// RAPIER 46 -> 46, PHANTOM 61 -> 71 (forward 25 -> 33, reversed 36 -> 38).
/// PHANTOM forward is the one class that moved beyond seed spread.
const BOUND: &[(&str, u64, u32, u32)] = &[
    ("VENOM", 1, 0, 0),
    ("VENOM", 2, 2, 1),
    ("VENOM", 3, 0, 0),
    ("FLASH", 1, 2, 1),
    ("FLASH", 2, 0, 1),
    ("FLASH", 3, 1, 2),
    ("RAPIER", 1, 1, 1),
    ("RAPIER", 2, 1, 2),
    ("RAPIER", 3, 3, 3),
    ("PHANTOM", 1, 0, 2),
    ("PHANTOM", 2, 3, 3),
    ("PHANTOM", 3, 3, 2),
];

fn check(class: &str, seed: u64) {
    let Some(forward) = destroyed(class, seed, false) else {
        return;
    };
    let reversed = destroyed(class, seed, true).expect("the reversed entry is on the disc");
    let (_, _, forward_bound, reversed_bound) = BOUND
        .iter()
        .find(|(c, s, _, _)| *c == class && *s == seed)
        .expect("a bound for every cell");
    println!("{class} seed {seed}: forward {forward} destroyed, reversed {reversed}");
    assert!(
        forward <= *forward_bound,
        "{class} seed {seed}: {forward} of 7 destroyed on de Konstruct Black, bound {forward_bound}"
    );
    assert!(
        reversed <= *reversed_bound,
        "{class} seed {seed}: {reversed} of 7 destroyed on the reversed layout, bound {reversed_bound}"
    );
}

macro_rules! cell {
    ($name:ident, $class:expr, $seed:expr) => {
        #[test]
        #[ignore = "needs a disc image in data/images/"]
        fn $name() {
            check($class, $seed);
        }
    };
}

cell!(venom_seed_1, "VENOM", 1);
cell!(venom_seed_2, "VENOM", 2);
cell!(venom_seed_3, "VENOM", 3);
cell!(flash_seed_1, "FLASH", 1);
cell!(flash_seed_2, "FLASH", 2);
cell!(flash_seed_3, "FLASH", 3);
cell!(rapier_seed_1, "RAPIER", 1);
cell!(rapier_seed_2, "RAPIER", 2);
cell!(rapier_seed_3, "RAPIER", 3);
cell!(phantom_seed_1, "PHANTOM", 1);
cell!(phantom_seed_2, "PHANTOM", 2);
cell!(phantom_seed_3, "PHANTOM", 3);

/// Regenerates [`BOUND`]. `OAG_SWEEP=1`, release, `--no-capture`.
#[test]
#[ignore = "a scratch generator: set OAG_SWEEP, paste the rows"]
fn print_bounds() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    for class in ["VENOM", "FLASH", "RAPIER", "PHANTOM"] {
        for seed in 1..=3 {
            let (Some(forward), Some(reversed)) =
                (destroyed(class, seed, false), destroyed(class, seed, true))
            else {
                return;
            };
            println!("    (\"{class}\", {seed}, {forward}, {reversed}),");
        }
    }
}
