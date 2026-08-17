//! Which entries of a PSP disc hold music, and how one of them is read.
//!
//! The half of [`crate::audio`] that has to know **which title booted**.
//! Everything else in that module is policy over a listing - what to play, when
//! to seek, which release to prefer - and none of it changes between two
//! Wipeout discs. Finding the listing does, so it lives here.
//!
//! # Two mechanisms, and the second one is not a fallback
//!
//! A title that declares its soundtrack ([`oag_title::DeclaredTracks`]) is read
//! from its own plugin definition: one `PI_Music` node per track, each naming
//! the directory the audio sits in. That is Wipeout Pure, whose `BOOT.BIN`
//! spells out both halves of the path - `%s\%s` and `music.at3`, either side of
//! `MusicManager.cpp` in the strings - and whose nineteen declarations all
//! resolve on both pressings.
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

use std::path::Path;

use anyhow::{Context, Result};
use oag_assets::Archives;
use oag_audio::Sound;
use oag_disc::Platform;
use oag_title::Title;

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

/// One soundtrack entry: where it is in `Data.wad`, and how long it is.
///
/// The name hash rather than the name, because that is what addresses an entry
/// and because the population route has no name to give.
#[derive(Debug, Clone, Copy)]
pub struct Entry {
    /// The `Data.wad` name hash.
    pub name_hash: u32,
    /// How long it is, from the `fact` chunk. See [`describe`].
    pub seconds: f64,
}

/// A source opened as whichever title it turned out to be, for a music read.
///
/// `None` rather than an error for every reason a source is not a PSP disc of a
/// title this knows - a PS2 image, a partial extract, a path that will not
/// open. All of them mean "no soundtrack here", which is what the caller does
/// something with; distinguishing them would give it nothing to do differently.
fn open_psp(source: &str) -> Option<(&'static Title, Archives)> {
    // Resolved through the layout rather than a literal `PSP_GAME/USRDIR/...`
    // path, so a directory somebody extracted with `oag-unpack` answers the
    // same as a disc image does.
    let opened = crate::title::open_source(source, Vec::new()).ok()?;
    (opened.archives.layout.platform == Platform::Psp).then_some((opened.title, opened.archives))
}

/// The soundtrack `source` carries, in the order that source's own title
/// decides.
///
/// `Ok(None)` when the source is not a PSP disc of a title this knows, and
/// `Ok(Some(vec![]))` when it is one whose music could not be found at all -
/// two different answers, because the first means "ask something else" and the
/// second means "this disc has none".
///
/// # Errors
///
/// An archive that opens and then will not read its own directory.
pub fn listing(source: &str) -> Result<Option<Vec<Entry>>> {
    let Some((title, mut archives)) = open_psp(source) else {
        return Ok(None);
    };
    let declared = title.music.and_then(|music| music.tracks);
    match declared {
        Some(tracks) => Ok(Some(self::declared(&mut archives, tracks))),
        None => Ok(Some(found(&mut archives)?)),
    }
}

/// The tracks a title's own plugin definition declares, in file order.
///
/// An entry that is declared and then absent is **skipped rather than
/// reported**: a `PI_Music` node is a declaration, and the same caveat
/// `oag_game::catalogue::all_tracks` records for a partial pack set applies
/// here. The length still comes from the payload's `fact` chunk, never from the
/// declaration, which carries none.
fn declared(archives: &mut Archives, tracks: oag_title::DeclaredTracks) -> Vec<Entry> {
    let Ok(definition) = archives.read_name(tracks.declared_in) else {
        return Vec::new();
    };
    let Ok(definition) = String::from_utf8(definition) else {
        return Vec::new();
    };

    let mut out = Vec::new();
    for track in crate::catalogue::music(&definition) {
        let name_hash = oag_formats::wad::hash_name(&track.entry_name(tracks.file));
        let Some(seconds) = length(archives, name_hash) else {
            continue;
        };
        out.push(Entry { name_hash, seconds });
    }
    out
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
        let Some(seconds) = describe(&header) else {
            continue;
        };
        out.push(Entry { name_hash, seconds });
    }
    Ok(out)
}

/// How long the entry at `name_hash` is, or `None` when it is not a stereo
/// ATRAC3+ stream this can measure.
fn length(archives: &mut Archives, name_hash: u32) -> Option<f64> {
    let data = archives.data.as_wad_mut("a name hash").ok()?;
    let index = data
        .directory()
        .entries
        .iter()
        .position(|entry| entry.name_hash == name_hash)?;
    let header = data.peek(index, RIFF_HEADER_PEEK).ok()?;
    describe(&header)
}

/// A RIFF header's declared length, for a stereo ATRAC3+ stream at 44,100 Hz
/// and nothing else.
///
/// The channel count and the rate are checked on **both** routes, not just the
/// population one: a declared entry that turned out to be mono would be a
/// finding rather than something to play at the wrong speed.
fn describe(header: &[u8]) -> Option<f64> {
    let stream = crate::at3::describe(header).ok()?;
    if stream.format.channels != 2 || stream.format.sample_rate != 44_100 {
        return None;
    }
    stream.seconds()
}

/// Reads one ATRAC3+ soundtrack entry out of `Data.wad` by name hash and
/// decodes it.
///
/// The whole entry is read rather than ranged, unlike the PS2 path: 2.2 MiB of
/// ATRAC3+ is a moment's work and the whole thing has to go to `ffmpeg` anyway.
/// It is the *decoded* form that is large - 33 MiB of PCM for three minutes -
/// and that is what the cache in [`crate::at3`] exists to avoid paying twice.
///
/// # Errors
///
/// A source that will not open, an entry that will not read, and a decode that
/// failed - including `ffmpeg` being absent.
pub fn load_entry(source: &str, name_hash: u32, cache_dir: &Path) -> Result<Sound> {
    let (_, mut archives) =
        open_psp(source).with_context(|| format!("opening {source} as a PSP disc"))?;
    let data = archives.data.as_wad_mut("a name hash")?;
    let at3 = data
        .read_hash(name_hash)
        .with_context(|| format!("reading Data.wad entry {name_hash:08x}"))?;
    let pcm = crate::at3::decode(&at3, cache_dir)
        .with_context(|| format!("decoding Data.wad entry {name_hash:08x}"))?;
    Sound::new(pcm.samples, pcm.channels, pcm.sample_rate)
        .with_context(|| format!("Data.wad entry {name_hash:08x}"))
}

/// The front end's own music, decoded, and the name it came off.
///
/// **Not one of the soundtrack tracks, and that is the point.** It is short, it
/// loops, and no PS2 entry has ever been matched to one - which is why
/// [`crate::audio::MusicSource`] cannot reach it. Each title names its own; see
/// [`oag_title::Music::front_end`].
///
/// `Ok(None)` when the source is not a PSP disc of a title this knows, or is
/// one that holds no such entry - an ordinary outcome, not an error.
///
/// The whole entry is read rather than ranged, unlike the PS2 path: 349 KiB of
/// ATRAC3+ is a fifth of a second's work and the whole thing has to go to
/// `ffmpeg` anyway. It is the *decoded* form that is large - 5 MiB of PCM for
/// 30 seconds - and that is what the cache exists to avoid paying twice.
///
/// # Errors
///
/// An entry that is not a readable RIFF, or a decode that failed - including
/// `ffmpeg` being absent, which the caller reports rather than treating as
/// fatal.
pub fn load_front_end(source: &str, cache_dir: &Path) -> Result<Option<(String, Sound)>> {
    let Some((title, mut archives)) = open_psp(source) else {
        return Ok(None);
    };
    let Some(name) = title.music.map(|music| music.front_end) else {
        return Ok(None);
    };
    if archives.locate(name).is_none() {
        return Ok(None);
    }

    let at3 = archives
        .read_name(name)
        .with_context(|| format!("reading {name}"))?;
    let pcm = crate::at3::decode(&at3, cache_dir).with_context(|| format!("decoding {name}"))?;
    let sound = Sound::new(pcm.samples, pcm.channels, pcm.sample_rate).context(name)?;
    Ok(Some((name.to_string(), sound)))
}

#[cfg(test)]
mod tests;
