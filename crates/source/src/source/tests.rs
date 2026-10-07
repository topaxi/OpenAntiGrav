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

/// `PCSA00015` (USA) sorts before `PCSF00007` (Europe) alphabetically; the
/// Europe-first policy puts the European extract ahead of it anyway.
#[test]
fn a_european_2048_extract_is_listed_before_a_usa_one() {
    let root = temp_dir("vita-eu-first");
    for serial in ["PCSA00015", "PCSF00007"] {
        let data = root.join(serial).join("base/PSP2");
        std::fs::create_dir_all(&data).unwrap();
        std::fs::write(data.join("data.psarc"), b"").unwrap();
    }
    assert_eq!(
        package_directories_in(&root),
        [root.join("PCSF00007"), root.join("PCSA00015")]
    );
    std::fs::remove_dir_all(&root).unwrap();
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
/// module doc's "PSP first" rationale depends on this order. Europe leads the
/// USA pressing for every title that has both - see [`IMAGE_NAMES`]'s doc.
#[test]
fn image_names_starts_with_pulse_psp_matching_the_hint_text() {
    assert_eq!(IMAGE_NAMES[0], "pulse-psp-eu.chd");
    assert_eq!(IMAGE_NAMES[1], "pulse-psp-usa.chd");
    assert_eq!(IMAGE_NAMES[2], "pulse-ps2-eu.chd");
}

/// A directory holding both pressings of one title opens the EU one, and the
/// USA one is still found when it is the only one there.
#[test]
fn europe_is_preferred_over_usa_when_a_directory_holds_both() {
    for (eu, usa) in [
        ("pulse-psp-eu.chd", "pulse-psp-usa.chd"),
        ("pure-psp-eu.chd", "pure-psp-usa.chd"),
    ] {
        let directory = temp_dir("eu-first");
        std::fs::write(directory.join(usa), b"").unwrap();
        assert_eq!(
            first_image(&directory).unwrap().file_name().unwrap(),
            std::ffi::OsStr::new(usa)
        );
        std::fs::write(directory.join(eu), b"").unwrap();
        assert_eq!(
            first_image(&directory).unwrap().file_name().unwrap(),
            std::ffi::OsStr::new(eu)
        );
        std::fs::remove_dir_all(&directory).unwrap();
    }
}

/// The maintainer's actual `data/images/` holds every documented name at
/// once - this is the check that extending `IMAGE_NAMES` to recognise Pure,
/// HD/Fury and, since 2026-09-07, Pulse's EU disc by name does not change
/// which file that directory resolves to.
#[test]
fn a_known_pulse_name_still_wins_when_every_documented_name_is_present() {
    let directory = temp_dir("all-names");
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

/// The bug a portable deploy actually hit: a 2048 extract dropped beside the
/// AppImage or in the stable data directory, the same two places a disc
/// image is found in either of those situations, must not be invisible just
/// because the current directory has no `data/` in it.
#[test]
fn the_package_search_path_covers_the_same_ground_as_the_image_search_path() {
    assert_eq!(
        package_search_path().first().unwrap(),
        Path::new("data/extracted/vita"),
        "the checkout's own directory is searched first, same as search_path"
    );
}

/// One package root per image root, and each one is that same image root
/// with its trailing `images` swapped for `extracted/vita` - not just the
/// same *count*, which would pass by coincidence if either list's shape ever
/// changed without the other.
///
/// `$APPIMAGE` deliberately left unset, the same reason
/// `dlc_roots_fall_back_to_the_search_path_and_never_fail` leaves `$OAG_DLC`
/// alone: it is a real thing a developer's own shell may already be setting,
/// and both `search_path` and `package_search_path` read it straight from the
/// environment rather than taking it as an argument.
#[test]
fn the_package_search_path_mirrors_the_image_search_path_root_for_root() {
    if std::env::var_os("APPIMAGE").is_some() {
        return;
    }

    let images = search_path();
    let packages = package_search_path();
    assert_eq!(
        images.len(),
        packages.len(),
        "one package root per image root: {images:?} vs {packages:?}"
    );

    for (image_root, package_root) in images.iter().zip(&packages) {
        // "images" is always the last component `search_path` adds, whether
        // it is `data/images` or `<data dir>/oag/images` - swapping it for
        // `extracted/vita` turns one list into the other exactly.
        let mut expected = image_root.clone();
        assert!(expected.pop(), "{image_root:?} has no trailing component");
        expected.push("extracted");
        expected.push("vita");
        assert_eq!(
            package_root, &expected,
            "{package_root:?} does not mirror image root {image_root:?}"
        );
    }
}

/// Never the mounted AppImage itself, for the same reason `search_path`
/// excludes it: a package found inside `$APPDIR` would mean game content had
/// been packaged.
#[test]
fn the_mounted_appimage_is_not_searched_for_a_package_either() {
    for path in package_search_path() {
        let text = path.to_string_lossy().to_lowercase();
        assert!(
            !text.contains("/tmp/.mount_") && !text.contains("appdir"),
            "the package itself is on the search path: {}",
            path.display()
        );
    }
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

/// Unlike a 2048 copy, the PS4 root itself is the candidate - see
/// [`ps4_package_directories`]'s own doc for why the shapes differ.
#[test]
fn a_ps4_root_is_found_by_its_patchs_own_shape() {
    let root = temp_dir("ps4-found");
    std::fs::create_dir_all(root.join("omega-eu-patch/uroot")).unwrap();
    std::fs::write(root.join("omega-eu-patch/uroot/data09.psarc"), b"").unwrap();

    assert_eq!(
        ps4_package_directories_in(std::slice::from_ref(&root)),
        std::slice::from_ref(&root)
    );
    std::fs::remove_dir_all(&root).unwrap();
}

/// A base-only extract, with no patch, is not offered - it has no
/// `skin.xml` to boot, the same reason `just play omega` refuses it.
#[test]
fn a_base_only_ps4_extract_is_not_offered() {
    let root = temp_dir("ps4-base-only");
    std::fs::create_dir_all(root.join("omega-eu/uroot")).unwrap();
    std::fs::write(root.join("omega-eu/uroot/data00.psarc"), b"").unwrap();

    assert!(ps4_package_directories_in(std::slice::from_ref(&root)).is_empty());
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn a_missing_ps4_root_contributes_nothing_rather_than_failing() {
    assert!(ps4_package_directories_in(&[PathBuf::from("/nonexistent/oag/ps4")]).is_empty());
}

/// The same portable-deploy ground [`the_package_search_path_covers_the_same_ground_as_the_image_search_path`]
/// covers for 2048, for Omega's own PS4 extract.
#[test]
fn the_ps4_search_path_covers_the_same_ground_as_the_image_search_path() {
    assert_eq!(
        ps4_search_path().first().unwrap(),
        Path::new("data/extracted/ps4"),
        "the checkout's own directory is searched first, same as search_path"
    );
}

/// Never the mounted AppImage itself, for the same reason `search_path`
/// excludes it.
#[test]
fn the_mounted_appimage_is_not_searched_for_a_ps4_root_either() {
    for path in ps4_search_path() {
        let text = path.to_string_lossy().to_lowercase();
        assert!(
            !text.contains("/tmp/.mount_") && !text.contains("appdir"),
            "the package itself is on the search path: {}",
            path.display()
        );
    }
}

/// A directory of this test's own, so the tests do not depend on - or
/// disturb - whatever the developer has in `data/images`.
/// A 2048 extract and an Omega extract are found by the no-argument path, not
/// only by the chooser, and a disc image still beats both.
#[test]
fn an_unpacked_folder_is_found_without_an_image() {
    let vita = temp_dir("vita-root");
    let copy = vita.join("PCSF00007");
    std::fs::create_dir_all(copy.join("base/PSP2")).unwrap();
    std::fs::write(copy.join("base/PSP2/data.psarc"), b"").unwrap();
    let ps4 = temp_dir("ps4-root");
    let data09 = ps4.join("omega-eu-patch").join(oag_omega::archives::DATA09);
    std::fs::create_dir_all(data09.parent().unwrap()).unwrap();
    std::fs::write(&data09, b"").unwrap();

    assert_eq!(
        first_package(std::slice::from_ref(&vita), std::slice::from_ref(&ps4), &[]),
        Some(copy.clone())
    );
    assert_eq!(
        first_package(&[], std::slice::from_ref(&ps4), &[]),
        Some(ps4.clone())
    );
    assert_eq!(first_package(&[], &[], &[]), None);

    std::fs::remove_dir_all(&vita).unwrap();
    std::fs::remove_dir_all(&ps4).unwrap();
}

/// `$OAG_IMAGE` takes the same unpacked folders the command line does: the
/// package itself, or the folder holding it.
#[test]
fn an_oag_image_naming_an_unpacked_folder_is_used() {
    let vita = temp_dir("env-vita");
    let copy = vita.join("PCSF00007");
    std::fs::create_dir_all(copy.join("base/PSP2")).unwrap();
    std::fs::write(copy.join("base/PSP2/data.psarc"), b"").unwrap();
    let want = Some(copy.to_string_lossy().into_owned());

    assert_eq!(
        stated(None, Some(copy.clone().into_os_string()), None).unwrap(),
        want
    );
    assert_eq!(
        stated(None, Some(vita.clone().into_os_string()), None).unwrap(),
        want
    );
    std::fs::remove_dir_all(&vita).unwrap();
}

/// The decrypted HD image is tried before the encrypted one beside it.
#[test]
fn the_decrypted_hd_image_wins_over_the_encrypted_one() {
    let directory = temp_dir("hd-dec");
    std::fs::write(directory.join("hdfury-ps3-eu.iso"), b"").unwrap();
    std::fs::write(directory.join("hdfury-ps3-eu-dec.iso"), b"").unwrap();
    assert_eq!(
        first_image(&directory),
        Some(directory.join("hdfury-ps3-eu-dec.iso"))
    );
    std::fs::remove_dir_all(&directory).unwrap();
}

/// The not-found text names every form and place it looked, `.pkg` included,
/// and gives the beside-the-executable advice only for an AppImage.
#[test]
fn the_not_found_error_names_every_place_and_form() {
    let text = nothing_found(&search_path()).to_string();
    for needle in [
        "data/images",
        "data/extracted/vita",
        "data/extracted/ps4",
        ".pkg",
        "installing.md",
        ".dkey",
    ] {
        assert!(text.contains(needle), "missing {needle}: {text}");
    }
    if std::env::var_os("APPIMAGE").is_none() {
        assert!(!text.contains("beside the AppImage"), "{text}");
    }
}

/// A `PARAM.SFO` holding one `TITLE_ID`.
fn sfo_with(title_id: &str) -> Vec<u8> {
    let key = b"TITLE_ID\0";
    let value = [title_id.as_bytes(), &[0]].concat();
    let key_table = 20 + 16;
    let data_table = key_table + key.len();
    let mut out = b"\0PSF".to_vec();
    out.extend_from_slice(&0x101_u32.to_le_bytes());
    out.extend_from_slice(&u32::try_from(key_table).unwrap().to_le_bytes());
    out.extend_from_slice(&u32::try_from(data_table).unwrap().to_le_bytes());
    out.extend_from_slice(&1_u32.to_le_bytes());
    out.extend_from_slice(&0_u16.to_le_bytes());
    out.extend_from_slice(&0x0204_u16.to_le_bytes());
    out.extend_from_slice(&u32::try_from(value.len()).unwrap().to_le_bytes());
    out.extend_from_slice(&16_u32.to_le_bytes());
    out.extend_from_slice(&0_u32.to_le_bytes());
    out.extend_from_slice(key);
    out.extend_from_slice(&value);
    out
}

/// A PSN HD install is `PARAM.SFO` beside `USRDIR`, and nothing else is offered:
/// not a decrypted disc extract (those keep both under `PS3_GAME/`), not a raw
/// zip extract, not a folder with only one of the two. A European `TITLE_ID`
/// sorts ahead of an American one whatever the folders are called.
#[test]
fn a_psn_hd_install_is_found_by_its_shape_and_europe_sorts_first() {
    let root = temp_dir("ps3-installs");
    for (folder, serial) in [("a-us", "NPUA80001"), ("z-eu", "NPEA00057")] {
        std::fs::create_dir_all(root.join(folder).join("USRDIR")).unwrap();
        std::fs::write(root.join(folder).join("PARAM.SFO"), sfo_with(serial)).unwrap();
    }
    std::fs::create_dir_all(root.join("disc/PS3_GAME/USRDIR")).unwrap();
    std::fs::write(root.join("disc/PS3_GAME/PARAM.SFO"), sfo_with("BCES00664")).unwrap();
    std::fs::create_dir_all(root.join("zip-extract")).unwrap();
    std::fs::write(root.join("zip-extract/PARAM.SFO"), sfo_with("NPEA00057")).unwrap();

    assert_eq!(
        ps3_package_directories_in(&root),
        [root.join("z-eu"), root.join("a-us")]
    );
    assert_eq!(package_at(&root.join("a-us")), Some(root.join("a-us")));
    assert_eq!(package_at(&root.join("zip-extract")), None);
    std::fs::remove_dir_all(&root).unwrap();
}

fn temp_dir(name: &str) -> PathBuf {
    let directory = std::env::temp_dir().join(format!("oag-source-{name}"));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).unwrap();
    directory
}

/// A Windows zip or a tarball has no `$APPIMAGE`: the executable's own folder is
/// the portable one. Inside an AppImage the executable is under the mounted
/// `$APPDIR`, which is never searched, so the AppImage's folder wins.
#[test]
fn the_portable_folder_is_the_appimages_else_the_executables() {
    let exe = Some(PathBuf::from("/mnt/appdir/usr/bin/oag-game"));
    assert_eq!(
        portable_directory_of(Some("/home/deck/Games/oag.AppImage".into()), exe.clone()),
        Some(PathBuf::from("/home/deck/Games"))
    );
    assert_eq!(
        portable_directory_of(None, Some(PathBuf::from("C:/Games/OAG/oag-game.exe"))),
        Some(PathBuf::from("C:/Games/OAG"))
    );
    assert_eq!(portable_directory_of(None, None), None);
}

/// With no `$APPIMAGE`, an `images` folder next to the program is searched, and
/// so is the folder itself.
#[test]
fn an_images_folder_beside_the_program_is_on_the_search_path() {
    if std::env::var_os("APPIMAGE").is_some() {
        return;
    }
    let beside = portable_directory().expect("a test binary has a folder");
    let paths = search_path();
    assert!(paths.contains(&beside), "{paths:?}");
    assert!(paths.contains(&beside.join("images")), "{paths:?}");
}
