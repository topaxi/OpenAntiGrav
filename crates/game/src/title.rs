//! Which Studio Liverpool title a source is, when it might be more than one.
//!
//! Everywhere else in this crate names `oag_pulse::TITLE` outright, because
//! everywhere else is gameplay this engine actually plays - a race, the
//! loading screen into one, the music-disc pairing - and Pulse is the only
//! title that gets there. The boot sequence is the one exception: a player
//! can point `--source` at any disc they own, and a Wipeout Pure one should
//! show *its own* intro and menu rather than a named refusal, even though
//! nothing past the menu (a race, DLC) plays it yet. See roadmap M8.

use oag_assets::{Archives, Error, Result, dlc::Pack};

/// Opens `source` as whichever title it identifies as, Pulse or Pure.
///
/// Tries Wipeout Pulse first - the title every other call site in this crate
/// already assumes - and falls back to Wipeout Pure only when Pulse's own
/// deny-list names it explicitly. Any other failure (a directory with
/// nothing recognisable in it, an image that will not open at all) is
/// reported as Pulse's error, since Pulse is what a caller with no better
/// information should be told about.
///
/// **Packs are not mounted behind a Pure source.** Nothing under
/// [ADR-0022](../../../docs/architecture/adr/0022-title-packages.md)
/// describes a Pure pack yet, and mounting Pulse's own DLC behind a
/// different title would be silently wrong rather than merely incomplete -
/// so `packs` is dropped, not carried through, when the fallback fires.
///
/// # Errors
///
/// Propagates [`oag_pulse::open_with_packs`] and [`oag_pure::open`].
pub fn open_source(source: &str, packs: Vec<Pack>) -> Result<Archives> {
    match oag_pulse::open_with_packs(source, packs) {
        Err(Error::WrongTitle { title, .. }) if title == "Wipeout Pure" => oag_pure::open(source),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A source neither title's deny-list names anything about - the
    /// ordinary "no such file" case - stays Pulse's error, unchanged by this
    /// module existing.
    #[test]
    fn a_source_that_is_neither_titles_error_is_pulses() {
        let error = open_source("does/not/exist.chd", Vec::new()).unwrap_err();
        assert!(!matches!(error, Error::WrongTitle { .. }), "{error}");
    }
}
