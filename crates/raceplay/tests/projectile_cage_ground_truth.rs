//! A projectile whose probe or travel sweep ends outside the track's collision cage ends there,
//! as in Pulse PSP (`Collision_SweepSegment`'s second query, code 4).
//!
//! **`#[ignore]`d and never run in CI**: needs `data/images/pulse-psp-usa.chd`.
//!
//! The rows are the original's own: 17 queries that returned `4`, recorded from PPSSPP on Moa
//! Therma White (`03`) and Outpost 7 (`07`), and the five states a shot is seeded from, with the
//! update each ended at (`docs/ghidra/functions/psp-pulse-usa/rocket-visuals.md`, "Code 4 is the
//! collision cage" and "States a test can start from").

#![allow(
    clippy::excessive_precision,
    reason = "the recorded f32 values, written so they round-trip"
)]

use oag_core::math::Vec3;
use oag_physics::cage::Containment;
use oag_raceplay as race;
use oag_tables::weapons::Weapon;
use oag_weapons::projectile::{BlastPush, Impact, MAX_FLIGHT_SECONDS, Projectile, WeaponHit, step};

/// Circuit, the point handed to the cage, the boundary point and the inward normal the original
/// wrote.
type Row = (&'static str, [f32; 3], [f32; 3], [f32; 3]);

const CODE_FOUR: [Row; 17] = [
    (
        "07",
        [-1003.14124, 27.6935787, 323.518341],
        [-1003.03137, 27.3187866, 324.717529],
        [0.217815444, -0.742014706, 0.634011507],
    ),
    (
        "07",
        [-1057.76074, 26.0380802, 340.277557],
        [-1057.76099, 23.4980984, 341.191925],
        [-0.000021537, -0.995635986, 0.0933219418],
    ),
    (
        "03",
        [-399.711243, -3.04446721, -195.946106],
        [-400.731567, -2.5530529, -195.113831],
        [-0.493117094, 0.768526137, 0.407680124],
    ),
    (
        "03",
        [-403.710052, -3.02149534, -199.790573],
        [-404.261719, -2.73824906, -199.276031],
        [-0.446457237, 0.788755894, 0.422540098],
    ),
    (
        "03",
        [-408.239197, -3.08226919, -207.286865],
        [-409.452789, -2.44744635, -206.012726],
        [-0.433653533, 0.773905933, 0.46153456],
    ),
    (
        "03",
        [-414.67337, -8.14909363, -211.108215],
        [-418.061096, -5.22365379, -207.178375],
        [-0.274389625, 0.904527545, 0.326405019],
    ),
    (
        "07",
        [-995.413513, 20.0413914, 314.958344],
        [-995.413391, 20.0325489, 314.973816],
        [0.000000000, -0.765413702, 0.643538535],
    ),
    (
        "03",
        [315.995728, 156.423172, -308.0867],
        [317.471527, 149.448441, -312.112213],
        [0.28801614, -0.315787911, -0.904060185],
    ),
    (
        "03",
        [419.23172, 147.842957, -284.953735],
        [419.275146, 147.486908, -285.207123],
        [0.154092729, -0.437603176, -0.885866165],
    ),
    (
        "03",
        [421.332306, 149.149902, -281.375488],
        [421.774567, 145.728683, -284.122803],
        [0.150157258, -0.388105422, -0.909300268],
    ),
    (
        "03",
        [424.644073, 153.560471, -292.032837],
        [424.663361, 153.083725, -292.162262],
        [0.0363765396, -0.968361974, -0.246884197],
    ),
    (
        "03",
        [427.125061, 98.7301712, -294.697662],
        [427.298889, 104.014793, -295.53064],
        [0.104847066, 0.777993023, -0.619462669],
    ),
    (
        "03",
        [493.138702, 138.099609, -269.22757],
        [493.253723, 136.772324, -271.069824],
        [0.073917754, -0.296649545, -0.952121496],
    ),
    (
        "03",
        [495.809082, 146.5896, -277.276733],
        [495.828674, 144.731491, -278.270874],
        [0.010019266, -0.877997518, -0.478560209],
    ),
    (
        "03",
        [553.301697, 143.563202, -280.20932],
        [553.281616, 143.167572, -280.293335],
        [-0.0438401513, -0.953392804, -0.298530191],
    ),
    (
        "03",
        [596.443726, 107.809517, -261.695465],
        [596.445251, 109.337738, -265.648773],
        [0.0364655964, 0.684079885, -0.728495061],
    ),
    (
        "03",
        [770.331238, 103.8703, -310.767151],
        [767.587646, 104.24128, -304.187561],
        [-0.218374431, 0.664490044, 0.714678645],
    ),
];

const PLASMA_DT: [f32; 5] = [0.016678, 0.016684, 0.016719, 0.016652, 0.016684];
/// The Shuriken's own state at the entry of updates 50 to 58: `dt`, age, position, velocity and
/// carried normal.
type State = (f32, f32, [f32; 3], [f32; 3], [f32; 3]);
const SHURIKEN_STATES: [State; 9] = [
    (
        0.016679,
        0.81766,
        [458.71701, 123.838112, -291.732483],
        [230.851028, 125.619408, 91.0196915],
        [0.120310582, 0.427946955, -0.895760417],
    ),
    (
        0.016676,
        0.834339,
        [462.567383, 125.933319, -290.214355],
        [230.851501, 125.619469, 91.0202866],
        [0.120310582, 0.427946955, -0.895760417],
    ),
    (
        0.016628,
        0.851015,
        [466.397491, 128.039871, -288.72287],
        [229.677902, 126.322388, 89.4390488],
        [0.112735413, 0.428093076, -0.896675527],
    ),
    (
        0.01671,
        0.867643,
        [470.519531, 130.138611, -286.906616],
        [247.897491, 126.217194, 109.228622],
        [0.222724646, 0.461705059, -0.858616471],
    ),
    (
        0.016657,
        0.884353,
        [474.588593, 132.095703, -284.798737],
        [243.510544, 117.121017, 126.144798],
        [0.222724646, 0.461705059, -0.858616471],
    ),
    (
        0.016941,
        0.90101,
        [478.644745, 134.04657, -282.69751],
        [243.510376, 117.119934, 126.146774],
        [0.222724646, 0.461705059, -0.858616471],
    ),
    (
        0.016418,
        0.917951,
        [482.453247, 136.274139, -280.720032],
        [224.809769, 131.489853, 116.727348],
        [0.101934679, 0.511382878, -0.853285909],
    ),
    (
        0.016683,
        0.934369,
        [486.473663, 138.420303, -278.474701],
        [244.878555, 130.720169, 136.7603],
        [0.219511256, 0.546030641, -0.808495641],
    ),
    (
        0.016713,
        0.951052,
        [491.137177, 142.501282, -277.876251],
        [279.536865, 244.618988, 35.87183],
        [0.222532511, 0.70749855, -0.670764565],
    ),
];
const CANNON_DT: [f32; 25] = [
    0.016694, 0.017065, 0.016326, 0.016689, 0.01683, 0.01653, 0.016672, 0.016669, 0.016683,
    0.016698, 0.016669, 0.016688, 0.016695, 0.016667, 0.016705, 0.016986, 0.016366, 0.01668,
    0.016725, 0.016627, 0.016683, 0.016683, 0.016684, 0.016692, 0.016679,
];
const ROCKET_DT: [f32; 16] = [
    0.017101, 0.016269, 0.016728, 0.016916, 0.016518, 0.016624, 0.016726, 0.016683, 0.016648,
    0.016672, 0.016629, 0.016939, 0.016444, 0.016742, 0.016982, 0.016287,
];

fn v3(v: [f32; 3]) -> Vec3 {
    Vec3::new(v[0], v[1], v[2])
}

fn start(circuit: &str) -> Option<race::Race> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        track: Some(format!("Data\\Environments\\{circuit}_Track\\track.vex")),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::TimeTrial,
        ..race::Options::default()
    })
    .expect("loading the race");
    Some(race::Race::start(loaded.setup))
}

fn cage_of(race: &race::Race) -> &dyn Containment {
    race.spline()
        .cage()
        .expect("Pulse runs the cage on this circuit")
}

/// Every query the original answered `4` is outside here, at its boundary point and normal; the
/// craft's own grid slot is inside.
fn the_recorded_code_four_queries_are_outside(race: &race::Race, circuit: &str) {
    let cage = cage_of(race);
    let mut seen = 0;
    for (_, point, boundary, normal) in CODE_FOUR.iter().filter(|row| row.0 == circuit) {
        let (found, found_normal) = cage
            .outside(v3(*point))
            .unwrap_or_else(|| panic!("{point:?} is outside the original's cage"));
        assert!(
            (found - v3(*boundary)).length() < 0.01,
            "{point:?}: boundary {found:?}, the original's {boundary:?}"
        );
        assert!(
            found_normal.dot(v3(*normal)) > 0.9999,
            "{point:?}: normal {found_normal:?}, the original's {normal:?}"
        );
        seen += 1;
    }
    assert!(seen > 0, "no recorded row for {circuit}");
    let grid = race.sim.world.ships[0].physics.body.position;
    assert!(
        cage.outside(grid).is_none(),
        "the grid slot is inside the cage"
    );
}

struct Flight {
    /// How many updates ran, counting the one that ended it.
    calls: usize,
    impact: Option<Impact>,
}

/// One update of the pool in \`world\`; the impact if its first slot ended.
fn update(
    race: &race::Race,
    world: &mut oag_gameplay::World,
    cage: Option<&dyn Containment>,
    dt: f32,
) -> Option<Impact> {
    let count = world.ship_count as usize;
    let mut hits = vec![WeaponHit::default(); world.ships.len()];
    let impacts = step(
        &mut world.projectiles,
        &mut world.ships[..count],
        dt,
        race.collision(),
        cage,
        race.weapon_stats(),
        "VENOM",
        BlastPush::Posted,
        oag_physics::DamageRules::default(),
        &mut hits,
    );
    impacts[0]
}

/// Steps a seeded projectile through the original's own `dt` of each update, then a few more
/// at the last, until it ends.
fn fly(race: &race::Race, cage: Option<&dyn Containment>, seed: Projectile, dts: &[f32]) -> Flight {
    let limit = dts.len() + 3;
    let mut world = race.sim.world.clone();
    world.projectiles.clear();
    world.projectiles.slots[0] = seed;
    for calls in 1..=limit {
        let dt = dts[(calls - 1).min(dts.len() - 1)];
        if let Some(impact) = update(race, &mut world, cage, dt) {
            return Flight {
                calls,
                impact: Some(impact),
            };
        }
        assert!(
            world.projectiles.slots[0].kind.is_some(),
            "reaped silently after {calls} updates"
        );
    }
    Flight {
        calls: limit,
        impact: None,
    }
}

fn seeded(
    kind: Weapon,
    age: f32,
    position: [f32; 3],
    velocity: [f32; 3],
    surface: [f32; 3],
    launch_speed_kmh: f32,
) -> Projectile {
    Projectile {
        kind: Some(kind),
        position: v3(position),
        velocity: v3(velocity),
        surface: v3(surface),
        owner: 0,
        lifetime: MAX_FLIGHT_SECONDS - age,
        launch_speed_kmh,
        ..Projectile::default()
    }
}

/// The impact is the update the original ended on, within `tolerance` of one of its boundary
/// points.
fn ends_at(flight: &Flight, calls: usize, points: &[[f32; 3]], tolerance: f32) {
    let impact = flight
        .impact
        .unwrap_or_else(|| panic!("still flying after {} updates", flight.calls));
    assert_eq!(flight.calls, calls, "ended on update {}", flight.calls);
    let nearest = points
        .iter()
        .map(|p| (impact.point - v3(*p)).length())
        .fold(f32::MAX, f32::min);
    assert!(
        nearest < tolerance,
        "impact {:?} is {nearest} from the original's {points:?}",
        impact.point
    );
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn moa_therma_white_recorded_queries() {
    let Some(race) = start("03") else { return };
    the_recorded_code_four_queries_are_outside(&race, "03");
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn outpost_7_recorded_queries() {
    let Some(race) = start("07") else { return };
    the_recorded_code_four_queries_are_outside(&race, "07");
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn moa_therma_white_plasma_ends_at_the_cage() {
    let Some(race) = start("03") else { return };
    let cage = Some(cage_of(&race));
    // The Plasma rides the bank's kerb from update 45 and the original's query returns 4 at 49.
    let plasma = seeded(
        Weapon::Plasma,
        0.7338,
        [407.899, 148.454, -295.326],
        [204.506, 60.023, 72.350],
        [0.261, 0.239, -0.935],
        700.77,
    );
    let flight = fly(&race, cage, plasma, &PLASMA_DT);
    ends_at(
        &flight,
        5,
        &[[421.77, 145.73, -284.12], [424.66, 153.08, -292.16]],
        0.5,
    );
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn moa_therma_white_shuriken_ends_at_the_cage() {
    let Some(race) = start("03") else { return };
    let cage = Some(cage_of(&race));
    // The Shuriken's travel sweep returns 4 at update 58. Its speed is not constant in the
    // original and ours keeps its length (`docs/ghidra/functions/psp-pulse-usa/shuriken.md`), so
    // a chain from update 50 drifts off the kerb; each update starts from the original's own
    // state instead, which isolates this law.
    let mut ended = None;
    for (index, (dt, age, position, velocity, surface)) in SHURIKEN_STATES.iter().enumerate() {
        let mut world = race.sim.world.clone();
        world.projectiles.clear();
        world.projectiles.slots[0] =
            seeded(Weapon::Shuriken, *age, *position, *velocity, *surface, 0.0);
        let impact = update(&race, &mut world, cage, *dt);
        assert_eq!(
            impact.is_some(),
            index == SHURIKEN_STATES.len() - 1,
            "update {}: {impact:?}",
            50 + index
        );
        ended = impact;
    }
    let flight = Flight {
        calls: 9,
        impact: ended,
    };
    ends_at(&flight, 9, &[[495.83, 144.73, -278.27]], 0.5);
    assert!(
        flight.impact.is_some_and(|i| !i.blast),
        "the blade's teardown spends no damage"
    );
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn moa_therma_white_cannon_ends_at_the_cage() {
    let Some(race) = start("03") else { return };
    let cage = Some(cage_of(&race));
    // The Cannon round flies straight and ends at update 25.
    let cannon = seeded(
        Weapon::Cannon,
        0.0,
        [256.346, 123.760, -347.127],
        [687.304, 33.099, 299.866],
        [0.0, 0.0, 0.0],
        0.0,
    );
    let flight = fly(&race, cage, cannon, &CANNON_DT);
    ends_at(&flight, 25, &[[553.28, 143.17, -280.29]], 0.5);
}

#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn outpost_7_rocket_ends_at_the_cage() {
    let Some(race) = start("07") else { return };
    let cage = Some(cage_of(&race));
    // The third rocket of the volley rides the ramp from update 6; the original ends it on update
    // 21, above the lip.
    let rocket = seeded(
        Weapon::Rocket,
        0.0835,
        [-1050.274, -1.294, 343.232],
        [164.477, -3.584, -26.922],
        [0.0, 1.0, 0.0],
        800.0,
    );
    let flight = fly(&race, cage, rocket, &ROCKET_DT);
    ends_at(&flight, 16, &[[-1003.03, 27.32, 324.72]], 0.5);
}
