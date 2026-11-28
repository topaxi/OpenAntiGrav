//! What stands for a title on the launcher where a disc has no system picture.

/// Its title screen's parts: the logo over the menu backdrop movie
/// (`docs/ui/launcher.md`, "The PS2 card").
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LauncherCard {
    /// The logo's image name in the archives.
    pub logo: &'static str,
    /// The backdrop movie's loose files on the disc, the preferred cut first.
    pub backdrop: [&'static str; 2],
}
