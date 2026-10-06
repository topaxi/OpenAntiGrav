//! The tables a Wipeout title authors as text, rather than the bytes it draws.
//!
//! Everything here parses a human-authored file - XML in every case so far -
//! into numbers the engine reads: [`handling`] tunes a ship, [`weapons`] tunes
//! a pickup, [`race_campaign`] lays out a campaign's grid, [`mjolnir`] is
//! Wipeout 2048's own differently-shaped campaign database, [`trackstartup`]
//! says what a circuit loads, [`envsettings`] and [`effectsettings`] carry
//! the lighting and colour-grade rigs the later titles ship, [`enginelight`]
//! is where Wipeout HD hangs each ship's engine light, and
//! [`fury_backdrop`] is the one `.envsettings` that is a menu's camera paths
//! rather than a circuit's light, and [`track_stats`] is a track's own
//! `stats.xml` - the per-class `SkillScaleValue` curve the built-in campaign
//! reads to scale AI difficulty.
//!
//! [`fexml`] is the layer underneath all of them: the tag-and-attribute reader,
//! including the name shortening the originals apply to element names.
//!
//! **This crate has no dependencies, and that is deliberate.** It is the one
//! the simulation reads its constants from, so it has no compilable path to an
//! archive reader, a byte order or a texture decoder. Getting the file *out* of
//! a WAD is `oag-formats`' job and happens before anything here is called - see
//! [ADR-0050](../../../docs/architecture/adr/0050-format-crates-split-by-format-family.md).
//!
//! Which schema a file uses is still decided by the file, never by a caller
//! naming a title, per
//! [ADR-0022](../../../docs/architecture/adr/0022-title-packages.md).

pub mod ai_race_stats;
pub mod billboard_pool;
pub mod effectsettings;
pub mod enginelight;
pub mod envsettings;
pub mod fexml;
pub mod fury_backdrop;
pub mod handling;
pub mod mjolnir;
pub mod race_campaign;
pub mod track_stats;
pub mod trackstartup;
pub mod weapons;
