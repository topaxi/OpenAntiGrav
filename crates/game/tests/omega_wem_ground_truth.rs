//! The Omega Collection's `.wem` media: identified as ATRAC9, decoded in
//! process, and one circuit's cue played through the mixer.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! `omega_data08_*` read the patch's archive, which carries the newer copy of
//! every bank and all of the loose media; `omega_data00_*` the base's. The
//! ffmpeg cross-check needs `ffmpeg` on `PATH` and **fails, not skips, under
//! `OAG_REQUIRE_GAME_DATA` without it**.

use std::path::PathBuf;
use std::sync::Arc;

use oag_audio::{Bus, Mixer, Play, Sound};
use oag_formats::wwise::wem::{TAG_ATRAC9, TAG_PCM, Wem};
use oag_formats::wwise::{Bank, Library};
use oag_game::wem;

fn archive(dir: &str, name: &str) -> Option<oag_assets::psarc::Archive> {
    let path: PathBuf = oag_testdata::exact(&format!("data/extracted/ps4/{dir}/uroot/{name}"))?;
    Some(oag_assets::psarc::Archive::open_file(&path).expect("the archive opens"))
}

/// What one `.wem` measured.
#[derive(Debug, Default, PartialEq, Eq)]
struct Census {
    files: usize,
    atrac9: usize,
    pcm: usize,
    /// ATRAC9 files by channel count.
    mono: usize,
    stereo: usize,
    quad: usize,
    hexa: usize,
    octo: usize,
    /// Files at 24 kHz; the rest are 48 kHz.
    rate_24k: usize,
    /// The chunk sequence `fmt `, `JUNK`, `data` on every file.
    plain_chunk_order: usize,
    /// Files whose config word, `fmt `, and frame arithmetic all agree.
    consistent: usize,
    /// Files the decoder took without an error, and whose output is exactly
    /// the declared length.
    decoded: usize,
    /// Of those, ones that are digital silence throughout.
    silent: usize,
    /// Embedded media that are the head of a loose `.wem` and no whole file:
    /// the bytes are that file's first ones, and its `data` chunk runs on.
    prefetch_heads: usize,
    /// Files `wem::decode` refused: stereo and mono (none), and four or eight
    /// channels (the crate's README rates multi-channel "partial").
    refused_stereo_or_mono: usize,
    refused_multichannel: usize,
}

/// Structural and decode checks for one file; `decode` says whether to run the
/// decoder (the music streams are hundreds of megabytes between them).
fn check(c: &mut Census, label: &str, blob: &[u8], decode: bool) {
    let wem = Wem::parse(blob).unwrap_or_else(|e| panic!("{label}: {e}"));
    c.files += 1;
    let tags = wem.tags();
    c.plain_chunk_order += usize::from(tags == [*b"fmt ", *b"JUNK", *b"data"]);
    let format = wem.format();
    match format.tag {
        TAG_PCM => {
            c.pcm += 1;
            assert_eq!(
                (format.channels, format.bits_per_sample),
                (1, 16),
                "{label}: Wwise PCM here is mono 16-bit, at 32 or 44.1 kHz"
            );
            assert_eq!(wem.payload().len() % 2, 0, "{label}");
            return;
        }
        TAG_ATRAC9 => c.atrac9 += 1,
        other => panic!("{label}: a fmt tag {other:#06x} that is neither ATRAC9 nor PCM"),
    }
    match format.channels {
        1 => c.mono += 1,
        2 => c.stereo += 1,
        4 => c.quad += 1,
        6 => c.hexa += 1,
        8 => c.octo += 1,
        n => panic!("{label}: {n} channels"),
    }
    let a = format
        .atrac9()
        .unwrap_or_else(|| panic!("{label}: no 18-byte ATRAC9 extra data"));
    // Sample-rate index 7 is 48 kHz and 4 is 24 kHz in ATRAC9's table, and the
    // frame is 256 samples at the first and 128 at the second.
    let (rate_index, frame_samples) = match format.sample_rate {
        48_000 => (7, 256),
        24_000 => {
            c.rate_24k += 1;
            (4, 128)
        }
        other => panic!("{label}: {other} Hz"),
    };
    assert_eq!(format.bits_per_sample, 0, "{label}");
    assert_eq!(usize::from(a.frame_samples), frame_samples, "{label}");
    assert_eq!(
        a.delay,
        u32::from(a.frame_samples),
        "{label}: the delay is one frame"
    );
    // The config word: sync, sample-rate index, channel-config index,
    // validation bit, frame bytes - 1 and superframe index, read big-endian.
    let word = u32::from_be_bytes(a.config);
    let (sync, rate, config_index, validation, frame_bytes, superframe) = (
        word >> 24,
        (word >> 20) & 0xf,
        (word >> 17) & 7,
        (word >> 16) & 1,
        ((word >> 5) & 0x7ff) + 1,
        (word >> 3) & 3,
    );
    assert_eq!(
        (sync, rate, validation, superframe),
        (0xfe, rate_index, 0, 0),
        "{label}"
    );
    assert_eq!(
        frame_bytes,
        u32::from(format.block_align),
        "{label}: frame bytes is block align"
    );
    let want_index = match format.channels {
        1 => 0,
        2 => 2,
        4 => 5,
        6 => 3,
        _ => 4,
    };
    assert_eq!(
        config_index, want_index,
        "{label}: channel-config index tracks channels"
    );
    // Wwise's AkChannelConfig: count, type 1, speaker mask.
    assert_eq!(
        a.channel_config & 0xff,
        u32::from(format.channels),
        "{label}"
    );
    assert_eq!((a.channel_config >> 8) & 0xf, 1, "{label}");
    assert_eq!(
        a.channel_config >> 12,
        match format.channels {
            1 => 0x4,
            2 => 0x3,
            4 => 0x603,
            6 => 0x60f,
            _ => 0x63f,
        },
        "{label}"
    );
    // The frame arithmetic: whole blocks, a 256-sample delay, and less than a
    // frame of padding after the last sample.
    assert_eq!(
        wem.payload().len() % usize::from(format.block_align),
        0,
        "{label}"
    );
    let decoded = wem.frames() * frame_samples;
    let pad = decoded
        .checked_sub(a.samples as usize + a.delay as usize)
        .unwrap_or_else(|| panic!("{label}: fewer samples decoded than the header states"));
    assert!(pad < frame_samples, "{label}: {pad} samples of padding");
    c.consistent += 1;
    if !decode {
        return;
    }
    match wem::decode(blob) {
        Ok(pcm) => {
            assert_eq!(
                pcm.samples.len(),
                a.samples as usize * usize::from(format.channels),
                "{label}: the decode is exactly the declared length"
            );
            c.decoded += 1;
            c.silent += usize::from(pcm.samples.iter().all(|&s| s == 0));
        }
        Err(e) => {
            if format.channels <= 2 {
                c.refused_stereo_or_mono += 1;
            } else {
                c.refused_multichannel += 1;
            }
            println!("{label}: {e}");
        }
    }
}

/// Every loose `.wem` of an archive, and its bytes.
fn loose(archive: &mut oag_assets::psarc::Archive) -> Vec<(String, Vec<u8>)> {
    let paths: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with(".wem"))
        .cloned()
        .collect();
    paths
        .into_iter()
        .map(|p| {
            let blob = archive.read_path(&p).expect("the .wem reads");
            (p, blob)
        })
        .collect()
}

/// Every bank of an archive, and its bytes.
fn banks(archive: &mut oag_assets::psarc::Archive) -> Vec<(String, Vec<u8>)> {
    let paths: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.ends_with(".bnk"))
        .cloned()
        .collect();
    paths
        .into_iter()
        .map(|p| {
            let blob = archive.read_path(&p).expect("the bank reads");
            (p, blob)
        })
        .collect()
}

/// Files above this many bytes are checked structurally and not decoded.
const DECODE_LIMIT: usize = usize::MAX;

fn loose_census(dir: &str, name: &str) -> Option<Census> {
    let mut archive = archive(dir, name)?;
    let mut c = Census::default();
    for (path, blob) in loose(&mut archive) {
        check(&mut c, &path, &blob, blob.len() <= DECODE_LIMIT);
    }
    Some(c)
}

fn embedded_census(dir: &str, name: &str) -> Option<Census> {
    let mut archive = archive(dir, name)?;
    let mut c = Census::default();
    // Loose files by media id, to answer what a truncated embedded one is the
    // head of.
    let loose_paths: std::collections::BTreeMap<u32, String> = archive
        .paths()
        .iter()
        .filter_map(|p| {
            let id = p.rsplit('/').next()?.strip_suffix(".wem")?.parse().ok()?;
            Some((id, p.clone()))
        })
        .collect();
    for (path, blob) in banks(&mut archive) {
        let Ok(bank) = Bank::parse(&blob) else {
            continue;
        };
        for media in bank.media() {
            let bytes = bank.embedded(media.id).expect("a range inside DATA");
            let label = format!("{path}#{}", media.id);
            if let Err(oag_formats::wwise::wem::Error::Truncated { .. }) = Wem::parse(bytes) {
                let loose = loose_paths
                    .get(&media.id)
                    .unwrap_or_else(|| panic!("{label}: a truncated wem with no loose file"));
                let whole = archive.read_path(loose).expect("the loose .wem reads");
                assert!(whole.starts_with(bytes), "{label}: the head of {loose}");
                c.prefetch_heads += 1;
                continue;
            }
            check(&mut c, &label, bytes, bytes.len() <= DECODE_LIMIT);
        }
    }
    Some(c)
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu"]
fn omega_data00_loose_wems_are_atrac9_and_decode() {
    let Some(c) = loose_census("omega-eu", "data00.psarc") else {
        return;
    };
    assert_eq!(
        c,
        Census {
            files: 714,
            atrac9: 714,
            pcm: 0,
            mono: 0,
            stereo: 665,
            quad: 38,
            hexa: 0,
            octo: 11,
            rate_24k: 0,
            plain_chunk_order: 714,
            consistent: 714,
            decoded: 665,
            silent: 1,
            prefetch_heads: 0,
            refused_stereo_or_mono: 0,
            refused_multichannel: 49
        }
    );
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu-patch"]
fn omega_data08_loose_wems_are_atrac9_and_decode() {
    let Some(c) = loose_census("omega-eu-patch", "data08.psarc") else {
        return;
    };
    assert_eq!(
        c,
        Census {
            files: 734,
            atrac9: 734,
            pcm: 0,
            mono: 0,
            stereo: 685,
            quad: 38,
            hexa: 0,
            octo: 11,
            rate_24k: 0,
            plain_chunk_order: 734,
            consistent: 734,
            decoded: 685,
            silent: 1,
            prefetch_heads: 0,
            refused_stereo_or_mono: 0,
            refused_multichannel: 49
        }
    );
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu"]
fn omega_data00_embedded_media_are_wems_too() {
    let Some(c) = embedded_census("omega-eu", "data00.psarc") else {
        return;
    };
    assert_eq!(
        c,
        Census {
            files: 1958,
            atrac9: 1956,
            pcm: 2,
            mono: 738,
            stereo: 1209,
            quad: 0,
            hexa: 2,
            octo: 7,
            rate_24k: 295,
            plain_chunk_order: 1052,
            consistent: 1956,
            decoded: 1947,
            silent: 0,
            prefetch_heads: 164,
            refused_stereo_or_mono: 0,
            refused_multichannel: 9
        }
    );
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu-patch"]
fn omega_data08_embedded_media_are_wems_too() {
    let Some(c) = embedded_census("omega-eu-patch", "data08.psarc") else {
        return;
    };
    assert_eq!(
        c,
        Census {
            files: 2082,
            atrac9: 2080,
            pcm: 2,
            mono: 838,
            stereo: 1233,
            quad: 0,
            hexa: 2,
            octo: 7,
            rate_24k: 295,
            plain_chunk_order: 1076,
            consistent: 2080,
            decoded: 2071,
            silent: 1,
            prefetch_heads: 175,
            refused_stereo_or_mono: 0,
            refused_multichannel: 9
        }
    );
}

/// Tech De Ra's ambience bank, and the first event of it that plays a stereo
/// ATRAC9 sound embedded in the bank itself: **one circuit's cue, from the
/// bank's own event to the mixer's output.**
///
/// The path is the whole of it - `event -> Play action -> target -> the sounds
/// below it -> media id -> the bank's own `DIDX` -> a `.wem` -> ATRAC9 -> PCM ->
/// [`oag_audio::Sound`] -> [`oag_audio::Mixer`] -> a WAV** - with nothing chosen
/// on the way except *which* event, which is the first that qualifies in file
/// order. No sound is played on any device: the mixer's output is written as a
/// WAV and measured, the same way `--dump-audio` does.
#[test]
#[ignore = "needs data/extracted/ps4/omega-eu-patch"]
fn a_tech_de_ra_cue_plays_through_the_mixer() {
    let Some(mut archive) = archive("omega-eu-patch", "data08.psarc") else {
        return;
    };
    let path = archive
        .paths()
        .iter()
        .find(|p| {
            p.to_ascii_lowercase()
                .ends_with("audio/sound/env12_techdera.bnk")
        })
        .cloned()
        .expect("Tech De Ra's ambience bank is in the patch");
    let blob = archive.read_path(&path).expect("the bank reads");
    let bank = Bank::parse(&blob).expect("it parses");
    let library = Library::new(vec![bank]);
    let bank = &library.banks()[0];
    let (event, media) = bank
        .objects()
        .iter()
        .filter(|o| o.kind == oag_formats::wwise::Kind::Event)
        .find_map(|event| {
            let plan = library.resolve_event(event.id)?;
            let media = plan
                .media
                .iter()
                .find(|m| m.embedded_in == Some(0) && !m.is_loose())?
                .source
                .media_id;
            let bytes = bank.embedded(media)?;
            (wem::describe(bytes).ok()?.channels == 2).then_some((event.id, media))
        })
        .expect("an event that plays an embedded stereo sound");
    let bytes = bank.embedded(media).expect("in DATA");
    let stream = wem::describe(bytes).expect("ATRAC9");
    let pcm = wem::decode(bytes).expect("decodes");
    assert_eq!(pcm.channels, 2);
    assert_eq!(pcm.sample_rate, 48_000);
    assert_eq!(pcm.samples.len(), stream.frames as usize * 2);

    let sound = Arc::new(Sound::new(pcm.samples.clone(), pcm.channels, pcm.sample_rate).unwrap());
    let mut mixer = Mixer::new(pcm.sample_rate);
    mixer
        .play(Play::once(sound, Bus::Sfx))
        .expect("a free voice");
    let frames = stream.frames as usize + 256;
    let mut out = vec![0.0f32; frames * 2];
    mixer.render(&mut out);

    let rms = |v: &mut dyn Iterator<Item = f64>| {
        let (n, sum) = v.fold((0usize, 0.0), |(n, s), x| (n + 1, s + x * x));
        (sum / n.max(1) as f64).sqrt()
    };
    let decoded_rms = rms(&mut pcm.samples.iter().map(|&s| f64::from(s) / 32768.0));
    let rendered_rms = rms(&mut out[..stream.frames as usize * 2]
        .iter()
        .map(|&s| f64::from(s)));
    let peak = out.iter().fold(0.0f32, |p, s| p.max(s.abs()));
    println!(
        "event {event:#010x} -> media {media} ({} frames, {:.3} s): decoded rms {decoded_rms:.4}, \
         rendered rms {rendered_rms:.4}, peak {peak:.3}, clipped {}",
        stream.frames,
        stream.frames as f64 / 48_000.0,
        mixer.clipped(),
    );
    assert!(decoded_rms > 0.001, "the decoded clip is not silence");
    assert!(
        (rendered_rms / decoded_rms - 1.0).abs() < 0.02,
        "the mixer passes it through at unity: {rendered_rms} against {decoded_rms}"
    );
    let wav = oag_audio::wav::from_samples(&out, pcm.sample_rate);
    if let Some(dir) = std::env::var_os("OAG_WEM_OUT") {
        let dir = PathBuf::from(dir);
        std::fs::create_dir_all(&dir).expect("the output directory");
        std::fs::write(dir.join("techdera_cue.wav"), &wav).expect("the WAV writes");
    }
    assert_eq!(&wav[..4], b"RIFF");
}

/// The crate and FFmpeg's independent decoder agree, up to polarity.
///
/// FFmpeg is the second ATRAC9 implementation: the crate is a port of
/// LibAtrac9 and FFmpeg's is its own, so a sample-for-sample agreement is not
/// one program checking itself. **One is the negation of the other**, which is
/// asserted rather than hidden.
#[test]
#[ignore = "needs data/extracted/ps4/omega-eu-patch and ffmpeg"]
fn the_decoder_agrees_with_ffmpeg_up_to_polarity() {
    let Some(mut archive) = archive("omega-eu-patch", "data08.psarc") else {
        return;
    };
    // The smallest stereo `.wem`, and the smallest four-channel one, so the
    // check stays cheap and covers a second layout.
    let mut files = loose(&mut archive);
    files.sort_by_key(|(_, blob)| blob.len());
    let mut picked: Vec<(u16, &str, &[u8])> = Vec::new();
    for channels in [2u16, 4] {
        if let Some((p, b)) = files.iter().find(|(_, b)| {
            Wem::parse(b)
                .is_ok_and(|w| w.format().tag == TAG_ATRAC9 && w.format().channels == channels)
        }) {
            picked.push((channels, p, b));
        }
    }
    assert!(!picked.is_empty(), "a stereo file at least");
    let ffmpeg_present = std::process::Command::new("ffmpeg")
        .arg("-version")
        .output()
        .is_ok();
    if !ffmpeg_present {
        assert!(
            std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
            "OAG_REQUIRE_GAME_DATA is set but ffmpeg is not on PATH"
        );
        println!("skipping: ffmpeg is not on PATH");
        return;
    }
    for (channels, label, blob) in picked {
        let wem = Wem::parse(blob).unwrap();
        let a = wem.format().atrac9().unwrap();
        let reference = ffmpeg_pcm(&wem);
        assert_eq!(
            reference.len(),
            wem.frames() * usize::from(a.frame_samples) * usize::from(channels),
            "{label}: ffmpeg decodes every frame without error"
        );
        let ours = wem::decode(blob);
        if channels > 2 {
            // The measured limitation: FFmpeg decodes it and the crate stops
            // part-way. When the crate is fixed this fails, and the census
            // pins above want their multichannel numbers updated.
            let e = ours.expect_err("the crate refuses four-channel ATRAC9 today");
            println!("{label} ({channels} ch): ffmpeg decodes it; the crate: {e}");
            continue;
        }
        let ours = ours.expect("stereo decodes");
        let skip = a.delay as usize * usize::from(channels);
        let mut worst = 0i32;
        let mut identical = 0usize;
        for (i, &s) in ours.samples.iter().enumerate() {
            let theirs = reference[skip + i];
            worst = worst.max((i32::from(s) + i32::from(theirs)).abs());
            identical += usize::from(i32::from(s) + i32::from(theirs) == 0);
        }
        println!(
            "{label} ({channels} ch): {} samples, worst |ours + ffmpeg| {worst}, identical {:.2}%",
            ours.samples.len(),
            100.0 * identical as f64 / ours.samples.len() as f64
        );
        assert!(
            worst <= 1,
            "{label}: stereo agrees with ffmpeg to one LSB, negated"
        );
    }
}

/// FFmpeg's decode of an ATRAC9 `.wem`, rewrapped as the `.at9` its demuxer
/// knows: an extensible `fmt ` with ATRAC9's GUID and the 12-byte extra data
/// (`version`, the config word, a reserved word), then the frames.
fn ffmpeg_pcm(wem: &Wem<'_>) -> Vec<i16> {
    const ATRAC9_GUID: [u8; 16] = [
        0xD2, 0x42, 0xE1, 0x47, 0xBA, 0x36, 0x8D, 0x4D, 0x88, 0xFC, 0x61, 0x65, 0x4F, 0x8C, 0x83,
        0x6C,
    ];
    let f = wem.format();
    let a = f.atrac9().unwrap();
    let mut ext = 0x0FFFu16.to_le_bytes().to_vec();
    ext.extend_from_slice(&(a.channel_config >> 12).to_le_bytes());
    ext.extend_from_slice(&ATRAC9_GUID);
    ext.extend_from_slice(&2u32.to_le_bytes());
    ext.extend_from_slice(&a.config);
    ext.extend_from_slice(&0u32.to_le_bytes());
    let mut fmt = 0xFFFEu16.to_le_bytes().to_vec();
    fmt.extend_from_slice(&f.channels.to_le_bytes());
    fmt.extend_from_slice(&f.sample_rate.to_le_bytes());
    fmt.extend_from_slice(&f.avg_bytes_per_sec.to_le_bytes());
    fmt.extend_from_slice(&f.block_align.to_le_bytes());
    fmt.extend_from_slice(&0u16.to_le_bytes());
    fmt.extend_from_slice(&(ext.len() as u16).to_le_bytes());
    fmt.extend(ext);
    let mut body = b"WAVEfmt ".to_vec();
    body.extend_from_slice(&(fmt.len() as u32).to_le_bytes());
    body.extend(fmt);
    body.extend_from_slice(b"data");
    body.extend_from_slice(&(wem.payload().len() as u32).to_le_bytes());
    body.extend_from_slice(wem.payload());
    let mut file = b"RIFF".to_vec();
    file.extend_from_slice(&(body.len() as u32).to_le_bytes());
    file.extend(body);
    let dir = std::env::temp_dir().join(format!("oag-wem-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let input = dir.join("in.at9");
    std::fs::write(&input, &file).unwrap();
    let out = std::process::Command::new("ffmpeg")
        .args(["-hide_banner", "-v", "error", "-xerror", "-i"])
        .arg(&input)
        .args(["-f", "s16le", "-"])
        .output()
        .expect("ffmpeg runs");
    assert!(
        out.status.success(),
        "ffmpeg: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.stderr.is_empty(),
        "ffmpeg: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    std::fs::remove_dir_all(&dir).ok();
    out.stdout
        .chunks_exact(2)
        .map(|b| i16::from_le_bytes([b[0], b[1]]))
        .collect()
}
