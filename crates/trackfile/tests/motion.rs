//! Keyframed motion of the art (ADR-0076 decision 10): animations read back bit
//! for bit and byte-stable, a file without motion writes no `animations`, and
//! every way motion is refused by name.

use oag_trackfile::art::motion::{Animation, Channel, Interpolation, Keys, Repeat, Target, Trs};
use oag_trackfile::{Error, Track, from_parts, to_parts};

use crate::art::{art, with_art};

/// The art of `art()` with its second mesh moving (a rest pose, a translation
/// and a stepped rotation, clamped) and its sun turning (looped).
pub(crate) fn moving() -> Track {
    let mut a = art();
    let h = std::f32::consts::FRAC_1_SQRT_2;
    a.meshes[1].transform = Some(Trs {
        translation: [10.0, 2.5, -0.1],
        rotation: [0.0, h, 0.0, h],
        scale: [1.0, 2.0, 1.0],
    });
    a.sun_rotation = Some([0.0, 0.0, 0.0, 1.0]);
    a.animations = vec![
        Animation {
            name: "airship".into(),
            repeat: Repeat::Clamp,
            on: Some("test.gate/open".into()),
            on_value: Some(2.0),
            channels: vec![
                Channel {
                    target: Target::Mesh(1),
                    interpolation: Interpolation::Linear,
                    times: vec![0.0, 1.0 / 3.0, 4.0],
                    keys: Keys::Translation(vec![
                        [10.0, 2.5, -0.1],
                        [11.0, 2.5, 0.0],
                        [20.0, 3.0, 0.1],
                    ]),
                },
                Channel {
                    target: Target::Mesh(1),
                    interpolation: Interpolation::Step,
                    times: vec![0.0, 2.0],
                    keys: Keys::Rotation(vec![[0.0, h, 0.0, h], [0.0, 0.0, 0.0, 1.0]]),
                },
            ],
        },
        Animation {
            name: "day".into(),
            repeat: Repeat::Loop,
            on_value: None,
            on: None,
            channels: vec![Channel {
                target: Target::Sun,
                interpolation: Interpolation::Linear,
                times: vec![0.0, 60.0],
                keys: Keys::Rotation(vec![[0.0, 0.0, 0.0, 1.0], [h, 0.0, 0.0, h]]),
            }],
        },
    ];
    with_art(a)
}

fn bits(track: &Track) -> Vec<u32> {
    let art = track.art.as_ref().unwrap();
    let mut out = Vec::new();
    for ch in art.animations.iter().flat_map(|a| &a.channels) {
        out.extend(ch.times.iter().map(|t| t.to_bits()));
        match &ch.keys {
            Keys::Translation(k) | Keys::Scale(k) => {
                out.extend(k.iter().flatten().map(|v| v.to_bits()))
            }
            Keys::Rotation(k) => out.extend(k.iter().flatten().map(|v| v.to_bits())),
        }
    }
    let t = art.meshes[1].transform.unwrap();
    out.extend(
        t.translation
            .iter()
            .chain(&t.rotation)
            .chain(&t.scale)
            .map(|v| v.to_bits()),
    );
    out
}

#[test]
fn motion_reads_back_bit_for_bit_and_stays_byte_stable() {
    let track = moving();
    let p = to_parts(&track, "t.glb").expect("writable");
    let loaded = from_parts(&p.manifest, &p.glb).expect("reads");
    assert!(loaded.unread.is_empty(), "{:?}", loaded.unread);
    assert_eq!(loaded.track, track);
    assert_eq!(bits(&loaded.track), bits(&track));
    assert_eq!(to_parts(&loaded.track, "t.glb").unwrap(), p);
    let json = String::from_utf8_lossy(&p.glb);
    assert!(json.contains("\"oag_loop\":\"clamp\""), "{json}");
    assert!(json.contains("\"name\":\"oag/sun\""), "{json}");
}

#[test]
fn art_without_motion_writes_no_animations() {
    let p = to_parts(&with_art(art()), "t.glb").unwrap();
    let json = String::from_utf8_lossy(&p.glb);
    assert!(!json.contains("animations"), "{json}");
    assert!(!json.contains("oag/sun"), "{json}");
}

fn refused(track: &Track, says: &str) {
    let err = to_parts(track, "t.glb").unwrap_err();
    assert!(err.to_string().contains(says), "{err}");
}

#[test]
fn motion_is_refused_by_name() {
    let mut t = moving();
    let a = t.art.as_mut().unwrap();
    a.animations[0].channels[0].times = vec![0.0, 2.0, 1.0];
    refused(&t, "strictly increase");

    let mut t = moving();
    t.art.as_mut().unwrap().animations[0].channels[0].keys = Keys::Translation(vec![[0.0; 3]]);
    refused(&t, "1 keys for 3 times");

    let mut t = moving();
    t.art.as_mut().unwrap().animations[1].channels[0].keys =
        Keys::Translation(vec![[0.0; 3], [1.0, 0.0, 0.0]]);
    refused(&t, "the sun takes rotation only");

    let mut t = moving();
    t.art.as_mut().unwrap().animations[0].channels[1].target = Target::Mesh(9);
    refused(&t, "mesh 9");

    let mut t = moving();
    let a = t.art.as_mut().unwrap();
    a.animations[0].channels[1].keys = Keys::Translation(vec![[0.0; 3], [1.0; 3]]);
    refused(&t, "a second channel drives");

    // A rest pose on a mesh nothing moves: bake it.
    let mut t = moving();
    let a = t.art.as_mut().unwrap();
    a.meshes[0].transform = Some(Trs::IDENTITY);
    refused(&t, "apply it before export");

    let mut t = moving();
    t.art.as_mut().unwrap().sun_rotation = None;
    refused(&t, "no rest rotation");
}

/// `glb` with its JSON chunk rewritten by `edit`.
fn rejson(glb: &[u8], edit: impl FnOnce(&mut serde_json::Value)) -> Vec<u8> {
    let word = |at: usize| u32::from_le_bytes(glb[at..at + 4].try_into().unwrap()) as usize;
    let json_len = word(12);
    let mut doc: serde_json::Value = serde_json::from_slice(&glb[20..20 + json_len]).unwrap();
    edit(&mut doc);
    let mut text = serde_json::to_vec(&doc).unwrap();
    while !text.len().is_multiple_of(4) {
        text.push(b' ');
    }
    let bin = &glb[20 + json_len..];
    let mut out = Vec::new();
    out.extend_from_slice(b"glTF");
    out.extend_from_slice(&2u32.to_le_bytes());
    out.extend_from_slice(&((12 + 8 + text.len() + bin.len()) as u32).to_le_bytes());
    out.extend_from_slice(&(text.len() as u32).to_le_bytes());
    out.extend_from_slice(b"JSON");
    out.extend_from_slice(&text);
    out.extend_from_slice(bin);
    out
}

/// A reader meets glTF it did not write: a cubic spline, an animated collision
/// node, a rest pose on an art node nothing moves.
#[test]
fn foreign_motion_is_refused_by_name_on_read() {
    let p = to_parts(&moving(), "t.glb").unwrap();
    let read = |glb: Vec<u8>| from_parts(&p.manifest, &glb).unwrap_err().to_string();

    let cubic = rejson(&p.glb, |d| {
        d["animations"][0]["samplers"][0]["interpolation"] = "CUBICSPLINE".into();
    });
    assert!(read(cubic).contains("CUBICSPLINE"));

    let collision = rejson(&p.glb, |d| {
        let at = d["nodes"]
            .as_array()
            .unwrap()
            .iter()
            .position(|n| n["name"] == "oag/collision/0")
            .unwrap();
        d["animations"][0]["channels"][0]["target"]["node"] = at.into();
    });
    let err = read(collision);
    assert!(err.contains("only art meshes and the sun move"), "{err}");

    let still = rejson(&p.glb, |d| {
        d.as_object_mut().unwrap().remove("animations");
        // The sun is the last node.
        d["nodes"].as_array_mut().unwrap().pop();
        d["scenes"][0]["nodes"].as_array_mut().unwrap().pop();
    });
    let err = from_parts(&p.manifest, &still).unwrap_err();
    assert!(matches!(err, Error::Transformed { .. }), "{err}");
}
