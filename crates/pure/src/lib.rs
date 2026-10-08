//! What Wipeout Pure ships.
//!
//! A title package in the sense of [ADR-0022], and the one that proves the seam
//! is real rather than decorative: if `oag-assets` can open Pure's disc through
//! the same mechanism it opens Pulse's, with nothing but a different table, then
//! title is a data axis.
//!
//! # Deliberately thin
//!
//! This carries only what [`pure-status.md`] measured. In particular it does
//! **not** restate Pure's `.vex` class ids: those are keyed off each file's own
//! version word inside `oag-formats`, because a class-id table is a property of
//! a format version rather than of a release - which the Pulse disc's own
//! version-4 `Data\Defaults\Skycube.vex` settles.
//!
//! Nothing about Pure's collision classes or its HUD is here, because none of
//! it is recovered. An empty module is the honest record of that; a plausible
//! guess would not be.
//!
//! Its **music** is the one axis that has since been recovered, and by
//! measurement rather than by analogy: [`MUSIC`] carries a front-end path read
//! out of `BOOT.BIN` and a soundtrack the disc's own plugin definition
//! declares. Its **sound banks** are located but not decoded -
//! `Data\Sound\frontend.bnk`, `hud.bnk` and `generaltrack.bnk` are named in the
//! same strings dump and present on both pressings - so they are recorded in
//! `docs/formats/pure-status.md` and deliberately absent here: a constant
//! nothing can read is a constant nothing can be wrong about, but it is also
//! not a recovery.
//!
//! [ADR-0022]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/architecture/adr/0022-title-packages.md
//! [`pure-status.md`]: https://github.com/topaxi/OpenAntiGrav/blob/main/docs/formats/pure-status.md

use oag_assets::{Archives, Result};
use oag_title::{ArchiveCandidates, ForeignSerial, Platform, Title};

/// Wipeout Pure, as the asset layer needs to know it.
pub const TITLE: &Title = &Title {
    name: "Wipeout Pure",
    archives: ArchiveCandidates {
        patch: &[],
        data: DATA_CANDIDATES,
        fe: FE_CANDIDATES,
        // Both PSP titles ship exactly two archives anything reads. See
        // `ArchiveCandidates::extra` for the release that does not.
        extra: &[],
    },
    foreign_serials: FOREIGN_SERIALS,
    front_end: Some(FRONT_END),
    hud: hud::LAYOUTS,
    hud_art: hud::ART,
    race: race::DEFAULTS,
    // The same literal Pulse's executable carries, and Pure's disc answers it
    // - measured, not assumed: a Pure race reports `64x64 .mip, 4 mip
    // level(s)` for it exactly as Pulse does.
    exhaust: &oag_title::exhaust::Exhaust::Named(r"Data\Tex\engineFlare\Engine_noise.mip"),
    // The same sprite Pulse names, and found under the same name on Pure's
    // disc - see `oag_pulse`'s note for the literal it was read from.
    flare: &oag_title::flare::Flare::Sprite(r"Data\Tex\EngineFlare\grabbedEngineFlare128x64x8.mip"),
    // No prompt glyph of this title's has been read; the disc's own draw unchanged.
    prompts: &oag_title::prompts::Prompts::UNREAD,
    plugin_definition: names::GAME_PLUGIN_DEFINITION,
    track_plugin_definition: None,
    // **`None`, and it is a measurement.** Neither
    // `Data\Plugins\loading\Definition.xml` nor
    // `Data\Defaults\Loading\LoadingPulseOverlay.mip` is in any of Pure's
    // three archives, checked 2026-08-12 - so this title authors no loading
    // screen of its own and the screen draws this build's own counts over a
    // cleared frame. See `oag_title::Loading`.
    loading: None,
    music: Some(MUSIC),
    cursor: include_str!("../../../assets/cursors/pure.svg"),
    // **One table, all lower case, and no Eliminator variant.**
    // `Data\XML\weaponstats.xml` at `0x08a445a0` in `/psp-pure-usa/BOOT.BIN`,
    // three strings before the `"WeaponStats"` and `"Weapon"` element names the
    // parser matches. Pure ships no `WeaponStats_Elimination.xml` under any
    // spelling - see `oag_title::weapons::Weapons::elimination`.
    weapons: &oag_title::weapons::Weapons {
        race: r"Data\XML\weaponstats.xml",
        elimination: None,
        ai: Some(r"Data\XML\WeaponAIstats.xml"),
    },
    // Pure's own weapon models are unmeasured. **Restated, not imported** -
    // a title package does not depend on another title package outside
    // `[dev-dependencies]` (`oag-pulse` is one here, for the cross-checks in
    // this file's own tests, and ADR-0022 is why runtime code does not reach
    // for it) - so these are Pulse's exact literals copied rather than a
    // cross-title reference, the same restatement `oag_tables::handling`
    // already does for `oag_title::race::SHIP_DIR`. See `oag_pulse::race`'s
    // own `WEAPON_MODELS` for the evidence behind each one.
    weapon_models: &oag_title::weapons::WeaponModels {
        rocket: Some(r"Data\Weapons\Rocket.vex"),
        mine: Some(r"Data\Weapons\Pulse_Mine.vex"),
        bomb: Some(r"Data\Weapons\Pulse_Bomb.vex"),
        cannon: Some(r"Data\Weapons\pulse_muzzleflash.vex"),
        plasma_ball: None,
        plasma_blast_pulse: Some(oag_title::weapons::PulsePlasmaBlast {
            halo: r"Data\Weapons\pulse_plasma_halo1.vex",
            hemisphere2: r"Data\Weapons\pulse_plasma_hemisphere2.vex",
            hemisphere1: r"Data\Weapons\pulse_plasma_hemisphere1.vex",
        }),
        plasma_blast_hd: None,
        // Pure's Bomb authors no `timetodie` at all and never detonates -
        // see `oag_tables::weapons::BombStats::timetodie`.
        bomb_blast_pulse: None,
        bomb_blast_hd: None,
        missile_blast_hd: None,
        repulser_field: None,
        // Pure ships no magstrip and no magfloor effect: its executable has no
        // `visual_effects` or `MagEffect` string and no `Mag Floor Collision`
        // class (docs/ghidra/functions/psp-pure-usa/magfloor-absent.md).
        mag_floor: None,
        magstrip_wake: None,
        magstrip_pob: false,
        rocket_trail: None,
        leachbeam_ball: None,
        shuriken: None,
        // Pure's own Cannon draw is unread; it keeps Pulse's terms.
        cannon_look: None,
        // Pure's BOOT.BIN names `staticglow` (grep, both pressings).
        ghost_static: Some(oag_title::weapons::GhostStatic {
            entry: r"Data\Tex\staticglow.mip",
            origin: oag_title::Origin::Measured,
        }),
    },
    effects: effects::EFFECTS,
    looks: effects::LOOKS,
    // No campaign layout read (`docs/formats/pure-status.md`): the load path
    // that reaches this falls through to Pulse's reader and refuses.
    campaign: &oag_title::Campaign {
        dialect: oag_title::CampaignDialect::Pulse,
        definition_entry: None,
        circuit_unlocks: false,
        loyalty_unlocks: false,
        // Pure authors no circuit or variant `<Unlock>` (`docs/formats/pure-status.md`).
        unlocks_origin: oag_title::Origin::Measured,
        selection_strings: false,
        screen_archive: None,
        origin: oag_title::Origin::InheritedFrom("Wipeout Pulse"),
    },
    pressings: Some(frontend::PRESSINGS),
    pre_race: None,
    // None, checked: Pure ships `trackstartup.xml` (five colour slots on
    // `01_Vineta_K`) and its own `PI004` catalogue, but the catalogue's adverts are
    // `3DVexWindow`, `2DVexWindow` and `MusicLicensed` models that carry no `Camera`
    // node (four of four read), so Pulse's camera-into-a-target pass has nothing to
    // draw them with. A different mechanism, not read.
    adverts: None,
};

/// Where Pure keeps its music.
///
/// **Both halves come off the disc itself**, and neither is Pulse's shape.
///
/// The front-end path is a literal in `BOOT.BIN`, sitting between
/// `c:/Work/Wipeout/Code/System/Sound/MusicManager.cpp` and the sound-bank
/// names that follow it. Pulse expands a `frontend%d.at3` template there; Pure
/// names one file and has no such template, and none of Pulse's eight
/// expansions hashes to an entry on either Pure pressing.
///
/// The soundtrack tracks are `music.at3` inside the directory each `PI_Music`
/// node of [`names::GAME_PLUGIN_DEFINITION`] declares - `MusicManager.cpp`'s
/// other two strings, `%s\%s` and `music.at3`, joined. All **nineteen**
/// resolve to real entries on both `pure-psp-usa.chd` and `pure-psp-eu.chd`.
/// Confidence **90**: the path template and the declaration are each read off
/// the disc and every expansion hits, but nothing has been watched running
/// under an emulator. See `docs/formats/pure-status.md`.
pub const MUSIC: &oag_title::Music = &oag_title::Music {
    front_end: Some(names::FRONT_END_MUSIC),
    tracks: Some(oag_title::DeclaredTracks {
        declared_in: names::GAME_PLUGIN_DEFINITION,
        file: names::MUSIC_TRACK_FILE,
    }),
    state_tracks: None,
};

/// Pure's front end: the layout its own `Skin.xml` authors and the boot chain a
/// cold boot was measured to walk.
///
/// A separate measurement from Pulse's in both halves - the two skins agree on
/// nothing they share and the two chains differ in length - which is the
/// evidence [`oag_title::FrontEnd`] exists on.
pub const FRONT_END: &oag_title::FrontEnd = &oag_title::FrontEnd {
    root: names::FRONTEND_ROOT,
    // **Not the same five ids Pulse uses - only French/German/Spanish/Italian
    // are.** The old list here was Pulse's USA set copied wholesale, on the
    // unchecked assumption that `PI012` is Pure's English the way it is
    // Pulse USA's. Read directly off both Pure pressings (`oag-wad cat` on
    // every `Data\Plugins\PI0NN\Definition.xml`), `PI012` on Pure carries no
    // `<Font>` block and no self-naming `<Entry ID="English">` at all - it is
    // a small, unlocalised American-spelling patch (its own comment: "US-
    // English ie English with different spellings"), the same shape as
    // `PI003`'s Japanese button-glyph patch. Pure's real English is `PI000`:
    // eight `<Font Language="English">` blocks naming exactly the files
    // `oag_game::language`'s role table already credits to "Pure" (
    // `FX300ANG.fnt`, `HUDFont.fnt`, `LTe50325.fnt`...), plus a self-naming
    // `<Entry ID="English" String="English">`. See
    // `docs/formats/pure-status.md#the-language-plugin-id-space-is-pures-own-not-pulses`.
    // Confidence 90.
    // The EU manifest's languages, which is also what an unlisted serial gets;
    // `language_manifests` carries each release's own list.
    language_plugins: &["PI000", "PI010", "PI008", "PI009", "PI011"],
    disc_strings: Some("pure"),
    language_manifests: LANGUAGE_MANIFESTS,
    assumed_release: None,
    menu: Some(frontend::MENU_SKIN),
    menu_ps2: None,
    // Pure authors the same `FEGlobals`/`<Menu>` vocabulary Pulse does, not
    // the touch-icon idiom - see ADR-0054.
    touch: None,
    boot: frontend::BOOT_PROFILE,
    // Pure authors a frame on `FE Screen` too - rule lines, scroll arrows, a
    // squiggle-text date strip - the same idiom as Pulse's and HD's, once
    // `oag_game::screen`'s widget collection recurses into
    // `<BackgroundController>` the way it does `<Viewport>`/`<Animation>`.
    // Neither of its two images names a `src` on either pressing.
    // `BackgroundTopRightImage` is filled by `hashes::MENU_TOPRIGHT_LOGO`
    // through `frontend::FALLBACK_IMAGES`, the same content-scan mechanism
    // `TitleFrame` uses. `BackgroundImage` - the full-screen backdrop - stays
    // unfilled: the same scan that found the other two checked every
    // `480x272`/`512x256`/`512x512`-shaped candidate against a real `Main
    // Menu` capture and none matched it, because the real capture's
    // background is genuinely flat white with no picture at all. See
    // `frontend::states::FE_SCREEN`'s own doc and
    // `docs/formats/pure-status.md`.
    menu_frame: Some(frontend::states::FE_SCREEN),
    bottom_up_gnf: &[],
    // Pure's race box is a chain of screens (`Class Selection`, `League
    // Selection`, `Tournament Selection`, `Track Selection`,
    // `Team Selection`, ...) authored in one file, read 2026-09-10 - see
    // `docs/formats/race-setup.md`'s Pure section. Only `Track Selection`
    // and `Team Selection` map onto `oag_ui_screens::picker::Kind` and are wired as
    // pickers here, the same two screens Pulse's own race box authors;
    // `Class Selection`/`League Selection`/`Tournament Selection` are not -
    // this build's RACE page settles mode/class the way it already does on
    // Pulse.
    race_box: Some(names::RACE_BOX_DEFINITION),
    team_select: None,
    track_select: None,
    race_setup: None,
    // Pure ships neither `FE\forward.vex` nor `ship_FE.vex`; both screens
    // preview with the stills the entry's own `screen.xml` authors.
    preview_meshes: false,
    // Not checked this pass - a gap, not a measurement that Pure ships no
    // such screen. See `oag_title::FrontEnd::endrace_entry`.
    endrace_entry: None,
    endrace_style: None,
};

pub mod effects;
pub mod frontend;
pub mod hud;
pub mod race;

/// Name hashes for `Data.wad` entries whose names are not recovered.
///
/// A WAD directory stores only the hash of each name, so an entry nobody has
/// named is still perfectly addressable. Recording the hash is what keeps such
/// an entry usable without inventing a name for it - which the naming rules in
/// `CLAUDE.md` forbid below 50 confidence. Mirrors `oag_pulse::hashes`, which
/// this module's own doc comment there explains at more length.
pub mod hashes {
    /// `Title Screen->TitleFrame`'s own wordmark texture **on the USA
    /// pressing**: orange "wipEout" over a white-outlined "pure", `512x128`,
    /// 8bpp indexed.
    ///
    /// **`TitleFrame` names no `src` in `Skin.xml` at all** - `<?This is the
    /// Title screen backdrop?> <Image name="TitleFrame" StartEnabled="false">
    /// <Values width="480" x="0" y="76" height="128" TxtrWidth="480"
    /// TxtrHeight="128">`. **The runtime mechanism is now read, not just
    /// content-matched**: `Screen_ConstructTitleScreen`
    /// (`docs/ghidra/functions/psp-pure-eu/title-screen.md`) finds the
    /// `TitleFrame` element by path and loads a texture by a literal,
    /// **region-suffixed** name it builds at runtime by concatenating
    /// `Data\FE\Images\FMV_last_frame` with a per-pressing suffix baked into
    /// that pressing's own executable: `_EU`, `_US` or `_JAP`. A breakpoint
    /// on that function fires on a real `pure-psp-eu.chd` boot under
    /// PPSSPP, confirming the call is live rather than dead code.
    ///
    /// **This constant is the USA pressing's own value**, `Data.wad` entry
    /// 537 (`Data\FE\Images\FMV_last_frame_US.mip`, hashed with
    /// [`oag_formats::wad::hash_name`]) - confirmed against a live PPSSPP
    /// capture of `pure-psp-usa.chd`'s `Title Screen`: orange "wipEout" over a
    /// white-outlined "pure". See [`TITLE_LOGO_EU`] for the EU pressing's own,
    /// **different**, value - the two are not interchangeable, and using this
    /// one unconditionally is the bug `oag_pure::frontend::title_frame_src`
    /// exists to fix.
    ///
    /// **`TxtrWidth="480"` against a `512`-wide physical texture is a
    /// corroborating measurement, not a coincidence.** PSP textures are
    /// commonly padded to a power of two; the widget's own declared sample
    /// width crops the rightmost 32 columns, which are opaque white and carry
    /// no ink - trimming the decoded picture to its own opaque content lands a
    /// `335x80` box entirely inside the `480`-wide crop, with room to spare on
    /// both sides. Applies equally to the EU value.
    ///
    /// Confidence **95**: the literal name is read directly out of the USA
    /// executable, its hash lands exactly on this pre-existing content-scan
    /// find, the construction call is confirmed live under a real emulator,
    /// and the resulting picture matches a real captured frame of this exact
    /// pressing. See `docs/formats/pure-status.md`'s "The Title screen
    /// wordmark" section.
    pub const TITLE_LOGO: u32 = 0x3af1_8d90;

    /// `Title Screen->TitleFrame`'s own wordmark texture **on the EU
    /// pressing**: blue "wipEout" over orange "pure" - a different colourway
    /// from [`TITLE_LOGO`]'s USA one, `512x128`, 8bpp indexed, `Data.wad`
    /// entry 535 (`Data\FE\Images\FMV_last_frame_EU.mip`).
    ///
    /// Same mechanism, same confidence basis and the same **95** as
    /// [`TITLE_LOGO`] - see that constant's own doc comment for the full
    /// chain. Confirmed against a live PPSSPP capture of `pure-psp-eu.chd`'s
    /// `Title Screen`, breakpointed at the construction call: blue "wipEout"
    /// over orange "pure", not the USA pressing's orange-over-white.
    ///
    /// A third region variant exists on the disc but is not wired anywhere -
    /// `Data.wad` entry 536 (`3f313472`, byte-identical in content to this
    /// one), whose hash matches `Data\FE\Images\FMV_last_frame_JAP.mip`. No
    /// disc this project has names a Japanese-region SKU, so which pressing's
    /// executable actually names that suffix is unread.
    pub const TITLE_LOGO_EU: u32 = 0xb667_7aab;

    /// `FE Screen->BackgroundTopRightImage`'s own texture: the "ワイプアウト"
    /// katakana wordmark beside the swoosh/arrow logo, `256x32`, 8bpp indexed.
    ///
    /// **Names no `src` either** - `<Image name="BackgroundTopRightImage">
    /// <Values x="252" y="3" width="256" height="32" U="0" V="0"
    /// TxtrWidth="256" TxtrHeight="32">`, nested in the same
    /// `<BackgroundController>` as `TITLE_LOGO`'s sibling gap,
    /// `BackgroundImage`.
    ///
    /// **The runtime mechanism is now read.** Unlike `TitleFrame`,
    /// `BackgroundTopRightImage`'s texture is not a literal baked into the
    /// executable: `BackgroundController_UpdateImages`
    /// (`docs/ghidra/functions/psp-pure-eu/title-screen.md`) resolves it every
    /// frame through a declared `FEGlobals->BackgroundTopRightTexture` global,
    /// the same indirection `FrameLineColor`/`TitleColor` already go through
    /// for colours - just carrying a WAD path string instead of an ARGB one.
    /// `Data\Skins\Default\Skin.xml` declares it as
    /// `Data\Skins\Default\Images\default_texture.mip`, which hashes to
    /// exactly this constant.
    ///
    /// This constant is kept only as the evidence trail for that hash; the
    /// front end itself now resolves the global directly (see
    /// [`crate::frontend::FALLBACK_IMAGES`]) rather than hard-coding it,
    /// since the value is re-derivable from the skin's own declared string
    /// and hand-transcribing a re-derivable value is exactly what
    /// `CLAUDE.md`'s "never invent what the assets already author" section
    /// warns against.
    ///
    /// **Entry 27 of `Data.wad`, `Data.wad`-only** - not in `FE.wad` or
    /// `FEData.wad`, checked by hash across all three. `9,232` bytes matches
    /// `256*32 + 256*4` palette `+ 16` header exactly, with no padding to
    /// account for - `256x32` is already a power of two on both axes, unlike
    /// `TitleFrame`'s `512`-wide crop of a `480`-wide widget.
    ///
    /// Byte-identical on both pressings (`pure-psp-usa.chd` and
    /// `pure-psp-eu.chd`), at the same hash - consistent with the same global
    /// declaration on both, since `Data\Skins\Default\Skin.xml` is identical
    /// on both pressings. Confidence **95**: an exact visual match to a real
    /// captured frame, agreement across both pressings, and now the exact
    /// declared global and its literal value read directly.
    pub const MENU_TOPRIGHT_LOGO: u32 = 0x7ba7_8aca;
}

/// The archives a PSP Pure disc ships, relative to the image root.
///
/// **Three, not four.** `Data.wad`, `FE.wad` and `FEData.wad` are all present
/// under `PSP_GAME/USRDIR/`; Pure has no `BEData.wad`. All 1,229 entries across
/// the three decode to their declared size, so the container itself is
/// unchanged from Pulse's - it is the payloads' class numbering that differs.
pub mod archives {
    /// Front-end fonts and shared images.
    pub const FE: &str = "PSP_GAME/USRDIR/FE.wad";
    /// Per-ship and per-track front-end screens.
    pub const FEDATA: &str = "PSP_GAME/USRDIR/FEData.wad";
    /// Everything else: tracks, ships, handling, plugins.
    pub const DATA: &str = "PSP_GAME/USRDIR/Data.wad";
}

/// Entry names inside `Data.wad`.
///
/// The WAD name hash carries over from Pulse with no salt or seed change, so a
/// Pulse-shaped path hits Pure's directory when Pure really has that entry.
/// These are the ones confirmed present.
pub mod names {
    /// The front-end root: every boot screen, the `FEGlobals` variables, and
    /// the `LoadXML` list that pulls in the rest of the menus.
    ///
    /// **The same path Pulse uses, and written out here rather than borrowed.**
    /// Both PSP titles keep their front end in the numbered plugin `PI001`, and
    /// on a two-title corpus that made one constant in `oag-pulse` the obvious
    /// home for it. Wipeout HD names its plugins instead, so the path became a
    /// per-title axis ([`oag_title::FrontEnd::root`]) and every title now states
    /// its own - including the two that agree, since "these two happen to
    /// match" is a measurement worth being able to see rather than an
    /// arrangement to depend on.
    pub const FRONTEND_ROOT: &str = r"Data\Plugins\PI001\GUI\Skin.xml";

    /// The race box's own definition: `Class Selection`, `League Selection`,
    /// `Tournament Selection`, `Track Selection`, `Zone Track Selection` and
    /// `Team Selection`, all in one file.
    ///
    /// **The same relative path Pulse's own race box uses**, read directly off
    /// `pure-psp-eu.chd` (`Data\Plugins\PI001\GUI\Skin.xml`'s own `LoadXML`
    /// list names it, not a localised entry) - confirmed 2026-09-10, not
    /// assumed from the name matching. See `docs/formats/race-setup.md`'s
    /// "Pure differs in shape, not only in value" section for what the file
    /// holds and [`oag_title::FrontEnd::race_box`] for why the path lives here
    /// rather than being found through the full `LoadXML` list at boot.
    pub const RACE_BOX_DEFINITION: &str = r"Data\Plugins\PI001\GUI\Selection_Definition.xml";

    /// The game plugin's own definition.
    ///
    /// Carries Pure's `PI_Music` declarations as well as its circuits and
    /// teams: one node per soundtrack track, each naming the directory the
    /// track lives in. See [`crate::MUSIC`].
    pub const GAME_PLUGIN_DEFINITION: &str = r"Data\Plugins\PI001\Definition.xml";

    /// The music the front end loops under its menus.
    ///
    /// A literal in `BOOT.BIN`, not a template - see [`crate::MUSIC`]. Present
    /// on both pressings.
    pub const FRONT_END_MUSIC: &str = r"Data\Music\frontend.at3";

    /// The file a `PI_Music` location holds, joined onto that location to
    /// address one soundtrack track. See [`crate::MUSIC`].
    pub const MUSIC_TRACK_FILE: &str = "music.at3";

    /// One team's `handlingstats.xml`.
    ///
    /// **Eleven** ship directories carry one to Pulse's eight, and ten of the
    /// eleven author **five** speed classes to Pulse's four. The eleventh,
    /// `Zone_01`, authors no `<Class>` block at all.
    ///
    /// The roster is still not a constant here, deliberately: it is declared by
    /// `Data\Plugins\PI001\Definition.xml` on the disc itself, so a caller that
    /// reads it gets the list the player's own pressing ships rather than one
    /// this crate remembered. `crates/pure/tests/handling_schema_ground_truth.rs`
    /// does exactly that.
    #[must_use]
    pub fn handling_stats(team: &str) -> String {
        format!(r"Data\Ships\{team}\handlingstats.xml")
    }

    /// The boot movie the `IntroMovie1` widget on `Intro Screen` plays.
    ///
    /// The widget's `src` is `Data\Movies\IntroMovieP1`, `localised="true"` -
    /// `oag_game::screen::Movie::entry_name` appends `_US.PMF` for a localised
    /// source with no extension of its own, per its own doc comment. Hashing
    /// that gives `3d2c85f8`, a real entry in `pure-psp-usa.chd`'s `Data.wad` -
    /// but that is not independent confirmation, since a wrong guess can still
    /// land on a real entry by accident. **Checked properly**: extracted and
    /// decoded (`ffmpeg` on the cached `.ivf`), frames 150 and 230 read
    /// "SONY COMPUTER ENTERTAINMENT AMERICA" and "A STUDIO LIVERPOOL GAME" -
    /// this is `oag_pulse`'s own `DEVPUB_REEL` (`hash:b1ba72c3` is the EU cut,
    /// `hash:3d2c85f8` the American one both share), not new content. Pure's
    /// disc genuinely opens on this reel; Pulse ships the identical asset
    /// (`oag_pulse::DEVPUB_REEL`'s own doc comment already says the two discs
    /// carry it byte-identically) but no longer plays it at boot.
    ///
    /// **It plays on `Developer Publisher Screen`**, the step straight after the
    /// picker - inherited from the parent `Intro Screen` that declares the widget.
    /// Confirmed by matching captured frames against the decoded video: frame 144
    /// is the "PRESENTS" card and frame 231 the "A STUDIO LIVERPOOL GAME" card,
    /// each within resampling noise. See
    /// [`crate::frontend::states::DEVELOPER_PUBLISHER`].
    ///
    /// **This name is one of four** - see [`INTRO_MOVIE_CUTS`] for the other
    /// three. Picking this one unconditionally was a known bug (a European
    /// pressing shown the American card); the fix is [`intro_movie`], which
    /// [`oag_game::boot::load_shell`] calls with
    /// [`crate::frontend::localised_movie_region`] of the source's own serial.
    /// `Movie_ParseAttributes` (`docs/ghidra/functions/psp-pure-eu/
    /// movie-localised-suffix.md`) settles that each pressing bakes exactly
    /// one such suffix into its own executable, the same shape of fix
    /// `TitleFrame`'s wordmark already needed.
    pub const INTRO_MOVIE: &str = r"Data\Movies\IntroMovieP1_US.PMF";

    /// [`INTRO_MOVIE_CUTS`]'s entry for `region` (`"EU"`/`"US"`/`"JAP"`/`"KO"`),
    /// falling back to the `"EU"` cut for a region this table does not carry -
    /// this project's own convention, and the same fallback
    /// [`crate::frontend::title_frame_src`] and
    /// [`crate::frontend::localised_movie_region`] both take.
    ///
    /// # Panics
    ///
    /// Never in practice: [`INTRO_MOVIE_CUTS`] always carries an `"EU"` row.
    #[must_use]
    pub fn intro_movie(region: &str) -> &'static str {
        cut(INTRO_MOVIE_CUTS, region)
    }

    /// The same lookup as [`intro_movie`], over [`FMV_INTRO_MOVIE_CUTS`].
    #[must_use]
    pub fn fmv_intro_movie(region: &str) -> &'static str {
        cut(FMV_INTRO_MOVIE_CUTS, region)
    }

    fn cut(cuts: &[(&str, &'static str)], region: &str) -> &'static str {
        cuts.iter()
            .find(|(code, _)| *code == region)
            .or_else(|| cuts.iter().find(|(code, _)| *code == "EU"))
            .map(|(_, name)| *name)
            .expect("both cut tables carry an EU row")
    }

    /// Every regional cut of the dev/pub reel, by the name that resolves it.
    ///
    /// **All four are on both Pure pressings** (the Korean one only on the EU
    /// disc), and all three of the cuts Pulse ships are here too - the reel is a
    /// shared asset across both titles, played by Pure and carried unused by
    /// Pulse. Each name was recovered by hashing the `_<REGION>` suffix pattern
    /// and confirming the entry exists, then cross-validated against the matching
    /// [`FMV_INTRO_MOVIE_CUTS`] set.
    ///
    /// This is what `localised="true"` selects between. The suffix the original
    /// picks for a given pressing has **not** been read out of any binary; that
    /// the EU disc carries an `_EU` cut whose frame 144 reads "EUROPE" is the
    /// evidence that it picks one at all.
    pub const INTRO_MOVIE_CUTS: &[(&str, &str)] = &[
        ("EU", r"Data\Movies\IntroMovieP1_EU.PMF"),
        ("US", r"Data\Movies\IntroMovieP1_US.PMF"),
        ("JAP", r"Data\Movies\IntroMovieP1_JAP.PMF"),
        ("KO", r"Data\Movies\IntroMovieP1_KO.PMF"),
    ];

    /// The same four cuts of the second boot movie, on the same evidence.
    pub const FMV_INTRO_MOVIE_CUTS: &[(&str, &str)] = &[
        ("EU", r"Data\Movies\WoFMVNew_EU.PMF"),
        ("US", r"Data\Movies\WoFMVNew_US.PMF"),
        ("JAP", r"Data\Movies\WoFMVNew_JAP.PMF"),
        ("KO", r"Data\Movies\WoFMVNew_KO.PMF"),
    ];

    /// The second boot movie, played by the `FMV Intro` screen state.
    ///
    /// The widget's `src` is `Data\Movies\WoFMVNew`, also `localised="true"`,
    /// declared as a sibling of `IntroMovie1` on `Intro Screen` rather than
    /// owned by `FMV Intro` itself - `FMV Intro` carries no widgets of its own,
    /// just a placeholder `Item` and a `Redirect` to `Title Screen`. Confirmed
    /// present the same way as [`INTRO_MOVIE`] (`oag-wad hash` -> `03fff874`).
    ///
    /// **Not wired to playback yet.** `oag_game::frontend` reaches this state
    /// and lets a button leave it, but does not yet decode or draw the movie -
    /// that needs its own decoded-frame slot the way the intro and the menu
    /// backdrop each have one (`crate::frontend::Video`, `Session::feed` /
    /// `Session::backdrop` in `oag_game::main`), which is real, separate work.
    pub const FMV_INTRO_MOVIE: &str = r"Data\Movies\WoFMVNew_US.PMF";
}

/// The bulk archive's candidates. Pure is PSP-only.
const DATA_CANDIDATES: &[(&str, Platform)] = &[(archives::DATA, Platform::Psp)];

/// The companion archive's candidates.
const FE_CANDIDATES: &[(&str, Platform)] = &[(archives::FE, Platform::Psp)];

/// Serials positively identified as a Studio Liverpool title other than Pure.
///
/// The mirror of `oag_pulse`'s list, and one-directional in the same way: it
/// rules a source *out*, never in. Pulse's verified serials are here because
/// Pulse ships `Data.wad` and `FE.wad` under the identical names Pure does,
/// so name matching alone would open one as if it were the other. `UCES-00465`
/// is Pulse's EU/Australia PSP pressing, the same one `oag_pulse::FOREIGN_SERIALS`
/// had to grow when Pure's own EU disc (`UCES-00001`) turned up missing from
/// *that* list - the two are added in the same change here so the pair does
/// not drift apart again.
const FOREIGN_SERIALS: &[ForeignSerial] = &[
    ForeignSerial {
        serial: "UCUS-98712",
        title: "Wipeout Pulse",
    },
    ForeignSerial {
        serial: "UCES-00465",
        title: "Wipeout Pulse",
    },
    ForeignSerial {
        serial: "SCES-54748",
        title: "Wipeout Pulse",
    },
];

/// Opens whichever archives `source` carries, as Wipeout Pure.
///
/// # Errors
///
/// Propagates [`Archives::open`].
pub fn open(source: &str) -> Result<Archives> {
    Archives::open(source, TITLE)
}

/// [`open`], with `packs` mounted behind the disc - see
/// [`Archives::open_with_packs`] for the search order and why the disc wins a
/// collision. The mount itself is title-agnostic (see
/// [ADR-0021](../../../docs/architecture/adr/0021-region-independent-dlc.md)),
/// so this is the same call `oag_pulse::open_with_packs` makes, against this
/// title's own [`TITLE`] - what makes it Pure's is that `packs` came from
/// Pure's own DLC, decrypted per
/// `docs/formats/dlc-pack.md#pures-packs-decrypt-with-an-external-key-table`.
///
/// # Errors
///
/// Propagates [`Archives::open_with_packs`].
pub fn open_with_packs(source: &str, packs: Vec<oag_assets::dlc::Pack>) -> Result<Archives> {
    Archives::open_with_packs(source, TITLE, packs)
}

/// Each Pure release's offered languages, read out of its own `BOOT.BIN`'s
/// plugin manifest: a null-terminated pointer table at file offset `0x2abd98`
/// (USA) and `0x2a5558` (EU), walked in order. Both discs carry the same nine
/// plugins; the executables load different subsets.
///
/// USA loads `PI000 PI010 PI008 PI012 PI001 PI004`: English, Spanish, French -
/// the three languages the USA picker was observed to offer, in the order it
/// offered them - then `PI012` (a US-spelling overlay on `PI000`, no `<Font>`,
/// not a language and so left out here), the skin and the billboards. EU loads
/// `PI000 PI010 PI008 PI009 PI011 PI001 PI004`: those three plus German and
/// Italian. Neither manifest names `PI003` or `PI005`, so the Japanese plugin
/// is never loaded on either pressing. Confidence 85: the tables are read
/// directly; that picker order *is* manifest order rests on the USA and EU
/// pickers' observed orders matching it.
pub const LANGUAGE_MANIFESTS: &[oag_title::LanguageManifest] = &[
    oag_title::LanguageManifest {
        serial: "UCUS-98612",
        plugins: &["PI000", "PI010", "PI008"],
        evidence: "pure-psp-usa BOOT.BIN plugin manifest, PI012 overlay not offered",
    },
    oag_title::LanguageManifest {
        serial: "UCES-00001",
        plugins: &["PI000", "PI010", "PI008", "PI009", "PI011"],
        evidence: "pure-psp-eu BOOT.BIN plugin manifest",
    },
];

#[cfg(test)]
mod tests {
    use super::*;

    /// The deny-list rules out, never in - the same asymmetry `oag_pulse`'s has.
    #[test]
    fn pulses_serials_are_ruled_out_and_pures_own_get_no_verdict() {
        assert_eq!(TITLE.foreign_title("UCUS-98712"), Some("Wipeout Pulse"));
        assert_eq!(TITLE.foreign_title("UCES-00465"), Some("Wipeout Pulse"));
        assert_eq!(TITLE.foreign_title("SCES-54748"), Some("Wipeout Pulse"));
        assert_eq!(TITLE.foreign_title("UCUS-98612"), None);
        assert_eq!(TITLE.foreign_title("UCES-00001"), None);
    }

    /// Pure and Pulse rule *each other* out, which is what stops name matching
    /// opening one as the other.
    #[test]
    fn the_two_titles_are_mutually_exclusive() {
        assert!(TITLE.foreign_title("UCUS-98712").is_some());
        assert!(oag_pulse::TITLE.foreign_title("UCUS-98612").is_some());
    }

    /// A pressing's row carries the very cut its region names, so the table the
    /// boot code reads and the lookups it replaced cannot drift. Covers EU, USA,
    /// no serial and a serial nobody measured (JP, say).
    #[test]
    fn the_pressings_table_matches_the_per_region_cut_lookups() {
        let table = TITLE.pressings.expect("Pure carries its pressings");
        for serial in [
            Some("UCES-00001"),
            Some("UCUS-98612"),
            None,
            Some("UCJP-00001"),
        ] {
            let row = table.of(serial);
            assert_eq!(row.movie_region, frontend::localised_movie_region(serial));
            assert_eq!(row.title_frame, frontend::title_frame_src(serial));
            assert_eq!(row.intro_movie, names::intro_movie(row.movie_region));
            assert_eq!(
                row.fmv_intro_movie,
                names::fmv_intro_movie(row.movie_region)
            );
        }
        assert_eq!(table.of(Some("UCUS-98612")).movie_region, "US");
        assert_eq!(table.of(None).movie_region, "EU");
        assert_eq!(table.of(Some("UCJP-00001")).movie_region, "EU");
    }
}
