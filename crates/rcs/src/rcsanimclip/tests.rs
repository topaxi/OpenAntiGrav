//! What [`super`] is asserted to do, on a clip built by hand.

use super::*;
use crate::rcsmodel::psp2::container::wrap_section;
use crate::rcsskeleton::tests::Section;

/// One channel to author: its slot, kind, seconds per key, and keys.
pub(crate) struct Keys {
    pub(crate) slot: usize,
    pub(crate) kind: Kind,
    pub(crate) seconds_per_key: f32,
    pub(crate) keys: Vec<f32>,
}

/// One track to author: its loop and its channels; the node id is the
/// bound id at the same position, as the file binds them.
pub(crate) struct Authored {
    pub(crate) duration: f32,
    pub(crate) channels: Vec<Keys>,
}

/// A `.rcsanimclip` image binding `bound` ids, with a track per entry of
/// `tracks` on the first ids in order.
pub(crate) fn clip(bound: &[u32], tracks: &[Authored], duration: f32) -> Vec<u8> {
    let mut s = Section(Vec::new());
    s.u32(2);
    s.u32(bound.len() as u32);
    s.u32(tracks.len() as u32);
    s.u32(0);
    let p_ids = s.u32(0);
    let p_tracks = s.u32(0);
    s.f32s(&[duration]);
    let ids_at = s.0.len();
    for &id in bound {
        s.u32(id);
    }
    let tracks_at = s.0.len();
    let mut table_sites = Vec::new();
    for t in tracks {
        s.u32(0);
        s.u32(SLOTS as u32);
        table_sites.push(s.u32(0));
        s.u32(0);
        s.f32s(&[t.duration]);
    }
    for (i, t) in tracks.iter().enumerate() {
        let table_at = s.0.len();
        let mut slot_sites = Vec::new();
        for _ in 0..SLOTS {
            slot_sites.push(s.u32(0));
        }
        s.patch(table_sites[i], table_at as u32);
        for k in &t.channels {
            let at = s.0.len();
            let type_byte: u32 = match k.kind {
                Kind::Scalar => 0,
                Kind::Vec3 => 1,
                Kind::Quat => 3,
                Kind::Bool => 4,
            };
            let width = match k.kind {
                Kind::Scalar | Kind::Bool => 1,
                Kind::Vec3 => 3,
                Kind::Quat => 4,
            };
            let count = k.keys.len() / width;
            s.u32(0x10000 | k.slot as u32);
            s.f32s(&[t.duration]);
            s.u32(count as u32);
            s.u32((type_byte << 16) | 1);
            let keys_site = s.u32(0);
            s.f32s(&[1.0 / k.seconds_per_key, k.seconds_per_key]);
            let keys_at = s.0.len();
            match k.kind {
                Kind::Bool => {
                    for v in &k.keys {
                        s.0.push(u8::from(*v != 0.0));
                    }
                    while !s.0.len().is_multiple_of(4) {
                        s.0.push(0);
                    }
                }
                _ => {
                    s.f32s(&k.keys);
                }
            }
            s.patch(keys_site, keys_at as u32);
            s.patch(slot_sites[k.slot], at as u32);
        }
    }
    s.patch(p_ids, ids_at as u32);
    s.patch(p_tracks, tracks_at as u32);
    wrap_section(&s.0, &[])
}

#[test]
fn reads_tracks_channels_and_keys_of_every_kind() {
    let file = clip(
        &[7, 8, 9],
        &[Authored {
            duration: 0.6,
            channels: vec![
                Keys {
                    slot: 1,
                    kind: Kind::Quat,
                    seconds_per_key: 0.2,
                    keys: vec![0.0, 0.0, 0.0, 1.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0],
                },
                Keys {
                    slot: 2,
                    kind: Kind::Vec3,
                    seconds_per_key: 0.2,
                    keys: vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0],
                },
                Keys {
                    slot: 3,
                    kind: Kind::Bool,
                    seconds_per_key: 0.2,
                    keys: vec![1.0, 0.0, 1.0],
                },
            ],
        }],
        0.6,
    );
    let parsed = parse(&file).expect("parses");
    assert_eq!(parsed.bound, vec![7, 8, 9]);
    assert_eq!(parsed.tracks.len(), 1);
    assert!((parsed.duration - 0.6).abs() < 1e-6);
    let track = parsed.track(7).expect("track for 7");
    assert!(parsed.track(8).is_none(), "bound without keys");
    let rotation = track.channels[1].as_ref().expect("rotation");
    assert_eq!(rotation.kind, Kind::Quat);
    assert_eq!(rotation.count, 3);
    assert_eq!(rotation.key(1), &[0.0, 1.0, 0.0, 0.0]);
    assert!((rotation.seconds_per_key - 0.2).abs() < 1e-6);
    let translation = track.channels[2].as_ref().expect("translation");
    assert_eq!(translation.key(2), &[7.0, 8.0, 9.0]);
    let visibility = track.channels[3].as_ref().expect("visibility");
    assert_eq!(visibility.kind, Kind::Bool);
    assert_eq!(visibility.keys, vec![1.0, 0.0, 1.0]);
    assert!(track.channels[0].is_none());
}

#[test]
fn refuses_more_tracks_than_bound_ids() {
    let mut file = clip(&[1], &[], 1.0);
    // The track count word sits at section +0x08.
    let section_at = u32::from_le_bytes(file[12..16].try_into().unwrap()) as usize;
    file[section_at + 8..section_at + 12].copy_from_slice(&5u32.to_le_bytes());
    assert!(parse(&file).is_err());
}
