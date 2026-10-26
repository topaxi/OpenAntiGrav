//! Which `.dat` a race's player craft flies: the declared path out of the
//! source's own definition and every mounted pack's manifest.
//!
//! The half of the skin feature that needs the catalogue; putting the skin on
//! the hull is `oag_livery::ship_skin::apply`, whose module docs carry the
//! evidence and the "chosen, not measured" note this inherits.

use oag_race::Mode;

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
pub(super) fn resolve(
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
    // An Eliminator forces its own skin on every craft, whatever was asked: see [`forced`].
    if mode == Mode::Eliminator && title.race.eliminator_skin.is_some() {
        report.push(format!(
            "--skin {asked}: ignored on an Eliminator race - every craft flies its forced skin"
        ));
        return None;
    }
    let documents = documents(archives, title, &format!("--skin {asked}"), report);
    declared(&documents, team, asked, report)
}

/// The title's own definition and every mounted pack's manifest, as text. A definition that will
/// not read is named in `report` under `what` and left out.
fn documents(
    archives: &mut oag_assets::Archives,
    title: &'static oag_title::Title,
    what: &str,
    report: &mut Vec<String>,
) -> Vec<String> {
    let definition = title.plugin_definition;
    let mut documents = Vec::new();
    match archives
        .read_name(definition)
        .map_err(|error| error.to_string())
        .and_then(|blob| oag_tables::fexml::text(&blob).map_err(|error| error.to_string()))
    {
        Ok(xml) => documents.push(xml),
        Err(error) => report.push(format!(
            "{what}: {definition} is unreadable ({error}) - no skin"
        )),
    }
    documents.extend(archives.manifests.iter().cloned());
    documents
}

/// `(team id, archive entry)` of the skin an Eliminator race forces on every craft
/// ([`oag_title::RaceDefaults::eliminator_skin`]), for each team that declares it. Empty outside
/// Eliminator and for a title that forces none. A team that declares no such skin is absent, so
/// it races the plain paint and the report says how many were found.
pub(super) fn forced(
    archives: &mut oag_assets::Archives,
    title: &'static oag_title::Title,
    mode: Mode,
    report: &mut Vec<String>,
) -> Vec<(String, String)> {
    let Some(name) = title
        .race
        .eliminator_skin
        .filter(|_| mode == Mode::Eliminator)
    else {
        return Vec::new();
    };
    let documents = documents(archives, title, "Eliminator skin", report);
    let out = forced_skins(&crate::catalogue::all_teams(&documents), name);
    report.push(format!(
        "Eliminator: every craft flies the plain hull in its {name} skin, as Ship_LoadModel's \
         case 8 does with no unlock checked ({} team(s) declare it)",
        out.len()
    ));
    out
}

/// [`forced`]'s half that needs no archive.
fn forced_skins(teams: &[crate::catalogue::Team], name: &str) -> Vec<(String, String)> {
    teams
        .iter()
        .filter_map(|team| Some((team.id.clone(), team.skin(name)?.location.clone())))
        .collect()
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

#[cfg(test)]
mod tests;
