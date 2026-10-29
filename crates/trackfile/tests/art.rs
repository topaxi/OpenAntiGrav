//! The art section (ADR-0076): a round trip that is exact, a file with no art
//! that is what 1.0 wrote, and every way an art section is refused by name.

use oag_trackfile::art::{
    Art, ArtMaterial, ArtMesh, Blend, LensFlare, Light, ShadowRole, Sun, TextureSource,
};
use oag_trackfile::{Error, Track, fixture, from_parts, to_parts};

/// Two triangles of a unit quad: project-authored, never disc content.
fn quad(name: &str, texture: Option<usize>) -> ArtMesh {
    ArtMesh {
        name: name.to_string(),
        positions: vec![
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [1.0, 0.0, 1.0],
            [0.0, 0.0, 1.0],
        ],
        normals: vec![[0.0, 1.0, 0.0]; 4],
        texcoords: vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]],
        colours: vec![[0.25, 0.5, 0.75, 1.0]; 4],
        indices: vec![0, 1, 2, 0, 2, 3],
        material: ArtMaterial {
            texture,
            blend: Blend::Cutout,
            culled: false,
            lit: false,
            alpha_ref: None,
        },
        shadow: ShadowRole::Cast,
        layer: 0,
        transform: None,
        pad: None,
    }
}

pub(crate) fn art() -> Art {
    let mut kerb = quad("kerb", Some(0));
    kerb.material.alpha_ref = Some(0);
    kerb.layer = 0x4a00_0000;
    Art {
        meshes: vec![kerb, quad("bare", None)],
        textures: vec![
            TextureSource::Image("checker.png".into()),
            TextureSource::Host {
                entry: r"Data\Tracks\01_Track.vex".into(),
                name: "LensflareComponents.tga".into(),
            },
        ],
        sky: Some(r"Data\Tracks\01_Track.vex".into()),
        skybox: None,
        sun: Some(Sun {
            direction: [0.0, 1.0, 0.0],
            colour: [1.0, 0.9, 0.7],
            intensity: 2.5,
            angular_diameter: 0.53,
            shadows: true,
        }),
        ambient: Some([0.1, 0.1, 0.15]),
        lights: vec![Light::Directional {
            direction: [1.0, 0.0, 0.0],
            colour: [0.2, 0.3, 0.4],
            intensity: 1.0,
        }],
        lens_flare: Some(LensFlare {
            direction: None,
            colour: None,
            atlas: Some(1),
        }),
        animations: Vec::new(),
        sun_rotation: None,
    }
}

pub(crate) fn with_art(art: Art) -> Track {
    Track {
        // The art borrows host textures, so the track names their disc.
        host: art
            .borrows_from_a_disc()
            .then(|| "wipeout-pulse".to_string()),
        art: Some(art),
        ..fixture::oval()
    }
}

fn round(track: &Track) -> Track {
    let p = to_parts(track, "t.glb").expect("writable");
    let loaded = from_parts(&p.manifest, &p.glb).expect("reads");
    assert!(loaded.unread.is_empty(), "{:?}", loaded.unread);
    loaded.track
}

#[test]
fn art_reads_back_identical_and_stays_byte_stable() {
    let track = with_art(art());
    let back = round(&track);
    assert_eq!(back, track);
    assert_eq!(
        to_parts(&track, "t.glb").unwrap(),
        to_parts(&back, "t.glb").unwrap()
    );
}

#[test]
fn a_track_without_art_writes_no_art_keys() {
    let p = to_parts(&fixture::oval(), "t.glb").unwrap();
    assert!(!p.manifest.contains("art"), "{}", p.manifest);
    assert_eq!(round(&fixture::oval()).art, None);
}

#[test]
fn an_empty_art_section_survives() {
    let track = with_art(Art::default());
    assert_eq!(round(&track).art, Some(Art::default()));
}

#[test]
fn a_texture_that_names_nothing_is_refused_by_name() {
    let mut a = art();
    a.meshes[0].material.texture = Some(9);
    let err = to_parts(&with_art(a), "t.glb").unwrap_err();
    assert!(matches!(err, Error::Range { .. }), "{err}");
    assert!(err.to_string().contains("texture 9"), "{err}");
}

#[test]
fn a_flare_with_no_sun_is_refused() {
    let mut a = art();
    a.sun = None;
    let err = to_parts(&with_art(a), "t.glb").unwrap_err();
    assert!(err.to_string().contains("lensflare"), "{err}");
}

#[test]
fn a_sun_direction_that_is_not_a_unit_vector_is_refused() {
    let mut a = art();
    a.sun.as_mut().unwrap().direction = [0.0, 2.0, 0.0];
    let err = to_parts(&with_art(a), "t.glb").unwrap_err();
    assert!(err.to_string().contains("not 1"), "{err}");
}

#[test]
fn a_mesh_whose_arrays_disagree_is_refused() {
    let mut a = art();
    a.meshes[0].colours.pop();
    let err = to_parts(&with_art(a), "t.glb").unwrap_err();
    assert!(matches!(err, Error::Accessor { .. }), "{err}");
}

#[test]
fn an_unknown_art_manifest_key_is_reported_as_unread() {
    let p = to_parts(&with_art(art()), "t.glb").unwrap();
    let manifest = p.manifest.replacen("[art]\n", "[art]\nfuture = 1\n", 1);
    let loaded = from_parts(&manifest, &p.glb).unwrap();
    assert!(
        loaded.unread.iter().any(|u| u.contains("art.future")),
        "{:?}",
        loaded.unread
    );
}

#[test]
fn a_manifest_that_counts_art_the_glb_lacks_is_refused() {
    let p = to_parts(&with_art(art()), "t.glb").unwrap();
    let none = to_parts(&fixture::oval(), "t.glb").unwrap();
    let err = from_parts(&p.manifest, &none.glb).unwrap_err();
    assert!(matches!(err, Error::CountMismatch { .. }), "{err}");
}

#[test]
fn a_sun_that_does_not_say_casts() {
    let p = to_parts(&with_art(art()), "t.glb").unwrap();
    assert!(p.manifest.contains("shadows = true"), "{}", p.manifest);
    let manifest = p.manifest.replacen("shadows = true\n", "", 1);
    let loaded = from_parts(&manifest, &p.glb).unwrap();
    assert!(loaded.track.art.unwrap().sun.unwrap().shadows);
}

#[test]
fn a_light_row_that_is_not_a_unit_vector_is_refused() {
    let mut a = art();
    a.lights[0] = Light::Directional {
        direction: [0.0, 0.5, 0.0],
        colour: [1.0; 3],
        intensity: 1.0,
    };
    let err = to_parts(&with_art(a), "t.glb").unwrap_err();
    assert!(err.to_string().contains("art light 0"), "{err}");
}

#[test]
fn absent_layer_and_alpha_ref_read_as_zero_and_none() {
    let back = round(&with_art(art()));
    let meshes = &back.art.unwrap().meshes;
    assert_eq!(
        (meshes[0].layer, meshes[0].material.alpha_ref),
        (0x4a00_0000, Some(0))
    );
    assert_eq!((meshes[1].layer, meshes[1].material.alpha_ref), (0, None));
}

/// The own sky (ADR-0076 decision 14) round-trips, and every way it can be
/// wrong is refused by name: beside a host sky, half a gradient, a face with
/// neither a texture nor the gradient, a face row past the textures.
#[test]
fn an_own_skybox_round_trips_and_is_refused_where_it_cannot_draw() {
    use oag_trackfile::art::Skybox;
    let mut a = art();
    a.sky = None;
    a.skybox = Some(Skybox {
        faces: [None, None, Some(0), None, None, None],
        zenith: Some([0.1, 0.2, 0.6]),
        horizon: Some([0.7, 0.8, 0.9]),
    });
    let track = with_art(a.clone());
    assert_eq!(round(&track), track);
    let gradient_only = Skybox {
        faces: [None; 6],
        ..a.skybox.unwrap()
    };
    let mut g = a.clone();
    g.skybox = Some(gradient_only);
    assert_eq!(round(&with_art(g.clone())), with_art(g));

    let refused = |edit: &dyn Fn(&mut Art)| {
        let mut bad = a.clone();
        edit(&mut bad);
        assert!(
            matches!(to_parts(&with_art(bad), "t.glb"), Err(Error::Range { .. })),
            "accepted"
        );
    };
    refused(&|b| b.sky = Some(r"Data\Tracks\01_Track.vex".into()));
    refused(&|b| b.skybox.as_mut().unwrap().zenith = None);
    refused(&|b| {
        let s = b.skybox.as_mut().unwrap();
        (s.zenith, s.horizon) = (None, None);
    });
    refused(&|b| b.skybox.as_mut().unwrap().faces[0] = Some(9));
}
/// A pad plate (`oag_pad`) reads back as tagged; an unknown tag and a plate an
/// animation moves are refused by name.
#[test]
fn a_pad_plate_round_trips_and_is_refused_when_wrong() {
    use oag_trackfile::art::PadPlate;
    let mut a = art();
    a.meshes[1].pad = Some(PadPlate::Weapon);
    let track = with_art(a);
    let back = round(&track);
    assert_eq!(
        back.art.as_ref().unwrap().meshes[1].pad,
        Some(PadPlate::Weapon)
    );
    assert_eq!(back, track);

    let p = to_parts(&track, "t.glb").unwrap();
    let mut glb = p.glb.clone();
    let from = b"\"oag_pad\":\"weapon\"";
    let at = glb
        .windows(from.len())
        .position(|w| w == from)
        .expect("tagged");
    glb[at..at + from.len()].copy_from_slice(b"\"oag_pad\":\"wobble\"");
    let err = from_parts(&p.manifest, &glb).unwrap_err();
    assert!(err.to_string().contains("oag_pad"), "{err}");

    let mut moving = crate::motion::moving();
    moving.art.as_mut().unwrap().meshes[1].pad = Some(PadPlate::Speedup);
    let err = to_parts(&moving, "t.glb").unwrap_err();
    assert!(err.to_string().contains("pads never move"), "{err}");
}
