use super::*;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("oag-profile-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a scratch directory");
    dir
}

#[test]
fn a_write_replaces_the_file_and_leaves_no_temp_file() {
    let dir = scratch("replace");
    let file = dir.join("records.toml");
    write(&file, "old").expect("first write");
    write(&file, "new").expect("second write");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "new");
    assert!(!staging(&file).exists(), "the temp file is renamed away");
}

#[test]
fn a_write_cut_short_keeps_the_old_file_whole() {
    let dir = scratch("cut");
    let file = dir.join("records.toml");
    write(&file, "medals = 12\n").expect("first write");

    super::super::cut_the_next_write_after(3);
    let crashed = write(&file, "medals = 13 and a great deal more\n");

    assert!(crashed.is_err(), "the cut write reports its failure");
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "medals = 12\n",
        "the file the crash interrupted is still the old one, whole"
    );
    assert_eq!(
        std::fs::read_to_string(staging(&file)).unwrap(),
        "med",
        "what the crash left behind is the half-written temp file"
    );
}

#[test]
fn the_next_write_replaces_a_temp_file_a_crash_left() {
    let dir = scratch("stray");
    let file = dir.join("records.toml");
    std::fs::write(staging(&file), "half a fil").unwrap();
    write(&file, "whole\n").expect("a write over a stray temp file");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "whole\n");
    assert!(!staging(&file).exists(), "the stray temp file is gone");
}

#[test]
fn a_failed_rename_keeps_the_old_file_and_removes_the_temp_file() {
    let dir = scratch("rename-fails");
    // A directory in the file's place: the rename cannot replace it.
    let file = dir.join("records.toml");
    std::fs::create_dir_all(file.join("inside")).unwrap();
    assert!(write(&file, "new").is_err());
    assert!(file.join("inside").is_dir(), "what was there is untouched");
    assert!(
        !staging(&file).exists(),
        "a failed write cleans up its temp file"
    );
}

#[test]
fn a_missing_directory_is_an_error_not_a_creation() {
    let dir = scratch("missing");
    assert!(write(dir.join("no-such").join("records.toml"), "x").is_err());
}

#[cfg(unix)]
#[test]
fn a_private_write_is_owner_only_from_the_first_byte() {
    use std::os::unix::fs::PermissionsExt;
    let dir = scratch("private");
    let file = dir.join("identity.toml");
    // A stray temp file with open permissions must not carry them into the secret.
    std::fs::write(staging(&file), "x").unwrap();
    write_private(&file, "secret").expect("writes");
    let mode = std::fs::metadata(&file).unwrap().permissions().mode();
    assert_eq!(mode & 0o777, 0o600);
}
