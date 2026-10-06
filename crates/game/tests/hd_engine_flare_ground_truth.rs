//! Wipeout HD's engine flare, off a real disc image: a per-team model, split
//! into an always-on flame and a boost plume by its own node tree.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! # or only this file:
//! OAG_REQUIRE_GAME_DATA=1 cargo nextest run -p oag-game --run-ignored all hd_engine_flare
//! ```
//!
//! # What this pins that a screenshot cannot
//!
//! The flame draws into the same few hundred pixels the exhaust ribbon already
//! saturates, so "it appeared" is not readable off a capture - which is exactly
//! the failure mode the load report exists to remove. What is assertable is
//! the chain the picture rests on: that every craft on the grid ships the pair,
//! that it builds, that its one material is the shared additive `flame_test`,
//! and that the node tree really does divide it into the two groups the title
//! package names.
//!
//! It also pins the finding that reopened the plume question. A sweep for a
//! `shipboost.vex` on this disc comes back empty and used to read as "HD has no
//! boost plume"; the plume is the `EF_Boost` subtree asserted here.

use std::path::PathBuf;

use oag_gameplay::PlayerInputs;
use oag_raceplay as race;

fn image() -> Option<PathBuf> {
    oag_testdata::image("data/images/hdfury-ps3-eu-dec.iso")
}

fn load() -> Option<race::Loaded> {
    let image = image()?;
    let loaded = race::load(&race::Options {
        source: image.display().to_string(),
        class: "VENOM".to_string(),
        mode: oag_race::Mode::SingleRace,
        opponent_teams: Vec::new(),
        ..race::Options::default()
    })
    .expect("loading the race");
    for line in &loaded.report {
        println!("{line}");
    }
    Some(loaded)
}

/// Every craft on the grid draws its own flame and its own plume.
///
/// **Both halves, per craft.** One team's flare on another's hull would be the
/// kind of wrong that renders plausibly, and the models genuinely differ - 350
/// to 961 triangles for `EF_Main` across the eight, and 616 to 1,492 for
/// `EF_Boost`.
#[test]
#[ignore = "needs a disc image"]
fn every_craft_on_the_grid_carries_both_halves_of_its_flare() {
    let Some(loaded) = load() else {
        return;
    };
    for livery in &loaded.liveries {
        let flame = livery
            .flare
            .as_ref()
            .unwrap_or_else(|| panic!("{}: no engine flare model", livery.team));
        let plume = livery
            .boost
            .as_ref()
            .unwrap_or_else(|| panic!("{}: no boost plume - EF_Boost is missing", livery.team));
        assert!(
            !flame.indices.is_empty() && !plume.indices.is_empty(),
            "{}: {} flame indices, {} plume indices",
            livery.team,
            flame.indices.len(),
            plume.indices.len()
        );
        // **Both halves index the one shared buffer pair**, which is what says
        // the split is a filter over draw calls rather than two builds.
        assert_eq!(
            flame.vertices.len(),
            plume.vertices.len(),
            "{}: the two groups are two views of one model",
            livery.team
        );
        // Additive, off the material rather than off a constant here: every
        // batch carries the factor pair `flame_test.rcsmaterial` authors.
        let blend = flame
            .transparent_draws
            .first()
            .and_then(|draw| draw.blend_state)
            .unwrap_or_else(|| panic!("{}: the flame authors no blend", livery.team));
        assert_eq!(
            blend.color.src_factor,
            wgpu::BlendFactor::SrcAlpha,
            "{}: src factor",
            livery.team
        );
        assert_eq!(
            blend.color.dst_factor,
            wgpu::BlendFactor::One,
            "{}: dst factor",
            livery.team
        );
    }
}

/// The flame is placed at the craft's own `Engine Flare` locator, not at the
/// hull's origin.
///
/// The model is authored about its own origin - every transform in the file is
/// the identity - so a flare that was *not* moved would sit in the middle of
/// the fuselage, which is a picture that reads as "no flare" rather than as a
/// misplaced one. Asserted against the locator the same report names.
#[test]
#[ignore = "needs a disc image"]
fn the_flame_sits_at_the_nozzle_rather_than_at_the_hulls_origin() {
    let Some(loaded) = load() else {
        return;
    };
    for livery in &loaded.liveries {
        let (Some(flame), Some(nozzle)) = (livery.flare.as_ref(), livery.nozzle) else {
            panic!("{}: no flare or no nozzle", livery.team);
        };
        // Well behind the hull's origin on every HD craft: the locators run
        // from z = -4.9 to -8.0. The flame's own centre is within its radius of
        // the nozzle, which is the statement "it was moved there".
        assert!(
            nozzle.z < -1.0,
            "{}: the nozzle locator is at {nozzle:?}",
            livery.team
        );
        let centre = oag_core::math::Vec3::from_array(flame.centre);
        assert!(
            (centre - nozzle).length() < flame.radius,
            "{}: flame centred at {centre:?}, nozzle at {nozzle:?}, radius {:.2}",
            livery.team,
            flame.radius
        );
    }
}

/// The colour set's fourth byte reaches the vertex **alpha**, which is where
/// `flame_test`'s own fragment program reads it.
///
/// Its last two instructions are `MOV H0.w, f[TC2]` and `MUL H0.w, H1, H0`,
/// and the paired vertex program writes `o[TC2] = (world position, v[3].w)`
/// with `v[3]` named `VertexColour1` by its hash. So the byte is the flame's
/// opacity ramp and not the sun-occlusion mask `mesh/rcs.rs` files it as for
/// the circuit materials that reading was measured on. See
/// `oag_livery::flare::alpha_ramp` and
/// `docs/ghidra/functions/ps3-hdfury-eu/engine-flare.md`.
#[test]
#[ignore = "needs a disc image"]
fn the_flame_carries_an_opacity_ramp_in_its_vertex_alpha() {
    let Some(loaded) = load() else {
        return;
    };
    for livery in &loaded.liveries {
        // **Both halves.** The two groups share one buffer pair, so a ramp
        // asserted on the flame alone would be asserted on the plume's
        // vertices too without saying so - but the ramp is per node, and the
        // plume's own nodes carry their own: `ef_BoostLeftShape` spans 0.000 to
        // 1.000 and `ef_DiamondsShape` 0.000 to 0.596 on Feisar.
        for (what, model) in [("flame", &livery.flare), ("plume", &livery.boost)] {
            let Some(model) = model.as_ref() else {
                panic!("{}: no {what}", livery.team);
            };
            let min = model
                .vertices
                .iter()
                .map(|v| v.colour[3])
                .fold(f32::INFINITY, f32::min);
            let max = model
                .vertices
                .iter()
                .map(|v| v.colour[3])
                .fold(f32::NEG_INFINITY, f32::max);
            assert!(
                min <= 0.01 && max >= 0.99,
                "{}: the {what}'s ramp spans {min} to {max}, so it is not a ramp",
                livery.team
            );
            // And the slot it came from is left unmasked, or the generic sun
            // term would be gated by an opacity ramp.
            assert!(
                model.vertices.iter().all(|v| v.sun_mask == 1.0),
                "{}: a {what} vertex still carries a sun mask",
                livery.team
            );
        }
    }
}

/// Both halves of every craft's flare carry the flame's own shading
/// parameters, read off that craft's own material record.
///
/// **Per craft, not shared**, even though all fourteen author the same six
/// numbers: a value that is read is a fact about the data and a value that is
/// assumed is a constant in disguise. The numbers themselves are pinned on the
/// format side by
/// `crates/formats/tests/rcsmodel_material_ground_truth.rs`; what this asserts
/// is that they reach the model the renderer draws.
#[test]
#[ignore = "needs a disc image"]
fn every_flare_carries_the_flames_own_shader_parameters() {
    let Some(loaded) = load() else {
        return;
    };
    for livery in &loaded.liveries {
        for (what, model) in [("flame", &livery.flare), ("plume", &livery.boost)] {
            let Some(model) = model.as_ref() else {
                panic!("{}: no {what}", livery.team);
            };
            let flame = model
                .flame
                .unwrap_or_else(|| panic!("{}: the {what} has no shading parameters", livery.team));
            assert_eq!(flame.rim_power, 10.0, "{}", livery.team);
            assert_eq!(flame.rim_scale, 0.3, "{}", livery.team);
            assert_eq!(flame.rim_min, 0.45, "{}", livery.team);
            assert_eq!(flame.alpha_scale, 2.0, "{}", livery.team);
            assert_eq!(flame.colour_scale, 1.0, "{}", livery.team);
            // `Speed`, the sixth. It multiplies the engine's own seconds clock
            // into the noise tap's `v`, so at 2.0 the surface advances two
            // whole texture repeats a second - the rate a recording of the
            // original falsifies if this chain is wrong.
            assert_eq!(flame.scroll_speed, 2.0, "{}", livery.team);
        }
    }
}

/// The ribbon's two textures, and **which is which**.
///
/// This is the assertion that would have caught a role swap that cost a round:
/// the loader once multiplied the coverage into the *noise* rather than
/// replacing it, and since the noise map's alpha averages 17 of 255 the ribbon
/// nearly vanished. The two are told apart by exactly the property that makes
/// the mistake fatal - one is nearly transparent and the other is not - so a
/// future swap fails here rather than on screen.
///
/// The names agree, the sampler units agree, and HD's own fragment program
/// agrees; see `docs/ghidra/functions/ps3-hdfury-eu/engine-flare.md`.
#[test]
#[ignore = "needs a disc image"]
fn the_ribbons_noise_is_the_near_transparent_one_and_its_coverage_is_not() {
    let Some(loaded) = load() else {
        return;
    };
    let mean = |texture: &oag_fx::exhaust::FlareTexture| {
        let pixels = texture.rgba.as_chunks::<4>().0;
        let total: u64 = pixels.iter().map(|p| u64::from(p[3])).sum();
        total as f64 / pixels.len() as f64
    };
    let noise = loaded.noise.as_ref().expect("the ribbon's noise texture");
    let shape = loaded
        .trail_shape
        .as_ref()
        .expect("the ribbon's coverage texture");
    let (noise_alpha, shape_alpha) = (mean(noise), mean(shape));
    assert!(
        noise_alpha < 40.0,
        "the noise map's alpha averages {noise_alpha:.1}, not the ~17 that says it is \
         the displacement source rather than the coverage"
    );
    assert!(
        shape_alpha > 120.0,
        "the coverage map's alpha averages {shape_alpha:.1}, not the ~162 that says it \
         is the ribbon's own falloff"
    );
    assert!(
        shape_alpha > noise_alpha * 4.0,
        "the two textures are not far enough apart to be sure they are the right way \
         round: noise {noise_alpha:.1}, coverage {shape_alpha:.1}"
    );
}

/// The trail and flame constants `oag_fx::exhaust::hd` carries are the
/// disc's own: `Data/ships/shipeffectstweaks.txt` names the runtime's whole
/// tuning block, field for field, and this reads it rather than trusting the
/// transcription.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_tube_constants_are_the_discs_own_tuning_file() {
    let Some(image) = image() else {
        return;
    };
    let spec = format!("{}:PS3_GAME/USRDIR/DATA02.PSARC", image.display());
    let blob = oag_mesh::mesh::read_blob(&spec, "/data/ships/shipeffectstweaks.txt")
        .expect("the tuning file HD ships");
    let text = String::from_utf8(blob).expect("ascii");
    let value = |key: &str| -> f32 {
        let line = text
            .lines()
            .find(|line| line.contains(key))
            .unwrap_or_else(|| panic!("no {key} in shipeffectstweaks.txt"));
        line.split('=')
            .nth(1)
            .expect("=")
            .trim()
            .parse()
            .expect("float")
    };
    use oag_fx::exhaust::hd;
    assert_eq!(value("Thrust Chase Rate"), hd::THRUST_CHASE_RATE);
    assert_eq!(value("Thrust Min Scale XY"), hd::THRUST_SCALE_XY.0);
    assert_eq!(value("Thrust Max Scale XY"), hd::THRUST_SCALE_XY.1);
    assert_eq!(value("Thrust Min Scale Z"), hd::THRUST_SCALE_Z.0);
    assert_eq!(value("Thrust Max Scale Z"), hd::THRUST_SCALE_Z.1);
    assert_eq!(value("Thrust Extra Boost Scale XY"), hd::BOOST_EXTRA_XY);
    assert_eq!(value("Thrust Extra Boost Scale Z"), hd::BOOST_EXTRA_Z);
    assert_eq!(value("Tex Scroll Speed Delta Min"), hd::SCROLL_DELTA.0);
    assert_eq!(value("Tex Scroll Speed Delta Max"), hd::SCROLL_DELTA.1);
    assert_eq!(value("Tex Scroll Speed Ship Min"), 0.0);
    assert_eq!(value("Tex Scroll Speed Ship Range"), hd::SPEED01_RANGE_KMH);
    assert_eq!(
        value("Tex Scroll Speed Thrust Contrib"),
        hd::SPEED01_THRUST_CONTRIB
    );
    assert_eq!(value("Tex UScale Max"), hd::TEX_USCALE_MAX);
    assert_eq!(value("Enable Engine Trails"), 1.0);
}

/// The sprite flare's constants are the tuning file's too, and its texture is
/// the executable's own literal.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_sprite_flares_constants_are_the_discs_own() {
    let Some(image) = image() else {
        return;
    };
    let spec = format!("{}:PS3_GAME/USRDIR/DATA02.PSARC", image.display());
    let text = String::from_utf8(
        oag_mesh::mesh::read_blob(&spec, "/data/ships/shipeffectstweaks.txt")
            .expect("the tuning file"),
    )
    .expect("ascii");
    let value = |key: &str| -> f32 {
        let line = text
            .lines()
            .find(|line| line.contains(key))
            .unwrap_or_else(|| panic!("no {key}"));
        line.split('=')
            .nth(1)
            .expect("=")
            .trim()
            .parse()
            .expect("float")
    };
    use oag_fx::exhaust::hd;
    assert_eq!(value("Enable Flare Sprite"), 1.0);
    assert_eq!(
        value("\"Ship Effects.Engine Trails.Flare Radius\""),
        hd::SPRITE_RADIUS
    );
    assert_eq!(value("Flare Radius Min"), hd::SPRITE_RADIUS_MIN);
    assert_eq!(value("Max Radius Jitter"), hd::SPRITE_RADIUS_JITTER);
    assert_eq!(value("Slow Alpha Noise Min"), hd::SPRITE_ALPHA_NOISE.0);
    assert_eq!(value("Slow Alpha Noise Max"), hd::SPRITE_ALPHA_NOISE.1);
    assert_eq!(
        value("Slow Alpha Noise Chase Speed"),
        hd::SPRITE_ALPHA_CHASE
    );
    assert_eq!(
        value("Slow Alpha Noise Timer"),
        hd::SPRITE_ALPHA_RETARGET as f32
    );
    assert_eq!(value("Flare Opacity Max"), hd::SPRITE_OPACITY_MAX);
    // The four `EngineFlare_RenderTick`'s fade loads on top of those (the
    // "Ninth session" of engine-trail.md, 2026-09-15).
    assert_eq!(value("Flare Fadeout Dist"), hd::SPRITE_FADEOUT_DIST);
    assert_eq!(value("Flare Fadeout Range"), hd::SPRITE_FADEOUT_RANGE);
    assert_eq!(value("Flare Highlight Power"), hd::SPRITE_HIGHLIGHT_POWER);
    assert_eq!(value("Flare Highlight Boost"), hd::SPRITE_HIGHLIGHT_BOOST);
    // And the texture the flare's init names decodes from the archive set.
    let blob = oag_mesh::mesh::read_blob(&spec, "/data/tex/engineflare/engine_flare_rich.gtf")
        .expect("Engine_Flare_Rich.gtf");
    assert!(blob.len() > 100_000, "{} bytes", blob.len());
}

/// **The two `WO_TRAIL_HITSHIP` systems are on the disc and load.**
///
/// What `Race::advance_trail_hits` plays when a craft flies into a trail. The
/// loader reports one line per effect and says "will not be drawn" when a name
/// is not in the mounted archives, so the report is the evidence - the same
/// line a screenshot could not tell apart from "the effect never fired".
///
/// This is the half the unit tests cannot reach: they pin *when* the effect
/// fires, and this pins that there is something to fire.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_trail_hit_effects_are_on_the_disc_and_load() {
    let Some(loaded) = load() else { return };
    for name in [
        oag_title::engine_effects::TRAIL_HITSHIP_EFFECT,
        oag_title::engine_effects::TRAIL_HITSHIP_RED_EFFECT,
    ] {
        let line = loaded
            .report
            .iter()
            .find(|line| line.contains(name))
            .unwrap_or_else(|| panic!("{name}: the loader said nothing about it at all"));
        assert!(!line.contains("will not be drawn"), "{name}: {line}");
        assert!(
            line.contains("emitter(s)"),
            "{name}: loaded but reported no emitters - {line}"
        );
    }
}

/// **Every HD hull authors `Ship Collision Fx` locators**, which is what the
/// trail-hit burst is anchored to.
///
/// Without them `Race::spark_anchor_of` answers `None` on every craft and the
/// burst silently falls back to a derived point clear of the hull - the exact
/// failure a player reported. So this asserts the data is there rather than
/// trusting the fallback never to be reached.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn every_hd_hull_authors_the_spark_anchors_a_trail_hit_lands_on() {
    let Some(loaded) = load() else { return };
    assert!(
        !loaded.setup.spark_anchors.is_empty(),
        "no slots carry anchors at all"
    );
    for (slot, anchors) in loaded.setup.spark_anchors.iter().enumerate() {
        assert!(
            !anchors.is_empty(),
            "slot {slot}: no Ship Collision Fx locators, so a trail hit would \
             fall back to a derived point"
        );
        println!("slot {slot}: {} spark anchor(s)", anchors.len());
    }
}

/// **A craft driving into a trail sparks from its front, not its exhaust.**
///
/// The regression a player caught on Assegai: with the contact taken at the
/// hull's centre, a craft already inside a ribbon is equidistant from it nose
/// and tail, so the pick among the authored anchors was a coin toss and landed
/// at the rear. The contact is taken at the leading edge now, and this asserts
/// the consequence on the disc's own hulls rather than on a synthetic one.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn a_craft_inside_a_trail_sparks_from_its_leading_half() {
    let Some(loaded) = load() else { return };
    let mut race = race::Race::start(loaded.setup);
    race.set_autopilot(true);
    for _ in 0..300 {
        race.tick(&PlayerInputs::none());
    }
    // Sit slot 1 squarely inside slot 0's ribbon, pointing the same way: the
    // geometry the complaint was about.
    let lead = race.sim.world.ships[0].physics.body;
    race.sim.world.ships[1].physics.body.orientation = lead.orientation;
    race.sim.world.ships[1].physics.body.position = lead.position - lead.forward() * 12.0;

    let craft = race.sim.world.ships[1].physics.body;
    let anchor = race
        .spark_anchor_of(1, 0)
        .expect("slot 1 authors spark anchors and slot 0 has a ribbon");
    let ahead = (anchor - craft.position).dot(craft.forward());
    println!("anchor {ahead:.2} units ahead of the hull centre");
    assert!(
        ahead > 0.0,
        "the burst anchored {ahead:.2} units along forward - behind the centre \
         is the exhaust end, which is the bug this pins"
    );
}

/// **How much tighter the anchor test is than the sphere it replaced.**
///
/// A player reported the sparks firing before the craft touched the trail. The
/// sphere was `hull_reach` - the origin-to-nozzle distance, about half a hull
/// length - around the craft's centre, which on a hull far narrower than it is
/// long reaches well past its side. This walks a craft sideways out of another's
/// ribbon and reports the centre distance at which each test stops firing.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_anchor_test_fires_later_than_the_sphere_it_replaced() {
    let Some(loaded) = load() else { return };
    let mut race = race::Race::start(loaded.setup);
    race.set_autopilot(true);
    for _ in 0..300 {
        race.tick(&PlayerInputs::none());
    }
    let lead = race.sim.world.ships[0].physics.body;
    race.sim.world.ships[1].physics.body.orientation = lead.orientation;

    let mut anchor_edge = 0.0_f32;
    for step in 0..200 {
        let across = step as f32 * 0.1;
        race.sim.world.ships[1].physics.body.position =
            lead.position - lead.forward() * 12.0 + lead.up() * across;
        if race.spark_anchor_of(1, 0).is_some() && race.trail_touches_for_tests(1, 0) {
            anchor_edge = across;
        }
    }
    let sphere_edge = race.hull_reach_for_tests(1) + oag_fx::exhaust::hd::FIN_HALF_WIDTH;
    println!("anchor test stops firing at {anchor_edge:.2} units off the ribbon");
    println!("the sphere it replaced fired out to {sphere_edge:.2}");
    assert!(
        anchor_edge < sphere_edge,
        "the anchor test ({anchor_edge:.2}) is not tighter than the sphere ({sphere_edge:.2})"
    );
}

/// **The sprite flare on the real grid: never on the player's craft, and on
/// every other craft only by the traced law.**
///
/// The original turns the viewing player's own craft away before its fade
/// math on every one of 30 measured frames and passes all seven AI crafts
/// (`docs/ghidra/functions/ps3-hdfury-eu/engine-trail.md`, "Tenth session",
/// confidence 92). This starts the same race the tenth session measured -
/// a single race, eight craft - runs the countdown out, and checks each
/// craft against the law computed from the race's own camera, printing the
/// per-craft table (distance, view dot, fade) beside the one the breakpoint
/// run gave: six of seven AI flares at 0.0 and one at 0.002-0.003 on the
/// start grid. This engine's grid stands the whole field past `Flare Fadeout
/// Dist + Range` from the player's camera, so here every one of the seven is
/// at 0 and none builds a quad - the original's own rule, the quad being
/// built only while the fade is positive. One opponent moved inside the fade
/// is the positive path.
#[test]
#[ignore = "needs data/images/hdfury-ps3-eu-dec.iso"]
fn the_sprite_flare_skips_the_players_craft_and_fades_the_rest_by_the_law() {
    use oag_core::math::Vec3;
    use oag_fx::exhaust::{self, hd};
    let Some(loaded) = load() else { return };
    let mut race = race::Race::start(loaded.setup);
    // Calibrated against the far chase view's eye. The default became the close
    // view on 2026-10-01 (a fresh Pulse profile starts on `OPT_CLOSE`), which sits
    // three units nearer the craft, and this fixture's numbers depend on the
    // distance to the eye, so the view it was written for is named here.
    race.set_camera_view(oag_display::display::CameraView::Far);
    for _ in 0..60 {
        race.tick(&PlayerInputs::none());
    }
    assert!(race.hd_trail_active(), "an HD race draws HD's exhaust");
    let camera = race.view();
    let right = Vec3::new(camera.x_axis.x, camera.y_axis.x, camera.z_axis.x);
    let up = Vec3::new(camera.x_axis.y, camera.y_axis.y, camera.z_axis.y);
    let forward = -Vec3::new(camera.x_axis.z, camera.y_axis.z, camera.z_axis.z);
    let eye = race.camera_position();

    assert!(
        race.hd_sprite_quad(0, right, up).is_empty(),
        "the viewing player's craft never gets the sprite"
    );
    let far = hd::SPRITE_FADEOUT_DIST + hd::SPRITE_FADEOUT_RANGE;
    for slot in 1..race.ship_count() as usize {
        let nozzle = race
            .nozzle_of(slot)
            .expect("every HD hull authors a nozzle");
        let body = race.sim.world.ships[slot].physics.body;
        let distance = (nozzle - eye).length();
        let view_dot = hd::sprite_view_dot(forward, -body.forward());
        let fade = hd::sprite_fade(distance * hd::SPRITE_DISTANCE_SCALE, view_dot, 1.0);
        let quad = race.hd_sprite_quad(slot, right, up);
        println!(
            "slot {slot}: {distance:6.1} units, view dot {view_dot:+.3}, fade {fade:.4} at walk \
             1.0, {} - nozzle {nozzle:.1?}, forward {:.3?}, up {:.3?}",
            if quad.is_empty() { "no quad" } else { "a quad" },
            body.forward(),
            body.up(),
        );
        assert!(
            distance * hd::SPRITE_DISTANCE_SCALE >= far,
            "slot {slot} is inside the fade at {distance:.1} units - this engine's grid \
             layout moved, and the assertions below assume nothing is"
        );
        assert!(
            quad.is_empty(),
            "slot {slot}: a quad past Dist + Range, where the fade is 0 and the original \
             builds none"
        );
    }

    // The positive path: slot 1 moved 8 units dead ahead of the player is
    // inside the fade and on the axis, so its alpha is the walk itself, its
    // quad is 4:1, and its half-height is the model-space law times the
    // craft's own global scale, between the law's floor and ceiling.
    let lead = race.sim.world.ships[0].physics.body;
    race.sim.world.ships[1].physics.body.orientation = lead.orientation;
    race.sim.world.ships[1].physics.body.position = lead.position + lead.forward() * 8.0;
    let nozzle = race.nozzle_of(1).expect("slot 1's nozzle");
    let quad = race.hd_sprite_quad(1, right, up);
    assert_eq!(quad.len(), 6, "an opponent 8 units ahead is one quad");
    let alpha = quad[0].colour[3];
    println!("slot 1 moved 8 units ahead: fade {alpha:.4}");
    assert!(
        alpha >= hd::SPRITE_ALPHA_NOISE.0 * 0.9 && alpha <= hd::SPRITE_ALPHA_NOISE.1,
        "{alpha} is not the walk inside the fadeout on the nozzle axis"
    );
    let extent = |axis: Vec3| {
        quad.iter()
            .map(|v| (Vec3::from_array(v.position) - nozzle).dot(axis).abs())
            .fold(0.0f32, f32::max)
    };
    let (half_height, half_width) = (extent(up), extent(right));
    assert!(
        (half_width - half_height * hd::SPRITE_ASPECT).abs() < 1e-3,
        "{half_width} wide by {half_height} tall is not 4:1"
    );
    let floor = hd::SPRITE_RADIUS_MIN * exhaust::CRAFT_ROW_SCALE;
    let ceiling = (hd::SPRITE_RADIUS_MIN + hd::SPRITE_RADIUS + hd::SPRITE_RADIUS_JITTER)
        * exhaust::CRAFT_ROW_SCALE;
    assert!(
        half_height >= floor - 1e-4 && half_height <= ceiling + 1e-4,
        "half-height {half_height} outside {floor}..{ceiling}"
    );
}
