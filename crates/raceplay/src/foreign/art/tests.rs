use super::*;

/// A unit quad, project-authored.
fn quad(at: f32, texture: Option<usize>, blend: Blend, lit: bool) -> ArtMesh {
    ArtMesh {
        name: format!("quad {at}"),
        positions: vec![
            [at, 0.0, 0.0],
            [at + 1.0, 0.0, 0.0],
            [at + 1.0, 0.0, 1.0],
            [at, 0.0, 1.0],
        ],
        normals: vec![[0.0, 1.0, 0.0]; 4],
        texcoords: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        colours: vec![[0.25, 0.5, 0.75, 1.0]; 4],
        indices: vec![0, 1, 2, 0, 2, 3],
        material: ArtMaterial {
            texture,
            blend,
            culled: blend == Blend::Cutout,
            lit,
            alpha_ref: None,
        },
        shadow: ShadowRole::Both,
        layer: 0,
        transform: None,
    }
}

fn texel(label: &str) -> Option<Arc<ModelTexture>> {
    Some(Arc::new(ModelTexture::rgba8(
        label.into(),
        1,
        1,
        vec![255, 0, 0, 255],
        None,
    )))
}

fn art() -> Art {
    Art {
        meshes: vec![
            quad(0.0, None, Blend::Opaque, false),
            quad(2.0, Some(0), Blend::Cutout, false),
            quad(4.0, Some(0), Blend::Additive, true),
            quad(6.0, None, Blend::Unblended, false),
        ],
        textures: vec![TextureSource::Image("t.png".into())],
        ..Art::default()
    }
}

#[test]
fn each_mesh_is_one_draw_in_the_list_its_blend_names() {
    let mut report = Vec::new();
    let m = model("t", &art(), vec![texel("t.png")], &mut report);
    assert_eq!(
        (
            m.draws.len(),
            m.alpha_tested_draws.len(),
            m.transparent_draws.len()
        ),
        (1, 1, 2)
    );
    assert_eq!(m.transparent_draws[0].blend, Some(BlendClass::Additive));
    assert_eq!(m.transparent_draws[1].blend, Some(BlendClass::None));
    assert!(m.alpha_tested_draws[0].culled && !m.draws[0].culled);
    assert_eq!(m.alpha_tested_draws[0].range, 6..12);
    assert_eq!(&m.indices[6..12], &[4, 5, 6, 4, 6, 7]);
    assert_eq!(m.vertices[8].lit, 1.0);
    assert_eq!(m.vertices[0].lit, 0.0);
    // Linear in the file, the gamma bytes the renderer draws: sRGB 137, 188, 225.
    assert_eq!(
        m.vertices[0].colour,
        [137.0 / 255.0, 188.0 / 255.0, 225.0 / 255.0, 1.0]
    );
    assert_eq!(m.draws[0].bounds.centre, [0.5, 0.0, 0.5]);
    assert_eq!(m.mesh_count, 4);
}

#[test]
fn a_mesh_whose_texture_did_not_resolve_draws_nothing_and_says_so() {
    let mut report = Vec::new();
    let m = model("t", &art(), vec![None], &mut report);
    assert_eq!(m.mesh_count, 2, "the two textured quads are left out");
    assert!(m.alpha_tested_draws.is_empty());
    assert!(
        report.iter().any(|l| l.contains("2 mesh(es) not drawn")),
        "{report:?}"
    );
}

#[test]
fn an_image_beside_the_manifest_reads_as_its_pixels() {
    let dir = std::env::temp_dir().join(format!("oag-art-png-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let pixels = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    std::fs::write(
        dir.join("t.png"),
        oag_texture::png::encode_rgba(2, 2, &pixels),
    )
    .unwrap();
    let texture = image(&dir.join("t.png")).expect("decodes");
    std::fs::remove_dir_all(&dir).ok();
    assert_eq!((texture.width, texture.height), (2, 2));
    match texture.texels {
        mesh::Texels::Rgba8(got) => assert_eq!(got, pixels),
        _ => panic!("not RGBA8"),
    }
}

#[test]
fn a_model_written_as_art_builds_the_same_draws_again() {
    let mut report = Vec::new();
    let first = model("t", &art(), vec![texel("t.png")], &mut report);
    let names = vec![Some("t.png".to_string())];
    let again = from_model(&first, "e", &names);
    let second = model("t", &again, vec![texel("t.png")], &mut report);
    for (a, b) in [
        (&first.draws, &second.draws),
        (&first.alpha_tested_draws, &second.alpha_tested_draws),
        (&first.transparent_draws, &second.transparent_draws),
    ] {
        assert_eq!(a.len(), b.len());
        for (x, y) in a.iter().zip(b) {
            assert_eq!(
                (
                    x.texture,
                    x.blend,
                    x.culled,
                    x.bounds.centre,
                    x.bounds.radius
                ),
                (
                    y.texture,
                    y.blend,
                    y.culled,
                    y.bounds.centre,
                    y.bounds.radius
                )
            );
        }
    }
    assert_eq!(first.indices.len(), second.indices.len());
    assert_eq!(
        again.textures,
        vec![TextureSource::Host {
            entry: "e".into(),
            name: "t.png".into()
        }]
    );
}

#[test]
fn the_flare_lies_along_the_sun_in_its_colour_unless_overridden() {
    let mut a = art();
    a.sun = Some(Sun {
        direction: [0.0, 1.0, 0.0],
        colour: [1.0, 0.5, 0.25],
        intensity: 3.0,
        angular_diameter: 0.5,
        shadows: true,
    });
    a.lens_flare = Some(LensFlare {
        direction: None,
        colour: None,
        atlas: Some(0),
    });
    let mut report = Vec::new();
    let flare = lens_flare(&a, &vec![texel("atlas")], &mut report).expect("draws");
    assert_eq!(flare.direction, [0.0, 1.0, 0.0]);
    assert_eq!(flare.colour, [1.0, 0.5, 0.25].map(colour::to_gamma));
    a.lens_flare = Some(LensFlare {
        direction: Some([1.0, 0.0, 0.0]),
        colour: Some([0.1, 0.2, 0.3]),
        atlas: Some(0),
    });
    let flare = lens_flare(&a, &vec![texel("atlas")], &mut report).unwrap();
    assert_eq!(flare.colour, [0.1, 0.2, 0.3].map(colour::to_gamma));
    assert_eq!(flare.direction, [1.0, 0.0, 0.0]);
    assert!(lens_flare(&a, &vec![None], &mut report).is_none());
    let extras = extras(&a, &vec![texel("atlas")], &mut report);
    assert_eq!(extras.sun_shadow, Some([0.0, 1.0, 0.0]));
}

#[test]
fn a_reference_without_a_directory_names_the_file() {
    assert!(spells(
        "LensflareComponents.tga",
        r"Data\Tracks\LENSFLARECOMPONENTS.TGA"
    ));
    assert!(spells(r"data\tracks\a.tga", r"Data\Tracks\a.tga"));
    assert!(!spells(r"Data\Other\a.tga", r"Data\Tracks\a.tga"));
}

#[test]
fn draws_go_by_layer_then_file_order_and_carry_their_alpha_reference() {
    let mut a = art();
    let mut late = quad(8.0, Some(0), Blend::Cutout, false);
    late.layer = 0x4500_0000;
    late.material.alpha_ref = Some(0);
    a.meshes[1].layer = 0x4a00_0000;
    a.meshes[1].material.alpha_ref = Some(0x7f);
    a.meshes.push(late);
    let mut report = Vec::new();
    let m = model("t", &a, vec![texel("t.png")], &mut report);
    let cutouts: Vec<_> = m
        .alpha_tested_draws
        .iter()
        .map(|d| (d.layer, d.alpha_test_ref, d.range.start))
        .collect();
    // The later mesh's lower layer goes first; its vertices stay where the file put them.
    assert_eq!(
        cutouts,
        [
            (0x4500_0000, Some(0.0), 24),
            (0x4a00_0000, Some(127.0 / 255.0), 6)
        ]
    );
    assert!(
        m.draws
            .iter()
            .all(|d| d.alpha_test_ref.is_none() && d.layer == 0)
    );
}

#[test]
fn the_hull_rig_is_the_ambient_then_the_sun_then_the_light_rows() {
    let mut a = art();
    assert!(hull_lights(&a).is_none(), "no light, no rig");
    a.ambient = Some([colour::to_linear(48.0 / 255.0); 3]);
    a.sun = Some(Sun {
        direction: [0.0, 1.0, 0.0],
        colour: [0.5, 0.25, 1.0],
        intensity: 2.0,
        angular_diameter: 0.5,
        shadows: true,
    });
    a.lights.push(Light::Directional {
        direction: [1.0, 0.0, 0.0],
        colour: [colour::to_linear(0.5); 3],
        intensity: 1.0,
    });
    let h = hull_lights(&a).expect("a rig");
    assert!(h.enabled());
    assert_eq!(h.ambient, [48.0 / 255.0, 48.0 / 255.0, 48.0 / 255.0, 1.0]);
    assert_eq!(h.direction[0], [0.0, 1.0, 0.0, 1.0]);
    // Colour times intensity, then the curve, then clipped to a byte: 1.0 and 2.0 clip.
    let sun = h.diffuse[0].map(|c| (c * 255.0).round() as u32);
    assert_eq!(sun, [255, 188, 255, 0]);
    assert_eq!(h.direction[1], [1.0, 0.0, 0.0, 1.0]);
    assert_eq!((h.diffuse[1][0] * 255.0).round() as u32, 128);
    assert_eq!(h.direction[2][3], 0.0, "slot 2 stays off");
}

#[test]
fn an_exported_colour_comes_back_as_its_byte() {
    let mut report = Vec::new();
    let mut a = art();
    let bytes = [0u8, 1, 17, 128, 254, 255];
    a.meshes[0].colours = (0..4)
        .map(|i| {
            let b = bytes[i];
            [f32::from(b) / 255.0; 4]
        })
        .collect();
    // The `.vex` path's floats, through the exporter's curve and back.
    let built = Model {
        vertices: a.meshes[0]
            .colours
            .iter()
            .map(|&c| GpuVertex {
                colour: c,
                ..vertex(&a.meshes[0], 0, 0.0)
            })
            .collect(),
        ..model("t", &art(), vec![texel("t.png")], &mut report)
    };
    let mut first = built.clone();
    first.vertices.truncate(4);
    first.indices = vec![0, 1, 2, 0, 2, 3];
    first.draws.truncate(1);
    first.draws[0].range = 0..6;
    first.alpha_tested_draws.clear();
    first.transparent_draws.clear();
    let again = from_model(&first, "e", &[]);
    let second = model("t", &again, vec![], &mut report);
    let got: Vec<_> = second.vertices.iter().map(|v| v.colour).collect();
    let want: Vec<_> = first.vertices.iter().map(|v| v.colour).collect();
    assert_eq!(got, want);
}

/// `art()` with its first two meshes moving on the same tracks (one object's
/// two material slots, as the add-on writes them) and the sun turning a
/// quarter turn about +X over four seconds.
fn moving() -> Art {
    use oag_trackfile::art::motion::{
        Animation, Channel, Interpolation, Keys, Repeat, Target, Trs,
    };
    let mut a = art();
    let rest = Trs {
        translation: [100.0, 0.0, 0.0],
        ..Trs::IDENTITY
    };
    a.meshes[0].transform = Some(rest);
    a.meshes[1].transform = Some(rest);
    let slide = |m: usize| Channel {
        target: Target::Mesh(m),
        interpolation: Interpolation::Linear,
        times: vec![0.0, 2.0],
        keys: Keys::Translation(vec![[100.0, 0.0, 0.0], [100.0, 0.0, 40.0]]),
    };
    let h = std::f32::consts::FRAC_1_SQRT_2;
    a.sun = Some(Sun {
        direction: [0.0, 1.0, 0.0],
        colour: [1.0; 3],
        intensity: 1.0,
        angular_diameter: 0.5,
        shadows: true,
    });
    a.sun_rotation = Some([0.0, 0.0, 0.0, 1.0]);
    a.animations = vec![
        Animation {
            name: "slide".into(),
            repeat: Repeat::Clamp,
            channels: vec![slide(0), slide(1)],
        },
        Animation {
            name: "sun".into(),
            repeat: Repeat::Loop,
            channels: vec![Channel {
                target: Target::Sun,
                interpolation: Interpolation::Linear,
                times: vec![0.0, 4.0],
                keys: Keys::Rotation(vec![[0.0, 0.0, 0.0, 1.0], [h, 0.0, 0.0, h]]),
            }],
        },
    ];
    a
}

/// A moving mesh rides one node slot of the model's table, its draw is moving
/// (drawn unculled, a moving caster for the `mapped` tier), the node is where
/// the keys put it at a time and holds past the end of a clamped animation,
/// two meshes on the same tracks share one node, and the model's sphere
/// reaches where the motion goes.
#[test]
fn a_moving_mesh_rides_a_node_slot_on_the_animation_clock() {
    let mut report = Vec::new();
    let m = model("t", &moving(), vec![texel("t.png")], &mut report);
    assert_eq!(m.anim_nodes.len(), 1, "{report:?}");
    assert!(m.draws[0].moving && m.alpha_tested_draws[0].moving);
    assert!(!m.transparent_draws[0].moving);
    assert!(m.vertices[..8].iter().all(|v| v.xform == 1));
    assert!(m.vertices[8..].iter().all(|v| v.xform == 0));
    let at = |s: f32| vex::transform_point(&m.sample_anim_nodes(s)[0], m.vertices[0].position);
    assert_eq!(at(0.0), [100.0, 0.0, 0.0]);
    assert_eq!(at(1.0), [100.0, 0.0, 20.0]);
    assert_eq!(at(9.0), [100.0, 0.0, 40.0]);
    assert!(m.radius > 40.0, "{}", m.radius);
    assert!(
        report.iter().any(|l| l.contains("2 mesh(es) on 1 node(s)")),
        "{report:?}"
    );
}

/// The sun's direction turns by its node's rotation from rest: a quarter turn
/// about +X takes "straight up" to +Z, halfway is the diagonal, and the loop
/// brings it back.
#[test]
fn a_turning_sun_turns_the_shadow_direction() {
    let mut report = Vec::new();
    let x = extras(&moving(), &Vec::new(), &mut report);
    let turn = x.sun_turn.expect("the sun turns");
    let near = |a: [f32; 3], b: [f32; 3]| a.iter().zip(&b).all(|(p, q)| (p - q).abs() < 1e-5);
    let h = std::f32::consts::FRAC_1_SQRT_2;
    assert!(near(turn.direction_at(0.0), [0.0, 1.0, 0.0]));
    assert!(
        near(turn.direction_at(2.0), [0.0, h, h]),
        "{:?}",
        turn.direction_at(2.0)
    );
    assert!(near(turn.direction_at(4.0), [0.0, 1.0, 0.0]));
    assert!(near(turn.direction_at(3.999), [0.0, 0.0, 1.0]) || turn.direction_at(3.999)[2] > 0.99);
}
