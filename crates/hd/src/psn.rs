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
//! Every row is **chosen, not measured** unless it says otherwise: the PSN
//! executable is an encrypted `EBOOT.BIN` this project cannot read, so what it
//! would name is unobserved, and the choice below is the file that exists.

use oag_title::{FrontEnd, RaceDefaults, Title, ZoneCircuit, exhaust::Exhaust};

use crate::{TITLE, frontend, race};

/// The circuit a bare `--race` opens: the first of the eight the PSN package
/// ships. Talon's Junction, the disc's default, is in `DATA00`, which a PSN
/// install does not have.
pub const DEFAULT_TRACK: &str = "/data/environments/01_vineta_k/track.vex";

/// The race defaults with the PSN circuit and a Zone shape whose files exist.
///
/// `zone_1` to `zone_4` are `DATA00`'s, so [`ZoneCircuit::Separate`] would name
/// four missing circuits; `SameCircuit` runs Zone on the circuit the player
/// picked, the shape this project already uses where no dedicated Zone circuit
/// ships. The package does carry `zonemode.effectsettings` and the `zone` craft.
const RACE: &RaceDefaults = &RaceDefaults {
    track: DEFAULT_TRACK,
    zone: ZoneCircuit::SameCircuit,
    ..*race::DEFAULTS
};

/// The front end without the two Fury selection files.
///
/// `Team_Selection_Definition.xml` and `Track_Selection_Definition.xml` are
/// `DATA06`'s alone. With neither named, the race box keeps its plain RACE page
/// rows instead of a picker the copy cannot draw. The older
/// `Selection_Definition.xml` the package does carry is a different dialect and
/// is not read.
const FRONT_END: &FrontEnd = &FrontEnd {
    team_select: None,
    track_select: None,
    ..*frontend::FRONT_END
};

/// Wipeout HD as the PSN download ships it.
pub const PSN: &Title = &Title {
    race: RACE,
    front_end: Some(FRONT_END),
    // The bluered template is `DATA06`'s. The plain one is in `DATA02`; the
    // disc's own doc comment calls it the pre-Fury build's, which is what this
    // package is.
    exhaust: &Exhaust::Authored("/data/ribboneffects/enginetrail_triangle.rcsmodel"),
    ..*TITLE
};
