//! The Missile, on a real circuit out of a real disc image.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this crate:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all
//! ```
//!
//! # Its own file rather than a section of `race_ground_truth.rs`
//!
//! That file is at its `BASELINE` ceiling in `scripts/check-file-size.py` - 2,387
//! lines, which it may shrink below but not grow past - so a line added to it
//! fails `just check-size`. A new file under `crates/game/tests/` is bound by the
//! plain 1,000-line limit instead. The harness helpers below are therefore
//! deliberate near-duplicates of that file's; they are eight lines each and
//! sharing them would need a `tests/common/` module for no other reason.
//!
//! # What only real data can say here
//!
//! Three things, and none of them is checkable on a synthetic fixture:
//!
//! 1. **That a missile closes on its target.** A guidance law that ran but did
//!    nothing - a clamp with the wrong sign, a target read as a position that is
//!    always the missile's own - looks exactly like a straight shot, and a
//!    straight shot down a synthetic corridor hits anyway.
//! 2. **That it survives a wall.** The Rocket detonates on its first; the Missile
//!    glances off up to five times. A missile wrongly wired to the Rocket's model
//!    passes any test that only asserts "it eventually detonates".
//! 3. **That the lock's along-track screen does not reject everything.** The ratio
//!    test needs a real closed course to measure a lap against, and a synthetic
//!    straight skips it entirely - so a threshold set the wrong way round would be
//!    invisible in every unit test while making the weapon dead on real geometry.
//!    This is the same trap `an_opponent_fires_at_a_craft_ahead_on_a_real_circuit`
//!    exists for on the AI's curvature gate.

use std::path::PathBuf;

use oag_gameplay::PlayerInputs;
use oag_gameplay::input::{Button, Input};
use oag_raceplay as race;
use oag_tables::weapons::Weapon;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/pulse-psp-usa.chd")
}

/// A single race: the one mode with weapons on, and so the only one a pickup can
/// happen in at all.
fn single_race() -> Option<race::Loaded> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        ..race::Options::default()
    })
    .expect("loading the race");
    for line in &loaded.report {
        println!("{line}");
    }
    Some(loaded)
}

/// One input snapshot with `button` newly pressed, off a fresh `Input`.
///
/// Two `begin_frame` calls, because the fire path is edge-triggered: a snapshot
/// built with the bit already held reports no press.
fn press(button: Button) -> oag_gameplay::InputSnapshot {
    let mut buttons = Input::new();
    buttons.begin_frame(0);
    buttons.begin_frame(button.bit());
    oag_gameplay::InputSnapshot {
        buttons,
        ..oag_gameplay::InputSnapshot::new()
    }
}

/// A missile fired at a craft ahead ends up closer to it than a straight shot
/// would have.
///
/// **The assertion that separates homing from flying.** The straight-line
/// counterfactual is computed from the missile's own launch heading and its own
/// travelled distance, so it is not a hand-picked number: it is where *this*
/// missile would have been had it never turned. A guidance term that ran and did
/// nothing puts the two within float noise of each other.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_missile_turns_toward_the_craft_it_locked() {
    let Some(loaded) = single_race() else { return };
    let mut race = race::Race::start(loaded.setup);
    // A few ticks so the grid settles onto its footprints and the standings take
    // a first fix - the lock's along-track screen reads `Standing::progress`.
    for _ in 0..30 {
        race.tick(&PlayerInputs::none());
    }

    // **Slot 0 is slid sideways so the target is off-axis.** On the corridor-midpoint
    // grid (`grid.md`) slot 0 and the slot it locks share a column, so the shot is
    // launched 0.13 degrees off the line to the target and a straight shot and a
    // homing one end within a tenth of a unit whichever way the sign falls - the
    // assertion below cannot fail on homing. Four units to the craft's left puts the
    // target a few degrees off, which is asserted as a precondition.
    {
        let body = &mut race.sim.world.ships[0].physics.body;
        let left = -body.right();
        body.position += left * 4.0;
    }

    let stats = race.missile_stats().expect("the disc authors a Missile");

    // Slot 0 sits at the back of the grid, so everybody is ahead of it. Fire by
    // hand rather than through a pad, because which weapon a pad hands out is a
    // draw and this test is about the missile rather than about the draw.
    //
    // **Held for a full second before firing**, since `Race::fire_missile` now
    // gates slot 0's candidate on `Race::sight`'s own lock - see its doc comment
    // - and that reticle only starts accumulating hold once the pickup is a
    // Missile. A second clears the recovered `sight::HOLD_SECONDS` (0.8) with
    // room for the f32 accumulation to land on either side of the boundary.
    race.sim.world.ships[0].pickup.weapon = Some(Weapon::Missile);
    for _ in 0..60 {
        race.tick(&PlayerInputs::none());
    }
    assert!(
        race.sight().locked(),
        "the reticle never locked in a full second of holding a Missile at a \
         full grid - the hold gate would silently turn this test's shot unguided"
    );
    assert!(
        race.fire_missile(0, &stats),
        "nothing on a full grid was lockable from the back of it - the lock's \
         window, cone or along-track screen is refusing everything on real geometry"
    );

    let launched = race
        .sim
        .world
        .projectiles
        .slots
        .iter()
        .find(|p| p.kind == Some(Weapon::Missile))
        .copied()
        .expect("a missile is in the air");
    let target = launched
        .target
        .expect("a launched missile carries its lock");
    let start = launched.position;
    let heading = launched.velocity.normalize();

    // Fly it until it stops, and remember the last tick it was still alive on.
    let mut last = launched;
    let mut travelled = 0.0;
    for _ in 0..240 {
        race.tick(&PlayerInputs::none());
        let Some(current) = race
            .sim
            .world
            .projectiles
            .slots
            .iter()
            .find(|p| p.kind == Some(Weapon::Missile) && p.owner == 0)
            .copied()
        else {
            break;
        };
        travelled = (current.position - start).length();
        last = current;
    }

    let target_position = race.sim.world.ships[target as usize].physics.body.position;
    let homed = (last.position - target_position).length();
    let straight = (start + heading * travelled - target_position).length();
    println!(
        "missile travelled {travelled:.1}, ended {homed:.1} from slot {target}; \
         a straight shot would have ended {straight:.1} away"
    );
    let off_line = heading
        .dot((target_position - start).normalize())
        .clamp(-1.0, 1.0)
        .acos()
        .to_degrees();
    println!("launched {off_line:.2} degrees off the line to the target");
    assert!(
        off_line > 1.0,
        "the shot was launched {off_line:.2} degrees off its target, so a straight \
         and a homing missile cannot be told apart and this test measures nothing"
    );
    assert!(
        travelled > 0.0,
        "the missile never moved, so nothing was measured"
    );
    assert!(
        homed < straight,
        "the missile ended {homed:.1} from its target where flying straight would \
         have ended {straight:.1} away - the guidance term is not steering"
    );
}

/// A missile never locks the craft that fired it, on a full grid where seven
/// other craft are available and the firer is the nearest thing to itself.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_missile_never_locks_its_own_firer() {
    let Some(loaded) = single_race() else { return };
    let mut race = race::Race::start(loaded.setup);
    for _ in 0..30 {
        race.tick(&PlayerInputs::none());
    }
    let stats = race.missile_stats().expect("the disc authors a Missile");

    // Slot 0 is the one candidate `Race::fire_missile` additionally gates on
    // `Race::sight` - see its doc comment - so without this the loop below
    // would exercise the self-lock invariant against every opponent slot and
    // never against the player's own. Every other slot's candidate is used as
    // soon as `lock` finds one and needs no priming.
    race.sim.world.ships[0].pickup.weapon = Some(Weapon::Missile);
    for _ in 0..60 {
        race.tick(&PlayerInputs::none());
    }

    let mut locks = 0;
    for slot in 0..race.sim.world.ship_count as usize {
        if !race.fire_missile(slot, &stats) {
            continue;
        }
        locks += 1;
        for projectile in race.sim.world.projectiles.slots.iter() {
            if projectile.kind != Some(Weapon::Missile) {
                continue;
            }
            assert_ne!(
                projectile.target,
                Some(projectile.owner),
                "slot {} locked itself",
                projectile.owner
            );
        }
    }
    assert!(
        locks > 0,
        "no slot on a full grid could lock anything, so nothing was checked"
    );
}

/// A missile glances off the circuit's own barriers instead of dying on them.
///
/// The Rocket detonates on its first face-on hit; the Missile mirrors and carries
/// on up to `MAX_BOUNCES`. This is the property that says the Missile is on its
/// own flight rules rather than sharing the Rocket's.
///
/// # Why this fires *unguided* missiles, and why that is not cheating
///
/// The first version of this test fired locked missiles from every slot and
/// watched for a bounce. **105 of them produced none**, and the reason is not a
/// broken bounce: a locked missile homes at a craft and reaches its hull - which
/// always detonates, being the other collision code - long before it meets a
/// barrier. The bounce is what happens to a missile whose target got away, and
/// arranging that on a live grid is a test about the AI rather than about the
/// weapon.
///
/// So this puts missiles in the air aimed **across** the track, with no lock,
/// which is exactly the state a real missile ends up in when its target dies. The
/// geometry it bounces off is still the circuit's own, which is the part a
/// synthetic fixture cannot supply. `projectile::tests` covers the reflection
/// arithmetic itself against a flat wall.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_missile_glances_off_the_circuits_barriers() {
    let Some(loaded) = single_race() else { return };
    let mut race = race::Race::start(loaded.setup);
    for _ in 0..30 {
        race.tick(&PlayerInputs::none());
    }
    let stats = race.missile_stats().expect("the disc authors a Missile");
    let launch_kmh = stats.launch_speed;

    // Aimed across the track from the player's craft, sweeping the whole circle
    // so the test does not depend on which side the barrier happens to be on at
    // this point of this circuit.
    let ship = race.sim.world.ships[0].physics.body;
    let forward = ship.forward();
    let right = ship.right();
    let mut seen_bounce = false;
    let mut flown = 0;
    'outer: for step in 0..16 {
        let angle = std::f32::consts::TAU * step as f32 / 16.0;
        let (sin, cos) = angle.sin_cos();
        let heading = forward * cos + right * sin;
        race.sim.world.projectiles.clear();
        assert!(
            race.sim.world.projectiles.spawn_guided(
                Weapon::Missile,
                ship.position + heading * 2.0,
                heading * 100.0,
                0,
                None,
                launch_kmh,
                ship.up(),
            ),
            "the projectile array refused a spawn into an empty world"
        );
        flown += 1;
        for _ in 0..90 {
            race.tick(&PlayerInputs::none());
            if race
                .sim
                .world
                .projectiles
                .slots
                .iter()
                .any(|p| p.kind == Some(Weapon::Missile) && p.bounces > 0)
            {
                seen_bounce = true;
                break 'outer;
            }
        }
    }
    println!("fired {flown} unguided missiles, saw a bounce: {seen_bounce}");
    assert!(
        seen_bounce,
        "none of {flown} missiles fired in every direction from the start line \
         glanced off anything - a missile is detonating on its first wall the way \
         a rocket does"
    );
}

/// A weapon pad on a real circuit can hand out a Missile, and firing one spends
/// the pickup.
///
/// **Scans seeds rather than pinning one.** `pickup::IMPLEMENTED` is expected to
/// grow, and the draw walks it in order, so a magic seed goes quietly wrong the
/// next time a weapon lands - the same rule
/// `race_ground_truth::a_weapon_pad_on_the_disc_hands_out_a_pickup_in_a_single_race`
/// records and for the same reason.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_pad_can_hand_out_a_missile_and_firing_one_spends_it() {
    let Some(loaded) = single_race() else { return };
    let weapons = loaded.setup.weapons.clone();
    let stats = weapons
        .as_ref()
        .and_then(oag_tables::weapons::WeaponStats::missile)
        .expect("the disc authors a Missile");

    let table = weapons
        .as_ref()
        .expect("the disc authors a weapon table")
        .pickups_for("Venom")
        .expect("Venom authors pickup odds");
    // The authored table has to weight the Missile at all, or the scan below is
    // looking for something the disc never hands out.
    assert!(
        table
            .get(Weapon::Missile)
            .is_some_and(|odds| odds.human > 0.0),
        "the shipped Venom table weights no Missile for a human, so this test is \
         asserting something the disc does not do"
    );

    let mut rng = oag_core::Rng::new(1);
    let drawn = (0..256).any(|_| {
        oag_weapons::pickup::draw(
            &mut rng,
            table,
            oag_weapons::pickup::Driver::HUMAN_UNPLACED,
            None,
            None,
        ) == Some(Weapon::Missile)
    });
    assert!(
        drawn,
        "256 draws off the shipped Venom table produced no Missile, though it is \
         weighted - the draw is not reaching it"
    );

    // And firing one spends it, through the button path rather than by hand.
    let mut race = race::Race::start(single_race().expect("the image is present").setup);
    for _ in 0..30 {
        race.tick(&PlayerInputs::none());
    }
    race.sim.world.ships[0].pickup.weapon = Some(Weapon::Missile);
    // `false` now means the pool was full and nothing else - a missing lock is
    // no longer a refusal, so this asserts the array had room rather than that
    // anything was lockable.
    assert!(
        race.fire_missile(0, &stats),
        "the projectile array was full"
    );
    race.sim.world.projectiles.clear();

    race.sim.world.ships[0].pickup.weapon = Some(Weapon::Missile);
    race.tick(&PlayerInputs::single(press(Button::Square)));
    assert_eq!(
        race.ship_pickup(),
        None,
        "firing a missile did not spend the pickup"
    );
    assert!(
        race.sim.world.projectiles.live() > 0,
        "the pickup was spent and nothing left the rail"
    );
}

/// A missile with nothing to lock leaves the rail on a real circuit, and ends
/// itself.
///
/// **The real-data half of the change that made an unlocked press fire.**
/// `Ship_FireHeldWeapon` (`0x08844ae8`) calls `Weapon_RequestFire` on both arms
/// of its lock test, and the pool's second pass in `Projectiles_Update_q`
/// (`0x08869588`) detonates anything older than
/// `missile::SELF_DETONATE_SECONDS`. A synthetic straight cannot say either
/// thing honestly: it has no along-track screen for the lock to run, and nothing
/// for a ballistic missile to ride or glance off on the way out.
///
/// Fired from **every** slot on a full grid, because which of them has nobody in
/// its cone is a property of the shipped starting grid rather than something to
/// hardcode - the leader looks at empty road, and that is the shot this is about.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_missile_with_no_lock_still_flies_a_real_circuit_and_ends_itself() {
    use oag_weapons::projectile::missile::SELF_DETONATE_SECONDS;

    let Some(loaded) = single_race() else { return };
    let mut race = race::Race::start(loaded.setup);
    for _ in 0..30 {
        race.tick(&PlayerInputs::none());
    }
    let stats = race.missile_stats().expect("the disc authors a Missile");

    for slot in 0..race.sim.world.ship_count as usize {
        assert!(
            race.fire_missile(slot, &stats),
            "slot {slot} put nothing in the air - a press now fires whether or              not anything is lockable, so the only `false` left is a full pool"
        );
    }

    let unguided = race
        .sim
        .world
        .projectiles
        .slots
        .iter()
        .filter(|p| p.kind == Some(Weapon::Missile) && p.target.is_none())
        .count();
    assert!(
        unguided > 0,
        "every craft on the shipped starting grid found something to lock, so          the unguided shot was never exercised"
    );

    // Two ticks past the recovered timer, to cover the tick of rounding the age
    // picks up over three seconds.
    let ticks = (SELF_DETONATE_SECONDS * 60.0) as usize + 2;
    for _ in 0..ticks {
        race.tick(&PlayerInputs::none());
    }

    let survivors = race
        .sim
        .world
        .projectiles
        .slots
        .iter()
        .filter(|p| p.kind == Some(Weapon::Missile) && p.target.is_none())
        .count();
    assert_eq!(
        survivors, 0,
        "{survivors} unguided missiles outlived {SELF_DETONATE_SECONDS}s on a          real circuit - the pool's own expiry is not ending them"
    );
}
