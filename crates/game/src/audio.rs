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
use oag_disc::{DiscImage, Platform};
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

/// Which track is played, by index into the booted release's own soundtrack.
///
/// The first, because nothing yet maps a circuit or a menu to a track: the
/// entries are addressed by name hash on both discs and no name for any of them
/// has been recovered. Playing a fixed one proves the path end to end without
/// claiming a mapping that has not been established.
const MUSIC_TRACK: usize = 0;

/// Which release's encode of the soundtrack is played.
///
/// **Music, not audio.** Only the sixteen soundtrack tracks have a counterpart
/// on the other disc - a 16-for-16 bijection with a mean duration gap of 11 ms,
/// established in `docs/formats/ps2-audio.md`. Voice and every sound bank stay
/// on whatever disc the game booted from, because nothing pairs them.
///
/// # This does not govern the front end's own music
///
/// **And a reader who expects it to should stop here rather than file a bug.**
/// The PSP release has music written for its menus,
/// `Data\Music\FEMusic\frontend1.at3`, a 28-second loop that is *not* one of
/// the sixteen: no PS2 entry has ever been matched to it, and `PS2MUSIC.WAD`
/// holds sixteen entries of which all sixteen are three-minute soundtrack
/// tracks. With no counterpart there is nothing for this to choose between, so
/// a PSP boot plays its front-end music at every value of this setting.
///
/// Offering it anyway would be worse than useless: the row promises that its
/// three values are **one recording encoded twice**, which is what licenses
/// [`Audio::set_music_source`] to seek rather than restart. Pointing it at two
/// unrelated pieces of music would break that promise and land the playhead 20
/// seconds into something else.
///
/// The PS2 release has no front-end music of its own that has been found, and
/// what it plays under its menus **is** soundtrack track 0 - one of the
/// sixteen. So the row is effective on a PS2 boot and inert on a PSP one, and
/// that asymmetry is the discs', not this module's. Nothing here adds music to
/// a screen that had none.
///
/// The rule is enforced in exactly one place, by
/// [`Audio::music_from`] being `None` for anything that was not loaded as one
/// of the sixteen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MusicSource {
    /// The disc the game booted from.
    ///
    /// **The default, and deliberately not "whichever is better".** Running the
    /// PSP release should sound like the PSP release; ATRAC3+ at 96 kbit/s is
    /// part of what that sounded like, and swapping it out by default would be
    /// this project deciding a player's copy of the game was wrong.
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
/// offered at all, and re-deriving it on a keypress would mean opening every
/// disc image on the search path while a menu is on screen.
///
/// # The counterpart is confirmed by its soundtrack, not by its serial
///
/// **Neither a file name nor a disc serial is enough here, and both were tried
/// first.** `crate::source::IMAGE_NAMES` does not cover every regional
/// pressing - `pulse-psp-eu.chd` is not on it - and a file somebody renamed is
/// still a valid source. `oag_assets::Layout::resolve` looked like the
/// answer, but its deny-list is deliberately *positive-only*: it rules out the
/// one Pure serial this project has verified (`UCUS-98612`) and gives an
/// unlisted serial no verdict, because allow-listing would hard-reject a
/// player's own legitimate pressing. Measured, a `data/images/` holding
/// `pure-psp-eu.chd` (`UCES-00001`, not on that list) reported it as the PSP
/// counterpart.
///
/// That polarity is right for the disc a player *named* and wrong for one
/// nobody did: here a false positive is silent and plays another game's music,
/// while a false negative only means the row is not offered. So a candidate has
/// to **pair**, through [`Soundtrack::pairs_with`] - sixteen tracks against
/// sixteen, each within [`PAIR_TOLERANCE`] of a distinct partner. That is the
/// measurement `docs/formats/ps2-audio.md` records, run against the disc in
/// hand rather than trusted from a table, and it is a far stronger test than
/// any serial: a disc that passes it demonstrably carries this soundtrack.
#[derive(Debug, Clone, Default)]
pub struct MusicDiscs {
    /// A Pulse PSP source, if one was found.
    psp: Option<String>,
    /// A Pulse PS2 source, if one was found.
    ps2: Option<String>,
    /// Which release the game booted from, when it is one of the two.
    booted: Option<Platform>,
}

impl MusicDiscs {
    /// Works out which releases are reachable, given the one that booted.
    ///
    /// The booted source is taken at its word - the game is already running off
    /// it - and only the *other* release is looked for, on
    /// [`crate::source::search_path`]. That halves the work and it also stops
    /// the survey from opening a second copy of the disc already open.
    ///
    /// Never fails: every reason a counterpart is not found - no such directory,
    /// an image that will not open, a Pure UMD sitting beside a Pulse one -
    /// collapses into "there is no counterpart", which is the same answer a
    /// machine with one disc gives.
    #[must_use]
    pub fn survey(booted: &str) -> Self {
        let mut discs = Self::default();
        let Ok(layout) = oag_assets::Layout::resolve(booted, oag_pulse::TITLE) else {
            return discs;
        };
        discs.booted = Some(layout.platform);
        match layout.platform {
            Platform::Psp => discs.psp = Some(booted.to_string()),
            Platform::Ps2 => discs.ps2 = Some(booted.to_string()),
            Platform::Unknown => return discs,
        }

        // The booted disc's own soundtrack is what a candidate has to match,
        // so it is read first. A source that has none - a partial extract -
        // leaves nothing to confirm a counterpart against, and the row is not
        // offered rather than offered on trust.
        let Ok(Some(mine)) = Soundtrack::read(booted, layout.platform) else {
            return discs;
        };

        let wanted = match layout.platform {
            Platform::Psp => Platform::Ps2,
            _ => Platform::Psp,
        };
        let Some(found) = find_release(booted, wanted, &mine) else {
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
        match (&self.psp, &self.ps2) {
            (Some(psp), Some(ps2)) => format!("PSP {psp} and PS2 {ps2}"),
            (Some(psp), None) => format!("PSP {psp} only"),
            (None, Some(ps2)) => format!("PS2 {ps2} only"),
            (None, None) => "neither release".to_string(),
        }
    }

    /// Which source `choice` resolves to, and which release it is.
    ///
    /// **Falls back to the booted disc** rather than to silence when the chosen
    /// release is not reachable: a settings file that says `ps2` carried onto a
    /// machine with only the PSP disc should still have music, and the row is
    /// not offered there to say otherwise.
    fn pick(&self, choice: MusicSource) -> Option<(&str, Platform)> {
        let chosen = choice.platform().and_then(|platform| match platform {
            Platform::Psp => self.psp.as_deref().map(|at| (at, platform)),
            _ => self.ps2.as_deref().map(|at| (at, platform)),
        });
        chosen.or_else(|| match self.booted? {
            Platform::Psp => self.psp.as_deref().map(|at| (at, Platform::Psp)),
            Platform::Ps2 => self.ps2.as_deref().map(|at| (at, Platform::Ps2)),
            Platform::Unknown => None,
        })
    }
}

/// The first source on the search path carrying `wanted`'s encode of `mine`.
///
/// Every container in every directory [`crate::source::search_path`] lists is
/// tried, in that order. `Layout::resolve` is only the **prefilter** - it says
/// what platform a disc is and skips one that carries no archives at all - and
/// [`Soundtrack::pairs_with`] is the test that decides, for the reason
/// [`MusicDiscs`] records at length: a serial cannot rule a disc *in*, and a
/// counterpart nobody named has to be ruled in rather than merely not ruled
/// out.
fn find_release(booted: &str, wanted: Platform, mine: &Soundtrack) -> Option<String> {
    for directory in crate::source::search_path() {
        let Ok(entries) = std::fs::read_dir(&directory) else {
            continue;
        };
        let mut candidates: Vec<PathBuf> = entries
            .flatten()
            .map(|entry| entry.path())
            .filter(|path| path.is_file() && crate::source::is_container(path))
            .collect();
        // Alphabetical, so two runs of the same directory pick the same image -
        // the same reason `crate::source::first_image` sorts.
        candidates.sort();

        for path in candidates {
            let source = path.to_string_lossy().into_owned();
            if source == booted {
                continue;
            }
            if !oag_assets::Layout::resolve(&source, oag_pulse::TITLE)
                .is_ok_and(|layout| layout.platform == wanted)
            {
                continue;
            }
            if Soundtrack::read(&source, wanted)
                .ok()
                .flatten()
                .is_some_and(|theirs| theirs.pairs_with(mine))
            {
                return Some(source);
            }
        }
    }
    None
}

/// The PSP front end's music, in `Data.wad`.
///
/// **Named, not guessed.** The executable builds this path at run time from the
/// template at `0x08a88e94`, `Data\Music\FEMusic\frontend%d.at3` - see
/// `docs/formats/vex.md` - and hashing the expansion finds an entry for every
/// `%d` from 1 to 8 and none for 0, so the numbering starts at one. All eight
/// are stereo ATRAC3+ at 44,100 Hz; the first declares 1,302,720 samples in its
/// `fact` chunk and decodes to **29.5 seconds**, which is what a `--dump-audio`
/// capture of a PSP boot reports.
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
    /// Which release the playing music came off, **when what is playing is one
    /// of the sixteen soundtrack tracks**.
    ///
    /// `None` means either nothing is playing or what is playing has no
    /// counterpart on the other disc - the PSP front end's own music is that
    /// case - and [`Audio::set_music_source`] declines to touch it. This one
    /// field is the whole of the row's scope; see [`MusicSource`].
    music_from: Option<Platform>,
    /// The soundtrack track each release has already been read for, kept so
    /// that moving the row back is instant. Only ever holds one of the sixteen:
    /// the front end's own music never enters here, because nothing would ever
    /// ask for it a second time.
    ///
    /// **This is what the setting costs**, and it is memory rather than a
    /// second live disc handle: track 0 is 33 MiB of PCM as the PSP stores it
    /// and 36 MiB as the PS2 does, so holding both is about 69 MiB. Nothing
    /// here keeps a [`DiscImage`] open - each release is read once, whole, and
    /// the handle dropped.
    ///
    /// The alternative is re-reading on every nudge of the row, and that is
    /// measured rather than assumed: on this machine the first move onto the
    /// PS2 release costs **2.0 seconds** and the first onto the PSP release
    /// about **0.4 seconds** with the `ffmpeg` cache already filled. Neither
    /// figure is only the payload read - a cross-disc selection also
    /// enumerates both soundtracks to pair them, and opens the chosen image
    /// twice, once in [`Audio::locate`] and once in [`load_track`]. A row a
    /// player cycles through three values would otherwise stall every time
    /// round.
    held: Vec<(Platform, Arc<Sound>)>,
    /// The movie's own sound, while a movie is playing one.
    ///
    /// Separate from [`Self::music`] although both are on [`Bus::Music`],
    /// because they are stopped at different moments and by different things:
    /// the music runs for the whole session and this lasts one movie. See
    /// [`Audio::start_movie`].
    movie: Option<VoiceId>,
}

/// A music track that has been read and decoded, before a voice is started on
/// it.
///
/// [`Loaded::from`] is the field that matters and the reason this is a struct
/// rather than a tuple: it carries whether what was loaded is one of the
/// sixteen soundtrack tracks, which is what decides whether MUSIC SOURCE may
/// ever move it. See [`MusicSource`].
struct Loaded {
    /// Which release it came off, or `None` for music with no counterpart -
    /// the PSP front end's own. Becomes [`Audio::music_from`].
    from: Option<Platform>,
    /// The decoded samples.
    sound: Arc<Sound>,
    /// One line naming it, for the load report on stdout.
    what: String,
}

// Written out rather than derived for the reason [`Dump`]'s is: a `{:?}` of one
// of these should say how long the track is, not print three minutes of it.
impl std::fmt::Debug for Loaded {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Loaded")
            .field("from", &self.from)
            .field("seconds", &self.sound.seconds())
            .field("what", &self.what)
            .finish()
    }
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
            .field("music_from", &self.music_from)
            .field("held", &self.held.len())
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
            music_from: None,
            held: Vec::new(),
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

    /// Starts the music this source plays under the front end, looping.
    ///
    /// **The two releases play different things here, and that is the disc's
    /// doing rather than a choice made in this module.** The PSP has front-end
    /// music of its own - [`PSP_MUSIC_NAME`], a 28-second loop - and that is
    /// what it plays. The PS2 release has no such entry anywhere that has been
    /// found, and what it plays is [`PS2_MUSIC_PATH`] track [`MUSIC_TRACK`],
    /// one of the sixteen soundtrack tracks, at three minutes. Neither is
    /// touched by [`MusicSource`]: see that type for why the row cannot reach
    /// the PSP's, and [`Self::set_music_source`] for the one line that enforces
    /// it.
    ///
    /// `cache_dir` is where a decoded PSP track lands - see
    /// [`crate::boot::default_audio_cache_dir`]. It is unused by the PS2 path,
    /// which stores its music as PCM and needs no decoder.
    ///
    /// **Never fatal.** A source with no decodable music says so on stdout and
    /// plays nothing, which is the same degradation the video path takes when
    /// `ffmpeg` is missing: name what is absent and carry on. A machine with no
    /// `ffmpeg` on `PATH` is that case for a PSP disc, and the message names
    /// the tool.
    pub fn start_music(&mut self, discs: &MusicDiscs, choice: MusicSource, cache_dir: &Path) {
        if self.music.is_some() {
            return;
        }
        // The PS2's front-end music **is** a soundtrack track, so it goes
        // through `fetch` and comes back stamped with the release it came off -
        // which is what makes the row able to move it. The PSP's is not one,
        // so it is loaded directly and left unstamped; `music_from` staying
        // `None` is the whole of the scoping rule.
        let loaded = match discs.booted() {
            Some(Platform::Ps2) => self.fetch(discs, choice, cache_dir),
            _ => front_end_music(discs, cache_dir),
        };

        match loaded {
            Ok(Some(loaded)) => {
                let seconds = loaded.sound.seconds();
                self.music = self
                    .output
                    .with_mixer(|mixer| mixer.play(Play::looping(loaded.sound, Bus::Music)));
                self.music_from = self.music.and(loaded.from);
                println!("audio: music {}, {seconds:.1} s, looping", loaded.what);
            }
            Ok(None) => println!("audio: this source carries no music this can play"),
            Err(error) => println!("audio: no music ({error:#})"),
        }
    }

    /// Moves the playing **soundtrack track** onto the release `choice` names,
    /// without restarting it.
    ///
    /// The playhead is carried across in seconds, which is the unit
    /// [`oag_audio::Mixer::position`] and [`oag_audio::Mixer::seek`] both speak
    /// and the only one that survives the change of rate: 48,000 frames into
    /// the PS2's track is one second and into the PSP's is 1.088. That the two
    /// land in the same bar is not an assumption either - the sixteen pairs
    /// agree in length to 11 milliseconds, which is a third of a video frame.
    ///
    /// # What it deliberately does not touch
    ///
    /// **Anything that is not one of the sixteen.** A voice with no
    /// [`Self::music_from`] was not loaded as a soundtrack track - the PSP
    /// front end's own music is the case that exists - and this returns without
    /// doing anything at all. That single condition is what scopes the row, and
    /// it is here rather than at the call site so no second caller can forget
    /// it. See [`MusicSource`].
    ///
    /// Also does nothing when the music is already on that release, so nudging
    /// the row back and forth past a value costs nothing.
    ///
    /// **Never fatal**, for the same reason [`Self::start_music`] is not: a
    /// release that will not load leaves the music where it was and says so.
    pub fn set_music_source(&mut self, discs: &MusicDiscs, choice: MusicSource, cache_dir: &Path) {
        let Some((_, wanted)) = discs.pick(choice) else {
            return;
        };
        let (Some(playing), Some(from)) = (self.music, self.music_from) else {
            // Either nothing is playing, or what is playing is not one of the
            // sixteen. Neither is this row's business - see above.
            return;
        };
        if from == wanted {
            return;
        }

        let at = self.playhead().unwrap_or(0.0);
        match self.fetch(discs, choice, cache_dir) {
            Ok(Some(loaded)) => {
                self.output.with_mixer(|mixer| {
                    mixer.stop(playing);
                    let started = mixer.play(Play::looping(loaded.sound, Bus::Music));
                    if let Some(id) = started {
                        mixer.seek(id, at);
                    }
                    self.music = started;
                });
                self.music_from = self.music.and(loaded.from);
                println!("audio: music {}, from {at:.1} s", loaded.what);
            }
            Ok(None) => {
                println!("audio: no soundtrack on the {wanted} release, so nothing changed")
            }
            Err(error) => println!("audio: the music stays where it is ({error:#})"),
        }
    }

    /// How far into the soundtrack track the music has got, in seconds.
    ///
    /// The **source's** own seconds, which is the unit a swap carries across -
    /// see [`Self::set_music_source`]. Unlike [`Self::movie_playhead`] this
    /// needs no "is the mixer actually moving" test, because nothing paces a
    /// picture against it: a music voice at zero on a mixer nobody pulls from
    /// is a track that has not started, which is the truth.
    #[must_use]
    pub fn playhead(&self) -> Option<f64> {
        self.output.with_mixer(|mixer| mixer.position(self.music?))
    }

    /// The soundtrack track `choice` names, from the cache or from a disc.
    ///
    /// Returns the release it came off, the sound, and a line naming it.
    /// `Ok(None)` when the release carries no soundtrack at all.
    ///
    /// # How a track on the *other* disc is found
    ///
    /// The booted release's own order decides which track [`MUSIC_TRACK`] is;
    /// the other release is then reached by **length**, through
    /// [`Soundtrack::nearest`]. That is one rule rather than two, and it is the
    /// only one available: neither archive stores names, and the two orders
    /// differ.
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

        // No fallback to the front end's own music when a release turns out to
        // carry no pairable soundtrack. It would be a *different recording*,
        // and the row's whole claim is that its three values are one recording
        // encoded twice - see [`MusicSource`]. The caller reports the absence.
        let Some(track) = self.locate(discs, platform, source)? else {
            return Ok(None);
        };

        let sound = Arc::new(load_track(source, platform, track, cache_dir)?);
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

    /// Which track of `platform`'s soundtrack [`MUSIC_TRACK`] means.
    ///
    /// On the booted release that is simply its own entry [`MUSIC_TRACK`]. On
    /// the other one it is whichever track is the same length, which is what
    /// makes the two selections the same recording rather than two unrelated
    /// pieces of music.
    ///
    /// **The index is not stable across boots**, and it is worth being plain
    /// about that: [`MUSIC_TRACK`] means "entry 0 of whichever disc booted",
    /// and the two archives are not in the same order. Entry 0 happens to name
    /// the same recording on both - the PS2's first track is also the first of
    /// the PSP's sixteen in `Data.wad` order - but that is coincidence, and at
    /// index 1 the two boots would start on different music. Nothing depends on
    /// it today because the index is a constant; a future circuit-to-track map
    /// has to be built on one disc's order, not on "index N".
    fn locate(
        &self,
        discs: &MusicDiscs,
        platform: Platform,
        source: &str,
    ) -> Result<Option<Track>> {
        let Some(soundtrack) = Soundtrack::read(source, platform)? else {
            return Ok(None);
        };
        if discs.booted() == Some(platform) {
            return Ok(soundtrack.tracks.get(MUSIC_TRACK).copied());
        }

        let Some((booted_source, booted_platform)) = discs.pick(MusicSource::Auto) else {
            return Ok(soundtrack.tracks.get(MUSIC_TRACK).copied());
        };
        let Some(booted) = Soundtrack::read(booted_source, booted_platform)? else {
            return Ok(soundtrack.tracks.get(MUSIC_TRACK).copied());
        };
        let Some(wanted) = booted.tracks.get(MUSIC_TRACK) else {
            return Ok(None);
        };
        Ok(soundtrack.nearest(wanted.seconds))
    }

    /// Starts one of the boot sequence's movie sounds, reporting what happened.
    ///
    /// The seam both tick loops go through - the window's and the headless
    /// capture's - so that the line on stdout, and the decision itself, cannot
    /// come out differently on one of them. `None` is the ordinary silent case
    /// and says so once; see [`crate::boot::Boot::movie_sound`] for the reasons
    /// it is `None`.
    ///
    /// `what` names the movie, because a boot can start more than one of them:
    /// Pure plays a second movie when `FMV Intro` is entered, and a report that
    /// called both "the intro movie" would make the handover between them
    /// invisible in exactly the place it needs to be visible.
    pub fn start_boot_movie(&mut self, what: &str, sound: Option<crate::at3::Pcm>) {
        let Some(pcm) = sound else {
            println!("audio: {what} plays silently");
            return;
        };
        let seconds = pcm.samples.len() as f64
            / f64::from(pcm.channels.max(1))
            / f64::from(pcm.sample_rate.max(1));
        match Sound::new(pcm.samples, pcm.channels, pcm.sample_rate) {
            Ok(sound) => {
                if self.start_movie(sound) {
                    println!("audio: {what}'s own track, {seconds:.2} s, clocking the picture");
                } else {
                    // A mixer with every slot busy, which cannot happen today -
                    // the music is the only other voice - but is reported rather
                    // than leaving a movie silent for no stated reason.
                    println!("audio: no free voice for {what}, so it plays silently");
                }
            }
            Err(error) => println!("audio: {what} plays silently ({error:#})"),
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
/// the entry asked for. [`oag_assets::read_loose_file`] would have been
/// the obvious call and reads a loose file whole; that is right for a movie
/// container and wrong by a factor of sixteen here.
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

/// Reads the archive's directory, and only its directory.
///
/// Four bytes to learn the entry count, then exactly the table those four bytes
/// size. The payload behind it is 559 MiB and nothing here wants any of it.
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
/// The two discs share nothing that could address a track: the PS2 keys its
/// archive by position and the PSP keys `Data.wad` by name hash, no name for
/// any of the sixteen has been recovered on either side, and the two orders are
/// **not** the same - PS2 track 12 is the second of the PSP's sixteen in
/// `Data.wad` order. What they do share is length, to within 11 milliseconds
/// over all sixteen, which is what `docs/formats/ps2-audio.md` establishes and
/// what [`Self::nearest`] pairs on.
#[derive(Debug, Clone)]
struct Soundtrack {
    /// The tracks, in the disc's own order. [`Track::at`] is addressed the way
    /// the release they came off addresses one.
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
/// The measured gap is **11.4 milliseconds, the same to a tenth of a
/// millisecond on all sixteen pairs** - the ATRAC3+ encoder's trailing padding,
/// which is a constant. A second of tolerance is therefore enormously more than
/// the match needs and still far less than the gap to the nearest wrong answer:
/// the sixteen tracks are 177 to 205 seconds long and the closest two of them
/// are 0.13 seconds apart, so a mispairing would have to be off by a hundred
/// times the observed error before this let it through.
///
/// It is not enormous compared to *another game*, and that is worth being
/// plain about: Wipeout Pure's shortest soundtrack track is 205.2 s and
/// Pulse's longest is 204.3 s, 0.87 s apart and so inside this. One track
/// matching by luck is exactly why [`Soundtrack::pairs_with`] demands a
/// complete one-for-one assignment rather than a match, and why it checks the
/// count first - Pure has nineteen tracks, Pulse sixteen.
const PAIR_TOLERANCE: f64 = 1.0;

/// The smallest a `Data.wad` entry can be and still hold a soundtrack track.
///
/// **Arithmetic, not a round number picked by eye.** The shortest of the
/// sixteen PS2 tracks is 177.2 seconds; at 44,100 Hz and ATRAC3+'s 560 bytes
/// per 2,048 samples that is `177.2 * 44100 / 2048 * 560` = 2,137,000 bytes of
/// stored stream, so nothing shorter than about 2 MiB can be one of them. It is
/// a prefilter and not the test - [`psp_soundtrack`] still checks the codec,
/// the channel count and the rate - and it exists because applying those checks
/// to all 1,142 entries would mean 1,142 reads off a disc image where 61 will
/// do.
const MIN_SOUNDTRACK_BYTES: u32 = 2_000_000;

/// Bytes of each candidate entry read to find its `fmt ` and `fact` chunks.
///
/// Both sit in front of `data` on every entry the disc carries, within the
/// first 100 bytes; a kibibyte is slack for an entry that orders its chunks
/// differently, and it is what stops this reading 2 MiB per candidate.
const RIFF_HEADER_PEEK: u64 = 1024;

impl Soundtrack {
    /// Reads the soundtrack `source` carries, for the release it is.
    ///
    /// `Ok(None)` when the source has none - a partially extracted directory,
    /// or a disc of the other release - which is an ordinary outcome rather
    /// than an error.
    ///
    /// # Errors
    ///
    /// An archive that opens and then will not parse.
    fn read(source: &str, platform: Platform) -> Result<Option<Self>> {
        let tracks = match platform {
            Platform::Ps2 => ps2_soundtrack(source)?,
            Platform::Psp => psp_soundtrack(source)?,
            Platform::Unknown => None,
        };
        Ok(tracks.map(|tracks| Self { tracks }))
    }

    /// Whether this is the same soundtrack as `other`, one track for one
    /// track.
    ///
    /// Sixteen against sixteen, every track within [`PAIR_TOLERANCE`] of a
    /// **distinct** partner. The distinctness is the half that does the work:
    /// a listing of sixteen roughly-three-minute tracks from a *different*
    /// game has tracks near some of these by luck, but a complete one-for-one
    /// assignment across all sixteen is not something two unrelated albums do.
    /// Measured on the real pair, every partner is within 11.4 ms and the
    /// closest wrong answer is 130 ms away.
    ///
    /// This is what stops another Studio Liverpool UMD being taken for the
    /// counterpart - see [`MusicDiscs`], where the case is not hypothetical.
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

    /// The track nearest `seconds` long, if one is close enough to be it.
    ///
    /// `None` rather than the least-bad answer when nothing is within
    /// [`PAIR_TOLERANCE`]: a source whose soundtrack does not pair is one this
    /// has misidentified, and playing the wrong three minutes of music is a
    /// worse outcome than playing none.
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
/// Each entry's length comes from its stored size, because the payload is
/// uncompressed PCM of a known geometry - `size / (4 * 48000)` seconds. See
/// `docs/formats/ps2-audio.md` for how the frame size and the rate were
/// established.
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

/// The soundtrack entries of a PSP `Data.wad`, by name hash.
///
/// The population is picked out by what the entries *are* rather than by where
/// they sit: stereo ATRAC3+ at 44,100 Hz, over [`MIN_SOUNDTRACK_BYTES`]. On
/// both PSP pressings that is exactly sixteen entries, matching the PS2
/// archive's sixteen - the channel count is what carries it, because the disc
/// also holds 32 *mono* ATRAC3+ streams at the same bitrate and a dozen shorter
/// stereo ones. This is the same population argument `docs/formats/ps2-voice.md`
/// makes for the pre-race clips, and for the same reason: a size filter alone
/// gets it wrong in both directions.
///
/// The length comes from the `fact` chunk, never from the stored size: ATRAC3+
/// pads its last block, so block count times samples per block overstates a
/// track by a few hundred samples and the pairing tolerance would be spent on
/// an avoidable error.
fn psp_soundtrack(source: &str) -> Result<Option<Vec<Track>>> {
    // Resolved through the layout rather than a literal `PSP_GAME/USRDIR/...`
    // path, so a directory somebody extracted with `oag-unpack` answers the
    // same as a disc image does.
    let Ok(mut archives) = oag_pulse::open(source) else {
        return Ok(None);
    };
    if archives.layout.platform != Platform::Psp {
        return Ok(None);
    }

    let candidates: Vec<(usize, u32)> = archives
        .data
        .directory()
        .entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.size >= MIN_SOUNDTRACK_BYTES)
        .map(|(index, entry)| (index, entry.name_hash))
        .collect();

    let mut tracks = Vec::new();
    for (index, name_hash) in candidates {
        let Ok(header) = archives.data.peek(index, RIFF_HEADER_PEEK) else {
            continue;
        };
        let Ok(stream) = crate::at3::describe(&header) else {
            continue;
        };
        let Some(seconds) = stream.seconds() else {
            continue;
        };
        if stream.format.channels != 2 || stream.format.sample_rate != 44_100 {
            continue;
        }
        tracks.push(Track {
            at: name_hash,
            seconds,
        });
    }
    Ok(Some(tracks))
}

/// Reads one soundtrack track off `source` and turns it into a sound.
///
/// # Errors
///
/// An archive that will not open or read, and on the PSP a decode that failed -
/// including `ffmpeg` being absent, which the caller reports rather than
/// treating as fatal.
fn load_track(source: &str, platform: Platform, track: Track, cache_dir: &Path) -> Result<Sound> {
    match platform {
        Platform::Ps2 => load_ps2_track(source, track.at as usize)?
            .with_context(|| format!("{source} carries no {PS2_MUSIC_PATH}")),
        _ => load_psp_entry(source, track.at, cache_dir),
    }
}

/// Reads one ATRAC3+ soundtrack entry out of `Data.wad` by name hash and
/// decodes it.
///
/// The whole entry is read rather than ranged, unlike the PS2 path: 2.2 MiB of
/// ATRAC3+ is a moment's work and the whole thing has to go to `ffmpeg` anyway.
/// It is the *decoded* form that is large - 33 MiB of PCM for three minutes -
/// and that is what the cache in [`crate::at3`] exists to avoid paying twice.
fn load_psp_entry(source: &str, name_hash: u32, cache_dir: &Path) -> Result<Sound> {
    let mut archives = oag_pulse::open(source).with_context(|| format!("opening {source}"))?;
    let at3 = archives
        .data
        .read_hash(name_hash)
        .with_context(|| format!("reading Data.wad entry {name_hash:08x}"))?;
    let pcm = crate::at3::decode(&at3, cache_dir)
        .with_context(|| format!("decoding Data.wad entry {name_hash:08x}"))?;
    Sound::new(pcm.samples, pcm.channels, pcm.sample_rate)
        .with_context(|| format!("Data.wad entry {name_hash:08x}"))
}

/// The music the front end plays, for a source that has its own.
///
/// The PSP's front-end music and nothing else, which is what
/// [`Audio::start_music`] wants for every source that is not the PS2 release.
/// It is deliberately **not** routed through [`Audio::fetch`]: what comes back
/// is not one of the sixteen, it has no counterpart on the other disc, and the
/// caller records that by leaving [`Audio::music_from`] unset.
fn front_end_music(discs: &MusicDiscs, cache_dir: &Path) -> Result<Option<Loaded>> {
    let Some((source, _)) = discs.pick(MusicSource::Auto) else {
        return Ok(None);
    };
    Ok(load_psp_track(source, cache_dir)?.map(|sound| Loaded {
        // **Unstamped, and that is the whole of the row's scope.** This track
        // has no counterpart on the other disc, so nothing may swap it.
        from: None,
        sound: Arc::new(sound),
        what: PSP_MUSIC_NAME.to_string(),
    }))
}

/// Reads the PSP front end's music out of `Data.wad` and decodes it.
///
/// **Not one of the sixteen soundtrack tracks, and that is the point.** It is a
/// *named* entry where the sixteen are not - the executable builds this path at
/// run time - and no PS2 entry has ever been matched to it, which is why
/// [`MusicSource`] cannot reach it.
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
    let Ok(mut archives) = oag_pulse::open(source) else {
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

    #[test]
    fn a_music_source_round_trips_through_its_own_text() {
        for value in MusicSource::ALL {
            assert_eq!(value.to_string().parse::<MusicSource>(), Ok(value));
        }
        assert_eq!("PS2".parse::<MusicSource>(), Ok(MusicSource::Ps2));
        assert!("umd".parse::<MusicSource>().is_err());
        assert_eq!(MusicSource::default(), MusicSource::Auto);
        assert_eq!(MusicSource::Auto.platform(), None, "auto names no release");
    }

    /// The row is offered only when both discs are reachable, and `pick` is
    /// what a value the machine cannot honour falls through: a settings file
    /// saying `ps2`, carried onto a machine that has only the PSP disc, has to
    /// play the PSP's music rather than nothing.
    #[test]
    fn a_release_that_is_not_there_falls_back_to_the_booted_one() {
        let psp_only = MusicDiscs {
            psp: Some("psp.chd".into()),
            ps2: None,
            booted: Some(Platform::Psp),
        };
        assert!(!psp_only.both(), "one disc is not a choice");
        for choice in MusicSource::ALL {
            assert_eq!(
                psp_only.pick(choice),
                Some(("psp.chd", Platform::Psp)),
                "{choice} on a machine with only the PSP disc"
            );
        }

        let both = MusicDiscs {
            psp: Some("psp.chd".into()),
            ps2: Some("ps2.chd".into()),
            booted: Some(Platform::Psp),
        };
        assert!(both.both());
        assert_eq!(
            both.pick(MusicSource::Auto),
            Some(("psp.chd", Platform::Psp)),
            "auto is the booted disc, not the better one"
        );
        assert_eq!(
            both.pick(MusicSource::Ps2),
            Some(("ps2.chd", Platform::Ps2))
        );
        assert_eq!(
            both.pick(MusicSource::Psp),
            Some(("psp.chd", Platform::Psp))
        );

        // A source that is neither release - an extracted directory of
        // something else - has nothing to fall back to and says so.
        assert_eq!(MusicDiscs::default().pick(MusicSource::Auto), None);
    }

    /// Track lengths as a listing, addressed by position - a test fixture, not
    /// how a disc addresses one.
    fn lengths(seconds: &[f64]) -> Vec<Track> {
        seconds
            .iter()
            .enumerate()
            .map(|(index, seconds)| Track {
                at: index as u32,
                seconds: *seconds,
            })
            .collect()
    }

    /// The pairing rule, on the real numbers. These sixteen lengths are the
    /// EU PS2 archive's, in its own order, and the PSP lengths are its sixteen
    /// large stereo `Data.wad` entries in *theirs* - which is a different order,
    /// and the whole reason a length is what pairs them.
    #[test]
    fn the_two_discs_soundtracks_pair_one_for_one_by_length() {
        let ps2 = [
            187.592, 195.789, 188.739, 193.550, 189.731, 187.814, 182.009, 177.226, 183.913,
            202.632, 200.624, 204.347, 194.013, 183.440, 182.143, 197.474,
        ];
        let psp = [
            187.581, 194.001, 188.728, 195.778, 193.538, 189.719, 187.803, 181.998, 177.215,
            183.902, 202.620, 200.612, 204.336, 183.429, 182.132, 197.462,
        ];
        let listing = Soundtrack {
            tracks: psp
                .iter()
                .enumerate()
                .map(|(index, seconds)| Track {
                    at: index as u32,
                    seconds: *seconds,
                })
                .collect(),
        };

        let mut matched: Vec<u32> = ps2
            .iter()
            .map(|seconds| {
                listing
                    .nearest(*seconds)
                    .unwrap_or_else(|| panic!("{seconds} s has no partner"))
                    .at
            })
            .collect();
        matched.sort_unstable();
        assert_eq!(
            matched,
            (0..16).collect::<Vec<u32>>(),
            "every PSP track must be claimed exactly once"
        );

        // And the pairing is not the identity, which is the thing that would
        // make indexing one archive by the other's order silently wrong: PS2
        // track 1 is the PSP's fourth in `Data.wad` order.
        assert_eq!(listing.nearest(ps2[1]).expect("a partner").at, 3);
        assert_eq!(listing.nearest(ps2[12]).expect("a partner").at, 1);
    }

    /// A *different game's* soundtrack must not be taken for the counterpart,
    /// and this is the case that made the check necessary rather than
    /// hypothetical: `pure-psp-eu.chd` carries the serial `UCES-00001`, which
    /// `oag_assets::Layout::resolve` gives no verdict on by design, and
    /// was reported as the PSP counterpart until a soundtrack had to pair.
    ///
    /// Both listings are read values - Pulse's sixteen from the USA UMD, Pure's
    /// nineteen from `pure-psp-eu.chd`, each the `fact` count over 44,100. Note
    /// how close the two populations come: Pure's shortest is 205.2 s and
    /// Pulse's longest 204.3 s, which is *inside* [`PAIR_TOLERANCE`]. One track
    /// matching is not the test; sixteen distinct partners is.
    #[test]
    fn another_games_soundtrack_does_not_pair() {
        let pulse = Soundtrack {
            tracks: lengths(&[
                187.581, 194.001, 188.728, 195.778, 193.538, 189.719, 187.803, 181.998, 177.215,
                183.902, 202.620, 200.612, 204.336, 183.429, 182.132, 197.462,
            ]),
        };
        let pure = Soundtrack {
            tracks: lengths(&[
                217.896, 213.693, 210.884, 205.217, 224.955, 220.667, 208.237, 219.103, 219.011,
                228.984, 209.816, 214.047, 213.862, 215.612, 227.857, 326.078, 213.240, 217.780,
                216.526,
            ]),
        };
        assert!(!pure.pairs_with(&pulse), "nineteen tracks is not sixteen");
        assert!(!pulse.pairs_with(&pure));

        // The near miss the count check catches first, isolated: Pure's
        // shortest against Pulse's longest, 0.87 s apart.
        assert!(
            (205.217f64 - 204.336).abs() < PAIR_TOLERANCE,
            "the two populations really do overlap within the tolerance"
        );

        // The PS2 side of the real pair, which does have to pass. Same
        // recordings, so every partner is within 11.4 ms.
        let ps2 = Soundtrack {
            tracks: lengths(&[
                187.592, 195.789, 188.739, 193.550, 189.731, 187.814, 182.009, 177.226, 183.913,
                202.632, 200.624, 204.347, 194.013, 183.440, 182.143, 197.474,
            ]),
        };
        assert!(ps2.pairs_with(&pulse), "the two Pulse discs must pair");
        assert!(
            pulse.pairs_with(&ps2),
            "and it must not depend on the order"
        );

        // Sixteen tracks that are each within a second of a Pulse track but
        // all of the *same* one: a listing that would pass a per-track match
        // and must fail a bijection.
        let all_alike = Soundtrack {
            tracks: lengths(&[187.6; 16]),
        };
        assert!(!all_alike.pairs_with(&pulse), "distinctness is the test");

        assert!(
            !Soundtrack { tracks: Vec::new() }.pairs_with(&Soundtrack { tracks: Vec::new() }),
            "two empty listings pair with nothing, not with each other"
        );
    }

    /// Nothing within a second is no answer at all, rather than the least bad
    /// one. Playing the wrong three minutes of music is worse than playing
    /// none, and it is what a misidentified population would produce.
    #[test]
    fn a_length_nothing_matches_pairs_with_nothing() {
        let listing = Soundtrack {
            tracks: vec![
                Track {
                    at: 0,
                    seconds: 187.5,
                },
                Track {
                    at: 1,
                    seconds: 204.3,
                },
            ],
        };
        assert!(listing.nearest(28.0).is_none(), "the front end's own music");
        assert_eq!(listing.nearest(187.511).expect("within tolerance").at, 0);
        assert!(
            listing.nearest(186.4).is_none(),
            "1.1 s out is a hundred times the observed error"
        );
    }

    /// **Seek, do not restart** - the row's headline constraint, measured
    /// rather than asserted, and through the real pieces: a real mixer, a real
    /// pair of [`Sound`]s at the two releases' actual rates, pulled by
    /// [`Audio::tick`] at the fixed 60 Hz, swapped by the same
    /// [`Audio::set_music_source`] a keypress calls.
    ///
    /// The two rates are the point. 48,000 frames into the PS2's track is one
    /// second and into the PSP's is 1.088, so a swap that carried *frames*
    /// across would land 8.8% out - two seconds adrift three minutes in, which
    /// is most of a bar. Carrying seconds lands where it started.
    ///
    /// No disc is read: both sounds are put straight into [`Audio::held`],
    /// which is exactly what a second visit to a release finds there.
    #[test]
    fn changing_the_music_source_seeks_rather_than_restarting() {
        // Booted from the PS2 release, because that is the one whose front-end
        // music is a soundtrack track and so the one the row can move. See
        // `MusicSource`, and the test below for the PSP boot.
        let discs = MusicDiscs {
            psp: Some("psp.chd".into()),
            ps2: Some("ps2.chd".into()),
            booted: Some(Platform::Ps2),
        };
        // Three minutes of silence at each release's own rate. What is measured
        // is where the playhead is, and the mixer advances it whatever the
        // samples are.
        let psp = Arc::new(Sound::new(vec![0i16; 180 * 44_100 * 2], 2, 44_100).expect("a sound"));
        let ps2 = Arc::new(Sound::new(vec![0i16; 180 * 48_000 * 2], 2, 48_000).expect("a sound"));

        let mut audio = Audio {
            output: Output::null(DUMP_SAMPLE_RATE),
            dump: Some(Dump {
                path: PathBuf::from("unused"),
                samples: Vec::new(),
            }),
            music: None,
            music_from: None,
            held: vec![
                (Platform::Psp, Arc::clone(&psp)),
                (Platform::Ps2, Arc::clone(&ps2)),
            ],
            movie: None,
        };
        audio.start_music(&discs, MusicSource::Auto, Path::new("unused"));
        assert_eq!(
            audio.music_from,
            Some(Platform::Ps2),
            "auto is the booted release"
        );

        // Twenty seconds in, so a restart is unmistakable against a seek.
        for _ in 0..(60 * 20) {
            audio.tick();
        }
        let before = audio.playhead().expect("music is playing");
        assert!(
            (before - 20.0).abs() < 0.01,
            "expected 20 s of the PS2 track, got {before}"
        );

        audio.set_music_source(&discs, MusicSource::Psp, Path::new("unused"));
        assert_eq!(audio.music_from, Some(Platform::Psp), "it moved");
        let after = audio.playhead().expect("music is still playing");
        assert!(
            (after - before).abs() < 1.0 / 60.0,
            "the playhead moved from {before} s to {after} s; a swap must seek, not restart"
        );

        // And back, from wherever it has got to by then - the return trip is
        // the one that would expose a frame count carried across, because the
        // rate ratio inverts.
        for _ in 0..(60 * 5) {
            audio.tick();
        }
        let before = audio.playhead().expect("music is playing");
        audio.set_music_source(&discs, MusicSource::Ps2, Path::new("unused"));
        let after = audio.playhead().expect("music is still playing");
        assert!(
            (after - before).abs() < 1.0 / 60.0,
            "coming back: {before} s became {after} s"
        );

        // Choosing what is already playing does nothing at all, so nudging the
        // row past a value it is already on cannot restart the track.
        let before = audio.playhead().expect("music is playing");
        let voice = audio.music;
        audio.set_music_source(&discs, MusicSource::Auto, Path::new("unused"));
        assert_eq!(audio.music, voice, "the same voice, untouched");
        assert_eq!(audio.playhead(), Some(before));
    }

    /// **The scope of the row, and the reason it has one.** The PSP front
    /// end's own music is not one of the sixteen and has no counterpart on the
    /// PS2 disc, so no value of MUSIC SOURCE may touch it - not even to the
    /// release it already is. Left ungoverned it would be swapped for an
    /// unrelated three-minute soundtrack track *and seeked into*, landing
    /// twenty seconds inside a different piece of music.
    ///
    /// A voice with no [`Audio::music_from`] is exactly that case, and the
    /// assertion here is that all three values leave it alone: the same voice,
    /// at the same playhead, on the same 28-second loop.
    #[test]
    fn music_with_no_counterpart_is_left_alone_whatever_the_row_says() {
        let discs = MusicDiscs {
            psp: Some("psp.chd".into()),
            ps2: Some("ps2.chd".into()),
            booted: Some(Platform::Psp),
        };
        // The PSP front end's own music: 28 seconds, not three minutes, and
        // never stamped with a release.
        let front_end =
            Arc::new(Sound::new(vec![0i16; 28 * 44_100 * 2], 2, 44_100).expect("a sound"));
        let ps2 = Arc::new(Sound::new(vec![0i16; 180 * 48_000 * 2], 2, 48_000).expect("a sound"));

        let mut audio = Audio {
            output: Output::null(DUMP_SAMPLE_RATE),
            dump: Some(Dump {
                path: PathBuf::from("unused"),
                samples: Vec::new(),
            }),
            music: None,
            music_from: None,
            // The PS2 track is *there to be chosen* and still must not be, so
            // this cannot pass by the swap merely failing to find anything.
            held: vec![(Platform::Ps2, Arc::clone(&ps2))],
            movie: None,
        };
        audio.music = audio
            .output
            .with_mixer(|mixer| mixer.play(Play::looping(Arc::clone(&front_end), Bus::Music)));
        assert!(audio.music.is_some(), "a free voice");
        assert_eq!(audio.music_from, None, "not one of the sixteen");

        for _ in 0..(60 * 5) {
            audio.tick();
        }
        let voice = audio.music;
        let before = audio.playhead().expect("music is playing");

        for choice in MusicSource::ALL {
            audio.set_music_source(&discs, choice, Path::new("unused"));
            assert_eq!(
                audio.music, voice,
                "{choice} restarted the front end's music"
            );
            assert_eq!(
                audio.playhead(),
                Some(before),
                "{choice} moved the playhead"
            );
            assert_eq!(
                audio.music_from, None,
                "{choice} claimed it as a soundtrack"
            );
        }
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
            music_from: None,
            held: Vec::new(),
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
            music_from: None,
            held: Vec::new(),
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
            music_from: None,
            held: Vec::new(),
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
            music_from: None,
            held: Vec::new(),
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
            music_from: None,
            held: Vec::new(),
            movie: None,
        };
        for _ in 0..120 {
            audio.tick();
        }
        assert!(audio.dump.is_none());
    }
}
