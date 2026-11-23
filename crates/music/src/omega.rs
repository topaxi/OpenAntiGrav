//! Wipeout: Omega Collection's soundtrack, which is a Wwise state per song.
//!
//! Nothing here is a file per track. The playlist is `PI_Music` plugin XML, the
//! song for `location` N is whatever the bank's own event
//! `Set_Music_Track_N__frontend` sets the `Music_Track` state to, and the audio
//! is found by walking the bank's music switch with that state and the race's
//! flow states set: switch -> random/sequence container -> segment -> tracks.
//! **A song is its segment's tracks played together** - stems of one piece
//! (7 to 11 stereo files), or one eight-channel file. See
//! `docs/formats/wwise.md`, "Music".
//!
//! An entry that cannot be played is **listed with the reason**
//! ([`Skip`]) and never replaced by another song.

use anyhow::{Context, Result, anyhow, ensure};
use oag_assets::Archives;
use oag_formats::wwise::hirc::StreamType;
use oag_formats::wwise::music::Clip;
use oag_formats::wwise::{Bank, Library, SongChain, name_hash, wem::Wem};
use oag_title::StateTracks;

use crate::at3::Pcm;
use crate::playlist;

/// One playable song.
#[derive(Debug, Clone, PartialEq)]
pub struct Song {
    /// The plugin's own title for it, read at run time.
    pub title: String,
    /// The `PI_Music` location number.
    pub location: u32,
    /// The `Music_Track` state the bank sets for it.
    pub state: u32,
    /// The segment's length.
    pub seconds: f64,
    /// The media ids of its tracks, in segment order.
    pub stems: Vec<u32>,
    /// Which of them are embedded in the bank's own `DATA` rather than loose.
    pub embedded: Vec<bool>,
    /// Each stem's clip: where it plays and how it is trimmed.
    pub clips: Vec<Clip>,
}

/// Why an entry is not played.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Skip {
    /// No event `Set_Music_Track_N__frontend` in the bank.
    NoEvent,
    /// The event sets no `Music_Track` state.
    NoState,
    /// The walk from the state stopped; the reason is the id.
    Walk(String),
    /// The song's stream is neither stereo nor eight-channel ATRAC9: this many
    /// channels.
    Channels(u16),
    /// A stem that is not a loose `.wem` of the archives.
    MissingMedia(u32),
}

/// The playlist as the bank resolves it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Plan {
    /// Playable songs, in `PI_Music` order.
    pub songs: Vec<Song>,
    /// Entries left unplayed, with their title and location.
    pub skipped: Vec<(String, u32, Skip)>,
}

fn wem_name(st: &StateTracks, media: u32) -> String {
    format!("{}{media}.wem", st.media_dir)
}

fn flow(pairs: &[(&str, &str)]) -> Vec<(u32, u32)> {
    pairs
        .iter()
        .map(|(group, state)| (name_hash(group), name_hash(state)))
        .collect()
}

fn walk(
    lib: &Library<'_>,
    st: &StateTracks,
    flow_states: &[(&str, &str)],
    track: Option<u32>,
) -> Result<SongChain, String> {
    let target = lib
        .play_targets(name_hash(st.play_event))
        .and_then(|targets| targets.first().copied())
        .ok_or_else(|| format!("event {} plays nothing", st.play_event))?;
    let mut states = flow(flow_states);
    if let Some(state) = track {
        states.push((name_hash(st.track_group), state));
    }
    lib.walk_music(target, &states, name_hash(st.none_state))
        .map_err(|e| format!("{e:?}"))
}

fn song_of(chain: &SongChain, title: String, location: u32, state: u32) -> Result<Song, String> {
    let [segment] = chain.segments.as_slice() else {
        return Err(format!("{} segments, expected one", chain.segments.len()));
    };
    let mut stems = Vec::new();
    let mut embedded = Vec::new();
    let mut clips = Vec::new();
    for track in &segment.tracks {
        let [media] = track.media.as_slice() else {
            return Err(format!(
                "track {} has {} sources",
                track.track,
                track.media.len()
            ));
        };
        let [clip] = track.clips.as_slice() else {
            return Err(format!(
                "track {} has {} clips",
                track.track,
                track.clips.len()
            ));
        };
        stems.push(media.source.media_id);
        embedded.push(media.source.stream == StreamType::Embedded);
        clips.push(*clip);
    }
    Ok(Song {
        title,
        location,
        state,
        seconds: segment.duration / 1000.0,
        stems,
        embedded,
        clips,
    })
}

fn bank_blob(archives: &mut Archives, st: &StateTracks) -> Result<Vec<u8>> {
    archives
        .read_name(st.bank)
        .with_context(|| format!("reading {}", st.bank))
}

/// Resolves the playlist. `check_media` reads each song's first stem to
/// refuse one that is not stereo; leave it off when only the walk is needed.
///
/// # Errors
///
/// A definition or bank that will not read or parse.
pub fn plan(archives: &mut Archives, st: &StateTracks, check_media: bool) -> Result<Plan> {
    let blob = bank_blob(archives, st)?;
    plan_with(archives, st, &blob, check_media)
}

/// What a run keeps of the bank between tracks: the playlist it resolves and
/// the bank's bytes, which [`mix_with`] reads an embedded stem out of.
#[derive(Debug, Clone, PartialEq)]
pub struct Prepared {
    /// The playlist, resolved without the media check ([`plan`]'s `false`).
    pub plan: Plan,
    /// The bank file, read once.
    pub bank: Vec<u8>,
}

/// Reads the definition and the bank once and resolves the playlist.
///
/// # Errors
///
/// As [`plan`].
pub fn prepare(archives: &mut Archives, st: &StateTracks) -> Result<Prepared> {
    let bank = bank_blob(archives, st)?;
    let plan = plan_with(archives, st, &bank, false)?;
    Ok(Prepared { plan, bank })
}

fn plan_with(
    archives: &mut Archives,
    st: &StateTracks,
    blob: &[u8],
    check_media: bool,
) -> Result<Plan> {
    let xml = String::from_utf8(
        archives
            .read_name(st.declared_in)
            .with_context(|| format!("reading {}", st.declared_in))?,
    )
    .context("the music definition is not UTF-8")?;
    let bank = Bank::parse(blob).map_err(|e| anyhow!("{}: {e}", st.bank))?;
    let lib = Library::new(vec![bank]);
    let group = name_hash(st.track_group);
    let mut plan = Plan::default();
    for entry in playlist::music(&xml) {
        let Ok(location) = entry.location.parse::<u32>() else {
            continue;
        };
        let mut skip = |why: Skip| plan.skipped.push((entry.id.clone(), location, why));
        let event = name_hash(&st.set_track_event.replace("{}", &location.to_string()));
        let Some(sets) = lib.set_states(event) else {
            skip(Skip::NoEvent);
            continue;
        };
        let Some(state) = sets.iter().find(|s| s.group == group).map(|s| s.state) else {
            skip(Skip::NoState);
            continue;
        };
        let chain = match walk(&lib, st, st.race_flow, Some(state)) {
            Ok(chain) => chain,
            Err(why) => {
                skip(Skip::Walk(why));
                continue;
            }
        };
        let song = match song_of(&chain, entry.id.clone(), location, state) {
            Ok(found) => found,
            Err(why) => {
                skip(Skip::Walk(why));
                continue;
            }
        };
        if check_media {
            let lead = song.stems[0];
            // A loose stem is its header only: the channel count is in its
            // `fmt ` chunk, and the stem is megabytes of ATRAC9.
            let channels = if song.embedded[0] {
                lib.banks()[0]
                    .embedded(lead)
                    .map(|blob| Wem::parse(blob).map(|w| w.format().channels).ok())
            } else {
                archives
                    .peek_name(&wem_name(st, lead), crate::MEDIA_HEADER_PEEK)
                    .ok()
                    .map(|head| crate::at3::describe(&head).ok().map(|s| s.format.channels))
            };
            let Some(channels) = channels else {
                skip(Skip::MissingMedia(lead));
                continue;
            };
            match channels.ok_or(()) {
                Ok(2 | FOLDED_CHANNELS) => {}
                Ok(other) => {
                    skip(Skip::Channels(other));
                    continue;
                }
                Err(_) => {
                    skip(Skip::MissingMedia(lead));
                    continue;
                }
            }
        }
        plan.songs.push(song);
    }
    Ok(plan)
}

/// Channels of a song stored as one 7.1 stream, which [`fold_to_stereo`] mixes
/// down.
pub const FOLDED_CHANNELS: u16 = 8;

/// A 7.1 stream mixed down to the stereo pair every other Omega song is.
///
/// **Chosen, not measured**: how the original folds 7.1 to stereo is unread and
/// there is no PS4 capture to compare. This is a standard ITU-R BS.775 style
/// downmix, replaceable by a measured one without touching a caller. The
/// stream's channel order is ATRAC9's 7.1 layout (`docs/formats/atrac9.md`):
/// front L/R, centre, LFE, side L/R, back L/R. Front pair at unity, centre and
/// side pair at -3 dB, back pair at -6 dB, LFE dropped (the two decoders
/// disagree on it, `docs/formats/atrac9.md`, "The LFE channel", and a
/// low-frequency channel adds nothing a stereo pair can place).
///
/// A stream that is not eight channels is returned as it is. The result can
/// exceed the 16-bit range before [`mix_with`]'s whole-song scale-down, so the
/// samples saturate here only if one stream alone would clip.
#[must_use]
pub fn fold_to_stereo(pcm: Pcm) -> Pcm {
    const MINUS_3_DB: f32 = std::f32::consts::FRAC_1_SQRT_2;
    const MINUS_6_DB: f32 = 0.5;
    if pcm.channels != FOLDED_CHANNELS {
        return pcm;
    }
    let ch = usize::from(FOLDED_CHANNELS);
    let mut samples = Vec::with_capacity(pcm.samples.len() / ch * 2);
    for frame in pcm.samples.chunks_exact(ch) {
        let at = |i: usize| f32::from(frame[i]);
        let left = at(0) + MINUS_3_DB * (at(2) + at(4)) + MINUS_6_DB * at(6);
        let right = at(1) + MINUS_3_DB * (at(2) + at(5)) + MINUS_6_DB * at(7);
        for side in [left, right] {
            samples.push(side.round().clamp(f32::from(i16::MIN), f32::from(i16::MAX)) as i16);
        }
    }
    Pcm {
        samples,
        channels: 2,
        sample_rate: pcm.sample_rate,
    }
}

/// The front end's own loop: what the bank plays with the menus' flow state
/// set and no song chosen. `Ok(None)` when the title names no such flow.
///
/// # Errors
///
/// A bank that will not read or parse, or a walk that stops.
pub fn front_end(archives: &mut Archives, st: &StateTracks) -> Result<Option<Song>> {
    if st.front_end_flow.is_empty() {
        return Ok(None);
    }
    let blob = bank_blob(archives, st)?;
    let bank = Bank::parse(&blob).map_err(|e| anyhow!("{}: {e}", st.bank))?;
    let lib = Library::new(vec![bank]);
    let chain = walk(&lib, st, st.front_end_flow, None).map_err(|e| anyhow!("{e}"))?;
    let title = format!("music container {}", chain.ranseq);
    song_of(&chain, title, 0, 0)
        .map(Some)
        .map_err(|e| anyhow!("{e}"))
}

/// Decodes every stem and sums them into one stereo stream.
///
/// A stem's source sample `t` plays at song time `play_at + t`, from
/// `max(begin_trim, 0)` to `source_duration + end_trim`, and the song ends at
/// the segment's length: the pairs fitted on all 17 stereo songs, where the
/// longest track's end lands on the segment length to the millisecond (for
/// example 307.5 s for a 453.75 s source). Stems are decoded on a thread each
/// (a song is up to eleven five-minute streams). **Level is chosen, not
/// measured**: unity sum, and a song whose sum would clip is scaled down as a
/// whole to fit (a 7.1 stream is first folded, [`fold_to_stereo`]), because the tracks carry only an unread property (id 13)
/// ranging -100 to 100 that looks like a position, not a gain.
///
/// # Errors
///
/// A stem that will not read or decode, or one that is not stereo at the
/// same rate as the first.
pub fn mix(archives: &mut Archives, st: &StateTracks, song: &Song) -> Result<Pcm> {
    mix_with(archives, st, song, None)
}

/// [`mix`] with the bank's bytes already read (a [`Prepared::bank`]); `None`
/// reads them when an embedded stem needs them.
///
/// # Errors
///
/// As [`mix`].
pub fn mix_with(
    archives: &mut Archives,
    st: &StateTracks,
    song: &Song,
    kept: Option<&[u8]>,
) -> Result<Pcm> {
    let read;
    let bank_data = if !song.embedded.contains(&true) {
        None
    } else if let Some(kept) = kept {
        Some(kept)
    } else {
        read = bank_blob(archives, st)?;
        Some(read.as_slice())
    };
    let bank = bank_data
        .map(|blob| Bank::parse(blob).map_err(|e| anyhow!("{}: {e}", st.bank)))
        .transpose()?;
    let mut blobs: Vec<std::borrow::Cow<'_, [u8]>> = Vec::new();
    for (&id, &embedded) in song.stems.iter().zip(&song.embedded) {
        blobs.push(if embedded {
            let bank = bank.as_ref().context("an embedded stem needs the bank")?;
            std::borrow::Cow::Borrowed(
                bank.embedded(id)
                    .with_context(|| format!("stem {id} is not in the bank's DATA"))?,
            )
        } else {
            std::borrow::Cow::Owned(
                archives
                    .read_name(&wem_name(st, id))
                    .with_context(|| format!("reading stem {id}"))?,
            )
        });
    }
    let decoded: Vec<Result<Pcm>> = std::thread::scope(|scope| {
        let handles: Vec<_> = blobs
            .iter()
            .map(|blob| scope.spawn(|| crate::wem::decode(blob.as_ref())))
            .collect();
        handles
            .into_iter()
            .map(|h| {
                h.join()
                    .unwrap_or_else(|_| Err(anyhow!("a decode thread panicked")))
            })
            .collect()
    });
    let decoded = decoded
        .into_iter()
        .map(|pcm| pcm.map(fold_to_stereo))
        .collect::<Result<Vec<_>>>()?;
    let first = decoded.first().context("a song with no stems")?;
    let (channels, rate) = (first.channels, first.sample_rate);
    ensure!(
        decoded
            .iter()
            .all(|p| p.channels == channels && p.sample_rate == rate),
        "the stems disagree on channels or rate"
    );
    let ch = usize::from(channels);
    let to_frames = |ms: f64| (ms * f64::from(rate) / 1000.0).round() as i64;
    let out_frames = to_frames(song.seconds * 1000.0).max(0) as usize;
    let mut sum = vec![0i32; out_frames * ch];
    for (pcm, clip) in decoded.iter().zip(&song.clips) {
        let src_frames = (pcm.samples.len() / ch) as i64;
        let offset = to_frames(clip.play_at);
        let from = to_frames(clip.begin_trim).max(0);
        let to =
            (to_frames(clip.source_duration) + to_frames(clip.end_trim.min(0.0))).min(src_frames);
        for t in from..to {
            let at = t + offset;
            if at < 0 || at >= out_frames as i64 {
                continue;
            }
            for c in 0..ch {
                sum[at as usize * ch + c] += i32::from(pcm.samples[t as usize * ch + c]);
            }
        }
    }
    let peak = sum.iter().map(|s| s.abs()).max().unwrap_or(0);
    let limit = i32::from(i16::MAX);
    let scale = if peak > limit {
        f64::from(limit) / f64::from(peak)
    } else {
        1.0
    };
    Ok(Pcm {
        samples: sum
            .into_iter()
            .map(|s| (f64::from(s) * scale).round() as i16)
            .collect(),
        channels,
        sample_rate: rate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pcm(frame: [i16; 8]) -> Pcm {
        Pcm {
            samples: frame.to_vec(),
            channels: 8,
            sample_rate: 48_000,
        }
    }

    #[test]
    fn a_front_pair_passes_at_unity_and_the_lfe_is_dropped() {
        let out = fold_to_stereo(pcm([1000, -2000, 0, 30_000, 0, 0, 0, 0]));
        assert_eq!((out.channels, out.sample_rate), (2, 48_000));
        assert_eq!(out.samples, vec![1000, -2000]);
    }

    #[test]
    fn centre_and_sides_come_in_at_minus_3_db_and_backs_at_minus_6_db() {
        assert_eq!(
            fold_to_stereo(pcm([0, 0, 1000, 0, 0, 0, 0, 0])).samples,
            vec![707, 707]
        );
        assert_eq!(
            fold_to_stereo(pcm([0, 0, 0, 0, 1000, -1000, 0, 0])).samples,
            vec![707, -707]
        );
        assert_eq!(
            fold_to_stereo(pcm([0, 0, 0, 0, 0, 0, 1000, -1000])).samples,
            vec![500, -500]
        );
    }

    #[test]
    fn a_stereo_stream_is_left_alone_and_a_loud_sum_saturates() {
        let stereo = Pcm {
            samples: vec![5, -5],
            channels: 2,
            sample_rate: 48_000,
        };
        assert_eq!(fold_to_stereo(stereo.clone()), stereo);
        let loud = fold_to_stereo(pcm([i16::MAX, 0, i16::MAX, 0, i16::MAX, 0, 0, 0]));
        assert_eq!(loud.samples[0], i16::MAX);
    }
}
