//! The determinism gate's race-level half.
//!
//! [`oag_physics::probe::hash_state`] covers one craft's dynamics and cannot
//! cover anything else: `oag-physics` depends on nothing but `oag-core`, so it
//! can name neither a [`Weapon`] nor a [`Mode`]. Everything a race carries
//! *around* those dynamics - the inventory, the projectiles, the lap state and
//! the generator's own position - was therefore outside the gate entirely, which
//! `docs/gameplay/pickups.md` recorded as a known hole with a worked example:
//! a pad refresh timer one tick out shifts the whole draw stream, and every
//! committed hash stays green.
//!
//! This is the fix that page names. [`hash_world`] covers the whole [`World`];
//! the pad timers live on `oag_game::race::Race` rather than in the world - they
//! belong to the track, not to a craft - and `Race::state_hash` folds them in
//! on top of this.
//!
//! # Adding a field is a compile error here, on purpose
//!
//! Every struct below is destructured **exhaustively, with no `..` rest
//! pattern**, exactly as `oag_physics::probe::hash_state` does it. A new field
//! stops this file compiling instead of silently being left out of the hash -
//! and a hash that silently covers less than it claims is worse than none,
//! because it still passes.
//!
//! **`oag_ai::Driver` was the exception until 2026-08-18**, read by name inside
//! [`write_ship`] rather than destructured in its own function - so this
//! paragraph was a promise the file did not keep, on the one struct that was
//! growing fastest. Finding D3 of that day's review. A struct that arrives here
//! gets a `write_*` of its own; reading `foo.bar` off one is the shape to
//! notice.
//!
//! [`Weapon`]: oag_formats::weapons::Weapon
//! [`Mode`]: oag_race::Mode

use oag_core::hash::StateHasher;
use oag_formats::weapons::Weapon;
use oag_race::{LapGate, Mode, RaceState};

use crate::pickup::Held;
use crate::projectile::{Projectile, Projectiles};
use crate::world::{Ship, World};

/// One 64-bit fingerprint of a whole world.
///
/// Two runs that agree on this at every tick agree on every bit of simulation
/// state the world holds.
#[must_use]
pub fn hash_world(world: &World) -> u64 {
    let mut hasher = StateHasher::new();
    write_world(&mut hasher, world);
    hasher.finish()
}

/// Absorbs a whole world into a hasher the caller owns.
///
/// Separate from [`hash_world`] so a caller with more state of its own - the
/// composition root, with its pad timers - folds the two into one value rather
/// than hashing a hash.
pub fn write_world(hasher: &mut StateHasher, world: &World) {
    let World {
        tick,
        rng,
        ships,
        ship_count,
        race,
        projectiles,
    } = world;

    hasher.write_u64(*tick);

    // **Where the generator is, not only what it emitted.** Two runs whose ships
    // agree but whose generators have drawn a different number of values have
    // already diverged - the next pickup will differ and nothing before it will
    // show why. See `oag_core::Rng::snapshot`.
    for word in rng.snapshot() {
        hasher.write_u32(word);
    }

    // **Every slot, not only the active prefix.** An inactive slot holds
    // whatever the last race left in it, and a run that cleared it differs from
    // one that did not; hashing the prefix would hide that. The count is hashed
    // beside them so "eight ships, one active" cannot collide with "one ship".
    hasher.write_u8(*ship_count);
    for ship in ships {
        write_ship(hasher, ship);
    }

    write_race(hasher, race);
    write_projectiles(hasher, projectiles);
}

fn write_ship(hasher: &mut StateHasher, ship: &Ship) {
    let Ship {
        physics,
        handling,
        segment,
        pickup,
        driver,
        standing,
        autopilot_timer,
        active,
    } = ship;

    oag_physics::probe::hash_state(hasher, physics);
    // `handling` is **not** hashed: it is read-only tunables loaded off the
    // player's disc, identical for every tick of a race, and hashing it would
    // make the gate's value depend on which ship was picked rather than on what
    // the simulation did. Named in the destructure so this is a decision rather
    // than an omission.
    let _ = handling;
    hasher.write_u32(u32::from(*segment));
    write_held(hasher, pickup);
    write_driver(hasher, driver);
    write_standing(hasher, standing);
    // Zero through every scenario the gate runs - none collects a pickup - so
    // this contributes a fixed run of four bytes per ship per tick and nothing
    // else. That is why adding it moved the committed hashes with no behaviour
    // changing; see the history note in `tests/determinism.rs`.
    hasher.write_f32(*autopilot_timer);
    hasher.write_u8(u8::from(*active));
}

/// The opponent driver's own state, which seeds next tick's decisions.
///
/// **Its own function, destructured exhaustively**, like every other struct in
/// this module. It was seven fields read by name inside [`write_ship`] until
/// finding D3 of the 2026-08-18 review: `Driver` grew faster than anything else
/// in the world that week, and a field added to it would have compiled clean
/// and silently dropped out of the hash - the exact drift this module's own
/// docs promise is a compile error. Nothing about what is written changed, so
/// no committed constant moves.
fn write_driver(hasher: &mut StateHasher, driver: &oag_ai::Driver) {
    let oag_ai::Driver {
        index,
        seed,
        phase,
        place,
        provocation,
        pilot,
        mistake,
        reflex,
    } = driver;

    // **Hashed, unlike `handling`.** A driver's place on the racing line is the
    // seed for next tick's windowed search, so two runs whose craft agree but
    // whose drivers are looking at different stretches of line have already
    // diverged - the next steering command will differ and nothing before it
    // will show why. It is `0` for slot 0, which the player flies.
    hasher.write_u32(*index);
    // **The seed and the tick count with it.** The seed decides this craft's
    // whole character - which part of the corridor it holds, how hard it
    // commits to a corner - and the count is the argument its drift is a
    // function of, so two runs that agree on every position but disagree on
    // either are about to steer differently. See `oag_ai::Personality`.
    hasher.write_u32(*seed);
    hasher.write_u32(*phase);
    // **And the grudge.** `place` is what an overtake is detected against and
    // `provocation` is how long the last one still stings for; a driver that
    // has just been passed covers its line harder, so two runs agreeing on
    // every position and disagreeing on either are about to drive differently.
    // `write_u32` rather than a `u16` write because `StateHasher` has no
    // sixteen-bit one, and inventing a narrower write to save two bytes in a
    // hash is not worth a second way to feed it.
    hasher.write_u32(u32::from(*place));
    hasher.write_u32(u32::from(*provocation));
    // **And which pilot it is flying.** Unlike `handling` above, this can come
    // out of the player's own config directory and so differs between machines
    // by design; left out, two machines running "the same race" with different
    // pilot files would agree on the hash and disagree on the race. See
    // `oag_ai::Driver::pilot`.
    hasher.write_u32(*pilot);
    // **And whether it is in the middle of getting one wrong.** A driver
    // sailing through a braking point it should have taken is about to be
    // somewhere a driver that braked is not. See `oag_ai::Driver::mistake`.
    hasher.write_u32(u32::from(*mistake));
    // **And what it has noticed of the craft around it.** Reaction latency
    // holds a rival back for a few ticks after it arrives, so two runs that
    // agree on every position and disagree on which craft their drivers have
    // seen are about to lift, cover and shoot at different moments. The
    // half-noticed rival and its countdown are in it too: they decide *when*
    // the next one is seen, which is the same argument. See
    // `oag_ai::Reflex`.
    let oag_ai::Reflex {
        seen,
        pending,
        wait,
    } = reflex;
    for channel in 0..3 {
        hasher.write_u8(seen[channel]);
        hasher.write_u8(pending[channel]);
        hasher.write_u32(u32::from(wait[channel]));
    }
}

/// A craft's place in the race, which decides the finishing order and is
/// therefore simulation state rather than presentation.
fn write_standing(hasher: &mut StateHasher, standing: &oag_race::Standing) {
    let oag_race::Standing {
        lap,
        gate,
        progress,
        course_index,
        finish_tick,
        lap_start_tick,
        best_lap_ticks,
    } = standing;

    hasher.write_u32(*lap);
    hasher.write_u8(match gate {
        LapGate::NeedsNearHalf => 0,
        LapGate::NeedsFarHalf => 1,
        LapGate::Ready => 2,
    });
    // A discriminant byte first, for the reason `write_race` gives about
    // `progress`: "not yet located" must not hash the same as "at the line".
    match progress {
        None => hasher.write_u8(0),
        Some(progress) => {
            hasher.write_u8(1);
            hasher.write_f32(*progress);
        }
    }
    write_option_u32(hasher, *course_index);
    match finish_tick {
        None => hasher.write_u8(0),
        Some(tick) => {
            hasher.write_u8(1);
            hasher.write_u64(*tick);
        }
    }
    // The lap clock. A discriminant byte apiece for the same reason `progress`
    // and `finish_tick` carry one: "the clock has not started" must not hash the
    // same as "the clock started on tick zero", which is the whole reason
    // `lap_start_tick` is an `Option` where `RaceState`'s is a bare `u64`.
    match lap_start_tick {
        None => hasher.write_u8(0),
        Some(tick) => {
            hasher.write_u8(1);
            hasher.write_u64(*tick);
        }
    }
    write_option_u32(hasher, *best_lap_ticks);
}

fn write_held(hasher: &mut StateHasher, held: &Held) {
    let Held { weapon, last } = held;
    write_weapon(hasher, *weapon);
    // **The previous grant is state, not a convenience.** The draw refuses to
    // hand out the same weapon twice running, so two runs whose craft carry the
    // same thing but remember different last grants are about to be handed
    // different pickups. See `crate::pickup::draw`.
    write_weapon(hasher, *last);
}

/// A weapon slot as a discriminant byte, with `0` reserved for "nothing".
///
/// The mapping is [`Weapon::ALL`]'s index plus one, so it is the executable's
/// own class-pool order rather than this file's opinion - and a weapon inserted
/// into that pool moves the hash, which is correct, because it moves what a draw
/// means.
fn write_weapon(hasher: &mut StateHasher, weapon: Option<Weapon>) {
    let discriminant = match weapon {
        None => 0,
        Some(weapon) => Weapon::ALL
            .iter()
            .position(|&candidate| candidate == weapon)
            .map_or(0, |index| index as u8 + 1),
    };
    hasher.write_u8(discriminant);
}

fn write_race(hasher: &mut StateHasher, race: &RaceState) {
    let RaceState {
        mode,
        lap,
        laps_target,
        progress,
        lap_start_tick,
        best_lap_ticks,
        zone,
        zone_timer,
        score,
        lap_gate,
        zone_dirty,
        finished,
        course_index,
    } = race;

    hasher.write_u8(match mode {
        Mode::TimeTrial => 0,
        Mode::SpeedLap => 1,
        Mode::Zone => 2,
        Mode::SingleRace => 3,
    });
    hasher.write_u32(*lap);
    write_option_u32(hasher, *laps_target);
    // A discriminant byte first, or "not yet located" hashes the same as "at the
    // start line" - which is the difference between a lap that can be counted
    // and one that cannot.
    match progress {
        None => hasher.write_u8(0),
        Some(progress) => {
            hasher.write_u8(1);
            hasher.write_f32(*progress);
        }
    }
    hasher.write_u64(*lap_start_tick);
    write_option_u32(hasher, *best_lap_ticks);
    hasher.write_u32(u32::from(*zone));
    hasher.write_f32(*zone_timer);
    hasher.write_i32(*score);
    hasher.write_u8(match lap_gate {
        LapGate::NeedsNearHalf => 0,
        LapGate::NeedsFarHalf => 1,
        LapGate::Ready => 2,
    });
    hasher.write_u8(u8::from(*zone_dirty));
    hasher.write_u8(u8::from(*finished));
    write_option_u32(hasher, *course_index);
}

fn write_projectiles(hasher: &mut StateHasher, projectiles: &Projectiles) {
    let Projectiles { slots } = projectiles;
    for slot in slots {
        write_projectile(hasher, slot);
    }
}

fn write_projectile(hasher: &mut StateHasher, projectile: &Projectile) {
    let Projectile {
        kind,
        position,
        velocity,
        owner,
        lifetime,
        surface,
        target,
        bounces,
        launch_speed_kmh,
    } = projectile;

    // Every field of every slot, free or not - the same argument the inactive
    // ship slots get above. A freed slot is reset to `Projectile::default`, so
    // two runs that freed the same slot agree here and one that leaked a stale
    // position does not.
    write_weapon(hasher, *kind);
    hasher.write_vec3(*position);
    hasher.write_vec3(*velocity);
    hasher.write_u8(*owner);
    hasher.write_f32(*lifetime);
    // The surface being ridden decides which way next tick probes, so it is
    // simulation state and not a cached convenience.
    hasher.write_vec3(*surface);
    // All three are guidance state and all three steer a missile, so all three
    // are hashed. `target` is written as a discriminant plus a slot rather than
    // as a slot with a sentinel, so "no lock" and "locked slot 0" cannot collide
    // - slot 0 is the player, which is the one every opponent shoots at.
    hasher.write_u8(u8::from(target.is_some()));
    hasher.write_u8(target.unwrap_or(0));
    hasher.write_u8(*bounces);
    hasher.write_f32(*launch_speed_kmh);
}

fn write_option_u32(hasher: &mut StateHasher, value: Option<u32>) {
    match value {
        None => hasher.write_u8(0),
        Some(value) => {
            hasher.write_u8(1);
            hasher.write_u32(value);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use oag_core::math::Vec3;

    #[test]
    fn two_identical_worlds_hash_the_same() {
        let a = World::new(7);
        let b = World::new(7);
        assert_eq!(hash_world(&a), hash_world(&b));
    }

    #[test]
    fn a_different_seed_is_a_different_world() {
        assert_ne!(hash_world(&World::new(1)), hash_world(&World::new(2)));
    }

    /// **The gap this whole module closes**, at world scale: a generator that
    /// has been drawn from is not the generator it started as, even when nothing
    /// visible has moved. Without this the next pickup differs and nothing
    /// before it shows why.
    #[test]
    fn drawing_from_the_generator_moves_the_hash() {
        let mut world = World::new(1);
        let before = hash_world(&world);
        let _ = world.rng.next_f32();
        assert_ne!(before, hash_world(&world), "the generator's position");
    }

    /// The inventory, which reaches no force and therefore reaches
    /// `oag_physics::probe::hash_state` not at all.
    #[test]
    fn what_a_craft_is_carrying_moves_the_hash() {
        let mut world = World::new(1);
        let before = hash_world(&world);
        world.ships[0].pickup.weapon = Some(Weapon::Rocket);
        let with_rocket = hash_world(&world);
        assert_ne!(before, with_rocket);

        // And two different weapons are two different states, which a plain
        // `is_some` byte would miss.
        world.ships[0].pickup.weapon = Some(Weapon::Shield);
        assert_ne!(with_rocket, hash_world(&world));
    }

    /// A projectile a metre further along is a different world. This is the one
    /// that would have gone unnoticed longest without a race-level hash:
    /// nothing about a rocket in flight touches a ship's state until it lands.
    #[test]
    fn a_projectile_moving_moves_the_hash() {
        let mut world = World::new(1);
        world
            .projectiles
            .spawn(Weapon::Rocket, Vec3::ZERO, Vec3::Z, 0);
        let before = hash_world(&world);
        world.projectiles.slots[0].position += Vec3::Z;
        assert_ne!(before, hash_world(&world));
    }

    /// A rocket in slot 0 and the same rocket in slot 1 are different worlds -
    /// which is what makes the "first free slot in index order" rule in
    /// `crate::projectile::Projectiles::spawn` something the gate can see.
    #[test]
    fn which_slot_a_projectile_is_in_moves_the_hash() {
        let mut first = World::new(1);
        first.projectiles.slots[0] = Projectile {
            kind: Some(Weapon::Rocket),
            position: Vec3::Z,
            velocity: Vec3::Z,
            owner: 0,
            lifetime: 1.0,
            surface: Vec3::Y,
            ..Projectile::default()
        };
        let mut second = World::new(1);
        second.projectiles.slots[1] = first.projectiles.slots[0];
        assert_ne!(hash_world(&first), hash_world(&second));
    }

    /// Lap state, which is neither a force nor a pickup and had no gate at all.
    #[test]
    fn the_race_rules_move_the_hash() {
        let mut world = World::new(1);
        let before = hash_world(&world);
        world.race.lap += 1;
        assert_ne!(before, hash_world(&world));

        let mut progressed = World::new(1);
        progressed.race.progress = Some(0.0);
        assert_ne!(
            hash_world(&World::new(1)),
            hash_world(&progressed),
            "'not yet located' must not hash as 'at the start line'"
        );
    }

    /// An inactive slot still counts, because a run that cleared it differs from
    /// one that did not.
    #[test]
    fn a_stale_inactive_slot_is_visible() {
        let mut world = World::new(1);
        let before = hash_world(&world);
        world.ships[5].physics.body.position = Vec3::X;
        assert_ne!(
            before,
            hash_world(&world),
            "an inactive slot's contents were hidden from the gate"
        );
    }

    /// Handling is deliberately outside the hash - it is read-only tunables off
    /// the player's disc, constant for the whole race.
    #[test]
    fn the_ships_tunables_are_not_part_of_the_state() {
        let mut world = World::new(1);
        let before = hash_world(&world);
        world.ships[0].handling.engine.turbo += 1.0;
        assert_eq!(before, hash_world(&world));
    }
}
