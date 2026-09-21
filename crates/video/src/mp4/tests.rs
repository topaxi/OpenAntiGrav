use super::*;

/// One box, built by hand: a big-endian `u32` size, the four-byte kind, then
/// whatever body bytes the caller supplies. Mirrors exactly what
/// `read_box` reads back, the same relationship `bik::tests::build` has to
/// `bik::parse`.
fn make_box(kind: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + body.len());
    out.extend_from_slice(&((8 + body.len()) as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(body);
    out
}

fn concat(boxes: &[Vec<u8>]) -> Vec<u8> {
    boxes.iter().flat_map(|b| b.iter().copied()).collect()
}

/// A `stsd` holding one `avc1` entry with the given picture size - just
/// enough of `VisualSampleEntry` for [`parse_stsd_video`] to read, the rest
/// zeroed.
fn stsd_video(width: u16, height: u16) -> Vec<u8> {
    // 78 bytes: `SampleEntry`'s 8-byte prefix plus `VisualSampleEntry`'s own
    // fixed fields, width and height landing at 24..28 - see
    // `parse_stsd_video`'s own doc comment for the full field list.
    let mut entry_body = vec![0u8; 78];
    entry_body[24..26].copy_from_slice(&width.to_be_bytes());
    entry_body[26..28].copy_from_slice(&height.to_be_bytes());
    let entry = make_box(b"avc1", &entry_body);

    let mut body = vec![0u8, 0, 0, 0]; // version/flags
    body.extend_from_slice(&1u32.to_be_bytes()); // entry_count
    body.extend_from_slice(&entry);
    make_box(b"stsd", &body)
}

/// A `stsd` holding one `mp4a` entry with the given sample rate and channel
/// count.
fn stsd_audio(sample_rate: u32, channel_count: u16) -> Vec<u8> {
    let mut entry_body = vec![0u8; 28];
    entry_body[16..18].copy_from_slice(&channel_count.to_be_bytes());
    entry_body[24..28].copy_from_slice(&(sample_rate << 16).to_be_bytes());
    let entry = make_box(b"mp4a", &entry_body);

    let mut body = vec![0u8, 0, 0, 0];
    body.extend_from_slice(&1u32.to_be_bytes());
    body.extend_from_slice(&entry);
    make_box(b"stsd", &body)
}

/// One `stts` with a single `(sample_count, sample_delta)` entry.
fn stts_one_entry(sample_count: u32, sample_delta: u32) -> Vec<u8> {
    let mut body = vec![0u8, 0, 0, 0];
    body.extend_from_slice(&1u32.to_be_bytes());
    body.extend_from_slice(&sample_count.to_be_bytes());
    body.extend_from_slice(&sample_delta.to_be_bytes());
    make_box(b"stts", &body)
}

/// An `stts` with `n` entries, each `(1, 1)` - a shape no shipped file has,
/// built only to prove [`Error::VariableFrameRate`] fires.
fn stts_many_entries(n: u32) -> Vec<u8> {
    let mut body = vec![0u8, 0, 0, 0];
    body.extend_from_slice(&n.to_be_bytes());
    for _ in 0..n {
        body.extend_from_slice(&1u32.to_be_bytes());
        body.extend_from_slice(&1u32.to_be_bytes());
    }
    make_box(b"stts", &body)
}

fn stsz(sample_count: u32) -> Vec<u8> {
    let mut body = vec![0u8, 0, 0, 0];
    body.extend_from_slice(&0u32.to_be_bytes()); // sample_size: variable
    body.extend_from_slice(&sample_count.to_be_bytes());
    make_box(b"stsz", &body)
}

fn mdhd(timescale: u32, duration: u32) -> Vec<u8> {
    let mut body = vec![0u8, 0, 0, 0]; // version 0
    body.extend_from_slice(&0u32.to_be_bytes()); // creation_time
    body.extend_from_slice(&0u32.to_be_bytes()); // modification_time
    body.extend_from_slice(&timescale.to_be_bytes());
    body.extend_from_slice(&duration.to_be_bytes());
    body.extend_from_slice(&[0u8; 4]); // language + pre_defined
    make_box(b"mdhd", &body)
}

fn hdlr(handler_type: &[u8; 4]) -> Vec<u8> {
    let mut body = vec![0u8; 8];
    body.extend_from_slice(handler_type);
    body.extend_from_slice(&[0u8; 12]);
    body.push(0); // empty name
    make_box(b"hdlr", &body)
}

/// One `trak`, built from its own `mdhd`/`hdlr`/`stsd`/`stts`/`stsz`, the
/// way `intro.mp4`'s two really nest: `trak > mdia > (mdhd, hdlr, minf >
/// stbl > (stsd, stts, stsz))`.
fn trak(
    handler_type: &[u8; 4],
    timescale: u32,
    duration: u32,
    stsd_box: Vec<u8>,
    sample_count: u32,
    sample_delta: u32,
) -> Vec<u8> {
    let stbl = make_box(
        b"stbl",
        &concat(&[
            stsd_box,
            stts_one_entry(sample_count, sample_delta),
            stsz(sample_count),
        ]),
    );
    let minf = make_box(b"minf", &stbl);
    let mdia = make_box(
        b"mdia",
        &concat(&[mdhd(timescale, duration), hdlr(handler_type), minf]),
    );
    make_box(b"trak", &mdia)
}

/// A whole file: `ftyp` `free` `mdat` `moov`, `moov` holding whichever
/// `trak`s are passed - the box order all but one of the 26 shipped files
/// use.
fn file(traks: &[Vec<u8>], mdat_len: usize) -> Vec<u8> {
    let ftyp = make_box(b"ftyp", b"mp42\x00\x00\x00\x00mp42isom");
    let free = make_box(b"free", &[]);
    let mdat = make_box(b"mdat", &vec![0xcd; mdat_len.saturating_sub(8)]);
    let mvhd_body = vec![0u8; 100];
    let moov = make_box(
        b"moov",
        &concat(&[make_box(b"mvhd", &mvhd_body), concat(traks)]),
    );
    concat(&[ftyp, free, mdat, moov])
}

/// A silent, video-only file - the shape 25 of the 26 shipped files are, and
/// the one [`Header::audio`] must read back `None` for.
fn video_only(
    width: u16,
    height: u16,
    timescale: u32,
    sample_count: u32,
    sample_delta: u32,
) -> Vec<u8> {
    let stsd_box = stsd_video(width, height);
    let duration = sample_count * sample_delta;
    let video = trak(
        b"vide",
        timescale,
        duration,
        stsd_box,
        sample_count,
        sample_delta,
    );
    file(&[video], 4096)
}

#[test]
fn a_silent_file_reads_its_own_geometry_and_rate() {
    let blob = video_only(960, 544, 30_000, 2984, 1001);

    let header = parse(&blob).expect("parsing");
    assert_eq!((header.width, header.height), (960, 544));
    assert_eq!(header.frame_count, 2984);
    assert_eq!(header.stts_sample_count, 2984);
    assert_eq!(header.timescale, 30_000);
    assert_eq!(header.duration, 2984 * 1001);
    // 30000/1001 has a GCD of 1, so this is the file's own fraction unreduced -
    // the same rational `intro.mp4` itself carries.
    assert_eq!(header.frame_rate, (30_000, 1001));
    assert!(header.audio.is_none());
    assert!(
        (header.seconds() - 99.5661).abs() < 0.001,
        "{}",
        header.seconds()
    );
}

#[test]
fn a_second_rational_close_to_29_97_is_not_rounded_into_the_first() {
    // `bb2048Zone8.mp4`'s own timescale/delta - close to but not the same
    // fraction as `intro.mp4`'s 30000/1001.
    let blob = video_only(960, 544, 2_997, 3596, 100);

    let header = parse(&blob).expect("parsing");
    assert_eq!(header.frame_rate, (2997, 100));
}

#[test]
fn a_video_and_audio_track_are_told_apart_by_handler() {
    let video = trak(
        b"vide",
        30_000,
        2984 * 1001,
        stsd_video(960, 544),
        2984,
        1001,
    );
    let audio = trak(
        b"soun",
        48_000,
        4666 * 1024,
        stsd_audio(48_000, 2),
        4666,
        1024,
    );
    let blob = file(&[video, audio], 4096);

    let header = parse(&blob).expect("parsing");
    assert_eq!((header.width, header.height), (960, 544));
    assert_eq!(header.frame_count, 2984);

    let audio = header.audio.expect("the soun track");
    assert_eq!(&audio.codec, b"mp4a");
    assert_eq!(audio.sample_rate, 48_000);
    assert_eq!(audio.channel_count, 2);
    assert_eq!(audio.frame_count, 4666);
    assert_eq!(audio.frame_delta, 1024, "the soun track's own stts delta");
}

#[test]
fn track_order_inside_moov_does_not_matter() {
    // Audio first, video second - the opposite of `intro.mp4`'s own order -
    // to prove `parse` picks tracks by handler and not by position.
    let audio = trak(b"soun", 48_000, 1024, stsd_audio(44_100, 1), 1, 1024);
    let video = trak(b"vide", 30_000, 1001, stsd_video(320, 240), 1, 1001);
    let blob = file(&[audio, video], 256);

    let header = parse(&blob).expect("parsing");
    assert_eq!((header.width, header.height), (320, 240));
    assert_eq!(header.audio.expect("audio").sample_rate, 44_100);
}

#[test]
fn moov_after_mdat_is_read_the_same_as_moov_first() {
    // Every one of the 26 shipped files puts `moov` last; this proves
    // `parse` does not assume that either way by building it the other way.
    let ftyp = make_box(b"ftyp", b"mp42\x00\x00\x00\x00mp42isom");
    let stsd_box = stsd_video(128, 96);
    let video = trak(b"vide", 30_000, 1001, stsd_box, 1, 1001);
    let moov = make_box(b"moov", &concat(&[make_box(b"mvhd", &[0u8; 100]), video]));
    let mdat = make_box(b"mdat", &[0xcd; 16]);
    let blob = concat(&[ftyp, moov, mdat]);

    let header = parse(&blob).expect("parsing moov-first");
    assert_eq!((header.width, header.height), (128, 96));
}

#[test]
fn top_level_box_sizes_sum_to_the_blob_length() {
    let blob = video_only(64, 64, 30_000, 4, 1001);

    let boxes = top_level_boxes(&blob).expect("walking the top level");
    let kinds: Vec<[u8; 4]> = boxes.iter().map(|b| b.kind).collect();
    assert_eq!(kinds, [*b"ftyp", *b"free", *b"mdat", *b"moov"]);

    let sum: u64 = boxes.iter().map(|b| b.size).sum();
    assert_eq!(sum, blob.len() as u64);
}

#[test]
fn a_box_declaring_more_than_the_blob_holds_is_refused() {
    let mut blob = video_only(64, 64, 30_000, 4, 1001);
    let len = blob.len() as u32;
    // Corrupt `ftyp`'s own size field, at offset 0, to claim eight bytes
    // more than the file actually holds.
    blob[0..4].copy_from_slice(&(len + 8).to_be_bytes());

    assert!(matches!(parse(&blob), Err(Error::BoxTruncated { at: 0 })));
    assert!(matches!(
        top_level_boxes(&blob),
        Err(Error::BoxTruncated { at: 0 })
    ));
}

#[test]
fn a_moov_with_no_video_track_is_refused() {
    let audio = trak(b"soun", 48_000, 1024, stsd_audio(48_000, 2), 1, 1024);
    let blob = file(&[audio], 64);

    assert_eq!(parse(&blob), Err(Error::NoVideoTrack));
}

#[test]
fn a_zero_dimension_is_refused() {
    let blob = video_only(0, 544, 30_000, 4, 1001);
    assert_eq!(
        parse(&blob),
        Err(Error::ZeroDimension {
            width: 0,
            height: 544
        })
    );
}

#[test]
fn a_multi_entry_stts_is_refused_rather_than_averaged() {
    let stsd_box = stsd_video(64, 64);
    let stbl = make_box(b"stbl", &concat(&[stsd_box, stts_many_entries(3), stsz(3)]));
    let minf = make_box(b"minf", &stbl);
    let mdia = make_box(b"mdia", &concat(&[mdhd(30_000, 3), hdlr(b"vide"), minf]));
    let video = make_box(b"trak", &mdia);
    let blob = file(&[video], 64);

    assert_eq!(
        parse(&blob),
        Err(Error::VariableFrameRate {
            track: *b"vide",
            entries: 3,
        })
    );
}

#[test]
fn a_file_that_does_not_start_ftyp_is_not_mp4() {
    let mut blob = video_only(64, 64, 30_000, 4, 1001);
    blob[4..8].copy_from_slice(b"moov");

    assert!(!is_mp4(&blob));
    assert_eq!(parse(&blob), Err(Error::NotMp4));
    assert_eq!(top_level_boxes(&blob), Err(Error::NotMp4));
}

#[test]
fn stsz_and_stts_disagreeing_is_still_readable_and_visibly_disagrees() {
    // `parse` trusts `stsz` for `Header::frame_count` and carries the `stts`
    // sum separately - this proves the two really are independent reads
    // rather than one computed from the other, the way the ground-truth
    // test's invariant 2 depends on.
    let stsd_box = stsd_video(64, 64);
    let stbl = make_box(
        b"stbl",
        &concat(&[stsd_box, stts_one_entry(5, 1001), stsz(7)]),
    );
    let minf = make_box(b"minf", &stbl);
    let mdia = make_box(
        b"mdia",
        &concat(&[mdhd(30_000, 5 * 1001), hdlr(b"vide"), minf]),
    );
    let video = make_box(b"trak", &mdia);
    let blob = file(&[video], 64);

    let header = parse(&blob).expect("parsing");
    assert_eq!(header.frame_count, 7);
    assert_eq!(header.stts_sample_count, 5);
    assert_ne!(header.frame_count, header.stts_sample_count);
}
