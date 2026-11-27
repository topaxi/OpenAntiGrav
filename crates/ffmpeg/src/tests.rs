use super::*;

fn p(s: &str) -> Option<PathBuf> {
    Some(PathBuf::from(s))
}

#[test]
fn a_system_tool_wins_unless_the_pinned_one_is_asked_for() {
    assert_eq!(
        pick(p("/usr/bin/ffmpeg"), p("/c/ffmpeg"), false),
        p("/usr/bin/ffmpeg")
    );
    assert_eq!(
        pick(p("/usr/bin/ffmpeg"), p("/c/ffmpeg"), true),
        p("/c/ffmpeg")
    );
}

#[test]
fn either_one_alone_is_used_and_neither_is_missing() {
    assert_eq!(pick(None, p("/c/ffmpeg"), false), p("/c/ffmpeg"));
    assert_eq!(
        pick(p("/usr/bin/ffmpeg"), None, false),
        p("/usr/bin/ffmpeg")
    );
    assert_eq!(pick(p("/usr/bin/ffmpeg"), None, true), p("/usr/bin/ffmpeg"));
    assert_eq!(pick(None, None, false), None);
    assert_eq!(pick(None, None, true), None);
}

fn temp(name: &str) -> oag_testdata::scratch::TempDir {
    oag_testdata::scratch::dir(&format!("ffmpeg-{name}"))
}

#[test]
fn path_is_searched_directory_by_directory() {
    let a = temp("path-a");
    let b = temp("path-b");
    std::fs::write(b.join(Tool::Ffmpeg.file_name()), b"x").unwrap();
    let paths = std::env::join_paths([&a, &b]).unwrap();
    assert_eq!(
        on_path_in(&paths, Tool::Ffmpeg),
        Some(b.join(Tool::Ffmpeg.file_name()))
    );
    assert_eq!(on_path_in(&paths, Tool::Ffprobe), None);
    assert_eq!(on_path_in(std::ffi::OsStr::new(""), Tool::Ffmpeg), None);
}

#[test]
fn an_installed_copy_is_found_under_the_pins_own_directory() {
    let Some(dir) = Pin::current().map(|pin| pin.dir_name()) else {
        return;
    };
    let root = temp("pinned");
    assert_eq!(pinned(&root, Tool::Ffmpeg), None);
    std::fs::create_dir_all(root.join(&dir)).unwrap();
    std::fs::write(root.join(&dir).join(Tool::Ffmpeg.file_name()), b"x").unwrap();
    assert_eq!(
        pinned(&root, Tool::Ffmpeg),
        Some(root.join(&dir).join(Tool::Ffmpeg.file_name()))
    );
    assert_eq!(pinned(&root, Tool::Ffprobe), None);
}

#[test]
fn every_pin_names_its_hash_size_and_an_lgpl_license() {
    for pin in Pin::ALL {
        assert_eq!(pin.sha256.len(), 64, "{}", pin.url);
        assert!(
            pin.sha256
                .bytes()
                .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
        );
        assert!(pin.url.starts_with("https://"));
        assert!(pin.url.contains(pin.version), "{}", pin.url);
        assert!(
            pin.url.contains("lgpl") && !pin.url.contains("-gpl"),
            "{}",
            pin.url
        );
        assert!(pin.license.starts_with("LGPL"));
        assert!(pin.size > 50_000_000);
    }
}

#[test]
fn a_process_with_the_pinned_copy_installed_resolves_it_ahead_of_nothing() {
    // The one test that touches the process-wide policy; nextest runs it alone in its process.
    let Some(dir) = Pin::current().map(|pin| pin.dir_name()) else {
        return;
    };
    let root = temp("policy");
    std::fs::create_dir_all(root.join(&dir)).unwrap();
    let ffmpeg = root.join(&dir).join(Tool::Ffmpeg.file_name());
    std::fs::write(&ffmpeg, b"x").unwrap();
    configure(Policy {
        use_pinned: true,
        root: Some(root.to_path_buf()),
    });
    assert_eq!(resolve(Tool::Ffmpeg), Some(ffmpeg.clone()));
    assert!(!wants_download());
    assert_eq!(command(Tool::Ffmpeg).get_program(), ffmpeg.as_os_str());
}
