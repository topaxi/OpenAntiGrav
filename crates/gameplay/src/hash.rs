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
//! the pad timers live on `oag_raceplay::Race` rather than in the world - they
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
//! [`Weapon`]: oag_tables::weapons::Weapon
//! [`Mode`]: oag_race::Mode

use oag_core::hash::StateHasher;
use oag_race::{LapGate, Mode, RaceState};
use oag_tables::weapons::{DisruptorEffectKind, Weapon};

use crate::world::{Controller, Ship, World};
use oag_weapons::pickup::Held;
use oag_weapons::projectile::leach_beam::{Beam, Kind as BeamKind};
use oag_weapons::projectile::quake::Wave;
use oag_weapons::projectile::repulser::Repulser;
use oag_weapons::projectile::{Projectile, Projectiles};

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
        controllers,
        projectiles,
        quake,
        leach_beam,
        repulsers,
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

    // **Every slot, for [`World::ships`]'s reason exactly.** A slot's clock is
    // simulation state whether or not a HUD draws it, an AI slot's lap counter
    // already decides the field's order, and hashing only the human ones would
    // make the stream's shape depend on how the grid was configured - the thing
    // the fixed arrays exist to prevent.
    for state in race {
        write_race(hasher, state);
    }
    // Beside them, because who flies a slot changes what the slot does with an
    // input: the same snapshot steers a `Local` craft and is ignored by an `Ai`
    // one. See `oag_gameplay::world::Controller`.
    for controller in controllers {
        write_controller(hasher, *controller);
    }
    write_projectiles(hasher, projectiles);
    write_quake(hasher, quake);
    write_leach_beam(hasher, leach_beam);
    for repulser in repulsers {
        write_repulser(hasher, repulser);
    }
}

fn write_ship(hasher: &mut StateHasher, ship: &Ship) {
    let Ship {
        physics,
        handling,
        segment,
        pickup,
        driver,
        weapon_ai,
        standing,
        autopilot_timer,
        pending_slowdown,
        pending_thrust_scale,
        disruption,
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
    // Two counters, zero through every scenario the gate runs - none collects
    // a pickup - so eight fixed bytes per ship per tick: adding them moved the
    // committed hashes with no behaviour changing.
    let oag_ai::weapon_ai::WeaponAi {
        held_ticks,
        wait_ticks,
    } = weapon_ai;
    hasher.write_u32(*held_ticks);
    hasher.write_u32(*wait_ticks);
    write_standing(hasher, standing);
    // Zero through every scenario the gate runs - none collects a pickup - so
    // this contributes a fixed run of four bytes per ship per tick and nothing
    // else. That is why adding it moved the committed hashes with no behaviour
    // changing; see the history note in `tests/determinism.rs`.
    hasher.write_f32(*autopilot_timer);
    // The same shape as `autopilot_timer` above: zero through every scenario the
    // gate runs, because none of them fires a weapon, so it is a fixed run of
    // four more bytes per ship per tick. It moved the committed hashes with no
    // behaviour changing, and `tests/determinism.rs`'s history note records the
    // isolation that proved that.
    hasher.write_f32(*pending_slowdown);
    // The beam's one-shot throttle, `1.0` through every scenario the gate runs
    // for the reason the two above are `0.0`: none fires a LeachBeam. Four
    // more fixed bytes per ship per tick, and the history note in
    // `tests/determinism.rs` records the move.
    hasher.write_f32(*pending_thrust_scale);
    // A Disruptor hit's kind and its seconds left. The kind is
    // `DisruptorEffectKind::ALL`'s index plus one, `0` for none - the same
    // shape `write_weapon` gives a pickup. Zero and `0.0` through every
    // scenario the gate runs, so five fixed bytes per ship per tick: adding
    // it moved the committed hashes with no behaviour changing, the third
    // time this struct has done that. See `oag_weapons::disruption`.
    write_disruption(hasher, disruption);
    hasher.write_u8(u8::from(*active));
}

fn write_disruption(hasher: &mut StateHasher, disruption: &oag_weapons::disruption::Disruption) {
    let oag_weapons::disruption::Disruption { kind, timer } = disruption;
    write_effect_kind(hasher, *kind);
    hasher.write_f32(*timer);
}

/// A Disruptor effect as a discriminant byte, `0` for none.
///
/// [`DisruptorEffectKind::ALL`]'s index plus one - the original parser's own
/// branch order rather than this file's opinion, the same argument
/// [`write_weapon`] makes.
fn write_effect_kind(hasher: &mut StateHasher, kind: Option<DisruptorEffectKind>) {
    let discriminant = match kind {
        None => 0,
        Some(kind) => DisruptorEffectKind::ALL
            .iter()
            .position(|&candidate| candidate == kind)
            .map_or(0, |index| index as u8 + 1),
    };
    hasher.write_u8(discriminant);
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
        peak_curvature,
        reflex,
        roll_decided,
        branching,
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
    // **And the high-water mark that decides whether the differential
    // airbrake reads a corner as still being entered or as opening up.** Two
    // runs that agree on every position but disagree on this are about to
    // spend the differential differently, which is a different yaw, a
    // different grip, and a different line. See
    // `oag_ai::Driver::peak_curvature`.
    hasher.write_u32(*peak_curvature);
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
    // **And whether it has already made its mind up about a barrel roll this
    // jump.** One decision per airborne window, so two runs that agree on every
    // position and disagree on this are about to spend a twelfth of a shield
    // pool differently. See `oag_ai::Driver::roll_decided`.
    hasher.write_u8(u8::from(*roll_decided));
    // **And which side of a fork it is on.** Two runs whose craft agree on
    // every position but whose drivers took different coins are about to steer
    // down different roads. See `oag_ai::branch`.
    let oag_ai::branch::Branching {
        route,
        decided_at,
        entered,
        visits,
        pending,
    } = branching;
    hasher.write_u32(u32::from(*route));
    hasher.write_u32(u32::from(*decided_at));
    hasher.write_u8(u8::from(*entered));
    hasher.write_u32(*visits);
    hasher.write_u32(u32::from(*pending));
}

/// A craft's place in the race, which decides the finishing order and is
/// therefore simulation state rather than presentation.
fn write_standing(hasher: &mut StateHasher, standing: &oag_race::Standing) {
    let oag_race::Standing {
        lap,
        gate,
        reversed,
        progress,
        course_index,
        finish_tick,
        lap_start_tick,
        best_lap_ticks,
        lap_splits,
        kills,
        deaths,
    } = standing;

    hasher.write_u32(*lap);
    hasher.write_u8(match gate {
        LapGate::NeedsNearHalf => 0,
        LapGate::NeedsFarHalf => 1,
        LapGate::Ready => 2,
    });
    write_reversed(hasher, *reversed);
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
    for split in lap_splits {
        write_option_u32(hasher, *split);
    }
    // Eliminator-only counts, `0` on every other mode - see `Standing::kills`.
    hasher.write_u32(*kills);
    hasher.write_u32(*deaths);
}

/// The backward-crossing deficit, written only while there is one so that a
/// race that never reversed over the line hashes as it did before the field.
fn write_reversed(hasher: &mut StateHasher, reversed: u32) {
    if reversed > 0 {
        hasher.write_u8(0xA5);
        hasher.write_u32(reversed);
    }
}

fn write_held(hasher: &mut StateHasher, held: &Held) {
    let Held {
        weapon,
        last,
        dropping,
        drop_reload,
        cannon_rounds,
        cannon_reload,
    } = held;
    write_weapon(hasher, *weapon);
    // **The previous grant is state, not a convenience.** The draw refuses to
    // hand out the same weapon twice running, so two runs whose craft carry the
    // same thing but remember different last grants are about to be handed
    // different pickups. See `oag_weapons::pickup::draw`.
    write_weapon(hasher, *last);
    // **A drop in progress is state too, and it is the first pickup state that
    // spans ticks.** Two worlds carrying the same Mine differ if one has three
    // of its cluster still to lay and the other has one - they are about to put
    // a different number of mines on the track, in different places. The reload
    // goes in for the same reason at finer grain: it decides *which* tick the
    // next one leaves on, and a mine's position is where the craft was on that
    // tick. See `oag_weapons::pickup::Held::advance_drop`.
    hasher.write_u8(*dropping);
    hasher.write_f32(*drop_reload);
    // **The Cannon's own countdown is state that spans ticks too**, for the
    // same two reasons the drop's is: how many rounds are left decides how
    // much longer this craft goes on firing, and the reload decides which
    // tick the next one leaves on. See
    // `oag_weapons::pickup::Held::advance_cannon_reload`.
    hasher.write_u8(*cannon_rounds);
    hasher.write_f32(*cannon_reload);
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

/// One [`Controller`] as a discriminant byte.
///
/// Written out rather than taken from `as u8`, so reordering the enum's
/// variants cannot silently move every committed reference hash.
fn write_controller(hasher: &mut StateHasher, controller: Controller) {
    hasher.write_u8(match controller {
        Controller::Ai => 0,
        Controller::Local => 1,
        Controller::Remote => 2,
    });
}

fn write_race(hasher: &mut StateHasher, race: &RaceState) {
    let RaceState {
        mode,
        lap,
        laps_target,
        progress,
        lap_start_tick,
        best_lap_ticks,
        lap_splits,
        zone,
        zone_timer,
        score,
        lap_gate,
        reversed,
        zone_dirty,
        finished,
        course_index,
    } = race;

    hasher.write_u8(match mode {
        Mode::TimeTrial => 0,
        Mode::SpeedLap => 1,
        Mode::Zone => 2,
        Mode::SingleRace => 3,
        Mode::Eliminator => 4,
        Mode::Tournament => 5,
        Mode::Head2Head => 6,
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
    for split in lap_splits {
        write_option_u32(hasher, *split);
    }
    hasher.write_u32(u32::from(*zone));
    hasher.write_f32(*zone_timer);
    hasher.write_i32(*score);
    hasher.write_u8(match lap_gate {
        LapGate::NeedsNearHalf => 0,
        LapGate::NeedsFarHalf => 1,
        LapGate::Ready => 2,
    });
    write_reversed(hasher, *reversed);
    hasher.write_u8(u8::from(*zone_dirty));
    hasher.write_u8(u8::from(*finished));
    write_option_u32(hasher, *course_index);
}

/// A discriminant byte first, the same shape [`write_option_u32`] and every
/// other `Option` here takes - see that function's own doc comment for why.
fn write_quake(hasher: &mut StateHasher, quake: &Option<Wave>) {
    match quake {
        None => hasher.write_u8(0),
        Some(wave) => {
            hasher.write_u8(1);
            let Wave {
                owner,
                progress,
                direction,
                damage,
                radius,
                slowdown_time,
                hit,
                age,
            } = wave;
            hasher.write_u8(*owner);
            hasher.write_f32(*progress);
            hasher.write_f32(*direction);
            hasher.write_f32(*damage);
            hasher.write_f32(*radius);
            hasher.write_f32(*slowdown_time);
            hasher.write_f32(*age);
            for latch in hit {
                hasher.write_u8(u8::from(*latch));
            }
        }
    }
}

/// [`write_quake`]'s shape: a discriminant byte, then every field.
fn write_repulser(hasher: &mut StateHasher, repulser: &Option<Repulser>) {
    let Some(repulser) = repulser else {
        hasher.write_u8(0);
        return;
    };
    hasher.write_u8(1);
    let Repulser {
        owner,
        age,
        damage,
        force,
        slowdown_time,
        blast_time,
        wave_time,
        fronts,
        hit,
        fork,
    } = repulser;
    hasher.write_u8(*owner);
    for value in [
        *age,
        *damage,
        *force,
        *slowdown_time,
        *blast_time,
        *wave_time,
    ] {
        hasher.write_f32(value);
    }
    match fronts {
        None => hasher.write_u8(0),
        Some(fronts) => {
            hasher.write_u8(1);
            for front in fronts {
                hasher.write_u32(front.index);
                for v in [front.point, front.previous] {
                    hasher.write_f32(v.x);
                    hasher.write_f32(v.y);
                    hasher.write_f32(v.z);
                }
            }
        }
    }
    for latch in hit {
        hasher.write_u8(u8::from(*latch));
    }
    // **Only a live fork writes anything**, not a `None` byte: a Repulser that
    // never crosses a split hashes exactly as it did before the fork existed,
    // so every circuit without one keeps its committed reference.
    if let Some(fork) = fork {
        hasher.write_u8(1);
        hasher.write_u32(u32::from(fork.branch.unwrap_or(u16::MAX)));
        hasher.write_u32(fork.offset);
        hasher.write_u32(fork.front.index);
        for v in [fork.front.point, fork.front.previous] {
            hasher.write_f32(v.x);
            hasher.write_f32(v.y);
            hasher.write_f32(v.z);
        }
    }
}

/// [`write_quake`]'s twin, discriminant byte and all.
///
/// The nested `Option<f32>` goes through the same shape a second time: a byte
/// for "is it there" and a value only when it is, so a beam that broke at
/// `0.0` seconds cannot hash the same as one that has not broken.
fn write_leach_beam(hasher: &mut StateHasher, leach_beam: &Option<Beam>) {
    match leach_beam {
        None => hasher.write_u8(0),
        Some(beam) => {
            hasher.write_u8(1);
            let Beam {
                owner,
                target,
                kind,
                age,
                disconnected_at,
                first_drain,
                first_repair,
                damage,
                repair,
                range,
                active_time,
                energy_multiplier,
                slow_ship_factor,
            } = beam;
            hasher.write_u8(*owner);
            hasher.write_u8(*target);
            hasher.write_u8(match kind {
                BeamKind::Locked => 0,
                BeamKind::Unlocked => 1,
            });
            hasher.write_f32(*age);
            match disconnected_at {
                None => hasher.write_u8(0),
                Some(at) => {
                    hasher.write_u8(1);
                    hasher.write_f32(*at);
                }
            }
            hasher.write_u8(u8::from(*first_drain));
            hasher.write_u8(u8::from(*first_repair));
            hasher.write_f32(*damage);
            hasher.write_f32(*repair);
            hasher.write_f32(*range);
            hasher.write_f32(*active_time);
            hasher.write_f32(*energy_multiplier);
            hasher.write_f32(*slow_ship_factor);
        }
    }
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
        age,
        surface,
        target,
        bounces,
        launch_speed_kmh,
        charge,
        effect,
        // Deliberately excluded. `orientation` is a laid charge's frozen
        // drawing pose - `Projectile::orientation`'s own doc comment - and
        // nothing in this crate reads it back on a later tick, so it cannot
        // be the reason two runs diverge. Hashing it would only make this
        // reference move the day the field started being set, for a value the
        // determinism check exists to catch divergence *in*, not one that
        // stays the same on every re-run of the same input by construction.
        orientation: _,
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
    // A laid charge's age decides the tick its owner stops being exempt from it,
    // so it is simulation state. It is `0.0` for every weapon that is not a laid
    // Mine or Bomb and for a free slot, and written only when raised, so the
    // committed references of races that lay nothing do not move.
    if *age != 0.0 {
        hasher.write_f32(*age);
    }
    // A plasma bolt's wind-up decides the tick it starts flying on and
    // therefore where it is on every tick after that, so it is simulation
    // state - unlike `orientation` below. `0.0` for every other weapon and for
    // a bolt already in the air, so this only moves the reference for a race
    // that actually fires a Plasma. See
    // `oag_weapons::projectile::plasma::CHARGE_SECONDS`.
    hasher.write_f32(*charge);
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
    // The effect a Disruptor bolt carries to whoever it hits - decided when
    // it was fired, spent when it lands, so it is state for the whole flight.
    // `0` for every other weapon and for a free slot.
    write_effect_kind(hasher, *effect);
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
    /// `oag_weapons::projectile::Projectiles::spawn` something the gate can see.
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
        world.race[0].lap += 1;
        assert_ne!(before, hash_world(&world));

        let mut progressed = World::new(1);
        progressed.race[0].progress = Some(0.0);
        assert_ne!(
            hash_world(&World::new(1)),
            hash_world(&progressed),
            "'not yet located' must not hash as 'at the start line'"
        );
    }

    /// Every slot's clock, not only slot 0's - the widening of 2026-09-16 would
    /// be worth nothing to a replay if seven eighths of it were outside the
    /// gate, and hashing only the slot a HUD happens to draw is exactly the
    /// hole this file exists to close.
    #[test]
    fn every_players_race_state_moves_the_hash() {
        let base = hash_world(&World::new(1));
        for slot in 0..crate::world::MAX_PLAYERS {
            let mut world = World::new(1);
            world.race[slot].lap += 1;
            assert_ne!(base, hash_world(&world), "slot {slot} is outside the hash");
        }
    }

    /// Who flies a slot is state as much as what the slot is doing: the same
    /// snapshot steers a `Local` craft and is ignored by an `Ai` one, so two
    /// worlds that disagree here are one tick from disagreeing about a force.
    #[test]
    fn who_flies_a_slot_moves_the_hash() {
        let base = hash_world(&World::new(1));
        for slot in 0..crate::world::MAX_PLAYERS {
            for controller in [Controller::Ai, Controller::Local, Controller::Remote] {
                let mut world = World::new(1);
                if world.controllers[slot] == controller {
                    continue;
                }
                world.controllers[slot] = controller;
                assert_ne!(
                    base,
                    hash_world(&world),
                    "slot {slot} as {controller:?} is outside the hash"
                );
            }
        }
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
