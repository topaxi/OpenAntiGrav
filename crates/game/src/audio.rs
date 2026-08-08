//! Sound, from the composition root outwards.
//!
//! `oag-audio` owns the mixer and the device; this module owns the *policy*
//! around them - which track plays, where its bytes come from, what the volume
//! setting means, and how a headless run is turned into a file somebody can
//! listen to. It lives here for the same reason [`crate::source`] does: it
//! reads the disc and the settings file, neither of which a library crate below
//! is allowed to know about.
//!
//! # Where a tick ends and a frame begins
//!
//! Everything this module does is driven by the **tick count** and nothing
//! else, so it goes inside the fixed-timestep loop, next to the exhaust and the
//! chase camera (`crate::race::Race::tick`). That is what makes a headless
//! `--dump-audio` capture produce the same samples at the same tick count as a
//! window does, and it is the same argument
//! `docs/architecture/determinism.md` makes for putting audio outside the
//! simulation in the first place: the mixer may allocate and lock freely, but
//! it must never be advanced by a wall clock.
//!
//! There is deliberately **no** per-frame call to make. With a real device
//! `cpal` drains the mixer from its own callback thread, so servicing it from
//! the frame loop would be a second reader racing the first; with no device
//! [`oag_audio::Output::render_tick`] is the only reader, and it is tick-driven
//! by construction. See [`Audio::tick`].

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use oag_audio::{Bus, Output, Play, Sound, VoiceId};
use oag_disc::DiscImage;
use oag_formats::ps2_music;
use serde::{Deserialize, Serialize};

use crate::display::percentage;

/// A bus volume, as a percentage of unattenuated.
///
/// Zero is off and 100 is the samples as they were stored. Unlike
/// [`crate::display::Brightness`] there is no floor: a player who wants no
/// music should be able to say so, and unlike a black screen a silent one is
/// not a state they cannot navigate back out of.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "u32", into = "u32")]
pub struct Volume(u32);

impl Volume {
    /// The samples unattenuated.
    pub const FULL: Self = Self(100);

    /// The narrowest and widest this may be.
    pub const RANGE: std::ops::RangeInclusive<u32> = 0..=100;

    /// The percentages the menus offer.
    pub const OFFERED: [Self; 6] = [Self(0), Self(25), Self(50), Self(75), Self(90), Self(100)];

    /// The linear gain this percentage means.
    #[must_use]
    pub fn gain(self) -> f32 {
        self.0 as f32 / 100.0
    }

    /// The percentage itself.
    #[must_use]
    pub fn percent(self) -> u32 {
        self.0
    }
}

percentage!(Volume, FULL, "volume");

/// The rate `--dump-audio` renders at.
///
/// The PS2 music archive's own rate, and chosen for a reason that is not
/// convenience: at 48 kHz in and 48 kHz out the mixer's resampling step is
/// exactly 1.0, so an unattenuated voice round-trips the disc's PCM
/// sample-for-sample through `f32` and back. That turns the dump from "a
/// waveform that is not silent" - which a one-byte misalignment would also
/// produce - into a file that can be compared against the archive directly.
/// See `docs/formats/ps2-audio.md`.
pub const DUMP_SAMPLE_RATE: u32 = ps2_music::SAMPLE_RATE;

/// Where the PS2 release keeps its music, loose in the filesystem rather than
/// inside an archive.
///
/// `54748` is the disc's serial directory. See
/// `docs/ps2/pulse-disc-layout.md`.
const PS2_MUSIC_PATH: &str = "54748/PS2MUSIC.WAD";

/// Which track is played, by index into the archive's directory.
///
/// The first, because nothing yet maps a circuit or a menu to a track: the
/// entries are addressed by name hash and no name for any of them has been
/// recovered. Playing a fixed one proves the path end to end without claiming
/// a mapping that has not been established.
const MUSIC_TRACK: usize = 0;

/// The PSP front end's music, in `Data.wad`.
///
/// **Named, not guessed.** The executable builds this path at run time from the
/// template at `0x08a88e94`, `Data\Music\FEMusic\frontend%d.at3` - see
/// `docs/formats/vex.md` - and hashing the expansion finds an entry for every
/// `%d` from 1 to 8 and none for 0, so the numbering starts at one. All eight
/// are stereo ATRAC3+ at 44,100 Hz and about 28 seconds long.
///
/// Which of the eight belongs to which menu is **not** established, so the
/// first is played, for the same reason [`MUSIC_TRACK`] is zero: it proves the
/// path end to end without claiming a mapping nobody has recovered.
const PSP_MUSIC_NAME: &str = r"Data\Music\FEMusic\frontend1.at3";

/// The mixer, the device behind it, and what the composition root plays.
///
/// One of these exists per run, made in `main` before either way in - the boot
/// sequence or `--race` - because both want sound and neither owns the other.
pub struct Audio {
    output: Output,
    /// Where `--dump-audio` writes, and what has been rendered so far.
    ///
    /// `None` is the ordinary case: a run with a device attached has nothing to
    /// dump, because [`Output::render_tick`] refuses to pull samples the
    /// hardware callback is already draining.
    dump: Option<Dump>,
    /// The looping music voice, so a later volume change or stop can address
    /// it. `None` when this source carries no decodable music.
    music: Option<VoiceId>,
    /// The movie's own sound, while a movie is playing one.
    ///
    /// Separate from [`Self::music`] although both are on [`Bus::Music`],
    /// because they are stopped at different moments and by different things:
    /// the music runs for the whole session and this lasts one movie. See
    /// [`Audio::start_movie`].
    movie: Option<VoiceId>,
}

/// The offline capture: a path and the samples destined for it.
struct Dump {
    path: PathBuf,
    samples: Vec<f32>,
}

// Written out rather than derived: the sample buffer is minutes of audio, and a
// `{:?}` of an `Audio` should say how much there is rather than print it.
impl std::fmt::Debug for Dump {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Dump")
            .field("path", &self.path)
            .field("samples", &self.samples.len())
            .finish()
    }
}

impl std::fmt::Debug for Audio {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Audio")
            .field("output", &self.output)
            .field("dump", &self.dump)
            .field("music", &self.music)
            .field("movie", &self.movie)
            .finish()
    }
}

impl Audio {
    /// Opens the output and applies the persisted volumes.
    ///
    /// `dump` names the WAV `--dump-audio` should write, and when it is set the
    /// device is **not** opened at all. That is not a convenience: with a
    /// stream attached the `cpal` callback drains the mixer from its own
    /// thread, [`Output::render_tick`] returns zero rather than racing it, and
    /// the dump would be a file of silence written next to audio the player
    /// could hear. Forcing the null backend makes the two mutually exclusive
    /// where they would otherwise be quietly wrong.
    #[must_use]
    pub fn open(settings: &crate::settings::Audio, dump: Option<PathBuf>) -> Self {
        let output = match &dump {
            Some(_) => Output::null(DUMP_SAMPLE_RATE),
            None => Output::open_or_null(),
        };
        if let Some(name) = output.device_name() {
            println!("audio: {name} at {} Hz", output.sample_rate());
        }
        let audio = Self {
            output,
            dump: dump.map(|path| Dump {
                path,
                samples: Vec::new(),
            }),
            music: None,
            movie: None,
        };
        audio.apply(settings);
        audio
    }

    /// Applies every persisted volume to the buses.
    ///
    /// Separate from [`Self::open`] because the menus change these while the
    /// game runs and the change should be audible on the row the player is
    /// standing on, the way `BRIGHTNESS` is visible on it.
    pub fn apply(&self, settings: &crate::settings::Audio) {
        self.output
            .with_mixer(|mixer| mixer.set_bus_gain(Bus::Music, settings.music_volume.gain()));
    }

    /// The output, and through it the mixer.
    ///
    /// The seam a cue goes through when there is one to emit: a caller inside
    /// the tick loop reaches `with_mixer` from here. Nothing does yet, because
    /// no sound but the music decodes, and it is exposed rather than left
    /// private so that the first one does not have to reopen this module.
    #[must_use]
    pub fn output(&self) -> &Output {
        &self.output
    }

    /// Loads one music track off `source` and starts it looping.
    ///
    /// The two releases keep their music in unrelated places and unrelated
    /// formats, so both are tried in turn: the PS2's `PS2MUSIC.WAD` first,
    /// because it is loose on the disc and needs nothing but a ranged read, and
    /// then the PSP's `Data.wad`, whose ATRAC3+ has to go out to `ffmpeg` and
    /// through the cache in [`crate::at3`]. A source answers exactly one of
    /// them, so the order is only about doing the cheap check first.
    ///
    /// `cache_dir` is where a decoded PSP track lands - see
    /// [`crate::boot::default_audio_cache_dir`]. It is unused by the PS2 path,
    /// which needs no decoder.
    ///
    /// **Never fatal.** A source with no decodable music says so on stdout and
    /// plays nothing, which is the same degradation the video path takes when
    /// `ffmpeg` is missing: name what is absent and carry on. A machine with no
    /// `ffmpeg` on `PATH` is that case for a PSP disc, and the message names
    /// the tool.
    pub fn start_music(&mut self, source: &str, cache_dir: &Path) {
        if self.music.is_some() {
            return;
        }
        let loaded = load_ps2_track(source, MUSIC_TRACK)
            .map(|found| {
                found.map(|sound| (sound, format!("{PS2_MUSIC_PATH} track {MUSIC_TRACK}")))
            })
            .and_then(|found| match found {
                Some(found) => Ok(Some(found)),
                None => load_psp_track(source, cache_dir)
                    .map(|found| found.map(|sound| (sound, PSP_MUSIC_NAME.to_string()))),
            });

        match loaded {
            Ok(Some((sound, what))) => {
                let seconds = sound.seconds();
                self.music = self
                    .output
                    .with_mixer(|mixer| mixer.play(Play::looping(Arc::new(sound), Bus::Music)));
                println!("audio: music {what}, {seconds:.1} s, looping");
            }
            Ok(None) => println!(
                "audio: this source carries neither {PS2_MUSIC_PATH} nor {PSP_MUSIC_NAME}, so no \
                 music"
            ),
            Err(error) => println!("audio: no music ({error:#})"),
        }
    }

    /// Starts the boot sequence's movie sound, reporting what happened.
    ///
    /// The seam both tick loops go through - the window's and the headless
    /// capture's - so that the line on stdout, and the decision itself, cannot
    /// come out differently on one of them. `None` is the ordinary silent case
    /// and says so once; see [`crate::boot::Boot::movie_sound`] for the reasons
    /// it is `None`.
    pub fn start_boot_movie(&mut self, sound: Option<crate::at3::Pcm>) {
        let Some(pcm) = sound else {
            println!("audio: the intro movie plays silently");
            return;
        };
        let seconds = pcm.samples.len() as f64
            / f64::from(pcm.channels.max(1))
            / f64::from(pcm.sample_rate.max(1));
        match Sound::new(pcm.samples, pcm.channels, pcm.sample_rate) {
            Ok(sound) => {
                if self.start_movie(sound) {
                    println!(
                        "audio: the intro movie's own track, {seconds:.2} s, clocking the picture"
                    );
                } else {
                    // A mixer with every slot busy, which cannot happen today -
                    // the music is the only other voice - but is reported rather
                    // than leaving a movie silent for no stated reason.
                    println!("audio: no free voice for the intro movie, so it plays silently");
                }
            }
            Err(error) => println!("audio: the intro movie plays silently ({error:#})"),
        }
    }

    /// Starts a movie's own sound, replacing whatever was playing before.
    ///
    /// One shot rather than looping: a movie ends. It goes on [`Bus::Music`] so
    /// that a player who has turned the music down has turned this down too -
    /// there is no separate movie bus on the original either, and a movie's
    /// track *is* its music.
    ///
    /// Returns whether a voice was actually started, which is what
    /// [`Self::movie_playhead`] then paces the picture against. `false` means
    /// every voice slot was busy, and a movie that plays silently is a better
    /// outcome than one that will not play.
    pub fn start_movie(&mut self, sound: Sound) -> bool {
        self.stop_movie();
        self.movie = self
            .output
            .with_mixer(|mixer| mixer.play(Play::once(Arc::new(sound), Bus::Music)));
        self.movie.is_some()
    }

    /// Stops the movie's sound, if any is playing.
    ///
    /// Called when the movie ends **or is skipped**, and the skip is the reason
    /// it is a separate call rather than something the voice's own end handles:
    /// the intro can be cut short by START at any point, and 30 seconds of
    /// leftover intro playing under the menus would be the most obvious
    /// possible bug.
    pub fn stop_movie(&mut self) {
        if let Some(id) = self.movie.take() {
            self.output.with_mixer(|mixer| mixer.stop(id));
        }
    }

    /// How far into its own sound the playing movie has got, in seconds.
    ///
    /// **This is the whole of ADR-0019's clock rule**, in one place so that no
    /// two callers can answer it differently: `Some` means pace the picture
    /// against this, `None` means pace it against the tick. Every reason a
    /// movie has no playhead collapses into `None` here - no audio stream, a
    /// widget that says `sound="false"`, no `ffmpeg` to decode with, a voice
    /// that has ended, and the case below.
    ///
    /// # Why "is a voice playing" is not enough on its own
    ///
    /// [`Self::tick`] only pulls samples when there is a dump to write, because
    /// with a device attached `cpal`'s callback is already draining the mixer
    /// and pulling here would take samples out of its mouth. So a run with
    /// **neither** a device nor a dump - a null output, which is what a headless
    /// run without `--dump-audio` gets, and what CI has - advances the mixer
    /// never. A voice on it sits at zero for ever.
    ///
    /// Pacing a movie against that clock would stop the picture on frame one
    /// and the intro would never reach its end, so `--until` would run to its
    /// tick ceiling and fail. The mixer has to be **moving** as well as
    /// sounding, and only this module knows whether it is.
    #[must_use]
    pub fn movie_playhead(&self) -> Option<f64> {
        if !(self.dump.is_some() || self.output.is_streaming()) {
            return None;
        }
        self.output.with_mixer(|mixer| mixer.position(self.movie?))
    }

    /// Advances the mixer by one simulation tick.
    ///
    /// **Called from inside the fixed-timestep loop**, never once per frame.
    /// The frame count is whatever the machine happens to manage; the tick
    /// count is not, and a capture that renders `ticks * sample_rate / 60`
    /// frames is one a second run reproduces exactly. With a device attached
    /// this does nothing at all - `cpal`'s callback is already draining the
    /// mixer at the hardware's own pace, and pulling here would take samples
    /// out of its mouth.
    pub fn tick(&mut self) {
        if let Some(dump) = &mut self.dump {
            self.output.render_tick(TICK_HZ, &mut dump.samples);
        }
    }

    /// Writes the dump, if there is one.
    ///
    /// # Errors
    ///
    /// Propagates a file that cannot be written.
    pub fn finish(&self) -> Result<()> {
        let Some(dump) = &self.dump else {
            return Ok(());
        };
        let rate = self.output.sample_rate();
        let file = oag_audio::wav::from_samples(&dump.samples, rate);
        std::fs::write(&dump.path, file)
            .with_context(|| format!("writing {}", dump.path.display()))?;
        let frames = dump.samples.len() / 2;
        println!(
            "wrote {} ({frames} frames, {:.2} s at {rate} Hz)",
            dump.path.display(),
            frames as f32 / rate as f32
        );
        Ok(())
    }
}

/// The rate every loop in this crate steps the simulation at, per ADR-0007.
///
/// Named here rather than taken from the caller so a dump's length is a
/// function of the tick count alone: `oag_core::TickRate` is what the loop uses
/// and it is fixed at 60.
const TICK_HZ: u32 = 60;

/// Reads one entry of `PS2MUSIC.WAD` off `source` and turns it into a sound.
///
/// `Ok(None)` when the source carries no such file, which is an ordinary
/// outcome rather than an error - a PSP disc has none, and neither does a
/// partially extracted directory.
///
/// # The whole archive is never read
///
/// It is 585 MiB on the EU disc and one track is about 35 of them, so this
/// reads the 4-byte header, then the directory those 4 bytes size, then exactly
/// the entry asked for. [`oag_assets::pulse::read_loose_file`] would have been
/// the obvious call and reads a loose file whole; that is right for a movie
/// container and wrong by a factor of sixteen here.
fn load_ps2_track(source: &str, index: usize) -> Result<Option<Sound>> {
    let Some(mut archive) = MusicArchive::open(source)? else {
        return Ok(None);
    };

    let header = archive.read(0, ps2_music::HEADER_LEN as u64)?;
    let count = ps2_music::peek_entry_count(&header)
        .map_err(|e| anyhow::anyhow!("{PS2_MUSIC_PATH} is not a music archive: {e}"))?;
    let table = archive.read(0, ps2_music::directory_len(count))?;
    let directory = ps2_music::Directory::parse(&table, Some(archive.len()))
        .map_err(|e| anyhow::anyhow!("reading the {PS2_MUSIC_PATH} directory: {e}"))?;

    let entry = directory.entries.get(index).with_context(|| {
        format!(
            "{PS2_MUSIC_PATH} has {} track(s), so there is no track {index}",
            directory.entries.len()
        )
    })?;

    let pcm = archive.read(u64::from(entry.offset), u64::from(entry.size))?;
    // Signed 16-bit little-endian, two channels interleaved left first - none of
    // which the file states; see `docs/formats/ps2-audio.md` for how each was
    // established. `chunks_exact` drops a trailing odd byte, which
    // `Directory::parse` has already rejected as a partial frame.
    let samples: Vec<i16> = pcm
        .chunks_exact(2)
        .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    let sound = Sound::new(samples, ps2_music::CHANNELS, ps2_music::SAMPLE_RATE)
        .with_context(|| format!("track {index} of {PS2_MUSIC_PATH}"))?;
    Ok(Some(sound))
}

/// Reads the PSP front end's music out of `Data.wad` and decodes it.
///
/// `Ok(None)` when the source holds no such entry, which is every PS2 source
/// and any partially extracted directory - an ordinary outcome, not an error,
/// the same way [`load_ps2_track`] treats a missing archive.
///
/// The whole entry is read rather than ranged, unlike the PS2 path: 349 KiB of
/// ATRAC3+ is a fifth of a second's work and the whole thing has to go to
/// `ffmpeg` anyway. It is the *decoded* form that is large - 5 MiB of PCM for
/// 30 seconds - and that is what the cache exists to avoid paying twice.
///
/// # Errors
///
/// A source that will not open at all, an entry that is not a readable RIFF,
/// or a decode that failed - including `ffmpeg` being absent, which the caller
/// reports rather than treating as fatal.
fn load_psp_track(source: &str, cache_dir: &Path) -> Result<Option<Sound>> {
    // Resolved through the layout rather than a literal `PSP_GAME/USRDIR/...`
    // path, so a directory somebody extracted with `oag-unpack` answers the
    // same as a disc image does.
    let Ok(mut archives) = oag_assets::pulse::Archives::open(source) else {
        return Ok(None);
    };
    if archives.locate(PSP_MUSIC_NAME).is_none() {
        return Ok(None);
    }

    let at3 = archives
        .read_name(PSP_MUSIC_NAME)
        .with_context(|| format!("reading {PSP_MUSIC_NAME}"))?;
    let pcm = crate::at3::decode(&at3, cache_dir)
        .with_context(|| format!("decoding {PSP_MUSIC_NAME}"))?;
    let sound = Sound::new(pcm.samples, pcm.channels, pcm.sample_rate).context(PSP_MUSIC_NAME)?;
    Ok(Some(sound))
}

/// A seekable handle on `PS2MUSIC.WAD`, wherever it lives.
///
/// The same two cases every other reader in this crate handles: a disc image,
/// or a directory previously extracted with `oag-unpack`. Both read lazily,
/// which is the entire point - see [`load_ps2_track`].
#[derive(Debug)]
enum MusicArchive {
    File {
        file: std::fs::File,
        len: u64,
    },
    Disc {
        disc: Box<DiscImage>,
        entry: oag_disc::Entry,
    },
}

impl MusicArchive {
    fn open(source: &str) -> Result<Option<Self>> {
        let path = Path::new(source);
        if path.is_dir() {
            let Some(found) = find_in_dir(path) else {
                return Ok(None);
            };
            let file = std::fs::File::open(&found)
                .with_context(|| format!("opening {}", found.display()))?;
            let len = file
                .metadata()
                .with_context(|| format!("sizing {}", found.display()))?
                .len();
            return Ok(Some(Self::File { file, len }));
        }

        // Not a directory and not a disc this reader understands - an
        // `image:path` archive spec, say. Not an error: the caller's answer to
        // "is there music here" is no, and it already prints that.
        let Ok(mut disc) = DiscImage::open(source) else {
            return Ok(None);
        };
        let found = disc
            .entries()
            .with_context(|| format!("walking {source}"))?
            .iter()
            .find(|entry| !entry.is_directory && entry.path.eq_ignore_ascii_case(PS2_MUSIC_PATH))
            .cloned();
        Ok(found.map(|entry| Self::Disc {
            disc: Box::new(disc),
            entry,
        }))
    }

    fn len(&self) -> u64 {
        match self {
            Self::File { len, .. } => *len,
            Self::Disc { entry, .. } => entry.size,
        }
    }

    fn read(&mut self, offset: u64, len: u64) -> Result<Vec<u8>> {
        match self {
            Self::File { file, .. } => {
                use std::io::{Read, Seek, SeekFrom};
                file.seek(SeekFrom::Start(offset))
                    .context("seeking the music archive")?;
                let mut buffer = vec![0u8; usize::try_from(len).unwrap_or(usize::MAX)];
                file.read_exact(&mut buffer)
                    .context("reading the music archive")?;
                Ok(buffer)
            }
            Self::Disc { disc, entry } => disc
                .read_entry_range(entry, offset, len)
                .with_context(|| format!("reading {PS2_MUSIC_PATH} at {offset}")),
        }
    }
}

/// Finds `PS2MUSIC.WAD` under an extracted directory, case-insensitively.
///
/// Matched on the file name rather than the full path: a directory somebody
/// extracted with `-o` somewhere has the serial folder under whatever they
/// chose, and there is exactly one file with this name on the disc.
fn find_in_dir(root: &Path) -> Option<PathBuf> {
    let wanted = Path::new(PS2_MUSIC_PATH).file_name()?;
    let mut stack = vec![root.to_path_buf()];
    while let Some(directory) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else if path
                .file_name()
                .is_some_and(|name| name.eq_ignore_ascii_case(wanted))
            {
                return Some(path);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_volume_round_trips_through_its_own_text() {
        for value in Volume::OFFERED {
            assert_eq!(value.to_string().parse::<Volume>(), Ok(value));
        }
    }

    #[test]
    fn a_volume_outside_the_range_is_refused() {
        assert!("101".parse::<Volume>().is_err());
        assert!("-1".parse::<Volume>().is_err());
        assert_eq!("0".parse::<Volume>(), Ok(Volume(0)));
    }

    #[test]
    fn full_volume_is_unattenuated() {
        assert_eq!(Volume::FULL.gain(), 1.0);
        assert_eq!(Volume::default(), Volume::FULL);
        assert_eq!(Volume(0).gain(), 0.0);
    }

    /// The dump's length has to be a function of the tick count and nothing
    /// else, because that is the whole claim `--dump-audio` makes: the same
    /// control sequence renders the same file. A wall clock anywhere in the
    /// path would show up here as a count that moves between runs.
    #[test]
    fn a_dump_is_exactly_as_long_as_the_ticks_it_was_given() {
        let mut audio = Audio {
            output: Output::null(DUMP_SAMPLE_RATE),
            dump: Some(Dump {
                path: PathBuf::from("unused"),
                samples: Vec::new(),
            }),
            music: None,
            movie: None,
        };
        for _ in 0..120 {
            audio.tick();
        }
        let dump = audio.dump.as_ref().expect("the dump is set");
        let frames = dump.samples.len() / 2;
        assert_eq!(frames, 120 * (DUMP_SAMPLE_RATE as usize / 60));
    }

    /// **The A/V sync measurement**, and the only one that can be made without
    /// something to listen with: over a full 40-second reel, does the frame the
    /// picture is on stay within one frame of where the sound has got to?
    ///
    /// Run through the real pieces rather than a model of them - a real
    /// [`Sound`] in a real mixer, pulled by [`Audio::tick`] at the fixed 60 Hz,
    /// with the playhead read exactly where both tick loops read it and handed
    /// to [`crate::movie::Player::follow`]. Drift is what audio clocking exists
    /// to prevent and it is cumulative, so measuring it over one tick would
    /// measure nothing; 2,402 ticks is the whole intro.
    ///
    /// The bound is **one frame**, which is 33 ms of picture against 44,100
    /// samples a second of sound. The error is a floor, so the frame is at
    /// worst the one before the sound's own, never the one after.
    #[test]
    fn the_picture_stays_within_a_frame_of_the_sound_for_a_whole_reel() {
        let seconds = 40.17;
        let rate = 44_100;
        let frames = (seconds * f64::from(rate)) as usize;
        // Silence is fine: what is measured is where the playhead is, and the
        // mixer advances it whatever the samples are.
        let sound = Sound::new(vec![0i16; frames * 2], 2, rate).expect("a sound");

        let mut audio = Audio {
            output: Output::null(DUMP_SAMPLE_RATE),
            dump: Some(Dump {
                path: PathBuf::from("unused"),
                samples: Vec::new(),
            }),
            music: None,
            movie: None,
        };
        assert!(audio.start_movie(sound), "a free voice");

        let (num, den) = crate::movie::FRAME_RATE;
        let mut player = crate::movie::Player::new(1200, false, crate::movie::FRAME_RATE);
        let mut worst = 0.0f64;

        for tick in 0..(60 * 41) {
            let playhead = audio.movie_playhead().expect("a sounding voice");
            player.follow(playhead);
            audio.tick();

            if player.is_finished() {
                break;
            }
            // Where the sound says the picture should be, unrounded.
            let wanted = playhead * num as f64 / den as f64;
            let error = wanted - player.frame() as f64;
            assert!(
                (0.0..1.0).contains(&error),
                "tick {tick}: the picture is {error} frames from the sound"
            );
            worst = worst.max(error);
        }

        assert!(worst > 0.0, "the reel should actually have played");
    }

    /// The clock rule's own failure mode, and the reason the predicate is not
    /// just "is a voice playing": a run with **neither** a device nor a dump
    /// never advances the mixer, so a movie paced against it would stop on
    /// frame one and the boot sequence would never reach its end.
    #[test]
    fn a_mixer_that_is_never_advanced_offers_no_clock() {
        let mut audio = Audio {
            output: Output::null(DUMP_SAMPLE_RATE),
            dump: None,
            music: None,
            movie: None,
        };
        let sound = Sound::new(vec![0i16; 44_100 * 2], 2, 44_100).expect("a sound");
        assert!(audio.start_movie(sound), "a free voice");

        assert_eq!(
            audio.movie_playhead(),
            None,
            "a voice on a mixer nothing pulls from is not a clock"
        );
    }

    /// A movie with no sound is tick-clocked, which is `Backdrop.PMF` - the
    /// movie that plays most, and the one this must not get wrong.
    #[test]
    fn a_movie_with_no_voice_has_no_playhead() {
        let mut audio = Audio {
            output: Output::null(DUMP_SAMPLE_RATE),
            dump: Some(Dump {
                path: PathBuf::from("unused"),
                samples: Vec::new(),
            }),
            music: None,
            movie: None,
        };
        assert_eq!(audio.movie_playhead(), None, "nothing started");

        let sound = Sound::new(vec![0i16; 4], 2, 44_100).expect("a sound");
        assert!(audio.start_movie(sound));
        assert_eq!(audio.movie_playhead(), Some(0.0));

        audio.stop_movie();
        assert_eq!(audio.movie_playhead(), None, "and none once it is stopped");
    }

    /// With no dump asked for, nothing is accumulated at all - a windowed run
    /// must not grow a buffer nobody ever reads.
    #[test]
    fn a_run_with_no_dump_accumulates_nothing() {
        let mut audio = Audio {
            output: Output::null(DUMP_SAMPLE_RATE),
            dump: None,
            music: None,
            movie: None,
        };
        for _ in 0..120 {
            audio.tick();
        }
        assert!(audio.dump.is_none());
    }
}
