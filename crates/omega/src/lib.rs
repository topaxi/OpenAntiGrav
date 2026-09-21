//! What Wipeout: Omega Collection ships, on PS4.
//!
//! A title package in the sense of [ADR-0022], and the fifth. Its front end
//! is not a rewrite: [`docs/formats/omega-frontend.md`] found that
//! `Data\Plugins\Frontend\Gui\Skin.xml`'s `FEGlobals` block is bit-identical
//! to [Wipeout HD's own][hd-frontend.md], the boot chain is declared with the
//! same eight screens in the same order, and this project's existing
//! `oag_ui::screen::Screens` reader parses all three of Omega's own copies
//! (`skin.xml`, `mainmenu_definition.xml`, `cellmode_definition.xml`) with no
//! code change at all. This crate is mostly the plumbing that static reading
//! had nowhere to land: archive candidates, the boot profile, and Omega's own
//! re-derived [`frontend::MENU_SKIN`] numbers.
//!
//! # Deliberately thin, and racing is out of scope
//!
//! Like [`oag_2048`], this crate holds only what was actually measured. Two
//! gaps are structural rather than an oversight:
//!
//! - **No PS4 emulator capture exists in this project's toolchain**, so
//!   [`frontend::BOOT`]'s provenance is
//!   [`oag_title::Provenance::Declared`], never `Measured` - nobody has
//!   watched an Omega boot the way [ADR-0025] asks for before it upgrades.
//! - **Racing is unread.** Omega's `.rcsmodel`/`.gnf`/`.vex` files are either
//!   undecoded (`.gnf`, a PS4 texture container this project has no reader
//!   for) or read as garbage past a corrupted block-data offset (the same
//!   trap [`psarc.md`] documents for the base five archives). [`race`]'s own
//!   doc comment says what that means for [`oag_title::RaceDefaults`]'s
//!   otherwise-mandatory fields.
//!
//! # The source is a directory pair, and the patch is mandatory
//!
//! Wipeout: Omega Collection ships as a base `.pkg` plus a day-one patch that
//! **replaces** the front end wholesale rather than supplementing it -
//! `skin.xml`, `mainmenu_definition.xml`, `racebox_definition.xml` and
//! `cellmode_definition.xml` exist **only** in the patch's `data09.psarc`,
//! never in any of the base package's five archives. See
//! [`archives`] for how [`TITLE`] makes the patch load-bearing rather than
//! optional.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
//! [ADR-0025]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0025-a-boot-chain-carries-its-provenance.md
//! [`docs/formats/omega-frontend.md`]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/formats/omega-frontend.md
//! [hd-frontend.md]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/formats/hd-frontend.md
//! [`psarc.md`]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/formats/psarc.md

use oag_assets::{Archives, Result};
use oag_title::{ArchiveCandidates, Platform, Title};

pub mod campaign;
pub mod frontend;
pub mod hud;
pub mod race;

/// Wipeout: Omega Collection, as the asset layer needs to know it.
pub const TITLE: &Title = &Title {
    name: "Wipeout: Omega Collection",
    archives: ArchiveCandidates {
        data: DATA_CANDIDATES,
        // **No companion archive - the "bulk" slot already does the job a
        // companion would.** See [`archives`] for why `data09` sits there
        // instead of in `extra`.
        fe: &[],
        extra: EXTRA_CANDIDATES,
    },
    // **Empty, on 2048's own terms.** A foreign serial is how one title's
    // archive names are told apart from another's positively-identified disc
    // serial; a directory source never reaches that check at all (it has no
    // `UMD_DATA.BIN`/`SYSTEM.CNF`/`PS3_DISC.SFB` to read one off). Whether
    // Omega's own archive *names* could be confused with HD's or 2048's is a
    // different, checkable question - see `crates/omega/tests` for the
    // answer, run rather than argued.
    foreign_serials: &[],
    front_end: Some(frontend::FRONT_END),
    hud: hud::LAYOUTS,
    hud_art: hud::ART,
    race: race::DEFAULTS,
    // Unread, on the same terms `oag_2048::TITLE` states: the *files* are
    // there under HD's own spellings, but this build cannot decode their
    // `.rcsmodel` geometry. See `race`'s own doc comment.
    exhaust: &oag_title::exhaust::Exhaust::Unread,
    flare: &oag_title::flare::Flare::Unread,
    // **Split into three files, like 2048's, not combined like HD's or
    // Pulse's/Pure's.** `data09.psarc` carries `data/plugins/teams/
    // Definition.xml`, `data/plugins/tracks/Definition.xml` and
    // `data/plugins/music/Definition.xml` as three separate documents -
    // confirmed by direct listing, not assumed from HD's single
    // `Data\Plugins\Frontend\Definition.xml`. `plugin_definition` names the
    // team one, the same reasoning `oag_2048::names::TEAM_PLUGIN_DEFINITION`
    // states: a race needs a roster before it needs a circuit list.
    plugin_definition: names::TEAM_PLUGIN_DEFINITION,
    track_plugin_definition: Some(names::TRACK_PLUGIN_DEFINITION),
    // Neither read this lane - loading-screen and music plugin XML were not
    // opened. `None` is a gap, not a measurement, on Pure's own terms.
    loading: None,
    music: None,
    cursor: include_str!("../../../assets/cursors/omega.svg"),
    // Unread, on `oag_2048::TITLE`'s own terms: this crate never opened
    // either weapon-stats table this lane, so nothing here can say which of
    // Omega's copies (HD's own two, or a fourth `_2048`-suffixed pair the way
    // 2048 ships) is the one Omega reads from.
    weapons: &oag_title::weapons::Weapons {
        race: r"Data\XML\WeaponStats_Race.xml",
        elimination: Some(r"Data\XML\WeaponStats_Elimination.xml"),
    },
    weapon_models: &oag_title::weapons::WeaponModels::EMPTY,
};

/// Paths inside the packages.
pub mod names {
    /// The teams plugin - see [`crate::TITLE::plugin_definition`]'s own doc
    /// for why this title splits into three where HD ships one.
    pub const TEAM_PLUGIN_DEFINITION: &str = r"Data\Plugins\teams\Definition.xml";
    /// The circuits plugin - [`crate::TITLE::track_plugin_definition`].
    pub const TRACK_PLUGIN_DEFINITION: &str = r"Data\Plugins\tracks\Definition.xml";
    /// The soundtrack plugin. No [`oag_title::Title`] axis names this one
    /// yet (see `oag_2048::names::MUSIC_PLUGIN_DEFINITION` for the same
    /// gap there), so it is recorded here and unread.
    pub const MUSIC_PLUGIN_DEFINITION: &str = r"Data\Plugins\music\Definition.xml";
}

/// The nine archives Wipeout: Omega Collection ships, split across a base
/// package and a mandatory patch.
///
/// Named by tail (`uroot/dataNN.psarc`), the house style
/// [`oag_2048::DATA_CANDIDATES`]'s own doc states, so a `source` pointed at
/// `data/extracted/ps4` (the common parent, this crate's own template) or
/// straight at `data/extracted/ps4/omega-eu-patch` resolves the same way.
///
/// | Archive | Package | Front-end content |
/// | --- | --- | --- |
/// | `data00.psarc` | base | 10 `frontend/gui` XML, no `skin.xml` |
/// | `data01.psarc`-`data04.psarc` | base | none |
/// | `data05.psarc` | patch | 1 `frontend/gui` XML |
/// | `data07.psarc` | patch | 1 `frontend/gui` XML (garbage past a
///   corrupted offset) |
/// | `data08.psarc` | patch | 9 `frontend/gui` XML, the bulk of FE assets |
/// | `data09.psarc` | patch | 45 `frontend/gui` XML, all of `skin.xml`/
///   `mainmenu_definition.xml`/`cellmode_definition.xml`/
///   `racebox_definition.xml`, **and no circuits at all** |
///
/// See [`docs/formats/omega-frontend.md`]'s census for the full table this
/// summarises, and its "Load order is not settled" section for what this
/// module's own ordering choice does and does not claim.
///
/// [`docs/formats/omega-frontend.md`]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/formats/omega-frontend.md
pub mod archives {
    /// Base package, archive 0: no front-end XML, but the game's own
    /// non-front-end content (`ingame_definition.xml`, `manual_definition.xml`
    /// and friends).
    pub const DATA00: &str = "uroot/data00.psarc";
    /// Base package, archives 1-4: no front-end/language content at all.
    pub const DATA01: &str = "uroot/data01.psarc";
    pub const DATA02: &str = "uroot/data02.psarc";
    pub const DATA03: &str = "uroot/data03.psarc";
    pub const DATA04: &str = "uroot/data04.psarc";
    /// Patch, archive 5: one stray front-end file among 1,205 entries.
    pub const DATA05: &str = "uroot/data05.psarc";
    /// Patch, archive 7: six entries total - a stale `cellmode_definition.xml`,
    /// two Korean font pairs, a `Zone` ship model fix.
    pub const DATA07: &str = "uroot/data07.psarc";
    /// Patch, archive 8: the bulk of front-end *assets* - fonts, `.gnf`
    /// images, `.rcsmodel` flyers - and 9 XML files, a different and newer
    /// set than `data00`'s own 10.
    pub const DATA08: &str = "uroot/data08.psarc";
    /// Patch, archive 9: almost entirely front-end/language/plugin XML (91 of
    /// 120 entries) and the **only** archive carrying the complete front-end
    /// file set. No circuits, no ship models, no textures.
    pub const DATA09: &str = "uroot/data09.psarc";

    /// All nine, in package-then-archive-number order.
    pub const ALL: &[&str] = &[
        DATA00, DATA01, DATA02, DATA03, DATA04, DATA05, DATA07, DATA08, DATA09,
    ];
}

/// The bulk archive's one candidate: `data09`, the patch's front-end-complete
/// copy.
///
/// **Not "the biggest archive" or "the base package", the way [`oag_hd`]'s
/// and [`oag_2048`]'s own bulk candidates are - it is the one chosen to win a
/// name collision.** [`oag_assets::Archives::holder_of`] checks `data` before
/// `fe` before `extra`, so whichever archive fills this role is served first
/// for any path more than one archive carries. `data09.psarc` is the only
/// archive with the complete plugin/language set, reads clean far more often
/// than the other four front-end-bearing archives (39 of 45 `frontend/gui`
/// files clean from byte zero, against `data00`'s 8/10), and is the
/// highest-numbered patch archive - the same circumstantial shape
/// `oag_hd::TITLE`'s own `DATA00` had before its RPCS3 capture confirmed it.
/// **This is that shape of evidence again, not the capture: chosen, not
/// measured.** `docs/formats/omega-frontend.md`'s "Load order is not
/// settled" section is the record of what would close it, and the boot
/// report names which archive actually served each front-end file so a
/// future capture can correct this choice without guessing which file to
/// re-check.
///
/// **Also mandatory, which is deliberate.** [`oag_assets::source::Layout::resolve_on`]
/// hard-errors when no candidate here is found, and `data09` exists only in
/// the patch - so a base-only `omega-eu` directory with no `omega-eu-patch`
/// sibling refuses to open at all, honestly, rather than opening a base
/// package whose own front end has no `skin.xml` to boot.
///
/// **Carries no circuit.** A future racing lane will need the real
/// patch-ahead-of-base *overlay* role [`oag_2048`]'s own `EXTRA_CANDIDATES`
/// doc describes and deliberately does not build (searched ahead of, but not
/// replacing, a bulk archive that still carries the game's actual content) -
/// this crate's shortcut of making the patch's front-end archive *be* the
/// bulk archive only works because nothing here reads a circuit through it.
const DATA_CANDIDATES: &[(&str, Platform)] = &[(archives::DATA09, Platform::Ps4)];

/// The other eight archives, all mounted, searched after [`DATA_CANDIDATES`].
///
/// Order among these is unmeasured and does not matter for anything this
/// crate reads today: every front-end path any of the eight also carries is
/// served by `data09` first (see [`DATA_CANDIDATES`]), and nothing else is
/// read here yet.
const EXTRA_CANDIDATES: &[(&str, Platform)] = &[
    (archives::DATA00, Platform::Ps4),
    (archives::DATA01, Platform::Ps4),
    (archives::DATA02, Platform::Ps4),
    (archives::DATA03, Platform::Ps4),
    (archives::DATA04, Platform::Ps4),
    (archives::DATA05, Platform::Ps4),
    (archives::DATA07, Platform::Ps4),
    (archives::DATA08, Platform::Ps4),
];

/// Opens whichever archives `source` carries, as Wipeout: Omega Collection.
///
/// # Errors
///
/// Propagates [`Archives::open`] - including when `source` is the base
/// package alone, with no `omega-eu-patch` sibling: see [`DATA_CANDIDATES`].
pub fn open(source: &str) -> Result<Archives> {
    Archives::open(source, TITLE)
}

#[cfg(test)]
mod tests;
