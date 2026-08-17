//! What a title ships, described in types a title package fills in.
//!
//! This crate is the *vocabulary* half of [ADR-0022]. It holds no constant that
//! belongs to any game: `oag-pulse` and `oag-pure` supply those, and
//! `oag-assets` reads them.
//!
//! # Two questions, and only one of them is here
//!
//! **"How does this byte stream decode?" is answered by the file**, inside
//! `oag-formats`, from the artifact's own version word. Class-ID tables, header
//! shapes and schema variants live there and are selected per blob, the same way
//! PSP and PS2 have always been told apart. No type in this crate describes a
//! file's contents, and adding one would be the first step toward a decoder that
//! has to be told which disc it is reading.
//!
//! **"What does this title ship?" is answered here.** Which archives exist and
//! what they are called is not in any file - it is a property of the release -
//! so it is the one thing a title package has to state.
//!
//! # Why this is so small
//!
//! ADR-0022 supersedes ADR-0009's refusal to abstract at n=1, but only for the
//! axes where a second corpus has actually been measured. `pure-status.md`
//! measured Pure's archive layout; it did not map Pure's HUD atlas or its mode
//! set. Types for those would be designed from one example, which is the failure
//! ADR-0009 named and ADR-0022 does not license. They stay as plain constants
//! inside the title crate that knows them until a second title forces their
//! shape.
//!
//! [`boot`] is the one axis that has since been forced, and by measurement:
//! both titles' boot sequences were cold-booted, they differ in length as well as
//! in content, and neither can be derived from the other or from its own XML. See
//! [ADR-0023], which supersedes ADR-0022 item 4 for that axis alone.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
//! [ADR-0023]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0023-boot-sequence-as-title-data.md

pub mod boot;
pub mod menu;
pub mod race;

pub use boot::{BootProfile, BootStep};
pub use menu::MenuSkin;
pub use oag_disc::Platform;
pub use race::RaceDefaults;

/// One title's release-level facts.
///
/// A `&'static Title` is chosen once, by the composition root, from the title
/// crate the build is for. Nothing dispatches through it per call, and it is
/// deliberately a struct of tables rather than a trait: the differences between
/// two Wipeout releases are data, and a trait would put a virtual boundary
/// exactly where the PSP/PS2 split proved none is needed.
/// Not `Eq`: [`menu::MenuSkin`] carries the front end's offsets and scales as
/// `f32`, which is the unit the disc authors them in. Comparing two titles for
/// equality is not something this build does - selection is by `&'static`
/// identity - so the bound is not worth converting a layout table to fixed
/// point for.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Title {
    /// The title as a person would name it, for reports and error messages.
    pub name: &'static str,
    /// Which bulk and companion archives its releases carry.
    pub archives: ArchiveCandidates,
    /// Serials known to belong to a *different* title. See
    /// [`ArchiveCandidates`] for why name matching alone cannot tell them apart.
    pub foreign_serials: &'static [ForeignSerial],
    /// This title's front end, or `None` when none of it has been recovered.
    ///
    /// See [`FrontEnd`] for why the layout and the boot chain are one field
    /// rather than two, and why `None` is a measurement rather than a hole
    /// waiting to be filled.
    pub front_end: Option<&'static FrontEnd>,
    /// What a race falls back to when the caller names no circuit or team. See
    /// [`race::RaceDefaults`].
    ///
    /// Hung off `Title` for the same reason [`Self::boot`] is: it is needed
    /// **before** the archives are open, because a command line has to name a
    /// circuit, and the alternative is a caller comparing titles to pick a
    /// constant.
    ///
    /// That comparison is not merely awkward, it does not work: a title package
    /// declares its `Title` as a `const`, so `std::ptr::eq` against it compares
    /// promoted temporaries whose addresses need not be equal. Carrying the
    /// answer removes the question.
    pub race: &'static race::RaceDefaults,
}

/// One title's front end: how it lays menus out, and how it boots into them.
///
/// # Why the two are one field
///
/// They are recovered together and they are useless apart. A boot chain walks
/// screens whose layout comes from the skin; a skin with no chain has nothing
/// to open it. Carrying them as two independent `Option`s would admit three
/// states, two of which no title can be in - and `oag-game`'s `load_shell`
/// would then need two refusals where one is the honest answer.
///
/// # Why `None` is a result
///
/// Wipeout HD ships `/data/plugins/frontend/gui/skin.xml` and a directory of
/// screen definitions beside it, and **not one number out of either has been
/// read**. A [`MenuSkin`](menu::MenuSkin) filled in from Pulse's would be a
/// table of measurements attributed to a disc nobody measured, which is exactly
/// what `CLAUDE.md`'s "never invent what the assets already author" forbids.
/// `None` says the front end is unrecovered; a caller that needs one refuses by
/// name and says so, which is a visible absence rather than a wrong picture.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrontEnd {
    /// How this title lays menus out. See [`menu::MenuSkin`].
    ///
    /// A measured axis: both PSP titles' `Skin.xml` files were read and they
    /// agree on nothing they share. Presentation only - the menu tree itself is
    /// this project's, not the disc's, and lives in `assets/ui/menu.toml`.
    pub menu: &'static menu::MenuSkin,
    /// How this title's own boot sequence goes. See [`boot::BootProfile`].
    ///
    /// Hung off [`Title`] rather than selected separately so that there is
    /// **one** selection point: opening one title's archives while driving
    /// another's chain is then not an inconsistency to be avoided but a state
    /// that cannot be constructed. The build reached this design after three
    /// independent screen-name probes had each been answering "which title is
    /// this?" in their own words, with three chances to disagree.
    ///
    /// `oag-assets` receives this and never reads it, which is the price: the
    /// asset layer holds a field of screen names it has no business in. Inert
    /// data, and cheaper than two things to keep in step.
    pub boot: &'static boot::BootProfile,
}

/// The archive names a title's releases carry, in the order they are tried.
///
/// **Names, not paths, and found rather than derived.** The candidates are
/// matched against a source's own file list by trailing path component, so a
/// PS2 pressing whose serial directory differs needs no change here, and a
/// source that identifies as neither console still opens if it holds an archive
/// one of them would recognise. A path constant would be right for exactly one
/// pressing.
///
/// The [`Platform`] on each candidate is what the archive *implies* about its
/// source, used only when the source itself says nothing. Nothing branches on
/// it: every decode this project has is chosen by the data rather than by the
/// disc it came off.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ArchiveCandidates {
    /// The bulk archive: tracks, ships, handling.
    pub data: &'static [(&'static str, Platform)],
    /// The companion archive, for the releases that have one.
    pub fe: &'static [(&'static str, Platform)],
}

/// A serial positively identified as belonging to some other title.
///
/// **This rules a source out, never in, and that asymmetry is the point.** The
/// serials any one title has actually been verified against are not the full
/// universe of its legitimate pressings, so an allow-list would hard-reject a
/// real player's own disc - worse than not checking at all. A serial absent from
/// every list gets no verdict and still has to find its archives by name.
///
/// The check exists because sibling titles ship archives under identical names:
/// Pure's PSP disc carries `Data.wad` and `FE.wad` exactly as Pulse does, so
/// name matching alone would open it as if it were Pulse.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForeignSerial {
    /// The serial, normalised as `AAAA-NNNNN`.
    pub serial: &'static str,
    /// What it actually is, for the error message.
    pub title: &'static str,
}

impl Title {
    /// What `serial` really belongs to, if this title knows it belongs to
    /// something else.
    #[must_use]
    pub fn foreign_title(&self, serial: &str) -> Option<&'static str> {
        self.foreign_serials
            .iter()
            .find(|known| known.serial == serial)
            .map(|known| known.title)
    }

    /// Every archive name this title's releases might carry, bulk first.
    ///
    /// For the "looked for" list in a "no archive here" error.
    #[must_use]
    pub fn archive_names(&self) -> Vec<String> {
        self.archives
            .data
            .iter()
            .chain(self.archives.fe)
            .map(|(name, _)| (*name).to_string())
            .collect()
    }
}
