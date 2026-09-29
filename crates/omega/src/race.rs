//! Where a race would look for its craft and circuit, if this lane read one.
//!
//! **Racing is out of scope for this crate.** Omega's circuit files are not
//! `oag_vex`-shaped the way every other title's are: `Data\environments\
//! talons_junction\` ships `TrackStartup.xml`, `track.final.audio` and
//! `track.final.rcsskeleton` in place of a `track.vex` - a `.final.*`
//! extension family this project has never seen and has not opened. Most
//! other environments (`tech_de_ra`, `zone_4`, the `environments2048\*`
//! set) *do* still ship a plain `track.vex`, so the format split is per
//! circuit rather than title-wide, and which is which needs a real survey
//! this lane did not run. Ship geometry is behind the same wall from the
//! other side: `.rcsmodel` bodies sit beside `.gnf` textures this project has
//! no PS4 texture reader for (see `docs/formats/omega-frontend.md`'s note on
//! `.gnf`).
//!
//! So nothing here is a claim about what Omega's race path looks like -
//! [`oag_title::RaceDefaults`]'s mandatory fields are filled with real,
//! archive-confirmed entries chosen for existing rather than for being
//! correct, and every field that has an honest "unmeasured" state
//! ([`Option`], or a zero-fabrication enum variant) takes it. **This is
//! provably safe**: `crates/game/src/title.rs`'s Omega branch (once wired)
//! refuses `--race`/a cell confirm by name before any of this is read, the
//! same way `hud`'s own placeholders are inert - see `crate::hud`'s doc
//! comment for the one place `Title::race` and `Title::hud` are actually
//! consumed.

/// The HD-derived ship directory, lowercase, confirmed by direct listing on
/// `data00.psarc` (`Data/art/published/hdships/ag_systems/...`). 2048's own
/// scheme for its HD-derived roster, not HD's `Data\Ships`.
pub const SHIP_DIR: &str = r"Data\art\published\hdships";

/// Real-but-unverified race defaults - see the module doc for why nothing
/// here is more than "confirmed present in the archive".
pub const DEFAULTS: &oag_title::RaceDefaults = &oag_title::RaceDefaults {
    // Confirmed present as a plain `track.vex` (not the `.final.*` family
    // some other environments ship) in `data02.psarc`'s listing. Chosen for
    // existing, not for being any kind of "first" or default circuit - no
    // menu order was read.
    track: r"Data\environments\tech_de_ra\track.vex",
    // Confirmed present: `Data\art\published\hdships\ag_systems\` is a real
    // directory in `data00.psarc`'s listing (two `.gnf` thumbnails, no
    // `.rcsmodel`/handling file located this lane). Chosen for existing.
    team: "ag_systems",
    // **2048's ship-directory scheme, not HD's `Data\Ships\<Team>\`** -
    // confirmed by direct listing: Omega's HD-derived roster lives at
    // `Data\art\published\hdships\<Team>\`, lowercase ids, the same
    // directory 2048's own `oag_title::race::SHIP_DIR` names for its
    // HD-derived craft (`crates/2048/src/lib.rs`'s module doc, "The asset
    // tree is Wipeout HD's, and the ship directory is not"). New finding for
    // Omega, not carried from `omega-frontend.md`, which never opened this
    // tree. **Not [`oag_title::race::SHIP_DIR`]** - that constant is
    // `Data\Ships`, the path Pulse/Pure/HD share and Omega does not.
    ship_dir: SHIP_DIR,
    // **Not independently confirmed.** No `handlingstats.xml` turned up
    // under `hdships\ag_systems\` in this lane's (partial) listing - it may
    // sit elsewhere, the way 2048's own non-HD roster splits models and
    // tuning into two trees. Equal to `ship_dir` is the same-directory guess
    // every title but 2048's native roster gets right, carried here as the
    // more likely of two unverified answers rather than as a measurement.
    handling_dir: SHIP_DIR,
    // **Zero-fabrication placeholder, not a finding.** `SameCircuit` is the
    // one `ZoneCircuit` variant that names no path at all - 2048's own shape,
    // picked here only because it requires inventing nothing. Omega's Zone
    // mode has not been read; this is not a claim that it works like 2048's.
    zone: oag_title::ZoneCircuit::SameCircuit,
    // Same reasoning as `zone` above: `PlayerShip` is the one `ZoneCraft`
    // variant that names no model stem or directory. Not a finding.
    zone_craft: oag_title::ZoneCraft::PlayerShip,
    // Not searched for this lane, on the same footing as everything else in
    // this module doc's opening paragraph: `None` is silence, not a finding.
    boost: None,
    sounds: SOUND_BANKS,
    zone_announcer: None,
    countdown_voice: None,
    zone_class_announcer: None,
    zone_palette: None,
    zone_stages: None,
    zone_transition: None,
    zone_stage_textures: None,
    zone_sky: None,
    team_variants: None,
    guest_roster: None,
    hull_variants: None,
    speed_classes: None,
};

/// Sound banks, confirmed present by name in `data00.psarc`'s listing -
/// **not** copied from [`oag_hd::race::SOUND_BANKS`], whose paths
/// (`Data\Sound\...`) do not exist on Omega at all. Omega's own tree is
/// `Data\audio\sound\...`, lowercase, a different directory this project has
/// not seen before. `hud` reuses the same bank `weapons` does, on the same
/// unverified assumption HD's own table makes for the same reason - no
/// separate `hud.bnk`-named file was found.
pub const SOUND_BANKS: &oag_title::SoundBanks = &oag_title::SoundBanks {
    hud: r"Data\audio\sound\weapons.bnk",
    ship: r"Data\audio\sound\shipHD.bnk",
    ship_zone: r"Data\audio\sound\shipHD.bnk",
    weapons: r"Data\audio\sound\weapons.bnk",
    speech: r"Data\audio\sound\speech.bnk",
    // Not searched for this lane.
    track_general: None,
};
