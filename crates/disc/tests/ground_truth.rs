//! Validates `oag-disc` against real disc images and against `chdman`.
//!
//! **Every test here is `#[ignore]`d and never runs in CI.** They need game
//! content, which this project does not and will not ship. See
//! `docs/architecture/adr/0006-no-copyrighted-content.md`.
//!
//! Run them yourself once `data/images/` is populated:
//!
//! ```sh
//! just test-data
//! ```
//!
//! Because CI cannot run these, they are the tests most likely to rot. Run them
//! before any release, and after any change to `oag-disc`.
//!
//! Tests skip with a printed message when their image is absent, rather than
//! failing, so a partially populated `data/images/` is usable.

use std::path::PathBuf;
use std::process::Command;

use oag_disc::{DiscImage, Platform};

/// **`exact`, not `image`.** This file's whole subject is the container: it
/// walks a CHD and its own extracted `.iso` and diffs the two listings.
/// `oag_testdata::image` would substitute the extract for the CHD when one is
/// present, and the comparison would be a file against itself - green, and
/// asserting nothing.
///
/// It also gains the `OAG_REQUIRE_GAME_DATA` guard it never had: this was one
/// of the copies that skipped silently even when a run had declared the data
/// present.
fn image(name: &str) -> Option<PathBuf> {
    oag_testdata::exact(name)
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn psp_pulse_is_identified_from_its_own_contents() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open");

    let title = disc.identify().expect("identify");
    assert_eq!(title.platform, Platform::Psp);
    assert_eq!(title.serial.as_deref(), Some("UCUS-98712"));

    // The unencrypted executable is the whole reason M2 can start. If this
    // stops being true, something is wrong with the image or the reader.
    let boot = disc
        .entries()
        .expect("entries")
        .iter()
        .find(|e| e.path == "PSP_GAME/SYSDIR/BOOT.BIN")
        .expect("BOOT.BIN present")
        .clone();

    let head = disc.read_entry_head(&boot, 4).expect("read");
    assert_eq!(&head, b"\x7fELF", "BOOT.BIN should be an unencrypted ELF");
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn ps2_pulse_reads_through_a_cd_wrapped_dvd() {
    let Some(path) = image("pulse-ps2-eu.chd") else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open");

    // This image is a DVD packed with `chdman createcd`: the container reports
    // 2448-byte CD units, but the track is cooked MODE1, so the user data sits
    // at offset 0. Reading at the raw-frame offset of 16 produces plausible
    // garbage rather than an error, so this test is the guard against a
    // regression that would otherwise be silent.
    let title = disc.identify().expect("identify");
    assert_eq!(title.platform, Platform::Ps2);
    assert_eq!(title.serial.as_deref(), Some("SCES-54748"));

    let pvd = disc.volume_descriptor().expect("pvd");
    assert_eq!(pvd.logical_block_size, 2048);
    assert_eq!(pvd.volume_space_size, 1_900_848);
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn pure_uses_the_same_disc_structure_as_pulse() {
    let Some(path) = image("pure-psp-usa.chd") else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open");

    let title = disc.identify().expect("identify");
    assert_eq!(title.platform, Platform::Psp);
    assert_eq!(title.serial.as_deref(), Some("UCUS-98612"));

    // The shared archive naming is the evidence that one asset pipeline can
    // serve both titles.
    let paths: Vec<_> = disc
        .entries()
        .expect("entries")
        .iter()
        .map(|e| e.path.clone())
        .collect();

    for expected in [
        "PSP_GAME/USRDIR/Data.wad",
        "PSP_GAME/USRDIR/FE.wad",
        "PSP_GAME/USRDIR/FEData.wad",
    ] {
        assert!(paths.iter().any(|p| p == expected), "missing {expected}");
    }
}

#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_ps3_disc_identifies_itself_even_while_encrypted() {
    let Some(path) = image("hdfury-ps3-eu.iso") else {
        return;
    };
    let mut disc = DiscImage::open(&path).expect("open");

    // `PS3_DISC.SFB` is in a plain region, so this holds for the encrypted
    // image as shipped - nothing here needs the disc key, and nothing here
    // should ever start needing it.
    let title = disc.identify().expect("identify");
    assert_eq!(title.platform, Platform::Ps3);
    assert_eq!(title.serial.as_deref(), Some("BCES-00664"));
    assert_eq!(
        title.boot_path.as_deref(),
        Some("PS3_GAME/USRDIR/EBOOT.BIN")
    );

    // The archives are encrypted and stay that way; what this asserts is that
    // the filesystem walk reaches them, which is what identification rests on.
    let paths: Vec<_> = disc
        .entries()
        .expect("entries")
        .iter()
        .map(|e| e.path.clone())
        .collect();

    for expected in [
        "PS3_DISC.SFB",
        "PS3_GAME/PARAM.SFO",
        "PS3_GAME/USRDIR/EBOOT.BIN",
    ] {
        assert!(paths.iter().any(|p| p == expected), "missing {expected}");
    }
}

#[test]
#[ignore = "needs a disc image in data/images/ and chdman on PATH"]
fn listing_matches_chdman_and_the_reference_iso() {
    let Some(path) = image("pulse-psp-usa.chd") else {
        return;
    };
    if Command::new("chdman").arg("--help").output().is_err() {
        println!("skipping: chdman not on PATH (install mame-tools)");
        return;
    }

    let iso = std::env::temp_dir().join("oag-ground-truth.iso");
    let status = Command::new("chdman")
        .args(["extractdvd", "-f", "-i"])
        .arg(&path)
        .arg("-o")
        .arg(&iso)
        .status()
        .expect("run chdman");
    assert!(status.success(), "chdman extractdvd failed");

    // Reading the extracted ISO exercises RawSource; reading the CHD exercises
    // ChdSource plus decompression. Both must produce an identical listing, so
    // a bug in either path shows up here.
    let mut from_chd = DiscImage::open(&path).expect("open chd");
    let mut from_iso = DiscImage::open(&iso).expect("open iso");

    let a: Vec<_> = from_chd.entries().expect("chd entries").to_vec();
    let b: Vec<_> = from_iso.entries().expect("iso entries").to_vec();

    assert_eq!(a.len(), b.len(), "entry counts differ");
    for (x, y) in a.iter().zip(b.iter()) {
        assert_eq!(x, y, "entry mismatch between CHD and extracted ISO");
    }

    // Spot-check that the bytes agree too, not only the metadata.
    let boot = a
        .iter()
        .find(|e| e.path == "PSP_GAME/SYSDIR/BOOT.BIN")
        .expect("BOOT.BIN");
    assert_eq!(
        from_chd.read_entry(boot).expect("read from chd"),
        from_iso.read_entry(boot).expect("read from iso"),
        "BOOT.BIN differs between the CHD and the extracted ISO"
    );

    std::fs::remove_file(&iso).ok();
}

/// `ChdSource::read_sectors` splits a long read across threads; the bytes must
/// be exactly what reading the same sectors one at a time gives, including a
/// range that starts and ends inside a hunk. 40 MiB of the PS2 disc's music
/// archive, raw PCM in LZMA hunks, is the read a race track makes.
#[test]
#[ignore = "needs a disc image in data/images/"]
fn a_split_chd_read_is_byte_identical_to_a_sector_by_sector_one() {
    use oag_disc::chd_source::ChdSource;
    use oag_disc::{SECTOR_SIZE, SectorSource};
    let Some(path) = image("pulse-ps2-eu.chd") else {
        return;
    };
    let music = DiscImage::open(&path)
        .expect("open")
        .entries()
        .expect("entries")
        .iter()
        .find(|e| e.path == "54748/PS2MUSIC.WAD")
        .expect("the music archive")
        .lba;
    let mut source = ChdSource::open(&path).expect("open");
    let (lba, count) = (music + 3, 20_011);
    let split = source.read_sectors(lba, count).expect("split read");
    let mut serial = vec![0u8; count as usize * SECTOR_SIZE];
    for (at, sector) in (lba..).zip(serial.as_chunks_mut::<SECTOR_SIZE>().0) {
        source.read_sector(at, sector).expect("sector read");
    }
    assert!(
        split == serial,
        "the split read differs from the serial one"
    );
    assert!(source.read_sectors(source.sector_count() - 1, 2).is_err());
}
