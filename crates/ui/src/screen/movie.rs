//! The `Movie` widget: a `src` naming a video, and where on disc it resolves.

use super::Node;

/// Container extensions a `Movie` widget's `src` may already carry.
///
/// The PS2 front-end XML spells them lower case; the disc's own ISO 9660
/// directory spells the files upper case, so the test ignores case rather than
/// picking a side.
/// `.bik` is Wipeout HD's, and it is here for the same reason the PS2's two
/// are: its `<Movie>` widget names `Data/FE/Images/StudioLiverpool.bik`,
/// extension included, and appending `.PMF` to that would ask for nothing at
/// all.
pub const MOVIE_EXTENSIONS: [&str; 4] = [".pmf", ".pss", ".ipf", ".bik"];

/// [`Movie::entry_name`]'s `region` for a caller with no pressing to read one
/// off - this project's own "prefer EU over USA" convention, restated here
/// because this is a generic widget type with no per-title serial to consult.
/// Only Pure's own boot resolves a real one; every other title's widgets are
/// never `localised`, so this default is never actually exercised there.
pub const DEFAULT_REGION: &str = "EU";

pub(super) fn has_movie_extension(src: &str) -> bool {
    let lower = src.to_ascii_lowercase();
    MOVIE_EXTENSIONS.iter().any(|ext| lower.ends_with(ext))
}

/// One `Movie` widget's attributes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Movie {
    /// The `src` attribute.
    ///
    /// A base name with **no extension** on the PSP. On the PS2 it carries its
    /// own - see [`Movie::entry_name`].
    pub src: String,
    /// Whether the movie has sound.
    pub sound: bool,
    /// Play as soon as the screen is entered.
    pub autostart: bool,
    /// Loop.
    pub repeat: bool,
    /// Fire the screen's `AutoRedirect` when the movie finishes.
    pub autoredirect: bool,
    /// Whether the `localised` attribute was present and true.
    pub localised: bool,
}

impl Movie {
    /// The archive entry name, assembled the way the widget assembles it.
    ///
    /// `Movie_ParseAttributes` appends `.PMF` when `localised` is absent, and
    /// `"_{region}.PMF"` otherwise. The PSP intro's `src` is
    /// `Data\Movies\Intro`, which is why the filename never turned up in a
    /// string search of the executable.
    ///
    /// **`region` is not read from this widget or this disc - it names which
    /// pressing's executable is running.** `Movie_ParseAttributes` bakes
    /// exactly one literal suffix into each pressing's own binary (`"_EU"` on
    /// the EU build, `"_US"` on the USA one - see
    /// `docs/ghidra/functions/psp-pure-eu/movie-localised-suffix.md`), the same
    /// mechanism `oag_pure::frontend::title_frame_src` already resolves for
    /// `TitleFrame`'s wordmark. A caller that cannot say which pressing it has
    /// passes `"EU"`, this project's own default for an unresolved source; a
    /// caller that can (Pure's own boot, keyed off
    /// [`oag_assets::Layout::serial`]) passes
    /// `oag_pure::frontend::localised_movie_region`'s result instead. Ignored
    /// entirely when `localised` is false - most titles' widgets, which carry
    /// no region axis at all.
    ///
    /// **The PS2 build diverges, and its own XML is the evidence**: its `src`
    /// values are `Data\Movies\Intro.pss` and `Data\Movies\Backdrop.ipf`,
    /// extension included, and `SCES_547.48` matches on exactly those spellings
    /// (`FUN_0019b168` tests for `\Intro.pss` and `\Backdrop.ipf` before
    /// rewriting them to the region's cut). Appending `.PMF` to those would ask
    /// for `Data\Movies\Backdrop.ipf.PMF`, which is nothing at all - so a `src`
    /// that already names its container is taken as it stands. See
    /// [`MOVIE_EXTENSIONS`].
    #[must_use]
    pub fn entry_name(&self, region: &str) -> String {
        if has_movie_extension(&self.src) {
            return self.src.clone();
        }
        if self.localised {
            format!("{}_{region}.PMF", self.src)
        } else {
            format!("{}.PMF", self.src)
        }
    }

    pub(super) fn from_node(node: &Node) -> Self {
        Self {
            src: node.value("src").unwrap_or_default().to_string(),
            sound: node.flag("sound").unwrap_or(false),
            autostart: node.flag("autostart").unwrap_or(false),
            repeat: node.flag("repeat").unwrap_or(false),
            autoredirect: node.flag("autoredirect").unwrap_or(false),
            localised: node.flag("localised").unwrap_or(false),
        }
    }
}
