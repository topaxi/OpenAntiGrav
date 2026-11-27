//! The near-camera law reaches a race's particles: the load puts it on a title's effects and
//! the frame's build reads the camera (`docs/ghidra/functions/ps3-hdfury-eu/particle-render.md`).

use super::*;
use oag_fx::psys::{Cull, NearLaw};
use oag_mesh::mesh::GpuVertex;
use oag_title::{Origin, Trigger};

const ONE_TO_FOUR: NearLaw = NearLaw {
    cull_depth: 1.0,
    fade_depth: 4.0,
    template_cap: None,
};

/// HD's law: its sprite templates are capped at twice their depth
/// (`docs/ghidra/functions/ps3-hdfury-eu/rocket-craft-blast.md`, "The screen wash").
const HD_LAW: NearLaw = NearLaw {
    template_cap: Some(2.0),
    ..ONE_TO_FOUR
};

/// HD's, 2048's and Omega's builders run it, each read in its own executable; Pulse's and
/// Pure's are unread.
#[test]
fn only_hd_2048_and_omega_run_the_near_law() {
    for title in [oag_hd::TITLE, oag_2048::TITLE, oag_omega::TITLE] {
        let law = title.effects.near_law.expect(title.name);
        assert_eq!(
            (law.cull_depth, law.fade_depth),
            (1.0, 4.0),
            "{}",
            title.name
        );
        assert_eq!(law.origin, Origin::Measured, "{}", title.name);
    }
    assert_eq!(
        oag_hd::TITLE.effects.near_law.unwrap().template_cap,
        Some((2.0, Origin::Measured))
    );
    for title in [oag_2048::TITLE, oag_omega::TITLE] {
        assert_eq!(
            title.effects.near_law.unwrap().template_cap,
            Some((2.0, Origin::InheritedFrom("Wipeout HD"))),
            "{}",
            title.name
        );
        assert_eq!(
            title.effects.templates,
            [oag_title::engine_effects::CRAFT_BLAST_EFFECT],
            "{}",
            title.name
        );
    }
    for title in [oag_pulse::TITLE, oag_pure::TITLE] {
        assert!(title.effects.near_law.is_none(), "{}", title.name);
    }
}

/// The PSP's near tests and screen range (`particle-system.md`, "near the eye") run on
/// Pulse's PSP and PS2 pressings and on Pure, and on no title with a near law of its own.
#[test]
fn only_pulse_and_pure_run_the_psp_particle_clip() {
    use oag_title::Platform;
    let pulse = oag_pulse::TITLE.looks.psp_particle_clip;
    assert!(pulse.applies(Platform::Psp) && pulse.applies(Platform::Ps2));
    assert_eq!(pulse.origin, Origin::Measured);
    let pure = oag_pure::TITLE.looks.psp_particle_clip;
    assert!(pure.applies(Platform::Psp));
    assert_eq!(pure.origin, Origin::InheritedFrom("Wipeout Pulse (PSP)"));
    for title in [oag_hd::TITLE, oag_2048::TITLE, oag_omega::TITLE] {
        assert_eq!(
            title.looks.psp_particle_clip,
            oag_title::Rule::UNREAD,
            "{}",
            title.name
        );
    }
}

/// Split screen: the law's eye is the view being drawn, not the player's.
#[test]
fn each_split_view_gives_the_law_its_own_eye() {
    let mut race = race_with_a_grid();
    for _ in 0..30 {
        race.tick(&oag_gameplay::PlayerInputs::none());
    }
    let eye = |race: &Race| race.near_cull().near.expect("an eye").eye;
    let own = eye(&race);
    assert_eq!(race.set_viewport_count(4), 4);
    race.view.set_viewpoint(&race.core, 2);
    let other = eye(&race);
    assert_eq!(other, race.view.camera_position(&race.core));
    assert!((other - own).length() > 0.1, "{other} against {own}");
    race.view.reset_viewpoint();
    assert_eq!(eye(&race), own);
}

fn race_on(image: &str) -> Option<Race> {
    race_on_track(image, None)
}

fn race_on_track(image: &str, track: Option<&str>) -> Option<Race> {
    let image = oag_testdata::exact(image)?;
    let loaded = crate::load(&crate::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: Mode::TimeTrial,
        track: track.map(str::to_string),
        ..crate::Options::default()
    })
    .expect("loading the race");
    let mut race = Race::start(loaded.setup, loaded.view, loaded.modules);
    race.tick(&oag_gameplay::PlayerInputs::none());
    Some(race)
}

/// A quad's centre depth along `forward` from `eye`, and its farthest corner from that centre.
fn depth_and_reach(quad: &[GpuVertex], eye: Vec3, forward: Vec3) -> (f32, f32) {
    let centre = quad.iter().map(|v| Vec3::from(v.position)).sum::<Vec3>() / quad.len() as f32;
    let reach = quad
        .iter()
        .map(|v| (Vec3::from(v.position) - centre).length())
        .fold(0.0, f32::max);
    ((centre - eye).dot(forward), reach)
}

/// HD's Rocket blast on a craft, played three units ahead of the lens: the frame's build drops,
/// and caps, the smoke the file alone would draw several times its own distance wide.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hds_craft_blast_near_the_camera_is_capped_by_its_depth() {
    let Some(mut race) = race_on("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let blast = race
        .view
        .handles
        .get(Trigger::CraftBlast)
        .cloned()
        .expect("HD loads its craft blast");
    assert_eq!(blast.near, Some(HD_LAW));
    assert_eq!(blast.emitters[0].near_cap, Some(0.5));

    let frame = race.view.camera_frame(&race.core);
    let (eye, forward) = (frame.position, -frame.back);
    race.view
        .stage
        .play(&blast, eye + forward * 3.0, 1.0)
        .expect("a free instance");
    for _ in 0..8 {
        race.view
            .stage
            .advance(1.0 / 60.0, &mut race.view.stage_rng);
    }
    let view = race.view.view(&race.core);
    let right = Vec3::new(view.x_axis.x, view.y_axis.x, view.z_axis.x);
    let up = Vec3::new(view.x_axis.y, view.y_axis.y, view.z_axis.y);

    let (mut additive, mut lawful) = (Vec::new(), Vec::new());
    race.extend_stage_vertices(&mut additive, &mut lawful, right, up);
    let (mut other, mut authored) = (Vec::new(), Vec::new());
    race.view
        .stage
        .extend_vertices(&mut other, &mut authored, right, up, Cull::NONE);

    // Every alpha-over emitter of the blast is a billboard capped at its depth or less; a square
    // of half size `depth` reaches `depth * sqrt(2)` to its corners.
    let widest = |quads: &[GpuVertex]| {
        quads
            .as_chunks::<6>()
            .0
            .iter()
            .map(|quad| {
                let (depth, reach) = depth_and_reach(quad, eye, forward);
                reach / depth.max(1e-3)
            })
            .fold(0.0, f32::max)
    };
    assert!(
        widest(&authored) > 2.0,
        "the blast fills the lens as authored: {}",
        widest(&authored)
    );
    assert!(!lawful.is_empty(), "the smoke beyond the cull still draws");
    for quad in lawful.as_chunks::<6>().0 {
        let (depth, reach) = depth_and_reach(quad, eye, forward);
        assert!(depth >= 1.0 - 1e-3, "a quad nearer than the cull: {depth}");
        assert!(
            reach <= depth * 1.5,
            "a quad past its cap: {reach} at {depth}"
        );
    }
}

/// HD's craft blast plays its root templates, the original's screen wash: `glow_longer` on the
/// root and a `Glow` each on `FIRE_TRAILS_ONEQUAD` and `MOLTEN_DEBRIS`, each emitting its one
/// particle and under the template cap
/// (`docs/ghidra/functions/ps3-hdfury-eu/rocket-craft-blast.md`, "The screen wash").
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hds_craft_blast_plays_the_screen_wash_templates() {
    let Some(race) = race_on("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let blast = race
        .view
        .handles
        .get(Trigger::CraftBlast)
        .expect("HD loads its craft blast");
    let playing: Vec<&str> = blast
        .emitters
        .iter()
        .filter(|spec| spec.template && spec.per_emission != (0, 0))
        .map(|spec| spec.name.as_str())
        .collect();
    assert_eq!(playing, ["glow_longer", "Glow", "Glow"]);
    assert_eq!(blast.near.and_then(|law| law.template_cap), Some(2.0));
    check_craft_blast_trails_start_on_the_hemisphere(blast);
}

/// HD's track blast plays its one root template, the `dc_flare_d` quad the original draws on a
/// wall hit (`docs/rendering/particle-fill.md`, "A Rocket on the wall paired").
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn hds_track_blast_plays_its_flare_template() {
    let Some(race) = race_on("data/images/hdfury-ps3-eu-dec.iso") else {
        return;
    };
    let blast = race
        .view
        .handles
        .get(Trigger::TrackBlast)
        .expect("HD loads its track blast");
    let playing: Vec<&str> = blast
        .emitters
        .iter()
        .filter(|spec| spec.template && spec.per_emission != (0, 0))
        .map(|spec| {
            assert!(spec.sprite.is_some(), "{}", spec.name);
            spec.name.as_str()
        })
        .collect();
    assert_eq!(playing, ["FLASHBANG"]);
}

/// HD's craft blast spreads by the extent law: `FIRE_TRAILS_ONEQUAD`, the heads whose `trail`
/// smoke the original draws fanning out around the blast, start on the authored 8.824-unit
/// hemisphere (`docs/rendering/particle-fill.md`, "The trails start on the blast's hemisphere").
fn check_craft_blast_trails_start_on_the_hemisphere(blast: &oag_fx::psys::Effect) {
    use oag_fx::psys::spawn::Spawn;
    let heads = blast
        .emitters
        .iter()
        .find(|spec| spec.name == "FIRE_TRAILS_ONEQUAD")
        .expect("the craft blast authors its trail heads");
    let Spawn::Sphere { extent, mode, .. } = heads.spawn else {
        panic!("the trail heads spawn at the anchor: {:?}", heads.spawn);
    };
    assert!((extent - 8.824).abs() < 1e-3, "extent {extent}");
    assert_eq!(mode, 0, "exactly on the radius");
}

/// The same on 2048's and Omega's own loads: their craft blast carries the law, HD's template
/// cap and an authored cap, and plays the templates of the title's own file, `want`.
fn craft_blast_loads_under_the_law(source: &str, track: Option<&str>, want: &[&str]) {
    let Some(race) = race_on_track(source, track) else {
        return;
    };
    let blast = race
        .view
        .handles
        .get(Trigger::CraftBlast)
        .expect("the title loads its craft blast");
    assert_eq!(blast.near, Some(HD_LAW));
    let playing: Vec<&str> = blast
        .emitters
        .iter()
        .filter(|spec| spec.template && spec.per_emission != (0, 0))
        .map(|spec| spec.name.as_str())
        .collect();
    assert_eq!(playing, want, "{source}");
    // 2048's and Omega's own updates are not wired: every particle keeps its spawn frame and
    // draws its size channel alone.
    for spec in &blast.emitters {
        assert_eq!(
            spec.frames,
            oag_fx::psys::frames::FrameAdvance::Still,
            "{source} {}",
            spec.name
        );
        assert_eq!(spec.frame_range, None, "{source} {}", spec.name);
        assert_eq!(spec.size_ramp, None, "{source} {}", spec.name);
    }
    assert!(
        blast
            .emitters
            .iter()
            .any(|e| e.near_cap.is_some_and(|cap| cap > 0.0 && cap <= 4.0))
    );
}

#[test]
#[ignore = "needs data/extracted/vita/PCSF00007"]
fn the_2048_craft_blast_loads_under_the_near_law() {
    // 2048's own `particles2048` file: its one template, not HD's.
    craft_blast_loads_under_the_law("data/extracted/vita/PCSF00007", None, &["BANG_GLOW"]);
}

#[test]
#[ignore = "needs data/images/omega-ps4-eu.pkg"]
fn the_omega_craft_blast_loads_under_the_near_law() {
    // An HD circuit reads `Data\particles`, whose blast authors no template; a 2048 circuit
    // reads `Data\particles2048`, which authors HD's two.
    craft_blast_loads_under_the_law("data/images/omega-ps4-eu.pkg", None, &[]);
    craft_blast_loads_under_the_law(
        "data/images/omega-ps4-eu.pkg",
        Some(r"Data\environments2048\altima\track.vex"),
        &["glow_longer", "Glow"],
    );
}

/// Pulse's effects carry no law.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn pulses_effects_load_without_the_near_law() {
    let Some(race) = race_on("data/images/pulse-psp-eu.chd") else {
        return;
    };
    let blast = race
        .view
        .handles
        .get(Trigger::CraftBlast)
        .expect("Pulse loads its craft blast");
    assert_eq!(blast.near, None);
}
