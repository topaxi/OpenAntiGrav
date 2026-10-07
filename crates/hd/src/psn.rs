//! The plain Wipeout HD PSN download (`NPEA00057`, v3.00), as a variant of [`TITLE`].
//!
//! **Same title, same name, fewer archives.** The PSN package is the base game
//! without Fury: four archives (`data01` to `data04`), no `DATA00`, `DATA05` or
//! `DATA06`. Everything this crate names that lives only in those three is
//! absent, and each row below is one such name replaced by what the PSN copy
//! does ship. The census is `docs/formats/hd-psn.md`.
//!
//! [`TITLE`]'s `name` is carried over unchanged, so nothing that asks "is this
//! Wipeout HD" changes its answer; only data that points at a file does.
//!
//! **What is measured and what is chosen.** The PSN `EBOOT.BIN` decrypts under
//! RPCS3 (`--decrypt`, with the player's own licence file), and `strings` over
//! the result is what the rows below cite: the exhaust ribbon and the menu frame
//! are the executable's own literals. The default circuit, the Zone shape and
//! the mount order are choices, labelled as such.

use oag_title::{
    FrontEnd, MenuBlocks, MenuSkin, RaceDefaults, Title, ZoneCircuit, exhaust::Exhaust,
};

use crate::{TITLE, frontend, race};

/// The circuit a bare `--race` opens: the first of the eight the PSN package
/// ships. Chosen, not measured. Talon's Junction, the disc's default, is in
/// `DATA00`, which a PSN install does not have.
pub const DEFAULT_TRACK: &str = "/data/environments/01_vineta_k/track.vex";

/// The race defaults with the PSN circuit and a Zone shape whose files exist.
///
/// `zone_1` to `zone_4` are `DATA00`'s, so [`ZoneCircuit::Separate`] would name
/// four missing circuits; `SameCircuit` runs Zone on the circuit the player
/// picked, the shape this project already uses where no dedicated Zone circuit
/// ships. Chosen, not measured. The package does carry `zonemode.effectsettings`
/// and the `zone` craft, and the executable names `ZoneMode` textures.
const RACE: &RaceDefaults = &RaceDefaults {
    track: DEFAULT_TRACK,
    zone: ZoneCircuit::SameCircuit,
    ..*race::DEFAULTS
};

/// The menu block frame this executable names: `file.gtf`, where the Fury disc
/// names `file2.gtf` (`DATA06`). Measured: the literal is in the decrypted
/// `EBOOT`, and the disc's own literal sits at the same job.
const MENU_BLOCKS: MenuBlocks = MenuBlocks {
    frame_texture: frontend::names::MENU_BLOCK_FRAME_PSN,
    ..frontend::MENU_BLOCKS
};

const MENU_SKIN: &MenuSkin = &MenuSkin {
    blocks: Some(MENU_BLOCKS),
    ..*frontend::MENU_SKIN
};

/// The front end without the two Fury selection files.
///
/// `Team_Selection_Definition.xml` and `Track_Selection_Definition.xml` are
/// `DATA06`'s alone. With neither named, the race box keeps its plain RACE page
/// rows instead of a picker the copy cannot draw. The older
/// `Selection_Definition.xml` the package does carry is a different dialect and
/// is not read; the executable names neither dialect's file as a literal.
const FRONT_END: &FrontEnd = &FrontEnd {
    menu: Some(MENU_SKIN),
    team_select: None,
    track_select: None,
    ..*frontend::FRONT_END
};

/// Wipeout HD as the PSN download ships it.
pub const PSN: &Title = &Title {
    race: RACE,
    front_end: Some(FRONT_END),
    // **Measured.** The decrypted PSN `EBOOT` names
    // `Data/RibbonEffects/enginetrail_triangle.vex`, where the Fury disc's names
    // the `bluered` one (`DATA06`); the `.rcsmodel` is the same template the way
    // the disc's constant derives it.
    exhaust: &Exhaust::Authored("/data/ribboneffects/enginetrail_triangle.rcsmodel"),
    ..*TITLE
};
