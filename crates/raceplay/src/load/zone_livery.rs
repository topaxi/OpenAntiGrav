//! Each craft's Zone or Detonator livery, out of the source's own plugin definition.
//!
//! The half of the Zone livery swap that needs the catalogue; substituting the
//! texture on the hull is `oag_livery`'s, whose `zone_livery` module carries
//! the evidence.

use oag_race::Mode;

/// `(team id, livery name)` for every team the definition gives a Zone livery (a Detonator
/// race: its `detonator` model's), each variant of it included (a Fury `_c1` craft wears its
/// base team's). Empty in any other mode, for a title whose own hull has no livery key, and
/// when the definition will not read - the last says so in `report`.
pub(super) fn resolve(
    archives: &mut oag_assets::Archives,
    title: &'static oag_title::Title,
    mode: Mode,
    report: &mut Vec<String>,
) -> Vec<(String, String)> {
    if oag_livery::entry::own_craft(title.race.ships(), mode)
        .and_then(|craft| craft.livery_key())
        .is_none()
    {
        return Vec::new();
    }
    let definition = title.plugin_definition;
    let xml = match archives
        .read_name(definition)
        .map_err(|error| error.to_string())
        .and_then(|blob| oag_tables::fexml::text(&blob).map_err(|error| error.to_string()))
    {
        Ok(xml) => xml,
        Err(error) => {
            report.push(format!(
                "{definition} is unreadable ({error}) - no {} livery for any craft",
                oag_livery::entry::craft_label(mode)
            ));
            return Vec::new();
        }
    };
    let teams = crate::catalogue::teams(&xml);
    teams
        .iter()
        .flat_map(|team| team.craft_liveries(title.race, mode))
        .collect()
}

#[cfg(test)]
mod tests {
    use crate::catalogue::zone_liveries as liveries;

    #[test]
    fn a_variant_wears_its_base_teams_livery_and_a_team_with_none_is_absent() {
        let xml = r#"<Plugin>
          <PI_Team name="Feisar"><Values type="Race" location="Data\Ships\Feisar"/>
            <PI_TeamModel name="zone"><Values texturelocation="zoneship_faisar"/></PI_TeamModel></PI_Team>
          <PI_Team name="Feisar2048"><Values type="Race" location="Data\Ships\Feisar2048"/></PI_Team>
        </Plugin>"#;
        let teams = crate::catalogue::teams(xml);
        let out = liveries(&teams, oag_2048::race::DEFAULTS);
        assert!(out.contains(&("Feisar".to_string(), "zoneship_faisar".to_string())));
        assert!(out.contains(&("Feisar_c1".to_string(), "zoneship_faisar".to_string())));
        assert!(!out.iter().any(|(id, _)| id.starts_with("Feisar2048")));
    }
}
