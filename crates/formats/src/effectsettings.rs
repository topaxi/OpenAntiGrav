//! `.effectSettings`/`.effectsettings`: the per-stage colour-grade table Zone
//! (and Detonator) read to reskin the same environment stage by stage.
//!
//! # Why this matters
//!
//! **Neither HD/Fury nor 2048 ships a dedicated Zone environment.** A Zone
//! race on, say, Altima runs on the circuit's own geometry, and what makes it
//! *look* like Zone rather than an ordinary race is this file: a title-wide
//! table with one full colour palette per Zone stage (`Start`, `Sub Flash`,
//! ..., `Supersonic`), keyed by the zone number the race is on. See
//! `handover/2048-and-hd-ship-an-unread-effectsettings-table.md` for how this
//! was found and what is still open.
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

#[cfg(test)]
mod tests;
