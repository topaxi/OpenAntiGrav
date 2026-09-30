use super::hirc::{Action, Codec, Event, MusicTrack, Sound, StreamType};
use super::*;

fn chunk(tag: &[u8; 4], body: &[u8]) -> Vec<u8> {
    let mut out = tag.to_vec();
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(body);
    out
}

fn object(kind: u8, id: u32, body: &[u8]) -> Vec<u8> {
    let mut out = vec![kind];
    out.extend_from_slice(&(body.len() as u32 + 4).to_le_bytes());
    out.extend_from_slice(&id.to_le_bytes());
    out.extend_from_slice(body);
    out
}

fn header(version: u32, id: u32, padding: usize) -> Vec<u8> {
    let mut body = Vec::new();
    body.extend_from_slice(&version.to_le_bytes());
    body.extend_from_slice(&id.to_le_bytes());
    body.extend_from_slice(&0u32.to_le_bytes());
    body.extend_from_slice(&0u16.to_le_bytes());
    body.extend_from_slice(&1u16.to_le_bytes());
    body.extend_from_slice(&3635u32.to_le_bytes());
    body.extend(std::iter::repeat_n(0u8, padding));
    chunk(b"BKHD", &body)
}

/// A node's base parameters up to and including the parent, then a tail that
/// stands in for everything after it.
fn base(parent: u32) -> Vec<u8> {
    let mut out = vec![0, 0, 0];
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&parent.to_le_bytes());
    out.extend_from_slice(&[0xaa; 9]);
    out
}

fn source(stream: u8, media: u32, size: u32, bits: u8, plugin: u32) -> Vec<u8> {
    let mut out = plugin.to_le_bytes().to_vec();
    out.push(stream);
    out.extend_from_slice(&media.to_le_bytes());
    out.extend_from_slice(&size.to_le_bytes());
    out.push(bits);
    out
}

fn sound(stream: u8, media: u32, parent: u32) -> Vec<u8> {
    let mut body = source(stream, media, 4, 0, 0x000c_0001);
    body.extend(base(parent));
    body
}

fn play(target: u32, bank: u32) -> Vec<u8> {
    let mut body = 0x0403u16.to_le_bytes().to_vec();
    body.extend_from_slice(&target.to_le_bytes());
    body.extend_from_slice(&[0, 0, 0, 4]);
    body.extend_from_slice(&bank.to_le_bytes());
    body
}

fn event(actions: &[u32]) -> Vec<u8> {
    let mut body = (actions.len() as u32).to_le_bytes().to_vec();
    for a in actions {
        body.extend_from_slice(&a.to_le_bytes());
    }
    body
}

/// Bank 7: event 100 plays action 200, which plays mixer 300, which holds
/// embedded sound 400 (media 9) and streamed sound 401 (media 10).
fn bank() -> Vec<u8> {
    let mut hirc = 6u32.to_le_bytes().to_vec();
    hirc.extend(object(4, 100, &event(&[200])));
    hirc.extend(object(3, 200, &play(300, 7)));
    hirc.extend(object(7, 300, &{
        let mut b = base(0);
        b.extend_from_slice(&2u32.to_le_bytes());
        b
    }));
    hirc.extend(object(2, 400, &sound(0, 9, 300)));
    hirc.extend(object(2, 401, &sound(1, 10, 300)));
    hirc.extend(object(3, 201, &play(999, 7)));
    // One DIDX record: media 9, at offset 0 of DATA, three bytes long.
    let mut didx = 9u32.to_le_bytes().to_vec();
    didx.extend_from_slice(&0u32.to_le_bytes());
    didx.extend_from_slice(&3u32.to_le_bytes());
    let mut out = header(118, 7, 8);
    out.extend(chunk(b"DIDX", &didx));
    out.extend(chunk(b"DATA", b"RIF"));
    out.extend(chunk(b"HIRC", &hirc));
    out
}

#[test]
fn a_bank_is_a_run_of_chunks_that_tiles_the_file() {
    let data = bank();
    let bank = Bank::parse(&data).unwrap();
    let tags: Vec<_> = bank.chunks().iter().map(|c| c.tag).collect();
    assert_eq!(tags, [*b"BKHD", *b"DIDX", *b"DATA", *b"HIRC"]);
    assert_eq!(bank.chunks().last().unwrap().body.end, data.len());
    let h = bank.header();
    assert_eq!((h.version, h.id, h.language_id, h.padding), (118, 7, 0, 8));
    assert_eq!(
        (h.alignment, h.device_allocated, h.project_id),
        (0, 1, 3635)
    );
    assert_eq!(bank.embedded(9), Some(&b"RIF"[..]));
    assert_eq!(bank.embedded(10), None);
    assert_eq!(bank.objects().len(), 6);
    assert_eq!(bank.object(400).unwrap().kind, Kind::Sound);
}

#[test]
fn a_file_that_does_not_tile_is_refused() {
    let data = bank();
    assert!(matches!(
        Bank::parse(&data[..data.len() - 1]),
        Err(Error::Truncated { .. })
    ));
    let mut extra = data.clone();
    extra.push(0);
    assert!(matches!(Bank::parse(&extra), Err(Error::Truncated { .. })));
    assert_eq!(
        Bank::parse(&chunk(b"DIDX", &[])).unwrap_err(),
        Error::NoHeader
    );
    assert!(matches!(
        Bank::parse(&header(140, 1, 0)),
        Err(Error::UnsupportedVersion { version: 140 })
    ));
    let mut bad = header(118, 1, 0);
    bad.extend(chunk(b"DIDX", &[0; 5]));
    assert_eq!(
        Bank::parse(&bad).unwrap_err(),
        Error::BadMediaIndex { size: 5 }
    );
}

#[test]
fn a_hierarchy_whose_count_is_wrong_is_refused() {
    let mut hirc = 2u32.to_le_bytes().to_vec();
    hirc.extend(object(4, 100, &event(&[])));
    let mut out = header(118, 1, 0);
    out.extend(chunk(b"HIRC", &hirc));
    assert!(matches!(Bank::parse(&out), Err(Error::BadHierarchy { .. })));
}

#[test]
fn events_actions_and_sounds_read_exactly() {
    let data = bank();
    let bank = Bank::parse(&data).unwrap();
    let e = Event::parse(bank.body(bank.object(100).unwrap())).unwrap();
    assert_eq!(e.actions, [200]);
    let a = Action::parse(bank.body(bank.object(200).unwrap())).unwrap();
    assert_eq!((a.kind, a.target), (Action::PLAY, 300));
    assert_eq!(a.play.unwrap().bank_id, 7);
    let s = Sound::parse(bank.body(bank.object(400).unwrap())).unwrap();
    assert_eq!(s.source.media_id, 9);
    assert_eq!(s.source.stream, StreamType::Embedded);
    assert_eq!(s.source.plugin.codec(), Some(Codec::Atrac9));
    assert_eq!(s.parent, Some(300));
    // A body one byte long is refused, not read short.
    assert!(Event::parse(&[1, 0, 0, 0, 1]).is_none());
    assert!(Sound::parse(&[0; 5]).is_none());
}

#[test]
fn a_play_that_does_not_end_where_it_should_carries_no_play() {
    let mut body = play(300, 7);
    body.push(0);
    let a = Action::parse(&body).unwrap();
    assert_eq!((a.kind, a.target), (Action::PLAY, 300));
    assert!(a.play.is_none());
}

#[test]
fn plugin_ids_decode_class_company_and_number() {
    use super::hirc::Plugin;
    assert_eq!(Plugin(0x0001_0001).codec(), Some(Codec::Pcm));
    assert_eq!(Plugin(0x000c_0001).codec(), Some(Codec::Atrac9));
    assert_eq!(Plugin(0x0004_0001).codec(), Some(Codec::Other(4)));
    // A source plugin (class 2, the generators) is not a codec.
    assert_eq!(Plugin(0x0064_0002).codec(), None);
    assert_eq!(Plugin(0x0064_0002).number(), 100);
}

#[test]
fn a_music_track_lists_its_sources() {
    let mut body = vec![0u8];
    body.extend_from_slice(&2u32.to_le_bytes());
    body.extend(source(1, 5, 10, 0, 0x000c_0001));
    body.extend(source(0, 6, 20, 1, 0x000c_0001));
    let t = MusicTrack::parse(&body).unwrap();
    assert_eq!(t.sources.len(), 2);
    assert!(t.sources[1].language_specific());
    assert!(MusicTrack::parse(&body[..body.len() - 1]).is_none());
}

#[test]
fn an_event_resolves_through_its_mixer_to_the_media_below_it() {
    let data = bank();
    let library = Library::new(vec![Bank::parse(&data).unwrap()]);
    let plan = library.resolve_event(100).unwrap();
    assert_eq!(plan.targets, [300]);
    let ids: Vec<_> = plan.media.iter().map(|m| m.source.media_id).collect();
    assert_eq!(ids, [9, 10]);
    assert_eq!(plan.media[0].embedded_in, Some(0));
    assert!(!plan.media[0].is_loose());
    assert_eq!(plan.media[1].embedded_in, None);
    assert!(plan.media[1].is_loose());
    assert!(plan.unresolved.is_empty());
    assert!(library.resolve_event(12345).is_none());
}

#[test]
fn a_target_in_no_bank_is_reported_and_not_guessed_at() {
    let data = bank();
    let library = Library::new(vec![Bank::parse(&data).unwrap()]);
    // Event 101 is added by giving action 201 to a second event's list.
    let mut hirc = 2u32.to_le_bytes().to_vec();
    hirc.extend(object(4, 101, &event(&[201])));
    hirc.extend(object(3, 201, &play(999, 7)));
    let mut second = header(118, 8, 0);
    second.extend(chunk(b"HIRC", &hirc));
    let both = Library::new(vec![
        Bank::parse(&data).unwrap(),
        Bank::parse(&second).unwrap(),
    ]);
    let plan = both.resolve_event(101).unwrap();
    assert_eq!(plan.unresolved, [999]);
    assert!(plan.media.is_empty());
    drop(library);
}
