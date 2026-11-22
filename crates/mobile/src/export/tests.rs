use super::*;

#[test]
fn a_shot_goes_to_the_gallery_and_a_replay_to_downloads() {
    let shot = plan(Kind::Picture, "oag-pulse-1-1.png");
    assert_eq!(shot.collection, Collection::Images);
    assert_eq!(shot.mime, "image/png");
    assert_eq!(shot.relative_dir, "Pictures/OpenAntiGrav");
    assert_eq!(shot.display_name, "oag-pulse-1-1.png");

    let replay = plan(Kind::Replay, "oag-pulse-1-1.oagr");
    assert_eq!(replay.collection, Collection::Downloads);
    assert_eq!(replay.relative_dir, "Download/OpenAntiGrav");
}

#[test]
fn a_name_cannot_leave_its_folder() {
    let p = plan(Kind::Picture, "../../x\\y\n.png");
    assert!(!p.display_name.contains('/') && !p.display_name.contains('\\'));
    assert!(!p.display_name.chars().any(char::is_control));
}

#[test]
fn off_android_nothing_is_exported_and_the_file_is_not_even_read() {
    if available() {
        return;
    }
    let missing = Path::new("/nonexistent/oag-shot.png");
    match export(missing, Kind::Picture) {
        Err(Error::Read(_)) => {}
        other => panic!("{other:?}"),
    }
    assert!(matches!(
        insert(&plan(Kind::Picture, "a.png"), b"x"),
        Err(Error::Unsupported(_))
    ));
}
