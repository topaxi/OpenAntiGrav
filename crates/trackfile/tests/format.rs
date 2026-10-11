//! The track-file format, with no disc and no Workshop item: a project-authored
//! track, its committed fixture, and every way a file is refused by name.

use oag_trackfile::{Error, Parts, Track, fixture, from_parts, to_parts};

fn fixture_dir() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")
}

fn parts() -> Parts {
    to_parts(&fixture::oval(), "oval.glb").expect("the fixture is writable")
}

#[test]
fn a_written_track_reads_back_identical() {
    let track = fixture::oval();
    let p = parts();
    let loaded = from_parts(&p.manifest, &p.glb).expect("reads");
    assert_eq!(loaded.track, track);
    assert_eq!(loaded.version, (1, 0));
    assert!(loaded.unread.is_empty(), "{:?}", loaded.unread);
}

#[test]
fn a_round_trip_is_bit_exact_and_writes_the_same_bytes_again() {
    let first = parts();
    let loaded = from_parts(&first.manifest, &first.glb).unwrap().track;
    let again = to_parts(&loaded, "oval.glb").unwrap();
    assert_eq!(first, again, "write, read, write must be byte-stable");
    // PartialEq on f32 cannot tell 0.0 from -0.0; the bits can.
    let a = fixture::oval();
    for (x, y) in a.ai.paths[0].points.iter().zip(&loaded.ai.paths[0].points) {
        assert_eq!(x.pos.map(f32::to_bits), y.pos.map(f32::to_bits));
        assert_eq!(x.progress.to_bits(), y.progress.to_bits());
    }
    assert_eq!(
        a.ai.paths[0].max_spacing.to_bits(),
        loaded.ai.paths[0].max_spacing.to_bits()
    );
}

#[test]
fn the_committed_fixture_is_what_the_generator_writes() {
    let p = parts();
    let manifest = std::fs::read_to_string(fixture_dir().join("oval.track.toml")).unwrap();
    let glb = std::fs::read(fixture_dir().join("oval.glb")).unwrap();
    assert_eq!(
        manifest, p.manifest,
        "run `cargo run -p oag-trackfile --example make_fixture`"
    );
    assert_eq!(
        glb, p.glb,
        "run `cargo run -p oag-trackfile --example make_fixture`"
    );
    let loaded =
        oag_trackfile::read(&fixture_dir().join("oval.track.toml")).expect("reads from disk");
    assert_eq!(loaded.track, fixture::oval());
    let t: &Track = &loaded.track;
    assert_eq!(t.ai.paths[0].points.len(), 64);
    assert_eq!(
        (t.speedup_pads.len(), t.weapon_pads.len(), t.nodes.len()),
        (1, 1, 2)
    );
}

#[test]
fn an_unknown_major_is_refused_by_name() {
    let p = parts();
    let bad = p.manifest.replace("version = \"1.0\"", "version = \"2.0\"");
    let err = from_parts(&bad, &p.glb).unwrap_err();
    assert!(matches!(err, Error::UnknownMajor { major: 2, .. }), "{err}");
}

#[test]
fn a_newer_minor_reads_and_names_the_keys_it_skipped() {
    let p = parts();
    let newer = p
        .manifest
        .replace("version = \"1.0\"", "version = \"1.7\"")
        .replace("[spline]", "future_key = 3\n\n[spline]");
    let loaded = from_parts(&newer, &p.glb).unwrap();
    assert_eq!(loaded.version, (1, 7));
    assert!(
        loaded.unread.iter().any(|u| u.contains("future_key")),
        "{:?}",
        loaded.unread
    );
}

#[test]
fn another_format_and_a_bad_version_string_are_refused() {
    let p = parts();
    let other = p
        .manifest
        .replace("format = \"oag-track\"", "format = \"other\"");
    assert!(matches!(
        from_parts(&other, &p.glb),
        Err(Error::NotThisFormat(_))
    ));
    let odd = p.manifest.replace("version = \"1.0\"", "version = \"one\"");
    assert!(matches!(
        from_parts(&odd, &p.glb),
        Err(Error::BadVersion(_))
    ));
}

#[test]
fn a_count_the_manifest_and_the_glb_disagree_on_is_refused() {
    let p = parts();
    let bad = p.manifest.replace("speedup_pads = 1", "speedup_pads = 2");
    let err = from_parts(&bad, &p.glb).unwrap_err();
    assert!(matches!(err, Error::CountMismatch { .. }), "{err}");
    let bad = p.manifest.replace("points = 64", "points = 63");
    let err = from_parts(&bad, &p.glb).unwrap_err();
    assert!(matches!(err, Error::CountMismatch { .. }), "{err}");
}

#[test]
fn a_junction_pointing_past_the_paths_is_refused() {
    let p = parts();
    let bad = p.manifest.replace("prev = [0, -1]", "prev = [5, -1]");
    let err = from_parts(&bad, &p.glb).unwrap_err();
    assert!(matches!(err, Error::Range { .. }), "{err}");
}

fn glb_json(glb: &[u8]) -> (serde_json::Value, usize, usize) {
    let len = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
    (serde_json::from_slice(&glb[20..20 + len]).unwrap(), 20, len)
}

fn with_json(glb: &[u8], edit: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
    let (mut json, start, len) = glb_json(glb);
    edit(&mut json);
    let mut text = serde_json::to_vec(&json).unwrap();
    while text.len() % 4 != 0 {
        text.push(b' ');
    }
    let mut out = glb[..12].to_vec();
    out.extend((text.len() as u32).to_le_bytes());
    out.extend(b"JSON");
    out.extend(&text);
    out.extend(&glb[start + len..]);
    let total = out.len() as u32;
    out[8..12].copy_from_slice(&total.to_le_bytes());
    out
}

#[test]
fn an_unknown_collision_tag_a_transformed_node_and_a_missing_attribute_are_refused() {
    let p = parts();
    let bad = with_json(&p.glb, |j| {
        for n in j["nodes"].as_array_mut().unwrap() {
            if n["extras"]["oag_kind"] == "wall" {
                n["extras"]["oag_kind"] = "scenery".into();
            }
        }
    });
    assert!(matches!(
        from_parts(&p.manifest, &bad),
        Err(Error::UnknownKind { .. })
    ));

    let bad = with_json(&p.glb, |j| {
        for n in j["nodes"].as_array_mut().unwrap() {
            if n["name"] == "oag/start" {
                n["translation"] = serde_json::json!([1.0, 0.0, 0.0]);
            }
        }
    });
    assert!(matches!(
        from_parts(&p.manifest, &bad),
        Err(Error::Transformed { .. })
    ));

    let bad = with_json(&p.glb, |j| {
        for m in j["meshes"].as_array_mut().unwrap() {
            if m["name"] == "oag/start" {
                m["primitives"][0]["attributes"]
                    .as_object_mut()
                    .unwrap()
                    .remove("_OAG_UP");
            }
        }
    });
    let err = from_parts(&p.manifest, &bad).unwrap_err();
    assert!(matches!(err, Error::Accessor { .. }), "{err}");
}

#[test]
fn a_truncated_glb_is_refused() {
    let p = parts();
    assert!(matches!(
        from_parts(&p.manifest, &p.glb[..p.glb.len() / 2]),
        Err(Error::Glb(_))
    ));
}

#[test]
fn an_unwritable_track_is_refused_before_a_file_exists() {
    let mut t = fixture::oval();
    t.start.position[0] = f32::NAN;
    assert!(matches!(
        to_parts(&t, "x.glb"),
        Err(Error::NonFinite { .. })
    ));
    let mut t = fixture::oval();
    t.nodes[0].geometry.meshes[0].triangles.clear();
    assert!(matches!(to_parts(&t, "x.glb"), Err(Error::Unwritable(_))));
    let mut t = fixture::oval();
    t.id.clear();
    assert!(matches!(to_parts(&t, "x.glb"), Err(Error::Unwritable(_))));
}
