//! Sound: the music and effects the game plays.
//!
//! `oag-audio` owns the mixer and the device; this crate owns the policy: which
//! track plays, where its bytes come from, what the volume setting means, and
//! how a race's cues become voices.
//!
//! This crate depends on no title package and not on `oag-game`, so three
//! things come in from outside:
//!
//! - **Opening a source as a title, and finding the other release's disc.**
//!   [`library::Library`] is the trait; [`MusicDiscs::survey`] keeps one for
//!   later fetches, so a worker thread can open sources without owning `Audio`.
//!   `oag_game::sound::GameLibrary` implements it.
//! - **Reading a race.** [`Audio::race_tick`] takes a plain-data
//!   [`sfx::RaceFrame`]; the cue queue's producers stay in `oag-game` and push
//!   [`sfx::CueEvent`]s.
//! - **Reporting what a loader did.** [`sfx::Banks`] and [`sfx::TrackEmitters`]
//!   carry their `report` for the host to send to `oag_raceplay::loader_log`.
//!
//! [`settings::Settings`] is the `[audio]` section of the settings file.
//!
//! Everything here is driven by the **tick count** only, inside the
//! fixed-timestep loop (`oag_raceplay::Race::tick`), so a headless
//! `--dump-audio` capture matches a window's samples at the same tick count
//! (`docs/architecture/determinism.md`). There is deliberately **no** per-frame
//! call: with a device `cpal` drains the mixer from its own thread, and with
//! none [`oag_audio::Output::render_tick`] is the only reader. See
//! [`Audio::tick`].

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::{Context, Result};
use log::{debug, info, warn};
use oag_audio::mixer::MUSIC_MASTER_TRIM;
use oag_audio::{Bus, Output, Play, Sound, VoiceId};
use oag_disc::{DiscImage, Platform};
use oag_formats::ps2_music;
use serde::{Deserialize, Serialize};

use oag_display::percentage;

pub mod hd_mix;
pub mod library;
pub mod settings;
pub mod sfx;

mod race_music;
use library::{Library, NoLibrary, listing, load_entry, load_front_end};
pub use race_music::MusicFetchWorker;
use race_music::{PendingSwitch, locate};

/// A bus volume, as a percentage of unattenuated.
///
/// Zero is off, 100 is the samples as stored. No floor, unlike
/// [`oag_display::display::Brightness`]: a silent screen is navigable, a black
/// one is not.
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
/// The PS2 music archive's own rate: at 48 kHz in and out the resampling step
/// is exactly 1.0, so an unattenuated voice round-trips the disc's PCM
/// sample-for-sample and the dump can be compared against the archive. See
/// `docs/formats/ps2-audio.md`.
pub const DUMP_SAMPLE_RATE: u32 = ps2_music::SAMPLE_RATE;

/// Where the PS2 release keeps its music, loose in the filesystem.
///
/// `54748` is the disc's serial directory (`docs/ps2/pulse-disc-layout.md`).
const PS2_MUSIC_PATH: &str = "54748/PS2MUSIC.WAD";

/// Which track the menu plays, by index into the booted release's soundtrack.
///
/// The first: each disc's plugin definition declares one `PI_Music` node per
/// track ([`oag_title::DeclaredTracks`]), but which one a menu or circuit
/// plays is unestablished. [`Audio::race_index`] addresses the same order for
/// the race playlist, starting one past this one ([`Audio::initial_race_index`]).
const MUSIC_TRACK: usize = 0;

/// Which release's encode of the soundtrack is played.
///
/// **Music, not audio.** Only the sixteen soundtrack tracks have a counterpart
/// on the other disc (a 16-for-16 bijection, mean duration gap 11 ms,
/// `docs/formats/ps2-audio.md`). Voice and sound banks stay on the booted disc.
///
/// **Does not govern the front end's own music.** The PSP's
/// `Data\Music\FEMusic\frontend1.at3` (a 28-second loop) matches no PS2 entry,
/// so a PSP boot plays it at every value. The row promises its values are one
/// recording encoded twice, which licenses [`Audio::set_music_source`] to seek
/// rather than restart; offering it for unrelated music would break that. The
/// PS2 menus play soundtrack track 0, so the row is effective on PS2 and inert
/// on PSP, which is the discs' asymmetry. Enforced by [`Audio::music_from`]
/// being `None` for anything not loaded as one of the sixteen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MusicSource {
    /// The disc the game booted from.
    ///
    /// The default, deliberately not "whichever is better": the PSP release
    /// should sound like the PSP release, ATRAC3+ at 96 kbit/s included.
    #[default]
    Auto,
    /// The PSP release's ATRAC3+ encode, 44,100 Hz and lossy.
    Psp,
    /// The PS2 release's uncompressed PCM, 48,000 Hz.
    Ps2,
}

impl MusicSource {
    /// The spelling used in a settings file and on a menu row.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Psp => "psp",
            Self::Ps2 => "ps2",
        }
    }

    /// Every value, for the menus and for error messages.
    pub const ALL: [Self; 3] = [Self::Auto, Self::Psp, Self::Ps2];

    /// The release this names outright, or `None` for [`Self::Auto`], which
    /// names whichever disc booted.
    #[must_use]
    pub fn platform(self) -> Option<Platform> {
        match self {
            Self::Auto => None,
            Self::Psp => Some(Platform::Psp),
            Self::Ps2 => Some(Platform::Ps2),
        }
    }
}

impl std::str::FromStr for MusicSource {
    type Err = String;

    fn from_str(text: &str) -> std::result::Result<Self, Self::Err> {
        Self::ALL
            .into_iter()
            .find(|source| source.name().eq_ignore_ascii_case(text))
            .ok_or_else(|| format!("{text:?} is not a music source; try auto, psp or ps2"))
    }
}

impl std::fmt::Display for MusicSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.name())
    }
}

/// Which Pulse releases this machine has, and which one the game booted from.
///
/// Surveyed once, at boot: the answer decides whether the MUSIC SOURCE row is
/// offered, and re-deriving it on a keypress would open every disc image on
/// the search path.
///
/// # The counterpart is confirmed by its soundtrack, not by its serial
///
/// A file name cannot cover every regional pressing (`oag_source::source::IMAGE_NAMES`
/// names only the copies this project has), and `oag_assets::Layout::resolve`'s
/// deny-list is positive-only: it rules out the one verified Pure serial
/// (`UCUS-98612`) and gives an unlisted serial no verdict. Measured, a
/// `data/images/` holding `pure-psp-eu.chd` (`UCES-00001`, not on that list)
/// reported it as the PSP counterpart.
///
/// That polarity suits a disc the player *named*; here a false positive
/// silently plays another game's music while a false negative only hides the
/// row. So a candidate has to **pair** through [`Soundtrack::pairs_with`]:
/// sixteen tracks against sixteen, each within [`PAIR_TOLERANCE`] of a distinct
/// partner (the measurement in `docs/formats/ps2-audio.md`, run against the
/// disc in hand).
#[derive(Debug, Clone)]
pub struct MusicDiscs {
    psp: Option<String>,
    ps2: Option<String>,
    /// The booted title's PS3 source, if one was found.
    ///
    /// **Never paired against anything**: Wipeout HD has one release, so this is
    /// only the booted disc and [`MusicDiscs::both`] is false whenever it is set.
    ps3: Option<String>,
    /// The booted title's Vita source, if one was found.
    ///
    /// Same terms as [`Self::ps3`]: 2048 has one release. Without it,
    /// `oag_2048::TITLE.music` going `Some` (2026-09-25) could not reach the
    /// menus, because [`MusicDiscs::survey`] folded `Platform::Vita` into the
    /// early return `Platform::Unknown` takes.
    vita: Option<String>,
    /// The booted title's PS4 source, if one was found. Omega has one release.
    ps4: Option<String>,
    booted: Option<Platform>,
    /// How a source string becomes an opened title. See [`Library`].
    library: &'static dyn Library,
}

impl Default for MusicDiscs {
    fn default() -> Self {
        Self {
            psp: None,
            ps2: None,
            ps3: None,
            vita: None,
            ps4: None,
            booted: None,
            library: &NoLibrary,
        }
    }
}

impl MusicDiscs {
    /// Works out which releases are reachable, given the one that booted.
    ///
    /// The booted source is taken at its word and only the *other* release is
    /// looked for, on [`Library::containers`]. Never fails: every reason a
    /// counterpart is missing collapses into "there is no counterpart".
    #[must_use]
    pub fn survey(booted: &str, library: &'static dyn Library) -> Self {
        let mut discs = Self {
            library,
            ..Self::default()
        };
        // Identified as whichever title it is, not as Pulse: Pulse's deny-list
        // names both Pure pressings, so a Pure boot got no platform and no music.
        let Some((title, layout)) = identify(library, booted) else {
            return discs;
        };
        discs.booted = Some(layout.platform);
        match layout.platform {
            Platform::Psp => discs.psp = Some(booted.to_string()),
            Platform::Ps2 => discs.ps2 = Some(booted.to_string()),
            // Recorded and then done with: HD has no second release to pair
            // against, so the search is skipped and MUSIC SOURCE never offered.
            Platform::Ps3 => {
                discs.ps3 = Some(booted.to_string());
                return discs;
            }
            // Same as PS3: one release, nothing to pair against.
            Platform::Vita => {
                discs.vita = Some(booted.to_string());
                return discs;
            }
            // Same as the Vita. Omega's music is unlocated (`oag_omega::TITLE`
            // carries `music: None`) and it has one release.
            Platform::Ps4 => {
                discs.ps4 = Some(booted.to_string());
                return discs;
            }
            Platform::Unknown => return discs,
        }

        // A source with no soundtrack (a partial extract) leaves nothing to
        // confirm a counterpart against, so the row is not offered.
        let Ok(Some(mine)) = Soundtrack::read(library, booted, layout.platform) else {
            return discs;
        };

        let wanted = match layout.platform {
            Platform::Psp => Platform::Ps2,
            _ => Platform::Psp,
        };
        let Some(found) = find_release(library, booted, title, wanted, &mine) else {
            return discs;
        };
        match wanted {
            Platform::Ps2 => discs.ps2 = Some(found),
            _ => discs.psp = Some(found),
        }
        discs
    }

    /// Whether both releases are reachable, which is when the row is offered.
    #[must_use]
    pub fn both(&self) -> bool {
        self.psp.is_some() && self.ps2.is_some()
    }

    /// The release the game booted from, when it is one this knows.
    #[must_use]
    pub fn booted(&self) -> Option<Platform> {
        self.booted
    }

    /// One line for a load report: what was found, and where.
    #[must_use]
    pub fn describe(&self) -> String {
        match (&self.psp, &self.ps2, &self.ps3, &self.vita, &self.ps4) {
            (Some(psp), Some(ps2), ..) => format!("PSP {psp} and PS2 {ps2}"),
            (Some(psp), None, ..) => format!("PSP {psp} only"),
            (None, Some(ps2), ..) => format!("PS2 {ps2} only"),
            (None, None, Some(ps3), ..) => format!("PS3 {ps3} only"),
            (None, None, None, Some(vita), _) => format!("Vita {vita} only"),
            (None, None, None, None, Some(ps4)) => format!("PS4 {ps4} only"),
            (None, None, None, None, None) => "neither release".to_string(),
        }
    }

    /// Which source `choice` resolves to, and which release it is.
    ///
    /// Falls back to the booted disc rather than silence: a settings file
    /// saying `ps2` carried onto a PSP-only machine should still have music.
    fn pick(&self, choice: MusicSource) -> Option<(&str, Platform)> {
        let chosen = choice.platform().and_then(|platform| match platform {
            Platform::Psp => self.psp.as_deref().map(|at| (at, platform)),
            _ => self.ps2.as_deref().map(|at| (at, platform)),
        });
        chosen.or_else(|| match self.booted? {
            Platform::Psp => self.psp.as_deref().map(|at| (at, Platform::Psp)),
            Platform::Ps2 => self.ps2.as_deref().map(|at| (at, Platform::Ps2)),
            Platform::Ps3 => self.ps3.as_deref().map(|at| (at, Platform::Ps3)),
            Platform::Vita => self.vita.as_deref().map(|at| (at, Platform::Vita)),
            Platform::Ps4 => self.ps4.as_deref().map(|at| (at, Platform::Ps4)),
            Platform::Unknown => None,
        })
    }
}

/// Which title `source` is, and how its archives are laid out.
///
/// `None` for a source that will not open as any title this build knows.
fn identify(
    library: &dyn Library,
    source: &str,
) -> Option<(&'static oag_title::Title, oag_assets::Layout)> {
    let opened = library.open(source)?;
    Some((opened.title, opened.archives.layout))
}

/// The first source on the search path carrying `wanted`'s encode of `mine`.
///
/// Identifying a candidate is only the prefilter; [`Soundtrack::pairs_with`]
/// decides (see [`MusicDiscs`]). A different title is refused before the
/// pairing test: Pulse's sixteen against Pure's nineteen would fail on count,
/// but that is a property of these two discs, not a rule.
fn find_release(
    library: &dyn Library,
    booted: &str,
    title: &'static oag_title::Title,
    wanted: Platform,
    mine: &Soundtrack,
) -> Option<String> {
    for path in library.containers() {
        let source = path.to_string_lossy().into_owned();
        if source == booted {
            continue;
        }
        let Some((theirs, layout)) = identify(library, &source) else {
            continue;
        };
        if theirs.name != title.name || layout.platform != wanted {
            continue;
        }
        if Soundtrack::read(library, &source, wanted)
            .ok()
            .flatten()
            .is_some_and(|theirs| theirs.pairs_with(mine))
        {
            return Some(source);
        }
    }
    None
}

/// The mixer, the device behind it, and what the composition root plays.
///
/// One per run, made in `main` before either way in (boot or `--race`).
pub struct Audio {
    output: Output,
    /// HD's authored mix, when the booted title has one. See [`hd_mix`].
    hd: hd_mix::Hd,
    /// Where `--dump-audio` writes. `None` with a device attached, because
    /// [`Output::render_tick`] refuses to pull samples the hardware callback
    /// is already draining.
    dump: Option<Dump>,
    /// The looping music voice. `None` when the source has no decodable music.
    music: Option<VoiceId>,
    /// Whether [`Audio::start_music`] has already run, whatever it decided.
    ///
    /// Separate from [`Self::music`] because both tick loops ask every tick and
    /// a source with no decodable music leaves `music` `None`: guarding on it
    /// alone would re-read the disc and re-run `ffmpeg` sixty times a second.
    music_attempted: bool,
    /// Which release the playing music came off, when it is one of the sixteen
    /// soundtrack tracks.
    ///
    /// `None` for nothing playing or music with no counterpart (the PSP front
    /// end's); [`Audio::set_music_source`] then declines to touch it. This is
    /// the whole of the row's scope; see [`MusicSource`].
    music_from: Option<Platform>,
    /// The soundtrack track each release has already been read for, so moving
    /// the row back is instant. Only ever one of the sixteen.
    ///
    /// Costs memory, not a live disc handle: track 0 is 33 MiB of PCM on PSP
    /// and 36 MiB on PS2, about 69 MiB held. Re-reading is slower, measured:
    /// the first move onto PS2 costs 2.0 s and onto PSP about 0.4 s with the
    /// `ffmpeg` cache filled (it also enumerates both soundtracks to pair them
    /// and opens the image twice, in [`Audio::locate`] and [`load_track`]).
    ///
    /// Unbounded only because the menu path never asks for more than one index
    /// ([`MUSIC_TRACK`]); [`Self::race_cache`] is the counterpart for a
    /// consumer without that guarantee.
    held: Vec<(Platform, Arc<Sound>)>,
    /// The movie's own sound. Separate from [`Self::music`] although both are on
    /// [`Bus::Music`]: the music runs for the session, this lasts one movie. See
    /// [`Audio::start_movie`].
    movie: Option<VoiceId>,
    /// The voice playing under a race, distinct from [`Self::music`] (the menu's):
    /// [`Audio::start_race_music`] stops one before starting the other. `None`
    /// outside a race or with no decodable music.
    ///
    /// **Authored, not recovered.** Nothing in the RE record says the original
    /// cycles a race playlist; the circuit-to-track mapping is unestablished
    /// (`docs/formats/ps2-audio.md`). Chosen, not measured, like
    /// `[graphics] boost_fov_kick`: no confidence score, no `names.tsv` row.
    race_voice: Option<VoiceId>,
    /// Which release the race voice came off, mirroring [`Self::music_from`].
    race_from: Option<Platform>,
    /// Which track of the **booted** disc's soundtrack order the race playlist
    /// is on. `None` until the first race starts one ([`Audio::initial_race_index`]);
    /// never reset by a visit to the menus.
    race_index: Option<usize>,
    /// Where the current race track was cut off, in the track's seconds
    /// ([`oag_audio::Mixer::seek`]'s unit), so the next race resumes.
    race_position: f64,
    /// The one race track presently decoded, replaced the moment the playlist
    /// advances or [`MusicSource`] moves it. Unlike [`Self::held`] (fixed
    /// index), this makes one pause/resume on the same track free, bounded at
    /// one track of PCM (33-36 MiB).
    race_cache: Option<(Platform, usize, Arc<Sound>)>,
    /// The menu voice's decoded sound, so leaving a race resumes it without
    /// re-reading the disc or re-running `ffmpeg`. `None` with no decodable menu music.
    menu_sound: Option<Arc<Sound>>,
    /// The race's held voices and generator. `None` outside a race and when the
    /// banks did not load; made on the first tick with cues, so a silent source
    /// never allocates one.
    sfx: Option<sfx::SfxVoices>,
    /// What [`Audio::start_race_music`] was last called with, so the
    /// advance-on-finish check in [`Audio::tick`] can fetch the next track
    /// without threading arguments through every caller. `None` outside a race.
    /// The fourth element is the booted soundtrack's length once known
    /// ([`Self::race_soundtrack_len`]).
    race_context: Option<(MusicDiscs, MusicSource, PathBuf, Option<usize>)>,
    /// The **next** race track being fetched ahead, as its index and worker.
    ///
    /// Keeps a track boundary from hitching: [`Self::advance_race_track`] used
    /// to decode the next track (0.4 to 2.8 s measured) inside the
    /// fixed-timestep loop. Started as soon as the current track plays
    /// ([`Self::maybe_prefetch_next_race_track`]), so advancing only joins it.
    ///
    /// The index travels with the worker because [`MusicSource`] can move the
    /// race voice mid-track, changing what "the next track" means without
    /// touching [`Self::race_index`]; [`Self::advance_race_track`] checks the
    /// two agree and [`Self::set_race_music_source`] drops a stale one.
    race_prefetch: Option<(usize, MusicFetchWorker)>,
    /// A `MUSIC SOURCE` switch being fetched off the tick thread
    /// ([`Self::set_music_source`]). At most one: a fresh request replaces it,
    /// and a late result from the detached worker is nobody's.
    source_switch: Option<PendingSwitch>,
}

/// A music track that has been read and decoded, before a voice is started on
/// it.
///
/// [`Loaded::from`] is why this is a struct: it records whether the track is
/// one of the sixteen, which decides whether MUSIC SOURCE may move it. See
/// [`MusicSource`]. Public, not its fields, so [`MusicFetchWorker::join`] can
/// hand one back across the lib/bin boundary.
pub struct Loaded {
    /// Which release it came off, or `None` for music with no counterpart.
    /// Becomes [`Audio::music_from`].
    from: Option<Platform>,
    sound: Arc<Sound>,
    what: String,
}

// Written out so a `{:?}` says how long the track is, not print three minutes of it.
impl std::fmt::Debug for Loaded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Loaded")
            .field("from", &self.from)
            .field("seconds", &self.sound.seconds())
            .field("what", &self.what)
            .finish()
    }
}

struct Dump {
    path: PathBuf,
    samples: Vec<f32>,
}

// Written out: the sample buffer is minutes of audio.
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
            .field("music_from", &self.music_from)
            .field("held", &self.held.len())
            .field("movie", &self.movie)
            .field("race_voice", &self.race_voice)
            .field("race_from", &self.race_from)
            .field("race_index", &self.race_index)
            .field("race_position", &self.race_position)
            .field("race_cache", &self.race_cache.is_some())
            .field("menu_sound", &self.menu_sound.is_some())
            .field("race_context", &self.race_context.is_some())
            .field(
                "race_prefetch",
                &self.race_prefetch.as_ref().map(|(i, _)| i),
            )
            .field(
                "source_switch",
                &self.source_switch.as_ref().map(|pending| pending.wanted),
            )
            .finish()
    }
}

impl Audio {
    /// Opens the output and applies the persisted volumes.
    ///
    /// When `dump` is set the device is **not** opened: with a stream attached
    /// the `cpal` callback drains the mixer, [`Output::render_tick`] returns
    /// zero, and the dump would be silence next to audible audio.
    ///
    /// `no_audio` (`--no-audio`) forces the same null backend with nothing
    /// collected. A `--screenshot --until` run with a real device is
    /// "audio-clocked" per ADR-0019 and finishes before the movie does, so a
    /// late frame's target is never reached; the null backend puts it on the
    /// tick-clocked fallback (see [`Self::movie_playhead`]), as a machine with
    /// no device gets (what CI exercises).
    #[must_use]
    pub fn open(
        settings: &crate::settings::Settings,
        dump: Option<PathBuf>,
        tap: Option<&oag_audio::TapSpec>,
        buffer: std::time::Duration,
        no_audio: bool,
    ) -> Self {
        let output = if dump.is_some() || no_audio {
            Output::null(DUMP_SAMPLE_RATE)
        } else {
            Output::open_or_null(tap, buffer)
        };
        if let Some(name) = output.device_name() {
            info!("audio: {name} at {} Hz", output.sample_rate());
        }
        let audio = Self {
            output,
            hd: hd_mix::Hd::default(),
            dump: dump.map(|path| Dump {
                path,
                samples: Vec::new(),
            }),
            music: None,
            music_attempted: false,
            music_from: None,
            held: Vec::new(),
            movie: None,
            race_voice: None,
            race_from: None,
            race_index: None,
            race_position: 0.0,
            race_cache: None,
            menu_sound: None,
            race_context: None,
            race_prefetch: None,
            source_switch: None,
            sfx: None,
        };
        audio.apply(settings);
        audio
    }

    /// Applies every persisted volume to the buses.
    ///
    /// Separate from [`Self::open`] because the menus change these while the
    /// game runs. The music bus alone carries [`MUSIC_MASTER_TRIM`], folded in
    /// here rather than [`crate::Volume::gain`] because the SFX and Speech
    /// sliders reach `Audio_SetSfxFadeTarget`, a separate chain; see
    /// `MUSIC_MASTER_TRIM` for the evidence.
    pub fn apply(&self, settings: &crate::settings::Settings) {
        // HD sets music and effects from its authored mix every tick, so only
        // the sliders are kept here. See `hd_mix`.
        self.hd.sliders.set([
            settings.music_volume.gain(),
            settings.sfx_volume.gain(),
            settings.speech_volume.gain(),
        ]);
        self.output.with_mixer(|mixer| {
            if self.hd.live.is_none() {
                mixer.set_bus_gain(Bus::Music, settings.music_volume.gain() * MUSIC_MASTER_TRIM);
                mixer.set_bus_gain(Bus::Sfx, settings.sfx_volume.gain());
            }
            mixer.set_bus_gain(Bus::Speech, settings.speech_volume.gain());
            // Group volumes first, then the master, as the original's chain does.
            // See `settings::Audio::master_volume`.
            mixer.set_master_gain(settings.master_volume.gain());
        });
    }

    /// The output, and through it the mixer: the seam a cue goes through.
    #[must_use]
    pub fn output(&self) -> &Output {
        &self.output
    }

    /// Starts the music this source plays under the front end, looping.
    ///
    /// Not at boot: the intro reel (`LogoFMV`) carries its own sound on
    /// [`Bus::Music`], so the front end's tick loops call this once the
    /// sequence has left a movie state (`Frontend::is_playing_movie`). Callers
    /// with no sequence to wait on start it outright.
    ///
    /// The releases play different things, by the disc's doing: the PSP its own
    /// 28-second loop ([`PSP_MUSIC_NAME`]), the PS2 [`PS2_MUSIC_PATH`] track
    /// [`MUSIC_TRACK`], one of the sixteen. [`MusicSource`] cannot reach the
    /// PSP's; [`Self::set_music_source`] enforces it.
    ///
    /// `cache_dir` is where a decoded PSP track lands
    /// (`oag_source::cache::default_audio_cache_dir`); the PS2 path ignores it.
    ///
    /// Never fatal: a source with no decodable music (or no `ffmpeg` on `PATH`
    /// for a PSP disc) says so on stdout and plays nothing.
    pub fn start_music(&mut self, discs: &MusicDiscs, choice: MusicSource, cache_dir: &Path) {
        // Idempotent: the front end's tick loops call this every tick and track
        // no edge. See [`Self::music_attempted`].
        if self.music_attempted {
            return;
        }
        self.music_attempted = true;
        if discs.booted() == Some(Platform::Ps3)
            && let Some((source, _)) = discs.pick(MusicSource::Auto)
        {
            self.load_hd_mix(discs.library, source);
        }
        // The PS2's front-end music is a soundtrack track, so it goes through
        // `fetch` and is stamped with its release; the PSP's is loaded unstamped,
        // and `music_from` staying `None` is the scoping rule.
        let loaded = match discs.booted() {
            Some(Platform::Ps2) => self.fetch(discs, choice, cache_dir),
            _ => front_end_music(discs, cache_dir),
        };

        match loaded {
            Ok(Some(loaded)) => {
                let seconds = loaded.sound.seconds();
                // So `pause_race_music` can restart this decode without disc or `ffmpeg`.
                self.menu_sound = Some(Arc::clone(&loaded.sound));
                self.music = self
                    .output
                    .with_mixer(|mixer| mixer.play(Play::looping(loaded.sound, Bus::Music)));
                self.music_from = self.music.and(loaded.from);
                debug!("audio: music {}, {seconds:.1} s, looping", loaded.what);
            }
            Ok(None) => warn!("audio: this source carries no music this can play"),
            Err(error) => race_music::warn_no_music("no music", &error),
        }
    }

    /// Starts (or resumes) the race playlist, stopping the menu voice first.
    ///
    /// **Authored, not recovered** (see [`Self::race_voice`]). Cycles the booted
    /// disc's soundtrack order instead of the menu's fixed [`MUSIC_TRACK`]. The
    /// first call picks an index via [`Self::initial_race_index`]; later ones
    /// continue from [`Self::pause_race_music`]'s position, as
    /// [`Self::race_index`] and [`Self::race_position`] are never reset here.
    ///
    /// Never fatal, like [`Self::start_music`].
    pub fn start_race_music(&mut self, discs: &MusicDiscs, choice: MusicSource, cache_dir: &Path) {
        if let Some(id) = self.music.take() {
            self.output.with_mixer(|mixer| mixer.stop(id));
        }
        // Defensive: nothing calls this with a race voice sounding today.
        if let Some(id) = self.race_voice.take() {
            self.output.with_mixer(|mixer| mixer.stop(id));
        }

        let index = self.reserve_race_music_index(discs);
        let seek = (self.race_position > 0.0).then_some(self.race_position);
        self.race_context = Some((discs.clone(), choice, cache_dir.to_path_buf(), None));
        self.play_race_track(discs, choice, cache_dir, index, seek);
        // Consumed only once applied to a sounding voice, so a failed attempt
        // keeps the saved position for a later call.
        if self.race_voice.is_some() {
            self.race_position = 0.0;
        }
    }

    /// Where the race playlist starts the first time it is asked.
    ///
    /// One past the menu's track when the menu plays one of the sixteen (a PS2
    /// boot), so the first race does not restart the recording the menu was
    /// playing; otherwise [`MUSIC_TRACK`].
    ///
    /// Not silent when "one past" fails: `menu_from.is_some()` means the
    /// soundtrack was already read once, so a failing re-read is an anomaly, and
    /// falling back unannounced would look like the intended behaviour in the
    /// load report.
    fn initial_race_index(menu_from: &Option<Platform>, discs: &MusicDiscs) -> usize {
        if menu_from.is_none() {
            return MUSIC_TRACK;
        }
        match Self::booted_soundtrack_len(discs) {
            0 => {
                warn!(
                    "audio: could not re-read the booted disc's own soundtrack length, so the \
                     race playlist starts at track {MUSIC_TRACK} rather than one past the menu's"
                );
                MUSIC_TRACK
            }
            len => next_race_index(MUSIC_TRACK, len),
        }
    }

    /// How many tracks the **booted** disc's soundtrack lists, or `0` when it
    /// cannot be read (a PSP boot or no soundtrack). The wrap modulus of
    /// [`Self::race_index`].
    fn booted_soundtrack_len(discs: &MusicDiscs) -> usize {
        let Some((source, platform)) = discs.pick(MusicSource::Auto) else {
            return 0;
        };
        Soundtrack::read(discs.library, source, platform)
            .ok()
            .flatten()
            .map_or(0, |soundtrack| soundtrack.tracks.len())
    }

    /// Stops the race playlist where it stands and resumes the menu voice.
    ///
    /// The position is saved to [`Self::race_position`] in the track's seconds
    /// so the next [`Self::start_race_music`] resumes; [`Self::race_index`] is
    /// untouched. The menu voice restarts from [`Self::menu_sound`] because
    /// [`Self::start_music`] is idempotent and would do nothing. Silent when no
    /// menu sound was ever loaded.
    pub fn pause_race_music(&mut self) {
        if let Some(id) = self.race_voice.take() {
            self.race_position = self
                .output
                .with_mixer(|mixer| mixer.position(id))
                .unwrap_or(0.0);
            self.output.with_mixer(|mixer| mixer.stop(id));
        }
        self.race_from = None;
        self.race_context = None;

        if let Some(sound) = &self.menu_sound {
            self.music = self
                .output
                .with_mixer(|mixer| mixer.play(Play::looping(Arc::clone(sound), Bus::Music)));
        }
    }

    /// How far into its track the race playlist has got, in seconds (see [`Self::playhead`]).
    #[must_use]
    pub fn playhead_race(&self) -> Option<f64> {
        self.output
            .with_mixer(|mixer| mixer.position(self.race_voice?))
    }

    /// Moves the playing **soundtrack track** onto the release `choice` names,
    /// without restarting it.
    ///
    /// The playhead carries across in seconds, the only unit that survives the
    /// change of rate (48,000 frames is one second on PS2, 1.088 on PSP); the
    /// sixteen pairs agree in length to 11 ms, a third of a video frame.
    ///
    /// Does nothing for anything that is not one of the sixteen (no
    /// [`Self::music_from`], e.g. the PSP front end's music), which is what
    /// scopes the row (see [`MusicSource`]); kept here so no second caller can
    /// forget it. Also a no-op when already on that release. Never fatal: a
    /// release that will not load leaves the music where it was.
    ///
    /// Moves whichever voice is live: while a race plays, that is the race
    /// voice, else the menu's. Using the menu's `music_from` during a race would
    /// desync the row from the audible voice and resume the wrong recording
    /// (see [`Self::set_race_music_source`]).
    ///
    /// Never blocks: a first read used to decode synchronously (0.4 s cached
    /// PSP, 2.0 s cold PS2) on the tick thread, a freeze on a player action. A
    /// release already in [`Self::held`]/[`Self::race_cache`] applies at once;
    /// a fresh one goes through a [`MusicFetchWorker`] and [`Self::tick`]
    /// applies the result.
    pub fn set_music_source(&mut self, discs: &MusicDiscs, choice: MusicSource, cache_dir: &Path) {
        if self.race_voice.is_some() {
            self.set_race_music_source(discs, choice, cache_dir);
        } else {
            self.set_menu_music_source(discs, choice, cache_dir);
        }
    }

    /// How far into the soundtrack track the music has got, in the **source's**
    /// seconds, the unit a swap carries across ([`Self::set_music_source`]). No
    /// "is the mixer moving" test (cf. [`Self::movie_playhead`]): nothing paces a
    /// picture against it.
    #[must_use]
    pub fn playhead(&self) -> Option<f64> {
        self.output.with_mixer(|mixer| mixer.position(self.music?))
    }

    /// The soundtrack track `choice` names, from the cache or from a disc.
    ///
    /// Returns the release it came off, the sound, and a line naming it;
    /// `Ok(None)` when the release has no soundtrack.
    ///
    /// The booted release's order decides which track [`MUSIC_TRACK`] is; the
    /// other release is reached by **length** ([`Soundtrack::nearest`]), since
    /// neither archive stores names and the two orders differ.
    fn fetch(
        &mut self,
        discs: &MusicDiscs,
        choice: MusicSource,
        cache_dir: &Path,
    ) -> Result<Option<Loaded>> {
        let Some((source, platform)) = discs.pick(choice) else {
            return Ok(None);
        };
        if let Some((_, sound)) = self.held.iter().find(|(held, _)| *held == platform) {
            return Ok(Some(Loaded {
                from: Some(platform),
                sound: Arc::clone(sound),
                what: format!("{platform} soundtrack track {MUSIC_TRACK}, already read"),
            }));
        }

        // No fallback to the front end's music: it would be a different
        // recording, and the row claims its values are one recording encoded
        // twice (see [`MusicSource`]). The caller reports the absence.
        let Some(track) = locate(discs, platform, source, MUSIC_TRACK)? else {
            return Ok(None);
        };

        let sound = Arc::new(load_track(
            discs.library,
            source,
            platform,
            track,
            cache_dir,
        )?);
        self.held.push((platform, Arc::clone(&sound)));
        Ok(Some(Loaded {
            from: Some(platform),
            sound,
            what: format!(
                "{platform} soundtrack track {MUSIC_TRACK}, {:.1} s as its own disc lists it",
                track.seconds
            ),
        }))
    }

    /// [`Self::fetch`]'s race counterpart: any track of the booted order, cached
    /// in the single-slot [`Self::race_cache`] rather than the unbounded
    /// [`Self::held`], which is safe only because the menu asks for one index.
    fn fetch_indexed(
        &mut self,
        discs: &MusicDiscs,
        choice: MusicSource,
        cache_dir: &Path,
        index: usize,
    ) -> Result<Option<Loaded>> {
        let Some((source, platform)) = discs.pick(choice) else {
            return Ok(None);
        };
        if let Some((cached_platform, cached_index, sound)) = &self.race_cache
            && *cached_platform == platform
            && *cached_index == index
        {
            return Ok(Some(Loaded {
                from: Some(platform),
                sound: Arc::clone(sound),
                what: format!("{platform} soundtrack track {index}, already read"),
            }));
        }

        let Some(track) = locate(discs, platform, source, index)? else {
            return Ok(None);
        };

        let sound = Arc::new(load_track(
            discs.library,
            source,
            platform,
            track,
            cache_dir,
        )?);
        Ok(Some(Loaded {
            from: Some(platform),
            sound,
            what: format!(
                "{platform} soundtrack track {index}, {:.1} s as its own disc lists it",
                track.seconds
            ),
        }))
    }

    /// Starts one of the boot sequence's movie sounds, reporting what happened.
    ///
    /// The seam both tick loops go through, so the stdout line and the decision
    /// cannot differ between them. `None` is the ordinary silent case (see the boot movie
    /// loading in `oag_game::boot`). `what` names the movie, because
    /// Pure plays a second one when `FMV Intro` is entered.
    ///
    /// The line reports what clocks the picture, checked against
    /// [`Self::movie_playhead`] after starting the voice: on a null backend
    /// nothing pulls from the mixer and the picture is tick-clocked (ADR-0019),
    /// and claiming otherwise misled two captures before `--no-audio` existed.
    pub fn start_boot_movie(&mut self, what: &str, sound: Option<oag_music::at3::Pcm>) {
        let Some(pcm) = sound else {
            warn!("audio: {what} plays silently");
            return;
        };
        let seconds = pcm.samples.len() as f64
            / f64::from(pcm.channels.max(1))
            / f64::from(pcm.sample_rate.max(1));
        match Sound::new(pcm.samples, pcm.channels, pcm.sample_rate) {
            Ok(sound) => {
                if self.start_movie(sound) {
                    if self.movie_playhead().is_some() {
                        debug!("audio: {what}'s own track, {seconds:.2} s, clocking the picture");
                    } else {
                        debug!(
                            "audio: {what}'s own track, {seconds:.2} s, but nothing pulls from \
                             the mixer - tick-clocked instead"
                        );
                    }
                } else {
                    // Every slot busy: cannot happen today, reported rather than silent.
                    warn!("audio: no free voice for {what}, so it plays silently");
                }
            }
            Err(error) => warn!("audio: {what} plays silently ({error:#})"),
        }
    }

    /// Starts a movie's own sound, replacing whatever was playing.
    ///
    /// One shot, on [`Bus::Music`] (the original has no movie bus either).
    /// Returns whether a voice started, which [`Self::movie_playhead`] paces the
    /// picture against; `false` (every slot busy) plays the movie silently.
    pub fn start_movie(&mut self, sound: Sound) -> bool {
        self.stop_movie();
        self.movie = self
            .output
            .with_mixer(|mixer| mixer.play(Play::once(Arc::new(sound), Bus::Music)));
        self.movie.is_some()
    }

    /// Stops the movie's sound, if any. A separate call because the intro can be
    /// skipped at any point and must not play on under the menus.
    pub fn stop_movie(&mut self) {
        if let Some(id) = self.movie.take() {
            self.output.with_mixer(|mixer| mixer.stop(id));
        }
    }

    /// How far into its own sound the playing movie has got, in seconds.
    ///
    /// ADR-0019's clock rule in one place: `Some` paces the picture against
    /// this, `None` against the tick. `None` covers no audio stream,
    /// `sound="false"`, no `ffmpeg`, an ended voice, and a mixer nobody pulls:
    /// [`Self::tick`] pulls samples only for a dump (with a device `cpal`
    /// drains it), so a null output with no dump never advances, a voice sits at
    /// zero, and `--until` would run to its tick ceiling. The mixer must be
    /// moving as well as sounding.
    #[must_use]
    pub fn movie_playhead(&self) -> Option<f64> {
        if !(self.dump.is_some() || self.output.is_streaming()) {
            return None;
        }
        self.output.with_mixer(|mixer| mixer.position(self.movie?))
    }

    /// Advances the mixer by one simulation tick.
    ///
    /// Called from inside the fixed-timestep loop, never per frame, so a capture
    /// of `ticks * sample_rate / 60` frames reproduces exactly. Does nothing with
    /// a device attached: `cpal`'s callback is already draining the mixer.
    pub fn tick(&mut self) {
        // Before the race-boundary check: a switch landing this tick may have
        // just replaced `race_voice`, and polling first keeps the two from
        // both moving it.
        self.poll_source_switch();
        self.hd_mix_tick(self.hd.state);
        if let Some(id) = self.race_voice
            && !self.output.with_mixer(|mixer| mixer.is_playing(id))
        {
            self.advance_race_track();
        } else {
            // Only while the voice is still the one playing: a track boundary or
            // `MUSIC SOURCE` toggle invalidates the prefetch (sorted out above or
            // in `Self::set_race_music_source`).
            self.maybe_prefetch_next_race_track();
        }
        if let Some(dump) = &mut self.dump {
            self.output.render_tick(TICK_HZ, &mut dump.samples);
        }
    }

    /// Pauses (`true`) or restarts (`false`) the device while the window is
    /// away. See [`Output::set_paused`]. Output only: no voice, cue or music
    /// position changes, so what was playing resumes where it stopped.
    pub fn set_suspended(&self, suspended: bool) {
        self.output.set_paused(suspended);
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
/// Fixed at 60 like `oag_core::TickRate`, so a dump's length depends on the tick
/// count alone.
const TICK_HZ: u32 = 60;

/// The index one past `index` in a soundtrack of `len` tracks, wrapping.
///
/// A free function so the wrap arithmetic is testable without a disc. `len == 0`
/// leaves `index` alone rather than dividing by zero.
fn next_race_index(index: usize, len: usize) -> usize {
    if len == 0 { index } else { (index + 1) % len }
}

/// Reads one entry of `PS2MUSIC.WAD` off `source` and turns it into a sound.
///
/// `Ok(None)` when the source has no such file (a PSP disc, a partial extract).
///
/// The whole archive is never read: it is 585 MiB on the EU disc and a track
/// about 35, so this reads the 4-byte header, the directory it sizes, then
/// exactly the entry. [`oag_assets::read_loose_file`] reads a loose file whole,
/// wrong by a factor of sixteen here.
fn load_ps2_track(source: &str, index: usize) -> Result<Option<Sound>> {
    let Some(mut archive) = MusicArchive::open(source)? else {
        return Ok(None);
    };

    let directory = ps2_directory(&mut archive)?;
    let entry = directory.entries.get(index).with_context(|| {
        format!(
            "{PS2_MUSIC_PATH} has {} track(s), so there is no track {index}",
            directory.entries.len()
        )
    })?;

    let pcm = archive.read(u64::from(entry.offset), u64::from(entry.size))?;
    // Signed 16-bit little-endian, two channels interleaved left first; none of
    // it stated by the file (`docs/formats/ps2-audio.md`). `as_chunks` drops a
    // trailing odd byte, which `Directory::parse` already rejected.
    let samples: Vec<i16> = pcm
        .as_chunks::<2>()
        .0
        .iter()
        .map(|pair| i16::from_le_bytes([pair[0], pair[1]]))
        .collect();
    let sound = Sound::new(samples, ps2_music::CHANNELS, ps2_music::SAMPLE_RATE)
        .with_context(|| format!("track {index} of {PS2_MUSIC_PATH}"))?;
    Ok(Some(sound))
}

/// Reads the archive's directory only: four bytes for the entry count, then the
/// table they size (the payload is 559 MiB).
fn ps2_directory(archive: &mut MusicArchive) -> Result<ps2_music::Directory> {
    let header = archive.read(0, ps2_music::HEADER_LEN as u64)?;
    let count = ps2_music::peek_entry_count(&header)
        .map_err(|e| anyhow::anyhow!("{PS2_MUSIC_PATH} is not a music archive: {e}"))?;
    let table = archive.read(0, ps2_music::directory_len(count))?;
    ps2_music::Directory::parse(&table, Some(archive.len()))
        .map_err(|e| anyhow::anyhow!("reading the {PS2_MUSIC_PATH} directory: {e}"))
}

/// One release's soundtrack, as that release lists it.
///
/// The PS2 keys its archive by position and the PSP keys `Data.wad` by name
/// hash, and the orders differ (PS2 track 12 is the PSP's second in `Data.wad`
/// order). The PSP names are recovered from the plugin definition, the PS2
/// archive carries none, so they pair on length, within 11 ms over all sixteen
/// (`docs/formats/ps2-audio.md`), via [`Self::nearest`].
#[derive(Debug, Clone)]
struct Soundtrack {
    tracks: Vec<Track>,
}

/// One soundtrack track, addressed the way its own disc addresses it.
#[derive(Debug, Clone, Copy)]
struct Track {
    /// A `PS2MUSIC.WAD` directory index, or a `Data.wad` name hash.
    at: u32,
    /// How long it is. The only thing the two releases have in common.
    seconds: f64,
}

/// How far apart two tracks' lengths may be and still be the same recording.
///
/// The measured gap is 11.4 ms, identical to a tenth of a millisecond on all
/// sixteen pairs (the ATRAC3+ encoder's constant trailing padding). A second is
/// far above that and far below the nearest wrong answer: the tracks are 177 to
/// 205 s and the closest two are 0.13 s apart. It is not wide against another
/// game: Pure's shortest track is 205.2 s and Pulse's longest 204.3 s. That is
/// why [`Soundtrack::pairs_with`] demands a complete one-for-one assignment
/// and checks the count first (Pure has nineteen tracks, Pulse sixteen).
const PAIR_TOLERANCE: f64 = 1.0;

impl Soundtrack {
    /// Reads the soundtrack `source` carries, for the release it is.
    ///
    /// `Ok(None)` when the source has none (a partial extract, or the other
    /// release's disc).
    ///
    /// # Errors
    ///
    /// An archive that opens and then will not parse.
    fn read(library: &dyn Library, source: &str, platform: Platform) -> Result<Option<Self>> {
        let tracks = match platform {
            Platform::Ps2 => ps2_soundtrack(source)?,
            Platform::Psp | Platform::Ps3 | Platform::Vita | Platform::Ps4 => {
                archived_soundtrack(library, source)?
            }
            Platform::Unknown => None,
        };
        Ok(tracks.map(|tracks| Self { tracks }))
    }

    /// Whether this is the same soundtrack as `other`, one track for one track.
    ///
    /// Every track within [`PAIR_TOLERANCE`] of a **distinct** partner: an
    /// unrelated album may land near some of the sixteen by luck, not across all
    /// of them. Measured on the real pair, every partner is within 11.4 ms and
    /// the closest wrong answer is 130 ms away. This stops another Studio
    /// Liverpool UMD being taken for the counterpart (see [`MusicDiscs`]).
    fn pairs_with(&self, other: &Self) -> bool {
        if self.tracks.len() != other.tracks.len() || self.tracks.is_empty() {
            return false;
        }
        let mut taken = vec![false; self.tracks.len()];
        for track in &other.tracks {
            let found = self
                .tracks
                .iter()
                .enumerate()
                .filter(|(index, mine)| {
                    !taken[*index] && (mine.seconds - track.seconds).abs() <= PAIR_TOLERANCE
                })
                .min_by(|(_, a), (_, b)| {
                    (a.seconds - track.seconds)
                        .abs()
                        .total_cmp(&(b.seconds - track.seconds).abs())
                })
                .map(|(index, _)| index);
            match found {
                Some(index) => taken[index] = true,
                None => return false,
            }
        }
        true
    }

    /// The track nearest `seconds` long, if one is within [`PAIR_TOLERANCE`].
    ///
    /// `None` rather than the least-bad answer: a source that does not pair is
    /// misidentified, and the wrong three minutes is worse than none.
    fn nearest(&self, seconds: f64) -> Option<Track> {
        self.tracks
            .iter()
            .copied()
            .filter(|track| (track.seconds - seconds).abs() <= PAIR_TOLERANCE)
            .min_by(|a, b| {
                (a.seconds - seconds)
                    .abs()
                    .total_cmp(&(b.seconds - seconds).abs())
            })
    }
}

/// The `PS2MUSIC.WAD` directory as a soundtrack listing.
///
/// Length is `size / (4 * 48000)` seconds: uncompressed PCM of known geometry
/// (`docs/formats/ps2-audio.md`).
fn ps2_soundtrack(source: &str) -> Result<Option<Vec<Track>>> {
    let Some(mut archive) = MusicArchive::open(source)? else {
        return Ok(None);
    };
    let directory = ps2_directory(&mut archive)?;
    Ok(Some(
        directory
            .entries
            .iter()
            .enumerate()
            .map(|(index, entry)| Track {
                at: index as u32,
                seconds: f64::from(entry.seconds()),
            })
            .collect(),
    ))
}

/// The soundtrack entries of a source whose music is inside an archive (every
/// release but PS2).
///
/// Which entries depends on the booted title, so it is asked in `oag_music`;
/// [`Track::at`] is opaque ([`oag_music::Entry::at`]).
fn archived_soundtrack(library: &dyn Library, source: &str) -> Result<Option<Vec<Track>>> {
    Ok(listing(library, source)?.map(|entries| {
        entries
            .into_iter()
            .map(|entry| Track {
                at: entry.at,
                seconds: entry.seconds,
            })
            .collect()
    }))
}

/// Reads one soundtrack track off `source` and turns it into a sound.
///
/// # Errors
///
/// An archive that will not open or read, or a failed PSP decode (including
/// `ffmpeg` being absent, which the caller reports rather than treats as fatal).
fn load_track(
    library: &dyn Library,
    source: &str,
    platform: Platform,
    track: Track,
    cache_dir: &Path,
) -> Result<Sound> {
    match platform {
        Platform::Ps2 => load_ps2_track(source, track.at as usize)?
            .with_context(|| format!("{source} carries no {PS2_MUSIC_PATH}")),
        _ => load_entry(library, source, track.at, cache_dir),
    }
}

/// The music the front end plays, for a source that has its own.
///
/// The PSP's front-end music, for every source but PS2. Not routed through
/// [`Audio::fetch`]: it is not a soundtrack track and has no counterpart, which
/// the caller records by leaving [`Audio::music_from`] unset.
fn front_end_music(discs: &MusicDiscs, cache_dir: &Path) -> Result<Option<Loaded>> {
    let Some((source, _)) = discs.pick(MusicSource::Auto) else {
        return Ok(None);
    };
    Ok(
        load_front_end(discs.library, source, cache_dir)?.map(|(what, sound)| Loaded {
            // Unstamped: no counterpart on the other disc, so nothing may swap it.
            from: None,
            sound: Arc::new(sound),
            what,
        }),
    )
}

/// A seekable handle on `PS2MUSIC.WAD`, from a disc image or a directory
/// extracted with `oag-unpack`. Both read lazily (see [`load_ps2_track`]).
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

        // Not a directory or a readable disc (an `image:path` spec, say): not an
        // error, the caller already reports "no music here".
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
/// Matched on file name: the serial folder sits under whatever `-o` chose, and
/// only one file has this name.
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
mod tests;
