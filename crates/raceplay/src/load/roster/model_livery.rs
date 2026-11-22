//! The player's livery folder, out of the source's own plugin definition: the half of the
//! HD-era model repaint that needs the catalogue; the texture substitution is
//! `oag_livery::LiveryFolder`'s.

use oag_race::Mode;

/// The folder the player's craft `team` (a combined id, `Feisar_c1`) names in its materials, the
/// `texturelocation` of its directory's own model ([`crate::catalogue::Team::model_for_directory`]),
/// paired with `worn`. `None` outside a race that asks for one, in Zone (one shared hull), and
/// when the definition will not read or names no model in that directory - the last two say so
/// in `report`.
pub(super) fn resolve(
    archives: &mut oag_assets::Archives,
    title: &'static oag_title::Title,
    team: &str,
    worn: Option<&str>,
    mode: Mode,
    report: &mut Vec<String>,
) -> Option<(String, String)> {
    let worn = worn.filter(|_| oag_livery::entry::own_craft(title.race.ships(), mode).is_none())?;
    let definition = title.plugin_definition;
    let xml = match archives
        .read_name(definition)
        .map_err(|error| error.to_string())
        .and_then(|blob| oag_tables::fexml::text(&blob).map_err(|error| error.to_string()))
    {
        Ok(xml) => xml,
        Err(error) => {
            report.push(format!(
                "{definition} is unreadable ({error}) - {team} keeps its own livery, not {worn}"
            ));
            return None;
        }
    };
    let own = own_folder(&crate::catalogue::teams(&xml), team);
    if own.is_none() {
        report.push(format!(
            "{team}: the definition names no model in its directory with a texturelocation - it \
             keeps its own livery, not {worn}"
        ));
    }
    Some((own?, worn.to_string()))
}

/// [`resolve`]'s half that needs no archive.
fn own_folder(teams: &[crate::catalogue::Team], team: &str) -> Option<String> {
    let leaf = |path: &str| {
        path.rsplit(['\\', '/'])
            .next()
            .unwrap_or(path)
            .to_ascii_lowercase()
    };
    let wanted = leaf(team);
    teams.iter().find_map(|candidate| {
        let model = candidate
            .models
            .iter()
            .find(|model| leaf(&model.model_location) == wanted)?;
        candidate
            .model_for_directory(&model.model_location)?
            .texture_location
            .clone()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const XML: &str = r#"<Screen>
<PI_Team name="Feisar"><Values type="Race" location="Data\Ships\Feisar"/>
<PI_TeamModel name="chrome_c1"><Values texturelocation="livery4" modellocation="Data\Ships\Feisar_c1"/><FE speed="8"/><Unlock MedalCountFURY="80"/></PI_TeamModel>
<PI_TeamModel name="concept1"><Values texturelocation="livery1" modellocation="Data\Ships\Feisar_c1"/><FE speed="8"/></PI_TeamModel>
<PI_TeamModel name="normal"><Values texturelocation="livery1" modellocation="Data\Ships\Feisar"/><FE speed="7"/></PI_TeamModel>
<PI_TeamModel name="SKIN1"><Values texturelocation="livery2" modellocation="Data\Ships\Feisar"/><FE speed="7"/><Unlock team="Feisar" loyalty="4000"/></PI_TeamModel>
</PI_Team></Screen>"#;

    /// The own folder is the directory's unlocked model's, whichever directory the craft flies.
    #[test]
    fn the_own_folder_is_the_directorys_own_models() {
        let teams = crate::catalogue::teams(XML);
        assert_eq!(own_folder(&teams, "Feisar").as_deref(), Some("livery1"));
        assert_eq!(own_folder(&teams, "Feisar_c1").as_deref(), Some("livery1"));
        assert_eq!(own_folder(&teams, "Qirex"), None);
    }
}
