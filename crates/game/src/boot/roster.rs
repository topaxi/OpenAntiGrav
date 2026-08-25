//! What a source offers to race on: the plugin definition, the circuits and
//! the roster.
//!
//! Split out of `boot.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change beyond the
//! one this split accompanied - the definition's entry name is the title's now
//! rather than Pulse's. See [`oag_title::Title::plugin_definition`].
//!
//! The functions run in order and each is fed the one above it:
//! [`definitions`] reads the documents, [`load_tracks`] and [`load_teams`] read
//! the two catalogues out of them, and [`load_circuit_names`] puts a name on
//! what [`load_tracks`] found.

use super::expand;

/// The game plugin's own definition, followed by every mounted pack's
/// manifest.
///
/// One list because they are one schema: a pack declares its additions as a
/// fragment of the very file the disc ships, so both go through
/// [`crate::catalogue`] unchanged. The disc's own definition is first, which is
/// what makes it win a collision.
///
/// A definition that will not read is reported and not fatal: a source whose
/// plugin is unreadable still boots and still races the default track.
///
/// `name` is [`oag_title::Title::plugin_definition`], not Pulse's constant: HD
/// does not carry that path. See [`load_teams`] for what reaching for it cost.
pub(super) fn definitions(
    archives: &mut oag_assets::Archives,
    name: &str,
    report: &mut Vec<String>,
) -> Vec<String> {
    let mut out = Vec::new();
    match archives
        .read_name(name)
        .and_then(|blob| expand(&blob).map_err(|e| oag_assets::Error::BadSpec(e.to_string())))
    {
        Ok(xml) => out.push(xml),
        Err(e) => report.push(format!("{name}: {e}")),
    }
    out.extend(archives.manifests.iter().cloned());
    out
}

/// Reads the raceable circuits out of [`definitions`].
///
/// A pack's circuit is dropped when the geometry it names is in none of the
/// mounted archives, which is what a partial set of packs looks like: two of
/// the four packs cross-declare each other's circuits, so owning one means
/// holding a declaration for a track whose `.vex` is in a pack you did not buy.
/// Skipped and reported, rather than offered and then failing at the archive
/// with a message about a missing entry.
pub(super) fn load_tracks(
    archives: &mut oag_assets::Archives,
    definition: &str,
    documents: &[String],
    report: &mut Vec<String>,
) -> Vec<crate::catalogue::Track> {
    let declared = crate::catalogue::all_tracks(documents);
    let declared_count = declared.len();
    let tracks: Vec<_> = declared
        .into_iter()
        .filter(|track| archives.locate(&track.entry_name()).is_some())
        .collect();

    if tracks.len() != declared_count {
        report.push(format!(
            "dlc: {} declared circuit(s) have no geometry on this source; a pack \
             they belong to is not mounted",
            declared_count - tracks.len()
        ));
    }
    report.push(format!(
        "{definition}: {} raceable circuit(s) over {} definition(s)",
        tracks.len(),
        documents.len()
    ));
    tracks
}

/// Reads the roster out of [`definitions`], the same way [`load_tracks`] reads
/// the circuits.
///
/// Filtered the same way too, and for the same reason - but against **both**
/// files a race needs, not just the model.
///
/// A pack splits the two across archives: the ship is in `PACKn.edat` and the
/// handling stats are in `PACKn_UI1.edat`. A hand-copied or repacked pack
/// holding only the first would pass a model-only filter, appear in the menu,
/// and then fail at the stats read the moment it was picked. Checking both is
/// what keeps "offered" and "raceable" the same set - the job the deleted
/// menu.rs assertion used to do when the roster was a fixed list.
///
/// Both names are composed by the same functions `race::load` will call, so the
/// filter cannot disagree with the loader about how a path is spelled.
pub(super) fn load_teams(
    archives: &mut oag_assets::Archives,
    definition: &str,
    documents: &[String],
    report: &mut Vec<String>,
) -> Vec<crate::catalogue::Team> {
    let declared = crate::catalogue::all_teams(documents);
    let declared_count = declared.len();
    let mut teams: Vec<_> = declared
        .into_iter()
        .filter(|team| raceable(archives, &team.id))
        .collect();

    if teams.len() != declared_count {
        report.push(format!(
            "{} declared team(s) have no ship or no handling stats on this \
             source; a pack they belong to may be only half mounted",
            declared_count - teams.len()
        ));
    }

    // A source whose plugin definition will not read is reported and not fatal
    // - see `definitions` - but a TEAM row with nothing in it would leave the
    // player unable to start a race at all, where the circuits row still has a
    // default to fall back on. So the eight teams the PSP disc always ships
    // stand in, still filtered against what is really there. This is the list
    // the menu definition itself carried before the roster became data. It
    // used to fire on every HD boot, when `definitions` asked for Pulse's
    // plugin name: eight of HD's twelve teams, off a list this crate holds
    // rather than one the disc declares. The line below says which happened.
    if teams.is_empty() {
        teams = oag_formats::handling::TEAMS
            .iter()
            .filter(|id| raceable(archives, id))
            .map(|id| crate::catalogue::Team {
                id: (*id).to_string(),
                location: format!(r"Data\Ships\{id}"),
                help_text: None,
            })
            .collect();
        if !teams.is_empty() {
            report.push(format!(
                "no team was declared; falling back to the {} shipped team(s) \
                 this source actually carries",
                teams.len()
            ));
        }
    }

    report.push(format!(
        "{definition}: {} team(s) over {} definition(s)",
        teams.len(),
        documents.len()
    ));
    teams
}

/// Whether both files a race reads for a team are on this source.
fn raceable(archives: &oag_assets::Archives, id: &str) -> bool {
    archives
        // The ordinary hull, spelled directly rather than through
        // `race::ship_entry_name`: this asks whether a team is *raceable*, which
        // is a question about the normal hull and never about the Zone one, and
        // routing it through the mode-aware helper meant passing a `ZoneCraft`
        // that could not affect the answer.
        .locate(&oag_pulse::race::ships::entry_name(
            id,
            oag_pulse::race::ships::HULL,
        ))
        .is_some()
        && archives
            .locate(&oag_formats::handling::entry_name(id))
            .is_some()
}

/// What each circuit is called, out of whichever copy of the string table names
/// them all.
///
/// **Separate from [`super::load_strings`] on purpose**: that resolves one
/// table for the whole front end and this resolves one *column* of it, because
/// on Wipeout HD the copy that names the circuits and the copy the front end is
/// otherwise served are different files. The rest of the menus keep the served
/// table - the two copies differ on 42 shared keys in english and 72 in german,
/// and none of those differences is about a circuit - so this changes the RACE
/// page and nothing else. [`crate::language::CircuitNames`] carries the
/// measurement and the reasoning.
///
/// `language` is the same one [`super::load_strings`] chose, passed in rather
/// than re-derived so the circuit names and the rest of the front end cannot
/// end up in two different languages. `strings` is that language's served
/// table, for the fallback and for the reverse marker - see
/// [`crate::catalogue::label`].
pub(super) fn load_circuit_names(
    archives: &mut oag_assets::Archives,
    language: Option<&crate::language::Language>,
    strings: &crate::language::StringTable,
    tracks: &[crate::catalogue::Track],
    report: &mut Vec<String>,
) -> crate::language::CircuitNames {
    let names = choose_circuit_names(archives, language, tracks, report);
    if tracks.is_empty() {
        return names;
    }
    // The rows themselves, once, and unconditionally - a source that resolved
    // none of them is exactly the source whose list is worth seeing. The count
    // above says how many resolved; this is what a reader checks it against, and
    // on a source carrying several copies of the table it is the only place the
    // chosen copy can be seen to have been the right one.
    report.push(format!(
        "circuits: {}",
        tracks
            .iter()
            .map(|track| format!(
                "{} ({})",
                crate::catalogue::label(track, &names, strings, tracks),
                track.id
            ))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    names
}

/// Which copy of the table names them all, and what it says.
fn choose_circuit_names(
    archives: &mut oag_assets::Archives,
    language: Option<&crate::language::Language>,
    tracks: &[crate::catalogue::Track],
    report: &mut Vec<String>,
) -> crate::language::CircuitNames {
    use crate::language::{CircuitNames, StringTable};

    let Some(entries) = language.and_then(|l| l.entries.as_deref()) else {
        return CircuitNames::default();
    };
    if tracks.is_empty() {
        return CircuitNames::default();
    }

    let copies: Vec<(String, StringTable)> = archives
        .read_every_name(entries)
        .into_iter()
        .filter_map(|(label, blob)| {
            let xml = expand(&blob).ok()?;
            Some((label, StringTable::from_xml(&xml)))
        })
        .collect();

    let ids: Vec<String> = tracks.iter().map(|track| track.id.clone()).collect();
    let Some(names) = CircuitNames::choose(&copies, &ids) else {
        // Said out loud rather than left to look like the table simply had no
        // entry: with several copies on the source, "none of them names all of
        // them" is a different fact from "this one does not", and the circuits
        // fall back to their ids either way.
        report.push(format!(
            "{entries}: none of the {} copy/copies names all {} circuit(s); \
             each one shows its id",
            copies.len(),
            ids.len()
        ));
        return CircuitNames::default();
    };

    report.push(format!(
        "{entries}: {} circuit name(s) from {}, of {} copy/copies on this source",
        names.len(),
        names.source(),
        copies.len()
    ));
    names
}
