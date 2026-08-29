//! `.effectSettings`/`.effectsettings`: the per-stage colour-grade table Zone
//! (and Detonator) read to reskin the same environment stage by stage.
//!
//! # Why this matters
//!
//! **2048 has no dedicated Zone circuit at all - it races Zone on whatever
//! circuit the player picked** (`docs/gameplay/race-modes.md#zone`), where
//! HD/Fury's own Zone races one of four ported `/data/environments/zone_N/`
//! circuits. Neither fact explains this file: even on a *dedicated* Zone
//! circuit, the look still has to escalate as the race goes on - the disc's
//! own `MSC_EVENT_ZONE` text says the top speed rises every ten seconds, and
//! this table is the palette that rises with it. One full colour palette per
//! Zone stage (`Start`, `Sub Flash`, ..., `Supersonic`), keyed by the zone
//! number the race is on, layered over whichever circuit is racing rather
//! than replacing its base environment.
//!
//! # The format is `.envsettings`'s tokeniser, with a stage prefix on the key
//!
//! Same syntax [`envsettings`] already parses - one quoted, dotted
//! `"key"=floats` per line, no sections, no escaping:
//!
//! ```text
//! "0 Start.Lighting.Sun colour"=1.000000 1.000000 1.000000 0.000000
//! "Zone 0 Start.Colour 1.Colour"=1.000000 1.000000 1.000000
//! ```
//!
//! so [`EffectSettings::parse`] reads the whole file through
//! [`EnvSettings::parse`] rather than a second tokeniser. What is new is that
//! most keys carry a **stage prefix** before the first `.` - HD writes
//! `"<n> <Name>"`, 2048 writes `"Zone <n> <Name>"` - and a handful of
//! title-wide keys (2048's `"Sky Radius"`, HD's `"Texture U scale"`) carry
//! none at all and read straight off [`EffectSettings::table`].
//!
//! # Confidence
//!
//! **85** on the syntax and the stage-prefix shape: all five files measured -
//! HD's `zonemode.effectsettings`, `zonemodedlc3.effectsettings`,
//! `detonatormode.effectsettings`, `detonatormodedlc3.effectsettings` and
//! 2048's ten identical `ZoneMode2048.effectSettings` copies - parse in full,
//! and every stage name in each recovers correctly (checked in
//! `effectsettings_ground_truth.rs`). Not 90, unlike `.envsettings`: **the
//! stage-index-to-zone-number correspondence is inferred from the names
//! alone**, not checked against `Zone_Update`'s own timer - see the thread
//! file's `## Open`. Nothing here reads either title's executable, and
//! nothing wires a stage's values into a race yet.
//!
//! [`cross_fade_rgba8`] is the one piece of this module that *does* read
//! the executable - HD/Fury's own runtime cross-fade between adjacent
//! stages, recovered from two independent functions (confidence 80, see
//! its own doc comment). It stays a standalone, untriggered utility: which
//! key feeds it and what selects the current stage are both still open.

use std::collections::BTreeMap;

use crate::envsettings::{self, EnvSettings};

/// One stage's own identity, as the file names it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Stage {
    /// The zone number, as the key's own leading integer - `0` for `Start`.
    pub index: u32,
    /// The stage's authored name (`"Start"`, `"Sub Venom"`, `"Supersonic"`).
    pub name: String,
    /// The exact text before the first `.` of every key this stage owns -
    /// `"0 Start"` on HD, `"Zone 0 Start"` on 2048. Kept whole rather than
    /// reconstructed from `index`/`name`, so [`EffectSettings::stage_key`]
    /// never has to guess which title's spelling to rebuild.
    prefix: String,
}

/// The zone index and stage name a key's own leading segment names, or
/// `None` for a key with no stage prefix at all (a title-wide key like
/// `"Sky Radius"`, or a plain group name like `"Lighting"` with no leading
/// number).
fn parse_stage_head(head: &str) -> Option<(u32, &str)> {
    let head = head.strip_prefix("Zone ").unwrap_or(head);
    let (number, name) = head.split_once(' ')?;
    let index: u32 = number.parse().ok()?;
    Some((index, name))
}

/// Everything one `.effectSettings`/`.effectsettings` file says.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct EffectSettings {
    /// Every entry, exactly as [`EnvSettings::parse`] read it - stage prefix
    /// and all, still part of the key. Title-wide keys with no stage prefix
    /// read straight off this.
    pub table: EnvSettings,
    /// Every stage this file names, keyed by its zone number.
    ///
    /// A `BTreeMap` for the same determinism reason [`EnvSettings::entries`]
    /// is one - see `docs/architecture/determinism.md`.
    pub stages: BTreeMap<u32, Stage>,
}

impl EffectSettings {
    /// Reads a whole file.
    ///
    /// # Errors
    ///
    /// [`envsettings::Error`] for the first line that is neither blank nor
    /// `key=value` - the same failure [`EnvSettings::parse`] reports, since
    /// this is that parser plus a stage index read off the key.
    pub fn parse(text: &str) -> Result<Self, envsettings::Error> {
        let table = EnvSettings::parse(text)?;
        let mut stages: BTreeMap<u32, Stage> = BTreeMap::new();
        for key in table.entries.keys() {
            let Some((head, _rest)) = key.split_once('.') else {
                continue;
            };
            let Some((index, name)) = parse_stage_head(head) else {
                continue;
            };
            stages.entry(index).or_insert_with(|| Stage {
                index,
                name: name.to_string(),
                prefix: head.to_string(),
            });
        }
        Ok(Self { table, stages })
    }

    /// The full key one stage's own `key` reads under, or `None` for a stage
    /// this file does not name.
    #[must_use]
    pub fn stage_key(&self, stage: u32, key: &str) -> Option<String> {
        let prefix = &self.stages.get(&stage)?.prefix;
        Some(format!("{prefix}.{key}"))
    }

    /// One number, for a per-stage key that carries exactly one.
    #[must_use]
    pub fn stage_scalar(&self, stage: u32, key: &str) -> Option<f32> {
        self.table.scalar(&self.stage_key(stage, key)?)
    }

    /// Three numbers, for a per-stage key that carries exactly three.
    ///
    /// Not clamped or normalised, the same as [`EnvSettings::vec3`] - several
    /// of these exceed `1.0` (2048's `"Edge Colour"` reaches 1.098).
    #[must_use]
    pub fn stage_vec3(&self, stage: u32, key: &str) -> Option<[f32; 3]> {
        self.table.vec3(&self.stage_key(stage, key)?)
    }

    /// Four numbers, for a per-stage key that carries exactly four.
    ///
    /// [`EnvSettings::rgba8`] cannot answer these: HD's four-number colour
    /// keys (`"Lighting.Sun colour"`, `"Lighting.Fog colour"`, ...) are
    /// written with decimal points, not the byte quadruple
    /// [`envsettings::SKY_COLOUR`] is.
    #[must_use]
    pub fn stage_vec4(&self, stage: u32, key: &str) -> Option<[f32; 4]> {
        let key = self.stage_key(stage, key)?;
        match self.table.entries.get(&key)?.numbers.as_slice() {
            [x, y, z, w] => Some([*x, *y, *z, *w]),
            _ => None,
        }
    }
}

/// Cross-fades two stages' packed-byte colour channels, byte for byte, the
/// way HD/Fury's own runtime does.
///
/// `weight` is the current stage's own share of the blend (`1.0` at the
/// moment a stage becomes current, falling toward `0.0` as the previous
/// stage's own share, `1.0 - weight`, takes over) - expected in `0.0..=1.0`;
/// values outside that range are not clamped going in, matching the traced
/// code, which never checks the lower bound either.
///
/// # Evidence
///
/// Recovered independently from two functions in HD/Fury's executable that
/// compute this exact arithmetic against the same `0x250`-stride runtime
/// stage table - `FUN_003da540` and `FUN_003ce2c0`, both `ps3-hdfury-eu`,
/// confidence 80 (real, exercised mechanism; corroborated twice rather than
/// found once). Per channel: `(int)(current * weight) + (int)(previous *
/// other_weight)`, each term truncated toward zero before the two are
/// summed, then clamped to `0xff` on the high side only - not a single
/// truncate-after-sum. See
/// [zone-effectsettings-loader.md](../../../docs/ghidra/functions/ps3-hdfury-eu/zone-effectsettings-loader.md#2026-08-29-a-fourth-pass-who-writes-0x008b7944--n0x38---one-near-repeat-of-the-opd-trap-caught-before-it-shipped-one-real-correction-one-new-lead).
///
/// # What this does not settle
///
/// **Nothing calls this yet.** Which parsed `.effectSettings` key feeds
/// which channel, which stage counts as "current" at a given moment, and
/// what `weight` should read at runtime are all still unrecovered - see the
/// thread file's `## Open`
/// (`handover/2048-and-hd-ship-an-unread-effectsettings-table.md`). This is
/// the confirmed blend arithmetic alone, ready to reuse once those are
/// found - implementing it ahead of its trigger is not the same as wiring
/// it into a race.
#[must_use]
pub fn cross_fade_rgba8(current: [u8; 4], previous: [u8; 4], weight: f32) -> [u8; 4] {
    let other_weight = 1.0 - weight;
    let mut out = [0u8; 4];
    for i in 0..4 {
        let from_current = (f32::from(current[i]) * weight) as i32;
        let from_previous = (f32::from(previous[i]) * other_weight) as i32;
        out[i] = (from_current + from_previous).clamp(0, 255) as u8;
    }
    out
}

#[cfg(test)]
mod tests;
