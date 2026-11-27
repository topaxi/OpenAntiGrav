use super::*;

const EPOCH_PLUS: std::time::Duration = std::time::Duration::from_secs(1_000_000);

#[test]
fn the_shelf_hands_back_the_most_recent_value_of_an_id() {
    let mut shelf = Shelf::new();
    assert_eq!(shelf.give("a", 1, 4), None);
    assert_eq!(shelf.give("b", 2, 4), None);
    assert_eq!(shelf.give("a", 3, 4), None);
    assert_eq!(shelf.take(&"a"), Some(3));
    assert_eq!(shelf.take(&"a"), Some(1));
    assert_eq!(shelf.take(&"a"), None);
    assert_eq!(shelf.take(&"b"), Some(2));
    assert!(
        shelf.entries.is_empty(),
        "a taken value leaves nothing behind"
    );
}

#[test]
fn a_full_shelf_evicts_the_value_returned_longest_ago() {
    let mut shelf = Shelf::new();
    for n in 0..4 {
        assert_eq!(shelf.give(n, n, 4), None);
    }
    assert_eq!(shelf.give(10, 10, 4), Some(0));
    // Taking and returning 1 makes it the most recent, so 2 goes next.
    let one = shelf.take(&1).unwrap();
    assert_eq!(shelf.give(1, one, 4), None);
    assert_eq!(shelf.give(11, 11, 4), Some(2));
    assert_eq!(shelf.entries.len(), 4);
}

fn identity_of(path: &Path) -> Identity {
    Identity::of(&std::fs::File::open(path).unwrap(), path).unwrap()
}

fn scratch(tag: &str) -> oag_testdata::scratch::TempDir {
    oag_testdata::scratch::dir(&format!("chd-{tag}"))
}

fn set_modified(path: &Path) {
    std::fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(std::time::UNIX_EPOCH + EPOCH_PLUS)
        .unwrap();
}

#[test]
fn a_file_replaced_under_one_path_has_another_identity() {
    let dir = scratch("replaced");
    let path = dir.join("image.chd");
    std::fs::write(&path, b"first!").unwrap();
    set_modified(&path);
    let first = identity_of(&path);
    // The same length and modification time, a different file: written beside it, renamed over.
    let other = dir.join("other.chd");
    std::fs::write(&other, b"second").unwrap();
    set_modified(&other);
    std::fs::rename(&other, &path).unwrap();
    let second = identity_of(&path);
    std::fs::remove_dir_all(&dir).unwrap();
    assert_ne!(first, second);
}

#[cfg(unix)]
#[test]
fn two_spellings_of_one_file_have_one_identity() {
    let dir = scratch("alias");
    let path = dir.join("image.chd");
    std::fs::write(&path, b"image").unwrap();
    let link = dir.join("link.chd");
    std::os::unix::fs::symlink(&path, &link).unwrap();
    let hard = dir.join("hard.chd");
    std::fs::hard_link(&path, &hard).unwrap();
    let direct = identity_of(&path);
    let found = [
        identity_of(&link),
        identity_of(&hard),
        identity_of(&dir.join(".").join("image.chd")),
    ];
    std::fs::remove_dir_all(&dir).unwrap();
    for id in found {
        assert_eq!(direct, id);
    }
}
