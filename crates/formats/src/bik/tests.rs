use super::*;

/// Builds a Bink file the way the shipped ones are laid out: the 44-byte fixed
/// header, three parallel per-track arrays, the frame offset table, and then
/// one blob of bytes per frame.
///
/// Authored rather than extracted, per
/// `docs/architecture/adr/0006-no-copyrighted-content.md`. What it reproduces is
/// the *layout* the survey measured, which is the only thing this module reads.
fn build(
    width: u32,
    height: u32,
    rate: (u32, u32),
    tracks: &[(u32, u16)],
    frames: &[usize],
) -> Vec<u8> {
    let header_len = FIXED_HEADER_LEN + 12 * tracks.len() + 4 * (frames.len() + 1);
    let total = header_len + frames.iter().sum::<usize>();

    let mut out = Vec::new();
    out.extend_from_slice(&MAGIC);
    out.push(SHIPPED_REVISION);
    out.extend_from_slice(&((total - 8) as u32).to_le_bytes());
    out.extend_from_slice(&(frames.len() as u32).to_le_bytes());
    out.extend_from_slice(&(frames.iter().copied().max().unwrap_or(0) as u32).to_le_bytes());
    out.extend_from_slice(&(frames.len() as u32).to_le_bytes());
    out.extend_from_slice(&width.to_le_bytes());
    out.extend_from_slice(&height.to_le_bytes());
    out.extend_from_slice(&rate.0.to_le_bytes());
    out.extend_from_slice(&rate.1.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(tracks.len() as u32).to_le_bytes());
    assert_eq!(out.len(), FIXED_HEADER_LEN);

    for _ in tracks {
        out.extend_from_slice(&0x0002_3a00u32.to_le_bytes());
    }
    for (sample_rate, flags) in tracks {
        out.extend_from_slice(&(u32::from(*flags) << 16 | sample_rate).to_le_bytes());
    }
    for (id, _) in (0..tracks.len()).map(|i| (i as u32, ())) {
        out.extend_from_slice(&id.to_le_bytes());
    }

    // Every frame offset, then the end of the last one, exactly as the file
    // writes it - and the first carries the keyframe bit, which is why the
    // parser masks before comparing.
    let mut at = header_len;
    for (index, len) in frames.iter().enumerate() {
        out.extend_from_slice(&((at as u32) | u32::from(index == 0)).to_le_bytes());
        at += len;
    }
    out.extend_from_slice(&(at as u32).to_le_bytes());
    assert_eq!(out.len(), header_len);

    out.resize(total, 0xcd);
    out
}

/// The two logo reels' shape: 1080p, four stereo DCT tracks at 48 kHz.
fn logo_reel() -> Vec<u8> {
    let tracks = [(48_000, AUD_16BITS | AUD_STEREO | AUD_USEDCT); 4];
    build(1920, 1080, (10_000_000, 166_833), &tracks, &[900, 400, 400])
}

#[test]
fn a_silent_file_reads_its_own_geometry() {
    let blob = build(352, 256, (10_000_000, 166_833), &[], &[120, 90, 90]);

    let header = parse(&blob).expect("parsing");
    assert_eq!(header.revision, SHIPPED_REVISION);
    assert_eq!((header.width, header.height), (352, 256));
    assert_eq!(header.frame_count, 3);
    assert_eq!(header.largest_frame, 120);
    assert_eq!(header.frame_rate, (10_000_000, 166_833));
    assert_eq!(header.video_flags, 0);
    assert!(header.audio.is_empty());
    // 31 of the disc's 37 files are this shape, so an empty track list is the
    // ordinary case rather than a failure to find one.
    assert_eq!(header.header_len, FIXED_HEADER_LEN + 4 * 4);
}

#[test]
fn the_declared_length_is_the_field_plus_eight() {
    let blob = logo_reel();

    let header = parse(&blob).expect("parsing");
    assert!(header.declares_length_of(blob.len()));
    assert_eq!(header.declared_len, blob.len() as u64);
    // The first of the three invariants the survey checked on all 37 files.
    assert!(!header.declares_length_of(blob.len() - 1));
}

#[test]
fn the_first_frame_offset_lands_on_the_end_of_the_header() {
    let blob = logo_reel();

    let header = parse(&blob).expect("parsing");
    // The load-bearing invariant: it can only agree when the three per-track
    // arrays were sized and ordered right, so it validates the variable-length
    // middle of the header rather than its ends.
    assert_eq!(first_frame_offset(&blob, &header), Some(header.header_len));
    assert_eq!(header.header_len, FIXED_HEADER_LEN + 12 * 4 + 4 * 4);
}

#[test]
fn four_stereo_dct_tracks_read_back_in_the_files_own_order() {
    let blob = logo_reel();

    let header = parse(&blob).expect("parsing");
    assert_eq!(header.audio.len(), 4);
    for (index, track) in header.audio.iter().enumerate() {
        assert_eq!(track.id, index as u32);
        assert_eq!(track.sample_rate, 48_000);
        assert_eq!(track.channels(), 2);
        assert!(track.is_dct());
        assert_eq!(track.max_decoded_len, 0x0002_3a00);
    }
}

#[test]
fn a_mono_rdft_track_is_told_apart_from_a_stereo_dct_one() {
    let blob = build(320, 240, (60, 1), &[(22_050, AUD_16BITS)], &[64]);

    let header = parse(&blob).expect("parsing");
    let track = header.audio[0];
    assert_eq!(track.sample_rate, 22_050);
    assert_eq!(track.channels(), 1);
    assert!(!track.is_dct());
}

#[test]
fn the_frame_rate_stays_the_files_own_fraction() {
    // 531 frames of 10000000/166833 is 8.8588 s, which is what `ffprobe`
    // reports for the logo reel; the same count against a rounded 60 Hz would
    // be 8.8500, and the reel would end 9 ms early.
    let mut frames = vec![64; 531];
    frames[0] = 900;
    let blob = build(1920, 1080, (10_000_000, 166_833), &[], &frames);

    let header = parse(&blob).expect("parsing");
    assert_eq!(header.frame_rate, (10_000_000, 166_833));
    assert!(
        (header.seconds() - 8.8588).abs() < 0.0001,
        "{}",
        header.seconds()
    );
}

#[test]
fn bink_two_is_not_read_as_bink_one() {
    let mut blob = logo_reel();
    blob[..4].copy_from_slice(b"KB2g");

    assert!(!is_bink(&blob));
    assert_eq!(parse(&blob), Err(Error::NotBink));
}

#[test]
fn a_revision_this_project_has_not_seen_is_still_read() {
    let mut blob = logo_reel();
    blob[3] = b'b';

    let header = parse(&blob).expect("parsing");
    assert_eq!(header.revision, b'b');
    assert_eq!(header.frame_count, 3);
}

#[test]
fn a_truncated_file_names_the_header_it_could_not_fit() {
    let blob = logo_reel();
    let header = parse(&blob).expect("parsing");

    let short = &blob[..header.header_len - 1];
    assert_eq!(
        parse(short),
        Err(Error::HeaderTruncated {
            want: header.header_len,
            got: short.len(),
        })
    );
    assert_eq!(
        parse(&blob[..FIXED_HEADER_LEN - 1]),
        Err(Error::TooShort {
            got: FIXED_HEADER_LEN - 1,
        })
    );
}

#[test]
fn an_absurd_track_count_is_refused_rather_than_indexed() {
    // The header's variable middle is attacker-controlled in the sense that
    // matters here: a misidentified blob reaching `parse` must not index past
    // its own end.
    let mut blob = logo_reel();
    blob[0x28..0x2c].copy_from_slice(&u32::MAX.to_le_bytes());

    assert!(matches!(parse(&blob), Err(Error::HeaderTruncated { .. })));
}

#[test]
fn a_picture_or_a_rate_nothing_can_use_is_refused() {
    let mut blob = logo_reel();
    blob[0x18..0x1c].copy_from_slice(&0u32.to_le_bytes());
    assert_eq!(
        parse(&blob),
        Err(Error::ZeroDimension {
            width: 1920,
            height: 0,
        })
    );

    let mut blob = logo_reel();
    blob[0x20..0x24].copy_from_slice(&0u32.to_le_bytes());
    assert_eq!(parse(&blob), Err(Error::ZeroFrameRate));
}
