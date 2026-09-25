//! Validates Wipeout 2048's front-end music against its real package.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship - `data/extracted/vita/PCSF00007/base`, the
//! decrypted EU package. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! # What this is for
//!
//! `frontend.bnk`, the bank `docs/formats/2048-frontend.md` first pointed
//! at for a front-end music cue, turned out to carry no name table at all
//! (`the_banks_cue_table_is_empty` below), so what names `front_end` instead
//! is a standalone RIFF-wrapped ATRAC9 file the base package ships beside it,
//! `data/audio/music/FEMusic/frontend_stereo.at9` - the same `FEMusic`
//! folder and `frontend` stem Pulse's and HD's own front-end tracks use.
//! [`oag_game::at9`] is the new decoder this needed: `oag_game::at3` only
//! ever saw RIFF-wrapped ATRAC3+ before this file existed.
//!
//! Mirrors `hd_music_ground_truth.rs`'s
//! `the_front_ends_music_loads_through_the_engines_own_path`, and adds a
//! second check that one does not need: this is the first title whose
//! `MusicDiscs::survey` had to be taught a fourth release
//! ([`Platform::Vita`]) at all, so `the_boots_own_music_discs_survey_finds_it`
//! pins that against the same source `oag_game::title::open_source` opens,
//! not just `music::load_front_end` in isolation.

use std::path::{Path, PathBuf};

use oag_disc::Platform;
use oag_game::{audio, music};

fn source() -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/vita/PCSF00007");
    if path.join("base/PSP2/data.psarc").exists() {
        return Some(path);
    }
    assert!(
        std::env::var_os("OAG_REQUIRE_GAME_DATA").is_none(),
        "OAG_REQUIRE_GAME_DATA is set but {} is missing",
        path.display()
    );
    println!("skipping: {} not present", path.display());
    None
}

/// The front end's music loads **through the path the engine would use**,
/// and decodes to real, audible signal - not just a header that parses.
///
/// Writes a WAV via `oag_audio::wav::from_samples` (the same writer
/// `--dump-audio` uses) under `data/cache/` so a reviewer without a way to
/// run this test can still play back what it decoded - see
/// `docs/formats/2048-frontend.md`'s own note on where it lands.
#[test]
#[ignore = "needs game content in data/extracted/vita/PCSF00007"]
fn the_front_ends_music_loads_through_the_engines_own_path() {
    let Some(source) = source() else { return };
    let cache = std::env::temp_dir().join("oag-2048-music-ground-truth");
    let (name, sound) = music::load_front_end(&source.display().to_string(), &cache)
        .expect("loading the front end's music")
        .expect("2048 names one");

    assert_eq!(name, oag_2048::names::FRONT_END_MUSIC);
    assert_eq!(sound.channels(), 2);
    assert_eq!(sound.sample_rate(), 48_000);
    assert!(
        (sound.seconds() - 302.0).abs() < 1.0,
        "ffprobe reads 302.000000 s off the file directly; got {} s",
        sound.seconds()
    );

    // Rendered through a real `Mixer`, the same way `Audio::start_music`
    // plays it - not just decoded PCM, so a silent-but-parses regression in
    // `oag_game::at9`'s cache path would still be caught here.
    let mut mixer = oag_audio::Mixer::new(sound.sample_rate());
    let played = mixer.play(oag_audio::Play::looping(
        std::sync::Arc::new(sound),
        oag_audio::Bus::Music,
    ));
    assert!(played.is_some(), "the mixer refused to play a valid sound");

    // Two seconds, interleaved stereo - enough to prove the mixer is
    // producing real signal without rendering the whole 302-second loop.
    let mut out = vec![0.0f32; mixer.sample_rate() as usize * 2 * 2];
    mixer.render(&mut out);

    let peak = out.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    let rms = (out.iter().map(|s| s * s).sum::<f32>() / out.len() as f32).sqrt();
    assert!(
        peak > 0.01,
        "rendered {} samples with peak {peak}, rms {rms} - this is silence, \
         not the front end's music",
        out.len()
    );

    let wav = oag_audio::wav::from_samples(&out, mixer.sample_rate());
    let dest = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/cache/2048-frontend-music-ground-truth.wav");
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).ok();
    }
    std::fs::write(&dest, &wav).expect("writing the WAV");
    println!(
        "wrote {} ({} frames, peak {peak:.4}, rms {rms:.4})",
        dest.display(),
        out.len() / 2
    );
}

/// The evidence for why `front_end` names a standalone file rather than a
/// `frontend.bnk` cue: the bank this thread's own Next Step originally
/// pointed at declares no name table at all, so nothing in it can be
/// addressed by name the way a cue-based track is.
#[test]
#[ignore = "needs game content in data/extracted/vita/PCSF00007"]
fn the_banks_cue_table_is_empty() {
    let Some(source) = source() else { return };
    let mut archive = oag_assets::psarc::Archive::open(
        &source.join("base/PSP2/data.psarc").display().to_string(),
    )
    .expect("open data.psarc");
    let blob = archive
        .read_path("data/audio/sound/frontend.bnk")
        .expect("read frontend.bnk");
    let bank = oag_formats::sblk::Bank::parse(&blob).expect("parse frontend.bnk");
    assert!(
        bank.sound_names().is_empty(),
        "frontend.bnk grew a name table - front_end can address a cue by \
         name after all, and this test (plus this file's own module doc) is \
         the place to update"
    );
}

/// `MusicDiscs::survey` recognises the Vita release and offers its music,
/// the same as it always has for the PSP/PS2/PS3 releases - pinned so a
/// future refactor of that match cannot silently drop `Platform::Vita`
/// again the way it was missing when `oag_2048::TITLE.music` first went
/// `Some`.
#[test]
#[ignore = "needs game content in data/extracted/vita/PCSF00007"]
fn the_boots_own_music_discs_survey_finds_it() {
    let Some(source) = source() else { return };
    let discs = audio::MusicDiscs::survey(&source.display().to_string());
    assert_eq!(discs.booted(), Some(Platform::Vita));
    assert!(
        discs.describe().starts_with("Vita "),
        "got {:?}",
        discs.describe()
    );
}
