//! Finding downloadable content packs and reading what each one declares.
//!
//! A Wipeout Pulse DLC pack is a `PACKn.edat` plus three `PACKn_UI*.edat`, and
//! every one of those is an ordinary [WAD](../../../docs/formats/wad.md) despite
//! the extension - see
//! [`docs/formats/dlc-pack.md`](../../../docs/formats/dlc-pack.md). So there is
//! no new container here and no decryption step: a pack is a directory holding
//! some number of archives, and mounting it is [`crate::Archives`]'
//! problem rather than this module's.
//!
//! ## Region independence is the point, so nothing here reads a region
//!
//! In the original, a pack only worked against the disc from its own territory.
//! This project deliberately does not reproduce that, so every rule below is
//! chosen to give the *region* nothing to attach to:
//!
//! - A candidate is anything that **parses as a WAD**. The `PACK*.edat` naming
//!   is never required, so a repacked or renamed set still works.
//! - The title-id folder a download ships inside (`UCES00465/` for the European
//!   packs) is not consulted, and [`crate::pulse::Layout`]'s serial check is not
//!   applied - that check exists to stop a *different game's* disc being opened
//!   as Pulse, and a pack is not a disc.
//! - Nothing here compares a pack against the image it will be mounted on. The
//!   two are matched by name hash at read time, like everything else, and a name
//!   hash carries no territory.
//!
//! ## What a pack declares
//!
//! Entry 0 of each archive is a manifest: a fragment shaped exactly like the
//! disc's own `Data\Plugins\PI001\Definition.xml`, holding the `PI_Team` and
//! `PI_Track` nodes this pack adds. The main archive's is populated and the UI
//! archives ship an empty `<Screen name="Top">` stub, so the manifests are
//! collected without judging them and the caller - `oag_game::catalogue` - finds
//! nothing in the stubs. Keeping that judgement out of here means the schema is
//! understood in one place rather than two.

use std::path::{Path, PathBuf};

use oag_tables::fexml;

use crate::Archive;

/// How deep [`discover`] looks below a root.
///
/// Two, so that a directory holding a download's own `UCES00465/PACK3.edat`
/// works without the player flattening it first, while a root pointed at a home
/// directory by accident does not turn into a filesystem walk.
///
/// **Public because it has to agree with whoever unpacks the zips.** A zip
/// found deeper than this walker reaches would be extracted and then never
/// discovered, and the player would see their pack quietly not load; sharing
/// the constant is what stops the two halves drifting. `oag_game::dlc` uses it.
pub const MAX_DEPTH: usize = 2;

/// One directory's worth of downloadable content.
#[derive(Debug)]
pub struct Pack {
    /// The directory this came from, for error messages and logs.
    pub label: String,
    /// Every manifest found, in archive order. Usually one populated one and
    /// some empty stubs; see the module docs.
    pub manifests: Vec<String>,
    /// The archives to mount, sorted by path so the order does not depend on
    /// what `read_dir` happened to return.
    pub archives: Vec<Archive>,
}

/// Every pack found under `roots`, in the order the roots were given.
///
/// Never fails. A root that does not exist, a file that is not an archive and a
/// directory holding nothing usable are all simply "no pack here" - a player
/// with no DLC is the common case, not an error, and a `PARAM.pbp` sitting
/// beside the packs is not a problem to report.
///
/// A directory reached twice - a root that is also inside another root, or a
/// hand-unpacked copy of a zip that is also in the cache - yields one pack, not
/// two. Mounting the same archives twice would double the boot report's count,
/// hold two copies of every directory for the session, and make every lookup
/// miss scan the duplicates.
#[must_use]
pub fn discover(roots: &[PathBuf]) -> Vec<Pack> {
    let mut out = Vec::new();
    let mut seen = Vec::new();
    for root in roots {
        walk(root, 0, &mut out, &mut seen);
    }
    out
}

fn walk(dir: &Path, depth: usize, out: &mut Vec<Pack>, seen: &mut Vec<PathBuf>) {
    // Canonical, so `data/dlc` and `./data/dlc/../dlc` are one directory, and
    // so a symlinked pack folder is not mounted beside its target.
    let identity = std::fs::canonicalize(dir).unwrap_or_else(|_| dir.to_path_buf());
    if seen.contains(&identity) {
        return;
    }
    seen.push(identity);

    if let Some(pack) = open_dir(dir) {
        out.push(pack);
    }
    if depth >= MAX_DEPTH {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut children: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    children.sort();
    for child in children {
        walk(&child, depth + 1, out, seen);
    }
}

/// Opens every archive directly inside `dir`, or `None` if there are none.
///
/// **Infallible, and that is the design rather than a shortcut.** Most
/// directories are not packs: the caller is walking a tree, "nothing here" is
/// the ordinary answer, and a directory that cannot be read or holds files that
/// will not parse is the same answer arrived at differently. Returning a
/// `Result` would make every caller handle an error that only ever means "no".
#[must_use]
pub fn open_dir(dir: &Path) -> Option<Pack> {
    let entries = std::fs::read_dir(dir).ok()?;
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file())
        .collect();
    files.sort();

    let mut archives = Vec::new();
    let mut manifests = Vec::new();
    for file in files {
        // `open_file` rather than `Archive::open`: a spec string would read a
        // colon in the directory name as a disc image and refuse an
        // extensionless file unopened, and neither has anything to do with
        // whether this is an archive. The header is the only test.
        //
        // Anything that is not a WAD is not a pack archive - a rejected
        // `PARAM.pbp` costs one failed header read.
        let Ok(mut archive) = Archive::open_file(&file) else {
            continue;
        };
        if let Some(manifest) = read_manifest(&mut archive) {
            manifests.push(manifest);
        }
        archives.push(archive);
    }

    if archives.is_empty() {
        return None;
    }
    Some(Pack {
        label: dir.display().to_string(),
        manifests,
        archives,
    })
}

/// Entry 0, as text, expanded if it is [shortened
/// XML](../../../docs/formats/fexml.md).
///
/// `None` for an archive whose first entry is not text at all, which is not a
/// thing any shipped pack does but is cheaper to tolerate than to prove
/// impossible.
fn read_manifest(archive: &mut Archive) -> Option<String> {
    let blob = archive.read(0).ok()?;
    if fexml::is_fexml(&blob) {
        fexml::expand(&blob).ok()
    } else {
        String::from_utf8(blob).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::{discover, open_dir};
    use crate::testing;

    /// A one-entry WAD holding `text`. Hand-authored through
    /// [`crate::testing::wad`], per
    /// [ADR-0006](../../../docs/architecture/adr/0006-no-copyrighted-content.md)
    /// - no shipped bytes appear in this repository.
    fn one_entry_wad(name: &str, text: &str) -> Vec<u8> {
        testing::wad(&[(name, text.as_bytes())])
    }

    fn temp_dir(name: &str) -> std::path::PathBuf {
        testing::temp_dir(&format!("dlc-{name}"))
    }

    const MANIFEST: &str = r#"<Screen name="Top"><PI_Team name="Ersatz"/></Screen>"#;

    #[test]
    fn a_directory_of_archives_becomes_one_pack_with_its_manifest() {
        let dir = temp_dir("one-pack");
        std::fs::write(dir.join("PACK9.edat"), one_entry_wad("manifest", MANIFEST)).unwrap();
        std::fs::write(dir.join("PARAM.pbp"), b"not an archive at all").unwrap();

        let pack = open_dir(&dir).expect("a pack");

        assert_eq!(pack.archives.len(), 1, "the .pbp is not an archive");
        assert_eq!(pack.manifests, vec![MANIFEST.to_string()]);
    }

    /// The rule that makes a renamed or repacked download work: nothing keys
    /// off the `PACK*.edat` spelling.
    ///
    /// **Including the two spellings a spec string cannot express.** A file
    /// with no extension at all reads as a mistyped disc spec, and a directory
    /// holding a colon reads as an image name - both are ordinary things to
    /// find on a player's disk, and neither says anything about whether the
    /// file is an archive. See [`Archive::open_file`].
    #[test]
    fn an_archive_is_recognised_whatever_it_is_called() {
        let dir = temp_dir("renamed");
        std::fs::write(
            dir.join("some other name.bin"),
            one_entry_wad("manifest", MANIFEST),
        )
        .unwrap();
        std::fs::write(dir.join("PACK9"), one_entry_wad("manifest", MANIFEST)).unwrap();

        let pack = open_dir(&dir).expect("a pack");
        assert_eq!(
            pack.archives.len(),
            2,
            "an extensionless archive counts too"
        );
    }

    #[test]
    fn a_directory_whose_name_holds_a_colon_is_not_read_as_a_disc_image() {
        let root = temp_dir("colon");
        let dir = root.join("Pulse: Mirage Pack");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("PACK9.edat"), one_entry_wad("manifest", MANIFEST)).unwrap();

        let packs = discover(&[root]);
        assert_eq!(packs.len(), 1, "{packs:?}");
        assert_eq!(packs[0].manifests, vec![MANIFEST.to_string()]);
    }

    /// One directory, one pack, however many ways the walk reaches it.
    /// Mounting the same archives twice costs memory for the session and makes
    /// every lookup miss scan the duplicates.
    #[test]
    fn a_directory_reachable_from_two_roots_is_mounted_once() {
        let root = temp_dir("duplicate");
        let inner = root.join("Harimau");
        std::fs::create_dir_all(&inner).unwrap();
        std::fs::write(
            inner.join("PACK9.edat"),
            one_entry_wad("manifest", MANIFEST),
        )
        .unwrap();

        let packs = discover(&[root, inner]);

        assert_eq!(packs.len(), 1, "{packs:?}");
    }

    /// The European downloads unpack into a title-id folder. Finding them there
    /// is what lets a player point `--dlc` at the folder they downloaded into,
    /// and the title id itself is never read.
    #[test]
    fn a_pack_nested_in_a_title_id_folder_is_found() {
        let root = temp_dir("nested");
        let inner = root.join("UCES00465");
        std::fs::create_dir_all(&inner).unwrap();
        std::fs::write(
            inner.join("PACK9.edat"),
            one_entry_wad("manifest", MANIFEST),
        )
        .unwrap();

        let packs = discover(&[root]);

        assert_eq!(packs.len(), 1, "{packs:?}");
        assert_eq!(packs[0].manifests, vec![MANIFEST.to_string()]);
    }

    #[test]
    fn a_directory_with_no_archives_is_not_a_pack() {
        let dir = temp_dir("empty");
        std::fs::write(dir.join("readme.txt"), b"nothing to mount").unwrap();

        assert!(open_dir(&dir).is_none());
    }

    #[test]
    fn a_root_that_does_not_exist_yields_nothing_rather_than_failing() {
        assert!(discover(&[std::path::PathBuf::from("/nonexistent/oag/dlc")]).is_empty());
    }
}
