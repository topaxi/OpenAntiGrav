//! Turning what a player downloaded into mountable archives.
//!
//! The composition root's half of the DLC path: unpack a downloaded zip into
//! the cache, then hand the result to [`oag_assets::dlc`], which does the
//! finding and reading. The split is deliberate - a decompressor is a
//! dependency, and keeping it here means no gameplay crate acquires one.
//!
//! The Pulse packs are distributed as `.zip`, each holding a `PACKn.edat` and
//! three `PACKn_UI*.edat` under a title-id folder. Those `.edat` files are
//! [ordinary WAD archives](../../../docs/formats/dlc-pack.md) despite the
//! extension, so once they are on disk `oag_assets::dlc` can open them exactly
//! as it opens a disc's own `Data.wad` - lazily, a range at a time.
//!
//! Getting them onto disk is this module's whole job, and it is deliberately
//! *extraction*, not a seek-inside-the-zip source. A pack is around 10 MiB and
//! a full set is four of them; decompressing that into memory on every boot
//! would be paid every run, for a file that never changes. Writing it once into
//! [`crate::boot::default_dlc_cache_dir`] keeps the read path identical to the
//! disc's and the resident cost at zero.
//!
//! ## The title-id folder is dropped on purpose
//!
//! A member arrives as `UCES00465/PACK3.edat` and lands as `PACK3.edat`. That
//! folder is the *European* title id, and the whole point of this path is that
//! a pack works against whichever Pulse image the player owns. Flattening it
//! means nothing downstream can accidentally come to depend on it. It also
//! makes traversal impossible by construction: only a basename is ever joined
//! to the output directory.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
// How deep to look for a zip below a root. Shared with the walker that finds
// the unpacked result rather than spelled again here: a zip found deeper than
// that walker reaches would be extracted and then never discovered, and the
// player would see their pack quietly not load.
use oag_assets::dlc::MAX_DEPTH;

/// Every pack under `roots`, unpacking any zip it finds into `cache_dir` first.
///
/// Returns what to mount, plus one line per zip that could not be unpacked.
/// Those are reported rather than propagated: a corrupt download in a folder of
/// four should cost the player that one pack, not the game. A player with no
/// DLC at all gets two empty vectors and never sees the difference.
#[must_use]
pub fn packs(roots: &[PathBuf], cache_dir: &Path) -> (Vec<oag_assets::dlc::Pack>, Vec<String>) {
    let mut roots = roots.to_vec();
    let mut problems = Vec::new();
    let mut empty = Vec::new();

    for zip in zips(&roots) {
        match ensure_extracted(&zip, cache_dir) {
            // A zip holding no archive is not a failure - a player's DLC folder
            // legitimately holds zips that are nothing to do with this game,
            // Wipeout Pure's own packs among them. Still worth saying, because
            // "my pack did not load" and "my pack is not a pack" look identical
            // from the menus otherwise. Gathered into one line rather than one
            // each: seven Pure packs would otherwise print seven lines on every
            // boot, forever, about a thing that is working correctly.
            Ok(unpacked) if !unpacked.is_dir() => empty.push(name_of(&zip)),
            Ok(unpacked) => {
                if !roots.contains(&unpacked) {
                    roots.push(unpacked);
                }
            }
            Err(e) => problems.push(format!("{}: {e:#}", zip.display())),
        }
    }

    if !empty.is_empty() {
        problems.push(format!(
            "{} zip(s) hold no WAD archive and were skipped: {}",
            empty.len(),
            empty.join(", ")
        ));
    }

    (oag_assets::dlc::discover(&roots), problems)
}

fn name_of(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

/// Every zip under `roots`, sorted, so a run does not depend on `read_dir`
/// order.
fn zips(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for root in roots {
        collect_zips(root, 0, &mut out);
    }
    out.sort();
    out.dedup();
    out
}

fn collect_zips(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut children = Vec::new();
    for path in entries.flatten().map(|e| e.path()) {
        if path.is_dir() {
            children.push(path);
        } else if is_zip(&path) {
            out.push(path);
        }
    }
    if depth >= MAX_DEPTH {
        return;
    }
    children.sort();
    for child in children {
        collect_zips(&child, depth + 1, out);
    }
}

/// The first four bytes of any zip, central-directory or not.
const ZIP_MAGIC: [u8; 4] = [b'P', b'K', 0x03, 0x04];

/// Whether this path is a zip we should try to unpack.
///
/// By content, not by extension, for the same reason
/// [`oag_disc::DiscImage::open`] sniffs a container rather than trusting `.chd`:
/// a renamed file is a real thing that happens, and the cost of looking is four
/// bytes.
#[must_use]
pub fn is_zip(path: &Path) -> bool {
    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    let mut magic = [0u8; 4];
    use std::io::Read as _;
    file.read_exact(&mut magic).is_ok() && magic == ZIP_MAGIC
}

/// Extract every archive member of `zip` into `<cache_dir>/<zip stem>/`,
/// returning that directory - which does **not** exist if the zip held none.
///
/// A member is an archive if its first bytes read as a WAD header, exactly the
/// test [`oag_assets::dlc`] applies on disk. Not the `.edat` extension: this
/// project's whole DLC path promises that a renamed or repacked download still
/// works, and gating here on a spelling would quietly break that promise one
/// step before the code that keeps it. `PARAM.pbp` and anything else a repacker
/// added fail the header test and are skipped rather than refused - a pack with
/// an unexpected extra file is still a usable pack.
///
/// Idempotent: a member already present at its recorded size is left alone, so
/// the second call does no work and no writes. Size rather than a hash because
/// the failure this actually guards against is a run interrupted partway
/// through a write, which truncates; a member that is the right length is one
/// the previous run finished.
pub fn ensure_extracted(zip: &Path, cache_dir: &Path) -> Result<PathBuf> {
    let out = cache_dir.join(
        zip.file_stem()
            .context("a zip path with no file name")?
            .to_string_lossy()
            .as_ref(),
    );

    let file = std::fs::File::open(zip).with_context(|| format!("opening {}", zip.display()))?;
    let mut archive = zip::ZipArchive::new(std::io::BufReader::new(file))
        .with_context(|| format!("reading {} as a zip", zip.display()))?;

    let mut written = false;
    for index in 0..archive.len() {
        let mut member = archive
            .by_index(index)
            .with_context(|| format!("reading member {index} of {}", zip.display()))?;

        // `enclosed_name` is `None` for a member whose path escapes the
        // archive; taking only the file name would already make that harmless,
        // but skipping it means such an archive is reported by what it is
        // rather than silently half-extracted.
        let Some(name) = member.enclosed_name() else {
            continue;
        };
        let Some(base) = name.file_name().map(std::ffi::OsString::from) else {
            continue;
        };

        let target = out.join(&base);
        if std::fs::metadata(&target).is_ok_and(|m| m.len() == member.size()) {
            written = true;
            continue;
        }

        // The header decides, not the name. Read before creating anything, so
        // a zip of holiday photos leaves no empty directory behind.
        let mut header = [0u8; oag_formats::wad::HEADER_LEN];
        use std::io::Read as _;
        if member.read_exact(&mut header).is_err()
            || oag_formats::wad::Directory::peek_entry_count(&header).is_err()
        {
            continue;
        }

        if !written {
            std::fs::create_dir_all(&out).with_context(|| format!("creating {}", out.display()))?;
            written = true;
        }

        let mut sink = std::fs::File::create(&target)
            .with_context(|| format!("creating {}", target.display()))?;
        let mut write = || -> std::io::Result<()> {
            use std::io::Write as _;
            // The header was consumed by the sniff above and has to go back.
            sink.write_all(&header)?;
            std::io::copy(&mut member, &mut sink)?;
            Ok(())
        };
        write().with_context(|| {
            format!(
                "extracting {} from {}",
                base.to_string_lossy(),
                zip.display()
            )
        })?;
    }

    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::{ensure_extracted, is_zip};

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
            let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
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

        let out = ensure_extracted(&zip, &cache).unwrap();

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

        let out = ensure_extracted(&zip, &cache).unwrap();
        let stamp = std::fs::metadata(out.join("PACK9.edat"))
            .unwrap()
            .modified()
            .unwrap();

        let again = ensure_extracted(&zip, &cache).unwrap();

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

        let out = ensure_extracted(&zip, &cache).unwrap();
        std::fs::write(out.join("PACK9.edat"), [9; 8]).unwrap();

        ensure_extracted(&zip, &cache).unwrap();

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

        let out = ensure_extracted(&path, &dir.join("cache")).unwrap();

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

        let out = ensure_extracted(&path, &dir.join("cache")).unwrap();

        assert!(!out.exists(), "no members, so no directory");
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
}
