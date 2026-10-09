//! Draws one surface through the mesh pipeline with Wipeout HD's Zone
//! parameters bound, reads the frame back, and asserts the pixel.
//!
//! # Why a pixel and not the uniform
//!
//! `crates/game/tests/zone_grade_ground_truth.rs` asserts the *assembly* of
//! `zoneColourTint.xy` and `zoneEffect.rgb` off the disc. Nothing there
//! reaches the GPU, so a wiring error - the stage texture on the wrong
//! binding, `zone_sample` unreachable, the uniform's `zone` field at a
//! different offset in Rust than in WGSL - passes every one of those tests
//! while drawing nothing.
//!
//! It also pins the **colour space**, which is the failure this test exists
//! for. `mesh.wgsl` decodes its samples with `pow(texel, 2.2)` and shades in
//! linear light; `zoneTex` is a texture and wants that decode, `zoneEffect` is
//! a shader parameter and does not. Summing before the decode instead of after
//! computes `pow(zoneTex * zoneEffect, 2.2)` and distorts every stage colour -
//! silently, because the two agree exactly when `zoneEffect` is `1.0`. The
//! effect below is deliberately `2.0` on red so the two answers are 175 and
//! 255.
//!
//! Skips when there is no adapter, like `msaa_resolve.rs` and
//! `velocity_target.rs`.

use std::sync::Arc;

use oag_mesh::mesh::{Bounds, DrawCall, GpuVertex, Model, ModelTexture, Texels, slots};
use oag_mesh::mesh_render::{self, Anisotropy, Scene, UNIFORMS_SIZE, Zone, ZoneSet, zone};

const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;
const SIZE: u32 = 64;

/// The zone texture's one texel, and the multiplier bound beside it.
const ZONE_TEXEL: u8 = 128;
const ZONE_EFFECT: [f32; 4] = [2.0, 1.0, 0.0, 0.0];

fn vertex(position: [f32; 3], lit: f32) -> GpuVertex {
    GpuVertex {
        position,
        normal: [0.0, 0.0, 1.0],
        colour: [1.0, 1.0, 1.0, 1.0],
        texcoord: [0.0, 0.0],
        lightmap_texcoord: [0.0, 0.0],
        // `1.0` takes the authored path, which shades in linear light; `0.0`
        // takes the stand-in path, which is gamma throughout.
        lit,
        anim: 0,
        xform: 0,
        sun_mask: 1.0,
        // **Emissive**, so `authored` is exactly `vec3(1.0)` and the specular
        // is switched off: the light rig is then the identity and the pixel
        // measures the Zone term and the sRGB round trip alone. That is the
        // material's own declaration in the original too - a program fed
        // neither a constant ambient nor a directional light.
        slots: slots::DEFAULT | slots::EMISSIVE,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
        texcoord2: [0.0, 0.0],
    }
}

fn model(albedo: Arc<ModelTexture>, lit: f32) -> Model {
    Model {
        airbrakes: [None, None],
        node_vertex_ranges: Vec::new(),
        lod_groups: Default::default(),
        label: "zone recolour test quad".into(),
        indices: vec![0, 1, 2],
        draws: vec![DrawCall {
            moving: false,
            blend: None,
            blend_state: None,
            layer: oag_vex::vex::LAYER_DEFAULT,
            culled: false,
            range: 0..3,
            texture: Some(0),
            bounds: Bounds {
                centre: [0.0, 0.0, 0.5],
                radius: 1.5,
            },
            node: None,
            chunk: None,
            alpha_test_ref: None,
        }],
        alpha_tested_draws: Vec::new(),
        transparent_draws: Vec::new(),
        textures: vec![Some(albedo)],
        lightmaps: vec![None],
        pad_masks: Vec::new(),
        wave_maps: Vec::new(),
        material_slots: Vec::new(),
        material_specular_exponent: Vec::new(),
        material_colour_factor: Vec::new(),
        material_variants: Vec::new(),
        material_anim: Vec::new(),
        shine_draws: Vec::new(),
        vertex_colour_is_light: false,
        stamps_glow: false,
        glow_by_texel: false,
        flame: None,
        absorb_shell: false,
        alpha_test_ref: None,
        centre: [0.0, 0.0, 0.5],
        radius: 2.0,
        mesh_count: 1,
        anim_tracks: Vec::new(),
        anim_nodes: Vec::new(),
        emissive: Vec::new(),
        vertices: vec![
            vertex([-0.9, -0.9, 0.5], lit),
            vertex([0.9, -0.9, 0.5], lit),
            vertex([0.0, 0.9, 0.5], lit),
        ],
    }
}

fn texture(label: &str, rgba: [u8; 4]) -> Arc<ModelTexture> {
    Arc::new(ModelTexture {
        label: label.into(),
        width: 1,
        height: 1,
        texels: Texels::Rgba8(rgba.to_vec()),
        mip_count: None,
    })
}

/// Identity camera and model, so clip space is model space.
fn uniforms() -> Vec<u8> {
    let identity: [[f32; 4]; 4] = [
        [1.0, 0.0, 0.0, 0.0],
        [0.0, 1.0, 0.0, 0.0],
        [0.0, 0.0, 1.0, 0.0],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let mut bytes = Vec::new();
    bytes.extend_from_slice(bytemuck::cast_slice(&identity)); // view_projection
    bytes.extend_from_slice(bytemuck::cast_slice(&identity)); // model
    bytes.extend_from_slice(&[0u8; 16]); // the four pad floats
    bytes.extend_from_slice(bytemuck::cast_slice(&identity)); // prev_mvp
    assert_eq!(bytes.len() as u64, UNIFORMS_SIZE);
    bytes
}

/// `stage` bound as the **Scene** publication's texture, Inner and Outer
/// alike, with the Track slot holding a magenta the tests below never expect
/// on either side - so a quad whose `slots` carry no `ZONE_TRACK` (every
/// quad here but the split test's) is measured through the set the file's
/// own bit selects for it, and a wrong selection paints a colour no
/// assertion accepts. The Outer half equals the Inner, so a caller that does
/// not itself vary the two (every test but the sphere one) draws the same
/// picture regardless of which side of the sphere a fragment lands on.
fn scene_art(stage: &Arc<ModelTexture>) -> zone::StageArt {
    let track = Some(texture("poisoned track stage", [255, 0, 255, 255]));
    zone::StageArt {
        track: track.clone(),
        track_outer: track,
        scene: Some(stage.clone()),
        scene_outer: Some(stage.clone()),
    }
}

/// A [`Zone`] whose Scene set is `set` and whose Track set is a magenta
/// poison, on the same terms as [`scene_art`].
///
/// No transition in flight: the Outer pair equals the Inner, so the sphere
/// test selects the same colours whichever side of it a fragment falls, and
/// the radius is irrelevant. `the_transition_sphere_selects_the_inner_pair_inside_and_the_outer_outside`
/// is where the two pairs differ.
fn scene_zone(set: ZoneSet) -> Zone {
    let track = ZoneSet {
        effect: [4.0, 0.0, 4.0, 0.0],
        base: [0.0; 4],
        base_alt: [0.0; 4],
    };
    Zone {
        // `zoneColourTint.xy`. `1.0` on both lanes is what
        // `zonemode.effectsettings` authors.
        uv_scale: [1.0, 1.0],
        enabled: 1.0,
        radius: 0.0,
        origin: [0.0; 4],
        track,
        scene: set,
        track_outer: track,
        scene_outer: set,
    }
}

/// What `mesh.wgsl` must produce for one channel: the zone sample decoded,
/// scaled by the parameter, then encoded back.
fn expected(effect: f32) -> u8 {
    let sample = f32::from(ZONE_TEXEL) / 255.0;
    let linear = (sample.powf(2.2) * effect).clamp(0.0, 1.0);
    (linear.powf(1.0 / 2.2) * 255.0).round() as u8
}

/// **The Zone recolour reaches the frame, in the right colour space, and
/// replaces the albedo whatever the albedo held.**
#[test]
fn the_zone_surface_replaces_the_albedo_in_the_right_colour_space() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");

    // A pure-black diffuse, which is exactly what the original's artists paint
    // where they want Zone mode to light a surface up.
    let black = model(texture("black albedo", [0, 0, 0, 255]), 1.0);
    let stage = texture("zone stage", [ZONE_TEXEL, ZONE_TEXEL, ZONE_TEXEL, 255]);

    let mut scene = Scene::off();
    // The authored rig, switched on so the `lit` vertices take it. `Emissive`
    // above makes its own terms the identity.
    scene.light.enabled = 1.0;
    // **The eye, dead in front and far away, so `rim` is zero here.** The
    // shader reads it out of the fog block, and `Fog::off` leaves it at the
    // origin - which for this quad (at `z = 0.5`, normal `+Z`) is *behind* the
    // surface, giving `rim = 2` and a `rim^10` of 1024 that would swamp every
    // assertion below. A hundred units along the normal makes
    // `dot(N, toEye) = 1` to four decimals, so the two rim summands vanish and
    // this test measures the texture term and the colour space alone.
    // `the_two_rim_summands_carry_their_own_literal_exponents` is where they
    // are measured instead.
    scene.fog.camera = [0.0, 0.0, 100.5];
    scene.zone = scene_zone(ZoneSet {
        effect: ZONE_EFFECT,
        base: [0.0; 4],
        base_alt: [0.0; 4],
    });
    let art = scene_art(&stage);

    // Black albedo: the recolour lands.
    let painted = draw(&device, &queue, &black, &art, scene, None);
    let (r, g, b) = (expected(ZONE_EFFECT[0]), expected(ZONE_EFFECT[1]), 0u8);
    assert_eq!(
        (painted[0], painted[1], painted[2]),
        (r, g, b),
        "the Zone term must be summed in linear light: summing before the \
         sRGB decode would clip red to 255 instead of {r}"
    );
    // The whole point of the constants: the two answers differ.
    assert_ne!(
        painted[0], 255,
        "red saturated - the sum is in the wrong space"
    );

    // **A white albedo draws the same pixel, and that is the finding.** The
    // Zone variant *replaces* a circuit material's shading rather than tinting
    // it: 20,084 of the disc's 20,214 Zone-bearing fragment blocks carry this
    // shape and not one of them contains the `100000` black-mask literal. This
    // assertion is the inverse of the one it replaces, which asserted the
    // `zoneAniso` shape that lives only in HD's four Zone arenas.
    let white = model(texture("white albedo", [255, 255, 255, 255]), 1.0);
    let replaced = draw(&device, &queue, &white, &art, scene, None);
    assert_eq!(
        (replaced[0], replaced[1], replaced[2]),
        (r, g, b),
        "the Zone surface must replace the albedo, not add to it - a white \
         albedo and a black one draw the same pixel"
    );

    // **The stand-in path, which is gamma throughout.** Prelit geometry
    // (`lit == 0`) takes it even under an authored rig, because its light is
    // baked into its vertex colours in the space the asset authors - so the
    // Zone term is summed there *undecoded*. Green is the discriminating
    // channel: `128` is the raw sample, and `56` is what a linear summand
    // would have left.
    let prelit = model(texture("black albedo", [0, 0, 0, 255]), 0.0);
    let gamma = draw(&device, &queue, &prelit, &art, scene, None);
    let raw =
        |effect: f32| ((f32::from(ZONE_TEXEL) / 255.0 * effect).min(1.0) * 255.0).round() as u8;
    assert_eq!(
        (gamma[0], gamma[1], gamma[2]),
        (raw(ZONE_EFFECT[0]), raw(ZONE_EFFECT[1]), 0),
        "the stand-in path must sum in gamma, not fold in a linear term"
    );

    // The same black surface with the Zone term switched off draws black,
    // which is what every draw outside a Zone race gets.
    let off = draw(
        &device,
        &queue,
        &black,
        &zone::StageArt::NONE,
        Scene::off(),
        None,
    );
    assert_eq!(
        (off[0], off[1], off[2]),
        (0, 0, 0),
        "no Zone stage must add nothing at all"
    );
}

/// **The two rim summands reach the frame, each with its own literal
/// exponent.**
///
/// `colour = zoneBase.rgb * rim^10 + zoneBaseAlt.rgb * rim^5`, where
/// `rim = 1 - dot(N, toEye)`. The exponents are inline constants in every one
/// of the 20,084 blocks that carry this shape, so nothing per-stage drives
/// them and a swap would be invisible to any test that only checks the
/// colours arrive.
///
/// **This makes a swap loud.** The eye is placed 60 degrees off the surface
/// normal, so `rim` is exactly `0.5`, and the two colours are scaled to
/// cancel their own exponent: `512 * 0.5^10` and `16 * 0.5^5` are both `0.5`.
/// Swap the two exponents and red becomes `512 * 0.5^5 = 16` (clipped white)
/// while green becomes `16 * 0.5^10 = 0.016` (black) - the exact opposite
/// corner of the cube from the `(186, 186, 0)` asserted here.
///
/// The stage colour is bound to zero throughout, so the texture term drops out
/// and the pixel is the rim terms alone.
#[test]
fn the_two_rim_summands_carry_their_own_literal_exponents() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");

    let surface = model(texture("albedo", [255, 255, 255, 255]), 1.0);
    let stage = texture("zone stage", [ZONE_TEXEL, ZONE_TEXEL, ZONE_TEXEL, 255]);

    let mut scene = Scene::off();
    scene.light.enabled = 1.0;
    // `toEye = (sin 60, 0, cos 60)` from the quad's own plane, a hundred units
    // out so the centre fragment's own offset within the triangle does not
    // move the angle. The quad's normal is `+Z`, so `dot(N, toEye) = 0.5` and
    // `rim = 0.5`.
    scene.fog.camera = [86.6025, 0.0, 50.5];
    scene.zone = scene_zone(ZoneSet {
        // Zero, so the texture term contributes nothing and the pixel is the
        // two rim summands alone.
        effect: [0.0; 4],
        // `512 * 0.5^10 == 0.5`, on red.
        base: [512.0, 0.0, 0.0, 0.0],
        // `16 * 0.5^5 == 0.5`, on green.
        base_alt: [0.0, 16.0, 0.0, 0.0],
    });

    let painted = draw(&device, &queue, &surface, &scene_art(&stage), scene, None);
    // Both summands are shader *parameters*, not textures, so they enter the
    // linear path undecoded exactly as `zoneEffect` does; only the encode back
    // out applies.
    let half = (0.5f32.powf(1.0 / 2.2) * 255.0).round() as u8;
    for (channel, got, want) in [("red", painted[0], half), ("green", painted[1], half)] {
        assert!(
            got.abs_diff(want) <= 2,
            "{channel} came out {got}, want about {want}: the two rim summands \
             must use 10 and 5 in that order"
        );
    }
    assert_eq!(painted[2], 0, "nothing was bound on blue");
}

/// A quad with a normal facing straight up (`+Y`), the shape [`zone_glow`]'s
/// own `saturate(N.y - 0.5)` gate wants a nonzero reading from - see
/// `oag_mesh::mesh_render::Zone`'s doc comment: the visualiser is a floor
/// display, and every other quad in this file faces the camera along `+Z`.
fn up_facing_model(albedo: Arc<ModelTexture>) -> Model {
    let vertex = |position: [f32; 3]| GpuVertex {
        position,
        normal: [0.0, 1.0, 0.0],
        colour: [1.0, 1.0, 1.0, 1.0],
        texcoord: [0.0, 0.0],
        lightmap_texcoord: [0.0, 0.0],
        lit: 0.0,
        anim: 0,
        xform: 0,
        sun_mask: 1.0,
        slots: slots::DEFAULT,
        specular_exponent: oag_mesh::mesh::DEFAULT_SPECULAR_EXPONENT,
        glow: 0.0,
        texcoord2: [0.0, 0.0],
    };
    Model {
        vertices: vec![
            vertex([-0.9, -0.9, 0.5]),
            vertex([0.9, -0.9, 0.5]),
            vertex([0.0, 0.9, 0.5]),
        ],
        ..model(albedo, 0.0)
    }
}

/// **The visualiser glow reaches the frame**, indexed by the stage texture's
/// own alpha and driven by a spectrum this test writes directly - the same
/// seam `race::Scene::render` calls once a frame in the real game, via
/// `oag_mesh::mesh_render::zone::write_vis`.
///
/// Isolated from every other term: `effect.rgb`, `base` and `base_alt` are
/// all zero (no surface colour), the albedo is irrelevant (the Zone term
/// replaces it), and `scene.light.enabled = 0.0` selects the gamma/stand-in
/// path unconditionally, so the pixel is `zone_glow` alone - see
/// `mesh.wgsl`'s own function for the formula this measures.
#[test]
fn the_visualiser_glow_is_driven_by_the_vis_lookup_and_gated_up_facing() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");

    let surface = up_facing_model(texture("albedo", [255, 255, 255, 255]));
    // Alpha 128 of 255 is the band index `zone_glow` reads through the
    // nearest-filtered clone of this same texture - `128 / 255 ≈ 0.50196`,
    // the midpoint of a 256-wide lookup's 128th texel, chosen so the exact
    // nearest-sampling convention cannot land it in a neighbour.
    let stage = texture("zone stage", [200, 200, 200, 128]);

    // Texel 128 is band 12's eighth segment under the recovered layout
    // (`128 = 1 + 10 * 12 + 7`), so band 12 at full level lights it - see
    // `oag_mesh::mesh_render::zone::write_vis`. Under the invented
    // one-band-per-texel spread this replaced, the same texel was band 128
    // of 256; the layout moved, what the shader does with the texel did not.
    let mut bands = [0.0f32; 16];
    bands[12] = 1.0;

    let mut scene = Scene::off();
    // Off, so `shaded.rgb` unconditionally takes the gamma/stand-in path -
    // see `mesh.wgsl`'s `mix(plain_rgb, authored_rgb, scene.light.enabled *
    // in.lit)`.
    scene.light.enabled = 0.0;
    scene.zone = scene_zone(ZoneSet {
        // `.rgb` zero so the texture term of the *surface* contributes
        // nothing; `.w` is `E.w`, the glow's own drive scalar.
        effect: [0.0, 0.0, 0.0, 1.6],
        base: [0.0; 4],
        base_alt: [0.0; 4],
    });
    let art = scene_art(&stage);

    let lit = draw(
        &device,
        &queue,
        &surface,
        &art,
        scene,
        Some((&bands, [255, 255, 255])),
    );
    // `up = saturate(1.0 - 0.5) = 0.5`; `window_depth` is `0.5` (identity
    // matrices put clip space and model space at the same numbers, and this
    // quad sits at `z = 0.5`), so `(1 - window_depth) = 0.5`; `vis = (1, 1,
    // 1)` at the written band. `0.5 * 0.5 * 1.6 * 1.0 = 0.4`, `0.4 * 255 =
    // 102`.
    let want = 102u8;
    for (channel, got) in [("red", lit[0]), ("green", lit[1]), ("blue", lit[2])] {
        assert!(
            got.abs_diff(want) <= 2,
            "{channel} came out {got}, want about {want}: the visualiser \
             glow did not reach the frame as the formula predicts"
        );
    }

    // **The gate, the other half of the finding.** The same draw with the
    // written band left at zero shows no glow - a lookup this test never
    // touched must not light anything, the same "a missing input draws
    // nothing" rule the rest of this file's Zone terms already keep.
    let dark = draw(
        &device,
        &queue,
        &surface,
        &art,
        scene,
        Some((&[0.0f32; 256], [255, 255, 255])),
    );
    assert_eq!(
        (dark[0], dark[1], dark[2]),
        (0, 0, 0),
        "an all-zero spectrum must draw no glow at all"
    );
}

/// **The chunk's own bit picks the publication.** HD publishes the Zone
/// parameters twice - `zoneModeTrack*` beside the `Track.*` colours and
/// `zoneMode*` beside the `Scene.*` ones - and selects per chunk on bit 0 of
/// its render-block flags, which `mesh::rcs` carries as `slots::ZONE_TRACK`.
/// Two draws that differ in nothing but that bit read different textures
/// *and* different colours, each pair the one the original binds together.
#[test]
fn the_track_bit_selects_the_track_texture_and_colours_and_its_absence_the_scene_pair() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");

    let black = texture("black albedo", [0, 0, 0, 255]);
    let scenery = model(black.clone(), 1.0);
    let mut road = model(black, 1.0);
    for vertex in &mut road.vertices {
        vertex.slots |= slots::ZONE_TRACK;
    }
    // Distinct texels in the two slots, and distinct colours in the two
    // sets, so a crossed pair - track texture with scene colours, or the
    // other way - lands on a third answer neither assertion accepts.
    // Outer equal to Inner on both sets: this test's own `Zone` carries a
    // zero radius, which makes `zone_inside` false everywhere (a distance is
    // never negative), so every fragment here samples the Outer texture -
    // and the assertions below are about the track/scene split, not the
    // sphere, so Outer must read exactly what Inner would.
    let art = zone::StageArt {
        track: Some(texture(
            "track stage",
            [ZONE_TEXEL, ZONE_TEXEL, ZONE_TEXEL, 255],
        )),
        track_outer: Some(texture(
            "track stage outer",
            [ZONE_TEXEL, ZONE_TEXEL, ZONE_TEXEL, 255],
        )),
        scene: Some(texture("scene stage", [255, 255, 255, 255])),
        scene_outer: Some(texture("scene stage outer", [255, 255, 255, 255])),
    };
    let mut scene = Scene::off();
    scene.light.enabled = 1.0;
    scene.fog.camera = [0.0, 0.0, 100.5];
    let track_set = ZoneSet {
        effect: ZONE_EFFECT,
        base: [0.0; 4],
        base_alt: [0.0; 4],
    };
    let scene_set = ZoneSet {
        effect: [0.0, 0.0, 0.25, 0.0],
        base: [0.0; 4],
        base_alt: [0.0; 4],
    };
    scene.zone = Zone {
        uv_scale: [1.0, 1.0],
        enabled: 1.0,
        radius: 0.0,
        origin: [0.0; 4],
        track: track_set,
        scene: scene_set,
        track_outer: track_set,
        scene_outer: scene_set,
    };

    let track = draw(&device, &queue, &road, &art, scene, None);
    assert_eq!(
        (track[0], track[1], track[2]),
        (expected(ZONE_EFFECT[0]), expected(ZONE_EFFECT[1]), 0),
        "a track chunk reads the track texture through the Track colours"
    );

    let general = draw(&device, &queue, &scenery, &art, scene, None);
    // A white texel decodes to 1.0, so the pixel is the Scene effect alone:
    // `0.25` in linear light encodes to `0.25^(1/2.2) * 255`.
    let blue = (0.25f32.powf(1.0 / 2.2) * 255.0).round() as u8;
    assert_eq!(
        (general[0], general[1]),
        (0, 0),
        "a scene chunk must not see the Track colours"
    );
    assert!(
        general[2].abs_diff(blue) <= 2,
        "blue came out {}, want about {blue}: the scene chunk reads the scene \
         texture through the Scene colours",
        general[2]
    );
}

/// **The stage-transition sphere reaches the frame: a fragment inside it
/// reads the Inner pair and one outside it the Outer.** One quad, one draw,
/// two texels read off it on either side of the boundary - the pattern the
/// original's own `distance(worldPos, zoneOrigin) < zoneColourTint.w` puts
/// on every Zone surface as a stage change sweeps out of the craft.
///
/// The identity matrices make world space clip space, so the quad spans
/// `x` in `-0.9..0.9` at `z = 0.5`. The origin sits at its left edge and the
/// radius is one unit: `x = -0.5` is half a unit in and `x = 0.5` a unit and
/// a half out. Inner is red, Outer is green, both through a white stage
/// texel so the pixel is the parameter alone - and a swapped select lands
/// each fragment on the other's colour, which no assertion here accepts.
#[test]
fn the_transition_sphere_selects_the_inner_pair_inside_and_the_outer_outside() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");

    let surface = model(texture("black albedo", [0, 0, 0, 255]), 1.0);
    let stage = texture("zone stage", [255, 255, 255, 255]);
    let inner = ZoneSet {
        effect: [1.0, 0.0, 0.0, 0.0],
        base: [0.0; 4],
        base_alt: [0.0; 4],
    };
    let outer = ZoneSet {
        effect: [0.0, 1.0, 0.0, 0.0],
        base: [0.0; 4],
        base_alt: [0.0; 4],
    };
    let mut scene = Scene::off();
    scene.light.enabled = 1.0;
    scene.fog.camera = [0.0, 0.0, 100.5];
    scene.zone = Zone {
        uv_scale: [1.0, 1.0],
        enabled: 1.0,
        radius: 1.0,
        origin: [-1.0, 0.0, 0.5, 1.0],
        // The Track pair is a magenta poison on both sides, as `scene_zone`
        // binds it: this quad carries no `ZONE_TRACK`, so it must read the
        // Scene pair.
        track: ZoneSet {
            effect: [4.0, 0.0, 4.0, 0.0],
            base: [0.0; 4],
            base_alt: [0.0; 4],
        },
        track_outer: ZoneSet {
            effect: [4.0, 0.0, 4.0, 0.0],
            base: [0.0; 4],
            base_alt: [0.0; 4],
        },
        scene: inner,
        scene_outer: outer,
    };

    let frame = draw_frame(&device, &queue, &surface, &scene_art(&stage), scene, None);
    // Row 48 is `y = -0.5`, where the triangle is 1.4 units wide, so both
    // columns land on it: column 16 is `x = -0.5`, column 48 is `x = 0.5`.
    let texel = |column: u32, row: u32| {
        let at = ((row * SIZE + column) * 4) as usize;
        (frame[at], frame[at + 1], frame[at + 2])
    };
    let (row, left, right) = (SIZE * 3 / 4, SIZE / 4, SIZE * 3 / 4);
    assert_eq!(
        texel(left, row),
        (255, 0, 0),
        "half a unit from the origin is inside the sphere and reads the Inner pair"
    );
    assert_eq!(
        texel(right, row),
        (0, 255, 0),
        "a unit and a half from the origin is outside and reads the Outer pair"
    );

    // **And with the Outer pair equal to the Inner, the radius is
    // irrelevant** - every draw outside a transition, and outside a Zone race
    // altogether, relies on that.
    scene.zone.scene_outer = inner;
    scene.zone.radius = 0.0;
    let settled = draw_frame(&device, &queue, &surface, &scene_art(&stage), scene, None);
    let texel = |column: u32, row: u32| {
        let at = ((row * SIZE + column) * 4) as usize;
        (settled[at], settled[at + 1], settled[at + 2])
    };
    assert_eq!(texel(left, row), (255, 0, 0));
    assert_eq!(
        texel(right, row),
        (255, 0, 0),
        "outside a zero-radius sphere the Outer pair draws, and it is the Inner one"
    );
}

/// **The stage's own texture follows the sphere too, on the identical
/// `zone_inside` test the colours above already select on.** Same geometry,
/// origin and radius as
/// `the_transition_sphere_selects_the_inner_pair_inside_and_the_outer_outside`,
/// so the two pixel columns land on the same two fragments; here the
/// `ZoneSet`s are a flat white multiplier with no rim terms
/// (`effect = [1, 1, 1, 0]`), so the pixel is the sampled texel alone, and
/// the Inner and Outer scene textures are solid red and green - full-channel
/// values a rounding error could not blur into each other. Strikes the
/// "stage texture does not follow the sphere" item in
/// `docs/rendering/hd-zone-recolour.md`.
#[test]
fn the_transition_sphere_samples_the_inner_texture_inside_and_the_outer_outside() {
    let instance = wgpu::Instance::default();
    let Ok(adapter) = pollster::block_on(instance.request_adapter(&Default::default())) else {
        eprintln!("no GPU adapter: skipping");
        return;
    };
    let (device, queue) =
        pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor::default()))
            .expect("requesting the device");

    let surface = model(texture("black albedo", [0, 0, 0, 255]), 1.0);
    let white = ZoneSet {
        effect: [1.0, 1.0, 1.0, 0.0],
        base: [0.0; 4],
        base_alt: [0.0; 4],
    };
    // This quad carries no `ZONE_TRACK`, so it reads the Scene pair; the
    // Track one is a magenta poison, identical on both sides, so a wrong
    // selection of *set* (not side) would still fail loudly.
    let poison = ZoneSet {
        effect: [4.0, 0.0, 4.0, 0.0],
        base: [0.0; 4],
        base_alt: [0.0; 4],
    };
    let art = zone::StageArt {
        track: Some(texture("poisoned track stage", [255, 0, 255, 255])),
        track_outer: Some(texture("poisoned track outer stage", [255, 0, 255, 255])),
        scene: Some(texture("inner scene stage", [255, 0, 0, 255])),
        scene_outer: Some(texture("outer scene stage", [0, 255, 0, 255])),
    };
    let mut scene = Scene::off();
    scene.light.enabled = 1.0;
    scene.fog.camera = [0.0, 0.0, 100.5];
    scene.zone = Zone {
        uv_scale: [1.0, 1.0],
        enabled: 1.0,
        radius: 1.0,
        origin: [-1.0, 0.0, 0.5, 1.0],
        track: poison,
        track_outer: poison,
        scene: white,
        scene_outer: white,
    };

    let frame = draw_frame(&device, &queue, &surface, &art, scene, None);
    let texel = |column: u32, row: u32| {
        let at = ((row * SIZE + column) * 4) as usize;
        (frame[at], frame[at + 1], frame[at + 2])
    };
    let (row, left, right) = (SIZE * 3 / 4, SIZE / 4, SIZE * 3 / 4);
    assert_eq!(
        texel(left, row),
        (255, 0, 0),
        "half a unit from the origin is inside the sphere and samples the Inner texture"
    );
    assert_eq!(
        texel(right, row),
        (0, 255, 0),
        "a unit and a half out is outside and samples the Outer texture"
    );
}

/// Draws `model` once into a `SIZE`x`SIZE` target and answers the centre texel.
///
/// `zone_vis` is the visualiser lookup to write before drawing - `(bands,
/// tint)`, forwarded to `mesh_render::zone::write_vis` - or `None` to leave
/// it at the all-black default [`mesh_render::build`] starts it at.
fn draw(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    model: &Model,
    stage: &zone::StageArt,
    scene: Scene,
    zone_vis: Option<(&[f32], [u8; 3])>,
) -> [u8; 4] {
    let frame = draw_frame(device, queue, model, stage, scene, zone_vis);
    // Dead centre of the triangle.
    let at = (((SIZE / 2) * SIZE + SIZE / 2) * 4) as usize;
    [frame[at], frame[at + 1], frame[at + 2], frame[at + 3]]
}

/// [`draw`], returning the whole `SIZE`x`SIZE` RGBA frame, row-major from
/// the top, for a test that reads more than one texel of it.
fn draw_frame(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    model: &Model,
    stage: &zone::StageArt,
    scene: Scene,
    zone_vis: Option<(&[f32], [u8; 3])>,
) -> Vec<u8> {
    let built = mesh_render::build(
        device,
        queue,
        model,
        FORMAT,
        Anisotropy::Off,
        1,
        mesh_render::Depth::Scene,
        mesh_render::TRANSPARENT_BLEND,
        mesh_render::GlowMask::Protected,
        mesh_render::Velocity::None,
        stage,
        // No shadow map, no depth map and no receiver: this test draws one
        // model against nothing.
        mesh_render::ShadowMaps::NONE,
        mesh_render::ShadowReceiver::Never,
    )
    .expect("building the mesh pipeline");
    queue.write_buffer(&built.fog_buffer, 0, bytemuck::bytes_of(&scene));
    if let Some((bands, tint)) = zone_vis {
        mesh_render::zone::write_vis(queue, &built.zone_vis_texture, bands, Some(tint));
    }

    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("zone uniforms"),
        size: UNIFORMS_SIZE,
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    queue.write_buffer(&uniform_buffer, 0, &uniforms());
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("zone uniforms"),
        layout: &built.pipeline.get_bind_group_layout(0),
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });

    let extent = wgpu::Extent3d {
        width: SIZE,
        height: SIZE,
        depth_or_array_layers: 1,
    };
    let make = |label, format, usage| {
        device.create_texture(&wgpu::TextureDescriptor {
            label: Some(label),
            size: extent,
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage,
            view_formats: &[],
        })
    };
    let colour = make(
        "colour",
        FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
    );
    let depth = make(
        "depth",
        mesh_render::DEPTH_FORMAT,
        wgpu::TextureUsages::RENDER_ATTACHMENT,
    );
    let colour_view = colour.create_view(&Default::default());
    let depth_view = depth.create_view(&Default::default());

    let mut encoder = device.create_command_encoder(&Default::default());
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("zone"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &colour_view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: &depth_view,
                depth_ops: Some(wgpu::Operations {
                    load: wgpu::LoadOp::Clear(1.0),
                    store: wgpu::StoreOp::Store,
                }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        pass.set_pipeline(&built.pipeline);
        pass.set_bind_group(0, &bind_group, &[]);
        // Slot 1: the model's own first texture. Slot 0 is `build`'s white 1x1.
        pass.set_bind_group(1, &built.texture_binds[1], &[]);
        pass.set_bind_group(2, &built.fog_bind, &[]);
        pass.set_bind_group(3, &built.anim_bind, &[]);
        pass.set_vertex_buffer(0, built.vertex_buffer.slice(..));
        pass.set_index_buffer(built.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
        pass.draw_indexed(0..3, 0, 0..1);
    }

    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("zone readback"),
        size: u64::from(SIZE * SIZE * 4),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        colour.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(SIZE * 4),
                rows_per_image: None,
            },
        },
        extent,
    );
    queue.submit(Some(encoder.finish()));
    readback.slice(..).map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .expect("draining the queue");
    let mapped = readback.slice(..).get_mapped_range().expect("mapping");
    mapped.to_vec()
}
