//! The art section (ADR-0076): a round trip that is exact, a file with no art
//! that is what 1.0 wrote, and every way an art section is refused by name.

use oag_trackfile::art::{
    Art, ArtMaterial, ArtMesh, Blend, LensFlare, ShadowRole, Sun, TextureSource,
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
        },
        shadow: ShadowRole::Cast,
    }
}

fn art() -> Art {
    Art {
        meshes: vec![quad("kerb", Some(0)), quad("bare", None)],
        textures: vec![
            TextureSource::Image("checker.png".into()),
            TextureSource::Host {
                entry: r"Data\Tracks\01_Track.vex".into(),
                name: "LensflareComponents.tga".into(),
            },
        ],
        sky: Some(r"Data\Tracks\01_Track.vex".into()),
        sun: Some(Sun {
            direction: [0.0, 1.0, 0.0],
            colour: [1.0, 0.9, 0.7],
            intensity: 2.5,
            angular_diameter: 0.53,
            shadows: true,
        }),
        ambient: Some([0.1, 0.1, 0.15]),
        lens_flare: Some(LensFlare {
            colour: None,
            atlas: Some(1),
        }),
    }
}

fn with_art(art: Art) -> Track {
    Track {
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
