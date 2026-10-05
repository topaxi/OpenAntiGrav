//! The determinism gate's race-level half, against committed constants.
//!
//! `crates/physics/tests/determinism.rs`'s twin, and it exists because that one
//! covers one craft's dynamics and nothing else. What a race carries *around*
//! the dynamics - the inventory, the projectiles, the lap state and the
//! generator's own position - was outside every committed hash until
//! [`oag_gameplay::hash::hash_world`] landed, which
//! `docs/gameplay/pickups.md` recorded as a known hole.
//!
//! # No disc image, deliberately
//!
//! The world here is built in this file: two craft, a synthetic wall, a rocket
//! fired down it. `data/` is gitignored and absent in CI, so a disc-backed run
//! would never execute on Windows or macOS - which is where a portability bug
//! shows up. The same argument `oag_physics::probe`'s module docs make.
//!
//! # What this does *not* cover, so nobody assumes it does
//!
//! The weapon pads' own refresh timers and distance caches, which live on
//! `oag_raceplay::Race` rather than in the world - pad timers belong to the
//! track, not to a craft. They are guarded instead by
//! `Race::state_hash`'s tests in `crates/raceplay/src/tests/hash.rs`, which run on every
//! `just` because that fixture needs no disc either. Between the two, nothing in
//! the pickup system is left to `just test-data`.

use oag_core::math::Vec3;
use oag_gameplay::hash::hash_world;
use oag_gameplay::world::World;
use oag_physics::DamageRules;
use oag_physics::params::Dimensions;
use oag_tables::weapons::Weapon;
use oag_weapons::projectile;

mod determinism_support;
use determinism_support::{TICK, corridor, weapon_stats};

/// The scenario: two craft, a rocket in the air, and a draw from the generator
/// every tick.
///
/// Each piece is there because it is a *different* part of the world, and a gate
/// that moved for only one of them would be claiming coverage it does not have:
///
/// - the **projectile array** flies, strikes the second craft's hull at a tick
///   nobody chose, and frees its slot;
/// - the **direct hit** spends the second craft's energy pool and the blast
///   force shoves it, which reaches `oag_physics::probe::hash_state` through
///   the ship. The craft sits where the rocket's own drop puts it at that
///   range - a wall hit spends nothing on anyone since 2026-09-16
///   (`Rocket_HitCraft`/`Rocket_ApplyBlastForce`), so a target 20 units short
///   of the wall would leave the damage path uncovered;
/// - the **generator** is drawn from every tick, so its position advances
///   independently of anything visible;
/// - the **inventory and the lap state** are written on fixed ticks.
fn run(ticks: u32) -> (u64, u64) {
    let world_geometry = corridor();
    let stats = weapon_stats();

    let mut world = World::new(0xC0FFEE);
    world.ship_count = 2;
    for (slot, position) in [Vec3::ZERO, Vec3::new(0.0, -14.0, 380.0)]
        .into_iter()
        .enumerate()
    {
        let ship = &mut world.ships[slot];
        ship.active = true;
        ship.handling.dimensions = Dimensions {
            length: 4.0,
            width: 2.0,
            height: 1.0,
            shield: 100.0,
            ..Dimensions::default()
        };
        ship.physics.shield = 100.0;
        ship.physics.body.mass = 1.0;
        ship.physics.body.position = position;
    }
    world
        .projectiles
        .spawn(Weapon::Rocket, Vec3::ZERO, Vec3::Z * 500.0, 0);

    let mut trajectory = oag_core::hash::StateHasher::new();
    for tick in 0..ticks {
        projectile::step(
            &mut world.projectiles,
            &mut world.ships[..world.ship_count as usize],
            TICK,
            &world_geometry,
            Some(&stats),
            "VENOM",
            DamageRules::default(),
            &mut [],
        );
        // A draw a tick, so the generator's position is not a function of
        // anything else in the run.
        let _ = world.rng.next_f32();
        if tick == 10 {
            world.ships[0].pickup.weapon = Some(Weapon::Shield);
        }
        if tick == 20 {
            world.race[0].lap += 1;
            world.race[0].progress = Some(1.0);
        }
        world.tick += 1;

        oag_gameplay::hash::write_world(&mut trajectory, &world);
    }

    (hash_world(&world), trajectory.finish())
}

/// Committed reference hashes: `(ticks, final, trajectory)`.
///
/// # History
///
/// - **Moved 2026-09-16 (second time today)**, when `Ship::pending_thrust_scale`
///   and `Beam::slow_ship_factor` joined the hash - the LeachBeam's one-shot
///   throttle, `craft+0x31c`, is now armed by the drain and spent by the
///   engine. **Isolated the documented way**: with only the two new
///   `write_f32`s removed and the throttle itself left in, the previous
///   constants - `0xa46c_eb85_8895_811b` / `0x21a2_22cd_c7b6_9b5d` at 60 ticks
///   and `0xb481_6943_0d50_eb4e` / `0xd842_9949_f3b5_9bec` at 600, and
///   `0xa901_3ebe_47aa_6426` / `0xfb1e_cac3_3b77_b824` and
///   `0xae8a_645f_7a5b_05af` / `0xfb29_615f_cef3_e900` for the volley -
///   reproduce bit for bit. Neither scenario fires a beam, so the new field is
///   `1.0` on every ship on every tick and the whole movement is four more
///   bytes per ship entering the stream.
/// - **Moved 2026-09-16**, by a *weapon-law* change and the scenario change it
///   forced. `Rocket_HitCraft` (`0x0886ebdc`) and `Rocket_ApplyBlastForce`
///   (`0x0886ee88`) give the Rocket the Plasma's shape: a hull hit credits the
///   struck craft alone and sweeps the rest for the impulse; a wall hit spends
///   nothing. The old scenario's rocket passed 14 units under the second craft
///   and blew up on the wall, so under that rule it touched nobody and the
///   coverage test said so; the craft now sits at `(0, -14, 380)`, in the
///   rocket's own arc. **Not isolated to one cause**: the rule alone and the
///   position alone each move the constants, so the previous values -
///   `0x3a3f_0ad0_4464_ffda` / `0x05ff_d20f_72e0_15a2` at 60 ticks and
///   `0x4de7_25cf_fbe0_9f7f` / `0xa4a7_dd7b_018f_de2b` at 600 - cannot be
///   reproduced with either in place. Checked instead: the one impact is a
///   `struck: Some(1)` hull hit, the craft loses shield and gains velocity,
///   and `crates/physics`'s own gate did not move.
/// - **Moved 2026-09-06**, when `Driver::roll_decided` joined the hash. Our
///   opponents barrel-roll on purpose - a deliberate, authorized deviation, the
///   original's never do - and that flag is what makes `Pilot::roll_chance` a
///   probability per airborne window rather than a per-tick rate: it records
///   that this driver has already made its mind up about the flight it is in.
///   Two runs agreeing on every position and disagreeing on it are about to
///   spend a twelfth of a shield pool differently, so it is simulation state.
///   **Isolated the documented way**: with `roll_decided` destructured as
///   `roll_decided: _` and its `write_u8` removed, and nothing else changed,
///   the previous constants - `0x3259_455a_d157_329f` / `0x6351_a74b_9671_1835`
///   at 60 ticks and `0xe010_a59d_c8a2_8cc6` / `0x4634_1855_0c96_d234` at 600,
///   and `0x84fa_0540_953f_7dbc` / `0x5544_ea00_8147_b29c` and
///   `0x71c8_e66d_871f_8f3b` / `0x9c54_22cf_8958_48ea` for the volley -
///   reproduce bit for bit. So the whole movement is one more byte per ship per
///   tick entering the stream, and no behaviour at all: **neither scenario ever
///   calls `Driver::drive`**, both step projectiles over a world whose drivers
///   are never run, so the flag reads `false` on every ship on every tick of
///   both. That is the load-bearing claim, not "there is no AI in it".
///   `crates/physics/tests/determinism.rs` did **not** move, and that is the
///   expected shape: the deviation reaches physics as two defaulted
///   `ShipControls` fields, `ShipState` gained nothing, and `oag-physics`
///   cannot depend on `oag-ai` to drive one anyway.
/// - **Moved 2026-08-26**, when `Driver::reflex` joined the hash. Reaction
///   latency holds a rival back for a few ticks after it arrives in one of a
///   driver's three channels, so which craft a driver has *noticed* decides
///   when it lifts, covers and shoots - and the half-noticed rival and its
///   countdown decide when the next one is seen. All three arrays are hashed,
///   for the reason `oag_gameplay::hash::write_driver` gives. **Isolated the
///   documented way**: with `reflex` destructured as `reflex: _` and no writes
///   for it, and nothing else changed, the previous constants -
///   `0xffc0_1c63_959d_dbf6` / `0xe299_6e55_2185_5b6c` at 60 ticks and
///   `0x5ed5_6f2b_3d34_7983` / `0x4611_ee6c_59a8_b981` at 600, and
///   `0xa1a8_de37_f513_74c4` / `0xa755_4730_fc35_b820` and
///   `0xb6e6_af7d_a1f9_fb77` / `0x32e3_49c0_29f2_3e96` for the volley -
///   reproduce bit for bit. That is the whole of the claim that the axis is
///   opt-in: `Tuning::reaction_ticks` defaults to zero, so a driver that has
///   not been given a difficulty notices on the frame exactly as it always
///   did, and what moved here is nine more writes per ship per tick entering
///   the stream. **Neither scenario ever calls `Driver::drive`** - both step
///   projectiles over a world whose drivers are never run - so every channel
///   reads `Reflex::IDLE` throughout both. That is the load-bearing claim, not
///   "there is no AI in it": a reflex only stays idle while nothing advances
///   it.
/// - **Moved 2026-08-24**, when `Ship::autopilot_timer` joined the hash. The
///   Autopilot pickup arms it and the composition root reads it to decide
///   whether slot 0 is flown by its own driver, so it is simulation state and a
///   replay that lost it would diverge - see
///   `docs/ghidra/functions/psp-pulse-usa/autopilot.md`. **Isolated the
///   documented way**: with only `write_f32(*autopilot_timer)` removed from
///   `oag_gameplay::hash::write_ship` and nothing else changed, the previous
///   constants - `0x603b_7db8_2f52_2c56` / `0xe133_7ae6_7445_d5ec` at 60 ticks
///   and `0x38cb_c8b3_20d6_50c3` / `0xf14f_a426_00c6_3801` at 600, and
///   `0x3518_c7a7_def5_c624` / `0xc9d2_d53c_8f24_9cc0` and
///   `0xc8e9_756b_9c7f_7337` / `0x9ae4_9df4_2937_0236` for the volley -
///   reproduce bit for bit. So the movement is four more bytes per ship per
///   tick entering the stream and nothing the simulation does. Neither scenario
///   collects a pickup, so the field is `0.0` throughout both.
///
///   Note that `pickup::IMPLEMENTED` gained `Weapon::Autopilot` in the same
///   change and that did **not** move anything here: these scenarios assign
///   `pickup.weapon` directly rather than drawing from the pool.
///
/// - **Moved 2026-08-17 (second time that day)**, when `pickup::Held` grew
///   `last`. The pickup draw refuses to hand out the same weapon twice running -
///   recovered from `WeaponPickup_Grant` (`0x08861d20`) - so what a craft was
///   last given decides what it can be given next, which makes it simulation
///   state. **Isolated the documented way**: with only `write_held`'s second
///   `write_weapon` removed and nothing else changed, the previous constants -
///   `0x2d22_d564_7848_a77a` / `0x81c4_2326_bf0f_8ddc` at 60 ticks and
///   `0x9dee_92a4_2631_48a3` / `0x95b8_b6ad_9886_1d41` at 600 - reproduce bit
///   for bit. This scenario sets `pickup.weapon` directly rather than through
///   `Held::grant`, so `last` is `None` throughout and what moved is one more
///   byte per ship per tick entering the stream.
/// - **Moved 2026-08-17**, when `Projectile` grew the three fields a guided
///   weapon needs: `target`, `bounces` and `launch_speed_kmh`. All three steer a
///   missile - the lock it is chasing, how many walls it has left to glance off,
///   and the speed its ramp blends away from - so all three are simulation state
///   and all three are hashed, across all 128 slots. **Isolated the documented
///   way**: with only the four new `write_*` calls in
///   `oag_gameplay::hash::write_projectile` removed and nothing else changed, the
///   previous constants - `0xf4f1_bc0c_30a7_27fa` / `0x780b_1ec5_4754_1bdc` at 60
///   ticks and `0xf63d_86b8_9120_e523` / `0x6688_3fef_ea90_5441` at 600 -
///   reproduce bit for bit. So what moved is four more writes per slot entering
///   the stream, and **not** the Rocket's flight, which this scenario is the only
///   committed guard on. The scenario flies no missile, so all three fields are
///   `None`/`0`/`0.0` throughout.
/// - **Moved 2026-08-12**, and by a *force-law* change rather than a wider hash.
///   `oag_physics` now reads `Antigrav::rebound_jump_time`, which it parsed and
///   ignored: the landing response is armed while a craft is in the air and only
///   once the flight has outlasted that parameter, so a short hop no longer
///   applies `landing_rebound`. `ShipState` also gained `time_airborne`, and
///   `time_since_landing`'s initial value became `Ship_InitCraft`'s recovered
///   `10.0`. See that crate's own determinism history for the isolation; the
///   scenario here inherits the movement because it steps real craft.
/// - **Recorded 2026-08-11**, with the module itself. There is no earlier value
///   to compare against - this is the first race-level gate the project has had.
/// - **Moved 2026-08-11**, when `Ship::driver` joined the hash. The field is the
///   opponent driver's place on the racing line, which seeds next tick's search
///   and so is simulation state; see `oag_gameplay::hash::write_ship`. **The
///   cause was isolated the way the paragraph below requires**: with the
///   driver's own `write_u32` removed and nothing else changed, the previous
///   constants - `0x0d6b_1685_6498_ed5e` / `0xd8f1_9e9e_49a7_9460` at 60 ticks
///   and `0x7b98_740e_2313_1d4f` / `0x77b7_b259_b11d_603d` at 600 - reproduce
///   bit for bit. So the movement is the new field entering the hash and not a
///   change in what the simulation does. The scenario here has no AI in it, so
///   every driver in it is at index `0` throughout.
/// - **Moved again 2026-08-11**, when `Ship::standing` joined the hash. A craft's
///   lap, place on the circuit and finish tick decide the finishing order, so
///   they are simulation state. **Isolated the same way**: with
///   `write_standing`'s call removed and nothing else changed, the previous
///   constants - `0xe8fa_1f54_0742_7f0e` / `0xf195_097c_9824_11b0` at 60 ticks
///   and `0x8f0c_b68a_5b98_18df` / `0x2a7b_fddd_ae47_fd8d` at 600 - reproduce bit
///   for bit. The scenario here has no course in it, so every standing stays at
///   its default and what moved is the field entering the stream, not any value
///   in it.
///
/// - **Moved a third time 2026-08-11**, when `Driver::seed` and `Driver::phase`
///   joined the hash. The seed decides an opponent's whole character - which
///   part of the AI corridor it holds, how hard it commits to a corner - and the
///   phase is the tick count its drift is a function of, so both decide what it
///   steers next; see `oag_ai::Personality`. **Isolated the same way**: with
///   those two `write_u32`s removed and nothing else changed, the previous
///   constants - `0x187f_03ac_8cc2_f9fe` / `0xaa73_ebe5_d207_ccd0` at 60 ticks
///   and `0x8b79_23a3_6397_3ab7` / `0x7bb3_4940_5709_0a5d` at 600 - reproduce
///   bit for bit. The scenario here still has no AI in it, so both fields are
///   `0` throughout and what moved is two more words entering the stream, not
///   any value in it.
///
/// - **Moved a fourth time 2026-08-11**, when `Driver::place` and
///   `Driver::provocation` joined the hash. A driver notices being overtaken by
///   its place getting worse and covers its line harder for a while afterwards,
///   so both decide what it does to the craft around it; see
///   `oag_ai::Driver::stew`. **Isolated the same way**: with those two
///   `write_u32`s removed and nothing else changed, the previous constants -
///   `0x2e8d_8a4d_ab71_199e` / `0x65bc_a8c9_0bc9_07f0` at 60 ticks and
///   `0xf019_f135_fae6_d657` / `0x177a_c6df_4c46_417d` at 600 - reproduce bit
///   for bit. The scenario here still has no AI in it, so both fields are `0`
///   throughout and what moved is two more words entering the stream, not any
///   value in it.
///
/// - **Moved a fifth time 2026-08-11**, when `Driver::pilot` joined the hash.
///   It is a fingerprint of the pilot a craft is flying, and unlike
///   `Ship::handling` - which comes off the player's own disc and is the same
///   everywhere - a pilot can come out of `<config dir>/oag/pilots/` and so
///   **differs between machines by design**. Left out, two machines running
///   "the same race" with different pilot files would agree here and disagree
///   on the race, which is a gate claiming an agreement it does not have.
///   **Isolated the same way**: with that one `write_u32` removed and nothing
///   else changed, the previous constants - `0x9c26_b4b5_c0c7_43be` /
///   `0xc9d7_d405_37ab_9310` at 60 ticks and `0x6c9c_3ab9_8500_0e77` /
///   `0x5c72_e308_83b1_c29d` at 600 - reproduce bit for bit. The scenario here
///   still has no AI in it, so the field is `0` throughout and what moved is one
///   more word entering the stream; `every_driver_in_this_scenario_flies_no_pilot`
///   pins that, because if it ever stopped being true these constants would
///   quietly become machine-dependent.
///
/// - **Moved a sixth time 2026-08-11**, when `Driver::mistake` joined the hash.
///   It counts down a braking point the driver is in the middle of missing, and
///   a craft sailing through one is about to be somewhere a craft that braked is
///   not; see `oag_ai::Driver::blunder`. **Isolated the same way**: with that
///   one `write_u32` removed and nothing else changed, the previous constants -
///   `0xeff4_5f7c_6a67_fa0e` / `0xd894_4c73_75ed_4c20` at 60 ticks and
///   `0x7e38_a669_8476_79e7` / `0xd7d6_954f_c46d_066d` at 600 - reproduce bit
///   for bit. The scenario has no AI, so the field is `0` throughout.
///
/// - **Moved a seventh time 2026-08-11, and this one is unlike the six above:
///   it is a change to what the simulation *does*, not to what is hashed.**
///   Projectiles now follow the track floor instead of flying straight - see
///   `oag_weapons::projectile` and
///   `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`. This scenario
///   flies a rocket down a corridor into a wall, so its whole trajectory
///   differs. That is the intended outcome, not a defect.
///
///   **Every entry above could isolate by removing one `write_*` call; this one
///   cannot**, so it was isolated in two steps instead. The first is the one
///   that matters, because it is what would catch an accidental change riding
///   along:
///
///   1. With **both** the new `Projectile::surface` write *and* the
///      surface-following disabled, and nothing else changed, the constants
///      this commit replaces - `0x2524_032f_46c2_e75e` /
///      `0xc9ba_8f3a_d73d_2530` at 60 and `0x2655_f848_d5d1_9117` /
///      `0xd5df_df48_d023_11bd` at 600 - **reproduce bit for bit**. Nothing
///      else in this change touches the simulation.
///   2. With the field hashed but the flight still straight they read
///      `0x9c82_3577_13b6_c4d7` / `0x9bcf_b626_ad64_1387` at 60 and `0xfc3e_1605_38a7_e79e` / `0x38f8_30b3_2f75_8c5a` at 600. That is the field's own
///      contribution; the rest of the distance to the constants below is the
///      flight model, which is the point of the change.
///
/// - **Moved an eighth time 2026-08-11**, when `MAX_PROJECTILES` went from 16 to
///   128. Sixteen could not hold one simultaneous volley from a full grid (24),
///   let alone an Eliminator race; the original's own pool is 48 and this is
///   deliberately past it. **Isolated the same way the field additions were**:
///   with the constant put back to 16 and nothing else changed, the constants
///   this commit replaces - `0x8a04_20ea_d659_f08e` / `0x3005_5382_1116_2a30`
///   at 60 and `0xd934_4be0_d70e_dc17` / `0x9ca7_8a00_bbbb_29ad` at 600 -
///   reproduce bit for bit. Every slot is hashed whether or not it is occupied, so what moved
///   is 112 more empty slots entering the stream, not anything the simulation
///   does: this scenario fires one rocket and never fills a second slot.
///
/// - **Moved a ninth time 2026-08-16**, when [`oag_race::Standing`] gained a lap
///   clock - `lap_start_tick` and `best_lap_ticks`, the two fields `RaceState`
///   already carried for the player alone. Every craft on the grid times its own
///   laps now, which is what "adaptation between races" was blocked on.
///   **Isolated in the order the compiler forces**, which is the cleanest form of
///   this proof: `write_standing` destructures exhaustively, so the fields had to
///   be *named* there before the crate would build. With them named and neither
///   one written to the hasher, the constants this commit replaces -
///   `0x86c3_7eae_232b_caba` / `0x4e1f_368e_1f13_b9ec` at 60 ticks and
///   `0xbd24_86ff_da6a_56b3` / `0x7dc5_c309_e8f9_8311` at 600 - reproduce bit for
///   bit. So the movement is those two writes entering the stream and nothing
///   the simulation does differently. **The scenario here has no course in it**,
///   the same fact the 2026-08-11 entry above turns on, so both fields stay
///   `None` on every tick: what moved is one discriminant byte apiece per craft
///   per tick, in the same way the projectile-pool widening moved empty slots
///   rather than behaviour.
///
/// - **Moved 2026-08-19**, when `oag_physics::ShipState` gained
///   `pending_impulse` - see `crates/physics/tests/determinism.rs`'s own
///   history for that field, which this file inherits through
///   `hash_world`'s call into `oag_physics::probe::hash_state` (`crates/gameplay/src/hash.rs`)
///   rather than a change made here. No isolation check repeated in this file:
///   it would exercise the identical function that entry already isolated, and
///   nothing in `oag_gameplay` writes the new field either - both scenarios
///   here spawn craft through `oag_physics::ShipState` defaults and never touch
///   it. The constants this commit replaces - `0xa88c_cfc7_3a30_f316` /
///   `0x4e6e_0a8f_accb_a74c` at 60 ticks and `0xb5fc_13ea_252e_8e43` /
///   `0x92e8_8d29_f718_d061` at 600 - are exactly what `probe::hash_state`'s
///   own isolation check reproduced bit for bit with the new write removed.
///
/// - **Moved 2026-08-26**, when `oag_weapons::pickup::Held` gained `dropping`
///   and `drop_reload`, the two counters a Mine cluster comes out on. **The
///   move is the hash stream and nothing else.** Both are zero through every
///   scenario here - neither collects a pickup, so neither ever starts a drop -
///   so the addition contributes a fixed five bytes per ship per tick, exactly
///   as `autopilot_timer` did two days earlier. The isolation was run the way
///   this paragraph below requires: with only the two new writes in
///   `hash_world`'s `write_held` removed, the constants this commit replaces -
///   `0x9c5d_3b4c_b229_2baa` / `0x0d31_b55a_669c_4254` at 60 ticks and
///   `0xfc00_1ad4_d9ef_7f57` / `0x0e7b_e5cd_c260_ab11` at 600 - reproduced bit
///   for bit.
///
///   **The Mine also joined `pickup::IMPLEMENTED` in the same change**, which
///   *would* be a behaviour move if either scenario drew a pickup. Neither
///   does, which is what makes the isolation above sufficient rather than
///   merely suggestive: a run that changed both the draw pool and the hash
///   stream and then reproduced the old constants with one of them reverted has
///   shown the other one was inert.
///
/// - **Moved 2026-08-26, later, and this one is a real behaviour move** - the
///   first on this constant that is not a hash-stream addition.
///   `projectile::blast` scaled its impulse flat inside the radius, which this
///   module recorded as ours; `Weapon_PostBlastImpulse` (`0x0886794c`) scales
///   it by `1.0 - d / blastradius`, and the four `<Stats>` offsets it spends
///   are the Mine's, identified in
///   `docs/ghidra/functions/psp-pulse-usa/mine.md`. So an invented rule was
///   replaced by a recovered one and craft caught by a blast now end up
///   somewhere else. **The damage is untouched and stays flat**, which is the
///   same read's other half.
///
///   Isolated the way this paragraph requires: with `falloff` forced to `1.0`
///   and nothing else changed, the constants this commit replaces -
///   `0x3b00_aaf7_5fa6_012e` / `0x8190_a8a4_a597_de7c` at 60 ticks and
///   `0x9014_04ca_b696_5f8f` / `0xea4f_cef3_68eb_a4b9` at 600 - reproduced bit
///   for bit. [`REFERENCE_VOLLEY`] did **not** move, which is itself
///   informative: its craft never gets close enough to a detonation to be
///   pushed, so the two scenarios cover different halves of this function.
///
/// - **Moved 2026-09-05**, inherited the same way the 2026-08-19
///   `pending_impulse` entry above is: `oag_physics::ShipState` gained four
///   fields for the barrel roll's gesture and phase, see
///   `crates/physics/tests/determinism.rs`'s own history for that field set.
///   **The move is the hash stream and nothing else** - neither scenario here
///   taps out a roll, so all four fields hold their defaults on every tick of
///   both. No isolation repeated in this file for the same reason the
///   inherited entry gives. The constants this commit replaces -
///   `0xde9b_58b2_91cb_0dfb` / `0xc5fb_0d05_0b56_bb85` at 60 ticks and
///   `0x91fe_78d0_5a4b_01aa` / `0x08a9_2c99_ab0f_f54c` at 600 - are what
///   [`REFERENCE`] held before this move.
///
/// - **Moved 2026-09-05, later the same day**, inherited the same way: a fifth
///   barrel-roll field, `roll_payout_timer`, plus its three consumer branches
///   in `crate::airbrake`/`crate::hover`/`crate::engine` and the
///   landing-transition wiring in `crate::forces::evaluate` - see
///   `crates/physics/tests/determinism.rs`'s own history for that step.
///   **Still the hash stream and nothing else**: `roll_payout_timer` can never
///   leave `0.0` here either, so every new consumer branch's `else` arm is
///   byte-for-byte what ran before it existed. Replaces
///   `0x2e8b_8631_f20f_99d1` / `0xfb92_8769_d310_f9eb` at 60 ticks and
///   `0xf9b1_492d_6749_a348` / `0x46e3_c9f5_b867_ef0a` at 600.
///
/// - **Moved 2026-09-06**, when the barrel roll became reachable:
///   `oag_physics::ShipControls` gained the d-pad tap edges, this crate's
///   `controls::ship_controls` fills them, and `oag_physics::ShipState` gained
///   `roll_axis_zone` - the latch that makes a steering-axis *crossing*
///   distinguishable from a held axis. Inherited through the same
///   `hash_world` call, and isolated in
///   `crates/physics/tests/determinism.rs` rather than twice.
///
///   **Reachable is not reached, here.** No craft in this scenario is flown at
///   all - `run` steps a world whose drivers never run, and the entry above
///   already records that every reflex channel reads `Reflex::IDLE`
///   throughout - so `ShipControls::steer_x` is `0.0` on every tick and no
///   crossing is ever recorded. The whole movement is the one extra `u8` per
///   craft per tick that `roll_axis_zone` adds to the stream; with that single
///   write removed and the gesture left in place, the constants from
///   2026-09-05 reproduce bit for bit. Replaces `0xc2cd_afd3_1c1f_7e01` /
///   `0x4bf6_9ad5_c266_27db` at 60 ticks and `0x7713_5775_9662_2238` /
///   `0xe413_77c8_89ff_33ba` at 600.
///
/// - **Moved again 2026-09-06, by a merge rather than by one change.** The AI
///   barrel-roll axes and the weapon-slowdown port each moved these constants on
///   their own branch, and neither branch's value survives their merge: the
///   merged tree writes both `Pilot`'s three roll axes and `World`'s
///   `pending_slowdown` into the same stream. The value recorded here is
///   **measured from the merged tree**, not chosen from either side - both
///   causes are already isolated and explained in their own entries above, so
///   what is new here is only their composition.
///
/// - **Moved again 2026-09-07, for the Cannon's two new `Held` fields.**
///   `cannon_rounds`/`cannon_reload` are hashed unconditionally on every
///   craft, every tick - the same shape `dropping`/`drop_reload` already
///   are - so the digest stream differs from tick 0 whether or not a Cannon
///   is ever held, exactly as it did when those two landed. **Isolated the
///   way this comment requires**: with `write_held`'s two new `write_*`
///   calls removed, both rows reproduce the previous constants bit for bit -
///   this scenario never grants a Cannon, so the movement is the hash
///   primitive's own shape, not a change to what this scenario's craft do.
///   See `crates/gameplay/src/hash.rs`'s `write_held`.
///
/// - **Moved again 2026-09-07, for the Quake's new `World::quake` field.**
///   `hash::write_quake` writes a discriminant byte for `Option<Wave>` every
///   tick whether or not a Quake is in flight, the same "always in the
///   stream" shape the Cannon's two `Held` fields added above. **Isolated the
///   way this comment requires**: with `write_world`'s `write_quake` call
///   removed (and `quake` destructured as `quake: _`), both rows reproduced
///   the previous constants - `0x8bbe_7452_37aa_a9d8` /
///   `0x71ae_5fa3_1894_6b8a` at 60 ticks and `0xf25a_43fe_dd1b_5219` /
///   `0xde90_bd8b_7c5a_4d17` at 600 - bit for bit. Neither scenario in this
///   file ever launches a Quake, so the movement is the hash primitive's own
///   shape and not a behaviour change to what either scenario's craft do.
///   See `crates/gameplay/src/hash.rs`'s `write_quake`.
///
/// - **Moved again 2026-09-07, for `RaceState::lap_splits` and
///   `Standing::lap_splits`.** Both are four `Option<u32>` slots hashed
///   unconditionally per craft per tick and on the race itself, the per-lap
///   split history `crates/race/src/state.rs`'s `MAX_RECORDED_LAPS` adds so
///   HD's `Lap1Image`-`Lap4Image` have something to draw. **Isolated the way
///   this comment requires**: with both new `for split in lap_splits` loops
///   removed (and `lap_splits` destructured as `lap_splits: _` in
///   `write_standing` and `write_race`), both rows reproduced the previous
///   constants - `0x1f49_7fb4_96fe_9a08` / `0x08cd_936e_e73d_b4d8` at 60 ticks
///   and `0xeab3_a311_b56c_807b` / `0xcebe_2c5c_6232_1c5d` at 600 - bit for
///   bit. This scenario sets `world.race[0].lap` directly at tick 20 rather than
///   through `RaceState::complete_lap`/`Standing::update`, so no slot in
///   either array is ever written to anything but `None` here; the movement
///   is the hash primitive's own shape, not a behaviour change to what this
///   scenario's craft do. See `crates/gameplay/src/hash.rs`'s `write_standing`
///   and `write_race`.
///
/// **Never edit these to make the test pass**, the same rule
/// `crates/physics/tests/determinism.rs` states at length: a movement here is a
/// change to what a race *does*, and the change is the thing to find. When a
/// movement is legitimate - a new field on `World`, say - record why it moved
/// beneath this comment and isolate the cause first, by removing the new field's
/// own write and checking that the previous constants reproduce bit for bit.
/// - **Moved 2026-09-08**, and it is the hash stream and nothing else.
///   [`oag_gameplay::World`] gained `leach_beam: Option<Beam>` for the
///   LeachBeam, so `hash::write_world` writes one extra discriminant byte per
///   tick. Neither scenario here fires a LeachBeam - neither collects a pickup
///   at all - so the field is `None` on every tick of both and that byte is a
///   constant `0`; nothing else about either run changed.
///
///   **Isolated the way this comment requires.** With
///   `write_leach_beam(hasher, leach_beam)` removed from `write_world` and
///   nothing else touched, the constants this commit replaces -
///   `0x2321_c766_223d_0fc8` / `0x3917_5149_7ee8_d198` at 60 ticks and
///   `0xb9c2_9ac6_b8fe_010b` / `0x34c7_a5fa_88c8_357d` at 600 - reproduced bit
///   for bit, and [`REFERENCE_VOLLEY`]'s did too. That is what makes this an
///   addition to what the gate covers rather than a change to what a race does.
/// - **Moved again 2026-09-08, same day, for [`oag_race::Mode::Eliminator`].**
///   `oag_race::Standing` gained `kills: u32` and `deaths: u32` - Eliminator's
///   own bookkeeping - so `hash::write_standing` writes eight more bytes per
///   ship per tick. Neither scenario here runs Eliminator or writes to either
///   field, so both stay a constant `0` through the whole run; nothing else
///   about either scenario changed. The mode enum also gained a fifth
///   discriminant (`Mode::Eliminator => 4` in `hash::write_race`), which
///   changes nothing for the four pre-existing modes' own encoding - neither
///   scenario's `world.race[0].mode` is ever `Eliminator`.
///
///   **Isolated the way this comment requires.** With `write_u32(*kills)` and
///   `write_u32(*deaths)` removed from `write_standing` and nothing else
///   touched, the constants this commit replaces - `0x2321_c766_223d_0fc8` /
///   `0x3917_5149_7ee8_d198` at 60 ticks and `0xb9c2_9ac6_b8fe_010b` /
///   `0x34c7_a5fa_88c8_357d` at 600 - reproduced bit for bit, and
///   [`REFERENCE_VOLLEY`]'s did too.
///
/// - **Moved 2026-09-09**, for the Plasma's recovered wind-up.
///   `projectile::Projectile` gained `charge: f32` - the seconds a bolt is
///   held on the firing craft's nose before it flies, from `Plasma_Init`'s
///   `+0x50` - so `hash::write_projectile` writes four more bytes per slot per
///   tick. Neither scenario here fires a Plasma; both fire Rockets, whose
///   `charge` is a constant `0.0` for every slot on every tick of both runs.
///   Nothing about either scenario's flight, blast or inventory changed.
///
///   **Isolated the way this comment requires.** With `write_f32(*charge)`
///   removed from `write_projectile` and nothing else touched - the whole
///   charging branch in `Projectiles::advance` still in place, both Plasma
///   call sites still on `charge_up` - the constants this commit replaces
///   reproduced bit for bit at both tick counts and for both scenarios.
///
/// - **Moved 2026-09-11**, when `oag_ai::Driver::peak_curvature` joined the
///   hash - the differential airbrake's new gate needs it, see
///   `crates/ai/tests/determinism.rs`'s own history for the real-track
///   measurement that added it. **Isolated the documented way**: with
///   `peak_curvature` destructured as `peak_curvature: _` and its
///   `write_u32` removed, and nothing else changed, the previous constants
///   above reproduced bit for bit at both tick counts. So the whole movement
///   is one more `u32` per ship per tick entering the stream, and no
///   behaviour at all: **neither scenario here ever calls `Driver::drive`**
///   either, for the same reason the `roll_decided` and `reflex` entries
///   above give. Replaces `0xf3aa_6cce_9139_99e8` / `0x6f71_5c3a_c11d_3de2`
///   at 60 ticks and `0x9290_5461_4434_9d81` / `0xd773_b51b_1a49_815f` at 600.
///
/// - **Moved 2026-09-15**, when Pure's Disruptor landed: `Ship::disruption`
///   (a kind byte and an `f32` timer, `oag_weapons::disruption`) and
///   `Projectile::effect` (a kind byte) entered the stream - five more bytes
///   per ship and one more per projectile slot per tick. **Isolated the
///   documented way**: with `write_disruption` and the projectile's
///   `write_effect_kind` replaced by `let _ =` and nothing else changed, the
///   previous constants reproduced bit for bit at both tick counts for both
///   scenarios. Neither scenario fires a Disruptor or is hit by one, so every
///   new byte is `0`. Replaces `0xd98f_5ee7_45c1_2fd8` /
///   `0x59cb_0602_21c2_ccf2` at 60 ticks and `0xc88f_9c58_aa40_a211` /
///   `0x02af_2239_1229_462f` at 600.
///
/// - **Moved 2026-09-16**, when `World::race` widened from one `RaceState` to
///   `[RaceState; MAX_PLAYERS]` and `World::controllers` joined it - the shared
///   prerequisite split screen, multiple windows and network play all sit on.
///   Seven more `RaceState`s and eight `Controller` discriminant bytes per tick
///   entered the stream. **Isolated the documented way**: with `write_race`
///   called on `race[0]` alone and `controllers` taken as `let _ =`, and
///   nothing else changed, the previous constants reproduced bit for bit at
///   both tick counts for both scenarios. Neither scenario has a second timed
///   racer or a second human, so slots 1..7 hold `RaceState::default()` for
///   every tick of both and every controller byte is `0` but slot 0's `1`.
///   Replaces `0xa31c_f93f_c44f_7004` / `0xe197_c1d8_e0b5_d07e` at 60 ticks and
///   `0xbe25_d8d5_f406_e799` / `0xc8b7_8f68_8555_be83` at 600.
///
/// - **Moved 2026-10-03**, when `Ship::weapon_ai` (two `u32` clocks for the
///   original's opponent fire law, `oag_ai::weapon_ai`) joined the hash: eight
///   more bytes per ship per tick. Neither scenario here steps an opponent, so
///   every clock stays `0`. **Isolated the documented way**: with the two
///   writes taken out of `write_ship` and nothing else changed, the previous
///   constants reproduced bit for bit at both tick counts for both scenarios.
///   Replaces `0x57a6_3ed9_c3d1_ede7` / `0x5706_b5b3_ef8f_3411` at 60 ticks and
///   `0xf313_687a_6471_7f52` / `0xf931_ba11_7aca_8358` at 600.
///
/// - **Moved 2026-10-04**, when `World::repulsers` (sixteen empty Repulser
///   slots, one discriminant byte each) joined the hash. No Repulser is fired
///   here. Isolated the documented way: with the pool's writes taken out of
///   `write_world` and nothing else changed, the previous constants reproduced
///   bit for bit at both tick counts for both scenarios. Replaces `0x5879_0798_3068_5567` /
///   `0x2c97_d702_c8e1_58b1` at 60 ticks and `0xac74_cd4a_cb3b_9dd2` / `0x11eb_c0b9_fd45_0178` at 600.
///
/// - **Moved 2026-10-05**, when `Driver::branching` (which line a driver is on
///   at a fork, `oag_ai::branch`) joined the hash: thirteen more bytes per ship
///   per tick. The synthetic track here has no fork, so every field stays at its
///   default. Isolated the documented way: with the four writes taken out of
///   `write_driver` and nothing else changed, the previous constants reproduced
///   bit for bit at both tick counts. Replaces `0x393b_148b_b8f2_5727` /
///   `0x7f62_1ea3_430c_a071` at 60 ticks and `0x5e00_c1b5_b928_c252` / `0x7838_5bc9_954f_c738` at 600.
///
/// - **Moved 2026-10-05 again**, when `Branching::pending` (the route a driver
///   drew, held until it is near the split) joined the driver's hash: four
///   more bytes per ship per tick, zero on this forkless track. Isolated the
///   same way: with that one write removed the previous constants reproduced
///   bit for bit. Replaces `0x6f3a_9672_4942_4caf` / `0x6423_786d_5305_88bb` at 60 ticks and `0xdbb7_da22_3b37_4562` /
///   `0x46e0_cbc8_9e6a_ea96` at 600.
const REFERENCE: &[(u32, u64, u64)] = &[
    (60, 0xeaa8_47a2_289e_87cf, 0x4134_e0d4_2589_124b),
    (600, 0x2f1c_7e70_2e92_c262, 0x9191_035b_9d0d_e5a6),
];

#[test]
fn the_race_state_matches_the_committed_reference() {
    let mut failures = Vec::new();
    for &(ticks, expected_final, expected_trajectory) in REFERENCE {
        let (final_hash, trajectory_hash) = run(ticks);
        if final_hash != expected_final || trajectory_hash != expected_trajectory {
            failures.push(format!(
                "ticks={ticks}\n  final:      expected {expected_final:#018x}, got \
                 {final_hash:#018x}\n  trajectory: expected {expected_trajectory:#018x}, got \
                 {trajectory_hash:#018x}"
            ));
        }
    }
    assert!(
        failures.is_empty(),
        "the race state is not reproducible on this platform ({} / {}):\n\n{}\n\nDo not update \
         the constants to silence this. See the module docs.",
        std::env::consts::OS,
        std::env::consts::ARCH,
        failures.join("\n\n")
    );
}

/// The run has to actually exercise what the module docs claim it does, or the
/// constants above pin an empty scenario. The same guard
/// `oag_physics`'s `the_run_visits_the_paths_it_claims_to_cover` is.
#[test]
fn the_run_visits_the_paths_it_claims_to_cover() {
    let world_geometry = corridor();
    let stats = weapon_stats();

    let mut world = World::new(0xC0FFEE);
    world.ship_count = 2;
    for (slot, position) in [Vec3::ZERO, Vec3::new(0.0, -14.0, 380.0)]
        .into_iter()
        .enumerate()
    {
        let ship = &mut world.ships[slot];
        ship.active = true;
        ship.handling.dimensions = Dimensions {
            length: 4.0,
            width: 2.0,
            height: 1.0,
            shield: 100.0,
            ..Dimensions::default()
        };
        ship.physics.shield = 100.0;
        ship.physics.body.mass = 1.0;
        ship.physics.body.position = position;
    }
    world
        .projectiles
        .spawn(Weapon::Rocket, Vec3::ZERO, Vec3::Z * 500.0, 0);

    let mut impacts = 0;
    let mut flew = false;
    for _ in 0..600 {
        let reported = projectile::step(
            &mut world.projectiles,
            &mut world.ships[..world.ship_count as usize],
            TICK,
            &world_geometry,
            Some(&stats),
            "VENOM",
            DamageRules::default(),
            &mut [],
        );
        impacts += reported.iter().flatten().count();
        if world.projectiles.live() > 0 && world.projectiles.slots[0].position.z > 0.0 {
            flew = true;
        }
    }

    assert!(flew, "the rocket never moved, so flight is not covered");
    assert_eq!(impacts, 1, "the rocket never struck the craft");
    assert!(
        world.ships[1].physics.shield < 100.0,
        "the blast reached nobody, so the damage path is not covered"
    );
    assert!(
        world.ships[1].physics.body.linear_velocity.length() > 0.0,
        "the blast pushed nobody, so the impulse path is not covered"
    );

    // **The committed constants are only machine-independent while this holds.**
    // `Driver::pilot` is a digest of a pilot that can come out of the player's
    // own config directory; it is `0` here because this scenario has no AI in
    // it. An edit that gave it one would make the references above depend on
    // whatever is in `~/.config/oag/pilots/`, which is precisely the failure
    // that field was added to make loud rather than silent.
    for ship in &world.ships {
        assert_eq!(
            ship.driver.pilot, 0,
            "a craft here is flying a pilot, so the committed constants now \
             depend on the machine's config directory"
        );
    }
}

/// A second run of the same scenario reproduces the first, bit for bit. Catches
/// a dependency on address order, or on a hasher that carries state between
/// runs, which committed constants alone would not: both runs would be wrong the
/// same way only if the cause were deterministic.
#[test]
fn the_race_state_is_stable_across_repeated_runs() {
    assert_eq!(run(300), run(300));
}
