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
        },
        shadow: ShadowRole::Both,
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
    assert_eq!(m.vertices[0].colour, [0.25, 0.5, 0.75, 1.0]);
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
        colour: None,
        atlas: Some(0),
    });
    let mut report = Vec::new();
    let flare = lens_flare(&a, &vec![texel("atlas")], &mut report).expect("draws");
    assert_eq!(flare.direction, [0.0, 1.0, 0.0]);
    assert_eq!(flare.colour, [1.0, 0.5, 0.25]);
    a.lens_flare = Some(LensFlare {
        colour: Some([0.1, 0.2, 0.3]),
        atlas: Some(0),
    });
    let flare = lens_flare(&a, &vec![texel("atlas")], &mut report).unwrap();
    assert_eq!(flare.colour, [0.1, 0.2, 0.3]);
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
