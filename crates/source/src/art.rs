//! A source's own system art: the picture the console shows for the game.
//!
//! Read at run time from the player's own files and never copied anywhere
//! (`docs/ui/launcher.md`, "Where the art comes from", lists each location and
//! the page that documents it). What is read depends on what the source *is*,
//! not on which title it turns out to be:
//!
//! | source | picture | logo drawn over it |
//! | --- | --- | --- |
//! | PSP disc | `PSP_GAME/PIC1.PNG` | none |
//! | PS3 disc | `PS3_GAME/PIC1.PNG` | `PS3_GAME/ICON0.PNG` |
//! | installed PSN folder | `PIC1.PNG` | `ICON0.PNG` |
//! | Vita folder or `.vpk` | `sce_sys/pic0.png` | none |
//! | PS4 `.pkg` | header entries `pic0.png`, `pic1.png` | none |
//! | unpacked PS4 folder | the same, from the title's `.pkg` if one is still there | none |
//!
//! A PS2 disc has none of these, so its picture is put together from its title
//! screen's own parts ([`title_screen`]). A file that will not decode counts as
//! absent.

use std::path::{Path, PathBuf};

use oag_disc::DiscImage;
use oag_disc::ps4_pkg::{self, header};

mod title_screen;

/// A decoded picture: RGBA8888, row-major from the top left.
#[derive(Clone)]
pub struct Picture {
    /// Pixels across.
    pub width: u32,
    /// Pixels down.
    pub height: u32,
    /// `width * height * 4` bytes.
    pub rgba: Vec<u8>,
}

impl std::fmt::Debug for Picture {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Picture({}x{})", self.width, self.height)
    }
}

/// The picture a source offers for its title.
#[derive(Debug, Clone)]
pub struct Art {
    /// `pic0` or `pic1`, the file's own name, or `title` for a composed one:
    /// what a remembered choice is keyed by.
    pub name: &'static str,
    /// The picture.
    pub picture: Picture,
    /// A logo the console draws over the picture, where the picture has no
    /// wordmark of its own.
    pub logo: Option<Picture>,
}

/// The picture `source` carries, `None` when it carries none.
///
/// Reads files, and so belongs on a worker thread.
#[must_use]
pub fn read(source: &str) -> Option<Art> {
    read_with(source, crate::source::ps4_base_packages)
}

/// [`read`], with where an unpacked PS4 folder's package is looked for handed
/// in: the search path is relative to the run's directory.
#[must_use]
pub fn read_with(source: &str, ps4_packages: impl FnOnce() -> Vec<PathBuf>) -> Option<Art> {
    let path = Path::new(source);
    let found = if path.is_dir() {
        directory(path, ps4_packages)
    } else if is_ps4_package(path) {
        ps4_package(path)
    } else {
        disc(path)
    };
    log::debug!(
        "{source}: system picture {}",
        found.as_ref().map_or("none", |art| art.name)
    );
    found
}

fn is_ps4_package(path: &Path) -> bool {
    path.extension()
        .is_some_and(|extension| extension.eq_ignore_ascii_case("pkg"))
        && ps4_pkg::title_id(path).is_some()
}

/// An unpacked folder, told apart by the same files `crate::source` lists it by.
fn directory(root: &Path, ps4_packages: impl FnOnce() -> Vec<PathBuf>) -> Option<Art> {
    if root.join("base/sce_sys").is_dir() {
        let pic0 = file(&root.join("base/sce_sys/pic0.png"));
        return one("pic0", pic0, None);
    }
    if root.join("USRDIR").is_dir() {
        let pic1 = file(&root.join("PIC1.PNG"));
        let logo = file(&root.join("ICON0.PNG"));
        return one("pic1", pic1, logo);
    }
    if let Some(mark) = crate::title::ps4_folder_content_id(root) {
        return ps4_packages()
            .iter()
            .filter(|pkg| ps4_pkg::header::content_id(pkg).is_some_and(|id| id.contains(mark)))
            .find_map(|pkg| ps4_package(pkg));
    }
    None
}

/// A PS4 package's picture, `pic0` before `pic1`, from whichever package of
/// the title keeps it (a patch keeps neither).
fn ps4_package(path: &Path) -> Option<Art> {
    let packages: Vec<PathBuf> = ps4_pkg::set::siblings(path);
    [("pic0", header::PIC0_PNG), ("pic1", header::PIC1_PNG)]
        .into_iter()
        .find_map(|(name, id)| {
            let picture = packages
                .iter()
                .find_map(|package| header::header_entry(package, id))
                .and_then(|bytes| decode(&bytes, &format!("{} {name}", path.display())));
            one(name, picture, None)
        })
}

/// A disc image or a `.vpk`, told apart by which of the three art files it holds.
fn disc(path: &Path) -> Option<Art> {
    let mut image = DiscImage::open(path).ok()?;
    let entries = image.entries().map(<[_]>::to_vec).ok()?;
    let mut read = |wanted: &str| {
        // A package prefixes each file with its archive's name, so the match is
        // on the tail.
        let entry = entries.iter().find(|entry| {
            !entry.is_directory
                && (entry.path.eq_ignore_ascii_case(wanted)
                    || entry
                        .path
                        .to_ascii_lowercase()
                        .ends_with(&format!("/{}", wanted.to_ascii_lowercase())))
        })?;
        let bytes = image.read_entry(entry).ok()?;
        decode(&bytes, &format!("{} {wanted}", path.display()))
    };
    if let Some(pic1) = read("PSP_GAME/PIC1.PNG") {
        return one("pic1", Some(pic1), None);
    }
    if let Some(pic1) = read("PS3_GAME/PIC1.PNG") {
        let logo = read("PS3_GAME/ICON0.PNG");
        return one("pic1", Some(pic1), logo);
    }
    if let Some(pic0) = read("sce_sys/pic0.png") {
        return one("pic0", Some(pic0), None);
    }
    one("title", title_screen::compose(path), None)
}

fn one(name: &'static str, picture: Option<Picture>, logo: Option<Picture>) -> Option<Art> {
    picture.map(|picture| Art {
        name,
        picture,
        logo,
    })
}

/// A loose file, absent without complaint when it is not there.
fn file(path: &Path) -> Option<Picture> {
    let bytes = std::fs::read(path).ok()?;
    decode(&bytes, &path.display().to_string())
}

fn decode(bytes: &[u8], what: &str) -> Option<Picture> {
    let Some(image) = oag_texture::replacer::decode_png(bytes) else {
        log::warn!("{what} is not a PNG this build reads; the card shows the name");
        return None;
    };
    Some(Picture {
        width: image.width,
        height: image.height,
        rgba: image.rgba,
    })
}
