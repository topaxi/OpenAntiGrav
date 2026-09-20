//! What [`super`] is asserted to do: both the ATRAC3+ demux off a `.PMF` and
//! the PCM de-interleave off a `.PSS`.
//!
//! Its own file rather than a `#[cfg(test)]` block inside `track.rs`, per
//! `scripts/check-file-size.py`'s 200-line rule for an inline module - this
//! one moved with the code under test when `movie_audio` and its tests split
//! out of `movie.rs`/`movie/tests.rs`.

use super::*;

/// Pulls the raw ATRAC3+ blocks back out of a [`MovieAudio`], for tests that
/// need to see past [`MovieAudio::decode`] into what was actually demuxed.
fn atrac3plus_blocks(audio: &MovieAudio) -> &[u8] {
    match &audio.kind {
        MovieAudioKind::Atrac3Plus { blocks, .. } => blocks,
        MovieAudioKind::Pcm(_) => panic!("expected an ATRAC3+ track"),
    }
}

/// One audio packet, built the way a `.PMF` builds them: a four-byte
/// sub-header, then frames that are an eight-byte header and a block.
fn audio_packet(pointer: u16, frames: &[Vec<u8>]) -> Vec<u8> {
    let mut out = vec![0, 0];
    out.extend_from_slice(&pointer.to_be_bytes());
    for frame in frames {
        out.extend_from_slice(frame);
    }
    out
}

fn atrac_frame(block_align: usize, fill: u8) -> Vec<u8> {
    let mut out = Vec::from(ATRAC3PLUS_SYNC);
    out.extend_from_slice(&[0x28, 0x5c, 0, 0, 0, 0]);
    out.extend_from_slice(&vec![fill; block_align]);
    out
}

fn stereo_44k() -> pmf::AudioStream {
    pmf::AudioStream {
        channels: 2,
        frequency_code: 2,
    }
}

/// Both layers of framing come off and the block size is measured rather
/// than assumed - the disc uses 744 and 560, so a constant would decode one
/// of them into noise.
#[test]
fn the_audio_framing_comes_off_and_the_block_size_is_measured() {
    let demuxed = pmf::Demuxed {
        audio: vec![audio_packet(
            0,
            &[atrac_frame(744, 0xab), atrac_frame(744, 0xcd)],
        )],
        ..pmf::Demuxed::default()
    };
    let audio = movie_audio(&demuxed, stereo_44k(), "test").expect("a track");

    assert_eq!(audio.block_align(), Some(744), "measured, not assumed");
    assert_eq!(audio.channels(), 2);
    assert_eq!(audio.sample_rate(), 44_100);
    assert_eq!(audio.block_count(), 2);
    let blocks = atrac3plus_blocks(&audio);
    assert_eq!(blocks.len(), 744 * 2);
    assert!(
        blocks[..744].iter().all(|&b| b == 0xab),
        "the sync word and its header should be gone"
    );
    assert!(blocks[744..].iter().all(|&b| b == 0xcd));
}

/// A frame straddles the packet boundary in a real `.PMF` - `Intro.PMF`'s
/// first packet ends 243 bytes into its third frame - so reassembly has to
/// be a concatenation rather than a frame per packet.
#[test]
fn a_frame_split_across_two_packets_is_put_back_together() {
    let whole = atrac_frame(560, 0x11);
    let (head, tail) = whole.split_at(200);
    let demuxed = pmf::Demuxed {
        audio: vec![
            audio_packet(0, &[atrac_frame(560, 0x22), head.to_vec()]),
            // 368 bytes of the previous frame before the next one starts.
            audio_packet(368, &[tail.to_vec(), atrac_frame(560, 0x33)]),
        ],
        ..pmf::Demuxed::default()
    };
    let audio = movie_audio(&demuxed, stereo_44k(), "test").expect("a track");
    assert_eq!(audio.block_align(), Some(560));
    assert_eq!(audio.block_count(), 3);
    assert!(
        atrac3plus_blocks(&audio)[560..1120]
            .iter()
            .all(|&b| b == 0x11)
    );
}

/// A movie whose first packet opens partway into a frame skips to the
/// frame boundary rather than handing `ffmpeg` a fragment. No movie on the
/// disc does this, and a decode half a block out is silent noise rather
/// than an error, so it is pinned rather than left to chance.
#[test]
fn a_first_packet_that_opens_mid_frame_is_skipped_to_the_boundary() {
    let mut packet = audio_packet(16, &[]);
    packet.extend_from_slice(&[0xff; 16]);
    packet.extend_from_slice(&atrac_frame(560, 0x44));
    packet.extend_from_slice(&atrac_frame(560, 0x44));
    let demuxed = pmf::Demuxed {
        audio: vec![packet],
        ..pmf::Demuxed::default()
    };
    let audio = movie_audio(&demuxed, stereo_44k(), "test").expect("a track");
    assert_eq!(audio.block_count(), 2);
    assert!(atrac3plus_blocks(&audio).iter().all(|&b| b == 0x44));
}

/// Every way a track can be unreadable is silence rather than a failure,
/// because none of them should cost the movie its picture.
#[test]
fn an_unreadable_track_is_silence_rather_than_an_error() {
    let good = audio_packet(0, &[atrac_frame(560, 0x55), atrac_frame(560, 0x66)]);

    // A header that declares audio and a demux that found none.
    assert!(movie_audio(&pmf::Demuxed::default(), stereo_44k(), "test").is_none());

    // A sample-rate code this build does not know.
    let unknown = pmf::AudioStream {
        channels: 2,
        frequency_code: 7,
    };
    let demuxed = pmf::Demuxed {
        audio: vec![good.clone()],
        ..pmf::Demuxed::default()
    };
    assert!(movie_audio(&demuxed, unknown, "test").is_none());

    // A first frame with no sync word at all.
    let mut wrong = good.clone();
    wrong[4] = 0x00;
    let demuxed = pmf::Demuxed {
        audio: vec![wrong],
        ..pmf::Demuxed::default()
    };
    assert!(movie_audio(&demuxed, stereo_44k(), "test").is_none());

    // One sync word and never a second, so no stride can be measured.
    let demuxed = pmf::Demuxed {
        audio: vec![audio_packet(0, &[atrac_frame(560, 0x77)])],
        ..pmf::Demuxed::default()
    };
    assert!(movie_audio(&demuxed, stereo_44k(), "test").is_none());
}

/// Block granularity, stated as such: a track is always a whole number of
/// 2,048-sample blocks and so runs slightly past the movie's own duration.
#[test]
fn a_tracks_length_is_a_whole_number_of_blocks() {
    let demuxed = pmf::Demuxed {
        audio: vec![audio_packet(
            0,
            &[
                atrac_frame(744, 0),
                atrac_frame(744, 0),
                atrac_frame(744, 0),
            ],
        )],
        ..pmf::Demuxed::default()
    };
    let audio = movie_audio(&demuxed, stereo_44k(), "test").expect("a track");
    assert_eq!(audio.block_count(), 3);
    let expected = 3.0 * 2048.0 / 44_100.0;
    assert!((audio.seconds() - expected).abs() < 1e-9);
}

/// A minimal `pss::Demuxed` with a 512-byte-per-channel interleave: `left`
/// and `right` are each padded to whole 512-byte chunks and interleaved
/// chunk by chunk, the same layout `oag_video::pss`'s module docs measure
/// off the real disc.
fn pcm_demuxed(left: &[i16], right: &[i16]) -> pss::Demuxed {
    let interleave = 512usize;
    let zeros = vec![0u8; interleave];
    let to_bytes = |samples: &[i16]| -> Vec<u8> {
        let mut out: Vec<u8> = samples.iter().flat_map(|s| s.to_le_bytes()).collect();
        out.resize(out.len().next_multiple_of(interleave), 0);
        out
    };
    let (l, r) = (to_bytes(left), to_bytes(right));
    let chunks = l.len().max(r.len()) / interleave;
    let mut body = Vec::with_capacity(chunks * interleave * 2);
    for i in 0..chunks {
        let at = i * interleave;
        body.extend_from_slice(l.get(at..at + interleave).unwrap_or(&zeros));
        body.extend_from_slice(r.get(at..at + interleave).unwrap_or(&zeros));
    }
    pss::Demuxed {
        format: pss::Format {
            format: pss::FORMAT_PCM16,
            sample_rate: 48_000,
            channels: 2,
            interleave,
        },
        body,
        packet_count: 1,
    }
}

/// The PS2 path needs no codec at all: the samples come back exactly as
/// supplied, interleaved left-first per stereo frame.
#[test]
fn pcm_channels_are_read_back_interleaved_left_first() {
    let left: Vec<i16> = (0i16..300).collect();
    let right: Vec<i16> = (0i16..300).map(|n| -n).collect();
    let demuxed = pcm_demuxed(&left, &right);

    let audio = pss_audio(&demuxed, "test").expect("a track");
    assert_eq!(audio.channels(), 2);
    assert_eq!(audio.sample_rate(), 48_000);
    assert_eq!(
        audio.block_align(),
        None,
        "a PCM track has no ATRAC3+ blocks"
    );
    assert_eq!(audio.block_count(), 0);

    let pcm = audio
        .decode(Path::new("/does-not-matter"))
        .expect("decoding is infallible for PCM");
    assert_eq!(pcm.channels, 2);
    assert_eq!(pcm.sample_rate, 48_000);
    for i in 0..300 {
        assert_eq!(pcm.samples[i * 2], left[i], "left sample {i}");
        assert_eq!(pcm.samples[i * 2 + 1], right[i], "right sample {i}");
    }
}

/// `format` codes other than [`pss::FORMAT_PCM16`] are not decoded on a
/// guess - the track stays silent, the same treatment an unreadable ATRAC3+
/// track gets.
#[test]
fn an_unknown_format_code_is_silence_rather_than_a_guess() {
    let mut demuxed = pcm_demuxed(&[0; 512], &[0; 512]);
    demuxed.format.format = 2;
    assert!(pss_audio(&demuxed, "test").is_none());
}

/// A channel count other than the measured stereo layout is also silence:
/// de-interleaving a scheme never seen is worse than saying nothing.
#[test]
fn an_unmeasured_channel_count_is_silence_rather_than_a_guess() {
    let mut demuxed = pcm_demuxed(&[0; 512], &[0; 512]);
    demuxed.format.channels = 1;
    assert!(pss_audio(&demuxed, "test").is_none());
}
