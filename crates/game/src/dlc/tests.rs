use super::*;

/// A one-entry WAD holding `text`, hand-authored to the layout in
/// `docs/formats/wad.md`. No shipped bytes appear in this repository; see
/// [ADR-0006](../../../docs/architecture/adr/0006-no-copyrighted-content.md).
fn tiny_wad(text: &[u8]) -> Vec<u8> {
    let offset: u32 = 64;
    let len = u32::try_from(text.len()).unwrap();
    let mut out = Vec::new();
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&oag_formats::wad::hash_name("only").to_le_bytes());
    out.extend_from_slice(&offset.to_le_bytes());
    out.extend_from_slice(&len.to_le_bytes());
    out.extend_from_slice(&len.to_le_bytes());
    out.resize(offset as usize, 0);
    out.extend_from_slice(text);
    out
}

/// A zip shaped like a download: an archive and a `PARAM.pbp`, both under a
/// title-id folder, plus `members` spelled however the caller wants.
fn zip_of(members: &[(&str, Vec<u8>)]) -> Vec<u8> {
    use std::io::Write as _;
    let mut buffer = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut buffer);
        let options: zip::write::FileOptions<'_, ()> =
            zip::write::FileOptions::default().compression_method(zip::CompressionMethod::Deflated);
        for (name, bytes) in members {
            writer.start_file(*name, options).unwrap();
            writer.write_all(bytes).unwrap();
        }
        writer.finish().unwrap();
    }
    buffer.into_inner()
}

fn write_sample(dir: &std::path::Path, payload: &[u8]) -> std::path::PathBuf {
    let path = dir.join("Some Pack (Europe) (DLC).zip");
    let bytes = zip_of(&[
        ("UCES00465/PACK9.edat", tiny_wad(payload)),
        ("UCES00465/PARAM.pbp", b"\0PBP not a pack".to_vec()),
    ]);
    std::fs::write(&path, bytes).unwrap();
    path
}

/// A directory of this test's own, the same shape
/// [`crate::source`]'s tests use, so nothing here touches `data/cache`.
fn temp_dir(name: &str) -> std::path::PathBuf {
    let directory = std::env::temp_dir().join(format!("oag-dlc-cache-{name}"));
    let _ = std::fs::remove_dir_all(&directory);
    std::fs::create_dir_all(&directory).unwrap();
    directory
}

#[test]
fn the_title_id_folder_is_dropped_and_non_packs_are_skipped() {
    let dir = temp_dir("flatten");
    let zip = write_sample(&dir, &[1, 2, 3, 4]);
    let cache = dir.join("cache");

    let out = ensure_extracted(&zip, &cache, &[]).unwrap();

    assert_eq!(
        out,
        cache.join("Some Pack (Europe) (DLC)"),
        "the output directory is named for the zip, not for the title id inside it"
    );
    assert_eq!(
        std::fs::read(out.join("PACK9.edat")).unwrap(),
        tiny_wad(&[1, 2, 3, 4]),
        "the member arrives whole, header included"
    );
    assert!(
        !out.join("PARAM.pbp").exists(),
        "PARAM.pbp is PSN packaging, fails the header test, and nothing reads it"
    );
    assert!(
        !out.join("UCES00465").exists(),
        "the European title id must not survive into the cache: a pack is \
         region-independent here and a folder named after one region is \
         exactly the thing later code could come to depend on"
    );
}

/// The property that makes this a cache rather than an unpack step.
#[test]
fn a_second_extraction_rewrites_nothing() {
    let dir = temp_dir("idempotent");
    let zip = write_sample(&dir, &[7; 64]);
    let cache = dir.join("cache");

    let out = ensure_extracted(&zip, &cache, &[]).unwrap();
    let stamp = std::fs::metadata(out.join("PACK9.edat"))
        .unwrap()
        .modified()
        .unwrap();

    let again = ensure_extracted(&zip, &cache, &[]).unwrap();

    assert_eq!(out, again);
    assert_eq!(
        std::fs::metadata(again.join("PACK9.edat"))
            .unwrap()
            .modified()
            .unwrap(),
        stamp,
        "an already-extracted member must not be rewritten"
    );
}

/// A member left truncated by an interrupted run is the case the size
/// check exists for.
#[test]
fn a_truncated_member_is_written_again() {
    let dir = temp_dir("truncated");
    let zip = write_sample(&dir, &[9; 32]);
    let cache = dir.join("cache");

    let out = ensure_extracted(&zip, &cache, &[]).unwrap();
    std::fs::write(out.join("PACK9.edat"), [9; 8]).unwrap();

    ensure_extracted(&zip, &cache, &[]).unwrap();

    assert_eq!(
        std::fs::read(out.join("PACK9.edat")).unwrap(),
        tiny_wad(&[9; 32]),
        "a short file is a half-written one, and must be replaced"
    );
}

/// The promise `oag_assets::dlc` and ADR-0021 both make, kept on this side
/// of the boundary too: a repacked download whose members carry another
/// name, or no extension at all, still unpacks.
#[test]
fn a_member_is_extracted_on_its_header_rather_than_its_name() {
    let dir = temp_dir("renamed-member");
    let path = dir.join("Repacked.zip");
    std::fs::write(
        &path,
        zip_of(&[
            ("PACK9.wad", tiny_wad(b"renamed")),
            ("PACK9_UI1", tiny_wad(b"extensionless")),
            ("readme.txt", b"not an archive".to_vec()),
        ]),
    )
    .unwrap();

    let out = ensure_extracted(&path, &dir.join("cache"), &[]).unwrap();

    assert!(out.join("PACK9.wad").is_file());
    assert!(out.join("PACK9_UI1").is_file());
    assert!(
        !out.join("readme.txt").exists(),
        "the header is the test, and this has none"
    );
}

/// A zip with nothing to mount leaves no directory behind, which is how
/// [`super::packs`] tells "not a pack" from "a pack that failed".
#[test]
fn a_zip_holding_no_archive_creates_nothing() {
    let dir = temp_dir("no-archive");
    let path = dir.join("Holiday Photos.zip");
    std::fs::write(&path, zip_of(&[("beach.jpg", b"not an archive".to_vec())])).unwrap();

    let out = ensure_extracted(&path, &dir.join("cache"), &[]).unwrap();

    assert!(!out.exists(), "no members, so no directory");
}

/// A hand-encrypted fixture, never a shipped `pi.wad` - same rule as
/// [`tiny_wad`]. Built with the real
/// [`oag_formats::pure_dlc::crypt_with_key`], which is symmetric (see its
/// docs), so this is exactly the operation [`oag_formats::pure_dlc::decrypt_pack`]
/// will apply to undo it - this test exercises this module's wiring, not
/// a reimplementation of `pure_dlc`'s own algorithm.
fn encrypted_pack(plain: &[u8], key: [u32; 4]) -> (Vec<u8>, Vec<oag_formats::pure_dlc::DlcKey>) {
    let mut pack = oag_formats::pure_dlc::crypt_with_key(plain, key);
    pack.resize(pack.len() + oag_formats::pure_dlc::SIGNATURE_LEN, 0xaa);
    let keys = vec![oag_formats::pure_dlc::DlcKey {
        name: "TEST00000DTESTPACK".to_string(),
        key,
    }];
    (pack, keys)
}

/// The path Pure's packs need and Pulse's never do: a member that fails
/// the plain WAD sniff still gets picked up, because `keys` decrypts it
/// into one. See the module docs.
#[test]
fn a_member_that_only_decrypts_is_still_extracted() {
    let dir = temp_dir("pure-pack");
    let plain = tiny_wad(&[5; 40]);
    let key = [0x1122_3344, 0x5566_7788, 0x99aa_bbcc, 0xddee_ff00];
    let (ciphertext, keys) = encrypted_pack(&plain, key);

    let path = dir.join("A Pure Pack.zip");
    std::fs::write(&path, zip_of(&[("UCES00001DTESTPACK/pi.wad", ciphertext)])).unwrap();

    let out = ensure_extracted(&path, &dir.join("cache"), &keys).unwrap();

    assert_eq!(
        std::fs::read(out.join("pi.wad")).unwrap(),
        plain,
        "the cache holds the decrypted payload, trailer stripped, not the ciphertext"
    );
}

/// The same member, with no key that fits - today's behaviour, and what a
/// checkout with no `data/keys/pure-dlc-keys.txt` actually sees.
#[test]
fn a_member_that_decrypts_with_no_key_present_is_skipped() {
    let dir = temp_dir("pure-pack-no-keys");
    let plain = tiny_wad(&[5; 40]);
    let key = [0x1122_3344, 0x5566_7788, 0x99aa_bbcc, 0xddee_ff00];
    let (ciphertext, _keys) = encrypted_pack(&plain, key);

    let path = dir.join("A Pure Pack.zip");
    std::fs::write(&path, zip_of(&[("UCES00001DTESTPACK/pi.wad", ciphertext)])).unwrap();

    let out = ensure_extracted(&path, &dir.join("cache"), &[]).unwrap();

    assert!(!out.exists(), "no key fits, so this reads as no pack here");
}

#[test]
fn a_zip_is_recognised_by_its_magic_and_a_plain_file_is_not() {
    let dir = temp_dir("magic");
    let zip = write_sample(&dir, &[0; 4]);
    assert!(is_zip(&zip));

    let plain = dir.join("PACK9.edat");
    std::fs::write(&plain, [1u8, 0, 0, 0]).unwrap();
    assert!(!is_zip(&plain));

    assert!(
        !is_zip(&dir.join("absent.zip")),
        "a path that does not exist is not a zip, and asking must not fail"
    );
}
