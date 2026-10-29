//! The enhanced weather's splashes on Pulse's two weather circuits
//! (`oag_raceplay::scenery_fx::splash`): this project's own, so the claim is
//! about what `original` leaves alone and what `enhanced` adds, not about a
//! measurement of the original.
//!
//! Drop the wiring (`Race::set_weather`, the cast in `advance_splashes`, the
//! draw hook, the loader's art placement) and `enhanced` throws nothing.

use oag_core::math::Vec3;
use oag_display::display::Weather;
use oag_gameplay::PlayerInputs;
use oag_mesh::mesh::GpuVertex;
use oag_raceplay as race;

fn started(track: &str) -> Option<race::Race> {
    let image = oag_testdata::image("pulse-psp-usa.chd")?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::TimeTrial,
        track: Some(format!(r"Data\Environments\{track}")),
        ..race::Options::default()
    })
    .expect("loading the race");
    let mut race = race::Race::start(loaded.setup);
    race.set_autopilot(true);
    Some(race)
}

fn lists(race: &race::Race) -> (Vec<GpuVertex>, Vec<GpuVertex>) {
    let (mut additive, mut alpha) = (Vec::new(), Vec::new());
    race.extend_stage_vertices(&mut additive, &mut alpha, Vec3::X, Vec3::Y);
    (additive, alpha)
}

fn bytes(vertices: &[GpuVertex]) -> &[u8] {
    bytemuck::cast_slice(vertices)
}

struct Census {
    ticks_with_splashes: usize,
    most_at_once: usize,
    spawned_points: usize,
    /// The most the wet road reached at `enhanced`, which `original` must never leave `0.0`.
    wettest: f32,
}

/// Runs an original race and an enhanced one in step and checks, every tick,
/// that the enhanced draw lists begin with the original's byte for byte and
/// that each splash lies on a floor.
fn compare(track: &str, ticks: u32) -> Option<Census> {
    let mut original = started(track)?;
    let mut enhanced = started(track)?;
    original.set_weather(Weather::Original);
    enhanced.set_weather(Weather::Enhanced);
    let mut census = Census {
        ticks_with_splashes: 0,
        most_at_once: 0,
        spawned_points: 0,
        wettest: 0.0,
    };
    for tick in 0..ticks {
        original.tick(&PlayerInputs::none());
        enhanced.tick(&PlayerInputs::none());
        assert_eq!(
            original.sim.state_hash(),
            enhanced.sim.state_hash(),
            "the setting is render-side: the simulation must not notice it"
        );
        assert!(original.scenery_fx().weather().splashes().is_empty());
        assert_eq!(
            original.wet_road(),
            0.0,
            "the original's road is dry, whatever the weather"
        );
        census.wettest = census.wettest.max(enhanced.wet_road());
        let (o_add, o_alpha) = lists(&original);
        let (e_add, e_alpha) = lists(&enhanced);
        assert!(
            bytes(&e_add).starts_with(bytes(&o_add))
                && bytes(&e_alpha).starts_with(bytes(&o_alpha)),
            "enhanced only appends to the original's geometry"
        );
        let splashes = enhanced.scenery_fx().weather().splashes();
        if splashes.is_empty() {
            assert_eq!(e_add.len(), o_add.len());
            let spray = enhanced.scenery_fx().weather().spray().is_empty();
            assert!(spray || e_alpha.len() > o_alpha.len());
            assert!(spray || enhanced.wet_road() > 0.0, "spray on a dry road");
            continue;
        }
        assert!(e_add.len() > o_add.len(), "a live splash draws");
        census.ticks_with_splashes += 1;
        if std::env::var_os("OAG_SWEEP").is_some() && splashes.len() >= 4 {
            eprintln!("tick {tick}: {} splashes", splashes.len());
        }
        census.most_at_once = census.most_at_once.max(splashes.len());
        for point in splashes.points() {
            census.spawned_points += 1;
            assert!(
                !enhanced.point_is_under_cover(point),
                "{point:?} is a splash under a roof"
            );
            let above = enhanced
                .floor_above(point, Vec3::Y)
                .or_else(|| enhanced.floor_above(point, -Vec3::Y));
            assert!(above.is_some(), "{point:?} is not on a floor");
        }
    }
    Some(census)
}

/// Fort Gale's rain: 32 drops, a splash where one crosses the road.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn rain_splashes_on_fort_gales_road_and_original_draws_none() {
    let Some(census) = compare(r"14_Track\track.vex", 1500) else {
        return;
    };
    assert!(
        census.ticks_with_splashes > 100,
        "{} ticks with a splash in 1500",
        census.ticks_with_splashes
    );
    assert!(
        census.most_at_once <= race::scenery_fx::splash::CAPACITY,
        "{} at once",
        census.most_at_once
    );
    assert!(
        census.wettest > 0.3,
        "Fort Gale's rain wets the road at enhanced: wettest {}",
        census.wettest
    );
    eprintln!(
        "Fort Gale: splashes on {} of 1500 ticks, at most {} at once",
        census.ticks_with_splashes, census.most_at_once
    );
}

/// The cover check is per hit, whatever the camera's section: a spline sample
/// of a covered section is under cover, one of an open section is not, and the
/// wet road is not zeroed because the weather is anchored.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn a_hit_in_a_covered_section_is_rejected_and_an_open_one_is_not() {
    let Some(image) = oag_testdata::image("pulse-psp-usa.chd") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::TimeTrial,
        track: Some(r"Data\Environments\14_Track\track.vex".to_string()),
        ..race::Options::default()
    })
    .expect("loading the race");
    let spline = loaded.setup.spline.clone();
    let mut race = race::Race::start(loaded.setup);
    race.set_weather(Weather::Enhanced);
    let (mut covered, mut open) = (0, 0);
    for index in 0..spline.len() {
        let Some(sample) = spline.sample(index) else {
            continue;
        };
        let point = Vec3::from_array(sample.pos);
        let under = race
            .scenery_fx()
            .weather()
            .section_is_covered(i32::from(sample.section_id));
        // A sample can sit near a stacked stretch of another section; judge
        // only where the nearest sample is this one's own section.
        let Some((nearest, _, _)) = spline.nearest(point) else {
            continue;
        };
        if spline.sample(nearest).map(|s| s.section_id) != Some(sample.section_id) {
            continue;
        }
        assert_eq!(race.point_is_under_cover(point), under, "sample {index}");
        if under {
            covered += 1;
        } else {
            open += 1;
        }
    }
    assert!(
        covered > 100 && open > 100,
        "{covered} covered, {open} open"
    );
}

/// The road of a covered section is dry for good (cleared at load), so the
/// open road seen from inside a tunnel is wet and the wet road does not
/// depend on the camera's section: after a minute of rain it is full, at any
/// tick, anchored or not.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_road_under_cover_is_dry_at_load_and_the_wetness_ignores_the_camera() {
    let image = oag_testdata::image("pulse-psp-usa.chd");
    let Some(image) = image else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::TimeTrial,
        track: Some(r"Data\Environments\14_Track\track.vex".to_string()),
        ..race::Options::default()
    })
    .expect("loading the race");
    let line = loaded
        .report
        .iter()
        .find(|line| line.contains("road vertices under cover kept dry"))
        .expect("the loader says how many it dried");
    let dried: usize = line
        .trim_start_matches("weather: ")
        .split(' ')
        .next()
        .and_then(|n| n.parse().ok())
        .expect("a count");
    assert!(dried > 1000, "{line}");
    let marked = loaded
        .track_model
        .vertices
        .iter()
        .filter(|v| v.texcoord2[0] == oag_mesh::mesh::ROAD_MARK)
        .count();
    assert!(
        marked > dried,
        "open road stays marked: {marked} vs {dried}"
    );
    eprintln!("Fort Gale: {dried} road vertices dried, {marked} still marked");
}

/// Outpost 7's snow: 64 flakes, one settles where it meets the road.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn snow_settles_on_outpost_7s_road_and_original_draws_none() {
    let Some(census) = compare(r"07_Track\track.vex", 1500) else {
        return;
    };
    assert!(
        census.ticks_with_splashes > 100,
        "{} ticks with a flake in 1500",
        census.ticks_with_splashes
    );
    assert_eq!(census.wettest, 0.0, "snow leaves the road dry");
    eprintln!(
        "Outpost 7: flakes on {} of 1500 ticks, at most {} at once",
        census.ticks_with_splashes, census.most_at_once
    );
}

/// A scratch sweep, not a check: the milliseconds a tick and a draw-list
/// gather cost at each setting, printed. Needs `OAG_SWEEP`.
#[test]
#[ignore = "a sweep that only prints; needs OAG_SWEEP and data/images/pulse-psp-usa.chd"]
fn splash_cost_sweep() {
    if std::env::var_os("OAG_SWEEP").is_none() {
        return;
    }
    for track in [r"14_Track\track.vex", r"07_Track\track.vex"] {
        for round in 0..2 {
            for setting in [Weather::Original, Weather::Enhanced] {
                let Some(mut race) = started(track) else {
                    return;
                };
                race.set_weather(setting);
                let (mut tick, mut gather) = (std::time::Duration::ZERO, std::time::Duration::ZERO);
                for _ in 0..1500 {
                    let start = std::time::Instant::now();
                    race.tick(&PlayerInputs::none());
                    tick += start.elapsed();
                    let start = std::time::Instant::now();
                    std::hint::black_box(lists(&race));
                    gather += start.elapsed();
                }
                eprintln!(
                    "{track} round {round} {setting}: tick {:.4} ms, gather {:.4} ms (per tick, mean of 1500)",
                    tick.as_secs_f64() * 1000.0 / 1500.0,
                    gather.as_secs_f64() * 1000.0 / 1500.0
                );
            }
        }
    }
}

/// The PS2 pressing plays the same circuit's weather (`circuit_effects`), so it
/// loads the splash art too and inherits the setting from the PSP.
#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd"]
fn the_ps2_pressing_loads_the_splash_art_with_its_weather() {
    let Some(image) = oag_testdata::image("data/images/pulse-ps2-eu.chd") else {
        return;
    };
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::TimeTrial,
        track: Some(r"Data\Environments\14_Track\track.vex".to_string()),
        ..race::Options::default()
    })
    .expect("loading the race");
    let has = |text: &str| loaded.report.iter().any(|line| line.contains(text));
    assert!(
        has("WO_RAIN.POB on 16 covered section(s)"),
        "{}",
        loaded.report.join("\n")
    );
    assert!(has("splash art"), "{}", loaded.report.join("\n"));
    let weather = loaded.setup.scenery_fx.weather.expect("weather");
    assert!(weather.splash_cell.is_some(), "the art did not place");
}

/// The road spray on Fort Gale (`oag_raceplay::scenery_fx::spray`): none at
/// `original` or on a dry road, a mist behind the craft once the road is wet,
/// and none where the road lies in a covered section.
#[test]
#[ignore = "needs data/images/pulse-psp-usa.chd"]
fn the_craft_throws_spray_on_wet_open_road_only() {
    let (Some(mut original), Some(mut enhanced)) = (
        started(r"14_Track\track.vex"),
        started(r"14_Track\track.vex"),
    ) else {
        return;
    };
    original.set_weather(Weather::Original);
    enhanced.set_weather(Weather::Enhanced);
    let (mut sprayed, mut covered_ticks, mut open_emitting, mut peak) = (0, 0, 0, 0);
    let (mut misted, mut mist_peak, mut mist_before) = (0, 0, 0);
    for tick in 0..3200 {
        original.tick(&PlayerInputs::none());
        enhanced.tick(&PlayerInputs::none());
        assert!(original.scenery_fx().weather().spray().is_empty());
        let spray = enhanced.scenery_fx().weather().spray();
        if enhanced.wet_road() == 0.0 {
            assert!(spray.is_empty(), "tick {tick}: spray on a dry road");
        }
        peak = peak.max(spray.len());
        let mist = spray.mist_len();
        mist_peak = mist_peak.max(mist);
        misted += usize::from(mist > 0);
        if !spray.is_empty() {
            sprayed += 1;
            let (o_add, o_alpha) = lists(&original);
            let (e_add, e_alpha) = lists(&enhanced);
            assert!(e_add.len() >= o_add.len());
            assert!(
                e_alpha.len() > o_alpha.len(),
                "spray draws: tick {tick} n={} {} vs {} cell {:?}",
                spray.len(),
                e_alpha.len(),
                o_alpha.len(),
                enhanced.scenery_fx().weather().splash_cell()
            );
        }
        let under = enhanced.point_is_under_cover(enhanced.ship_tick_body(0).position);
        let emitters = enhanced.spray_emitters();
        if under {
            covered_ticks += 1;
            assert!(mist <= mist_before, "tick {tick}: mist thrown under a roof");
            assert!(
                emitters.is_empty(),
                "tick {tick}: spray source under a roof"
            );
        } else if !emitters.is_empty() {
            open_emitting += 1;
        }
        mist_before = mist;
    }
    assert!(sprayed > 200, "{sprayed} ticks with spray in 3200");
    assert!(
        covered_ticks > 100 && open_emitting > 500,
        "{covered_ticks} covered, {open_emitting} open"
    );
    assert!(peak <= race::scenery_fx::spray::CAPACITY);
    assert!(misted > 200, "{misted} ticks with mist in 3200");
    assert!(mist_peak <= race::scenery_fx::spray::MIST_CAPACITY);
    eprintln!(
        "spray on {sprayed} ticks, peak {peak}, {covered_ticks} covered ticks, {open_emitting} open emitting"
    );
}
