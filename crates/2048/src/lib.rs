//! What Wipeout 2048 ships.
//!
//! A title package in the sense of [ADR-0022], and the fourth. It is shaped
//! like none of the other three: not a disc at all but a **decrypted `.pkg`
//! extracted to a directory**, whose bulk is one PSARC with 18,430 entries
//! under `PSP2/data.psarc`.
//!
//! # Deliberately thin, and thinner than `oag-pure`
//!
//! This carries what was measured off `PCSF00007` (the EU release) and nothing
//! else. The scope it was written for is "load a craft and a circuit and
//! spawn", so several axes are honestly empty rather than filled from Wipeout
//! HD by analogy - which would be easy and wrong, because 2048 ships HD's
//! files under HD's names and *changed the containers underneath them*. See
//! `docs/formats/2048-status.md`.
//!
//! - [`Title::front_end`] is `Some` since [ADR-0054]: this build's boot
//!   chain, language plugins and touch-icon layout are read - see
//!   [`frontend`]. Its `menu` axis stays `None`, on evidence rather than a
//!   gap: 2048 authors no `FEGlobals`/`<Menu>`/`<HorizMenu>` vocabulary
//!   anywhere, a touch-icon grid over a persistent scene instead
//!   ([`oag_title::FrontEnd::touch`]).
//! - [`Title::loading`] and [`Title::music`] are `None` on the same terms
//!   `front_end` used to be: real, narrower gaps rather than a structural
//!   one - a real loading screen with no located plugin XML, and a located
//!   sound bank (`data/audio/sound/frontend.bnk`) whose cues are unread.
//! - [`Title::exhaust`] and [`Title::flare`] are `Unread`. 2048 ships
//!   `data/ribboneffects/EngineTrail_BlueRed_triangle.vex` and a per-team
//!   `Engineflare.vex`, so the *files* are there under HD's own spellings -
//!   but their geometry is in `.rcsmodel` siblings this build cannot decode,
//!   and HD's `EF_Main`/`EF_Boost` group names have not been checked against
//!   2048's own. Naming them here would claim a measurement nobody made.
//! - [`hud::ART`] carries an empty always-on set on the same terms: which
//!   widgets a race actually shows needs a running frame this title has no
//!   capture harness for. The reticle axis (`sights`) and the played skin the
//!   layouts read from are both measured now, off this title's own composed
//!   data and two decompiled race-manager constructors respectively - see
//!   `docs/formats/2048-hud.md`.
//!
//! # The asset tree is Wipeout HD's, and the ship directory is not
//!
//! `.rcsmodel`/`.rcsmaterial`/`.pob`/`.pvs`/`.bnk` - HD's whole extension
//! family is here, and all fourteen HD teams ship verbatim. What moved is
//! *where*: [`race::SHIP_DIR`] is `Data\art\published\hdships`, not the
//! `Data\Ships` Pulse, Pure and HD all spell. That disagreement is what made
//! [`oag_title::RaceDefaults::ship_dir`] an axis.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
//! [ADR-0054]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0054-a-touch-front-end-is-a-second-axis-not-a-menuskin-variant.md

use oag_assets::{Archives, Result};
use oag_title::{ArchiveCandidates, Platform, Title};

pub mod frontend;
pub mod hud;
pub mod race;

/// Wipeout 2048, as the asset layer needs to know it.
pub const TITLE: &Title = &Title {
    name: "Wipeout 2048",
    archives: ArchiveCandidates {
        data: DATA_CANDIDATES,
        // **No companion archive.** The base package ships exactly one PSARC
        // and everything a race reads is in it; the two DLC packages are
        // [`EXTRA_CANDIDATES`], which is the role they fit.
        fe: &[],
        extra: EXTRA_CANDIDATES,
    },
    // **Empty, and not a hole.** A foreign serial is how one title's archive
    // names are told apart from another's, and nothing else in this lineage
    // ships a `PSP2/data.psarc` for these names to collide with. A directory
    // source never reaches the serial check at all - it is read off a disc's
    // `UMD_DATA.BIN`/`SYSTEM.CNF`, and this source is neither.
    foreign_serials: &[],
    // See `frontend::FRONT_END`'s own doc: real and read, `menu: None` on
    // evidence rather than a gap.
    front_end: Some(frontend::FRONT_END),
    hud: hud::LAYOUTS,
    hud_art: hud::ART,
    race: race::DEFAULTS,
    exhaust: &oag_title::exhaust::Exhaust::Unread,
    flare: &oag_title::flare::Flare::Unread,
    plugin_definition: names::TEAM_PLUGIN_DEFINITION,
    track_plugin_definition: Some(names::TRACK_PLUGIN_DEFINITION),
    loading: None,
    music: None,
    // Invented like the other four - and 2048 is the one title where that
    // is worth a second look: its `data/FE/Images/cursor.gxt` turned out to
    // be a 32x16 mark, the same shape as HD's strip underline `cursor.gtf`,
    // not a pointer, and whether PS TV mode draws one at all is unread.
    cursor: include_str!("../../../assets/cursors/2048.svg"),
    // **2048's own tables, beside the HD-derived ones.** The archive carries
    // five `weaponstats*` documents: HD's three (`_Race`, `_Elimination`,
    // `_Detonator`) and two suffixed `_2048`. The suffixed pair is this
    // title's, by the only evidence available - the suffix - and nothing has
    // parsed either.
    weapons: &oag_title::weapons::Weapons {
        race: r"Data\XML\weaponstats_Race_2048.xml",
        elimination: Some(r"Data\XML\weaponstats_Elimination_2048.xml"),
    },
    // Unread, like `exhaust` and `flare` above: 2048 is a Vita/PSP2 asset
    // tree, not Pulse's, so reusing Pulse's `.vex` paths the way `oag_pure`
    // does would be a claim this build has not checked.
    weapon_models: &oag_title::weapons::WeaponModels::EMPTY,
};

/// Paths inside the packages.
pub mod names {
    /// The plugin definition this build reads the roster from.
    ///
    /// **2048 splits into three files where every other title ships one**, and
    /// that is the recovered shape rather than a choice: `Data\Plugins\teams\`,
    /// `Data\Plugins\tracks\` and `Data\Plugins\music\` each hold their own
    /// `Definition.xml`, against the single `Data\Plugins\PI001\Definition.xml`
    /// (both PSP titles) or `Data\Plugins\frontend\definition.xml` (HD) that
    /// carries `PI_Team`, `PI_Track` and `PI_Music` nodes together.
    ///
    /// [`oag_title::Title::plugin_definition`] holds one name, so this holds
    /// the teams one - the roster is what a race needs before it can fly
    /// anything. The circuit list is [`oag_title::Title::track_plugin_definition`],
    /// [`TRACK_PLUGIN_DEFINITION`] below; the soundtrack list has no
    /// equivalent axis yet and this build still sees none, see
    /// `docs/formats/2048-status.md`.
    pub const TEAM_PLUGIN_DEFINITION: &str = r"Data\Plugins\teams\Definition.xml";

    /// The circuits plugin - [`oag_title::Title::track_plugin_definition`],
    /// read where [`oag_title::Title::plugin_definition`] cannot reach it.
    pub const TRACK_PLUGIN_DEFINITION: &str = r"Data\Plugins\tracks\Definition.xml";

    /// The soundtrack plugin, on the same terms as [`TRACK_PLUGIN_DEFINITION`].
    pub const MUSIC_PLUGIN_DEFINITION: &str = r"Data\Plugins\music\Definition.xml";
}

/// The bulk archive: the base package's one PSARC.
///
/// Named by its tail rather than its whole path, the way every candidate is,
/// so a caller may point at `.../PCSF00007`, at `.../PCSF00007/base` or
/// straight at `.../base/PSP2` and get the same answer.
const DATA_CANDIDATES: &[(&str, Platform)] = &[("PSP2/data.psarc", Platform::Vita)];

/// The two downloadable packages, mounted behind the base one.
///
/// `dlc1.psarc` carries five Wipeout HD circuits and `dlc2.psarc` eight more
/// plus the four Zone ones - `docs/formats/track.md` traced all twelve back to
/// HD's own splines, reshipped verbatim. **None of this build's Zone axis
/// reads the Zone four**: [`race::DEFAULTS`] plays Zone on whichever circuit
/// the player picked, base or DLC alike, so mounting these two packages is
/// only ever about the race circuits they add, never about a Zone-specific
/// one.
///
/// **The 1.04 patch's `data1.psarc`/`data2.psarc` are deliberately not here.**
/// A patch archive shadows entries the base also holds, and
/// [`oag_assets::Archives`] searches the bulk archive *first* - so mounting
/// them as extras would put them behind the very entries they exist to
/// replace, which is worse than not mounting them. Doing it properly needs a
/// role that is searched ahead of `data`, and nothing has needed one yet.
///
/// **A circuit both a package and the base ship is served from the base**, by
/// that same ordering: `Vineta_K`, `Anulpha_Pass`, `Chenghou_Project` and
/// `Moa_Therma` are in `data.psarc` as well as in `dlc1.psarc`, and it is the
/// base copy a race gets.
const EXTRA_CANDIDATES: &[(&str, Platform)] = &[
    ("PSP2/dlc1.psarc", Platform::Vita),
    ("PSP2/dlc2.psarc", Platform::Vita),
];

/// Opens whichever packages `source` carries, as Wipeout 2048.
///
/// # Errors
///
/// Propagates [`Archives::open`].
pub fn open(source: &str) -> Result<Archives> {
    Archives::open(source, TITLE)
}
