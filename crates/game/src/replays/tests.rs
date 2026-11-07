use oag_replay::{Header, Recorder};

use super::*;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oag-replays-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// A replay of `ticks` ticks; each tick is a stored input, so the file's size grows with it.
fn replay(track: &str, ticks: usize) -> Replay {
    let header = Header::new("Wipeout Pulse", track, "single_race", "Venom", "feisar", 1);
    let mut recorder = Recorder::new(header, 7);
    for tick in 0..ticks {
        let mut input = oag_gameplay::InputSnapshot::EMPTY;
        input.stick_x = (tick % 255) as f32 / 255.0;
        let mut inputs = oag_gameplay::PlayerInputs::none();
        inputs.set(0, input);
        recorder.record(&inputs, || tick as u64);
    }
    recorder.finish()
}

fn names(root: &Path, folder: Folder) -> Vec<String> {
    let mut names: Vec<String> = files(root, folder)
        .into_iter()
        .map(|(path, _, _)| path.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn a_file_is_named_for_its_title_and_second_and_two_in_one_second_do_not_collide() {
    let root = scratch("names");
    let first = write_auto_in(&root, "Wipeout Pulse", 1_000, &replay("a", 10)).unwrap();
    let second = write_auto_in(&root, "Wipeout Pulse", 1_000, &replay("b", 10)).unwrap();
    assert_eq!(first.file_name().unwrap(), "oag-wipeout-pulse-1000-1.oagr");
    assert_eq!(second.file_name().unwrap(), "oag-wipeout-pulse-1000-2.oagr");
    assert_eq!(stamp("oag-wipeout-pulse-1000-2.oagr"), (1_000, 2));
    assert_eq!(stamp("something-else.oagr"), (0, 0));
    let _ = std::fs::remove_dir_all(&root);
}

/// Acceptance, ADR-0087 stage 5: the ring deletes the oldest at 20 files.
#[test]
fn the_ring_holds_twenty_files_and_deletes_the_oldest_first() {
    let root = scratch("twenty");
    for unix in 1..=25_u64 {
        write_auto_in(&root, "Wipeout Pulse", unix, &replay("a", 10)).unwrap();
    }
    let held = names(&root, Folder::Auto);
    assert_eq!(held.len(), MAX_FILES);
    // 6..=25 stay; 1..=5 went, the oldest first.
    assert!(held.contains(&"oag-wipeout-pulse-6-1.oagr".to_string()));
    assert!(held.contains(&"oag-wipeout-pulse-25-1.oagr".to_string()));
    assert!(!held.contains(&"oag-wipeout-pulse-5-1.oagr".to_string()));
    let _ = std::fs::remove_dir_all(&root);
}

/// Acceptance, ADR-0087 stage 5: the ring deletes the oldest at 32 MiB, with the real limit.
#[test]
fn the_ring_holds_thirty_two_mebibytes_and_deletes_the_oldest_first() {
    let root = scratch("bytes");
    let auto = root.join(AUTO);
    std::fs::create_dir_all(&auto).unwrap();
    // Five files of 8 MiB are 40 MiB: one over, so the oldest goes and 32 MiB stays.
    let big = vec![0_u8; 8 * 1024 * 1024];
    for unix in 1..=5_u64 {
        std::fs::write(auto.join(file_name("Wipeout Pulse", unix, 1)), &big).unwrap();
    }
    let deleted = prune_in(&root);
    assert_eq!(deleted.len(), 1);
    assert!(deleted[0].ends_with("oag-wipeout-pulse-1-1.oagr"));
    let held: u64 = files(&root, Folder::Auto)
        .iter()
        .map(|(_, _, size)| size)
        .sum();
    assert_eq!(held, MAX_BYTES);
    assert_eq!(
        prune_in(&root),
        Vec::<PathBuf>::new(),
        "at the limit, nothing more goes"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_newest_file_stays_though_it_alone_is_over_the_byte_limit() {
    let root = scratch("one-big");
    let auto = root.join(AUTO);
    std::fs::create_dir_all(&auto).unwrap();
    std::fs::write(
        auto.join(file_name("Wipeout Pulse", 1, 1)),
        vec![0_u8; 4096],
    )
    .unwrap();
    std::fs::write(
        auto.join(file_name("Wipeout Pulse", 2, 1)),
        vec![0_u8; 4096],
    )
    .unwrap();
    let deleted = prune_within(&root, MAX_FILES, 1024);
    assert_eq!(deleted.len(), 1);
    assert_eq!(names(&root, Folder::Auto), ["oag-wipeout-pulse-2-1.oagr"]);
    let _ = std::fs::remove_dir_all(&root);
}

/// Age is the name, not the modification time: the oldest-named file goes though it was
/// written last.
#[test]
fn age_is_the_name_not_the_modification_time() {
    let root = scratch("age");
    write_auto_in(&root, "Wipeout Pulse", 500, &replay("a", 10)).unwrap();
    write_auto_in(&root, "Wipeout Pulse", 100, &replay("a", 10)).unwrap();
    let deleted = prune_within(&root, 1, MAX_BYTES);
    assert_eq!(deleted.len(), 1);
    assert!(deleted[0].ends_with("oag-wipeout-pulse-100-1.oagr"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn saved_is_never_pruned_and_keep_copies_into_it() {
    let root = scratch("keep");
    let kept = write_auto_in(&root, "Wipeout Pulse", 1, &replay("keep", 10)).unwrap();
    let entry = list_in(&root).into_iter().next().unwrap();
    assert_eq!(entry.folder, Folder::Auto);
    assert_eq!(entry.path, kept);
    let saved = keep_in(&root, &entry).unwrap();
    assert!(saved.starts_with(root.join(SAVED)));
    for unix in 2..=40_u64 {
        write_auto_in(&root, "Wipeout Pulse", unix, &replay("a", 10)).unwrap();
    }
    assert!(!kept.exists(), "the ring deleted the original");
    assert!(saved.exists(), "the kept copy stayed");
    assert_eq!(names(&root, Folder::Saved).len(), 1);
    assert_eq!(names(&root, Folder::Auto).len(), MAX_FILES);
    let listed = list_in(&root);
    assert_eq!(listed[0].folder, Folder::Saved, "saved lists first");
    assert_eq!(listed[1].unix, 40, "then the ring, newest first");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_list_reads_the_header_and_names_a_file_it_cannot() {
    let root = scratch("list");
    write_auto_in(
        &root,
        "Wipeout Pulse",
        1,
        &replay("Data\\Track\\one.vex", 30),
    )
    .unwrap();
    let auto = root.join(AUTO);
    std::fs::write(auto.join(file_name("Wipeout Pulse", 2, 1)), b"not a replay").unwrap();
    let listed = list_in(&root);
    assert_eq!(listed.len(), 2);
    assert_eq!(listed[0].head, Head::Damaged);
    let Head::Read(header) = &listed[1].head else {
        panic!("the real one reads");
    };
    assert_eq!(header.track, "Data\\Track\\one.vex");
    assert!(listed[1].bytes > 0);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_header_of_a_later_major_is_newer_not_damaged() {
    let root = scratch("newer");
    let path = write_auto_in(&root, "Wipeout Pulse", 1, &replay("a", 10)).unwrap();
    let mut bytes = std::fs::read(&path).unwrap();
    let text = String::from_utf8_lossy(&bytes).into_owned();
    assert!(text.contains("version = \"1.0.0\""));
    let at = text.find("version = \"1.0.0\"").unwrap() + "version = \"".len();
    bytes[at] = b'2';
    std::fs::write(&path, bytes).unwrap();
    assert_eq!(list_in(&root)[0].head, Head::Newer);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn delete_removes_the_file() {
    let root = scratch("delete");
    write_auto_in(&root, "Wipeout Pulse", 1, &replay("a", 10)).unwrap();
    let entry = list_in(&root).remove(0);
    delete(&entry).unwrap();
    assert!(list_in(&root).is_empty());
    let _ = std::fs::remove_dir_all(&root);
}
