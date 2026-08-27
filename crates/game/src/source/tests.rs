//! The search order, and what a directory of images contributes to it.
//!
//! A file of its own since the chooser arrived and this block passed the
//! 200-line ceiling `just check-size` holds inline test modules to. `use
//! super::*` still reaches `first_image`, `images_in` and the rest, so nothing
//! here changed in the move.

use std::ffi::OsString;

use super::*;

#[test]
fn a_named_source_is_used_verbatim() {
    // Including an archive spec, which is not a path at all.
    let spec = "data/images/pulse-psp-usa.chd:PSP_GAME/USRDIR/FE.wad";
    assert_eq!(resolve(Some(spec), None).unwrap(), spec);
    assert_eq!(
        resolve(Some("nonexistent.chd"), None).unwrap(),
        "nonexistent.chd"
    );
}

/// The DLC roots follow the same precedence, with `$OAG_DLC` left out of
/// the test on purpose: setting a process-wide variable would race the
/// other tests in this binary, and what is worth pinning is the ordering
/// this function decides rather than that `var_os` works.
#[test]
fn dlc_roots_follow_the_command_line_then_the_settings_file() {
    let cli = ["/from/cli".to_string()];
    let settings = ["/from/settings".to_string()];

    assert_eq!(
        resolve_dlc(&cli, &settings),
        [PathBuf::from("/from/cli")],
        "the command line wins"
    );
    assert_eq!(
        resolve_dlc(&[], &settings),
        [PathBuf::from("/from/settings")],
        "and the settings file is next"
    );
}

/// No DLC is the ordinary state of a copy of the game, so the empty case
/// falls through to the search path rather than failing or returning
/// nothing.
#[test]
fn dlc_roots_fall_back_to_the_search_path_and_never_fail() {
    // Only when the environment is not already pointing somewhere, which
    // is a real thing a developer's shell may do.
    if std::env::var_os(DLC_ENV).is_some() {
        return;
    }
    assert_eq!(resolve_dlc(&[], &[]), dlc_search_path());
    assert!(
        dlc_search_path().contains(&PathBuf::from("data/dlc")),
        "a checkout's own directory is always searched"
    );
}

/// The images and the packs are looked for in sibling directories, never
/// the same one: a disc image is not a pack and scanning one folder for
/// both would make every image a candidate archive.
#[test]
fn the_dlc_search_path_is_a_sibling_of_the_image_search_path() {
    for path in dlc_search_path() {
        assert!(
            !search_path().contains(&path),
            "{} is searched for both images and packs",
            path.display()
        );
    }
}

#[test]
fn a_settings_file_source_is_tried_before_the_search_path() {
    assert_eq!(
        resolve(None, Some("from-settings.chd")).unwrap(),
        "from-settings.chd"
    );
}

#[test]
fn the_command_line_wins_over_the_settings_file() {
    assert_eq!(
        resolve(Some("from-cli.chd"), Some("from-settings.chd")).unwrap(),
        "from-cli.chd"
    );
}

#[test]
fn the_checkouts_own_directory_is_searched_first() {
    assert_eq!(search_path().first().unwrap(), Path::new("data/images"));
}

/// The mounted AppImage must never be searched: an image found inside the
/// package would mean game content had been packaged, which is the one
/// thing `docs/overview/legal.md` forbids outright.
#[test]
fn the_mounted_appimage_is_not_searched() {
    for path in search_path() {
        let text = path.to_string_lossy().to_lowercase();
        assert!(
            !text.contains("/tmp/.mount_") && !text.contains("appdir"),
            "the package itself is on the search path: {}",
            path.display()
        );
    }
}

#[test]
fn a_known_name_wins_over_an_alphabetically_earlier_image() {
    let directory = temp_dir("known-name");
    std::fs::write(directory.join("aaa-other.iso"), b"").unwrap();
    std::fs::write(directory.join(IMAGE_NAMES[0]), b"").unwrap();

    assert_eq!(
        first_image(&directory).unwrap().file_name().unwrap(),
        std::ffi::OsStr::new(IMAGE_NAMES[0])
    );
    std::fs::remove_dir_all(&directory).unwrap();
}

/// `IMAGE_NAMES[0]` is interpolated into the not-found hint text, and the
/// module doc's "PSP first" rationale depends on this order.
#[test]
fn image_names_starts_with_pulse_psp_matching_the_hint_text() {
    assert_eq!(IMAGE_NAMES[0], "pulse-psp-usa.chd");
    assert_eq!(IMAGE_NAMES[1], "pulse-ps2-eu.chd");
}

/// The maintainer's actual `data/images/` holds all four documented
/// names at once - this is the check that extending `IMAGE_NAMES` to
/// recognise Pure and HD/Fury by name does not change which file that
/// directory resolves to.
#[test]
fn a_known_pulse_name_still_wins_when_every_documented_name_is_present() {
    let directory = temp_dir("all-four-names");
    for name in IMAGE_NAMES {
        std::fs::write(directory.join(name), b"").unwrap();
    }

    assert_eq!(
        first_image(&directory).unwrap().file_name().unwrap(),
        std::ffi::OsStr::new(IMAGE_NAMES[0])
    );
    std::fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn any_image_is_found_under_a_name_nobody_normalised() {
    let directory = temp_dir("any-name");
    std::fs::write(directory.join("notes.txt"), b"").unwrap();
    std::fs::write(directory.join("zzz.chd"), b"").unwrap();
    std::fs::write(directory.join("my-copy.ISO"), b"").unwrap();

    // Alphabetical, so the same image is opened on every run.
    assert_eq!(
        first_image(&directory).unwrap().file_name().unwrap(),
        std::ffi::OsStr::new("my-copy.ISO")
    );
    std::fs::remove_dir_all(&directory).unwrap();
}

/// Every container, not the first one - and the known name still leads, so
/// a chooser's first row is what a boot with nothing named would have
/// opened.
#[test]
fn a_directory_contributes_every_image_it_holds_known_names_first() {
    let directory = temp_dir("every-image");
    std::fs::write(directory.join("aaa-other.iso"), b"").unwrap();
    std::fs::write(directory.join("notes.txt"), b"").unwrap();
    std::fs::write(directory.join(IMAGE_NAMES[0]), b"").unwrap();

    let found = images_in(&directory);
    assert_eq!(
        found,
        [
            directory.join(IMAGE_NAMES[0]),
            directory.join("aaa-other.iso")
        ]
    );
    assert_eq!(first_image(&directory), found.first().cloned());
    std::fs::remove_dir_all(&directory).unwrap();
}

/// A name that is both a known one and on disk must not be offered twice.
#[test]
fn a_known_name_is_listed_once() {
    let directory = temp_dir("listed-once");
    for name in IMAGE_NAMES {
        std::fs::write(directory.join(name), b"").unwrap();
    }

    let found = images_in(&directory);
    assert_eq!(found.len(), IMAGE_NAMES.len(), "{found:?}");
    std::fs::remove_dir_all(&directory).unwrap();
}

/// The whole point of the split: the scan is a guess and `explicit` is not
/// part of it, so a chooser is only ever offered when nothing was stated.
#[test]
fn explicit_covers_the_command_line_and_the_settings_file_and_nothing_else() {
    assert_eq!(
        explicit(Some("from-cli.chd"), Some("from-settings.chd")).unwrap(),
        Some("from-cli.chd".to_string())
    );
    assert_eq!(
        explicit(None, Some("from-settings.chd")).unwrap(),
        Some("from-settings.chd".to_string())
    );
    // Only when the developer's own shell is not already pointing
    // somewhere, which is a real thing it may do.
    if std::env::var_os(IMAGE_ENV).is_none() {
        assert_eq!(explicit(None, None).unwrap(), None, "no scan in here");
    }
}

/// **`$OAG_IMAGE` naming something unusable must fail, not fall through.**
///
/// The silent regression this guards is the worst one available here: an
/// `Ok(None)` would send `resolve` on to the search path and boot a
/// *different* disc than the variable named, with nothing said. Checked
/// through `stated`, which takes the value rather than reading it - see the
/// note there on why the variable itself is never set by a test.
#[test]
fn an_oag_image_that_names_nothing_usable_is_an_error() {
    let nothing = OsString::from("/nonexistent/oag/not-an-image");

    let error = stated(None, Some(nothing.clone()), Some("from-settings.chd"))
        .expect_err("a source that cannot be used must not fall through");
    assert!(error.to_string().contains(IMAGE_ENV), "{error}");

    // The command line still wins over it, unread.
    assert_eq!(
        stated(Some("from-cli.chd"), Some(nothing), None).unwrap(),
        Some("from-cli.chd".to_string())
    );
}

/// A variable naming a real image is used, and one naming a *directory* picks
/// from it the same way the search path does - which is what keeps a chooser
/// out of the way of someone who has stated where their disc is.
#[test]
fn an_oag_image_naming_a_file_or_a_directory_is_used() {
    let directory = temp_dir("env-directory");
    std::fs::write(directory.join(IMAGE_NAMES[0]), b"").unwrap();

    assert_eq!(
        stated(None, Some(directory.clone().into_os_string()), None).unwrap(),
        Some(
            directory
                .join(IMAGE_NAMES[0])
                .to_string_lossy()
                .into_owned()
        )
    );

    let file = directory.join(IMAGE_NAMES[0]);
    assert_eq!(
        stated(None, Some(file.clone().into_os_string()), None).unwrap(),
        Some(file.to_string_lossy().into_owned())
    );
    std::fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn a_missing_directory_contributes_nothing_rather_than_failing() {
    assert!(images_in(Path::new("/nonexistent/oag/images")).is_empty());
}

#[test]
fn a_directory_with_nothing_in_it_is_not_a_source() {
    let directory = temp_dir("empty");
    assert_eq!(first_image(&directory), None);
    std::fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn a_missing_directory_is_not_an_error() {
    assert_eq!(first_image(Path::new("/nonexistent/oag/images")), None);
}

/// A 2048 extract is a directory, not a file, so [`images_in`]'s own scan
/// never finds it - [`package_directories_in`] is the other half.
#[test]
fn an_extracted_package_is_found_by_its_own_shape() {
    let root = temp_dir("packages");
    std::fs::create_dir_all(root.join("PCSF00007/base/PSP2")).unwrap();
    std::fs::write(root.join("PCSF00007/base/PSP2/data.psarc"), b"").unwrap();
    // A partial extract - decrypted but not yet unpacked this far, or a
    // directory that is not a 2048 package at all - must not be offered as
    // if it opens.
    std::fs::create_dir_all(root.join("PCSA00015/base")).unwrap();
    std::fs::create_dir_all(root.join("notes")).unwrap();

    assert_eq!(
        package_directories_in(&root),
        [root.join("PCSF00007")],
        "only the directory that actually has PSP2/data.psarc under base/"
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_missing_package_root_contributes_nothing_rather_than_failing() {
    assert!(package_directories_in(Path::new("/nonexistent/oag/vita")).is_empty());
}

/// [`candidates`] appends package directories after every image, so a
/// no-argument boot on a machine with both keeps opening the image it always
/// did - see [`candidates`]'s own doc comment.
#[test]
fn package_directories_are_appended_after_every_image_not_folded_in() {
    let root = temp_dir("packages-order");
    std::fs::write(root.join(IMAGE_NAMES[0]), b"").unwrap();
    std::fs::create_dir_all(root.join("PCSF00007/base/PSP2")).unwrap();
    std::fs::write(root.join("PCSF00007/base/PSP2/data.psarc"), b"").unwrap();

    let mut found = images_in(&root);
    found.extend(package_directories_in(&root));
    assert_eq!(
        found,
        [root.join(IMAGE_NAMES[0]), root.join("PCSF00007")],
        "the image still leads"
    );
    std::fs::remove_dir_all(&root).unwrap();
}

/// A directory of this test's own, so the tests do not depend on - or
/// disturb - whatever the developer has in `data/images`.
fn temp_dir(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!("oag-source-{name}"));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).unwrap();
    directory
}
