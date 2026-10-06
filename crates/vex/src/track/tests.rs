//! What the `WO Track` spline reader in [`super`] is asserted to do: the
//! control points it decodes, the graph it links them into, and the payloads it
//! refuses.

use super::*;
use oag_formats::ByteOrder;

/// Builds a payload with `paths` control-point counts, one junction per
/// path, wired as a ring.
fn build(version: u32, counts: &[usize]) -> Vec<u8> {
    build_in(version, counts, ByteOrder::Little)
}

/// The same payload, written the way the console `order` names would write it.
///
/// A PS3 `WO Track` payload is this layout with every word the other way round
/// and the magic reading `WOtd` rather than `dtOW`; see
/// `docs/formats/hd-status.md`.
fn build_in(version: u32, counts: &[usize], order: ByteOrder) -> Vec<u8> {
    let w32 = |v: u32| -> [u8; 4] {
        match order {
            ByteOrder::Little => v.to_le_bytes(),
            ByteOrder::Big => v.to_be_bytes(),
        }
    };
    let wf32 = |v: f32| -> [u8; 4] { w32(v.to_bits()) };
    let reserved = reserved_len(version);
    let paths_at = HEADER_LEN + reserved;
    let junctions_at = paths_at + counts.len() * PATH_LEN;
    let points_at = junctions_at + counts.len() * JUNCTION_LEN;
    let total: usize = counts.iter().sum();

    let stride = point_len(version);
    let mut out = vec![0u8; points_at + total * stride];
    out[0..4].copy_from_slice(&w32(MAGIC));
    out[4..8].copy_from_slice(&w32(version));
    out[8..12].copy_from_slice(&w32(counts.len() as u32));
    out[12..16].copy_from_slice(&w32(counts.len() as u32));

    for (i, &count) in counts.iter().enumerate() {
        let base = paths_at + i * PATH_LEN;
        out[base..base + 4].copy_from_slice(&w32(count as u32));
        out[base + 4..base + 8].copy_from_slice(&wf32(7.5));
        // entry is the junction before this path, exit the one after.
        let entry = (i + counts.len() - 1) % counts.len();
        out[base + 0x0c..base + 0x10].copy_from_slice(&w32(entry as u32));
        out[base + 0x10..base + 0x14].copy_from_slice(&w32(i as u32));
    }

    for j in 0..counts.len() {
        let base = junctions_at + j * JUNCTION_LEN;
        let next = (j + 1) % counts.len();
        out[base..base + 4].copy_from_slice(&w32(j as u32));
        out[base + 4..base + 8].copy_from_slice(&w32(NULL_INDEX));
        out[base + 8..base + 12].copy_from_slice(&w32(next as u32));
        out[base + 12..base + 16].copy_from_slice(&w32(NULL_INDEX));
    }

    let mut at = points_at;
    for (i, &count) in counts.iter().enumerate() {
        for k in 0..count {
            // Position walks along +x so segment lengths are checkable.
            write_vec3(&mut out, order, at, [k as f32 * 5.0, 0.0, i as f32 * 100.0]);
            write_vec3(&mut out, order, at + 0x10, [1.0, 0.0, 0.0]);
            write_vec3(&mut out, order, at + 0x20, [0.0, -1.0, 0.0]);
            write_vec3(&mut out, order, at + 0x30, [0.0, 0.0, 1.0]);
            out[at + 0x44..at + 0x48].copy_from_slice(&wf32(20.0));
            out[at + 0x48..at + 0x4c].copy_from_slice(&wf32(20.0));
            out[at + 0x4c..at + 0x50].copy_from_slice(&wf32(-9.0));
            out[at + 0x50..at + 0x54].copy_from_slice(&wf32(9.0));
            out[at + 0x54..at + 0x58].copy_from_slice(&wf32(0.5));
            let tail = if version >= SHORT_POINT_VERSION {
                0x5c
            } else {
                0x60
            };
            out[at + tail] = k as u8;
            out[at + tail + 1] = 1 << i;
            at += stride;
        }
    }
    out
}

fn write_vec3(out: &mut [u8], order: ByteOrder, at: usize, v: [f32; 3]) {
    for (k, c) in v.iter().enumerate() {
        let bytes = match order {
            ByteOrder::Little => c.to_le_bytes(),
            ByteOrder::Big => c.to_be_bytes(),
        };
        out[at + k * 4..at + k * 4 + 4].copy_from_slice(&bytes);
    }
}

#[test]
fn parses_a_two_path_ring() {
    let payload = build(0x105, &[3, 2]);
    let track = parse(&payload).expect("parse");

    assert_eq!(track.version, 0x105);
    assert_eq!(track.paths.len(), 2);
    assert_eq!(track.junctions.len(), 2);
    assert_eq!(track.paths[0].points.len(), 3);
    assert_eq!(track.paths[1].points.len(), 2);
    assert_eq!(track.point_count(), 5);
    assert_eq!(track.paths[0].entry, Some(1));
    assert_eq!(track.paths[0].exit, Some(0));
    assert_eq!(track.junctions[0].prev, [Some(0), None]);
    assert_eq!(track.junctions[0].next, [Some(1), None]);
}

/// A correct parse accounts for every byte (the synthetic version of the
/// ground-truth suite's check against all 40 shipped tracks).
#[test]
fn the_decoded_structure_accounts_for_every_byte() {
    for counts in [vec![1], vec![3, 2], vec![4, 4, 9]] {
        let payload = build(0x105, &counts);
        let track = parse(&payload).expect("parse");
        assert_eq!(track.encoded_len(), payload.len(), "counts {counts:?}");
    }
}

/// Version 0x100 has no reserved block, so every array moves 32 bytes
/// earlier. Getting this backwards is the error the module docs warn about.
#[test]
fn the_reserved_block_exists_only_from_version_0x101() {
    assert_eq!(reserved_len(0x100), 0);
    assert_eq!(reserved_len(0x101), RESERVED_LEN);
    assert_eq!(reserved_len(0x105), RESERVED_LEN);

    let old = parse(&build(0x100, &[3, 2])).expect("parse 0x100");
    let new = parse(&build(0x105, &[3, 2])).expect("parse 0x105");
    // Below `0x103` the light scales read full whatever the bytes say.
    assert_eq!(
        without_light_scale(&old.paths),
        without_light_scale(&new.paths)
    );
    assert_eq!(new.encoded_len(), old.encoded_len() + RESERVED_LEN);
}

/// Reading the paths at `+0x20` on a `0x105` file must not pass silently:
/// either an index lands outside its array or the structure stops accounting for
/// every byte (on a real track the second, hence [`AiTrack::encoded_len`]).
#[test]
fn skipping_the_reserved_block_is_detectable() {
    let payload = build(0x105, &[3, 2]);
    let mut shifted = payload.clone();
    // Claiming 0x100 moves every array back by the reserved block: the misreading.
    shifted[4..8].copy_from_slice(&0x100u32.to_le_bytes());
    match parse(&shifted) {
        Err(_) => {}
        Ok(wrong) => assert_ne!(
            wrong.encoded_len(),
            shifted.len(),
            "a shifted parse must not look self-consistent"
        ),
    }
}

#[test]
fn a_null_junction_slot_reads_as_none() {
    let track = parse(&build(0x105, &[2, 2])).expect("parse");
    assert_eq!(track.junctions[0].prev[1], None);
    assert_eq!(track.junctions[0].next[1], None);
}

#[test]
fn rejects_a_wrong_magic() {
    let mut payload = build(0x105, &[2]);
    payload[0] = 0;
    assert!(matches!(parse(&payload), Err(Error::BadMagic { .. })));
}

#[test]
fn rejects_a_version_the_game_would_reject() {
    let mut payload = build(0x105, &[2]);
    payload[4..8].copy_from_slice(&0xffu32.to_le_bytes());
    assert_eq!(
        parse(&payload),
        Err(Error::UnsupportedVersion { version: 0xff })
    );
}

#[test]
fn rejects_a_truncated_payload() {
    let payload = build(0x105, &[3, 2]);
    for len in [0, 4, HEADER_LEN, HEADER_LEN + 8, payload.len() - 1] {
        assert!(
            matches!(parse(&payload[..len]), Err(Error::OutOfBounds { .. })),
            "truncating to {len} should not parse"
        );
    }
}

/// A hostile count must not reach a `Vec` reservation. `0x7fffffff` points
/// would be 224 GiB.
#[test]
fn rejects_a_huge_point_count_before_allocating() {
    let mut payload = build(0x105, &[2]);
    let base = HEADER_LEN + RESERVED_LEN;
    payload[base..base + 4].copy_from_slice(&0x7fff_ffffu32.to_le_bytes());
    assert!(matches!(
        parse(&payload),
        Err(Error::OutOfBounds {
            what: "control points",
            ..
        })
    ));
}

#[test]
fn rejects_a_huge_path_count_before_allocating() {
    let mut payload = build(0x105, &[2]);
    payload[8..12].copy_from_slice(&0xffff_ffffu32.to_le_bytes());
    assert!(matches!(
        parse(&payload),
        Err(Error::OutOfBounds { what: "paths", .. })
    ));
}

#[test]
fn rejects_an_index_that_names_nothing() {
    let mut payload = build(0x105, &[2, 2]);
    let base = HEADER_LEN + RESERVED_LEN;
    payload[base + 0x0c..base + 0x10].copy_from_slice(&9u32.to_le_bytes());
    assert_eq!(
        parse(&payload),
        Err(Error::BadIndex {
            what: "path entry",
            index: 9,
            count: 2
        })
    );
}

#[test]
fn has_magic_is_cheap_and_does_not_panic_on_short_input() {
    assert!(has_magic(&build(0x105, &[1])));
    assert!(!has_magic(&[]));
    assert!(!has_magic(&[0x64, 0x74, 0x4f]));
}

#[test]
fn the_lift_moves_the_point_up_when_down_is_negative_y() {
    let track = parse(&build(0x105, &[1])).expect("parse");
    let p = track.paths[0].points[0];
    assert_eq!(p.down, [0.0, -1.0, 0.0]);
    assert_eq!(p.lifted_pos(), [p.pos[0], p.pos[1] + HOVER_LIFT, p.pos[2]]);
}

#[test]
fn the_basis_is_a_partition_of_unity() {
    for step in 0..=10 {
        let t = step as f32 / 10.0;
        let w = basis(t);
        let sum: f32 = w.iter().sum();
        assert!((sum - 1.0).abs() < 1e-6, "t={t} sum={sum}");
        assert!(w.iter().all(|&x| x >= 0.0), "t={t} has a negative weight");
    }
}

/// A B-spline is C2 continuous, so a segment's end is the next one's start; a
/// wrong basis or control-point window shows here.
#[test]
fn segments_join_up() {
    let track = parse(&build(0x105, &[6])).expect("parse");
    let path = &track.paths[0];
    for segment in 0..4 {
        let a = path.sample(segment, 1.0).expect("sample");
        let b = path.sample(segment + 1, 0.0).expect("sample");
        for k in 0..3 {
            assert!(
                (a.pos[k] - b.pos[k]).abs() < 1e-4,
                "segment {segment} axis {k}: {} vs {}",
                a.pos[k],
                b.pos[k]
            );
        }
        assert!((a.half_width_left - b.half_width_left).abs() < 1e-4);
    }
}

/// On a straight, evenly spaced path the curve lies on the same line as the
/// control points, so the sample can be checked against the exact value.
#[test]
fn sampling_a_straight_path_lands_on_the_line() {
    let track = parse(&build(0x105, &[5])).expect("parse");
    let path = &track.paths[0];
    // Control points at x = 0, 5, 10, 15, 20: with equal spacing, t=0 in segment
    // 2 is the 1/6, 4/6, 1/6 average of its neighbours, the control point itself.
    let s = path.sample(2, 0.0).expect("sample");
    assert!((s.pos[0] - 10.0).abs() < 1e-4, "got {}", s.pos[0]);
    assert!((s.pos[1]).abs() < 1e-6);
    // Halfway between control points 2 and 3.
    let mid = path.sample(2, 0.5).expect("sample");
    assert!((mid.pos[0] - 12.5).abs() < 1e-4, "got {}", mid.pos[0]);
}

#[test]
fn sample_refuses_a_segment_past_the_end() {
    let track = parse(&build(0x105, &[2])).expect("parse");
    assert!(track.paths[0].sample(2, 0.0).is_none());
}

#[test]
fn sample_or_accumulates_flags_across_the_window() {
    // Two paths, each with a distinct flag bit; within one path every point
    // carries the same bit, so the OR is that bit.
    let track = parse(&build(0x105, &[4, 4])).expect("parse");
    assert_eq!(track.paths[0].sample(1, 0.3).expect("sample").flags, 0b01);
    assert_eq!(track.paths[1].sample(1, 0.3).expect("sample").flags, 0b10);
}

/// Builds a `Start Position` payload from four rows.
/// [`start_position`] on a fixture, which is written little-endian.
fn start_position_le(payload: &[u8]) -> Option<StartPosition> {
    start_position(payload, ByteOrder::Little)
}

fn slot(rows: [[f32; 3]; 4]) -> Vec<u8> {
    let mut out = vec![0u8; START_POSITION_LEN];
    for (r, row) in rows.iter().enumerate() {
        for (c, value) in row.iter().enumerate() {
            let at = r * 16 + c * 4;
            out[at..at + 4].copy_from_slice(&value.to_le_bytes());
        }
        out[r * 16 + 12..r * 16 + 16].copy_from_slice(&f32::to_le_bytes(if r == 3 {
            1.0
        } else {
            0.0
        }));
    }
    out
}

#[test]
fn a_start_position_reads_its_rows_as_left_up_forward() {
    // Every shipped slot satisfies cross(left, up) = forward (`16_Track` faces
    // `+x` with `-z` to its left).
    let position = start_position_le(&slot([
        [0.0, 0.0, -1.0],
        [0.0, 1.0, 0.0],
        [1.0, 0.0, 0.0],
        [8.0, -50.0, -196.0],
    ]))
    .expect("a non-degenerate frame");

    assert_eq!(position.position, [8.0, -50.0, -196.0]);
    assert_eq!(position.forward, [1.0, 0.0, 0.0]);
    assert_eq!(position.up, [0.0, 1.0, 0.0]);
    assert_eq!(position.left, [0.0, 0.0, -1.0]);
}

/// The bind forces up rather than reading it, so a tilted authored frame comes
/// back level, forward perpendicular to the up it was given.
#[test]
fn the_bind_levels_a_tilted_slot() {
    let tilt = 0.25f32;
    let position = start_position_le(&slot([
        [0.0, 0.0, -1.0],
        [-tilt, 1.0, 0.0],
        [1.0, tilt, 0.0],
        [0.0, 0.0, 0.0],
    ]))
    .expect("a non-degenerate frame");

    assert_eq!(position.up, [0.0, 1.0, 0.0]);
    assert!(position.forward[1].abs() < 1e-6, "{:?}", position.forward);
    assert!(
        (position.forward[0] - 1.0).abs() < 1e-6,
        "{:?}",
        position.forward
    );
    // Still right-handed after the fix-up.
    let expected = cross(position.left, position.up);
    for (want, got) in expected.iter().zip(position.forward) {
        assert!((want - got).abs() < 1e-6);
    }
}

#[test]
fn a_vertical_forward_axis_has_no_slot_to_report() {
    assert!(
        start_position_le(&slot([
            [1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0],
        ]))
        .is_none()
    );
}

#[test]
fn a_short_start_position_payload_is_refused() {
    assert!(start_position_le(&[0u8; 63]).is_none());
}

/// The same spline, written both ways round, parses to the same thing.
///
/// The payload declares its byte order in its own magic (`dtOW` PSP/PS2, `WOtd`
/// PS3), so [`parse`] needs no argument; pinned because
/// `docs/formats/hd-status.md` reads as though it does not.
#[test]
fn a_big_endian_payload_parses_to_the_same_spline_as_its_little_endian_twin() {
    let le = build(0x105, &[3, 4]);
    let be = build_in(0x105, &[3, 4], ByteOrder::Big);

    assert_eq!(&le[..4], b"dtOW", "the PSP and PS2 spelling");
    assert_eq!(&be[..4], b"WOtd", "and the PS3's, which is the same word");
    assert_eq!(byte_order(&le), Some(ByteOrder::Little));
    assert_eq!(byte_order(&be), Some(ByteOrder::Big));
    assert!(has_magic(&le) && has_magic(&be));

    let from_le = parse(&le).expect("the little-endian payload");
    let from_be = parse(&be).expect("the big-endian payload");
    assert_eq!(from_le.version, from_be.version);
    assert_eq!(from_le.junctions, from_be.junctions);
    assert_eq!(from_le.paths.len(), from_be.paths.len());
    for (a, b) in from_le.paths.iter().zip(&from_be.paths) {
        assert_eq!(a.points, b.points);
        assert_eq!(a.max_spacing, b.max_spacing);
        assert_eq!((a.entry, a.exit), (b.entry, b.exit));
    }
    assert_eq!(
        from_be.encoded_len(),
        be.len(),
        "and it accounts for every byte"
    );
}

/// Neither spelling is a wrong magic that happens to parse.
#[test]
fn a_payload_whose_magic_is_neither_spelling_is_still_refused() {
    let mut payload = build(0x105, &[2]);
    payload[0..4].copy_from_slice(b"OhNo");
    assert_eq!(byte_order(&payload), None);
    assert!(!has_magic(&payload));
    assert!(matches!(parse(&payload), Err(Error::BadMagic { .. })));
}

/// 2048's version `0x107` shortens the control point to 96 bytes and moves
/// `section_id`/`flags`; everything up to `racing_line` stays put. Offsets are
/// measured: see `docs/formats/track.md`.
#[test]
fn version_0x107_shortens_the_control_point() {
    assert_eq!(point_len(0x106), POINT_LEN);
    assert_eq!(point_len(SHORT_POINT_VERSION), POINT_LEN_SHORT);

    let payload = build(SHORT_POINT_VERSION, &[4, 3]);
    let track = parse(&payload).expect("a 0x107 payload parses");
    assert_eq!(track.version, SHORT_POINT_VERSION);
    assert_eq!(track.encoded_len(), payload.len());
    assert_eq!(track.paths[0].points.len(), 4);
    assert_eq!(track.paths[1].points.len(), 3);
}

/// The same authored values decode identically either side of the version
/// boundary.
#[test]
fn the_short_record_decodes_the_same_values_as_the_long_one() {
    let long = parse(&build(0x106, &[4, 3])).expect("0x106 parses");
    let short = parse(&build(SHORT_POINT_VERSION, &[4, 3])).expect("0x107 parses");
    // The short record's light scales are not read, and read full.
    assert_eq!(
        without_light_scale(&long.paths),
        without_light_scale(&short.paths)
    );
    assert!(short.encoded_len() < long.encoded_len());
}

/// A 0x107 payload read at the old 112-byte stride runs off the end (what once
/// blocked a 2048 track from loading).
#[test]
fn the_old_stride_does_not_fit_a_0x107_payload() {
    let payload = build(SHORT_POINT_VERSION, &[4, 3]);
    let fixed = HEADER_LEN + RESERVED_LEN + 2 * PATH_LEN + 2 * JUNCTION_LEN;
    assert_eq!(payload.len(), fixed + 7 * POINT_LEN_SHORT);
    assert!(fixed + 7 * POINT_LEN > payload.len());
}

#[test]
fn a_run_of_full_light_blends_to_just_under_full_the_way_the_original_truncates() {
    let track = parse(&build(0x105, &[1])).expect("parse");
    let mut point = track.paths[0].points[0];
    point.light_scale = [255, 127, 0, 255];
    let four = [&point; 4];
    // At t = 0 the weights are 1/6, 2/3, 1/6, 0: trunc(x255) is 42, 170, 42, 0,
    // each product shifted right by eight before the sum.
    assert_eq!(blend_light_scale(&basis(0.0), &four), [251, 124, 0, 251]);
}

#[test]
fn a_track_older_than_0x103_reads_full_light_whatever_its_bytes_say() {
    let track = parse(&build(0x102, &[1])).expect("parse");
    assert_eq!(track.paths[0].points[0].light_scale, [0xff; 4]);
}

/// Every point with its light scales cleared, for comparing two layouts that
/// carry them differently.
fn without_light_scale(paths: &[Path]) -> Vec<Vec<SplinePoint>> {
    paths
        .iter()
        .map(|path| {
            path.points
                .iter()
                .map(|p| SplinePoint {
                    light_scale: [0; 4],
                    ..*p
                })
                .collect()
        })
        .collect()
}
