//! The player's alternate paint job: which `.dat` a race flies, and putting
//! it on the hull.
//!
//! **A skin is a texture swap on the same geometry**, so this is not a second
//! model load: the hull is built exactly as it always was, and four of its
//! texture slots are then replaced from a `.dat` the *definition* names. The
//! format half is [`oag_texture::ship_skin`] and the applier half is
//! [`oag_render::mesh::ship_skin::apply`]; both landed with unit and
//! ground-truth coverage and neither had a caller until this module. See
//! `docs/ghidra/functions/psp-pulse-usa/ship-skin.md`.
//!
//! # Which skin a race flies is chosen, not measured
//!
//! **This build has the player name it, and applies it to the player's craft
//! alone. That is this project's choice and not the original's**, on the same
//! footing [`super::teams_for_slots`] already stands on, and it carries no
//! confidence score because nothing was measured to arrive at it.
//!
//! What the original does instead, and why it is not reproduced here:
//! `Skin_LoadForTeam` picks `ship_eliminator.dat` when a global compares equal
//! to `0x12` and consults no unlock at all on that path, while `ship_alt.dat`
//! is the `loyalty`-gated one. That split is confidence 75. **What `0x12`
//! denotes is a much weaker claim, confidence 55**: "Eliminator race mode" is
//! the natural reading, but no Pulse mode-id table has been recovered to check
//! it against and the enclosing function is multiplayer-lobby code, so a lobby
//! game-type id fits the same evidence. Building a mode gate on that number
//! would be a guess dressed as a reproduction, so this build does not.
//!
//! **No unlock is checked either.** What `loyalty` accumulates is untraced -
//! per-save or per-team, and what the `Team="any"` tier changes about the
//! check - so there is nothing to gate on yet, and every declared skin is
//! offered. That is part of what is chosen here rather than measured.
//!
//! What would replace the choice: resolving the `0x12` global's address with
//! `scripts/psp-relocate.py resolve <instruction-address>`, or tracing the six
//! non-lobby callers of `Skin_ApplyToModel` - `0x08825638`, `0x088256b8`,
//! `0x08828398`, `0x0882885c`, `0x08843804`, `0x088eaa84`. Both need the
//! Ghidra bridge.
//!
//! # The PS2 build paints one atlas, not four slots
//!
//! **The original applies these same `.dat` files on PS2 too - evidenced,
//! not assumed.** `SCES_547.48`'s own strings carry `ship_alt.dat`,
//! `ship_eliminator.dat`, `PI_ModelSkin` and `ALL_TEXTURES.TGA` together
//! (`strings SCES_547.48 | grep -i -E 'ship_alt|all_textures'`). But a PS2
//! ship's `Ship.vex` names one `ALL_Textures.tga` (256x256) where the PSP
//! one names four 128x128 `texture1.tga`..`texture4.tga` slots - confirmed
//! against `pulse-ps2-eu.chd`'s own nested texture set, which decodes no
//! standalone `textureN.tga` entry at all for a base-roster team, only the
//! merged atlas. [`apply`] below detects this case and says so rather than
//! reporting the generic "no slot is named `texture1.tga`" miss, which
//! would read as "this hull has no skin support" when the disc's own
//! strings say the opposite.
//!
//! **What is not recovered: the byte layout the PS2 counterpart of
//! `Skin_ApplyToModel` writes into that one atlas.** A straight 2x2 packing
//! of the four blocks is the natural reading of the dimensions alone - four
//! 128x128 squares tile a 256x256 exactly - but a reading is not a
//! measurement, and nothing here writes a guessed quadrant order into the
//! atlas. The next step is decompiling the PS2 executable's own xrefs to
//! the `ALL_TEXTURES.TGA` string (or to `%s\%s.dat`, the format string that
//! builds `ship_alt.dat`'s path) to find that function and read what it
//! actually does, which needs the Ghidra bridge against the
//! `ps2-pulse-eu` program.

use oag_race::Mode;
use oag_render::mesh::{self, Model};

/// The label [`oag_render::mesh::resolve_texture_slots`] gives a PS2 hull's
/// single paintable atlas, where the PSP build's
/// [`oag_render::mesh::ship_skin::SLOT_NAMES`] names four separate slots
/// instead. Used only to tell that specific miss apart from a hull that
/// truly declares none of the four PSP slot names, in [`apply`]'s own
/// report line - not to paint it, since the byte layout is unrecovered.
const PS2_ATLAS_LABEL: &str = "ALL_Textures.tga";

/// The archive entry holding the skin `team` should fly, or nothing.
///
/// `asked` is a `PI_ModelSkin` name - `Alternative` or `Eliminator` on every
/// source measured - matched case-insensitively against what the team itself
/// declares. `None` in, `None` out, which is the baseline hull with its own
/// textures and is what every race did before this module.
///
/// **The path is the definition's own, verbatim.** A `PI_ModelSkin`'s
/// `location` is a full archive entry name, unlike the bare file stem its
/// `PI_TeamModel` sibling carries under the same attribute name, so nothing
/// here composes a path out of a team id. A team that declares no such skin
/// gets a report line naming what it does declare, and races the baseline -
/// an absence that is visible in the log rather than a substituted file.
///
/// The disc's own definition and every mounted pack's manifest are both read,
/// so a DLC team's skin resolves the same way a disc team's does. `text`
/// rather than `expand`, for the reason `race::load::roster` records: shortening
/// is per file, and `expand` refuses a document with no `<code>` dictionary.
pub(crate) fn resolve(
    archives: &mut oag_assets::Archives,
    title: &'static oag_title::Title,
    team: &str,
    asked: Option<&str>,
    mode: Mode,
    report: &mut Vec<String>,
) -> Option<String> {
    let asked = asked?;
    // **Nothing pairs a `ship_*.dat` with `Zone.vex`.** A Zone run flies a
    // different hull whose own selection is a title fact
    // (`oag_title::ZoneCraft`), and no `PI_ModelSkin` names a file for it, so
    // offering one here would be painting a surface the disc does not specify.
    // The same carve-out `ship_entry_name` makes for the hull variant.
    if mode == Mode::Zone {
        report.push(format!(
            "--skin {asked}: ignored on a Zone race - the Zone hull is a different model \
             and no PI_ModelSkin names a file for it"
        ));
        return None;
    }
    let definition = title.plugin_definition;
    let mut documents = Vec::new();
    match archives
        .read_name(definition)
        .map_err(|error| error.to_string())
        .and_then(|blob| oag_tables::fexml::text(&blob).map_err(|error| error.to_string()))
    {
        Ok(xml) => documents.push(xml),
        Err(error) => report.push(format!(
            "--skin {asked}: {definition} is unreadable ({error}) - no skin for {team}"
        )),
    }
    documents.extend(archives.manifests.iter().cloned());
    declared(&documents, team, asked, report)
}

/// [`resolve`]'s own half that needs no archive: the declared path for one
/// team's named skin, out of already-read definition documents.
///
/// Separate so the lookup and its report lines are testable against XML
/// alone - opening an image to find out what a missing skin says is a test
/// that would only run with a disc in the tree.
fn declared(
    documents: &[String],
    team: &str,
    asked: &str,
    report: &mut Vec<String>,
) -> Option<String> {
    let Some(declared) = crate::catalogue::all_teams(documents)
        .into_iter()
        .find(|declared| declared.id.eq_ignore_ascii_case(team))
    else {
        report.push(format!(
            "--skin {asked}: nothing read declares a team {team} - racing the baseline paint"
        ));
        return None;
    };
    let Some(skin) = declared.skin(asked) else {
        let offered: Vec<&str> = declared.skins.iter().map(|s| s.name.as_str()).collect();
        report.push(format!(
            "--skin {asked}: {team} declares no such PI_ModelSkin (it declares {}) - \
             racing the baseline paint",
            if offered.is_empty() {
                "none".to_string()
            } else {
                offered.join(", ")
            }
        ));
        return None;
    };
    report.push(format!(
        "--skin {asked}: {team} declares {} - which skin a race flies is this project's \
         choice, not the original's, and no unlock is checked (livery/ship_skin.rs)",
        skin.location
    ));
    Some(skin.location.clone())
}

/// Puts `entry`'s four blocks over `hull`'s matching texture slots.
///
/// Every outcome is reported and none is silent, on the module rule a missing
/// hull already follows: a race that quietly flew the baseline paint after
/// being asked for a skin looks exactly like one that was never asked.
///
/// **A file that will not read or will not parse leaves the hull's own
/// textures alone**, which is the disc's own data rather than a stand-in, and
/// says so. Nothing here invents a texture.
pub(super) fn apply(
    archives: &mut oag_assets::Archives,
    entry: &str,
    hull: &mut Model,
    report: &mut Vec<String>,
) {
    let blob = match archives.read_name(entry) {
        Ok(blob) => blob,
        Err(error) => {
            report.push(format!(
                "{entry}: not in the archive set ({error}) - the craft keeps its own paint"
            ));
            return;
        }
    };
    let skin = match oag_texture::ship_skin::parse(&blob) {
        Ok(skin) => skin,
        Err(error) => {
            report.push(format!("{entry}: {error} - the craft keeps its own paint"));
            return;
        }
    };
    // **The dimensions are the block's, and the original reads the target
    // texture's descriptor instead** - the file declares none. They agree on
    // every shipped pair, and a mismatch would stretch the paint rather than
    // fail, so it is worth a line of its own when it happens.
    let stretched: Vec<String> = hull
        .textures
        .iter()
        .flatten()
        .filter_map(|slot| {
            let block = &skin.blocks[mesh::ship_skin::slot_of(&slot.label)?];
            (slot.width != block.width as u32 || slot.height != block.height as u32).then(|| {
                format!(
                    "{} is {}x{} and its block is {}x{}",
                    slot.label, slot.width, slot.height, block.width, block.height
                )
            })
        })
        .collect();
    let applied = mesh::ship_skin::apply(hull, &skin);
    report.push(format!(
        "{entry}: {} ({applied} of {} texture slot(s) repainted)",
        skin.team_name,
        hull.textures.len()
    ));
    if applied == 0 {
        // **A PS2 hull's paintable surface is one atlas, not four slots.**
        // Its `Ship.vex` names `ALL_Textures.tga` (256x256) where the PSP
        // one names `texture1.tga`..`texture4.tga` (four 128x128 slots) -
        // `Ps2TextureSet::resolve` finds no standalone `textureN.tga` entry
        // at all for a base-roster team, only the merged atlas, so
        // `mesh::ship_skin::slot_of` has nothing to match against and this
        // is not the generic "hull declares none of the four slots" miss.
        // **The original applies the same `.dat` files here too, evidenced
        // rather than assumed**: `SCES_547.48`'s own strings carry
        // `ship_alt.dat`, `ship_eliminator.dat`, `PI_ModelSkin` and
        // `ALL_TEXTURES.TGA` together. What is not recovered is the byte
        // layout `Skin_ApplyToModel`'s PS2 counterpart writes into that one
        // atlas - a 2x2 packing of the four blocks is the natural reading of
        // the dimensions alone, but a reading is not a measurement, so this
        // build leaves the atlas untouched rather than write a guessed
        // layout into it. See `docs/formats/ps2-texture.md`.
        let atlas = hull
            .textures
            .iter()
            .flatten()
            .any(|slot| slot.label.eq_ignore_ascii_case(PS2_ATLAS_LABEL));
        if atlas {
            report.push(format!(
                "{entry}: this hull's paintable surface is {PS2_ATLAS_LABEL}, not the four \
                 texture1.tga..texture4.tga slots this file's blocks target - the PS2 \
                 executable's own strings show it applies ship_alt.dat/ship_eliminator.dat \
                 here too, but the byte layout it writes into the atlas is not recovered, so \
                 this build will not guess it - the craft keeps its own paint"
            ));
        } else {
            report.push(format!(
                "{entry}: no slot of this hull is named texture1.tga..texture4.tga - \
                 the craft keeps its own paint"
            ));
        }
    }
    if !stretched.is_empty() {
        report.push(format!(
            "{entry}: {} slot(s) whose size the skin does not match ({})",
            stretched.len(),
            stretched.join("; ")
        ));
    }
}

#[cfg(test)]
mod tests;
