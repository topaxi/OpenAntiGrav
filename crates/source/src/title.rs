//! Which Studio Liverpool title a source is, when it might be more than one.
//!
//! **Every title this crate opens now reaches a race.** Pulse and Pure both
//! do it through their own boot chain, START at `Title Screen` and out.
//! **Wipeout 2048 does it through `--race` alone**, and that stopped being
//! "because its front end is unwired" on [ADR-0054] - its boot chain is real
//! and `Title::front_end` is `Some` - and started being "because its front
//! end draws no `MenuSkin`-shaped menu": `load_shell` refuses the
//! menu-driven boot by name on that narrower ground and points at `--race`
//! instead. See roadmap M8 for the dates each title's boot landed. DLC packs
//! remain Pulse-only - `open_source` drops `packs` rather than mounting
//! Pulse's own DLC behind a different title, since no other title's pack
//! format is described yet.
//!
//! [ADR-0054]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0054-a-touch-front-end-is-a-second-axis-not-a-menuskin-variant.md

use oag_assets::{Archives, Error, Result, dlc::Pack};
use oag_title::Title;

/// A source, opened as whichever title it turned out to be.
#[derive(Debug)]
pub struct Opened {
    pub archives: Archives,
    /// Which title identified it - carried rather than discarded.
    ///
    /// **This is the strongest evidence anyone gets about which title a source
    /// is**, and it is available before a line of front-end XML has been parsed:
    /// it comes from the serial in `UMD_DATA.BIN`, through the deny-list below.
    /// This build used to throw it away here and then re-derive it four separate
    /// times downstream by asking whether some screen name existed - three of
    /// those asking in different words, each a fresh chance to disagree.
    ///
    /// It carries [`oag_title::Title::boot`] with it, which is what
    /// [ADR-0023](../../../docs/architecture/adr/0023-boot-sequence-as-title-data.md)
    /// consumes.
    pub title: &'static Title,
}

/// Opens `source` as whichever title it identifies as.
///
/// Tries Wipeout Pulse first - the title every other call site in this crate
/// already assumes - and falls back to Wipeout Pure only when Pulse's own
/// deny-list names it explicitly. Any other failure (a directory with
/// nothing recognisable in it, an image that will not open at all) is
/// reported as Pulse's error, since Pulse is what a caller with no better
/// information should be told about.
///
/// **`packs` and `pure_packs` are two different lists, not a title guess.**
/// [`crate::dlc::packs`] and [`crate::dlc::pure_packs`] discover them
/// separately, from the same `data/dlc` folder - see those functions' docs
/// for why one shared list would leak a pack into the wrong title. `packs`
/// mounts behind Pulse; `pure_packs` mounts behind Pure only when the
/// fallback below actually fires, so a Pulse source never even asks Pure's
/// own list to exist.
///
/// # Errors
///
/// Propagates [`oag_pulse::open_with_packs`] and [`oag_pure::open_with_packs`].
pub fn open_source(source: &str, packs: Vec<Pack>, pure_packs: Vec<Pack>) -> Result<Opened> {
    // **Tried before Pulse, and by a different test.** Pure is reached by
    // Pulse's deny-list naming it, because the two ship archives under identical
    // names; HD shares no archive name with either, so there is nothing for a
    // deny-list to catch and the question is simply whether this source carries
    // a `.PSARC`. Asking that first costs a Pulse open on an HD disc and keeps
    // the Pulse/Pure pair exactly as it was.
    // **Tried before HD, on the same rule and for the same reason.** Wipeout
    // 2048 shares no archive name with any of the other three - its bulk is
    // `PSP2/data.psarc` and nothing else in the lineage ships that - so the
    // question is again simply whether this source carries the one archive
    // this title names. Ahead of HD because 2048's package is a directory and
    // HD's candidates are `PS3_GAME/USRDIR/DATA0*.PSARC`, so neither can match
    // the other's source and the order between them is free; putting the
    // cheapest-to-refuse first is the only thing that decides it.
    if let Ok(archives) = oag_2048::open(source) {
        return Ok(Opened {
            archives,
            title: oag_2048::TITLE,
        });
    }

    // **HD's refusal is kept for one case.** An encrypted PS3 image is HD's
    // to name; falling through to Pulse would report it as a missing `Data.wad`.
    let hd_encrypted = match oag_hd::open(source) {
        Ok(archives) => {
            let title = oag_hd::title_of(&archives);
            return Ok(Opened { archives, title });
        }
        Err(error @ Error::EncryptedDisc { .. }) => Some(error),
        Err(_) => None,
    };

    // **Tried before Pulse, on the same rule as 2048's and HD's above.**
    // Omega shares no archive name with any of the other three either: its
    // bulk candidate is `uroot/data09.psarc`, a tail neither Pulse/Pure's
    // `Data.wad`/`FE.wad`, HD's `PS3_GAME/USRDIR/DATA0*.PSARC` nor 2048's
    // `PSP2/data.psarc` can match - `crates/omega/tests/
    // omega_title_ground_truth.rs`'s `hd_and_2048_both_refuse_the_omega_source`
    // and `omega_refuses_hd_and_2048_sources` run this both ways rather than
    // arguing it. Ordered after 2048 and HD only because there is nothing to
    // order by - all three directory-sourced titles are equally cheap to
    // refuse, so this just keeps the newest addition last.
    if let Ok(archives) = oag_omega::open(source) {
        return Ok(Opened {
            archives,
            title: oag_omega::TITLE,
        });
    }

    if let Some(error) = hd_encrypted {
        return Err(error);
    }

    match oag_pulse::open_with_packs(source, packs) {
        Ok(archives) => Ok(Opened {
            archives,
            title: oag_pulse::TITLE,
        }),
        Err(error) => match deny_list_opener(&error) {
            Some(opener) => (opener.open)(source, pure_packs).map(|archives| Opened {
                archives,
                title: opener.target,
            }),
            None => Err(error),
        },
    }
}

/// The opener for the title `error` says the source belongs to, when Pulse's
/// deny-list named one this build can open.
fn deny_list_opener(error: &Error) -> Option<&'static DenyListOpener> {
    let Error::WrongTitle { title: named, .. } = error else {
        return None;
    };
    PULSE_DENY_LIST_OPENERS
        .iter()
        .find(|opener| opener.target.name == named)
}

/// A title Pulse's deny-list can hand a source over to, and how to open it.
struct DenyListOpener {
    target: &'static Title,
    open: fn(&str, Vec<Pack>) -> Result<Archives>,
}

/// The titles whose serials Pulse's own deny-list names (it rules them out by
/// the name of the title they belong to), each with its opener. A source Pulse
/// refuses as a title not listed here is still reported as Pulse's error.
const PULSE_DENY_LIST_OPENERS: &[DenyListOpener] = &[DenyListOpener {
    target: oag_pure::TITLE,
    open: oag_pure::open_with_packs,
}];

/// Which title a source is, for a caller that needs the answer before it has
/// any other reason to open the archives.
///
/// **`None` means "could not tell", not "neither"** - an unreadable path, a
/// directory with nothing recognisable in it. Every caller so far is choosing a
/// *default* (`oag-game`'s `--track` and `--team`, which name different files on
/// the two discs), and there the right response to not knowing is to carry on
/// and let the real load report the real problem. An error return would make
/// that awkward for no gain.
///
/// This does open the source, so it is not free - the same work [`open_source`]
/// does, with the archives dropped. Deliberate rather than an oversight:
/// threading a half-opened source through argument parsing to save one archive
/// open would put the disc's layout into the shape of the command line. If it
/// ever shows up in a profile, the serial in `UMD_DATA.BIN` is what actually
/// answers this and reading it alone is far cheaper than mounting anything.
#[must_use]
pub fn identify(source: &str) -> Option<&'static Title> {
    open_source(source, Vec::new(), Vec::new())
        .ok()
        .map(|opened| opened.title)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A source that will not open is `None` rather than a guess.
    #[test]
    fn a_source_that_cannot_be_opened_identifies_as_nothing() {
        assert!(identify("does/not/exist.chd").is_none());
    }

    /// A source neither title's deny-list names anything about - the
    /// ordinary "no such file" case - stays Pulse's error, unchanged by this
    /// module existing.
    #[test]
    fn a_source_that_is_neither_titles_error_is_pulses() {
        let error = open_source("does/not/exist.chd", Vec::new(), Vec::new()).unwrap_err();
        assert!(!matches!(error, Error::WrongTitle { .. }), "{error}");
    }
}
