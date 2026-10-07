//! The encrypted PS3 image read in place against the decrypted copy of the same disc.
//!
//! **`#[ignore]`d: needs `hdfury-ps3-eu.iso`, its `.dkey` beside it, and
//! `hdfury-ps3-eu-dec.iso`** in `data/images/`. The key is read inside the
//! reader; this file never names or prints it.

use oag_disc::{DiscImage, Ps3State, SECTOR_SIZE};

fn pair() -> Option<(DiscImage, DiscImage)> {
    let enc = oag_testdata::exact("hdfury-ps3-eu.iso")?;
    let dec = oag_testdata::exact("hdfury-ps3-eu-dec.iso")?;
    Some((
        DiscImage::open(enc).expect("open encrypted"),
        DiscImage::open(dec).expect("open decrypted"),
    ))
}

#[test]
#[ignore = "needs the encrypted and decrypted HD images and the key beside them"]
fn the_encrypted_image_unlocks_with_its_sibling_key_and_the_decrypted_one_is_left_plain() {
    let Some((enc, dec)) = pair() else { return };
    assert_eq!(enc.ps3_state(), Ps3State::Decrypting);
    assert_eq!(
        dec.ps3_state(),
        Ps3State::AlreadyPlain,
        "a decrypted dump keeps sector 0's table and must not be decrypted twice"
    );
}

#[test]
#[ignore = "needs the encrypted and decrypted HD images and the key beside them"]
fn every_file_on_the_disc_reads_byte_identical_to_the_decrypted_copy() {
    let Some((mut enc, mut dec)) = pair() else {
        return;
    };
    let entries: Vec<_> = enc
        .entries()
        .expect("entries")
        .iter()
        .filter(|e| !e.is_directory)
        .cloned()
        .collect();
    assert_eq!(entries.len(), 22, "census: files on the HD disc");
    let mut compared = 0u64;
    let mut encrypted_files = 0;
    for e in &entries {
        // PS3UPDAT.PUP is 256 MiB of plain data; its first MiB stands for it.
        let len = if e.size > 64 << 20 && e.path.ends_with("PUP") {
            1 << 20
        } else {
            e.size
        };
        let a = enc.read_entry_range(e, 0, len).expect("enc read");
        let b = dec.read_entry_range(e, 0, len).expect("dec read");
        assert!(a == b, "{} differs", e.path);
        compared += len;
        if e.path.contains("USRDIR") {
            encrypted_files += 1;
        }
    }
    assert!(encrypted_files > 0);
    assert!(compared > 1 << 30, "compared {compared} bytes");
}

#[test]
#[ignore = "needs the encrypted and decrypted HD images and the key beside them"]
fn a_strided_sector_sample_across_every_region_matches() {
    let Some((mut enc, mut dec)) = pair() else {
        return;
    };
    let total = enc.sector_count();
    assert_eq!(total, dec.sector_count());
    // Not `read_sector`: the public surface is entries, so sample file starts
    // and a stride through each file; the whole-file test above covers bulk.
    let entries: Vec<_> = enc.entries().unwrap().to_vec();
    for e in entries
        .iter()
        .filter(|e| !e.is_directory && e.size > 8 * SECTOR_SIZE as u64)
    {
        let mut offset = 0u64;
        while offset < e.size {
            let a = enc.read_entry_range(e, offset, 4096).unwrap();
            let b = dec.read_entry_range(e, offset, 4096).unwrap();
            assert!(a == b, "{} at {offset}", e.path);
            offset += 9_999_361;
        }
    }
}

#[test]
#[ignore = "needs the encrypted HD image"]
fn without_any_key_the_image_reports_locked_and_a_wrong_key_is_rejected() {
    let Some(path) = oag_testdata::exact("hdfury-ps3-eu.iso") else {
        return;
    };
    let wrong = oag_disc::ps3_crypt::DiscKey::parse(b"00112233445566778899aabbccddeeff").unwrap();
    let disc = DiscImage::open_with_keys(&path, std::slice::from_ref(&wrong)).unwrap();
    assert_eq!(disc.ps3_state(), Ps3State::Locked);
    assert!(!oag_disc::ps3_crypt::key_opens(&path, &wrong).unwrap());
}
