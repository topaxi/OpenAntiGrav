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
//! ## Pure's packs need one more step
//!
//! Pure's `pi.wad` is genuinely encrypted (see
//! `docs/formats/dlc-pack.md#pures-packs-decrypt-with-an-external-key-table`),
//! so unlike a Pulse `.edat`, its bytes are not already a WAD once unzipped.
//! [`ensure_extracted`] tries [`oag_formats::pure_dlc::decrypt_pack`] on
//! anything that fails the plain WAD-header sniff, and writes the **decrypted**
//! bytes to the cache when a key fits - so from `oag_assets::dlc` downward,
//! everything still reads a plain file-backed WAD, decrypted or not.
//!
//! Decryption needs a key table this crate does not ship (see
//! `data/keys/README.md` for why), so `keys` is threaded in from the caller
//! rather than loaded here; an empty slice is exactly today's behaviour -
//! Pure's packs are found, fail the header sniff, and are reported the same
//! way a zip of holiday photos would be.
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

/// Every Pulse pack under `roots`, unpacking any zip it finds into
/// `cache_dir` first.
///
/// `keys` decrypts Pure's packs if it names any, purely so a Pure zip sitting
/// in the same folder gets extracted rather than left as a permanent
/// "problem" line - see [`extract_all`] for why this call still never
/// *returns* Pure's own packs. An empty slice is fine and is what a checkout
/// with no key table produces.
///
/// Returns what to mount, plus one line per zip that could not be unpacked.
/// Those are reported rather than propagated: a corrupt download in a folder of
/// four should cost the player that one pack, not the game. A player with no
/// DLC at all gets two empty vectors and never sees the difference.
#[must_use]
pub fn packs(
    roots: &[PathBuf],
    cache_dir: &Path,
    keys: &[oag_formats::pure_dlc::DlcKey],
) -> (Vec<oag_assets::dlc::Pack>, Vec<String>) {
    let (pulse_roots, _pure_roots, problems) = extract_all(roots, cache_dir, keys);
    (oag_assets::dlc::discover(&pulse_roots), problems)
}

/// [`packs`]'s counterpart for Wipeout Pure: every Pure pack under `roots`,
/// from the same extraction pass.
///
/// A caller wanting both titles' packs calls both functions; each re-walks
/// `roots` and re-extracts, but [`ensure_extracted`] is idempotent, so the
/// second call touches disk only to `stat` what the first one already wrote.
/// Two calls, not one returning both lists, because every existing call site
/// wants one title's packs for one title's
/// [`oag_assets::Archives::open_with_packs`] call - only
/// [`crate::title::open_source`] wants both, and it is the one caller free to
/// make two calls.
#[must_use]
pub fn pure_packs(
    roots: &[PathBuf],
    cache_dir: &Path,
    keys: &[oag_formats::pure_dlc::DlcKey],
) -> (Vec<oag_assets::dlc::Pack>, Vec<String>) {
    let (_pulse_roots, pure_roots, problems) = extract_all(roots, cache_dir, keys);
    (oag_assets::dlc::discover(&pure_roots), problems)
}

/// Unpacks every zip under `roots`, sorting each result into the roots a
/// Pulse-facing or a Pure-facing [`oag_assets::dlc::discover`] should walk.
///
/// The split matters because `roots` is one download folder shared by every
/// title in the lineage, and a Pulse `.edat` and a Pure `pi.wad` land in
/// different subtrees of `cache_dir` precisely so this function can tell them
/// apart without opening either archive again - see
/// [`ensure_extracted`]'s docs for where that split is made. Before it
/// existed, both kinds of pack sat in the one cache directory
/// [`oag_assets::dlc::discover`] walked, and once *any* run had a key table
/// present to decrypt Pure's packs, they stayed on disk as ordinary WADs and
/// got mounted behind Wipeout Pulse forever after, key table or not - the
/// disc a player never asked to add them to.
fn extract_all(
    roots: &[PathBuf],
    cache_dir: &Path,
    keys: &[oag_formats::pure_dlc::DlcKey],
) -> (Vec<PathBuf>, Vec<PathBuf>, Vec<String>) {
    let mut pulse_roots = roots.to_vec();
    let mut pure_roots = roots.to_vec();
    let pure_cache = cache_dir.join("pure");
    let mut problems = Vec::new();
    let mut empty = Vec::new();

    for zip in zips(roots) {
        match ensure_extracted(&zip, cache_dir, keys) {
            // A zip holding no archive is not a failure - a player's DLC folder
            // legitimately holds zips that are nothing to do with this game,
            // Wipeout Pure's own packs among them. Still worth saying, because
            // "my pack did not load" and "my pack is not a pack" look identical
            // from the menus otherwise. Gathered into one line rather than one
            // each: seven Pure packs would otherwise print seven lines on every
            // boot, forever, about a thing that is working correctly.
            Ok(unpacked) if !unpacked.is_dir() => empty.push(name_of(&zip)),
            Ok(unpacked) => {
                let destination = if unpacked.starts_with(&pure_cache) {
                    &mut pure_roots
                } else {
                    &mut pulse_roots
                };
                if !destination.contains(&unpacked) {
                    destination.push(unpacked);
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

    (pulse_roots, pure_roots, problems)
}

/// Where Pure's PSN DLC key table lives, if it does anywhere.
///
/// **Not a cache** - nothing here is derived, and there is no path back to a
/// player's own files that would regenerate it. It is user-supplied license
/// material, the same category as a disc image or a DLC pack, kept out of
/// this repository for the
/// [`data/keys/README.md`](../../../data/keys/README.md) reasons the Vita
/// zRIF table is - which is why this follows
/// [`crate::source::dlc_search_path`]'s three candidates exactly, under
/// `keys/` rather than `dlc/`: a player who deploys `data/dlc/` beside a
/// portable AppImage (`scripts/deploy-to-deck.sh`) needs the key table found
/// the same way, not left behind in a location only a checkout's build
/// resolves.
///
/// A missing file is not an error: it is the ordinary state of a checkout
/// that has not sourced Pure's keys, and [`packs`] treats it exactly like a
/// checkout with no DLC at all - the packs are found, fail to decrypt, and
/// are reported the same way an unrelated zip would be.
#[must_use]
pub fn default_pure_dlc_keys_path() -> PathBuf {
    for root in crate::source::dlc_search_path() {
        let candidate = root
            .parent()
            .unwrap_or(&root)
            .join("keys")
            .join("pure-dlc-keys.txt");
        if candidate.is_file() {
            return candidate;
        }
    }
    Path::new("data/keys/pure-dlc-keys.txt").to_path_buf()
}

/// Reads and parses [`default_pure_dlc_keys_path`], or an empty table if the
/// file is absent - see that function's docs for why absence is not an error.
#[must_use]
pub fn load_pure_dlc_keys() -> Vec<oag_formats::pure_dlc::DlcKey> {
    let text = std::fs::read_to_string(default_pure_dlc_keys_path()).unwrap_or_default();
    oag_formats::pure_dlc::parse_keys(&text)
}

/// Both titles' packs, loading Pure's key table from
/// [`default_pure_dlc_keys_path`] rather than taking one, from a single
/// extraction pass rather than [`packs`] and [`pure_packs`] called
/// separately.
///
/// The shape every real caller wants: [`crate::title::open_source`] needs
/// both lists before it knows which title `source` turned out to be, and
/// `problems` is shared rather than split in two, because it is not a
/// per-title fact - a corrupt zip or one holding no archive at all is
/// exactly as much a problem for a Pure source as for a Pulse one. [`packs`]
/// and [`pure_packs`] stay explicit for anything that needs its own key
/// table or only one title's list, such as a test.
#[must_use]
pub fn packs_from_defaults(
    roots: &[PathBuf],
    cache_dir: &Path,
) -> (
    Vec<oag_assets::dlc::Pack>,
    Vec<oag_assets::dlc::Pack>,
    Vec<String>,
) {
    let (pulse_roots, pure_roots, problems) = extract_all(roots, cache_dir, &load_pure_dlc_keys());
    (
        oag_assets::dlc::discover(&pulse_roots),
        oag_assets::dlc::discover(&pure_roots),
        problems,
    )
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
/// test [`oag_assets::dlc`] applies on disk, **or** if `keys` decrypts it into
/// one - see the module docs for Pure's packs. Not the `.edat` extension: this
/// project's whole DLC path promises that a renamed or repacked download still
/// works, and gating here on a spelling would quietly break that promise one
/// step before the code that keeps it. `PARAM.pbp` and anything else a repacker
/// added fail both tests and are skipped rather than refused - a pack with an
/// unexpected extra file is still a usable pack.
///
/// Idempotent: a member already present at its recorded size is left alone, so
/// the second call does no work and no writes. Size rather than a hash because
/// the failure this actually guards against is a run interrupted partway
/// through a write, which truncates; a member that is the right length is one
/// the previous run finished. A decrypted member is shorter than its source by
/// exactly [`oag_formats::pure_dlc::SIGNATURE_LEN`] (the trailer this project
/// never writes back), so that is the length checked for it instead - the
/// decryption itself still runs every call, since knowing *which* length to
/// expect would mean already having decrypted it.
///
/// Every member's top-level folder is dropped - see the module docs -
/// **unless the zip holds more than one distinct one**, which only Pure's
/// Omega pack does: it bundles two content ids (`UCES00001DOMEGAPAK` and
/// `UCES00001DOMEGAPAKS`) as two folders in one zip, each with its own
/// `pi.wad`. Flattening both would let the second overwrite the first on
/// disk. Keeping the folder in that case still does not leak a *region*:
/// Pure's per-pack folder is a content id, not a territory, and a single
/// folder (the common case, including every Pulse zip) still flattens.
pub fn ensure_extracted(
    zip: &Path,
    cache_dir: &Path,
    keys: &[oag_formats::pure_dlc::DlcKey],
) -> Result<PathBuf> {
    let stem = zip
        .file_stem()
        .context("a zip path with no file name")?
        .to_string_lossy()
        .into_owned();
    let out = cache_dir.join(&stem);
    // A decrypted member's plaintext is written under its own subtree, never
    // under `out` - see the module docs. Without this split, a Pure pack
    // decrypted once (by a run that had the key table) and a Pulse pack that
    // never needed one would sit side by side as indistinguishable plain
    // WADs on a later `oag_assets::dlc::discover` walk, and a title would
    // mount packs that were never its own. `out`'s own idempotency guarantee
    // is unaffected: a plain member still never touches this path.
    let pure_out = cache_dir.join("pure").join(&stem);

    let file = std::fs::File::open(zip).with_context(|| format!("opening {}", zip.display()))?;
    let mut archive = zip::ZipArchive::new(std::io::BufReader::new(file))
        .with_context(|| format!("reading {} as a zip", zip.display()))?;

    // Whether flattening to a bare basename is safe. It is for every Pulse
    // zip, which carries exactly one top-level folder (a region's title id)
    // holding every member of one pack - dropping it is the point, see the
    // module docs. It is also safe for a zip with no folder at all, which a
    // repacked or renamed download can be. It is **not** safe for a zip like
    // Pure's Omega pack, which bundles two distinct content ids as two
    // top-level folders, each with its own `pi.wad` - flattening both to
    // `pi.wad` would let the second silently overwrite the first. Two or
    // more distinct top-level folders means keep them, so two same-named
    // members never collide.
    //
    // Only a member with a real subfolder counts: a root-level member's own
    // filename is its first path component too, so counting every member
    // rather than only folder-nested ones would see three "folders" in a
    // three-file zip with no folder structure at all, and wrongly keep it.
    let mut top_level_folders = std::collections::BTreeSet::new();
    for index in 0..archive.len() {
        let Ok(entry) = archive.by_index(index) else {
            continue;
        };
        if let Some(name) = entry.enclosed_name()
            && name.components().count() > 1
            && let Some(first) = name.components().next()
        {
            top_level_folders.insert(first.as_os_str().to_os_string());
        }
    }
    let keep_folder = top_level_folders.len() > 1;
    let target_in = |root: &Path, name: &Path, base: &std::ffi::OsStr| -> PathBuf {
        if keep_folder {
            match name.components().next() {
                Some(first) => root.join(first.as_os_str()).join(base),
                None => root.join(base),
            }
        } else {
            root.join(base)
        }
    };
    // Which of `out`/`pure_out` actually gained a member - see the module
    // docs. `out` if the zip turns out to hold nothing, matching this
    // function's existing "no members, no directory" contract for a zip with
    // no archive in it at all.
    let mut used_out: Option<PathBuf> = None;

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

        // Checked against both roots before anything is read: a plain
        // member never lands under `pure_out` and a decrypted one never
        // lands under `out`, so exactly one of these can ever match, and
        // matching it skips the decrypt attempt entirely - the idempotency
        // this function promises.
        let target_plain = target_in(&out, &name, &base);
        let target_pure = target_in(&pure_out, &name, &base);
        let decrypted_len = member
            .size()
            .checked_sub(oag_formats::pure_dlc::SIGNATURE_LEN as u64);
        if std::fs::metadata(&target_plain).is_ok_and(|m| m.len() == member.size()) {
            used_out = Some(out.clone());
            continue;
        }
        if std::fs::metadata(&target_pure).is_ok_and(|m| Some(m.len()) == decrypted_len) {
            used_out = Some(pure_out.clone());
            continue;
        }

        // The header decides, not the name. Read before creating anything, so
        // a zip of holiday photos leaves no empty directory behind.
        let mut header = [0u8; oag_formats::wad::HEADER_LEN];
        use std::io::Read as _;
        if member.read_exact(&mut header).is_err() {
            continue;
        }
        let is_wad = oag_formats::wad::Directory::peek_entry_count(&header).is_ok();

        // Not a plain WAD - see if it is a Pure pack `keys` can decrypt.
        // `raw` reconstructs the whole member: `header`'s bytes were already
        // consumed off the zip reader above.
        let decrypted = if is_wad || keys.is_empty() {
            None
        } else {
            let mut raw = header.to_vec();
            if member.read_to_end(&mut raw).is_err() {
                continue;
            }
            oag_formats::pure_dlc::decrypt_pack(&raw, keys).map(|(payload, _key)| payload)
        };

        if !is_wad && decrypted.is_none() {
            continue;
        }

        let (target, root) = if decrypted.is_some() {
            (&target_pure, &pure_out)
        } else {
            (&target_plain, &out)
        };
        used_out = Some(root.clone());

        // `target`'s parent is `root` itself when `keep_folder` is false, and
        // `root/<top-level folder>` when it is true - either way, creating it
        // creates `root`, which is all the "no members, no directory" contract
        // above actually needs. `create_dir_all` is a cheap no-op once the
        // directory exists, so there is nothing to gate this on.
        let parent = target.parent().unwrap_or(root);
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;

        let mut sink = std::fs::File::create(target)
            .with_context(|| format!("creating {}", target.display()))?;
        let mut write = || -> std::io::Result<()> {
            use std::io::Write as _;
            if let Some(payload) = &decrypted {
                return sink.write_all(payload);
            }
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

    Ok(used_out.unwrap_or(out))
}

#[cfg(test)]
mod tests;
