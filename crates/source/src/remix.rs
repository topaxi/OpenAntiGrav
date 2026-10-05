//! Where a race's track and its craft come from: the same source for both in
//! the ordinary case, two different ones for a Race Remix.
//!
//! Lives beside [`crate::title`] rather than on `oag-title`, which stays deliberately
//! types-only, no title's data: a `Title` value assumes one archive-candidate source,
//! so it is not the right shape to *represent* a remix, only to be one of the two
//! things a remix holds.
//!
//! Nothing here is a new abstraction over `Title` - two [`Opened`] values
//! coexisting is unremarkable, plain owned data with no global state
//! (the movie prefetch's `Vec<oag_assets::Archive>` already does the same for
//! one title). What is new is naming the split: which loader reads track
//! content (collision, environment, pads, weapon tuning) and which reads
//! craft content (livery, HUD, exhaust/flare, handling stats, boost plume) -
//! see `oag_raceplay`'s `load.rs`.

use crate::title::Opened;
use oag_assets::{Result, dlc::Pack};
use oag_title::Title;

/// A race's track and craft source, together.
///
/// `Single`'s own [`Opened`] costs the same as `Split`'s: one of the two
/// fields, not both - `Split` is twice the size, not a mismatch clippy's
/// default threshold should read as suspicious. `oag_assets::Layout` gaining
/// a `serial` field is what pushed this over the lint's own limit; boxing
/// either field would only move the cost from this enum onto every match
/// site instead of removing it, for a type this crate opens at most twice per
/// race and never in a hot loop.
#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum Remix {
    /// Track and craft load from the same opened source - today's behaviour,
    /// and the only case before Race Remix existed.
    Single(Opened),
    /// A Race Remix: track and craft come from different sources.
    Split { track: Opened, craft: Opened },
}

impl Remix {
    /// Opens `source` for the track, and `craft_source` for the craft when it
    /// names something other than `source`. `packs`/`pure_packs` mount
    /// behind `source` only - a remix does not widen what a DLC pack may be
    /// mounted behind, on the same terms [`crate::title::open_source`]
    /// already applies to an ordinary race.
    pub fn open(
        source: &str,
        craft_source: Option<&str>,
        packs: Vec<Pack>,
        pure_packs: Vec<Pack>,
    ) -> Result<Self> {
        let track = crate::title::open_source(source, packs, pure_packs)?;
        match craft_source {
            Some(craft_source) if craft_source != source => {
                let craft = crate::title::open_source(craft_source, Vec::new(), Vec::new())?;
                Ok(Self::Split { track, craft })
            }
            _ => Ok(Self::Single(track)),
        }
    }

    /// Splits into the track's opened source, the craft's own when this is a
    /// genuine remix (`None` in the ordinary single-source case), and both
    /// titles - track first, craft second, equal to each other when `Single`.
    ///
    /// Returned by value rather than through `&mut self` accessors on
    /// purpose: `race::load` reads both `archives` on and off throughout its
    /// whole body, and a borrow taken from a method call would have to stay
    /// live for that entire span, which rules out ever calling the other
    /// accessor again. Two owned locals - `archives` and `Option<Archives>` -
    /// have no such conflict.
    #[must_use]
    pub fn into_parts(self) -> (Opened, Option<Opened>, &'static Title, &'static Title) {
        match self {
            Self::Single(opened) => {
                let title = opened.title;
                (opened, None, title, title)
            }
            Self::Split { track, craft } => {
                let (track_title, craft_title) = (track.title, craft.title);
                (track, Some(craft), track_title, craft_title)
            }
        }
    }
}

/// `craft_archives` when this is a genuine remix, `archives` itself
/// otherwise - the one line every craft-governed load in `race/load.rs`
/// wraps its archive argument in, for the same reason [`Remix::into_parts`]
/// returns owned values rather than borrows: two short-lived reborrows here
/// beat one long-lived one there.
pub fn craft_of<'a>(
    craft_archives: &'a mut Option<oag_assets::Archives>,
    archives: &'a mut oag_assets::Archives,
) -> &'a mut oag_assets::Archives {
    craft_archives.as_mut().unwrap_or(archives)
}
