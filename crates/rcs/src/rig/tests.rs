//! What [`super`] is asserted to do: the evaluation rules, on hand-built
//! nodes and tracks.

use super::*;
use crate::rcsanimclip::{
    self,
    tests::{Authored, Keys, clip},
};
use crate::rcsskeleton::{
    self,
    tests::{Bind, skeleton},
};

fn motion(bind: Bind, track: Option<Authored>) -> NodeMotion {
    let id = bind.id;
    let node = rcsskeleton::parse(&skeleton(&[bind]))
        .expect("skeleton")
        .nodes
        .remove(0);
    let track = track.map(|t| {
        let duration = t.duration;
        rcsanimclip::parse(&clip(&[id], &[t], duration))
            .expect("clip")
            .tracks
            .remove(0)
    });
    NodeMotion { node, track }
}

fn translation(m: &[f32; 16]) -> [f32; 3] {
    [m[12], m[13], m[14]]
}

#[test]
fn a_node_with_no_track_sits_at_bind() {
    let m = motion(Bind::plain(1, None, [1.0, 2.0, 3.0]), None);
    assert!(!m.is_animated());
    assert_eq!(translation(&m.sample(0.0)), [1.0, 2.0, 3.0]);
    assert_eq!(translation(&m.sample(99.0)), [1.0, 2.0, 3.0]);
}

#[test]
fn keys_blend_linearly_and_wrap_on_the_track_duration() {
    let m = motion(
        Bind::plain(1, None, [0.0; 3]),
        Some(Authored {
            duration: 0.6,
            channels: vec![Keys {
                slot: 2,
                kind: Kind::Vec3,
                seconds_per_key: 0.2,
                keys: vec![0.0, 0.0, 0.0, 10.0, 0.0, 0.0, 20.0, 0.0, 0.0],
            }],
        }),
    );
    assert!(m.is_animated());
    assert!((translation(&m.sample(0.0))[0]).abs() < 1e-5);
    assert!((translation(&m.sample(0.1))[0] - 5.0).abs() < 1e-4);
    assert!((translation(&m.sample(0.2))[0] - 10.0).abs() < 1e-4);
    // Past the last key the segment heads back to key 0, closing the loop.
    assert!((translation(&m.sample(0.5))[0] - 10.0).abs() < 1e-4);
    // And time wraps on the track's own length.
    assert!((translation(&m.sample(0.7))[0] - 5.0).abs() < 1e-4);
    assert!((translation(&m.sample(-0.1))[0] - 10.0).abs() < 1e-4);
}

#[test]
fn a_rotation_blends_along_the_shorter_arc() {
    // Key 0 is the identity, key 1 is the identity's negation - the same
    // rotation - so a blend that took the long way would pass through a
    // zero-length quaternion halfway; the shorter arc keeps it the identity.
    let m = motion(
        Bind::plain(1, None, [0.0; 3]),
        Some(Authored {
            duration: 0.4,
            channels: vec![Keys {
                slot: 1,
                kind: Kind::Quat,
                seconds_per_key: 0.2,
                keys: vec![0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, -1.0],
            }],
        }),
    );
    let halfway = m.sample(0.1);
    assert!(
        halfway
            .iter()
            .zip(&rcsskeleton::IDENTITY)
            .all(|(a, b)| (a - b).abs() < 1e-5),
        "{halfway:?}"
    );
}

#[test]
fn visibility_holds_its_key_and_hides_as_a_zero_matrix() {
    let m = motion(
        Bind::plain(1, None, [1.0, 0.0, 0.0]),
        Some(Authored {
            duration: 0.6,
            channels: vec![Keys {
                slot: 3,
                kind: Kind::Bool,
                seconds_per_key: 0.2,
                keys: vec![1.0, 0.0, 1.0],
            }],
        }),
    );
    assert!(m.visible_at(0.0));
    assert!(!m.visible_at(0.2));
    assert!(!m.visible_at(0.39), "held, not blended");
    assert!(m.visible_at(0.4));
    assert_eq!(m.sample(0.3), [0.0; 16]);
    assert_eq!(translation(&m.sample(0.0)), [1.0, 0.0, 0.0]);
}

#[test]
fn a_node_authored_invisible_with_no_keys_stays_hidden() {
    let mut bind = Bind::plain(1, None, [0.0; 3]);
    bind.visible = false;
    let m = motion(bind, None);
    assert!(!m.visible_at(0.0));
    assert_eq!(m.sample(5.0), [0.0; 16]);
}
