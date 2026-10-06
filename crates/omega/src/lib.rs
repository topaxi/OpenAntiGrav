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
//! # Deliberately thin, and racing is measured but not finished
//!
//! Like [`oag_2048`], this crate holds only what was actually measured.
//!
//! - **No PS4 emulator capture exists in this project's toolchain**, so
//!   [`frontend::BOOT`]'s provenance is
//!   [`oag_title::Provenance::Declared`], never `Measured` - nobody has
//!   watched an Omega boot the way [ADR-0025] asks for before it upgrades.
//! - **A race starts** - real spline, collision, hull and circuit geometry, see
//!   [`race`]'s module doc and `docs/formats/omega-status.md` - but this crate
//!   is honest about what it has not read (Zone, boost, speed classes, the
//!   sound banks, the `.envsettings` lighting): those fields stay `None`.
//! - **Every archive must be extracted whole.** An earlier extraction of the
//!   package pair lost most of the bytes of most entries to a short read in
//!   the extraction tool; everything on this page's racing side was measured
//!   on the corrected one (`omega-status.md`, "An extraction bug, not this
//!   build's data").
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
    // Unread, on the same terms `oag_2048::TITLE` states: nothing here has
    // looked for an engine exhaust or flare model beside the hull.
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
    // Not read: the loading-screen plugin XML was not opened. `None` is a gap,
    // not a measurement, on Pure's own terms.
    loading: None,
    music: Some(MUSIC),
    cursor: include_str!("../../../assets/cursors/omega.svg"),
    // Unread, on `oag_2048::TITLE`'s own terms: this crate never opened
    // either weapon-stats table this lane, so nothing here can say which of
    // Omega's copies (HD's own two, or a fourth `_2048`-suffixed pair the way
    // 2048 ships) is the one Omega reads from.
    weapons: &oag_title::weapons::Weapons {
        race: r"Data\XML\WeaponStats_Race.xml",
        elimination: Some(r"Data\XML\WeaponStats_Elimination.xml"),
        // `data00.psarc` carries `weaponaistats.xml` and `WeaponAIStats2048.xml`
        // beside the weapon tables (searched 2026-10-03). The unsuffixed one,
        // for the reason the stats above are unsuffixed. Opponents run Pulse's
        // fire law on it: **inherited from Pulse, unmeasured on Omega**.
        ai: Some(r"Data\XML\WeaponAIstats.xml"),
    },
    weapon_models: &oag_title::weapons::WeaponModels::EMPTY,
    // Omega's executable carries HD's weapon-spark and absorb strings, but its
    // wiring is unread, so no trigger draws anything (ADR-0058: `None` is the
    // visible absence). The shield tint is Pulse's, by the rule that an
    // unmeasured title runs Pulse's.
    effects: &oag_title::Effects::NONE,
    looks: &oag_title::Looks::unread(oag_title::ShieldPalettes {
        ps2: oag_title::ShieldPalette::Ps2Pulse,
        elsewhere: oag_title::ShieldPalette::Pulse,
        origin: oag_title::Origin::InheritedFrom("Wipeout Pulse"),
    }),
    // HD's `PI001` front end carried forward (`docs/formats/omega-frontend.md`),
    // read through its own `load_omega`; no grids file is read on its own and no
    // unlock row is wired.
    campaign: &oag_title::Campaign {
        dialect: oag_title::CampaignDialect::Omega,
        definition_entry: None,
        circuit_unlocks: false,
        loyalty_unlocks: false,
        // Not wired, not measured absent: Omega's unlock rows are unread here.
        unlocks_origin: oag_title::Origin::Chosen,
        selection_strings: false,
        origin: oag_title::Origin::InheritedFrom("Wipeout HD"),
    },
    pressings: None,
};

/// Omega's music: no standalone files, a playlist in plugin XML and the audio
/// picked by Wwise state.
///
/// Every name below is hashed by `oag_formats::wwise::name_hash` and matches
/// an id in `Music.bnk` (checked against the bank's own `Music.txt` listing,
/// which names the events, state groups and states). **Race flow, measured in
/// the bank:** `Game_FLOW` `Gameplay` then `Gameplay_FLOW` `Race` select the
/// music switch's race branch. **Front end, the same way:** `Game_FLOW`
/// `Menus` selects the front end's own loop. Which of these the original sets
/// at which moment was not watched; the states' own names are the evidence.
pub const MUSIC: &oag_title::Music = &oag_title::Music {
    front_end: None,
    tracks: None,
    state_tracks: Some(oag_title::StateTracks {
        declared_in: names::MUSIC_PLUGIN_DEFINITION,
        bank: names::MUSIC_BANK,
        media_dir: names::MEDIA_DIR,
        track_group: "Music_Track",
        set_track_event: "Set_Music_Track_{}__frontend",
        play_event: "Play_External_Music",
        none_state: "None",
        race_flow: &[("Game_FLOW", "Gameplay"), ("Gameplay_FLOW", "Race")],
        front_end_flow: &[("Game_FLOW", "Menus")],
    }),
};

/// Paths inside the packages.
pub mod names {
    /// The music bank, inside the patch's `data08.psarc`.
    pub const MUSIC_BANK: &str = "data/audio/sound/Music.bnk";
    /// Where the loose `<media id>.wem` streams are.
    pub const MEDIA_DIR: &str = "data/audio/sound/";
    /// The teams plugin - see [`crate::TITLE::plugin_definition`]'s own doc
    /// for why this title splits into three where HD ships one.
    pub const TEAM_PLUGIN_DEFINITION: &str = r"Data\Plugins\teams\Definition.xml";
    /// The circuits plugin - [`crate::TITLE::track_plugin_definition`].
    pub const TRACK_PLUGIN_DEFINITION: &str = r"Data\Plugins\tracks\Definition.xml";
    /// The soundtrack plugin, [`oag_title::StateTracks::declared_in`].
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
/// settled" section is the record of what would close it;
/// `crates/omega/tests/omega_title_ground_truth.rs`'s
/// `data09_serves_every_front_end_file_this_crate_reads` names which archive
/// actually serves each front-end file this crate reads, as a tripwire
/// rather than a log line - a future capture that settles a different order
/// fails that test by name instead of only disagreeing with a boot report
/// nobody kept.
///
/// **Also mandatory, which is deliberate.** [`oag_assets::source::Layout::resolve_on`]
/// hard-errors when no candidate here is found, and `data09` exists only in
/// the patch - so a base-only `omega-eu` directory with no `omega-eu-patch`
/// sibling refuses to open at all, honestly, rather than opening a base
/// package whose own front end has no `skin.xml` to boot.
///
/// **Carries no circuit.** A circuit is in a base archive (`data02`) with
/// its `.EnvSettings` repacked in the patch's `data08`, so [`EXTRA_CANDIDATES`]
/// puts the patch's other archives ahead of the base - the closest this
/// crate's role slots come to a real overlay.
const DATA_CANDIDATES: &[(&str, Platform)] = &[(archives::DATA09, Platform::Ps4)];

/// The other eight archives, all mounted, searched after [`DATA_CANDIDATES`]:
/// the patch's `data08`, `data07` and `data05` first, then the base package.
///
/// **Patch ahead of base, and that is chosen, not measured** - no PS4 was
/// watched mounting these. What *is* measured is that the choice matters:
/// 10,921 of the 27,077 distinct paths across the nine archives are named by
/// more than one (the patch repacks most of the base), and where the copies
/// differ the patch's is the newer one - 29 of 40 sampled race-relevant
/// collisions (`.EnvSettings`, `.vex`, `.col`, `.final.rcsmodel`,
/// `handlingstats.xml`) differ, every `.EnvSettings` among them is the base's
/// ~4.8 KB against the patch's ~5.6 KB. A base-first order would serve the
/// older copy of every one of those, so the patch goes first, highest number
/// first, the way [`DATA_CANDIDATES`] already puts `data09` ahead of all of
/// them. Order among the three patch archives is unmeasured too.
///
/// **What this order does not change, checked 2026-09-29:** every front-end XML
/// this crate reads is served by `data09` before any of these (see
/// `crates/omega/tests/omega_title_ground_truth.rs`), and the images the boot
/// samples come from these eight archives - so the boot's `image` report was
/// diffed before and after this order on both the short-read and the
/// corrected extraction and is line-for-line identical (14 and 11 lines). What
/// it does change is which copy of a `.EnvSettings` a race reads, and the
/// patch's copy carries no HDR/bloom block the reader recognises where the
/// base's does, so the read bloom chain is off in a race; which copy the
/// game itself reads is the thing nobody has observed.
const EXTRA_CANDIDATES: &[(&str, Platform)] = &[
    (archives::DATA08, Platform::Ps4),
    (archives::DATA07, Platform::Ps4),
    (archives::DATA05, Platform::Ps4),
    (archives::DATA00, Platform::Ps4),
    (archives::DATA01, Platform::Ps4),
    (archives::DATA02, Platform::Ps4),
    (archives::DATA03, Platform::Ps4),
    (archives::DATA04, Platform::Ps4),
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
