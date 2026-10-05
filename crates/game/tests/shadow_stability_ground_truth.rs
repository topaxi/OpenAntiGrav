//! The `original` tier's shadow, tick against tick, on a real circuit.
//!
//! `#[ignore]`d: needs a disc image. `OAG_REQUIRE_GAME_DATA=1 cargo nextest run
//! -p oag-game --run-ignored all shadow_stability`.
//!
//! The shadow is a flat polygon laid on the plane of the one collision
//! triangle the craft's downward ray hit. A hull is several units across and a
//! road is not flat, so away from that triangle the plane can stand under the
//! road - and which triangle is hit changes as the craft crosses seams. A
//! polygon buried under the road for one tick and clear of it on the next is a
//! shadow that blinks. This measures how deep, in world units, the drawn
//! polygon sits under the circuit's own floor, vertex by vertex and at each
//! triangle's centre.

use oag_core::math::Vec3;
use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

/// Ticks past the warm-up each window covers.
const WINDOW: u64 = 60;

/// Where each window starts, in ticks after the race began: Zone's start, a
/// lap in at Zone's own pace, and later still.
const WARM: [u64; 3] = [900, 3000, 6000];

/// How far under the road a conformed polygon may still sit, in world units.
///
/// **Chosen, not measured.** Well under the 0.6 the unconformed polygon reaches,
/// and above what the conformed one does, which is at worst the centre of a
/// sub-triangle between fitted vertices.
const CLEAR_ENOUGH: f32 = 0.1;

/// What one window of ticks measured.
struct Window {
    /// The deepest any sampled point of the drawn polygon sat under the floor.
    deepest: f32,
    /// How many ticks had a point buried by more than [`CLEAR_ENOUGH`].
    ticks_buried: usize,
    /// Ticks the craft had a shadow at all.
    ticks: usize,
}

fn measure(mode: oag_race::Mode, warm: u64, conform: bool) -> Option<Window> {
    let image = oag_testdata::image("data/images/pulse-psp-usa.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode,
        ..race::Options::default()
    })
    .expect("loading the race");
    let hull = loaded.shadow_hulls[0].clone().expect("an authored hull");
    let mut race = race::Race::start(loaded.setup);
    race.set_autopilot(true);
    for _ in 0..warm {
        race.tick(&PlayerInputs::none());
    }
    let mut out = Window {
        deepest: 0.0,
        ticks_buried: 0,
        ticks: 0,
    };
    for _ in 0..WINDOW {
        race.tick(&PlayerInputs::none());
        let placements = race.shadow_placements();
        let Some(p) = placements.iter().find(|p| p.silhouette == 0) else {
            continue;
        };
        let mut v = Vec::new();
        oag_render::shadow::hull_triangles(
            &oag_render::shadow::Cast {
                hull: &hull,
                model: race.ship_model_matrix_of(0),
                axis: Vec3::from_array(oag_pulse::shadow::AUTHORED_AXIS),
                contact: p.contact,
                normal: p.normal,
                strength: p.strength,
            },
            &mut v,
        );
        if conform {
            oag_render::shadow::conform_to_floor(&mut v, 0, |at, normal| {
                race.floor_above(at, normal)
            });
        }
        let mut points: Vec<Vec3> = v.iter().map(|x| Vec3::from_array(x.position)).collect();
        points.extend(
            v.as_chunks::<3>()
                .0
                .iter()
                .map(|t| t.iter().map(|x| Vec3::from_array(x.position)).sum::<Vec3>() / 3.0),
        );
        let mut deepest = 0.0f32;
        for at in points {
            // The floor above a point, measured along the polygon's own normal.
            if let Some(above) = race.floor_above(at, p.normal) {
                deepest = deepest.max(above);
            }
        }
        out.ticks += 1;
        out.deepest = out.deepest.max(deepest);
        if deepest > CLEAR_ENOUGH {
            out.ticks_buried += 1;
        }
    }
    Some(out)
}

/// The shadow polygon stays clear of the road it is drawn on, tick after tick.
///
/// Before [`oag_render::shadow::conform_to_floor`] existed this measured the
/// polygon up to 0.6 units under the road in Zone late in a lap, on several
/// ticks in a minute; `docs/rendering/shadows.md` has the table.
#[test]
#[ignore = "needs a real disc image under data/images/"]
fn the_shadow_polygon_is_not_buried_under_the_road() {
    let mut raw_worst = 0.0f32;
    for warm in WARM {
        let Some(raw) = measure(oag_race::Mode::Zone, warm, false) else {
            return;
        };
        let fit = measure(oag_race::Mode::Zone, warm, true).expect("same image");
        println!(
            "zone @{warm}: raw deepest {:.3} buried on {}/{} ticks; conformed deepest {:.3} buried on {}/{}",
            raw.deepest, raw.ticks_buried, raw.ticks, fit.deepest, fit.ticks_buried, fit.ticks
        );
        raw_worst = raw_worst.max(raw.deepest);
        assert!(fit.ticks > 0, "no shadow in the window at {warm}");
        assert!(
            fit.deepest <= CLEAR_ENOUGH,
            "zone @{warm}: the conformed polygon is {:.3} under the road, past {CLEAR_ENOUGH}",
            fit.deepest,
        );
    }
    // For contrast only, nothing asserted: the same circuit as an ordinary
    // race, whose craft is slower over the same stretches.
    for warm in WARM {
        let Some(raw) = measure(oag_race::Mode::SingleRace, warm, false) else {
            return;
        };
        println!(
            "single race @{warm}: raw deepest {:.3} buried on {}/{} ticks",
            raw.deepest, raw.ticks_buried, raw.ticks
        );
    }
    assert!(
        raw_worst > 0.3,
        "the unconformed polygon went only {raw_worst:.3} under the road: the probe has lost the \
         sensitivity this test is for"
    );
}
