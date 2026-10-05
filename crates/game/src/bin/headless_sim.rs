//! `oag-headless-sim`: runs the tick loop with no renderer, no window and no
//! disc, and prints the state hash it arrives at.
//!
//! **A smoke test, not a server.** It exists to prove one thing and nothing
//! else: that `Race`, `RaceSim` and `Race::tick` are reachable through
//! `[lib] oag_game` with no graphics, audio or windowing attached at all. That
//! is the cheap half of what a headless network server needs - rule 2 of
//! `docs/architecture/workspace-layout.md` says no crate may depend on
//! `oag-game`, so a server cannot be some other crate importing
//! `oag_game::race`; a second `[[bin]]` in this crate can, with no new
//! dependency edge and no rule bent.
//!
//! **There is no networking here, and there should not be.** A transport and a
//! message format are undecided, and `workspace-layout.md`'s own warning about
//! `oag-net` - "a crate created before its shape is understood tends to get the
//! wrong shape" - applies to a protocol written before the loopback
//! client/server test that would justify it.
//!
//! **The race it runs means nothing.** [`Setup::headless`] authors no track, no
//! handling and no weapon table, so the craft fall in a vacuum and no lap is
//! ever counted. A hash that moves is the whole assertion: it says the tick ran
//! and changed state, not that it produced a result anyone should read.
//!
//! ```sh
//! cargo run -p oag-game --bin oag-headless-sim
//! cargo run -p oag-game --bin oag-headless-sim -- 600
//! ```

use oag_gameplay::{Controller, PlayerInputs};
use oag_raceplay::{Race, Setup};

/// Ticks to run when the command line says nothing.
const DEFAULT_TICKS: u32 = 120;

/// The seed. Fixed, because two runs of this binary must agree: a smoke test
/// whose answer changed every run could not tell a broken tick from a new one.
const SEED: u64 = 0xC0FFEE;

/// How many slots a person flies, to show the tick consumes more than one
/// snapshot. The rest of the grid stays [`Controller::Ai`], exactly as a normal
/// race leaves it.
const HUMANS: usize = 3;

fn main() {
    let ticks = std::env::args()
        .nth(1)
        .map_or(Ok(DEFAULT_TICKS), |arg| arg.parse())
        .unwrap_or_else(|e| {
            eprintln!("ticks: {e}");
            std::process::exit(2);
        });

    let mut race = Race::start(Setup::headless(oag_race::Mode::SingleRace, SEED));

    // Three people and five drivers. Written here rather than by `Race::start`
    // because the front end is what decides how many people are racing, and
    // there is no front end in this process - which is the point.
    for slot in 0..HUMANS {
        race.sim.world.controllers[slot] = Controller::Local;
    }
    let humans: Vec<usize> = race.sim.world.human_slots().collect();
    // One craft, not eight: `opponents` asks for a grid and a grid is spawned
    // along a racing line, which `Setup::headless` does not author. Said out
    // loud rather than papered over - the slots past the first consume a
    // snapshot and reach the hash, which is what this binary is checking, and
    // a full grid needs a track this deliberately has not got.
    println!(
        "{} craft on the grid (no racing line to spawn one), human slots {humans:?}",
        race.sim.world.ship_count
    );

    let before = race.sim.state_hash();
    println!("tick {:>6}: {before:#018x}", race.sim.world.tick);

    for tick in 0..ticks {
        // A different held mask per slot, so a tick that quietly read slot 0's
        // snapshot for everybody would still produce slot 0's answer and this
        // would not notice. Nothing steers - the craft have `Handling::ZERO` -
        // but the masks reach the hash through the per-slot `Input` edges.
        let mut inputs = PlayerInputs::none();
        for &slot in &humans {
            let mut snapshot = oag_gameplay::InputSnapshot::EMPTY;
            snapshot.stick_x = slot as f32 * 0.25 - 0.5;
            snapshot
                .buttons
                .begin_frame(if tick % (slot as u32 + 2) == 0 {
                    oag_gameplay::input::Button::Cross.bit()
                } else {
                    0
                });
            inputs.set(slot, snapshot.sanitised());
        }
        race.tick(&inputs);
        // The cues a tick emits are an output, never state (ADR-0018). With no
        // audio attached they would otherwise grow without bound.
        race.drain_cues();
    }

    let after = race.sim.state_hash();
    println!("tick {:>6}: {after:#018x}", race.sim.world.tick);

    assert_eq!(
        race.sim.world.tick,
        u64::from(ticks),
        "the tick loop did not advance the clock"
    );
    assert_ne!(
        before, after,
        "{ticks} ticks changed no state at all - the simulation did not run"
    );
    println!("ok: {ticks} ticks with no renderer attached");
}
