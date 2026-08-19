//! What a source offers to race on: the plugin definition, the circuits and
//! the roster.
//!
//! Split out of `boot.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change beyond the
//! one this split accompanied - the definition's entry name is the title's now
//! rather than Pulse's. See [`oag_title::Title::plugin_definition`].
//!
//! The three functions run in order and each is fed the one above it:
//! [`definitions`] reads the documents, [`load_tracks`] and [`load_teams`] read
//! the two catalogues out of them.

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
