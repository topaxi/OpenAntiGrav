//! What the ATRAC3+ wrapper in [`super`] is asserted to do: the RIFF header
//! field by field against the bytes the disc stores, the config word for every
//! block size, the length `fact` declares, and the decode cache's key and its
//! rejections.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of
//! `at3.rs`: the tests are 278 lines, past the 200 an inline test
//! module may hold. See `scripts/check-file-size.py`, which is the rule as a
//! gate.

use super::*;

/// The layout in [`riff`]'s own doc comment, asserted field by field
/// against the bytes `SND0.AT3` stores. This is the evidence for the
/// wrapper: get one field wrong and `ffmpeg` decodes noise rather than
/// refusing, so it is pinned rather than left to the first movie that
/// tries it.
#[test]
fn the_wrapper_is_the_header_snd0_at3_carries() {
    let format = Format {
        channels: 2,
        sample_rate: 44_100,
        block_align: 560,
    };
    let wrapped = riff(&[0xab; 1120], format);

    assert_eq!(&wrapped[0..4], b"RIFF");
    assert_eq!(
        u32::from_le_bytes(wrapped[4..8].try_into().unwrap()),
        (HEADER_LEN - 8 + 1120) as u32
    );
    assert_eq!(&wrapped[8..12], b"WAVE");
    assert_eq!(&wrapped[12..16], b"fmt ");
    assert_eq!(u32::from_le_bytes(wrapped[16..20].try_into().unwrap()), 52);
    assert_eq!(
        u16::from_le_bytes(wrapped[20..22].try_into().unwrap()),
        0xfffe,
        "wFormatTag is WAVE_FORMAT_EXTENSIBLE"
    );
    assert_eq!(u16::from_le_bytes(wrapped[22..24].try_into().unwrap()), 2);
    assert_eq!(
        u32::from_le_bytes(wrapped[24..28].try_into().unwrap()),
        44_100
    );
    assert_eq!(
        u32::from_le_bytes(wrapped[28..32].try_into().unwrap()),
        12_058,
        "avg bytes/sec is 560 bytes per 2048 samples at 44.1 kHz, the value \
         SND0.AT3 stores"
    );
    assert_eq!(
        u16::from_le_bytes(wrapped[32..34].try_into().unwrap()),
        560,
        "blockAlign comes from the stream, never a constant"
    );
    assert_eq!(
        u16::from_le_bytes(wrapped[34..36].try_into().unwrap()),
        0,
        "wBitsPerSample is zero for this codec"
    );
    assert_eq!(u16::from_le_bytes(wrapped[36..38].try_into().unwrap()), 34);
    assert_eq!(
        u16::from_le_bytes(wrapped[38..40].try_into().unwrap()),
        2048,
        "wValidBitsPerSample is samples per block, not a depth"
    );
    assert_eq!(u32::from_le_bytes(wrapped[40..44].try_into().unwrap()), 3);
    assert_eq!(&wrapped[44..60], &ATRAC3PLUS_GUID);
    assert_eq!(
        &wrapped[60..72],
        &[0x01, 0x00, 0x28, 0x45, 0, 0, 0, 0, 0, 0, 0, 0],
        "the twelve bytes SND0.AT3 itself stores"
    );
    assert_eq!(&wrapped[72..76], b"data");
    assert_eq!(
        u32::from_le_bytes(wrapped[76..80].try_into().unwrap()),
        1120
    );
    assert_eq!(wrapped.len(), HEADER_LEN + 1120);
    assert_eq!(HEADER_LEN, 0x50, "the frames start at 0x50");
}

/// A mono clip at the other block size the disc uses. `blockAlign` being
/// per file is the one thing about this header that is easy to get wrong
/// and impossible to notice by listening to the file it was copied from.
#[test]
fn a_mono_clip_carries_its_own_block_size() {
    let wrapped = riff(
        &[0; 280],
        Format {
            channels: 1,
            sample_rate: 44_100,
            block_align: 280,
        },
    );
    assert_eq!(u16::from_le_bytes(wrapped[22..24].try_into().unwrap()), 1);
    assert_eq!(u16::from_le_bytes(wrapped[32..34].try_into().unwrap()), 280);
    assert_eq!(
        u32::from_le_bytes(wrapped[28..32].try_into().unwrap()),
        6_029
    );
    assert_eq!(
        &wrapped[60..72],
        &[0x01, 0x00, 0x24, 0x22, 0, 0, 0, 0, 0, 0, 0, 0],
        "the config word the disc's own 280-byte mono clips carry"
    );
}

/// Every `(block_align, channels)` combination the disc actually stores,
/// against the word its own `fmt ` chunk carries. These are read values,
/// not derived ones: the formula in [`codec_config`] was fitted to them and
/// this is what stops it being refactored into something that merely
/// reproduces the stereo 560 case it was originally copied from.
#[test]
fn the_config_word_is_what_the_discs_own_entries_carry() {
    for (block_align, channels, expected) in [
        (280u16, 1u16, 0x2422u16),
        (560, 1, 0x2445),
        (560, 2, 0x2845),
        // Not a `Data.wad` entry but `Intro.PMF`'s frame header, which
        // carries this same word - see `crate::movie`.
        (744, 2, 0x285c),
    ] {
        let format = Format {
            channels,
            sample_rate: 44_100,
            block_align,
        };
        assert_eq!(
            codec_config(format),
            expected,
            "block_align {block_align}, {channels} channel(s)"
        );
    }
}

/// What [`riff`] writes is what [`read_format`] reads. The two are each
/// other's inverse and nothing else checks that they agree.
#[test]
fn the_wrapper_round_trips_through_the_reader() {
    for format in [
        Format {
            channels: 2,
            sample_rate: 44_100,
            block_align: 560,
        },
        Format {
            channels: 1,
            sample_rate: 44_100,
            block_align: 280,
        },
    ] {
        let wrapped = riff(&[0; 560], format);
        assert_eq!(read_format(&wrapped).unwrap(), format);
    }
}

/// The `fmt ` chunk is found by walking, so it must still be found with
/// something in front of it - which is not hypothetical: `Data.wad`'s
/// entries carry `fact` and `smpl` chunks alongside it.
#[test]
fn the_format_is_found_past_an_intervening_chunk() {
    let format = Format {
        channels: 2,
        sample_rate: 44_100,
        block_align: 560,
    };
    let wrapped = riff(&[0; 560], format);

    let mut shuffled = Vec::new();
    shuffled.extend_from_slice(&wrapped[0..12]);
    // An odd-length chunk, so the pad-byte rule is exercised too.
    shuffled.extend_from_slice(b"fact");
    shuffled.extend_from_slice(&5u32.to_le_bytes());
    shuffled.extend_from_slice(&[0; 6]);
    shuffled.extend_from_slice(&wrapped[12..]);

    assert_eq!(read_format(&shuffled).unwrap(), format);
}

/// A soundtrack track is picked out by its declared length, which comes
/// from `fact` and never from the stored size - so `describe` has to find
/// the chunk, and has to find it with only the header in hand. That last
/// part is the case that actually happens: `psp_soundtrack` peeks a
/// kibibyte of a 2 MiB entry, which cuts the `data` chunk short.
#[test]
fn a_peeked_header_still_yields_the_length_the_fact_chunk_declares() {
    let format = Format {
        channels: 2,
        sample_rate: 44_100,
        block_align: 560,
    };
    let mut blob = riff(&[0; 5600], format);
    // The disc writes `fact` between `fmt ` and `data`; put it there.
    let data_at = HEADER_LEN - 8;
    let mut fact = Vec::from(*b"fact");
    fact.extend_from_slice(&4u32.to_le_bytes());
    fact.extend_from_slice(&8_272_316u32.to_le_bytes());
    blob.splice(data_at..data_at, fact);

    let whole = describe(&blob).expect("a readable stream");
    assert_eq!(whole.format, format);
    assert_eq!(whole.samples, Some(8_272_316));
    let seconds = whole.seconds().expect("a length");
    assert!((seconds - 187.581).abs() < 1e-3, "{seconds} s");

    // Truncated to the header, which is what a peek hands over. The `data`
    // chunk's declared length now runs past the end of the blob, and that
    // must not be read as a malformed file.
    let peeked = describe(&blob[..HEADER_LEN + 8]).expect("a peeked header");
    assert_eq!(peeked, whole);
}

/// Every stream [`riff`] writes has no `fact` chunk, and that is a length
/// this cannot state rather than a length of zero.
#[test]
fn a_stream_with_no_fact_chunk_states_no_length() {
    let stream = describe(&riff(
        &[0; 560],
        Format {
            channels: 1,
            sample_rate: 44_100,
            block_align: 280,
        },
    ))
    .expect("a readable stream");
    assert_eq!(stream.samples, None);
    assert_eq!(stream.seconds(), None);
}

#[test]
fn a_blob_that_is_not_riff_is_refused() {
    assert!(read_format(b"PSMF0015").is_err());
    assert!(read_format(&[]).is_err());
    // RIFF/WAVE with nothing in it at all.
    let mut empty = Vec::from(*b"RIFF");
    empty.extend_from_slice(&4u32.to_le_bytes());
    empty.extend_from_slice(b"WAVE");
    assert!(read_format(&empty).is_err());
}

/// The key has to be the same number in every process, or the cache is
/// written once per run and read never. Asserted against a literal rather
/// than against a second call: two calls in one process agree even under a
/// per-process seeded hasher, which is exactly the bug this guards.
#[test]
fn the_content_key_is_fixed_for_all_time() {
    assert_eq!(content_key(b""), "cbf29ce484222325");
    assert_eq!(content_key(b"RIFF"), "f3449c2c980f8250");
    assert_ne!(content_key(b"RIFF"), content_key(b"RIFG"));
}

/// A truncated cache file is re-decoded rather than played short - the
/// shape an interrupted `ffmpeg` leaves behind.
#[test]
fn a_cache_file_that_is_not_whole_frames_is_rejected() {
    let stereo = Format {
        channels: 2,
        sample_rate: 44_100,
        block_align: 560,
    };
    assert!(from_s16le(&[1, 2, 3], stereo).is_none(), "half a frame");
    assert!(from_s16le(&[], stereo).is_none(), "nothing at all");

    let whole = from_s16le(&[0, 1, 2, 3], stereo).expect("one stereo frame");
    assert_eq!(whole.samples, vec![0x0100_i16, 0x0302]);
    assert_eq!(whole.sample_rate, 44_100);

    // A byte count that is whole frames for one channel and not for two,
    // which is the case the channel count has to be consulted for.
    let mono = Format {
        channels: 1,
        ..stereo
    };
    assert!(from_s16le(&[0, 1], mono).is_some());
    assert!(from_s16le(&[0, 1], stereo).is_none());
}

/// The cache file a run never wrote is a miss, not a panic.
#[test]
fn an_absent_cache_file_is_a_miss() {
    let stereo = Format {
        channels: 2,
        sample_rate: 44_100,
        block_align: 560,
    };
    assert!(read_cached(Path::new("no/such/cache.s16le"), stereo).is_none());
}
