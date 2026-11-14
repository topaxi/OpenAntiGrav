//! What the Omega Collection's `.bnk` files say, over every one of the 107
//! banks in its base and patch archives rather than the one the reader was
//! written against.
//!
//! **`#[ignore]`d and never run in CI.** It needs game content, which this
//! project does not ship. See `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! ```sh
//! just test-data
//! ```
//!
//! One archive per test, so the sweep parallelises across tests. Every number
//! pinned below was measured from the archive it names; a change is a finding
//! and the page to update is `docs/formats/wwise.md`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use oag_formats::wwise::container::{ContainerError, RanSeq, Switch};
use oag_formats::wwise::hirc::{self, Action, Codec, Event, MusicTrack, Sound, StreamType};
use oag_formats::wwise::{BANK_VERSION, Bank, Kind, Library, name_hash};

fn archive_path(dir: &str, name: &str) -> Option<PathBuf> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("data/extracted/ps4")
        .join(dir)
        .join("uroot")
        .join(name);
    if path.exists() {
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

const SOUND_DIR: &str = "data/audio/sound/";
const LANGUAGE_DIR: &str = "data/audio/sound/english(us)/";

/// What one archive's banks and loose media measure.
#[derive(Debug, Default, PartialEq, Eq)]
struct Census {
    banks: usize,
    /// Banks that are a `BKHD` and nothing else.
    header_only: usize,
    /// Banks with a `DIDX`.
    with_media: usize,
    media_records: usize,
    objects: usize,
    events: usize,
    actions: usize,
    play_actions: usize,
    /// `Play` actions whose target is an object of the same bank / of no bank
    /// of the archive.
    play_in_bank: usize,
    play_nowhere: usize,
    sounds: usize,
    sounds_pcm: usize,
    sounds_atrac9: usize,
    sounds_other_plugin: usize,
    tracks: usize,
    track_sources: usize,
    /// Nodes with a parent in the bank, with none (the root), and with one
    /// that is in no bank of the archive.
    parent_in_bank: usize,
    parent_root: usize,
    parent_nowhere: usize,
    /// Distinct media ids the banks name, and how many of them are a loose
    /// `.wem` of the archive / are embedded in some bank of it.
    media_named: usize,
    named_loose: usize,
    named_embedded: usize,
    named_nowhere: usize,
    /// Loose `.wem` files in the archive that no bank names.
    loose_orphans: usize,
    /// Embedded sounds with no `DIDX` record in their own bank whose media is
    /// in another bank's, and whose media is in no bank (a loose file, then).
    embedded_elsewhere: usize,
    embedded_nowhere: usize,
    /// Events that resolve to at least one media, and to none.
    events_resolving: usize,
    events_empty: usize,
    containers: Containers,
}

/// What the sound containers (random/sequence, switch) read as.
#[derive(Debug, Default, PartialEq, Eq)]
struct Containers {
    ranseq: usize,
    ranseq_random: usize,
    ranseq_sequence: usize,
    /// Random containers that weight their picks / forbid a repeat.
    ranseq_weighted: usize,
    ranseq_avoid_repeat: usize,
    ranseq_playlist_items: usize,
    switch: usize,
    switch_state_group: usize,
    switch_cases: usize,
    /// Child ids the cases list; of them, direct children of the switch / objects
    /// of the library.
    switch_case_children: usize,
    switch_case_direct: usize,
    switch_case_resolving: usize,
    /// Refused by name (`ContainerError::BaseParams`): a 3D block that opens a path.
    ranseq_refused: usize,
    switch_refused: usize,
    /// Any other refusal; asserted zero.
    refused_other: usize,
    /// Child ids over both kinds, how many resolve to an object of the
    /// library, and how many of those name the container as their parent.
    children: usize,
    children_resolving: usize,
    children_parented: usize,
}

fn tally_container(
    t: &mut Containers,
    library: &Library<'_>,
    bank: &Bank<'_>,
    object: &oag_formats::wwise::Object,
) {
    let body = bank.body(object);
    let (children, refused) = match object.kind {
        Kind::RanSeqContainer => match RanSeq::parse(object.kind, body) {
            Ok(r) => {
                t.ranseq += 1;
                if r.is_sequence() {
                    t.ranseq_sequence += 1;
                } else {
                    t.ranseq_random += 1;
                }
                t.ranseq_weighted += usize::from(r.uses_weights());
                t.ranseq_avoid_repeat += usize::from(r.avoid_repeat > 0);
                t.ranseq_playlist_items += r.playlist.len();
                (r.children, None)
            }
            Err(e) => (Vec::new(), Some(e)),
        },
        Kind::SwitchContainer => match Switch::parse(object.kind, body) {
            Ok(s) => {
                t.switch += 1;
                t.switch_state_group += usize::from(s.group_type == 1);
                t.switch_cases += s.cases.len();
                for id in s.cases.iter().flat_map(|c| &c.children) {
                    t.switch_case_children += 1;
                    t.switch_case_direct += usize::from(s.children.contains(id));
                    t.switch_case_resolving += usize::from(library_has(library, *id));
                }
                (s.children, None)
            }
            Err(e) => (Vec::new(), Some(e)),
        },
        _ => return,
    };
    match refused {
        Some(ContainerError::BaseParams { .. }) if object.kind == Kind::RanSeqContainer => {
            t.ranseq_refused += 1;
        }
        Some(ContainerError::BaseParams { .. }) => t.switch_refused += 1,
        Some(_) => t.refused_other += 1,
        None => {}
    }
    for child in children {
        t.children += 1;
        let resolved = library.banks().iter().find_map(|b| {
            b.objects_with_id(child)
                .next()
                .map(|o| hirc::parent(o.kind, b.body(o)))
        });
        if let Some(parent) = resolved {
            t.children_resolving += 1;
            t.children_parented += usize::from(parent == Some(object.id));
        }
    }
}

fn census(
    name_of_archive: (&str, &str),
    banks_expected: usize,
    check: fn(&[String], &Library<'_>),
) -> Option<Census> {
    let path = archive_path(name_of_archive.0, name_of_archive.1)?;
    let mut archive = oag_assets::psarc::Archive::open_file(&path).expect("the archive opens");
    let bank_paths: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.ends_with(".bnk"))
        .cloned()
        .collect();
    let loose: BTreeSet<(bool, u32)> = archive
        .paths()
        .iter()
        .filter_map(|p| {
            // `Data/` in the base archives and `data/` in the patch's.
            let p = p.to_ascii_lowercase();
            let (language, file) = match p.strip_prefix(LANGUAGE_DIR) {
                Some(file) => (true, file.to_string()),
                None => (false, p.strip_prefix(SOUND_DIR)?.to_string()),
            };
            let id = file.strip_suffix(".wem")?.parse().ok()?;
            Some((language, id))
        })
        .collect();
    let blobs: Vec<Vec<u8>> = bank_paths
        .iter()
        .map(|p| archive.read_path(p).expect("the bank reads"))
        .collect();

    let mut c = Census::default();
    let mut banks = Vec::new();
    for (path, blob) in bank_paths.iter().zip(&blobs) {
        let bank = Bank::parse(blob).unwrap_or_else(|e| panic!("{path}: {e}"));
        assert_eq!(bank.header().version, BANK_VERSION, "{path}");
        // The chunks tile the file, and BKHD is the first.
        let mut end = 0;
        for chunk in bank.chunks() {
            assert_eq!(chunk.body.start, end + 8, "{path}: chunks abut");
            end = chunk.body.end;
        }
        assert_eq!(end, blob.len(), "{path}: chunks end at the file's end");
        assert_eq!(&bank.chunks()[0].tag, b"BKHD");
        let header = &bank.chunks()[0].body;
        assert!(
            blob[header.start + 20..header.end].iter().all(|&b| b == 0),
            "{path}: BKHD padding is zero"
        );
        let tags: BTreeSet<[u8; 4]> = bank.chunks().iter().map(|k| k.tag).collect();
        assert!(
            tags.iter().all(|t| matches!(
                t,
                b"BKHD" | b"INIT" | b"STMG" | b"ENVS" | b"PLAT" | b"DIDX" | b"DATA" | b"HIRC"
            )),
            "{path}: chunk tags {tags:?}"
        );
        assert_eq!(
            tags.contains(b"DIDX"),
            tags.contains(b"DATA"),
            "{path}: DIDX and DATA come together"
        );
        c.banks += 1;
        c.header_only += usize::from(bank.chunks().len() == 1);
        c.with_media += usize::from(!bank.media().is_empty());
        c.media_records += bank.media().len();
        c.objects += bank.objects().len();
        for media in bank.media() {
            let bytes = bank
                .embedded(media.id)
                .unwrap_or_else(|| panic!("{path}: media {} lies outside DATA", media.id));
            assert!(
                bytes.starts_with(b"RIFF") && bytes[8..12] == *b"WAVE",
                "{path}: media {} is a RIFF/WAVE",
                media.id
            );
        }
        banks.push(bank);
    }
    assert_eq!(c.banks, banks_expected, "{}: banks", name_of_archive.1);

    let library = Library::new(banks.clone());
    let mut named: BTreeSet<(bool, u32)> = BTreeSet::new();
    for (index, bank) in banks.iter().enumerate() {
        let path = &bank_paths[index];
        let own_id = bank.header().id;
        for object in bank.objects() {
            let body = bank.body(object);
            tally_container(&mut c.containers, &library, bank, object);
            match object.kind {
                Kind::Event => {
                    let event = Event::parse(body)
                        .unwrap_or_else(|| panic!("{path}: event {:#x} reads", object.id));
                    c.events += 1;
                    for id in event.actions {
                        let action = bank.object(id).unwrap_or_else(|| {
                            panic!(
                                "{path}: event {:#x} names action {id:#x} which is absent",
                                object.id
                            )
                        });
                        assert_eq!(action.kind, Kind::Action, "{path}: {id:#x}");
                    }
                    match library
                        .resolve_event(object.id)
                        .expect("an event in the library")
                    {
                        plan if plan.media.is_empty() => c.events_empty += 1,
                        _ => c.events_resolving += 1,
                    }
                }
                Kind::Action => {
                    c.actions += 1;
                    let action = Action::parse(body)
                        .unwrap_or_else(|| panic!("{path}: action {:#x} reads", object.id));
                    if action.kind == Action::PLAY {
                        c.play_actions += 1;
                        let play = action.play.unwrap_or_else(|| {
                            panic!(
                                "{path}: Play {:#x} does not end where its layout says",
                                object.id
                            )
                        });
                        assert_eq!(play.bank_id, own_id, "{path}: a Play names its own bank");
                        assert_eq!(action.target_flags, 0);
                        if bank.object(action.target).is_some() {
                            c.play_in_bank += 1;
                        } else if !library_has(&library, action.target) {
                            c.play_nowhere += 1;
                        }
                    }
                }
                Kind::Sound => {
                    c.sounds += 1;
                    let sound = Sound::parse(body)
                        .unwrap_or_else(|| panic!("{path}: sound {:#x} reads", object.id));
                    match sound.source.plugin.codec() {
                        Some(Codec::Pcm) => c.sounds_pcm += 1,
                        Some(Codec::Atrac9) => c.sounds_atrac9 += 1,
                        _ => c.sounds_other_plugin += 1,
                    }
                    let generator = sound.source.plugin.class() != 1;
                    if !generator {
                        named.insert((sound.source.language_specific(), sound.source.media_id));
                    }
                    if sound.source.stream == StreamType::Embedded && !generator {
                        match bank.media().iter().find(|m| m.id == sound.source.media_id) {
                            Some(record) => assert_eq!(
                                record.size, sound.source.in_memory_size,
                                "{path}: embedded sound {:#x}'s size is its DIDX size",
                                object.id
                            ),
                            None if library.bank_with_media(sound.source.media_id).is_some() => {
                                c.embedded_elsewhere += 1;
                            }
                            None => c.embedded_nowhere += 1,
                        }
                    }
                    tally_parent(&mut c, &library, bank, object, sound.parent);
                }
                Kind::MusicTrack => {
                    let track = MusicTrack::parse(body)
                        .unwrap_or_else(|| panic!("{path}: track {:#x} reads", object.id));
                    c.tracks += 1;
                    for source in track.sources {
                        c.track_sources += 1;
                        named.insert((source.language_specific(), source.media_id));
                    }
                }
                kind if kind.has_parent() => {
                    let parent = oag_formats::wwise::hirc::parent(kind, body);
                    tally_parent(&mut c, &library, bank, object, parent);
                }
                _ => {}
            }
        }
    }

    c.media_named = named.len();
    for &(language, id) in &named {
        if loose.contains(&(language, id)) {
            c.named_loose += 1;
        } else if library.bank_with_media(id).is_some() {
            c.named_embedded += 1;
        } else {
            c.named_nowhere += 1;
        }
    }
    let named_ids: BTreeSet<(bool, u32)> = named.clone();
    c.loose_orphans = loose.iter().filter(|k| !named_ids.contains(k)).count();
    check(&bank_paths, &library);
    println!("{}: {:?}", name_of_archive.1, c.containers);
    Some(c)
}

/// The executable posts `<event>[_PC]__<bank>`, hashed whole
/// (`docs/ghidra/functions/ps4-omega-eu/wwise-sound.md`, "The event name"):
/// the bare `_321_NGP` is no event, the suffixed one is, in the bank the
/// sound manager loads into the `Weapons_NGP` slot. The `_PC` twin is one
/// action that plays the other event, so the player and NPC variants reach the
/// same media. The countdown's media ids are the "In Memory Audio" rows of the
/// bank's own `Weapons_NGP.txt` listing; the missile's legacy event relays to
/// the newer `WPN_MISSILE_Fire`.
fn posted_events_resolve(paths: &[String], library: &Library<'_>) {
    let weapons = paths
        .iter()
        .position(|p| p.eq_ignore_ascii_case("data/audio/sound/english(us)/weapons_ngp.bnk"))
        .expect("English(US)/Weapons_NGP.bnk ships");
    let bank = &library.banks()[weapons];
    for (event, listed_media) in [
        ("_321_NGP", Some(47_216_879u32)),
        ("Go_NGP", Some(605_868_029)),
        ("NGP_MissileFire", None),
    ] {
        assert!(
            library.play_targets(name_hash(event)).is_none(),
            "{event} is an event by its bare name"
        );
        let npc = name_hash(&format!("{event}__Weapons_NGP"));
        let defined_in: Vec<usize> = library
            .banks()
            .iter()
            .enumerate()
            .filter(|(_, b)| b.objects_with_id(npc).any(|o| o.kind == Kind::Event))
            .map(|(i, _)| i)
            .collect();
        assert_eq!(defined_in, [weapons], "{event}__Weapons_NGP's bank");
        let media = reached_media(library, npc);
        assert!(!media.is_empty(), "{event} reaches media");
        if let Some(id) = listed_media {
            assert!(media.contains(&id), "{event}: media {id} in {media:?}");
        }

        let player = name_hash(&format!("{event}_PC__Weapons_NGP"));
        let relays = relays_of(bank, player);
        assert_eq!(relays.len(), 1, "{event}_PC is one relay");
        assert_eq!(reached_media(library, player), media, "{event}_PC");
    }
    assert_eq!(
        relays_of(bank, name_hash("NGP_MissileFire__Weapons_NGP")),
        [name_hash("WPN_MISSILE_Fire")]
    );

    // The relay's target plays one layer container of five layers: four
    // random containers (a child each per post, every child a sound; 275399150
    // has one item, the other three six) and one sound.
    let fire = name_hash("WPN_MISSILE_Fire");
    let targets = library.play_targets(fire).expect("the relayed event");
    assert_eq!(targets, [117_129_590]);
    let layers = &library.banks()[weapons];
    let layer = layers.object(targets[0]).expect("the layer container");
    assert_eq!(layer.kind, Kind::LayerContainer);
    let mut shape = (0, 0, 0);
    for object in layers.objects() {
        if hirc::parent(object.kind, layers.body(object)) != Some(targets[0]) {
            continue;
        }
        match object.kind {
            Kind::RanSeqContainer => {
                shape.0 += 1;
                let r =
                    RanSeq::parse(object.kind, layers.body(object)).expect("a random container");
                assert!(
                    !r.is_sequence() && !r.playlist.is_empty(),
                    "{:?}",
                    r.playlist
                );
                for item in &r.playlist {
                    let child = layers.object(item.id).expect("a child of the bank");
                    assert_eq!(child.kind, Kind::Sound, "container {}'s child", object.id);
                }
                shape.2 += r.playlist.len();
            }
            Kind::Sound => shape.1 += 1,
            other => panic!("a {other:?} under the missile's layers"),
        }
    }
    assert_eq!(
        shape,
        (4, 1, 19),
        "(random containers, sounds, their children)"
    );

    // The node property bundle `oag_sound`'s event timeline reads: present on
    // every node, a volume in dB and an initial delay in seconds when set.
    use oag_formats::wwise::hirc::{node_props, prop};
    for bank in library.banks() {
        for object in bank.objects().iter().filter(|o| o.kind.has_parent()) {
            // A generator source (silence, tone) carries its plugin's
            // parameters after the source, which the fitted head does not skip.
            if object.kind == Kind::Sound
                && Sound::parse(bank.body(object)).is_some_and(|s| s.source.plugin.class() != 1)
            {
                continue;
            }
            let props = node_props(object.kind, bank.body(object))
                .unwrap_or_else(|| panic!("node {}'s property bundle", object.id));
            if let Some(db) = props.float(prop::VOLUME) {
                assert!(
                    (-96.0..=24.0).contains(&db),
                    "node {}: volume {db}",
                    object.id
                );
            }
            if let Some(s) = props.float(prop::INITIAL_DELAY) {
                assert!((0.0..=60.0).contains(&s), "node {}: delay {s}", object.id);
            }
        }
    }
}

/// Kind `0x2103`: an action whose target is another event, played in turn.
const PLAY_EVENT: u16 = 0x2103;

fn relays_of(bank: &Bank<'_>, event: u32) -> Vec<u32> {
    let object = bank
        .objects_with_id(event)
        .find(|o| o.kind == Kind::Event)
        .unwrap_or_else(|| panic!("event {event} in the bank"));
    Event::parse(bank.body(object))
        .expect("an event")
        .actions
        .into_iter()
        .filter_map(|id| Action::parse(bank.body(bank.object(id)?)))
        .filter(|a| a.kind == PLAY_EVENT)
        .map(|a| a.target)
        .collect()
}

/// The media an event's `Play` actions reach, following its relays.
fn reached_media(library: &Library<'_>, event: u32) -> BTreeSet<u32> {
    let mut media = BTreeSet::new();
    let mut queue = vec![event];
    let mut seen = BTreeSet::new();
    while let Some(id) = queue.pop() {
        if !seen.insert(id) {
            continue;
        }
        let plan = library.resolve_event(id).expect("a relay names an event");
        assert!(plan.unresolved.is_empty(), "event {id}: {plan:?}");
        media.extend(plan.media.iter().map(|m| m.source.media_id));
        for bank in library.banks() {
            if bank.objects_with_id(id).any(|o| o.kind == Kind::Event) {
                queue.extend(relays_of(bank, id));
            }
        }
    }
    media
}

fn library_has(library: &Library<'_>, id: u32) -> bool {
    library.banks().iter().any(|bank| bank.object(id).is_some())
}

fn tally_parent(
    c: &mut Census,
    library: &Library<'_>,
    bank: &Bank<'_>,
    object: &oag_formats::wwise::Object,
    parent: Option<u32>,
) {
    let _ = object;
    match parent {
        None => c.parent_root += 1,
        Some(id) if bank.object(id).is_some() => c.parent_in_bank += 1,
        Some(id) if library_has(library, id) => c.parent_in_bank += 1,
        Some(_) => c.parent_nowhere += 1,
    }
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu"]
fn base_data00_banks() {
    let Some(c) = census(("omega-eu", "data00.psarc"), 53, posted_events_resolve) else {
        return;
    };
    assert_eq!(
        c,
        Census {
            banks: 53,
            header_only: 16,
            with_media: 33,
            media_records: 2122,
            objects: 19538,
            events: 3706,
            actions: 4109,
            play_actions: 2183,
            play_in_bank: 1473,
            play_nowhere: 710,
            sounds: 6873,
            sounds_pcm: 10,
            sounds_atrac9: 6858,
            sounds_other_plugin: 5,
            tracks: 927,
            track_sources: 927,
            parent_in_bank: 9779,
            parent_root: 110,
            parent_nowhere: 0,
            media_named: 2467,
            named_loose: 714,
            named_embedded: 1753,
            named_nowhere: 0,
            loose_orphans: 0,
            embedded_elsewhere: 140,
            embedded_nowhere: 0,
            events_resolving: 1337,
            events_empty: 2369,
            containers: Containers {
                ranseq: 462,
                ranseq_random: 453,
                ranseq_sequence: 9,
                ranseq_weighted: 0,
                ranseq_avoid_repeat: 458,
                ranseq_playlist_items: 2116,
                switch: 525,
                switch_state_group: 340,
                switch_cases: 4097,
                switch_case_children: 4393,
                switch_case_direct: 3771,
                switch_case_resolving: 4391,
                ranseq_refused: 1,
                switch_refused: 0,
                refused_other: 0,
                children: 5460,
                children_resolving: 5460,
                children_parented: 5455,
            },
        }
    );
}

/// **The ten `.bnk` files in `data02` are not Wwise.** They are the Studio
/// Liverpool `SBlk` container the PSP and HD share, which
/// `oag_formats::sblk` reads: six little-endian and four big-endian (the
/// PS3's byte-swapped layout), all of them environment banks under
/// `Data/environments/`. An earlier census counted them with the Wwise banks
/// because of the extension.
#[test]
#[ignore = "needs data/extracted/ps4/omega-eu"]
fn data02_banks_are_the_old_container_and_wwise_refuses_them() {
    let Some(path) = archive_path("omega-eu", "data02.psarc") else {
        return;
    };
    let mut archive = oag_assets::psarc::Archive::open_file(&path).expect("the archive opens");
    let paths: Vec<String> = archive
        .paths()
        .iter()
        .filter(|p| p.ends_with(".bnk"))
        .cloned()
        .collect();
    assert_eq!(paths.len(), 10);
    let mut orders = [0usize; 2];
    for path in &paths {
        let blob = archive.read_path(path).expect("the bank reads");
        assert_eq!(
            Bank::parse(&blob).unwrap_err(),
            oag_formats::wwise::Error::NoHeader,
            "{path}"
        );
        let order = oag_formats::sblk::byte_order_of(&blob)
            .unwrap_or_else(|| panic!("{path}: an SBlk container in one of its two byte orders"));
        orders[usize::from(order == oag_bytes::ByteOrder::Big)] += 1;
        oag_formats::sblk::Bank::parse_as(&blob, order)
            .unwrap_or_else(|e| panic!("{path}: the existing reader reads it: {e}"));
    }
    assert_eq!(orders, [6, 4], "little-endian and big-endian SBlk banks");
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu-patch"]
fn patch_data05_banks() {
    let Some(c) = census(("omega-eu-patch", "data05.psarc"), 1, |_, _| {}) else {
        return;
    };
    assert_eq!(
        c,
        Census {
            banks: 1,
            header_only: 0,
            with_media: 1,
            media_records: 60,
            objects: 2478,
            events: 752,
            actions: 948,
            play_actions: 510,
            play_in_bank: 484,
            play_nowhere: 26,
            sounds: 599,
            sounds_pcm: 0,
            sounds_atrac9: 599,
            sounds_other_plugin: 0,
            tracks: 0,
            track_sources: 0,
            parent_in_bank: 771,
            parent_root: 4,
            parent_nowhere: 0,
            media_named: 520,
            named_loose: 0,
            named_embedded: 60,
            named_nowhere: 460,
            loose_orphans: 0,
            embedded_elsewhere: 0,
            embedded_nowhere: 0,
            events_resolving: 484,
            events_empty: 268,
            containers: Containers {
                ranseq: 19,
                ranseq_random: 18,
                ranseq_sequence: 1,
                ranseq_weighted: 0,
                ranseq_avoid_repeat: 19,
                ranseq_playlist_items: 111,
                switch: 39,
                switch_state_group: 17,
                switch_cases: 328,
                switch_case_children: 309,
                switch_case_direct: 239,
                switch_case_resolving: 239,
                ranseq_refused: 0,
                switch_refused: 0,
                refused_other: 0,
                children: 243,
                children_resolving: 243,
                children_parented: 243,
            },
        }
    );
}

#[test]
#[ignore = "needs data/extracted/ps4/omega-eu-patch"]
fn patch_data08_banks() {
    let Some(c) = census(
        ("omega-eu-patch", "data08.psarc"),
        53,
        posted_events_resolve,
    ) else {
        return;
    };
    assert_eq!(
        c,
        Census {
            banks: 53,
            header_only: 16,
            with_media: 33,
            media_records: 2257,
            objects: 21552,
            events: 3830,
            actions: 4382,
            play_actions: 2264,
            play_in_bank: 1550,
            play_nowhere: 714,
            sounds: 7527,
            sounds_pcm: 10,
            sounds_atrac9: 7510,
            sounds_other_plugin: 7,
            tracks: 993,
            track_sources: 993,
            parent_in_bank: 10845,
            parent_root: 118,
            parent_nowhere: 0,
            media_named: 2588,
            named_loose: 725,
            named_embedded: 1863,
            named_nowhere: 0,
            loose_orphans: 9,
            embedded_elsewhere: 140,
            embedded_nowhere: 0,
            events_resolving: 1367,
            events_empty: 2463,
            containers: Containers {
                ranseq: 551,
                ranseq_random: 538,
                ranseq_sequence: 13,
                ranseq_weighted: 0,
                ranseq_avoid_repeat: 547,
                ranseq_playlist_items: 2449,
                switch: 626,
                switch_state_group: 400,
                switch_cases: 4517,
                switch_case_children: 4746,
                switch_case_direct: 4122,
                switch_case_resolving: 4742,
                ranseq_refused: 1,
                switch_refused: 0,
                refused_other: 0,
                children: 6015,
                children_resolving: 6015,
                children_parented: 6008,
            },
        }
    );
}
