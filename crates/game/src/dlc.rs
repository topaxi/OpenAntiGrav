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

/// The extension the packs use. Not a WAD, not a zip - see the module docs.
const PACK_EXTENSION: &str = "edat";

/// How deep [`packs`] looks for a zip below a root, matching
/// [`oag_assets::dlc`]'s own depth so a zip and an unpacked folder are found in
/// the same places.
const MAX_DEPTH: usize = 2;

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

    for zip in zips(&roots) {
        match ensure_extracted(&zip, cache_dir) {
            Ok(unpacked) => {
                if !roots.contains(&unpacked) {
                    roots.push(unpacked);
                }
            }
            Err(e) => problems.push(format!("{}: {e:#}", zip.display())),
        }
    }

    (oag_assets::dlc::discover(&roots), problems)
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

/// Extract every `.edat` member of `zip` into `<cache_dir>/<zip stem>/`,
/// returning that directory.
///
/// Idempotent: a member already present at its recorded size is left alone, so
/// the second call does no work and no writes. Size rather than a hash because
/// the failure this actually guards against is a run interrupted partway
/// through a write, which truncates; a member that is the right length is one
/// the previous run finished.
///
/// Members that are not `.edat` - `PARAM.pbp`, and anything a repacker added -
/// are skipped rather than refused. A pack with an unexpected extra file is
/// still a usable pack.
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
        if Path::new(&base)
            .extension()
            .is_none_or(|e| !e.eq_ignore_ascii_case(PACK_EXTENSION))
        {
            continue;
        }

        let target = out.join(&base);
        if std::fs::metadata(&target).is_ok_and(|m| m.len() == member.size()) {
            continue;
        }

        if !written {
            std::fs::create_dir_all(&out).with_context(|| format!("creating {}", out.display()))?;
            written = true;
        }

        let mut sink = std::fs::File::create(&target)
            .with_context(|| format!("creating {}", target.display()))?;
        std::io::copy(&mut member, &mut sink).with_context(|| {
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

    /// A zip holding one `.edat`, one `.pbp` and one member nested in a
    /// title-id folder, built in memory. Nothing here is game content: the
    /// payloads are counted bytes.
    fn sample_zip(edat: &[u8]) -> Vec<u8> {
        use std::io::Write as _;
        let mut buffer = std::io::Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut buffer);
            let options: zip::write::FileOptions<'_, ()> = zip::write::FileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);
            writer.start_file("UCES00465/PACK9.edat", options).unwrap();
            writer.write_all(edat).unwrap();
            writer.start_file("UCES00465/PARAM.pbp", options).unwrap();
            writer.write_all(b"not a pack").unwrap();
            writer.finish().unwrap();
        }
        buffer.into_inner()
    }

    fn write_sample(dir: &std::path::Path, edat: &[u8]) -> std::path::PathBuf {
        let path = dir.join("Some Pack (Europe) (DLC).zip");
        std::fs::write(&path, sample_zip(edat)).unwrap();
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
        assert_eq!(std::fs::read(out.join("PACK9.edat")).unwrap(), [1, 2, 3, 4]);
        assert!(
            !out.join("PARAM.pbp").exists(),
            "PARAM.pbp is PSN packaging and nothing reads it"
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
            std::fs::read(out.join("PACK9.edat")).unwrap().len(),
            32,
            "a short file is a half-written one, and must be replaced"
        );
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
