//! The files that sit beside a `.vex`: its `.rcsmodel`, `.rcsskeleton`,
//! `.rcsanimclip` and `.pvs`, and the Zone-mode set where a title ships one
//! (`docs/formats/rcsmodel-siblings.md`).
//!
//! **One rule, written as data.** Every file in a set shares one stem, and a
//! set is one of three spellings of that stem:
//!
//! | spelling | stem | where |
//! | --- | --- | --- |
//! | [`Spelling::Plain`] | `<dir>/track` | HD, 2048, Omega's `zone_N` circuits, every ship |
//! | [`Spelling::Cooked`] | `<dir>/track.final` | Omega's 22 other circuits, forward and reversed |
//! | [`Spelling::Zone`] | `<dir>/trackZone` | 2048's Zone-mode model, which has its own skeleton, clip and PVS |
//!
//! A spelling is not keyed by title, because the choice is per circuit and is
//! decided by which file the archive holds (Omega's `zone_N` circuits use the
//! plain name and the rest use `.final`, never both in one directory), so a
//! title axis could not decide it without probing the archive anyway. The
//! resolver therefore answers with candidates in the order a loader tries
//! them, and the caller, which owns the archive, takes the first whose model
//! reads ([`Siblings::probe`]). A model already found gives back its own set
//! ([`Siblings::beside`]).

/// How a set's stem is spelled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spelling {
    /// The `.vex`'s own stem.
    Plain,
    /// The `.vex`'s own stem under Omega's `.final` infix.
    Cooked,
    /// `trackZone` in the `.vex`'s directory.
    Zone,
}

impl Spelling {
    /// Every spelling that is not the Zone one, in the order a loader tries
    /// them: plain first, so a title that ships no `.final` name is never
    /// asked for one.
    const RACE: [Self; 2] = [Self::Plain, Self::Cooked];

    /// The infix between a `.vex`'s stem and the extension of a companion.
    const fn infix(self) -> &'static str {
        match self {
            Self::Plain | Self::Zone => "",
            Self::Cooked => ".final",
        }
    }
}

/// The extension of each companion, in [`Siblings`]' field order.
const MODEL: &str = ".rcsmodel";
const SKELETON: &str = ".rcsskeleton";
const CLIP: &str = ".rcsanimclip";
const PVS: &str = ".pvs";

/// The stem `Zone` swaps in for the `.vex`'s own file name.
const ZONE_STEM: &str = "trackZone";

/// The entry names of one stem's companions.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Siblings {
    /// Which spelling this is.
    pub spelling: Spelling,
    /// The `.rcsmodel`.
    pub model: String,
    /// The `.rcsskeleton`; HD ships none.
    pub skeleton: String,
    /// The `.rcsanimclip`; HD ships none.
    pub clip: String,
    /// The `.pvs`.
    pub pvs: String,
}

impl Siblings {
    fn of_stem(spelling: Spelling, stem: &str) -> Self {
        Self {
            spelling,
            model: format!("{stem}{MODEL}"),
            skeleton: format!("{stem}{SKELETON}"),
            clip: format!("{stem}{CLIP}"),
            pvs: format!("{stem}{PVS}"),
        }
    }

    /// The `.vex` name's stem. The game's own spelling is case-insensitive in
    /// both containers, and a PSP-shaped name reaches here through `oag_hd`'s
    /// tables. `None` for a name that does not end in `.vex`.
    fn vex_stem(vex_name: &str) -> Option<&str> {
        let at = vex_name.len().checked_sub(4)?;
        vex_name[at..]
            .eq_ignore_ascii_case(".vex")
            .then(|| &vex_name[..at])
    }

    /// The set a `.vex` has in one spelling, or `None` for a name that is not
    /// a `.vex`.
    #[must_use]
    pub fn spelled(vex_name: &str, spelling: Spelling) -> Option<Self> {
        let stem = Self::vex_stem(vex_name)?;
        Some(match spelling {
            Spelling::Zone => {
                let dir_end = vex_name.rfind(['\\', '/']).map_or(0, |at| at + 1);
                Self::of_stem(spelling, &format!("{}{ZONE_STEM}", &vex_name[..dir_end]))
            }
            _ => Self::of_stem(spelling, &format!("{stem}{}", spelling.infix())),
        })
    }

    /// The plain set beside a `.vex`: what every ship, HD, 2048 and a
    /// non-`.final` Omega circuit ships. `None` for a name that is not a
    /// `.vex`.
    #[must_use]
    pub fn plain(vex_name: &str) -> Option<Self> {
        Self::spelled(vex_name, Spelling::Plain)
    }

    /// The sets to try for a `.vex`, most preferred first: the Zone set when
    /// `zone` asks for it (a Zone race on 2048; the caller passes over a model
    /// that is not the Zone one), then the plain one, then Omega's `.final`
    /// one. Empty for a name that is not a `.vex`.
    #[must_use]
    pub fn probe(vex_name: &str, zone: bool) -> Vec<Self> {
        let zone = zone.then_some(Spelling::Zone);
        zone.into_iter()
            .chain(Spelling::RACE)
            .filter_map(|spelling| Self::spelled(vex_name, spelling))
            .collect()
    }

    /// The set that sits beside an `.rcsmodel` already found, whatever its
    /// spelling: `track.rcsmodel` gives `track.pvs`, Omega's
    /// `track.final.rcsmodel` gives `track.final.pvs`. `None` for a name that
    /// is not an `.rcsmodel`.
    ///
    #[must_use]
    pub fn beside(model_name: &str) -> Option<Self> {
        let at = model_name.len().checked_sub(MODEL.len())?;
        if !model_name[at..].eq_ignore_ascii_case(MODEL) {
            return None;
        }
        let stem = &model_name[..at];
        let file = stem.rsplit(['\\', '/']).next().unwrap_or(stem);
        let spelling = if file == ZONE_STEM {
            Spelling::Zone
        } else if stem.ends_with(Spelling::Cooked.infix()) {
            Spelling::Cooked
        } else {
            Spelling::Plain
        };
        Some(Self::of_stem(spelling, stem))
    }
}

#[cfg(test)]
mod tests;
