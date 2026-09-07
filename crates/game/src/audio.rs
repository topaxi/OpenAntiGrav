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
use log::{info, warn};
use oag_audio::mixer::MUSIC_MASTER_TRIM;
use oag_audio::{Bus, Output, Play, Sound, VoiceId};
use oag_disc::{DiscImage, Platform};
use oag_formats::ps2_music;
use serde::{Deserialize, Serialize};

use crate::display::percentage;

pub mod sfx;

mod race_music;
pub use race_music::MusicFetchWorker;
use race_music::{PendingSwitch, locate};

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

/// Which track the menu plays, by index into the booted release's own
/// soundtrack.
///
/// The first, because nothing yet maps a circuit or a menu to a track. **Both
/// titles' names have since been recovered** - each disc's own plugin
/// definition declares one `PI_Music` node per track - but a name is not a
/// mapping, and which of them a given menu or circuit plays is still
/// unestablished. Playing a fixed one proves the path end to end without
/// claiming otherwise.
///
/// What the names did buy is that "the first" now means the *release's* first
/// wherever [`crate::music`] reads the declaration: see
/// [`oag_title::DeclaredTracks`].
///
/// **Not the only index in play any more.** [`Audio::race_index`] addresses
/// the same booted-disc order for the race playlist below, starting one past
/// this one - see [`Audio::initial_race_index`]. Both are read and decoded
/// through the same [`fetch`](Audio::fetch)/[`locate`](Audio::locate), now
/// parameterised on an index rather than closing over this constant.
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
    /// The booted title's PSP source, if one was found.
    psp: Option<String>,
    /// The booted title's PS2 source, if one was found.
    ps2: Option<String>,
    /// The booted title's PS3 source, if one was found.
    ///
    /// **Never paired against anything**, unlike the two above: Wipeout HD has
    /// one release, so this is only ever the disc the game booted from and
    /// [`MusicDiscs::both`] is false whenever it is set.
    ps3: Option<String>,
    /// Which release the game booted from, when it is one of the three.
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
        // **As whichever title it is**, not as Pulse. Pulse's own deny-list
        // names both Pure pressings, so resolving against `oag_pulse::TITLE`
        // used to return `WrongTitle` here and leave a Pure boot with no
        // platform, no listing, and silence under its menus.
        let Some((title, layout)) = identify(booted) else {
            return discs;
        };
        discs.booted = Some(layout.platform);
        match layout.platform {
            Platform::Psp => discs.psp = Some(booted.to_string()),
            Platform::Ps2 => discs.ps2 = Some(booted.to_string()),
            // **Recorded and then done with.** A PS3 disc has music and no
            // counterpart to pair it against - there is no second release of
            // Wipeout HD - so the search below is skipped rather than run and
            // found empty, and MUSIC SOURCE is never offered.
            Platform::Ps3 => {
                discs.ps3 = Some(booted.to_string());
                return discs;
            }
            // **The same "recorded and then done with" as PS3's, one step
            // earlier**: Wipeout 2048's music is unlocated, so there is not
            // even a soundtrack to pair, and there is no second release of it
            // to pair against either.
            Platform::Vita | Platform::Unknown => return discs,
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
        let Some(found) = find_release(booted, title, wanted, &mine) else {
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
        match (&self.psp, &self.ps2, &self.ps3) {
            (Some(psp), Some(ps2), _) => format!("PSP {psp} and PS2 {ps2}"),
            (Some(psp), None, _) => format!("PSP {psp} only"),
            (None, Some(ps2), _) => format!("PS2 {ps2} only"),
            (None, None, Some(ps3)) => format!("PS3 {ps3} only"),
            (None, None, None) => "neither release".to_string(),
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
            Platform::Ps3 => self.ps3.as_deref().map(|at| (at, Platform::Ps3)),
            Platform::Vita | Platform::Unknown => None,
        })
    }
}

/// Which title `source` is, and how its archives are laid out.
///
/// The one place this module asks the question, so that the answer cannot
/// disagree with itself between the survey and the search. `None` for every
/// source that will not open as any title this build knows.
fn identify(source: &str) -> Option<(&'static oag_title::Title, oag_assets::Layout)> {
    let opened = crate::title::open_source(source, Vec::new(), Vec::new()).ok()?;
    Some((opened.title, opened.archives.layout))
}

/// The first source on the search path carrying `wanted`'s encode of `mine`.
///
/// Every container in every directory [`crate::source::search_path`] lists is
/// tried, in that order. Identifying a candidate is only the **prefilter** - it
/// says which title and platform a disc is and skips one that carries no
/// archives at all - and [`Soundtrack::pairs_with`] is the test that decides,
/// for the reason [`MusicDiscs`] records at length: a serial cannot rule a disc
/// *in*, and a counterpart nobody named has to be ruled in rather than merely
/// not ruled out.
///
/// **A different title is refused outright**, before the pairing test rather
/// than by it. The pairing test would refuse Pulse's sixteen against Pure's
/// nineteen on the count alone, but that is a property of these two discs and
/// not a rule: two titles with equally many tracks of similar length would slip
/// through it. `title` costs nothing and does not depend on how the corpus
/// grows.
fn find_release(
    booted: &str,
    title: &'static oag_title::Title,
    wanted: Platform,
    mine: &Soundtrack,
) -> Option<String> {
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
            let Some((theirs, layout)) = identify(&source) else {
                continue;
            };
            if theirs.name != title.name || layout.platform != wanted {
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
    /// Whether [`Audio::start_music`] has already run, whatever it decided.
    ///
    /// Separate from [`Self::music`] being `Some` because the two tick loops
    /// ask **every tick** - the music's cue is the intro's end, and neither
    /// loop tracks an edge - and a source with no decodable music leaves
    /// `music` at `None` for ever. Guarding on `music` alone would then re-read
    /// a disc, re-run `ffmpeg` and re-print the failure sixty times a second.
    music_attempted: bool,
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
    ///
    /// **Safe to leave unbounded only because the menu path never asks for
    /// more than one index** ([`MUSIC_TRACK`], always). [`Self::race_cache`]
    /// is the counterpart for a consumer that does not have that guarantee.
    held: Vec<(Platform, Arc<Sound>)>,
    /// The movie's own sound, while a movie is playing one.
    ///
    /// Separate from [`Self::music`] although both are on [`Bus::Music`],
    /// because they are stopped at different moments and by different things:
    /// the music runs for the whole session and this lasts one movie. See
    /// [`Audio::start_movie`].
    movie: Option<VoiceId>,
    /// The voice playing under a race, distinct from [`Self::music`] (the
    /// menu's) although both live on [`Bus::Music`] and never sound together -
    /// [`Audio::start_race_music`] stops one before starting the other. `None`
    /// outside a race, or inside one whose source carries no decodable music.
    ///
    /// **Authored, not recovered.** Nothing in the reverse-engineering record
    /// says the original cycles a race playlist - the circuit-to-track mapping
    /// is still unestablished (`docs/formats/ps2-audio.md`). This is a
    /// deliberate choice for this reimplementation, the same way
    /// `[graphics] boost_fov_kick` is: no confidence
    /// score, no `names.tsv` row.
    race_voice: Option<VoiceId>,
    /// Which release the race voice came off, mirroring [`Self::music_from`].
    race_from: Option<Platform>,
    /// Which track of the **booted** disc's own soundtrack order the race
    /// playlist is on. `None` until the first race of the process starts one -
    /// see [`Audio::initial_race_index`]. Never reset by a visit to the menus:
    /// that persistence, for the life of the process and no further, is the
    /// whole of what the feature asks for.
    race_index: Option<usize>,
    /// Where the race playlist's current track was cut off, in the track's
    /// own seconds - [`oag_audio::Mixer::seek`]'s unit, for the same reason
    /// [`Self::set_music_source`] carries a playhead in seconds rather than
    /// frames - so the next race resumes rather than restarts. Zero until
    /// something has actually been paused.
    race_position: f64,
    /// The one race track presently decoded.
    ///
    /// Replaced, never accumulated, the moment the playlist advances or
    /// [`MusicSource`] moves it while a race is live. Unlike [`Self::held`],
    /// which exists to make the settings row cheap to nudge back and forth
    /// across a *fixed* index, this exists only to make one pause/resume
    /// cycle on the *same* track free - a single slot bounds it at one
    /// track's worth of PCM (33-36 MiB) rather than growing with however many
    /// of the sixteen tracks a long session has cycled through.
    race_cache: Option<(Platform, usize, Arc<Sound>)>,
    /// The menu/front-end voice's own decoded sound, kept so leaving a race
    /// can resume it without re-reading the disc or re-running `ffmpeg`. Set
    /// whenever [`Audio::start_music`] or [`Audio::set_music_source`]
    /// successfully loads one; `None` when this source carries no decodable
    /// menu music at all.
    menu_sound: Option<Arc<Sound>>,
    /// The race's held voices and its own generator, while a race is running.
    ///
    /// `None` outside a race and on a source whose banks did not load. Made on
    /// the first tick that has cues to play rather than at race start, so a
    /// silent source never allocates one.
    sfx: Option<sfx::SfxVoices>,
    /// What [`Audio::start_race_music`] was last called with, kept so the
    /// advance-on-finish check in [`Audio::tick`] can fetch the next track on
    /// its own - `tick` runs every simulation tick from several call sites,
    /// and threading `discs`/`choice`/`cache_dir` through every one of them
    /// for the sake of one internal fetch would be a wider signature change
    /// than the feature needs. `None` outside a race.
    race_context: Option<(MusicDiscs, MusicSource, PathBuf)>,
    /// The **next** race track, being fetched ahead of the current one ending
    /// - the track index it targets, alongside the worker fetching it.
    ///
    /// **This is what keeps a track boundary - or a future skip control -
    /// from hitching**, the same way [`MusicFetchWorker`] keeps the race
    /// launch itself from hitching: [`Self::advance_race_track`] used to
    /// call [`Self::play_race_track`] synchronously the moment the current
    /// track ended, decoding the next one - 0.4 to 2.8 s by this module's own
    /// measurements - inside the fixed-timestep loop every time the playlist
    /// moved on. Started as soon as the current track is playing instead, by
    /// [`Self::maybe_prefetch_next_race_track`] from [`Self::tick`] - see its
    /// own doc for why "immediately" and not "a few seconds before the end" -
    /// so by the time anything ends the current track early, natural or not,
    /// the fetch has almost always already landed and
    /// [`Self::advance_race_track`] only ever joins it rather than starting
    /// it from nothing.
    ///
    /// The index is carried alongside the worker, not inferred from
    /// [`Self::race_index`] at the point of use: [`MusicSource`] can move the
    /// race voice mid-track, which changes what "the next track" means
    /// without touching the index, and a prefetch answering the wrong
    /// question would be worse than none - [`Self::advance_race_track`]
    /// checks the two still agree before trusting it, and
    /// [`Self::set_race_music_source`] drops a stale one outright rather than
    /// let it survive a source change it was not fetched for.
    race_prefetch: Option<(usize, MusicFetchWorker)>,
    /// A `MUSIC SOURCE` row switch presently being fetched off the tick
    /// thread - see [`Self::set_music_source`]. `None` when the row is not
    /// mid-switch.
    ///
    /// At most one in flight: a fresh request before this lands replaces it
    /// rather than queuing behind it, the same as [`Self::race_prefetch`]
    /// does for a stale prefetch - the detaching worker's own result, if it
    /// lands late, is simply nobody's.
    source_switch: Option<PendingSwitch>,
}

/// A music track that has been read and decoded, before a voice is started on
/// it.
///
/// [`Loaded::from`] is the field that matters and the reason this is a struct
/// rather than a tuple: it carries whether what was loaded is one of the
/// sixteen soundtrack tracks, which is what decides whether MUSIC SOURCE may
/// ever move it. See [`MusicSource`].
///
/// `pub` - not its fields - so [`MusicFetchWorker::join`] can hand one back
/// across the crate's own lib/bin boundary, the way [`crate::race::Loaded`]
/// already does for the circuit's own worker. Nothing outside this module
/// needs to read a field; it only ever moves one straight into
/// [`Audio::finish_race_music`].
pub struct Loaded {
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
    /// `dump` names the WAV `--dump-audio` should write, and when it is set the
    /// device is **not** opened at all. That is not a convenience: with a
    /// stream attached the `cpal` callback drains the mixer from its own
    /// thread, [`Output::render_tick`] returns zero rather than racing it, and
    /// the dump would be a file of silence written next to audio the player
    /// could hear. Forcing the null backend makes the two mutually exclusive
    /// where they would otherwise be quietly wrong.
    ///
    /// `no_audio` is `--no-audio`: the same forced null backend as a dump, but
    /// with nothing collected to write. It exists for the case a dump does not
    /// cover - a capture that wants the deterministic tick-clocked path (see
    /// [`Self::movie_playhead`]) without also paying for a growing sample
    /// buffer and a WAV nobody asked for. Without it, a `--screenshot --until`
    /// run on a machine with a real device is "audio-clocked" per ADR-0019,
    /// and a headless run finishes in far less wall-clock time than the movie
    /// takes to play - so a late frame's `--until` target is never reached.
    /// Forcing the null backend puts such a run on the same tick-clocked
    /// fallback ADR-0019 already specifies for a movie with no audio stream,
    /// which is also exactly what a machine with **no** device does today
    /// (that path is what CI exercises).
    #[must_use]
    pub fn open(
        settings: &crate::settings::Audio,
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
    /// game runs and the change should be audible on the row the player is
    /// standing on, the way `BRIGHTNESS` is visible on it.
    ///
    /// **The music bus alone also carries [`MUSIC_MASTER_TRIM`]**, folded in
    /// here rather than into [`crate::audio::Volume::gain`] because it is a
    /// fact about the original's music path specifically - the SFX and
    /// Speech sliders reach `Audio_SetSfxFadeTarget`, a completely separate
    /// chain that never touches this constant. See `MUSIC_MASTER_TRIM`'s own
    /// doc comment for the evidence.
    pub fn apply(&self, settings: &crate::settings::Audio) {
        self.output.with_mixer(|mixer| {
            mixer.set_bus_gain(Bus::Music, settings.music_volume.gain() * MUSIC_MASTER_TRIM);
            mixer.set_bus_gain(Bus::Sfx, settings.sfx_volume.gain());
            mixer.set_bus_gain(Bus::Speech, settings.speech_volume.gain());
            // After both buses, which is the order the original's own chain
            // has: a group volume, then the master the output thread scales
            // by. See `settings::Audio::master_volume`.
            mixer.set_master_gain(settings.master_volume.gain());
        });
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
    /// **Not at boot: the intro's own track comes first.** The reel that
    /// `LogoFMV` plays carries its own sound, and both it and this go on
    /// [`Bus::Music`] - the disc plays one of them at a time, and starting this
    /// while the intro runs mixes the menu loop over the movie. So the front
    /// end's two tick loops call this once the sequence has left a movie state
    /// (`Frontend::is_playing_movie`, which is also what stops the movie's own
    /// voice), whether the intro ended or was skipped. Callers with no
    /// sequence to wait on - a race, a single-screen capture - start it
    /// outright.
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
        // Idempotent by design, not by accident: the front end's two tick loops
        // call this every tick from the moment the intro is over, because that
        // end is the cue and neither loop tracks the edge. See
        // [`Self::music_attempted`] for why the flag rather than `music`.
        if self.music_attempted {
            return;
        }
        self.music_attempted = true;
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
                // Kept so a later `pause_race_music` can restart this exact
                // decode without touching the disc or `ffmpeg` again.
                self.menu_sound = Some(Arc::clone(&loaded.sound));
                self.music = self
                    .output
                    .with_mixer(|mixer| mixer.play(Play::looping(loaded.sound, Bus::Music)));
                self.music_from = self.music.and(loaded.from);
                info!("audio: music {}, {seconds:.1} s, looping", loaded.what);
            }
            Ok(None) => info!("audio: this source carries no music this can play"),
            Err(error) => warn!("audio: no music ({error:#})"),
        }
    }

    /// Starts (or resumes) the race playlist, stopping the menu voice first.
    ///
    /// **Authored, not recovered** - see [`Self::race_voice`]'s doc comment.
    /// Cycles through the booted disc's own soundtrack order, one race track
    /// at a time, in place of the fixed [`MUSIC_TRACK`] the menu always plays.
    ///
    /// The first call in the process picks a starting index through
    /// [`Self::initial_race_index`] and every later one continues from
    /// wherever [`Self::pause_race_music`] left off - [`Self::race_index`] and
    /// [`Self::race_position`] are never reset by this method, which is the
    /// whole of "the track list persists across races".
    ///
    /// **Never fatal**, the same as [`Self::start_music`]: a source with no
    /// decodable race music says so and plays nothing.
    pub fn start_race_music(&mut self, discs: &MusicDiscs, choice: MusicSource, cache_dir: &Path) {
        if let Some(id) = self.music.take() {
            self.output.with_mixer(|mixer| mixer.stop(id));
        }
        // Defensive rather than load-bearing: nothing in `main.rs` calls this
        // while a race voice is already sounding, `Stage::Race` being the only
        // state a race launch is reachable from is what would have to change
        // first. Guards against exactly that assumption quietly breaking.
        if let Some(id) = self.race_voice.take() {
            self.output.with_mixer(|mixer| mixer.stop(id));
        }

        let index = self.reserve_race_music_index(discs);
        let seek = (self.race_position > 0.0).then_some(self.race_position);
        self.race_context = Some((discs.clone(), choice, cache_dir.to_path_buf()));
        self.play_race_track(discs, choice, cache_dir, index, seek);
        // Only consumed once actually applied to a sounding voice - a failed
        // attempt (no `ffmpeg` this one time, a disc that briefly would not
        // open) must leave the saved position for a later, successful call
        // rather than silently discarding it.
        if self.race_voice.is_some() {
            self.race_position = 0.0;
        }
    }

    /// Where the race playlist should start the first time it is asked for.
    ///
    /// One past the menu's own track when the menu is playing one of the
    /// sixteen (a PS2 boot, whose front end plays [`MUSIC_TRACK`] itself) -
    /// so the very first race never restarts the recording the menu was just
    /// playing from 0:00. Otherwise (a PSP boot, whose front-end music is not
    /// one of the sixteen at all, or `--race` with no menu voice to speak of)
    /// simply [`MUSIC_TRACK`] itself.
    ///
    /// **Not silent when the "one past" half fails.** `menu_from.is_some()`
    /// means the disc's own soundtrack was already read once successfully -
    /// by [`Audio::start_music`], to load the menu's own track - so a second
    /// read failing here (a transient open failure; an `image:path` spec that
    /// stopped resolving) is a real anomaly, not the ordinary "this source has
    /// none" case [`Self::booted_soundtrack_len`] also returns `0` for.
    /// Falling back to [`MUSIC_TRACK`] without saying so would restart the
    /// menu's own recording exactly as the un-chosen option would have,
    /// indistinguishable in the load report from the intended behaviour.
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

    /// How many tracks the **booted** disc's own soundtrack lists, or `0` when
    /// it cannot be read at all (a PSP boot, or a source with no soundtrack).
    /// The modulus the race playlist wraps at, and the same order
    /// [`Self::race_index`] and [`Self::initial_race_index`] both address.
    fn booted_soundtrack_len(discs: &MusicDiscs) -> usize {
        let Some((source, platform)) = discs.pick(MusicSource::Auto) else {
            return 0;
        };
        Soundtrack::read(source, platform)
            .ok()
            .flatten()
            .map_or(0, |soundtrack| soundtrack.tracks.len())
    }

    /// Stops the race playlist where it stands and resumes the menu voice.
    ///
    /// The race voice's own position is saved to [`Self::race_position`], in
    /// the track's own seconds, so the next [`Self::start_race_music`] picks
    /// up close to here rather than restarting the track - the "pause" half
    /// of the feature. [`Self::race_index`] is untouched: which track this
    /// was is exactly the state that persists.
    ///
    /// The menu voice restarts from [`Self::menu_sound`] rather than through
    /// [`Self::start_music`], which is idempotent by design and would do
    /// nothing the second time it is asked - see that method's own doc
    /// comment. Silent, not an error, when there is no menu sound to resume
    /// (nothing ever loaded one, e.g. no `ffmpeg` on a PSP source).
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

    /// How far into its own track the race playlist has got, in seconds - the
    /// race counterpart of [`Self::playhead`].
    #[must_use]
    pub fn playhead_race(&self) -> Option<f64> {
        self.output
            .with_mixer(|mixer| mixer.position(self.race_voice?))
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
    ///
    /// **Moves whichever voice is actually live.** While a race is playing its
    /// own track from [`Self::race_index`], that is the voice this row means -
    /// otherwise it is the menu's, exactly as before this method learned about
    /// races at all. Reaching for the *menu's* stamped `music_from` while a
    /// race track from a different release is sounding would let the two
    /// desync: the row would claim a platform the audible voice was not on, and
    /// the next [`Self::pause_race_music`] would resume the wrong recording.
    /// See [`Self::set_race_music_source`].
    ///
    /// **Never blocks any more.** A release read for the first time this
    /// session used to decode synchronously on whatever thread called this
    /// (0.4 s for a cached PSP decode, 2.0 s cold for the PS2's 36 MiB of
    /// PCM), and this row is reachable mid-race, with `apply.rs` applying a
    /// settings change on the same thread that runs the tick loop - a freeze
    /// on a player action, not a hypothetical. A release already read this
    /// session (in [`Self::held`]/[`Self::race_cache`]) still applies on the
    /// spot, since that path does no I/O, but a fresh one now goes through a
    /// [`MusicFetchWorker`], the same way [`Self::start_race_music`]'s own
    /// hand-off does, and [`Self::tick`] applies the result once it lands.
    /// See `handover/streaming-decode-for-audio-would-break-seek-and.md`.
    pub fn set_music_source(&mut self, discs: &MusicDiscs, choice: MusicSource, cache_dir: &Path) {
        if self.race_voice.is_some() {
            self.set_race_music_source(discs, choice, cache_dir);
        } else {
            self.set_menu_music_source(discs, choice, cache_dir);
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
        let Some(track) = locate(discs, platform, source, MUSIC_TRACK)? else {
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

    /// [`Self::fetch`]'s race counterpart: any track of the booted disc's
    /// order, cached in the single-slot [`Self::race_cache`] rather than the
    /// unbounded [`Self::held`].
    ///
    /// `held` is safe to leave unbounded because the menu path only ever asks
    /// for [`MUSIC_TRACK`] - at most one entry per platform, ever. The race
    /// playlist asks for a different index every time it advances, and
    /// caching those the same way would grow without limit over a long
    /// session; a single replaced slot is what [`Self::race_cache`]'s own doc
    /// comment costs instead.
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

        let sound = Arc::new(load_track(source, platform, track, cache_dir)?);
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
    ///
    /// # The line printed is not "a voice started", it is "what clocks the picture"
    ///
    /// **Checked against [`Self::movie_playhead`] after starting the voice,
    /// not assumed from the voice alone.** A voice can start - occupy a mixer
    /// slot, decode correctly - on a null backend nothing ever pulls from
    /// (`--no-audio`, or no device and no `--dump-audio`), and on that path
    /// the picture is tick-clocked, not audio-clocked, per ADR-0019. Printing
    /// "clocking the picture" regardless would have been exactly the kind of
    /// misleading capture output that cost two wrong readings before
    /// `--no-audio` existed - see
    /// `handover/until-cannot-reach-a-late-movie-frame-on.md`'s history.
    pub fn start_boot_movie(&mut self, what: &str, sound: Option<crate::at3::Pcm>) {
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
                        info!("audio: {what}'s own track, {seconds:.2} s, clocking the picture");
                    } else {
                        info!(
                            "audio: {what}'s own track, {seconds:.2} s, but nothing pulls from \
                             the mixer - tick-clocked instead"
                        );
                    }
                } else {
                    // A mixer with every slot busy, which cannot happen today -
                    // the music is the only other voice - but is reported rather
                    // than leaving a movie silent for no stated reason.
                    warn!("audio: no free voice for {what}, so it plays silently");
                }
            }
            Err(error) => warn!("audio: {what} plays silently ({error:#})"),
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
        // Before the race-boundary check below: a switch that landed this
        // tick may have just replaced `race_voice` with a freshly-started
        // one, and that voice is playing by construction - nothing left for
        // the boundary check to do with it this tick either way, but polling
        // first keeps the two from racing each other over which one moves
        // `race_voice` on a tick they would otherwise both touch it.
        self.poll_source_switch();
        if let Some(id) = self.race_voice
            && !self.output.with_mixer(|mixer| mixer.is_playing(id))
        {
            self.advance_race_track();
        } else {
            // Only worth checking on the tick the voice is still the one
            // playing: the moment it changes - a track boundary, a `MUSIC
            // SOURCE` toggle - is exactly when a prefetch for what used to be
            // "next" stops meaning anything, and the branch above or
            // `Self::set_race_music_source` is where that gets sorted out.
            self.maybe_prefetch_next_race_track();
        }
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

/// The index one past `index` in a soundtrack of `len` tracks, wrapping.
///
/// A free function, deliberately: [`Audio::booted_soundtrack_len`] is the
/// part that needs a real disc and cannot be exercised without one, but the
/// arithmetic the playlist wraps by needs none of that and is worth pinning
/// on its own. `len == 0` (the soundtrack could not be read at all) leaves
/// `index` where it was rather than dividing by zero - there is nothing to
/// advance *to*.
fn next_race_index(index: usize, len: usize) -> usize {
    if len == 0 { index } else { (index + 1) % len }
}

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
    // established. `as_chunks` drops a trailing odd byte, which
    // `Directory::parse` has already rejected as a partial frame.
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
/// archive by position and the PSP keys `Data.wad` by name hash, and the two
/// orders are **not** the same - PS2 track 12 is the second of the PSP's
/// sixteen in `Data.wad` order. The PSP names have since been recovered from
/// the disc's own plugin definition and the PS2 archive still carries none, so
/// they remain unpairable by name. What they do share is length, to within 11
/// milliseconds over all sixteen, which is what `docs/formats/ps2-audio.md`
/// establishes and what [`Self::nearest`] pairs on.
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
            Platform::Psp | Platform::Ps3 => archived_soundtrack(source)?,
            // Wipeout 2048's music is unlocated - `oag_2048::TITLE` carries
            // `music: None` - so there is nothing here to read yet.
            Platform::Vita | Platform::Unknown => None,
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

/// The soundtrack entries of a source whose music is inside an archive - every
/// release but the PS2 one, which keeps its own loose in the filesystem.
///
/// Which entries those are is the one question here whose answer depends on
/// **which title booted**, so it is asked in [`crate::music`] rather than in
/// this module. [`Track::at`] comes back opaque; see [`crate::music::Entry::at`].
fn archived_soundtrack(source: &str) -> Result<Option<Vec<Track>>> {
    Ok(crate::music::listing(source)?.map(|entries| {
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
/// An archive that will not open or read, and on the PSP a decode that failed -
/// including `ffmpeg` being absent, which the caller reports rather than
/// treating as fatal.
fn load_track(source: &str, platform: Platform, track: Track, cache_dir: &Path) -> Result<Sound> {
    match platform {
        Platform::Ps2 => load_ps2_track(source, track.at as usize)?
            .with_context(|| format!("{source} carries no {PS2_MUSIC_PATH}")),
        _ => crate::music::load_entry(source, track.at, cache_dir),
    }
}

/// The music the front end plays, for a source that has its own.
///
/// The PSP's front-end music and nothing else, which is what
/// [`Audio::start_music`] wants for every source that is not the PS2 release.
/// It is deliberately **not** routed through [`Audio::fetch`]: what comes back
/// is not one of the soundtrack tracks, it has no counterpart on the other
/// disc, and the caller records that by leaving [`Audio::music_from`] unset.
fn front_end_music(discs: &MusicDiscs, cache_dir: &Path) -> Result<Option<Loaded>> {
    let Some((source, _)) = discs.pick(MusicSource::Auto) else {
        return Ok(None);
    };
    Ok(
        crate::music::load_front_end(source, cache_dir)?.map(|(what, sound)| Loaded {
            // **Unstamped, and that is the whole of the row's scope.** This
            // track has no counterpart on the other disc, so nothing may swap
            // it.
            from: None,
            sound: Arc::new(sound),
            what,
        }),
    )
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
mod tests;
