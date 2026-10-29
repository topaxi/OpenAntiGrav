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
    let bad = p.manifest.replace("/v1/track.tosd", "/v2/track.tosd");
    let err = from_parts(&bad, &p.glb).unwrap_err();
    assert!(
        matches!(
            err,
            Error::Schema(oag_schema::Error::UnknownMajor {
                found: 2,
                supported: 1,
                ..
            })
        ),
        "{err}"
    );
}

#[test]
fn a_key_added_to_the_schema_since_reads_and_is_named_as_unread() {
    let p = parts();
    let newer = p.manifest.replace("[spline]", "future_key = 3\n\n[spline]");
    let loaded = from_parts(&newer, &p.glb).unwrap();
    assert!(
        loaded.unread.iter().any(|u| u.contains("future_key")),
        "{:?}",
        loaded.unread
    );
}

#[test]
fn another_kind_and_a_missing_header_are_refused_by_name() {
    let p = parts();
    let other = p.manifest.replace("track.tosd", "pilot.tosd");
    let err = from_parts(&other, &p.glb).unwrap_err();
    assert!(
        matches!(err, Error::Schema(oag_schema::Error::WrongKind { .. })),
        "{err}"
    );
    // A pre-schema file: `format`/`version` under [track], no header.
    let old = p.manifest.split("[track]").nth(1).unwrap();
    let old = format!(
        "[track]\nformat = \"oag-track\"\nversion = \"1.1\"\n{}",
        old.trim_start_matches('\n')
    );
    let err = from_parts(&old, &p.glb).unwrap_err();
    assert!(
        matches!(err, Error::Schema(oag_schema::Error::Missing { .. })),
        "{err}"
    );
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

/// `track.host` is required exactly when the art borrows from a disc (ADR-0076
/// decision 13): a pinned track without one, or an empty one, is refused by
/// name on write and on read; a track that borrows nothing needs none, and
/// `titles` round-trips as written.
#[test]
fn a_track_whose_art_borrows_from_a_disc_must_name_its_host() {
    use oag_trackfile::art::{Art, TextureSource};
    let plain = parts();
    assert!(!plain.manifest.contains("host ="), "{}", plain.manifest);
    let pinned = Track {
        host: Some("wipeout-pulse".into()),
        titles: vec!["wipeout-pulse".into(), "wipeout-hd".into()],
        art: Some(Art {
            sky: Some(r"Data\Environments\03_Track\track.vex".into()),
            ..Art::default()
        }),
        ..fixture::oval()
    };
    let p = to_parts(&pinned, "oval.glb").expect("writable");
    let loaded = from_parts(&p.manifest, &p.glb).expect("reads");
    assert_eq!(loaded.track.host.as_deref(), Some("wipeout-pulse"));
    assert_eq!(loaded.track.titles, pinned.titles);

    let unnamed = Track {
        host: None,
        ..pinned.clone()
    };
    assert!(
        matches!(to_parts(&unnamed, "x.glb"), Err(Error::Unwritable(m)) if m.contains("track.host"))
    );
    let line = "host = \"wipeout-pulse\"\n";
    assert!(p.manifest.contains(line), "{}", p.manifest);
    let err = from_parts(&p.manifest.replace(line, ""), &p.glb).unwrap_err();
    assert!(
        matches!(&err, Error::Manifest(m) if m.contains("track.host")),
        "{err}"
    );
    let err = from_parts(&p.manifest.replace(line, "host = \"\"\n"), &p.glb).unwrap_err();
    assert!(
        matches!(&err, Error::Manifest(m) if m.contains("track.host")),
        "{err}"
    );

    let mut host_texture = Art::default();
    host_texture.textures.push(TextureSource::Host {
        entry: r"Data\Tracks\01_Track.vex".into(),
        name: "a".into(),
    });
    assert!(host_texture.borrows_from_a_disc());
    assert!(!Art::default().borrows_from_a_disc());
}

#[test]
fn the_original_ai_bytes_round_trip_when_a_point_carries_them() {
    let mut track = fixture::oval();
    let points = &mut track.ai.paths[0].points;
    points[3].unk_0x5c = Some(40);
    points[3].unk_0x5d = Some(200);
    points[3].unk_0x66 = Some(0);
    let p = to_parts(&track, "oval.glb").expect("writable");
    let loaded = from_parts(&p.manifest, &p.glb).expect("reads");
    assert_eq!(loaded.track, track);
}
