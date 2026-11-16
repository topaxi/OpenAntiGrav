//! The `.shprobes` reader on a synthetic file: the layout closes, a refusal names its cause, and
//! a position blends one lane of the triples either side of it.

use super::*;

/// `triples` triples 1.5 apart along x, their three records 18 apart along z; record `r` carries
/// coefficient `r` in every slot so a blend is readable off any one of them.
fn file(triples: usize) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(b"BPHS");
    out.extend_from_slice(&((3 * triples) as u32).to_le_bytes());
    for t in 0..triples {
        for lane in 0..3 {
            let r = (3 * t + lane) as f32;
            for c in [1.5 * t as f32, 0.0, 18.0 * lane as f32] {
                out.extend_from_slice(&c.to_le_bytes());
            }
            for _ in 0..27 {
                out.extend_from_slice(&r.to_le_bytes());
            }
        }
    }
    out
}

#[test]
fn the_layout_closes_and_refuses_by_name() {
    let probes = ShProbes::parse(&file(4)).expect("closes");
    assert_eq!(probes.len(), 12);
    assert_eq!(probes.position(7), [3.0, 0.0, 18.0]);
    assert_eq!(probes.coefficients(7), [7.0; 27]);
    assert_eq!(ShProbes::parse(&[0; 4]).unwrap_err(), Error::Truncated);
    let mut bad = file(4);
    bad[0] = b'X';
    assert!(matches!(ShProbes::parse(&bad), Err(Error::Magic(_))));
    let mut odd = file(4);
    odd[4] = 11;
    assert_eq!(ShProbes::parse(&odd).unwrap_err(), Error::Count(11));
    let short = &file(4)[..100];
    assert!(matches!(ShProbes::parse(short), Err(Error::Length { .. })));
}

#[test]
fn a_position_blends_the_nearest_lane_of_the_triples_either_side() {
    let probes = ShProbes::parse(&file(5)).expect("parses");
    let taps = probes.locate([3.3, 5.0, 17.0]);
    assert_eq!((taps.from, taps.to), (7, 10));
    assert!((taps.weight - 0.2).abs() < 1e-6, "{taps:?}");
    let blend = probes.blend(taps);
    assert!(blend.iter().all(|c| (c - 7.6).abs() < 1e-5), "{blend:?}");
    let before = probes.locate([2.9, 0.0, 0.0]);
    assert_eq!((before.from, before.to), (3, 6));
    assert!((before.weight - 0.9333333).abs() < 1e-5, "{before:?}");
}

#[test]
fn the_ends_of_the_track_clamp() {
    let probes = ShProbes::parse(&file(3)).expect("parses");
    let start = probes.locate([-50.0, 0.0, 36.0]);
    assert_eq!((start.from, start.to, start.weight), (2, 5, 0.0));
    let end = probes.locate([50.0, 0.0, 0.0]);
    assert_eq!((end.from, end.to, end.weight), (3, 6, 1.0));
}
