//! The trail-hit burst lent to Wipeout Pulse (`graphics.trail_hits`), off real disc images:
//! Pulse's race and Wipeout HD's `WO_TRAIL_HITSHIP.POB`.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this project does not ship.
//!
//! ```sh
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-raceplay --run-ignored all trail_hits_lent
//! ```
//!
//! Pulse authors no `WO_TRAIL_HITSHIP` (`docs/formats/pulse-absent-effects.md`). With the setting
//! on and a source that authors it mounted, a craft posed inside a rival's engine trail plays
//! the burst; with the setting off it does not and the loader says nothing of it; on with no
//! source, nothing plays and the report names why.

use oag_raceplay as race;

const PULSE: &str = "data/images/pulse-psp-eu.chd";
const PS2: &str = "data/images/pulse-ps2-eu.chd";
const HD: &str = "data/images/hdfury-ps3-eu-dec.iso";

fn load(trail_hits: race::TrailHits) -> Option<race::Loaded> {
    load_from(PULSE, trail_hits)
}

fn load_from(source: &str, trail_hits: race::TrailHits) -> Option<race::Loaded> {
    let pulse = oag_testdata::image(source)?;
    let loaded = race::load(&race::Options {
        source: pulse.display().to_string(),
        mode: oag_race::Mode::SingleRace,
        trail_hits,
        ..race::Options::default()
    })
    .expect("loading the Pulse race");
    for line in &loaded.report {
        println!("{line}");
    }
    Some(loaded)
}

fn lent_from_hd() -> Option<race::TrailHits> {
    let hd = oag_testdata::image(HD)?;
    Some(race::TrailHits::On {
        lender: Some(hd.display().to_string()),
    })
}

/// Slot 1 posed eight units behind slot 0 on slot 0's heading, both flying 60 units a second,
/// and the exhaust and trail-hit history of that flight replayed - `--field-warmup`'s path.
fn tucked_in_a_rivals_trail(loaded: race::Loaded) -> race::Race {
    let mut race = race::Race::start(loaded.setup, loaded.view, loaded.modules);
    assert!(
        !race.hd_trail_active(),
        "Pulse draws the PSP ribbon, not HD's tube"
    );
    let lead = race.core.sim.world.ships[0].physics.body;
    let forward = lead.forward();
    race.core.sim.world.ships[1].physics.body.orientation = lead.orientation;
    race.core.sim.world.ships[1].physics.body.position = lead.position - forward * 8.0;
    let velocities = [forward * 60.0; 2];
    let ticks = race.core.sim.field_warm_up_ticks();
    race.warm_up_field(&velocities, ticks, None);
    race
}

/// The PS2 leaves the ribbon's deep blue layer out, so its burst is pink-white where the PSP's is
/// violet.
#[test]
#[ignore = "needs data/images/pulse-ps2-eu.chd and data/images/hdfury-ps3-eu-dec.iso"]
fn trail_hits_lent_on_the_ps2_takes_its_own_ribbons_hue() {
    let Some(on) = lent_from_hd() else { return };
    let Some(loaded) = load_from(PS2, on) else {
        return;
    };
    assert_burst_is_in_the_ribbons_hue(&loaded, oag_fx::exhaust::TrailLaw::Baked);
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd and data/images/hdfury-ps3-eu-dec.iso"]
fn trail_hits_lent_on_pulse_a_craft_tucked_in_a_rivals_trail_plays_hds_burst() {
    let Some(on) = lent_from_hd() else { return };
    let Some(loaded) = load(on) else { return };
    let line = |needle: &str| loaded.report.iter().find(|l| l.contains(needle)).cloned();
    let burst = line("WO_TRAIL_HITSHIP: ").expect("the loader said nothing of the lent effect");
    assert!(
        burst.contains("emitter(s)") && !burst.contains("will not be drawn"),
        "{burst}"
    );
    let lent = line("trail hits: ").expect("the report names the lending");
    assert!(lent.contains("Wipeout HD"), "{lent}");
    assert_burst_is_in_the_ribbons_hue(&loaded, oag_fx::exhaust::TrailLaw::Ramped);
    let race = tucked_in_a_rivals_trail(loaded);
    assert!(
        race.stage().playing_count() > 0,
        "a craft eight units inside a rival's ribbon played no burst"
    );
}

/// The unit chroma (colour minus its Rec. 709 luma) of `rgb`, and its length.
fn chroma(rgb: [f32; 3]) -> ([f32; 3], f32) {
    let y = rgb[0] * 0.2126 + rgb[1] * 0.7152 + rgb[2] * 0.0722;
    let c = [rgb[0] - y, rgb[1] - y, rgb[2] - y];
    let length = c.iter().map(|v| v * v).sum::<f32>().sqrt();
    (c.map(|v| v / length.max(1e-9)), length)
}

/// **The lent burst is drawn in the colour of the ribbon it was struck in, not HD's cyan-blue**:
/// every emitter of the real `WO_TRAIL_HITSHIP`, sprite times colour table, points the way the
/// summed layers of the platform's ribbon do (`docs/formats/pulse-absent-effects.md`, "Trail-hit
/// burst colour"). The vertex colour alone does not say what is drawn: the sprite texel multiplies
/// it.
fn assert_burst_is_in_the_ribbons_hue(loaded: &race::Loaded, law: oag_fx::exhaust::TrailLaw) {
    let effect = loaded
        .view
        .handles
        .get(oag_title::Trigger::TrailHitship)
        .expect("the lent burst is loaded");
    let (target, _) = chroma(law.ribbon_colour());
    assert!(!effect.emitters.is_empty());
    for emitter in &effect.emitters {
        let sprite = emitter.sprite.as_ref().expect("HD authors both sprites");
        let (mut texel, mut weight) = ([0.0f32; 3], 0.0f32);
        for t in sprite.rgba.as_chunks::<4>().0 {
            let alpha = f32::from(t[3]);
            for c in 0..3 {
                texel[c] += f32::from(t[c]) * alpha;
            }
            weight += alpha;
        }
        let mut table = [0.0f32; 3];
        for entry in emitter.palette.iter() {
            for c in 0..3 {
                table[c] += entry[c];
            }
        }
        let drawn = std::array::from_fn(|c| texel[c] / weight.max(1.0) * table[c] / 256.0 / 255.0);
        let (direction, length) = chroma(drawn);
        assert!(length > 0.01, "{}: drawn grey {drawn:?}", emitter.name);
        let dot: f32 = (0..3).map(|c| direction[c] * target[c]).sum();
        assert!(
            dot > 0.99,
            "{}: drawn {drawn:?} is not the ribbon's hue (dot {dot})",
            emitter.name
        );
    }
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn trail_hits_lent_off_pulse_plays_no_burst_and_loads_no_effect() {
    let Some(loaded) = load(race::TrailHits::Off) else {
        return;
    };
    assert!(
        loaded
            .report
            .iter()
            .all(|l| !l.contains("WO_TRAIL_HITSHIP") && !l.contains("trail hits")),
        "the setting off added to the report: {:?}",
        loaded
            .report
            .iter()
            .filter(|l| l.contains("TRAIL_HITSHIP") || l.contains("trail hits"))
            .collect::<Vec<_>>()
    );
    let race = tucked_in_a_rivals_trail(loaded);
    assert_eq!(race.stage().playing_count(), 0);
}

#[test]
#[ignore = "needs data/images/pulse-psp-eu.chd"]
fn trail_hits_lent_with_no_source_plays_nothing_and_the_report_names_why() {
    let Some(loaded) = load(race::TrailHits::On { lender: None }) else {
        return;
    };
    let why = loaded
        .report
        .iter()
        .find(|l| l.contains("trail hits: "))
        .expect("the report is silent about a switched-on setting that found no source");
    assert!(
        why.contains("no source that authors WO_TRAIL_HITSHIP"),
        "{why}"
    );
    let race = tucked_in_a_rivals_trail(loaded);
    assert_eq!(race.stage().playing_count(), 0);
}
