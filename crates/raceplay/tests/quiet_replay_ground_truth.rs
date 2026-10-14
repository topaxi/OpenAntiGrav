//! A sim-only replay tick against the normal tick, on a real circuit out of a
//! real disc image: eight craft, two of them people, weapons in every hand.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all \
//!     -E 'test(/^quiet_replay_ground_truth::/)'
//! ```
//!
//! The unit tests beside `crates/raceplay/src/tests/reconcile.rs` pin the claim
//! on a fixture: the hash after N quiet ticks is the hash after N normal ticks.
//! A fixture has no walls to scrape, no pickups to spend and no reticle to lock
//! with, and each of those is a place the quiet path skips view work that
//! sits next to simulation work. These tests are the same claim where those
//! happen.

use oag_core::buttons::{Button, Input};
use oag_gameplay::{Controller, InputSnapshot, PlayerInputs};
use oag_raceplay as race;
use oag_tables::weapons::Weapon;

/// Ticks of the start-line countdown, through which thrust is gated.
const COUNTDOWN: u32 = oag_race::COUNTDOWN_TICKS as u32;

/// A pair of pads, one per person, so a press is an edge and not a level.
struct Pads([Input; 2]);

impl Pads {
    fn new() -> Self {
        Self([Input::new(); 2])
    }

    /// One tick's inputs for both people: slot 0 steers by a slow wave, slot 1
    /// by another, both on the throttle, and `fire` presses Square on slot 0.
    fn tick(&mut self, tick: u32, fire: bool) -> PlayerInputs {
        let mut inputs = PlayerInputs::none();
        for slot in 0..2 {
            let mut held = Button::Cross.bit();
            if fire && slot == 0 {
                held |= Button::Square.bit();
            }
            self.0[slot].begin_frame(held);
            let wave = ((tick as f32) * 0.05 + slot as f32 * 2.0).sin() * 0.6;
            inputs.set(
                slot,
                InputSnapshot {
                    buttons: self.0[slot],
                    stick_x: wave,
                    ..InputSnapshot::default()
                }
                .sanitised(),
            );
        }
        inputs
    }
}

/// A single race on Pulse's first circuit: weapons on, seven opponents, the
/// second slot made a remote person.
fn start() -> Option<race::Race> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    assert_eq!(race.ship_count(), 8, "the field is not eight craft");
    race.sim.world.controllers[1] = Controller::Remote;
    Some(race)
}

/// What each slot's craft and rules look like, as text.
fn slots(race: &race::Race) -> Vec<String> {
    (0..usize::from(race.ship_count()))
        .map(|slot| {
            format!(
                "{:?} {:?}",
                race.sim.world.ships[slot], race.sim.world.race[slot]
            )
        })
        .collect()
}

/// Two identical races driven in step to the tick the window opens on, then
/// `window` ticks of one through [`race::Race::tick`] and the other through
/// [`race::Race::tick_sim_only`], compared on every tick and every slot.
///
/// `prime` runs on both once `prime_at` ticks have run (hand out the weapons),
/// early enough that the reticle has had time to lock; `fire` says on which
/// window ticks slot 0 presses Square.
///
/// Returns what the window did, for the caller's non-vacuity checks.
fn compare(
    prime_at: u32,
    lead_in: u32,
    window: u32,
    prime: impl Fn(&mut race::Race),
    fire: impl Fn(u32) -> bool,
) -> Option<Window> {
    let mut normal = start()?;
    let mut quiet = start()?;
    let (mut pads_n, mut pads_q) = (Pads::new(), Pads::new());
    for tick in 0..lead_in {
        if tick == prime_at {
            prime(&mut normal);
            prime(&mut quiet);
        }
        normal.tick(&pads_n.tick(tick, false));
        quiet.tick(&pads_q.tick(tick, false));
        normal.drain_cues();
        quiet.drain_cues();
    }
    assert_eq!(normal.sim.state_hash(), quiet.sim.state_hash());
    let ignited_before = quiet.spark_ignitions();

    let mut seen = Window::default();
    for step in 0..window {
        let tick = lead_in + step;
        normal.tick(&pads_n.tick(tick, fire(step)));
        quiet.tick_sim_only(&pads_q.tick(tick, fire(step)));
        seen.cues += normal.drain_cues().len();
        assert!(
            quiet.pending_cues().is_empty(),
            "a quiet tick queued cues on window tick {step}"
        );
        assert_eq!(
            normal.sim.state_hash(),
            quiet.sim.state_hash(),
            "the quiet path left the normal one at window tick {step}"
        );
        for projectile in &normal.sim.world.projectiles.slots {
            if projectile.kind.is_some() {
                seen.projectile_ticks += 1;
                if projectile.owner == 0 && projectile.target.is_some() {
                    seen.locked_shots += 1;
                }
            }
        }
        seen.spark_ignitions = normal.spark_ignitions() - ignited_before;
    }
    assert_eq!(slots(&normal), slots(&quiet));
    assert_eq!(
        quiet.spark_ignitions(),
        ignited_before,
        "a quiet window ignited collision sparks"
    );
    Some(seen)
}

/// What the compared window contained.
#[derive(Default)]
struct Window {
    cues: usize,
    projectile_ticks: u32,
    /// Ticks on which a shot slot 0 fired carried a lock.
    locked_shots: u32,
    spark_ignitions: u32,
}

/// The primary's Missile takes its lock from the reticle, which is view state.
/// A quiet replay that skipped the reticle would fire the same shot unlocked and
/// leave the hash behind within a tick or two of the launch.
///
/// Run inside the start-line countdown, where the grid is still and a craft is
/// in the window for the whole 140 ticks.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_quiet_replay_fires_the_same_locked_missile() {
    let Some(seen) = compare(
        30,
        140,
        60,
        |race| race.sim.world.ships[0].pickup.weapon = Some(Weapon::Missile),
        |step| step == 2,
    ) else {
        return;
    };
    assert!(
        seen.locked_shots > 0,
        "slot 0's Missile never carried a lock, so the reticle dependency was not exercised"
    );
}

/// The racing case: the field underway, every opponent armed, both people in
/// the pack, and the window long enough to scrape a wall, spend pads and fire.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_quiet_replay_reaches_the_hash_of_a_normal_one_in_a_pack() {
    let Some(seen) = compare(
        COUNTDOWN + 600,
        COUNTDOWN + 600,
        240,
        |race| {
            let weapons = [Weapon::Rocket, Weapon::Bomb, Weapon::Mine, Weapon::Missile];
            for slot in 0..8usize {
                if slot != 1 {
                    race.sim.world.ships[slot].pickup.weapon = Some(weapons[slot % 4]);
                }
            }
        },
        |step| step % 40 == 5,
    ) else {
        return;
    };
    println!(
        "window: {} cues, {} projectile-ticks, {} locked shots, {} spark ignitions on the normal path",
        seen.cues, seen.projectile_ticks, seen.locked_shots, seen.spark_ignitions
    );
    assert!(seen.projectile_ticks > 0, "nothing was fired in the window");
    assert!(seen.cues > 0, "the normal path raised no cue in the window");
}
