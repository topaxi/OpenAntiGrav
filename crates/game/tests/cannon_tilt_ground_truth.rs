//! A Cannon round leaves the craft's nose on a tilted track, out of a real disc.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. Run it with `just test-data`.
//!
//! A maintainer report: on a tilted track (Moa Therma's loop) the cannon fired
//! with an offset to the side of the craft. The spawn was never at fault -
//! `cannon::launch` builds the muzzle from the craft's full orientation - the
//! flight was: a round born with world up as its ridden normal ran the generic
//! floor probe, found the floor under the *world* and snapped to it. On this
//! circuit's autopilot lap that moved a round up to 7 units off its muzzle
//! (and about 1 unit down even on flat track). `Cannon_UpdateRound` has no probe.
//!
//! The assertion is on residuals, so it names the law and not an authored value:
//! the round after its first tick is exactly where `cannon::launch` put it plus
//! one step along the launch velocity.

use oag_gameplay::PlayerInputs;
use oag_gameplay::input::{Button, Input};
use oag_raceplay as race;
use oag_tables::weapons::Weapon;

const MOA_THERMA_WHITE: &str = r"Data\Environments\03_Track\track.vex";

fn snapshot(bits: u32, previous: Option<&Input>) -> oag_gameplay::InputSnapshot {
    let mut buttons = previous.copied().unwrap_or_else(Input::new);
    buttons.begin_frame(bits);
    oag_gameplay::InputSnapshot {
        buttons,
        ..oag_gameplay::InputSnapshot::new()
    }
}

/// `(bank_cos, residual)` for every round slot 0 fired on one lap of the autopilot.
fn residuals() -> Option<Vec<(f32, f32)>> {
    let image = oag_testdata::image("data/images/pulse-psp-eu.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        track: Some(MOA_THERMA_WHITE.to_string()),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        opponent_teams: Vec::new(),
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    let dt = race.dt();
    let mut input: Option<Input> = None;
    let mut seen = [None::<u32>; oag_weapons::projectile::MAX_PROJECTILES];
    let mut out = Vec::new();
    for tick in 0..4_000_u32 {
        let burst = tick >= 240 && tick % 90 < 20;
        // The autopilot fires nothing, so it flies between bursts and the pad
        // takes over for each one.
        race.set_autopilot(!burst);
        if burst {
            race.sim.world.ships[0].pickup.weapon = Some(Weapon::Cannon);
        }
        let bits = if burst {
            Button::Cross.bit() | Button::Square.bit()
        } else {
            Button::Cross.bit()
        };
        let snap = snapshot(bits, input.as_ref());
        input = Some(snap.buttons);
        let before = race.sim.world.ships[0].physics;
        let dimensions = race.sim.world.ships[0].handling.dimensions;
        race.tick(&PlayerInputs::single(snap));
        for (slot, round) in race.sim.world.projectiles.slots.iter().enumerate() {
            if round.kind != Some(Weapon::Cannon) || round.owner != 0 {
                seen[slot] = None;
                continue;
            }
            if seen[slot].replace(tick).is_some() {
                continue;
            }
            let residual = [false, true]
                .into_iter()
                .map(|left| {
                    let (position, velocity) =
                        oag_weapons::projectile::cannon::launch(&before, &dimensions, left);
                    (round.position - (position + velocity * dt)).length()
                })
                .fold(f32::MAX, f32::min);
            out.push((before.body.up().y, residual));
        }
    }
    Some(out)
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_cannon_round_leaves_the_muzzle_on_a_tilted_track() {
    let Some(shots) = residuals() else { return };
    let tilted = shots.iter().filter(|(bank, _)| *bank < 0.95).count();
    assert!(shots.len() >= 100, "only {} rounds fired", shots.len());
    assert!(tilted >= 20, "only {tilted} rounds fired on a tilt");
    let worst = shots.iter().map(|(_, r)| *r).fold(0.0_f32, f32::max);
    println!(
        "{} rounds, {tilted} fired on a tilt, worst residual {worst}",
        shots.len()
    );
    assert!(
        worst < 1e-3,
        "a round left {worst} units from its muzzle ({} shots, {tilted} tilted)",
        shots.len()
    );
}

/// The Missile and the Shuriken leave a banked craft riding **its** up, as
/// `Missile_Init` and `Shuriken_Init` store `-craft+0xb10`, not world up.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_missile_fired_on_a_bank_is_born_riding_the_craft_up() {
    let Some(image) = oag_testdata::image("data/images/pulse-psp-eu.chd") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        track: Some(MOA_THERMA_WHITE.to_string()),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        opponent_teams: Vec::new(),
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    race.set_autopilot(true);
    let stats = race.missile_stats().expect("the disc authors a Missile");
    let mut input: Option<Input> = None;
    for _ in 0..4_000_u32 {
        let snap = snapshot(Button::Cross.bit(), input.as_ref());
        input = Some(snap.buttons);
        race.tick(&PlayerInputs::single(snap));
        let up = race.sim.world.ships[0].physics.body.up();
        if up.y > 0.9 {
            continue;
        }
        race.sim.world.projectiles.clear();
        assert!(race.fire_missile(0, &stats));
        let missile = race
            .sim
            .world
            .projectiles
            .slots
            .iter()
            .find(|p| p.kind == Some(Weapon::Missile))
            .expect("a missile in the air");
        assert!(
            (missile.surface - up).length() < 1e-4,
            "born riding {:?}, the craft's up is {up:?}",
            missile.surface
        );
        return;
    }
    panic!("the autopilot never banked past 25 degrees in 4000 ticks");
}
