//! Wipeout 2048's one movie sound track, `intro.mp4`'s AAC, decoded in
//! process and checked against FFmpeg's independent decoder.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! The census is every `.mp4` of both regions' base packages: the EU package's
//! `intro.mp4` carries a sound track, and the USA package's `intro.mp4` and
//! `introAS.mp4` do (`docs/formats/mp4.md`). A missing `ffmpeg` skips the
//! comparison, printing why.

use oag_assets::psarc::Archive;

/// What the census and the decode measured, per base package.
#[derive(Debug, PartialEq, Eq)]
struct Measured {
    mp4s: usize,
    /// Per-channel samples of each movie with a sound track, in path order.
    frames: Vec<usize>,
}

fn measure(path: &str) -> Option<Measured> {
    let path = oag_testdata::exact(path)?;
    let mut archive = Archive::open_file(&path).expect("the archive opens");
    let mp4s: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.to_ascii_lowercase().ends_with(".mp4"))
        .cloned()
        .collect();
    let mut frames = Vec::new();
    for p in &mp4s {
        let blob = archive.read_path(p).expect("the movie reads");
        let header = oag_video::mp4::parse(&blob).unwrap_or_else(|e| panic!("{p}: {e}"));
        let Some(track) = header.audio else {
            continue;
        };
        assert_eq!(&track.codec, b"mp4a", "{p}");
        let pcm = oag_music::aac::decode_mp4(&blob).unwrap_or_else(|e| panic!("{p}: {e:#}"));
        assert_eq!(
            (pcm.channels, pcm.sample_rate),
            (track.channel_count, track.sample_rate),
            "{p}"
        );
        let channels = usize::from(pcm.channels);
        frames.push(pcm.samples.len() / channels);
        assert_eq!(
            pcm.samples.len() / channels,
            track.frame_count * track.frame_delta as usize,
            "{p}: every AAC frame decodes to its 1,024 samples"
        );
        compare_with_ffmpeg(p, &blob, &pcm.samples);
    }
    Some(Measured {
        mp4s: mp4s.len(),
        frames,
    })
}

/// Ours against FFmpeg's decode of the same file, sample for sample.
///
/// The two are independent float AAC decoders, so they agree to rounding, not
/// bit for bit: the tolerance and the measured figures are on
/// `docs/formats/mp4.md`, "The audio decodes in process".
fn compare_with_ffmpeg(label: &str, blob: &[u8], ours: &[i16]) {
    let dir = oag_testdata::scratch::dir("aac-gt");
    let input = dir.join("in.mp4");
    std::fs::write(&input, blob).unwrap();
    let out = match std::process::Command::new("ffmpeg")
        .args(["-hide_banner", "-v", "error", "-i"])
        .arg(&input)
        .args(["-map", "0:a:0", "-f", "s16le", "-"])
        .output()
    {
        Ok(out) => out,
        Err(_) => {
            println!("skipping the ffmpeg comparison: ffmpeg is not on PATH");
            std::fs::remove_dir_all(&dir).unwrap();
            return;
        }
    };
    std::fs::remove_dir_all(&dir).unwrap();
    assert!(
        out.status.success(),
        "ffmpeg: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let theirs: Vec<i16> = out
        .stdout
        .as_chunks::<2>()
        .0
        .iter()
        .map(|b| i16::from_le_bytes(*b))
        .collect();
    let n = ours.len().min(theirs.len());
    let mut worst = 0i32;
    let mut sum_sq = 0f64;
    for (&a, &b) in ours[..n].iter().zip(&theirs[..n]) {
        let d = (i32::from(a) - i32::from(b)).abs();
        worst = worst.max(d);
        sum_sq += f64::from(d) * f64::from(d);
    }
    let rms = (sum_sq / n.max(1) as f64).sqrt();
    println!(
        "{label}: ours {} samples, ffmpeg {}, worst |ours - ffmpeg| {worst}, rms {rms:.3}",
        ours.len(),
        theirs.len()
    );
    assert!(worst <= 2, "{label}: worst {worst}");
}

#[test]
#[ignore = "needs data/extracted/vita"]
fn intro_mp4_aac_decodes_in_process() {
    for (path, mp4s, frames) in [
        (
            "data/extracted/vita/PCSF00007/base/PSP2/data.psarc",
            26,
            vec![4_666 * 1_024],
        ),
        (
            "data/extracted/vita/PCSA00015/base/PSP2/data.psarc",
            27,
            vec![4_665 * 1_024, 4_665 * 1_024],
        ),
    ] {
        let Some(m) = measure(path) else { continue };
        assert_eq!(m, Measured { mp4s, frames }, "{path}");
    }
}
