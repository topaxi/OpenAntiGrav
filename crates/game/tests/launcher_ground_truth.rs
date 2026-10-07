//! What the chooser says about the images this machine actually has.
//!
//! `launcher::tests` covers the cursor and the draw list with candidates built
//! by hand. This is the other half: [`oag_game::launcher::survey`] reading real
//! discs, which is the only way to check that the title, platform and serial on
//! a row are the disc's own rather than a guess - and the only way to check the
//! one case the whole "list it anyway" decision exists for, an encrypted PS3
//! image sitting beside its decrypted twin.
//!
//! `#[ignore]`d: needs real images under `data/images/`. `just test-data`.

use std::path::PathBuf;

use oag_game::launcher::{self, State};

/// **`exact`, not `image`.** Every test here is about the image *files* rather
/// than their contents: one asserts the serial each named pressing carries, and
/// the other reads the directory the images live in and expects a row per file.
/// `oag_testdata::image` substitutes `data/cache/<stem>.iso` for a CHD when an
/// extract is present, which points both of those at the wrong file - the
/// second at `data/cache` entire, which is movies and audio and no images at
/// all.
fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::exact(name)
}

/// Every row is labelled off the disc, not off its file name.
#[test]
#[ignore = "needs a disc image under data/images/"]
fn each_pressing_identifies_as_its_own_title() {
    // Every serial here was read off the maintainer's own images with
    // `oag-unpack info`, not copied from a database.
    let expected = [
        ("pulse-psp-usa.chd", "Wipeout Pulse", "PSP", "UCUS-98712"),
        ("pulse-psp-eu.chd", "Wipeout Pulse", "PSP", "UCES-00465"),
        ("pulse-ps2-eu.chd", "Wipeout Pulse", "PS2", "SCES-54748"),
        ("pure-psp-eu.chd", "Wipeout Pure", "PSP", "UCES-00001"),
        ("pure-psp-usa.chd", "Wipeout Pure", "PSP", "UCUS-98612"),
        ("hdfury-ps3-eu-dec.iso", "Wipeout HD", "PS3", "BCES-00664"),
    ];

    for (name, title, platform, serial) in expected {
        let Some(path) = image(name) else { continue };
        let rows = launcher::survey(&[path]);
        let row = rows.first().expect("one path in, one row out");

        assert_eq!(row.name, name);
        assert_eq!(row.title(), title, "{name}");
        assert_eq!(row.platform.to_string(), platform, "{name}");
        assert_eq!(row.serial.as_deref(), Some(serial), "{name}");
        assert!(row.is_playable(), "{name} should open");
    }
}

/// The reason the unavailable row exists at all.
///
/// The encrypted image and the decrypted one carry the same serial - the
/// `PS3_DISC.SFB` that answers it is outside the encrypted region - so nothing
/// short of trying to open the archives can tell them apart, and a chooser that
/// filtered on identity alone would offer the wrong one.
#[test]
#[ignore = "needs a disc image under data/images/"]
fn an_encrypted_ps3_image_is_listed_with_the_fix_rather_than_hidden() {
    let Some(encrypted) = image("hdfury-ps3-eu.iso") else {
        return;
    };
    // A symlink in a directory of its own, so the `.dkey` that sits beside the
    // real image is not beside this one.
    let dir = std::env::temp_dir().join(format!("oag-launcher-locked-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let link = dir.join("hdfury-ps3-eu.iso");
    let _ = std::fs::remove_file(&link);
    std::os::unix::fs::symlink(&encrypted, &link).unwrap();

    let rows = launcher::survey(std::slice::from_ref(&link));
    let row = rows.first().expect("one path in, one row out");
    assert_eq!(row.platform.to_string(), "PS3");
    assert_eq!(
        row.serial.as_deref(),
        Some("BCES-00664"),
        "the identity block reads through the encryption"
    );
    let keys_stored = oag_disc::ps3_crypt::keys_dir()
        .and_then(|d| std::fs::read_dir(d).ok())
        .is_some_and(|mut d| d.next().is_some());
    if keys_stored {
        // This machine's own keys directory unlocked it; that path is the
        // `ps3_crypt_ground_truth` suite's, not this one's.
        std::fs::remove_dir_all(&dir).ok();
        return;
    }
    assert!(
        !row.is_playable(),
        "a locked image must not be offered to play"
    );
    assert!(row.is_selectable(), "a locked image offers the key prompt");
    assert!(matches!(row.state, State::NeedsKey), "{:?}", row.state);
    std::fs::remove_dir_all(&dir).ok();
}

/// The PS4 extract is a directory rather than a disc image, so
/// `oag_source::source::candidates` needs its own scan for it - see
/// `oag_source::source::ps4_search_path` and its own doc for why that shape
/// differs from 2048's. This is the ground truth for that scan: a real
/// extract under `data/extracted/ps4` (the checkout's own, not a substitute)
/// both appears in `candidates()` and surveys as Omega.
#[test]
#[ignore = "needs an Omega Collection PS4 extract under data/extracted/ps4"]
fn the_ps4_extract_is_a_candidate_and_surveys_as_omega() {
    let Some(root) = oag_source::source::candidates()
        .into_iter()
        .find(|path| path.ends_with("ps4"))
    else {
        return;
    };

    let rows = launcher::survey(&[root]);
    let row = rows.first().expect("one path in, one row out");
    assert_eq!(row.title(), "Wipeout: Omega Collection");
    assert!(row.is_playable(), "{row:?} should open");
}

/// The chooser only ever appears because the search path holds more than one
/// image, so this is the state the whole feature is for.
///
/// The directory is reached through [`image`] rather than
/// `oag_source::source::candidates`, whose first search-path entry is
/// `data/images` **relative to the current directory** - the workspace root
/// when the game runs, and the crate directory under `cargo nextest`. That is
/// the composition root's policy working as intended and not something to bend
/// a test around; what is worth checking here is that a folder of several real
/// images surveys into several real rows.
#[test]
#[ignore = "needs a disc image under data/images/"]
fn a_folder_of_several_images_surveys_into_several_rows() {
    let Some(one) = image("pulse-psp-usa.chd") else {
        return;
    };
    let directory = one.parent().expect("images live in a directory");

    let mut paths: Vec<PathBuf> = std::fs::read_dir(directory)
        .expect("the directory reads")
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file() && oag_source::source::is_container(path))
        .collect();
    paths.sort();
    assert!(paths.len() > 1, "one image needs no chooser: {paths:?}");

    let rows = launcher::survey(&paths);
    assert_eq!(rows.len(), paths.len());
    assert!(
        rows.iter().any(oag_game::launcher::Candidate::is_playable),
        "nothing here opens"
    );
    for row in &rows {
        eprintln!("{:<14} {:<18} {}", row.title(), row.provenance(), row.name);
    }
}
