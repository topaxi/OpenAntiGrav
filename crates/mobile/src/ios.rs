//! iOS: the app container's folders. See docs/tools/ios.md.
//!
//! winit's iOS backend needs nothing handed to it: `EventLoop::new()` runs
//! `UIApplicationMain` itself, so this module has no event loop of its own.

use std::path::PathBuf;

/// The folder a player drops disc images into, relative to [`prepare`]'s
/// working directory, as on every other platform.
pub const IMAGES: &str = "data/images";

/// That folder as the Files app shows it: the container's real path is a UUID
/// the player cannot browse to. Names the chooser's not-found screen.
pub const IMAGES_IN_FILES: &str = "Files > On My iPhone > OpenAntiGrav > data > images";

/// Makes the app container behave like the checkout the rest of the game
/// expects.
///
/// - The working directory becomes `Documents`, so `data/images` is
///   `Documents/data/images`: the one folder the Files app shows (the bundle's
///   own directory, the default working directory, is read-only). The bundle
///   sets `UIFileSharingEnabled` and `LSSupportsOpeningDocumentsInPlace` for it.
/// - Settings, records, ghosts and caches need no help: `dirs` maps iOS to
///   `$HOME/Library/Application Support` and `$HOME/Library/Caches` (`$HOME` is
///   the container the OS sets), and ignores `$XDG_*` there, so setting those as
///   Android does would do nothing. The log's directory is the one place that
///   honours `XDG_STATE_HOME`; `$HOME/Library/Application Support` is where it
///   falls back, which is where it should be.
///
/// Returns the images folder, created if it was missing.
pub fn prepare() -> Option<PathBuf> {
    let documents =
        PathBuf::from(std::env::var_os("HOME").filter(|h| !h.is_empty())?).join("Documents");
    let images = documents.join(IMAGES);
    if let Err(why) = std::fs::create_dir_all(&images) {
        log::warn!("could not create {}: {why}", images.display());
    }
    if let Err(why) = std::env::set_current_dir(&documents) {
        log::warn!("could not enter {}: {why}", documents.display());
    }
    Some(images)
}
