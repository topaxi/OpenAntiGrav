//! What `race.rs` is asserted to do.
//!
//! Its own directory rather than a `#[cfg(test)] mod tests` block at the end of
//! `race.rs`: the tests are 3,400 lines and the code they cover 7,750, so the
//! file that had to be read to change one force law carried a third of itself
//! in assertions. See `scripts/check-file-size.py`, which is the rule as a
//! gate.
//!
//! This file holds only the fixtures more than one theme reads; the tests are in
//! the modules below, named for the seams `HANDOVER.md` records for the eventual
//! `race/` split - loading and its report, the race itself, the scene
//! composition. The capture seam has no tests here at all.

use super::*;
use oag_formats::track;

mod camera;
mod field;
mod hash;
mod held_buttons;
mod load;
mod pads;
mod respawn;
mod scene;
mod spawn;
mod spline;
mod weapons;

/// A model declaring `slots` texture slots of which the first `decoded` filled.
fn model(slots: usize, decoded: usize) -> Model {
    Model {
        airbrakes: [None, None],
        node_vertex_ranges: Vec::new(),
        label: "Ship.vex".to_string(),
        vertices: Vec::new(),
        indices: vec![0; 6],
        draws: Vec::new(),
        alpha_tested_draws: Vec::new(),
        transparent_draws: Vec::new(),
        textures: (0..slots)
            .map(|index| {
                (index < decoded).then(|| mesh::ModelTexture {
                    label: format!("#{index}"),
                    width: 1,
                    height: 1,
                    rgba: vec![255; 4],
                })
            })
            .collect(),
        centre: [0.0; 3],
        radius: 1.0,
        anim_tracks: Vec::new(),
        mesh_count: 1,
    }
}

/// A straight synthetic path, so the locator and the spawn can be tested with no
/// disc image. Eight control points along `+x`, twelve units wide either side.
fn straight_track() -> AiTrack {
    let count = 8usize;
    let paths_at = track::HEADER_LEN + track::RESERVED_LEN;
    let junctions_at = paths_at + track::PATH_LEN;
    let points_at = junctions_at + track::JUNCTION_LEN;
    let mut out = vec![0u8; points_at + count * track::POINT_LEN];
    out[0..4].copy_from_slice(&track::MAGIC.to_le_bytes());
    out[4..8].copy_from_slice(&0x105u32.to_le_bytes());
    out[8..12].copy_from_slice(&1u32.to_le_bytes());
    out[12..16].copy_from_slice(&1u32.to_le_bytes());
    out[paths_at..paths_at + 4].copy_from_slice(&(count as u32).to_le_bytes());
    for k in 0..count {
        let at = points_at + k * track::POINT_LEN;
        let mut put = |off: usize, v: f32| {
            out[at + off..at + off + 4].copy_from_slice(&v.to_le_bytes());
        };
        // Position along +x, tangent +x, down -y, lateral +z.
        put(0x00, k as f32 * 10.0);
        put(0x10, 1.0);
        put(0x24, -1.0);
        put(0x38, 1.0);
        put(0x44, 12.0);
        put(0x48, 12.0);
    }
    track::parse(&out).expect("the synthetic track must parse")
}

fn setup(handling: Handling) -> Setup {
    let ai = straight_track();
    let spline = Spline::from_track(&ai);
    // **This does produce a ring, and the comment here said otherwise until
    // 2026-08-11.** The blob's junction record is all zeros, so path 0's `next`
    // reads as path 0 and the chain closes on itself: `Course::from_track`
    // returns 32 points over 136.6 units, and every test built on this fixture
    // has lap counting *on*, over a circuit that is geometrically a straight
    // line. Harmless for what these tests assert - the force law and the camera
    // do not read the course - but a test that wants the no-ring path has to
    // build its own `None` rather than assume this one is it.
    let course = Course::from_track(&ai, None);
    Setup {
        airbrake_graphics: oag_gameplay::AirbrakeGraphics {
            amount: 0.4363323,
            up_speed: 30.0,
            down_speed: 30.0,
        },
        mode: Mode::TimeTrial,
        difficulty: oag_ai::Difficulty::default(),
        class: SpeedClass::Venom,
        opponents: false,
        seed: SEED,
        // A time trial does not read it, and these tests never run a Zone
        // race: the numbers are the disc's and there is no disc here.
        zone: None,
        ai,
        spline,
        course,
        // The synthetic track has no authored slot, which is the fallback
        // path: every assertion below is about a ship placed on the spline.
        start_position: None,
        collision: CollisionWorld::new(),
        handling,
        // Round numbers, chosen to make the geometry readable. None of these
        // claims to be the game's; a real race reads all seven off the disc.
        chase: ChaseParams {
            fov: 60.0,
            lookat_height: 1.0,
            lookat_length: 10.0,
            pos_height: 2.0,
            pos_length: 8.0,
            spring_horiz: 4.0,
            spring_vert: 2.0,
            // `1.0` rather than the craft's `0.75`, so the round numbers
            // above stay the numbers a reader can check the geometry
            // against. A real load passes the scale through.
            craft_scale: 1.0,
        },
        // The nearer external view, deliberately a *different* set of round
        // numbers from `chase`: a test that cycles to it and back has to be
        // able to tell the two apart. Note these go into `Setup` already
        // converted, so unlike a real load no `chase_params` scale or sign
        // flip is applied to them.
        chase_close: ChaseParams {
            fov: 50.0,
            lookat_height: 1.0,
            lookat_length: 10.0,
            pos_height: 1.0,
            pos_length: 4.0,
            spring_horiz: 4.0,
            spring_vert: 2.0,
            craft_scale: 1.0,
        },
        // The cockpit view. `length` positive puts the eye ahead of the
        // craft's origin, which is the sign the original's own measured
        // sample pins - see `oag_render::camera::internal`.
        internal: InternalParams {
            fov: 65.0,
            headtilt: Some(3.0),
            height: 1.0,
            length: 5.0,
            pitch: 0.0,
        },
        // A synthetic setup has no ship model, so no locators either. The
        // exhaust still ticks; it just has nowhere to be drawn, which is the
        // same path a model with no `Engine Flare` node takes. Sparks
        // likewise fall back to anchoring at the contact point.
        nozzles: vec![None; MAX_SHIPS],
        collision_fx: Vec::new(),
        // No disc, so no `.pob` at all: every effect trigger runs and
        // draws nothing, which is the same path a missing entry takes.
        // A headless test that wants real particles parses a blob itself
        // and inserts it here.
        effects: psys::Library::new(),
        // A synthetic track authors no pads, which is also what every Pure
        // track does: an empty set is an ordinary state, not a stub.
        speedup_pads: Vec::new(),
        weapon_pads: Vec::new(),
        // No disc here, so no weapon table: the same state a race reaches
        // when `WeaponStats_Race.xml` is unreadable, where a pad hands
        // nothing out. `race_with_weapon_pads` supplies one where a test
        // needs a pickup to exist.
        weapons: None,
        weapon_pad_refresh: 0.0,
        // These tests run on a synthetic straight and want the ordinary
        // spawn and the ordinary chase camera; `--pose` and its camera are
        // capture aids with nothing to say here.
        pose_override: None,
        camera_override: None,
        // The identity, so every assertion below is about the force law and
        // not about a scale. This is also what a race gets when the
        // engine-wide file is unreadable.
        class_gravity_scale: 1.0,
    }
}

/// A pad whose volume is `half` units across in every direction, centred on
/// `at`, pushing along world `+z`.
///
/// The identity basis is what makes the push direction readable: row 2 of an
/// identity matrix is `+z`, so the boost the assertions look for is `+z`.
fn pad_at(at: Vec3, half: f32) -> oag_formats::pads::PadVolume {
    let mut to_world = [0.0f32; 16];
    to_world[0] = 1.0;
    to_world[5] = 1.0;
    to_world[10] = 1.0;
    to_world[15] = 1.0;
    to_world[12] = at.x;
    to_world[13] = at.y;
    to_world[14] = at.z;
    oag_formats::pads::PadVolume {
        to_world,
        min: [-half; 3],
        max: [half; 3],
        disabled: 0.0,
    }
}

/// Switches the player's off-track rescue off for a fixture that drives.
///
/// **The synthetic straight is 70 units long.** Any fixture whose ship holds the
/// throttle down for more than a second leaves the far end of the sample table,
/// and [`Race::lost_off_the_track`] reads that - correctly - as a craft off the
/// track and puts it back, resetting the timers the test was measuring. A real
/// circuit's table closes on itself, so travelling forward never increases the
/// distance to it: this is a property of the fixture, not of the trigger.
///
/// Applied to the fixtures that drive rather than to `setup`, so a test *about*
/// the rescue still gets one - see `race/tests/respawn.rs`.
fn without_player_rescue(mut race: Race) -> Race {
    race.player_rescue_distance = f32::INFINITY;
    race
}

/// A race whose ship is inside a pad from its first tick, and one that is
/// nowhere near one, so a test can difference them.
fn race_with_pads(mode: Mode, pads: Vec<oag_formats::pads::PadVolume>) -> Race {
    let mut handling = hulled_handling();
    // Invented, and large enough that the boost is unmistakable against the
    // rest of the force law rather than lost in it. Deliberately **not** a
    // round hundred: the shipped `amount` is one, and ADR-0006 keeps shipped
    // values out of this repository even where they would read as arbitrary.
    handling.speedup_pads = oag_physics::params::SpeedupPads {
        amount: 37.0,
        time: 0.5,
    };
    let mut setup = setup(handling);
    setup.mode = mode;
    setup.speedup_pads = pads;
    without_player_rescue(Race::start(setup))
}

/// A pad big enough to hold the ship wherever `spawn_pose` puts it, so the
/// test is about the trigger rather than about the spawn.
fn enveloping_pad() -> Vec<oag_formats::pads::PadVolume> {
    vec![pad_at(Vec3::ZERO, 1.0e6)]
}

/// A full grid on the synthetic straight, which needs an authored slot: the
/// whole grid layout is offsets from one, and `Race::start` refuses to guess
/// at it. Faces along `+x`, which is the direction `straight_track` runs.
pub(super) fn race_with_a_grid() -> Race {
    let mut setup = setup(hulled_handling());
    setup.mode = Mode::SingleRace;
    setup.start_position = Some(oag_formats::track::StartPosition {
        position: [0.0, 0.0, 0.0],
        left: [0.0, 0.0, -1.0],
        up: [0.0, 1.0, 0.0],
        forward: [1.0, 0.0, 0.0],
    });
    // **Not `without_player_rescue`**, unlike the two fixtures that drive: this
    // one's craft have no engine, so nothing here reaches the end of the straight
    // - and the opponent-rescue tests built on it assert that slot 0 was *not*
    // recovered, which is worth nothing if slot 0's recovery is switched off.
    Race::start(setup)
}

/// A weapon table with one Turbo in it, weighted for a human.
///
/// **Built by parsing a document rather than by constructing the struct**,
/// which is not only because `WeaponStats::simple` is private: it means
/// every test below runs against the same reader a real disc goes through,
/// so a parser change that broke the pickup path could not pass here.
///
/// Every number is invented, per ADR-0006 - what the shipped file authors is
/// not in this repository. `absorb` and `time` are deliberately different
/// from each other and from every other constant here, so a test that
/// confused them would fail rather than pass by coincidence.
fn one_turbo_table() -> oag_formats::weapons::WeaponStats {
    oag_formats::weapons::parse(
        r#"<WeaponStats>
             <Weapon type="Global"><Stats slowdown_limit="0"/></Weapon>
             <Weapon type="Turbo"><Stats absorb="23" time="0.75"/></Weapon>
             <Pickupodds class="Venom">
               <Weapon type="Turbo"><Stats ai="1" back="1" front="1" human="1"/></Weapon>
             </Pickupodds>
           </WeaponStats>"#,
    )
    .expect("the fixture table must parse")
}

/// Drives buttons the way the real input layer does: a per-tick *level*,
/// with the pressed and released edges derived from the previous tick's.
///
/// Not [`HeldButtons`], which goes through the keyboard map and therefore
/// only reaches the seven buttons `key_for_button` binds - `SQUARE` and
/// `CIRCLE`, the two the pickup reads, are not among them.
struct Buttons(oag_gameplay::input::Input);

impl Buttons {
    fn new() -> Self {
        Self(oag_gameplay::input::Input::new())
    }

    /// One tick's snapshot with exactly `mask` held.
    fn tick(&mut self, mask: u32) -> InputSnapshot {
        self.0.begin_frame(mask);
        InputSnapshot {
            buttons: self.0,
            ..InputSnapshot::default()
        }
    }
}

const CROSS: u32 = 1 << oag_gameplay::input::button::CROSS;
const SQUARE: u32 = 1 << oag_gameplay::input::button::SQUARE;
const CIRCLE: u32 = 1 << oag_gameplay::input::button::CIRCLE;

/// A race with weapon pads, the mode that arms them, and a table to draw
/// from. `refresh` is `<WeaponPad refresh_time>`.
fn race_with_weapon_pads(
    mode: Mode,
    pads: Vec<oag_formats::pads::PadVolume>,
    refresh: f32,
) -> Race {
    race_with_weapon_table(mode, pads, refresh, one_turbo_table())
}

/// [`race_with_weapon_pads`] with the table chosen by the caller, so a test
/// about a weapon other than the Turbo does not have to reproduce the rest
/// of the fixture. Every existing caller wants a table weighting Turbo alone
/// - which is also what keeps their draws stable as `IMPLEMENTED` grows.
fn race_with_weapon_table(
    mode: Mode,
    pads: Vec<oag_formats::pads::PadVolume>,
    refresh: f32,
    table: oag_formats::weapons::WeaponStats,
) -> Race {
    let mut handling = hulled_handling();
    // `Handling::ZERO` gives an engine that produces no thrust and a hull
    // with no energy pool, and both of those are what these tests measure a
    // *difference* in. Invented numbers, large enough to read cleanly.
    handling.engine.amount = 20.0;
    handling.engine.accelcap = 1000.0;
    // The turbo add. Deliberately **larger than the whole capped thrust
    // above**, which is the shipped relationship rather than an arbitrary
    // one: every class on the disc authors `<Engine turbo>` an order of
    // magnitude above its own `accelcap`. A fixture where the turbo were a
    // small fraction would pass a test that had wired the wrong term.
    handling.engine.turbo = 500.0;
    handling.dimensions.shield = 100.0;
    let mut setup = setup(handling);
    setup.mode = mode;
    setup.weapon_pads = pads;
    setup.weapons = Some(table);
    setup.weapon_pad_refresh = refresh;
    without_player_rescue(Race::start(setup))
}

/// The Rocket's own fixture. Invented numbers, all distinct, and the four
/// class speeds ascending so a class read from the wrong index is a wrong
/// *value* rather than a coincidence.
///
/// **`spread` is the exception to "all distinct" and has to be**: it is an
/// angle in radians now that `Weapon_FireRocket` is read, so the arbitrary
/// `17` this fixture used while the attribute was undecoded is 974 degrees
/// and fires two of the three rockets backwards. A plausible fan instead.
fn one_rocket_table() -> oag_formats::weapons::WeaponStats {
    oag_formats::weapons::parse(
        r#"<WeaponStats>
             <Weapon type="Global"><Stats slowdown_limit="0"/></Weapon>
             <Weapon type="Rocket"><Stats absorb="11" blastforce="12" blastradius="13"
               damage="14" slowdown_time="15" venomspeed="600" flashspeed="700"
               rapierspeed="800" phantomspeed="900" launchSpeed="16" spread="0.2"/></Weapon>
             <Pickupodds class="Venom">
               <Weapon type="Rocket"><Stats ai="1" back="1" front="1" human="1"/></Weapon>
             </Pickupodds>
           </WeaponStats>"#,
    )
    .expect("the fixture table must parse")
}

/// A parameter set with a real hull, so the reset probes have something to
/// probe with, and no gravity, so a ship only moves when a test moves it.
fn hulled_handling() -> Handling {
    Handling {
        physical: oag_physics::params::Physical {
            mass: 1.0,
            ..Default::default()
        },
        antigrav: oag_physics::params::Antigrav {
            ride_height: 8.0,
            ..Default::default()
        },
        dimensions: oag_physics::params::Dimensions {
            width: 2.0,
            height: 1.0,
            length: 4.0,
            ..Default::default()
        },
        ..Handling::ZERO
    }
}

/// A large quad, as a collider of one class.
///
/// `axis` picks the plane: 1 is horizontal at height `at`, 0 is vertical at
/// `x = at`.
fn plane(
    axis: usize,
    at: f32,
    surface: oag_physics::Surface,
    collider: u32,
) -> oag_physics::TriangleSoup {
    let corner = |u: f32, v: f32| {
        let mut p = [0.0f32; 3];
        p[axis] = at;
        let others: Vec<usize> = (0..3).filter(|&k| k != axis).collect();
        p[others[0]] = u;
        p[others[1]] = v;
        p
    };
    oag_physics::TriangleSoup::new(
        vec![
            corner(-500.0, -500.0),
            corner(500.0, -500.0),
            corner(500.0, 500.0),
            corner(-500.0, 500.0),
        ],
        vec![[0, 1, 2], [0, 2, 3]],
        Vec::new(),
        surface,
        collider,
    )
}

fn setup_with(handling: Handling, colliders: Vec<oag_physics::TriangleSoup>) -> Setup {
    let mut setup = setup(handling);
    for c in colliders {
        setup.collision.push(c);
    }
    setup
}
