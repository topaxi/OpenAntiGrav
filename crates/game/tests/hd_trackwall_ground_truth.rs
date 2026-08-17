//! Wipeout HD's `collision_trackwall`, measured on the disc and then flown at.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all \
//!     -E 'binary(hd_trackwall_ground_truth)'
//! ```
//!
//! The image has to be layer-1 decrypted first - `scripts/ps3iso.py decrypt`,
//! `docs/formats/ps3-disc.md`.
//!
//! # What is being established
//!
//! Class `0x3ed` is authored by HD and by nothing else, and until now it was
//! deliberately unnamed: `docs/formats/hd-status.md` recorded the node name
//! `collision_trackwall` as a *hypothesis* and refused to put it in a class
//! table on the strength of a name. These tests are what replaced the name with
//! measurements, and they are three separate claims:
//!
//! 1. **It is collision geometry, everywhere.** All 16 circuits author exactly
//!    one, all 16 payloads parse to a clean end.
//! 2. **It behaves as a wall and not as any other class.** Its triangles stand
//!    on end where the floor's lie flat, and it occupies the road's own volume
//!    rather than [`SurfaceKind::Wall`]'s much larger one. That is the same
//!    facing statistic `docs/formats/collision.md` settled Pure's classes with.
//! 3. **A craft stops at it.** Which is the point of all of the above, and the
//!    one thing a format measurement cannot show.
//!
//! What none of them establish is that Pulse's own class table names this ID -
//! see `oag_formats::vex::CLASS_TRACK_WALL_COLLISION` for why that is left open.

use std::path::{Path, PathBuf};

use oag_core::math::Vec3;
use oag_formats::collision::{self, CollisionNode, SurfaceKind};
use oag_game::race;
use oag_gameplay::collision_world;
use oag_physics::{Body, Environment, ShipControls, ShipState, SpeedClass, Surface, step};

/// The decrypted PS3 image.
const PS3_IMAGE: &str = "hdfury-ps3-eu-dec.iso";

/// Every circuit on the disc, with the archive holding it.
const CIRCUITS: &[(&str, &str)] = &[
    ("DATA00.PSARC", "amphiseum"),
    ("DATA00.PSARC", "modesto_heights"),
    ("DATA00.PSARC", "talons_junction"),
    ("DATA00.PSARC", "tech_de_ra"),
    ("DATA00.PSARC", "zone_1"),
    ("DATA00.PSARC", "zone_2"),
    ("DATA00.PSARC", "zone_3"),
    ("DATA00.PSARC", "zone_4"),
    ("DATA02.PSARC", "01_vineta_k"),
    ("DATA02.PSARC", "02_track"),
    ("DATA02.PSARC", "03_track"),
    ("DATA02.PSARC", "04_chenghou_project"),
    ("DATA02.PSARC", "05_ubermall"),
    ("DATA02.PSARC", "10_sebenco_climb"),
    ("DATA02.PSARC", "12_sol_2"),
    ("DATA02.PSARC", "15_anulpha_pass"),
];

/// A fixed 60 Hz tick, matching the rest of the simulation.
const TICK: f32 = 1.0 / 60.0;

/// How fast the ship is thrown at the barrier, in units per second.
///
/// Fast enough to cross a zero-thickness shell inside one tick, so the swept
/// query is what is under test - the same speed
/// `wall_collision_ground_truth.rs` uses, and for the same reason.
const APPROACH_SPEED: f32 = 150.0;

/// A triangle counts as standing on end when its normal is this far off world
/// up. `|n.y| < 0.5` is 60 degrees from horizontal.
const UPRIGHT: f32 = 0.5;

fn image() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/images")
        .join(PS3_IMAGE);

    if path.exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

/// One circuit's `track.vex`, straight out of its archive.
fn track(archive: &str, circuit: &str) -> Option<Vec<u8>> {
    let image = image()?;
    let spec = format!("{}:PS3_GAME/USRDIR/{archive}", image.display());
    let mut open = oag_assets::psarc::Archive::open(&spec).expect("the archive opens");
    Some(
        open.read_path(&format!("/data/environments/{circuit}/track.vex"))
            .expect("the circuit's .vex reads"),
    )
}

/// What one class contributes to a circuit: triangles, how many of them stand
/// on end, and the box they occupy.
#[derive(Default)]
struct Shape {
    meshes: usize,
    /// Every triangle, degenerate ones included - what a collider carries.
    total: usize,
    /// Only those with a normal to measure, which is what a facing statistic
    /// can be taken over. Two of Talon's Junction's 4,146 have none.
    triangles: usize,
    upright: usize,
    min: [f32; 3],
    max: [f32; 3],
}

impl Shape {
    fn upright_fraction(&self) -> f32 {
        if self.triangles == 0 {
            return 0.0;
        }
        self.upright as f32 / self.triangles as f32
    }

    fn extent(&self) -> [f32; 3] {
        std::array::from_fn(|i| self.max[i] - self.min[i])
    }
}

/// Measures one class over the nodes of one circuit.
fn shape_of(nodes: &[CollisionNode], kind: SurfaceKind) -> Shape {
    let mut shape = Shape {
        min: [f32::MAX; 3],
        max: [f32::MIN; 3],
        ..Shape::default()
    };
    for node in nodes.iter().filter(|n| n.kind == kind) {
        for mesh in &node.geometry.meshes {
            shape.meshes += 1;
            if let Some((lo, hi)) = mesh.bounds() {
                for i in 0..3 {
                    shape.min[i] = shape.min[i].min(lo[i]);
                    shape.max[i] = shape.max[i].max(hi[i]);
                }
            }
            shape.total += mesh.triangles.len();
            for t in 0..mesh.triangles.len() {
                let Some([a, b, c]) = mesh.triangle(t) else {
                    continue;
                };
                let (a, b, c) = (Vec3::from(a), Vec3::from(b), Vec3::from(c));
                let Some(normal) = (b - a).cross(c - a).try_normalize() else {
                    continue;
                };
                shape.triangles += 1;
                if normal.y.abs() < UPRIGHT {
                    shape.upright += 1;
                }
            }
        }
    }
    shape
}

/// Claim 1 and 2: every circuit authors exactly one, it parses, it stands on
/// end, and it occupies the road rather than the environment.
///
/// **The two comparisons are what make this a measurement rather than a
/// restatement of the node's name.** `Floor Collision` on the same circuit is
/// the near-zero end of the facing statistic and `Wall Collision` the middle,
/// so the number this class produces is calibrated against two classes whose
/// meaning is already settled - and the extent comparison is against the floor,
/// which is the road, using nothing this test itself chose.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn every_circuit_authors_one_barrier_that_stands_on_end_over_the_road() {
    let mut measured = 0;
    let mut worst = f32::MAX;
    for (archive, circuit) in CIRCUITS {
        let Some(blob) = track(archive, circuit) else {
            return;
        };
        let nodes = collision::from_vex(&blob).expect("the collision nodes decode");

        let barriers = nodes
            .iter()
            .filter(|n| n.kind == SurfaceKind::TrackWall)
            .count();
        assert_eq!(
            barriers, 1,
            "{circuit}: {barriers} collision_trackwall node(s), expected exactly one"
        );

        let barrier = shape_of(&nodes, SurfaceKind::TrackWall);
        let floor = shape_of(&nodes, SurfaceKind::Floor);
        let wall = shape_of(&nodes, SurfaceKind::Wall);
        println!(
            "{circuit:>20}: barrier {:5} tri {:5.1} % upright, extent {:?}; \
             floor {:5} tri {:5.1} %, extent {:?}; wall {:5} tri {:5.1} %",
            barrier.triangles,
            100.0 * barrier.upright_fraction(),
            barrier.extent().map(|v| v.round()),
            floor.triangles,
            100.0 * floor.upright_fraction(),
            floor.extent().map(|v| v.round()),
            wall.triangles,
            100.0 * wall.upright_fraction(),
        );

        assert!(
            barrier.triangles > 1_000,
            "{circuit}: the barrier decoded to only {} triangle(s)",
            barrier.triangles
        );
        // Calibrated, not chosen: the floor is 2 % upright on Talon's Junction
        // and the wall 54 %, so 60 % is above anything either class reaches
        // while leaving room for the banked circuits - `03_track` is the worst
        // at 64.6 % and it is the only one under 86 %.
        assert!(
            barrier.upright_fraction() > 0.6,
            "{circuit}: only {:.1} % of the barrier stands on end, which is not \
             a wall-shaped statistic",
            100.0 * barrier.upright_fraction()
        );
        // It hugs the road: within a quarter of the floor's own extent on each
        // axis, where `Wall Collision` is half again as large on two of three.
        for axis in 0..3 {
            let (b, f) = (barrier.extent()[axis], floor.extent()[axis]);
            assert!(
                b <= f * 1.25 && b >= f * 0.75,
                "{circuit}: barrier extent {b} on axis {axis} against the floor's \
                 {f}, so it is not co-extensive with the road"
            );
        }
        worst = worst.min(barrier.upright_fraction());
        measured += 1;
    }
    assert_eq!(measured, CIRCUITS.len());
    println!("{measured} circuits, worst {:.1} % upright", 100.0 * worst);
}

/// Claim 3, part one: the barrier reaches the collision world the race queries.
///
/// The number that moves is the one the load report prints, and it moves by
/// exactly the barrier's own meshes and triangles - so this fails loudly if the
/// class is ever dropped somewhere between `from_vex` and `collision_world`.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn the_barrier_reaches_the_collision_world_the_race_queries() {
    let Some(blob) = track("DATA00.PSARC", "talons_junction") else {
        return;
    };
    let nodes = collision::from_vex(&blob).expect("the collision nodes decode");
    let world = collision_world(&nodes);

    let barrier = shape_of(&nodes, SurfaceKind::TrackWall);
    let without: Vec<CollisionNode> = nodes
        .iter()
        .filter(|n| n.kind != SurfaceKind::TrackWall)
        .cloned()
        .collect();
    let world_without = collision_world(&without);

    let colliders = world.colliders().len() - world_without.colliders().len();
    let triangles: usize = world.colliders().iter().map(|c| c.triangle_count()).sum();
    let triangles_without: usize = world_without
        .colliders()
        .iter()
        .map(|c| c.triangle_count())
        .sum();
    println!(
        "talons_junction: the barrier adds {colliders} collider(s) and {} triangle(s)",
        triangles - triangles_without
    );
    assert_eq!(colliders, barrier.meshes);
    assert_eq!(triangles - triangles_without, barrier.total);
    assert!(
        barrier.meshes > 100,
        "only {} barrier collider(s), which is too few to be the whole circuit",
        barrier.meshes
    );
}

/// Claim 3, part two: a craft thrown at the barrier stops at it.
///
/// Structural assertions only - a side, a sign, a finite number - for the
/// reason `wall_collision_ground_truth.rs` gives at length: the response law is
/// this project's arithmetic until M3 verifies it, so pinning a magnitude here
/// would pin our own answer and call it a measurement.
#[test]
#[ignore = "needs a decrypted PS3 disc image in data/images"]
fn a_ship_thrown_at_hds_barrier_does_not_pass_through_it() {
    let Some(image) = image() else {
        return;
    };
    let Some(blob) = track("DATA00.PSARC", "talons_junction") else {
        return;
    };
    let nodes = collision::from_vex(&blob).expect("the collision nodes decode");
    let barrier_only: Vec<CollisionNode> = nodes
        .into_iter()
        .filter(|n| n.kind == SurfaceKind::TrackWall)
        .collect();
    let world = collision_world(&barrier_only);
    assert!(
        world
            .colliders()
            .iter()
            .all(|c| c.surface() == Surface::Wall),
        "the barrier reached the physics layer as something other than a wall"
    );

    // The ship's own hull box, from HD's own handling stats.
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: SpeedClass::Venom,
        ..race::Options::default()
    })
    .expect("loading the race");
    let handling = loaded.setup.handling;

    // The first triangle of the first collider, not a convenient one: any part
    // of the barrier has to work, and choosing would be the test grading itself.
    let collider = world
        .colliders()
        .iter()
        .find(|c| c.triangle_count() > 0)
        .expect("the barrier has triangles");
    let [a, b, c] = collider.triangle(0).expect("its first triangle");
    let centroid = (a + b + c) / 3.0;
    let normal = (b - a).cross(c - a).try_normalize().expect("a facing");
    println!("barrier triangle at {centroid:?}, normal {normal:?}");

    let standoff = handling.dimensions.length.max(4.0);
    let start = centroid + normal * standoff;
    let mut state = ShipState {
        body: Body {
            position: start,
            linear_velocity: -normal * APPROACH_SPEED,
            mass: handling.physical.mass,
            ..Body::default()
        },
        ..ShipState::default()
    };

    let side_of = |p: Vec3| (p - centroid).dot(normal);
    assert!(side_of(start) > 0.0);

    let mut deepest = f32::MAX;
    let mut turned_round = false;
    for tick in 0..120 {
        step(
            &mut state,
            &ShipControls::default(),
            &handling,
            &Environment::default(),
            &world,
            TICK,
        );
        deepest = deepest.min(side_of(state.body.position));
        turned_round |= state.body.linear_velocity.dot(normal) > 0.0;
        assert!(
            state.body.position.is_finite(),
            "tick {tick}: position went non-finite"
        );
    }
    println!("deepest penetration {deepest}, turned round {turned_round}");

    let allowance = -handling.dimensions.length;
    assert!(
        deepest > allowance,
        "the ship went through the barrier: reached {deepest}, allowed {allowance}"
    );
    assert!(
        turned_round,
        "the ship never gained any velocity back along the barrier's normal"
    );
}
