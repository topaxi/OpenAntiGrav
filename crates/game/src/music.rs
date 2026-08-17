//! Which entries of a disc hold music, and how one of them is read.
//!
//! The half of [`crate::audio`] that has to know **which title booted**.
//! Everything else in that module is policy over a listing - what to play, when
//! to seek, which release to prefer - and none of it changes between three
//! Wipeout discs. Finding the listing does, so it lives here.
//!
//! # Two mechanisms, and the second one is not a fallback
//!
//! A title that declares its soundtrack ([`oag_title::DeclaredTracks`]) is read
//! from its own plugin definition: one `PI_Music` node per track, each naming
//! the directory the audio sits in. That is Wipeout Pure and Wipeout HD, whose
//! executables each spell out both halves of the path, and whose nineteen and
//! fifteen declarations all resolve.
//!
//! A title that does not is found by what its entries **are**: stereo ATRAC3+
//! at 44,100 Hz over [`MIN_SOUNDTRACK_BYTES`]. That is Wipeout Pulse today, and
//! it is a measurement rather than a guess - the disc also holds 32 *mono*
//! ATRAC3+ streams at the same bitrate and a dozen shorter stereo ones, so the
//! channel count is what carries it, the same population argument
//! `docs/formats/ps2-voice.md` makes for the pre-race clips. Pulse declares its
//! sixteen too; moving it over is a separate change, for the reason
//! [`oag_title::Music::tracks`] records.
//!
//! The declared route is the better one wherever it is available, because the
//! order it yields is the release's own rather than the archive directory's.
//!
//! # Two containers, told apart by content
//!
//! The PSP titles store music as RIFF-wrapped ATRAC3+ and Wipeout HD stores it
//! as MP3. Nothing here branches on the *title* to decide which: a blob is
//! offered to [`crate::at3`] and then to [`crate::mp3`], and whichever reads it
//! is what it was. That is the same rule `crate::at3`'s own header already
//! states - the content of the bytes decides, not a name or an archive - and it
//! is what keeps a title package to naming files rather than describing them.

use std::path::Path;

use anyhow::{Context, Result, bail};
use oag_assets::Archives;
use oag_audio::Sound;
use oag_disc::Platform;
use oag_title::Title;

use crate::at3::Pcm;

/// The smallest a `Data.wad` entry can be and still hold a soundtrack track.
///
/// **Arithmetic, not a round number picked by eye.** The shortest of the
/// sixteen PS2 tracks is 177.2 seconds; at 44,100 Hz and ATRAC3+'s 560 bytes
/// per 2,048 samples that is `177.2 * 44100 / 2048 * 560` = 2,137,000 bytes of
/// stored stream, so nothing shorter than about 2 MiB can be one of them. It is
/// a prefilter and not the test - [`found`] still checks the codec, the channel
/// count and the rate - and it exists because applying those checks to all
/// 1,142 entries would mean 1,142 reads off a disc image where 61 will do.
const MIN_SOUNDTRACK_BYTES: u32 = 2_000_000;

/// Bytes of each candidate entry read to find its `fmt ` and `fact` chunks.
///
/// Both sit in front of `data` on every entry the disc carries, within the
/// first 100 bytes; a kibibyte is slack for an entry that orders its chunks
/// differently, and it is what stops this reading 2 MiB per candidate.
const RIFF_HEADER_PEEK: u64 = 1024;

/// One soundtrack entry: how to get back to it, and how long it is.
#[derive(Debug, Clone, Copy)]
pub struct Entry {
    /// **An opaque token this module issues and consumes.** It addresses the
    /// entry the way the route that found it addresses one: a `Data.wad` name
    /// hash on the [`found`] route, and an index into the declaration on the
    /// [`declared`] one.
    ///
    /// Opaque rather than a hash for every title because there is nothing one
    /// number could be for all three: a PSARC has no name hash at all. That
    /// makes this the *third* meaning the token already carries -
    /// `crate::audio::load_track` reads it as a `PS2MUSIC.WAD` directory index
    /// on the PS2 path - rather than a new kind of thing.
    ///
    /// `crate::audio` never interprets it. It comes back here through
    /// [`load_entry`].
    pub at: u32,
    /// How long it is, as the stream itself declares.
    pub seconds: f64,
}

/// A source opened as whichever title it turned out to be, for a music read.
///
/// `None` rather than an error for every reason a source is not a disc of a
/// title this knows - a partial extract, a path that will not open. All of them
/// mean "no soundtrack here", which is what the caller does something with;
/// distinguishing them would give it nothing to do differently.
///
/// **The PS2 release is excluded here and not by accident**: its music is loose
/// in the filesystem rather than in an archive, so `crate::audio` reads it
/// through its own `PS2MUSIC.WAD` path and never arrives in this module.
fn open_archived(source: &str) -> Option<(&'static Title, Archives)> {
    // Resolved through the layout rather than a literal `PSP_GAME/USRDIR/...`
    // path, so a directory somebody extracted with `oag-unpack` answers the
    // same as a disc image does.
    let opened = crate::title::open_source(source, Vec::new()).ok()?;
    matches!(
        opened.archives.layout.platform,
        Platform::Psp | Platform::Ps3
    )
    .then_some((opened.title, opened.archives))
}

/// The soundtrack `source` carries, in the order that source's own title
/// decides.
///
/// `Ok(None)` when the source is not a disc of a title this knows, and
/// `Ok(Some(vec![]))` when it is one whose music could not be found at all -
/// two different answers, because the first means "ask something else" and the
/// second means "this disc has none".
///
/// # Errors
///
/// An archive that opens and then will not read its own directory.
pub fn listing(source: &str) -> Result<Option<Vec<Entry>>> {
    let Some((title, mut archives)) = open_archived(source) else {
        return Ok(None);
    };
    match title.music.and_then(|music| music.tracks) {
        Some(tracks) => Ok(Some(self::declared(&mut archives, tracks))),
        None => Ok(Some(found(&mut archives)?)),
    }
}

/// The tracks a title's own plugin definition declares, in file order.
///
/// An entry that is declared and then absent is **skipped rather than
/// reported**: a `PI_Music` node is a declaration, and the same caveat
/// `oag_game::catalogue::all_tracks` records for a partial pack set applies
/// here. The length always comes from the payload's own header, never from the
/// declaration, which carries none.
///
/// [`Entry::at`] is the index into *this* list rather than into the
/// declaration, so a declared-and-absent track does not leave a hole a later
/// [`load_entry`] would fall into.
fn declared(archives: &mut Archives, tracks: oag_title::DeclaredTracks) -> Vec<Entry> {
    let mut out = Vec::new();
    for name in declared_names(archives, tracks) {
        let Some(seconds) = measure(archives, &name) else {
            continue;
        };
        out.push(Entry {
            at: u32::try_from(out.len()).unwrap_or(u32::MAX),
            seconds,
        });
    }
    out
}

/// Every entry name the declaration resolves to, in file order.
///
/// Shared by [`declared`] and [`load_entry`] so that an index means the same
/// thing to both. Reading the definition twice is one small archive entry and
/// costs less than holding a parsed catalogue across the two calls, which are
/// minutes apart in a real session.
fn declared_names(archives: &mut Archives, tracks: oag_title::DeclaredTracks) -> Vec<String> {
    let Ok(definition) = archives.read_name(tracks.declared_in) else {
        return Vec::new();
    };
    let Ok(definition) = String::from_utf8(definition) else {
        return Vec::new();
    };
    crate::catalogue::music(&definition)
        .iter()
        .map(|track| track.entry_name(tracks.file))
        .filter(|name| archives.locate(name).is_some())
        .collect()
}

/// The tracks a title that declares none is found to carry, in `Data.wad`
/// order.
///
/// The population argument the module docs set out. The length comes from the
/// `fact` chunk, never from the stored size: ATRAC3+ pads its last block, so
/// block count times samples per block overstates a track by a few hundred
/// samples and the pairing tolerance would be spent on an avoidable error.
///
/// # Errors
///
/// An archive whose entry directory will not read.
fn found(archives: &mut Archives) -> Result<Vec<Entry>> {
    let data = archives.data.as_wad_mut("the entry directory")?; // WAD-shaped
    let candidates: Vec<(usize, u32)> = data
        .directory()
        .entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| entry.size >= MIN_SOUNDTRACK_BYTES)
        .map(|(index, entry)| (index, entry.name_hash))
        .collect();

    let mut out = Vec::new();
    for (index, name_hash) in candidates {
        let Ok(header) = data.peek(index, RIFF_HEADER_PEEK) else {
            continue;
        };
        let Some(seconds) = riff_seconds(&header) else {
            continue;
        };
        out.push(Entry {
            at: name_hash,
            seconds,
        });
    }
    Ok(out)
}

/// How long the entry called `name` is, or `None` when it is not a stream this
/// can measure.
///
/// **A WAD is peeked and anything else is read whole**, which is the one place
/// the container leaks into this module. A `.wad` entry's header is reachable
/// on its own through the directory, so a Pure listing costs a kibibyte a
/// track; `oag_assets::psarc::Archive` exposes no ranged read, because a PSARC
/// entry is block-compressed and has no offset to seek to, so a Wipeout HD
/// listing reads all fifteen tracks whole - about 75 MiB, and it is done once
/// per boot inside `crate::audio::MusicDiscs::survey`. Measured at well under a
/// second off a local image; if that ever stops being true, a first-block read
/// on `psarc::Archive` is the fix rather than caching the answer.
fn measure(archives: &mut Archives, name: &str) -> Option<f64> {
    if let Ok(data) = archives.data.as_wad_mut("a name")
        && let Some(index) = data
            .directory()
            .entries
            .iter()
            .position(|entry| entry.name_hash == oag_formats::wad::hash_name(name))
        && let Ok(header) = data.peek(index, RIFF_HEADER_PEEK)
        && let Some(seconds) = riff_seconds(&header)
    {
        return Some(seconds);
    }
    seconds_of(&archives.read_name(name).ok()?)
}

/// How long a stream is, whichever of the two containers it is in.
///
/// The dispatch the module header describes: RIFF first because that is what a
/// peeked header is, MPEG second.
fn seconds_of(blob: &[u8]) -> Option<f64> {
    riff_seconds(blob).or_else(|| crate::mp3::describe(blob)?.seconds)
}

/// A RIFF header's declared length, for a stereo ATRAC3+ stream at 44,100 Hz
/// and nothing else.
///
/// The channel count and the rate are checked on **both** routes, not just the
/// population one: a declared entry that turned out to be mono would be a
/// finding rather than something to play at the wrong speed. MPEG carries no
/// such rule, because there the declaration is the whole population - there is
/// nothing to sort a track out from.
fn riff_seconds(header: &[u8]) -> Option<f64> {
    let stream = crate::at3::describe(header).ok()?;
    if stream.format.channels != 2 || stream.format.sample_rate != 44_100 {
        return None;
    }
    stream.seconds()
}

/// Decodes a music blob, whichever of the two containers it is in.
///
/// ATRAC3+ goes out to `ffmpeg` through the cache in [`crate::at3`]; MP3 is
/// decoded in process by [`crate::mp3`] and cached nowhere, for the reason that
/// module's header gives.
///
/// # Errors
///
/// A blob that is neither container, or a decode that failed - including
/// `ffmpeg` being absent, which only the ATRAC3+ half can hit.
fn decode(blob: &[u8], cache_dir: &Path) -> Result<Pcm> {
    if crate::at3::describe(blob).is_ok() {
        return crate::at3::decode(blob, cache_dir);
    }
    if crate::mp3::describe(blob).is_some() {
        return crate::mp3::decode(blob);
    }
    bail!("this is neither a RIFF-wrapped ATRAC3+ stream nor an MPEG one")
}

/// Reads one soundtrack entry and decodes it.
///
/// `at` is an [`Entry::at`] this module issued, and is interpreted the way the
/// route that issued it does - which is why the title is looked up again here
/// rather than the caller being asked to remember.
///
/// # Errors
///
/// A source that will not open, an entry that will not read, and a decode that
/// failed.
pub fn load_entry(source: &str, at: u32, cache_dir: &Path) -> Result<Sound> {
    let (title, mut archives) =
        open_archived(source).with_context(|| format!("opening {source} as a known title"))?;

    let blob = match title.music.and_then(|music| music.tracks) {
        Some(tracks) => {
            let names = declared_names(&mut archives, tracks);
            let name = names
                .get(at as usize)
                .with_context(|| format!("track {at} is past the {} declared", names.len()))?
                .clone();
            archives
                .read_name(&name)
                .with_context(|| format!("reading {name}"))?
        }
        None => {
            let data = archives.data.as_wad_mut("a name hash")?;
            data.read_hash(at)
                .with_context(|| format!("reading Data.wad entry {at:08x}"))?
        }
    };

    let pcm = decode(&blob, cache_dir).with_context(|| format!("decoding track {at}"))?;
    Sound::new(pcm.samples, pcm.channels, pcm.sample_rate).with_context(|| format!("track {at}"))
}

/// The front end's own music, decoded, and the name it came off.
///
/// **Not one of the soundtrack tracks, and that is the point.** It is short, it
/// loops, and no PS2 entry has ever been matched to one - which is why
/// [`crate::audio::MusicSource`] cannot reach it. Each title names its own; see
/// [`oag_title::Music::front_end`].
///
/// `Ok(None)` when the source is not a disc of a title this knows, or is one
/// that holds no such entry - an ordinary outcome, not an error.
///
/// # Errors
///
/// An entry that is not a readable stream, or a decode that failed - including
/// `ffmpeg` being absent, which the caller reports rather than treating as
/// fatal.
pub fn load_front_end(source: &str, cache_dir: &Path) -> Result<Option<(String, Sound)>> {
    let Some((title, mut archives)) = open_archived(source) else {
        return Ok(None);
    };
    let Some(name) = title.music.map(|music| music.front_end) else {
        return Ok(None);
    };
    if archives.locate(name).is_none() {
        return Ok(None);
    }

    let blob = archives
        .read_name(name)
        .with_context(|| format!("reading {name}"))?;
    let pcm = decode(&blob, cache_dir).with_context(|| format!("decoding {name}"))?;
    let sound = Sound::new(pcm.samples, pcm.channels, pcm.sample_rate).context(name)?;
    Ok(Some((name.to_string(), sound)))
}

#[cfg(test)]
mod tests;
