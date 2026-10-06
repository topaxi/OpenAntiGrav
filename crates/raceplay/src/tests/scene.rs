//! What the frame is built out of: the uniform block, the per-craft
//! drawables, the exhaust and ribbon trails, and the projectile and blast
//! billboards.
//!
//! Split out of `race.rs`'s inline `#[cfg(test)] mod tests` under the 200-line
//! rule in `scripts/check-file-size.py`. Shared fixtures live in the parent
//! `tests.rs`. The camera half of the same seam is in `camera.rs`.

use super::*;
use oag_gameplay::PlayerInputs;

/// The blink light keeps the behaviour it had under the old global
/// 120-tick clock and its per-vertex rate: one full sweep of the palette
/// per 60 ticks, backwards through the rows.
///
/// The block here is the one every team's glow mesh authors, read off the
/// disc and recorded in `texture-animation.md` - offset `v` from 0 to
/// -256 over key times 1..60, constant scale, loop period 1.0 s. It is
/// worth pinning synthetically because this is the **one** animation on
/// the disc confirmed against a frame-accurate capture of the original, so
/// it is what fixes the sign, the 1/256 units and the tick-to-seconds
/// conversion for every other surface.
#[test]
fn the_blink_light_still_sweeps_once_every_sixty_ticks() {
    let blink = vex::TexTransform {
        offset: vex::TexTransformTrack {
            times: vec![1, 60],
            values: vec![(0, 0), (0, -256)],
        },
        scale: vex::TexTransformTrack {
            times: vec![60],
            values: vec![(256, 256)],
        },
        seconds_per_key: 1.0 / 60.0,
        loop_seconds: 1.0,
        step: false,
    };
    // The same conversion `Scene::draw` uses.
    let at = |tick: u64| blink.sample(tick as f32 / 60.0);
    assert_eq!(at(0).1, [0.0, 0.0], "clamped to the first key");
    // Negative, and that is the authored direction: the palette's white
    // column rises fast and fades slow, so playing it forwards reads as
    // backwards. See `docs/formats/vex.md`.
    let (scale, half) = at(30);
    assert_eq!(scale, [1.0, 1.0]);
    assert!((half[1] + 29.0 / 59.0).abs() < 1e-6, "{half:?}");
    // One whole sweep, then the wrap puts it back where it started rather
    // than sliding on into a second tile.
    assert_eq!(at(60).1, at(0).1);
    assert_eq!(at(90).1, at(30).1);
}

/// The uniform block is `oag-render`'s, not ours. If that side grows a field
/// this catches it before the picture goes quietly wrong.
#[test]
fn the_uniform_block_matches_the_shaders() {
    assert_eq!(
        std::mem::size_of::<Uniforms>() as u64,
        mesh_render::UNIFORMS_SIZE
    );
}

/// One craft's exhaust follows *its* throttle, not the field's.
///
/// The discriminator is the fixture's own asymmetry: the player holds no
/// button, so `ShipControls::thrust` is zero and slot 0's engine is off, while
/// every opponent's driver holds full throttle down the straight. A shared
/// `Exhaust` would show one intensity for all eight; eight of them show the
/// player's falling while the field's climbs.
#[test]
fn each_craft_s_exhaust_follows_its_own_throttle() {
    let mut race = race_with_a_grid();
    // Past the start-line countdown first - see `oag_race::RaceState::thrust_gated`
    // - which now holds every opponent's throttle at zero exactly like the
    // player's, so the field would show no engine at all if this test ran
    // inside the gated span.
    for _ in 0..=oag_race::COUNTDOWN_TICKS {
        race.tick(&PlayerInputs::none());
    }
    // Long enough for `INTENSITY_RISE` to separate a burning engine from a
    // cold one - the ramp takes four seconds end to end, so a quarter of a
    // second is plenty to order the two and far short of saturating.
    for _ in 0..15 {
        race.tick(&PlayerInputs::none());
    }

    assert!(
        !race.exhaust_of(0).engine_on(),
        "the player holds no throttle, so slot 0's engine must be off"
    );
    for slot in 1..8 {
        assert!(
            race.exhaust_of(slot).engine_on(),
            "slot {slot}'s driver holds throttle, so its engine must be on"
        );
        assert!(
            race.exhaust_of(slot).intensity() > race.exhaust_of(0).intensity(),
            "slot {slot} is burning and the player is not, but its intensity \
             ({}) is not above the player's ({})",
            race.exhaust_of(slot).intensity(),
            race.exhaust_of(0).intensity()
        );
    }
}

/// Every craft lays its own ribbon, from its own nozzle.
///
/// One shared trail ring fed from eight poses would zig-zag between the craft
/// once a tick, which is why this asserts the *newest sample* of each craft's
/// ribbon is at that craft's own nozzle rather than only that eight rings are
/// full.
#[test]
fn every_craft_lays_its_own_ribbon() {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    setup.start_position = Some(oag_vex::track::StartPosition {
        position: [0.0, 0.0, 0.0],
        left: [0.0, 0.0, -1.0],
        up: [0.0, 1.0, 0.0],
        forward: [1.0, 0.0, 0.0],
    });
    // The synthetic setup authors no ship model and so no locator, and with
    // no nozzle nothing is pushed at all - see `setup`. Invented, and behind
    // the craft's origin along model `-z`, which is where an engine sits;
    // what the test needs is only that the eight are far enough apart to tell
    // one craft's ribbon from another's, and the grid's own 19.79-unit step
    // is 25 times this.
    setup.nozzles = vec![Some(Vec3::new(0.0, 0.0, -0.8)); MAX_SHIPS];
    let mut race = Race::start(setup);
    // `Exhaust::trail_ready` gates the ribbon on a full ring, which is the
    // original's own gate, so this needs one tick per sample.
    for _ in 0..oag_fx::exhaust::TRAIL_SAMPLES {
        race.tick(&PlayerInputs::none());
    }

    let nozzles: Vec<Vec3> = (0..8)
        .map(|slot| {
            race.nozzle_of(slot)
                .expect("the fixture's hull authors a nozzle")
        })
        .collect();
    for slot in 0..8 {
        assert!(
            race.exhaust_of(slot).trail_ready(),
            "slot {slot}'s ring never filled"
        );
        // The head of the ribbon: `trail_vertices` builds every layer from
        // the newest sample first, so the first vertex sits on the rim of a
        // cross centred on this craft's own nozzle.
        let head = race.exhaust_of(slot).trail_vertices(Vec3::X, Vec3::Y)[0].position;
        let head = Vec3::from_array(head);
        let nearest = (0..8)
            .min_by(|a, b| {
                let (a, b) = (nozzles[*a], nozzles[*b]);
                head.distance(a).total_cmp(&head.distance(b))
            })
            .expect("eight craft");
        assert_eq!(
            nearest, slot,
            "slot {slot}'s ribbon starts nearest slot {nearest}'s nozzle"
        );
    }
}

/// The player's flicker does not change when the field grows.
///
/// This is the seeding decision under test, and it is worth a test rather
/// than a comment: one shared generator drawn from once per craft would make
/// slot 0's flicker depend on how many opponents the mode fields, so every
/// exhaust number pinned against a single-craft capture would move the day
/// a grid appeared behind it. See [`exhaust_seed`].
#[test]
fn the_players_flicker_does_not_depend_on_the_field_behind_it() {
    let mut alone = race_with_pads(Mode::TimeTrial, Vec::new());
    let mut field = race_with_a_grid();
    assert_eq!(alone.ship_count(), 1);
    assert_eq!(field.ship_count(), 8);

    for tick in 0..30 {
        alone.tick(&PlayerInputs::none());
        field.tick(&PlayerInputs::none());
        // Bit-identical, not close: both draw the same two numbers from a
        // generator seeded the same way, against the same thrust and the same
        // standing-start speed.
        assert_eq!(
            alone.exhaust().half_size(),
            field.exhaust().half_size(),
            "tick {tick}: the flare's size moved when opponents were added"
        );
        assert_eq!(
            alone.exhaust().alpha(),
            field.exhaust().alpha(),
            "tick {tick}: the flare's alpha moved when opponents were added"
        );
    }
}

/// Eight craft do not flicker in lockstep.
///
/// The other half of the seeding decision: distinct streams. Eight craft
/// sharing one seed would pulse together, which reads as a single effect
/// rather than eight engines.
#[test]
fn the_field_does_not_flicker_in_lockstep() {
    let mut race = race_with_a_grid();
    race.tick(&PlayerInputs::none());

    let sizes: Vec<f32> = (0..8)
        .map(|slot| race.exhaust_of(slot).half_size())
        .collect();
    for slot in 1..8 {
        assert_ne!(
            sizes[slot], sizes[0],
            "slot {slot}'s flare is exactly the player's size - the two are \
             drawing from the same stream"
        );
    }
}

/// A rocket whose model did not load is drawn as a billboard, and the whole
/// worst case fits the buffer it is drawn into.
///
/// **The second half is the regression.** `oag_fx::exhaust::Pipeline::upload`
/// clamps with `min`, and `MAX_VERTICES` was six - the flare's one quad -
/// so the projectile sprites were dropped on the floor with nothing in the
/// logs and nothing on screen. A budget test is the only thing that catches
/// a silent truncation, because every other symptom is "it does not appear".
///
/// The buffer shrank on 2026-08-12 rather than grew: the smoke trail and
/// the blast puffs that used to share it were inventions, and both now come
/// out of the disc's own `.pob` files through the particle pipeline. What
/// is left here is one quad per projectile, on the fallback path only,
/// plus one flare per craft.
#[test]
fn every_projectile_is_drawn_and_the_worst_case_fits_the_buffer() {
    let mut race =
        race_with_weapon_table(Mode::SingleRace, enveloping_pad(), 1.0, one_rocket_table());
    let right = Vec3::X;
    let up = Vec3::Y;
    assert!(
        race.projectile_sprites(right, up, |_| false).is_empty(),
        "an empty sky must draw nothing rather than a quad at the origin"
    );

    // The worst case: every slot in the air at once.
    for slot in 0..oag_weapons::projectile::MAX_PROJECTILES {
        race.sim.world.projectiles.spawn(
            oag_tables::weapons::Weapon::Rocket,
            Vec3::Z * slot as f32,
            Vec3::Z,
            0,
        );
    }
    let vertices = race.projectile_sprites(right, up, |_| false);
    assert_eq!(
        vertices.len(),
        oag_weapons::projectile::MAX_PROJECTILES * 6,
        "six vertices per projectile, and nothing else in this buffer"
    );
    // A modelled rocket's glow comes off the disc instead, so this buffer
    // gets nothing at all from it.
    assert!(
        race.projectile_sprites(right, up, |_| true).is_empty(),
        "a modelled rocket must not also get an invented billboard"
    );
    // Every craft's flare shares this buffer with the projectiles, so the
    // worst case is a full grid of flares *plus* a full sky of rockets.
    let flares = MAX_SHIPS * 6;
    assert!(
        vertices.len() + flares <= exhaust::MAX_VERTICES,
        "the worst case is {} vertices against a buffer of {} - `upload` would \
         silently drop the overflow",
        vertices.len() + flares,
        exhaust::MAX_VERTICES
    );
}

/// A rocket drawn as a model is oriented along its own velocity.
///
/// The forward column is the evidenced half of `Rocket_Update`'s basis (see
/// `docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`); the roll about
/// it is not, which is why only forward and orthonormality are asserted.
#[test]
fn a_modelled_rocket_points_where_it_is_going() {
    let mut race =
        race_with_weapon_table(Mode::SingleRace, enveloping_pad(), 1.0, one_rocket_table());
    assert!(
        race.rocket_model_matrices().is_empty(),
        "an empty sky must place no rocket models"
    );

    let heading = Vec3::new(1.0, 0.0, 2.0).normalize();
    race.sim.world.projectiles.spawn(
        oag_tables::weapons::Weapon::Rocket,
        Vec3::X,
        heading * 600.0,
        0,
    );
    let matrices = race.rocket_model_matrices();
    assert_eq!(matrices.len(), 1, "one live rocket, one matrix");

    let matrix = matrices[0];
    assert!(
        matrix.w_axis.truncate().abs_diff_eq(Vec3::X, 1e-5),
        "the model sits where the rocket is, got {:?}",
        matrix.w_axis
    );
    assert!(
        matrix.z_axis.truncate().abs_diff_eq(heading, 1e-5),
        "forward must be the velocity, got {:?} against {heading:?}",
        matrix.z_axis
    );
    // A degenerate basis would still translate correctly and would light the
    // model from nowhere, so orthonormality is worth its own assertion.
    let (x, y, z) = (
        matrix.x_axis.truncate(),
        matrix.y_axis.truncate(),
        matrix.z_axis.truncate(),
    );
    for (label, axis) in [("side", x), ("up", y), ("forward", z)] {
        assert!(
            (axis.length() - 1.0).abs() < 1e-5,
            "{label} is not unit length: {}",
            axis.length()
        );
    }
    assert!(x.dot(y).abs() < 1e-5 && x.dot(z).abs() < 1e-5 && y.dot(z).abs() < 1e-5);
    // A rotation, not a reflection, and the original's own row 0: side is
    // `up x forward` (`Rocket_Update`'s `vcrsp.t` at `0x0885d988`, with world
    // up standing in for the track normal). The reflection this used to be
    // passed every assertion above and drew the dart mirrored.
    assert!(
        (matrix.determinant() - 1.0).abs() < 1e-5,
        "the placement must be a rotation, got determinant {}",
        matrix.determinant()
    );
    assert!(
        x.abs_diff_eq(Vec3::Y.cross(heading).normalize(), 1e-5),
        "side must be up x forward, got {x:?}"
    );
}

/// A rocket and a missile in flight at once do not share a model slot.
///
/// **The regression test for the bug [`Race::rocket_model_matrices`] used to
/// carry**: before 2026-09-05 it filtered `kind.is_some()` rather than
/// `kind == Some(Weapon::Rocket)`, so a live projectile of *any* kind was
/// handed a Rocket matrix and drawn as the Rocket's own mesh. Every other
/// test here spawns a Rocket alone, which is exactly why none of them caught
/// it - the filter and the "any kind" version agree whenever there is only
/// one kind in the air.
#[test]
fn a_rocket_and_a_missile_in_flight_do_not_share_a_model_slot() {
    let mut race =
        race_with_weapon_table(Mode::SingleRace, enveloping_pad(), 1.0, one_rocket_table());
    race.sim.world.projectiles.spawn(
        oag_tables::weapons::Weapon::Rocket,
        Vec3::X,
        Vec3::Z * 600.0,
        0,
    );
    race.sim.world.projectiles.spawn(
        oag_tables::weapons::Weapon::Missile,
        Vec3::Y,
        Vec3::Z * 600.0,
        0,
    );
    assert_eq!(
        race.rocket_model_matrices().len(),
        1,
        "one live rocket, and the missile alongside it must not count"
    );
    assert_eq!(
        race.mine_model_matrices().len(),
        0,
        "neither a rocket nor a missile is a mine"
    );
}

/// A laid mine or bomb is drawn with the pose it landed in, not a bare
/// translation - the fix over what this asserted before 2026-09-07. See
/// [`Race::mine_model_matrices`]'s own doc comment and
/// `oag_weapons::projectile::mine::frozen_pose` for what `Mine_Init` carries
/// and which half of the reading is chosen rather than measured.
///
/// Goes through [`oag_weapons::projectile::Projectiles::lay`] rather than
/// `Projectiles::spawn` on purpose: `spawn` is the flying weapons' own entry
/// point and defaults `orientation` to identity, which would pass this test
/// whether the drawing code used the field at all - `lay` is the one entry
/// point a real drop calls, with a non-identity pose to tell a bug in the
/// plumbing apart from a coincidence.
#[test]
fn a_laid_mine_or_bomb_keeps_the_pose_it_landed_in() {
    let mut race =
        race_with_weapon_table(Mode::SingleRace, enveloping_pad(), 1.0, one_rocket_table());
    let pose = Quat::from_rotation_y(std::f32::consts::FRAC_PI_2);
    race.sim
        .world
        .projectiles
        .lay(oag_tables::weapons::Weapon::Mine, Vec3::X, 0, 5.0, pose);
    race.sim
        .world
        .projectiles
        .lay(oag_tables::weapons::Weapon::Bomb, Vec3::Y, 0, 20.0, pose);

    let mine = race.mine_model_matrices();
    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0], Mat4::from_rotation_translation(pose, Vec3::X));

    let bomb = race.bomb_model_matrices();
    assert_eq!(bomb.len(), 1);
    assert_eq!(bomb[0], Mat4::from_rotation_translation(pose, Vec3::Y));
}

/// A rocket fired straight down still gets a usable basis.
///
/// World up is the reference the side vector is built from, so a rocket
/// flying along it is exactly the degenerate case. Ours, not the original's -
/// it builds its second vector from the track normal, which a projectile in
/// this engine does not carry.
#[test]
fn a_rocket_flying_along_world_up_does_not_collapse_its_basis() {
    let mut race =
        race_with_weapon_table(Mode::SingleRace, enveloping_pad(), 1.0, one_rocket_table());
    race.sim.world.projectiles.spawn(
        oag_tables::weapons::Weapon::Rocket,
        Vec3::ZERO,
        Vec3::NEG_Y * 600.0,
        0,
    );
    let matrix = race.rocket_model_matrices()[0];
    let (x, y, z) = (
        matrix.x_axis.truncate(),
        matrix.y_axis.truncate(),
        matrix.z_axis.truncate(),
    );
    assert!(
        z.abs_diff_eq(Vec3::NEG_Y, 1e-5),
        "forward must still be the velocity, got {z:?}"
    );
    assert!(
        (x.length() - 1.0).abs() < 1e-5 && (y.length() - 1.0).abs() < 1e-5,
        "the fallback reference must still produce unit axes, got {} and {}",
        x.length(),
        y.length()
    );
    assert!(x.dot(z).abs() < 1e-5 && y.dot(z).abs() < 1e-5);
    assert!(
        (matrix.determinant() - 1.0).abs() < 1e-5,
        "the fallback basis must still be a rotation, got determinant {}",
        matrix.determinant()
    );
}

/// A hull hit and a track hit are different blasts, in different places.
///
/// **Both halves are the original's.** It authors `WO_ROCKET_EXPLO` and
/// `WO_ROCKET_EXPLO_TRACK` separately, and `Rocket_HitCraft`
/// (`0x0886ebdc`) puts the hull one at the *struck craft's* position dropped
/// by [`CRAFT_BLAST_DROP`] rather than at the impact point. This engine drew
/// one invented flash at the impact point for both until 2026-08-12.
///
/// Asserted through [`Race::blast_for`] rather than through the pool,
/// because *which file and where* is the recovered fact and it holds on a
/// headless fixture with no disc to load either file from.
#[test]
fn a_hull_blast_sits_under_the_craft_and_a_track_blast_where_it_struck() {
    use oag_tables::weapons::Weapon;

    let mut race =
        race_with_weapon_table(Mode::SingleRace, enveloping_pad(), 1.0, one_rocket_table());
    let impact = Vec3::new(5.0, 1.0, -3.0);

    assert_eq!(
        race.blast_for(Weapon::Rocket, impact, None),
        Some((Trigger::TrackBlast, impact)),
        "no craft struck means the track effect, where the rocket struck"
    );

    let struck = 1;
    race.sim.world.ships[struck].active = true;
    let craft = Vec3::new(-20.0, 4.0, 11.0);
    race.sim.world.ships[struck].physics.body.position = craft;
    assert_eq!(
        race.blast_for(Weapon::Rocket, impact, Some(struck)),
        Some((Trigger::CraftBlast, craft - Vec3::Y * CRAFT_BLAST_DROP)),
        "a hull blast is its own effect, drawn under the craft"
    );

    // Two separately authored files, which is the whole point of the
    // split - one file at two sizes would be this engine's invention.
    assert_ne!(Trigger::TrackBlast, Trigger::CraftBlast);
}

/// A Missile, a Mine and a Bomb each play their own single explosion file,
/// whatever they hit - the Bomb's own smoke ring, through this same lookup;
/// its other two pieces are `ignite_blast`'s own Bomb-only branch, not this
/// function's return.
///
/// **Regression test for the blast-side twin of the flare bug.** Before
/// 2026-08-26, `blast_for` took no `kind` at all, so *every* impact - mine
/// and missile included - played the Rocket's own
/// `Trigger::TrackBlast`/`Trigger::CraftBlast`. Reusing another weapon's file
/// is worse than drawing nothing: it is a wrong, confident-looking answer.
#[test]
fn each_weapon_plays_only_its_own_recovered_explosion() {
    use oag_tables::weapons::Weapon;

    let race = race_with_weapon_table(Mode::SingleRace, enveloping_pad(), 1.0, one_rocket_table());
    let point = Vec3::new(1.0, 2.0, 3.0);

    assert_eq!(
        race.blast_for(Weapon::Missile, point, None),
        Some((Trigger::MissileExplo, point)),
        "the Missile's own explosion, at the impact point, whatever it hit"
    );
    assert_eq!(
        race.blast_for(Weapon::Mine, point, None),
        Some((Trigger::MineExplo, point)),
        "the Mine's own explosion (Mine_SpawnExplosion), at the impact point"
    );
    assert_eq!(
        race.blast_for(Weapon::Plasma, point, None),
        Some((Trigger::PlasmaBlast, point)),
        "the Plasma's own detonation (Plasma_SpawnDetonation), at the impact \
         point, whatever it hit"
    );
    assert_eq!(
        race.blast_for(Weapon::Plasma, point, Some(2)),
        Some((Trigger::PlasmaBlast, point)),
        "one file for every ending - the teardown pass does not branch on \
         what was struck, unlike the Rocket's"
    );
    assert_eq!(
        race.blast_for(Weapon::Bomb, point, None),
        Some((Trigger::BombSmokering, point)),
        "the Bomb's own smoke ring (BombBlast_Construct), at the impact \
         point - one third of the detonation; the other two are the \
         hemisphere and shockwave `.vex` models `ignite_blast` spawns \
         separately, see `bomb_blast`"
    );
    assert_eq!(
        race.blast_for(Weapon::Shuriken, point, None),
        Some((Trigger::ShurikenExpire, point)),
        "the blade's teardown (FUN_08870c78) plays WO_SHURIKEN_EXPIRE at the \
         blade, on a fuse running out and on a craft hit alike"
    );
    // The Plasma's detonation is its own authored file, not the bolt's
    // riding flare replayed at the impact - the same distinction the Rocket's
    // two-file split above exists for.
    assert_ne!(Trigger::PlasmaBlast, Trigger::PlasmaFlare);
}

/// A Cannon round throws a spark on a wall it hits, and nothing on a craft.
///
/// **The original's own split, not a simplification.** `Cannon_UpdateRound`
/// (`0x0886593c`) only calls `Psys_Spawn_q` off its own world-collision
/// raycast; the separate craft-proximity test that produces `struck: Some`
/// (`FUN_088579a8`/`FUN_08857f2c`) applies damage and a sound cue but never
/// spawns a particle. Playing [`Trigger::CannonSparks`] on a craft hit too
/// would be exactly the kind of plausible-looking invention `CLAUDE.md`
/// forbids - see `oag_weapons::projectile::cannon`'s module doc for the
/// full read.
///
/// **The struck craft still sparks, from its own side.** `Ship_Damage`
/// (`0x088439ac`) throws `WO_SHIP_COLL_SPARK_DAMAGE` from the victim's hull
/// locators for every weapon hit that gets through, the Cannon's included -
/// measured on PPSSPP 2026-09-24. That is `race::hit_sparks`, a separate
/// trigger this test does not cover; what it pins is that the *weapon* spawns
/// nothing of its own on a craft.
#[test]
fn a_cannon_round_sparks_on_a_wall_and_silently_on_a_craft() {
    use oag_tables::weapons::Weapon;

    let race = race_with_weapon_table(Mode::SingleRace, enveloping_pad(), 1.0, one_rocket_table());
    let point = Vec3::new(4.0, 0.5, -1.0);

    assert_eq!(
        race.blast_for(Weapon::Cannon, point, None),
        Some((Trigger::CannonSparks, point)),
        "no craft struck means the wall/track spark, at the impact point"
    );
    assert_eq!(
        race.blast_for(Weapon::Cannon, point, Some(1)),
        None,
        "a craft hit applies damage and a sound cue but throws no spark"
    );
}

/// A rocket in the air carries a flare, and gives it back when it is gone.
///
/// `Rocket_Init` attaches `WO_ROCKET_FLARE` at launch and it rides the
/// rocket for the whole flight - see [`Race::advance_projectile_flares`].
/// The bookkeeping is asserted here without a disc; what the flare *looks*
/// like is `crates/fx/tests/psys_ground_truth.rs`' business.
#[test]
fn a_projectile_takes_a_flare_slot_and_hands_it_back() {
    let mut race =
        race_with_weapon_table(Mode::SingleRace, enveloping_pad(), 1.0, one_rocket_table());
    race.sim
        .world
        .projectiles
        .spawn(oag_tables::weapons::Weapon::Rocket, Vec3::ZERO, Vec3::Z, 0);
    race.advance_projectile_flares();
    // No disc in a headless fixture, so no effect loaded and no instance
    // is taken - the trigger still has to run, and still has to be a
    // no-op rather than a panic.
    assert!(
        race.view.effects.is_empty(),
        "the fixture loaded no effects"
    );
    assert_eq!(race.view.projectile_flare[0], None);

    // With the slot vacated the bookkeeping must clear either way, or the
    // next projectile in that slot inherits a flare it never started.
    race.sim.world.projectiles.slots[0].kind = None;
    race.advance_projectile_flares();
    assert_eq!(race.view.projectile_flare[0], None);
    assert_eq!(race.stage().playing_count(), 0);
}
