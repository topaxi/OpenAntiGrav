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
use crate::catalogue;

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
    /// The song's stream is not stereo ATRAC9: this many channels.
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

fn stems_of(chain: &SongChain) -> Result<(f64, Vec<u32>, Vec<bool>, Vec<Clip>), String> {
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
    Ok((segment.duration / 1000.0, stems, embedded, clips))
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
    let xml = String::from_utf8(
        archives
            .read_name(st.declared_in)
            .with_context(|| format!("reading {}", st.declared_in))?,
    )
    .context("the music definition is not UTF-8")?;
    let blob = bank_blob(archives, st)?;
    let bank = Bank::parse(&blob).map_err(|e| anyhow!("{}: {e}", st.bank))?;
    let lib = Library::new(vec![bank]);
    let group = name_hash(st.track_group);
    let mut plan = Plan::default();
    for entry in catalogue::music(&xml) {
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
        let (seconds, stems, embedded, clips) = match stems_of(&chain) {
            Ok(found) => found,
            Err(why) => {
                skip(Skip::Walk(why));
                continue;
            }
        };
        if check_media {
            let first = if embedded[0] {
                lib.banks()[0].embedded(stems[0]).map(<[u8]>::to_vec)
            } else {
                archives.read_name(&wem_name(st, stems[0])).ok()
            };
            let Some(first) = first else {
                skip(Skip::MissingMedia(stems[0]));
                continue;
            };
            match Wem::parse(&first).map(|w| w.format().channels) {
                Ok(2) => {}
                Ok(other) => {
                    skip(Skip::Channels(other));
                    continue;
                }
                Err(_) => {
                    skip(Skip::MissingMedia(stems[0]));
                    continue;
                }
            }
        }
        plan.songs.push(Song {
            title: entry.id.clone(),
            location,
            state,
            seconds,
            stems,
            embedded,
            clips,
        });
    }
    Ok(plan)
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
    let (seconds, stems, embedded, clips) = stems_of(&chain).map_err(|e| anyhow!("{e}"))?;
    Ok(Some(Song {
        title: format!("music container {}", chain.ranseq),
        location: 0,
        state: 0,
        seconds,
        stems,
        embedded,
        clips,
    }))
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
/// whole to fit, because the tracks carry only an unread property (id 13)
/// ranging -100 to 100 that looks like a position, not a gain.
///
/// # Errors
///
/// A stem that will not read or decode, or one that is not stereo at the
/// same rate as the first.
pub fn mix(archives: &mut Archives, st: &StateTracks, song: &Song) -> Result<Pcm> {
    let bank_data = if song.embedded.contains(&true) {
        Some(bank_blob(archives, st)?)
    } else {
        None
    };
    let bank = bank_data
        .as_deref()
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
    let decoded = decoded.into_iter().collect::<Result<Vec<_>>>()?;
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
