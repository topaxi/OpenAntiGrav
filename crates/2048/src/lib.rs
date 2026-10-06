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
//! - [`Title::loading`] is `None` on the same terms `front_end` used to be: a
//!   real gap rather than a structural one - a real loading screen with no
//!   located plugin XML. [`Title::music`] is `Some` since 2026-09-25:
//!   `data/audio/sound/frontend.bnk`, the bank this thread's own evidence
//!   first pointed at, turned out to carry no name table at all (so it
//!   cannot be what a cue name addresses); the front end's music is instead
//!   a standalone RIFF-wrapped ATRAC9 file, `FEMusic/frontend_stereo.at9`,
//!   named the same way Pulse's and HD's front-end tracks are - see
//!   [`names::FRONT_END_MUSIC`] and `docs/formats/2048-frontend.md`.
//! - [`Title::exhaust`] and [`Title::flare`] are read since 2026-10-05, on HD's
//!   model: 2048 ships `data/ribboneffects/EngineTrail_BlueRed_triangle.vex`
//!   and a per-team `EngineFlare.vex`, each with a psp2 `.rcsmodel` sibling
//!   that decodes. The executable names the same three literals HD's does
//!   (`data/RibbonEffects/enginetrail_bluered_triangle.vex`,
//!   `%s\engineflare.vex`, `data/Tex/EngineFlare/Engine_Flare_Rich.gxt`) and
//!   all 72 flare trees hang the same `EF_Main`/`EF_Boost` groups. See
//!   `docs/formats/2048-status.md`.
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

pub mod campaign;
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
    // **`bluered`, because the shipped executable names it** - the literal
    // `data/RibbonEffects/enginetrail_bluered_triangle.vex` - exactly as on HD.
    // The `.rcsmodel` beside it is the 2048 container; the loader reads it
    // through `oag_rcs::rcsmodel::psp2` and takes HD's blend pair for the
    // same-named material (inherited, not measured on 2048).
    exhaust: &oag_title::exhaust::Exhaust::Authored(
        "/data/ribboneffects/enginetrail_bluered_triangle.rcsmodel",
    ),
    // **Per team, on HD's shape**: `EngineFlare.vex` beside each hull and the
    // two groups its tree hangs, `EF_Main` and `EF_Boost`, read off every
    // shipped file. The sprite literal is the executable's own.
    flare: &oag_title::flare::Flare::PerTeam(oag_title::flare::Authored {
        stem: "engineflare",
        always: "EF_Main",
        boost: "EF_Boost",
        sprite: "/data/tex/engineflare/engine_flare_rich.gxt",
        // **Off, chosen, not measured.** See `Authored::engine_light`.
        engine_light: false,
    }),
    // No prompt glyph of this title's has been read; the disc's own draw unchanged.
    prompts: &oag_title::prompts::Prompts::UNREAD,
    plugin_definition: names::TEAM_PLUGIN_DEFINITION,
    track_plugin_definition: Some(names::TRACK_PLUGIN_DEFINITION),
    loading: None,
    music: Some(MUSIC),
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
        // `WeaponAIStats.xml` and `WeaponAIStats2048.xml` both ship and are
        // byte-identical (searched 2026-10-03); the suffixed one follows the
        // two tables above. Opponents run Pulse's fire law on it: **inherited
        // from Pulse, unmeasured on 2048**.
        ai: Some(r"Data\XML\WeaponAIStats2048.xml"),
    },
    // Unread, like `exhaust` and `flare` above: 2048 is a Vita/PSP2 asset
    // tree, not Pulse's, so reusing Pulse's `.vex` paths the way `oag_pure`
    // does would be a claim this build has not checked.
    // Only the magstrip effect is read, and it is the `.pob` one: every 2048
    // event carries a CRC-id mode (`>= 0x17`), the side of
    // `GameMode_IsHdLineage` (`0x81000930`) that plays `WO_MAGSTRIP_SPARKS` /
    // `WO_MAGSTRIP_ZONE` and builds no `MagstripWake` (live read of `0x8153fd24`
    // on Vita3K, 2026-10-05). Every other model stays unread. See
    // `docs/ghidra/functions/vita-2048-eu-v104/ships-effects.md`.
    weapon_models: &oag_title::weapons::WeaponModels {
        magstrip_pob: true,
        ..oag_title::weapons::WeaponModels::EMPTY
    },
    // The four triggers a title answers for itself are unread on 2048, so none
    // draws (ADR-0058). The engine's own names are tried as Pulse's, by the
    // rule that an unmeasured title runs Pulse's. The shield tint likewise.
    effects: &EFFECTS,
    looks: &oag_title::Looks::unread(oag_title::ShieldPalettes {
        ps2: oag_title::ShieldPalette::Ps2Pulse,
        elsewhere: oag_title::ShieldPalette::Pulse,
        origin: oag_title::Origin::InheritedFrom("Wipeout Pulse"),
    }),
    // No campaign layout read (2048's campaign is its own event map): the
    // load path that reaches this falls through to Pulse's reader and refuses.
    campaign: &oag_title::Campaign {
        dialect: oag_title::CampaignDialect::Pulse,
        definition_entry: None,
        circuit_unlocks: false,
        loyalty_unlocks: false,
        // Not wired, not measured absent: 2048's unlock rows are unread here.
        unlocks_origin: oag_title::Origin::Chosen,
        selection_strings: false,
        origin: oag_title::Origin::InheritedFrom("Wipeout Pulse"),
    },
    pressings: None,
};

/// The effects Wipeout 2048 plays: the engine's own names, tried as Pulse's, minus
/// the ones **Wipeout 2048 never authors**.
///
/// `WO_PLASMA_FLASH`, `WO_SHIP_ENGINEFLARE`, `WO_LEACHBEAM_ENERGY`,
/// `WO_BLUE_WELDER`, `WO_RAIN`, `WO_RAIN_LENS` and `WO_SNOW` are in no archive
/// of this title (every `.pob` of `data.psarc`, both `data1`/`data2` patch archives and both DLC packs, both regions listed, 2026-10-06) and no string of
/// its executable names them (`grep -a` over the v1.04 and base `eboot.elf`: 0 hits each, against 2
/// for `WO_ROCKET_FLARE`). They are Pulse's, so each loads as a report line
/// and nothing else; leaving them out is the title's own effect set, not a
/// hidden absence. See `docs/formats/pob.md`, "Effects Pulse names that
/// Wipeout 2048 never authors".
const EFFECTS: oag_title::Effects = {
    use oag_title::Trigger;
    let mut effects = oag_title::Effects::engine(oag_title::Origin::InheritedFrom("Wipeout Pulse"))
        .without(Trigger::PlasmaBlast)
        .without(Trigger::EngineFlare)
        .without(Trigger::LeachbeamEnergy);
    effects.scenery = &[oag_title::engine_effects::MODESTO_STEAM_EFFECT];
    effects
};

/// 2048's music: the front end's loop and the eleven race tracks.
///
/// The soundtrack is declared in [`names::MUSIC_PLUGIN_DEFINITION`], eleven
/// `PI_Music` nodes whose `location` is a directory holding
/// [`names::MUSIC_TRACK_FILE`] (read 2026-10-05, `data/audio/music/01` to
/// `11`). Each track carries an `.fft` sidecar beside it, which nothing here
/// reads.
///
/// [`oag_title::Music::front_end`] is [`names::FRONT_END_MUSIC`] - see that
/// constant's own doc for the evidence and for why it names a standalone
/// file rather than a `frontend.bnk` cue.
pub const MUSIC: &oag_title::Music = &oag_title::Music {
    front_end: Some(names::FRONT_END_MUSIC),
    tracks: Some(oag_title::DeclaredTracks {
        declared_in: names::MUSIC_PLUGIN_DEFINITION,
        file: names::MUSIC_TRACK_FILE,
    }),
    state_tracks: None,
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
    /// [`TRACK_PLUGIN_DEFINITION`] below; the soundtrack list is
    /// [`MUSIC_PLUGIN_DEFINITION`] below, reached through [`crate::MUSIC`]'s
    /// `tracks`, see `docs/formats/2048-status.md`.
    pub const TEAM_PLUGIN_DEFINITION: &str = r"Data\Plugins\teams\Definition.xml";

    /// The circuits plugin - [`oag_title::Title::track_plugin_definition`],
    /// read where [`oag_title::Title::plugin_definition`] cannot reach it.
    pub const TRACK_PLUGIN_DEFINITION: &str = r"Data\Plugins\tracks\Definition.xml";

    /// The soundtrack plugin, on the same terms as [`TRACK_PLUGIN_DEFINITION`].
    pub const MUSIC_PLUGIN_DEFINITION: &str = r"Data\Plugins\music\Definition.xml";

    /// The file a `PI_Music` location holds: RIFF-wrapped ATRAC9, stereo.
    /// Where Pure's is `music.at3` and HD's `music_stereo.mp3`.
    pub const MUSIC_TRACK_FILE: &str = "music_stereo.at9";

    /// The music the front end loops under its menus.
    ///
    /// **A standalone file, not a `frontend.bnk` cue** - the bank this
    /// thread's own Next Step originally pointed at is a version-5 bank whose
    /// name table is keyed by FNV-1 hashes, not the 16-byte names
    /// `oag_formats::sblk::Bank::sound_names` walks (it returns empty for it;
    /// `Bank::cue_named` resolves a name by hash, which was found later). The
    /// base package's
    /// `PSP2/data.psarc` instead carries
    /// `data/audio/music/FEMusic/frontend_stereo.at9` - RIFF-wrapped ATRAC9,
    /// 48000 Hz stereo, a 302-second `fact` chunk - beside a second,
    /// distinctly-named `data/audio/music/FEDemoMusic/frontend_stereo.at9`
    /// for the attract-mode demo screen. The `FEMusic` folder and `frontend`
    /// stem are exactly the shape Pulse's `Data\Music\FEMusic\frontend1.at3`
    /// and HD's `Data\Music\FEMusic\frontend1_stereo.mp3` already use, so the
    /// match is a naming-convention read across three titles rather than a
    /// guess picked by listening - **confidence 70**: unambiguous by
    /// placement and naming, not confirmed against a decompile the way
    /// Pulse's own template expansion is (see `oag_pulse`'s
    /// `FRONT_END_MUSIC`), because no `SoundManager`-equivalent construction
    /// site has been traced in 2048's executable yet. Read 2026-09-25; see
    /// `docs/formats/2048-frontend.md`.
    pub const FRONT_END_MUSIC: &str = r"Data\Audio\Music\FEMusic\frontend_stereo.at9";
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
