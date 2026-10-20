//! An opponent flies the player's exact variant, in its own team's file.
//!
//! The rule is [`oag_title::TeamVariants::opponents_match_player_variant`],
//! title data: HD/Fury's player on `_c1` faces `_c1` hulls, on `_n1` faces
//! `_n1`, on the unsuffixed HD hull faces HD. Each slot's own handling loads on
//! every title, whatever the rule.

use super::handling;

/// The title's variant table when it declares the rule, else `None`.
fn table(race: &oag_title::RaceDefaults) -> Option<&'static oag_title::TeamVariants> {
    race.team_variants
        .filter(|variants| variants.opponents_match_player_variant)
}

/// `available` without the player's own team in any of its variants, so a
/// player on `Feisar_c1` never faces `Feisar`. Unchanged for a title with no
/// families (its own exact-id filter in `oag_race::roster::teams_for_slots` stands).
pub(super) fn pool(
    race: &oag_title::RaceDefaults,
    player: &str,
    available: &[String],
) -> Vec<String> {
    let Some(variants) = table(race) else {
        return available.to_vec();
    };
    let Some((base, _)) = variants.split(player) else {
        return available.to_vec();
    };
    available
        .iter()
        .filter(|team| variants.split(team).is_none_or(|(b, _)| b != base))
        .cloned()
        .collect()
}

/// Moves every AI slot onto the player's exact variant: its base team kept, the
/// player's suffix applied. A slot the table does not recognise is left alone,
/// and so is a seated person's (`seated` slots after the first): they picked
/// their own variant.
pub(super) fn follow_player(
    race: &oag_title::RaceDefaults,
    slot_teams: &mut [String],
    seated: usize,
    report: &mut Vec<String>,
) {
    let Some(variants) = table(race) else { return };
    let Some(player) = slot_teams.first() else {
        return;
    };
    let Some((_, suffix)) = variants.split(player) else {
        return;
    };
    for team in slot_teams.iter_mut().skip(1 + seated) {
        if let Some((base, _)) = variants.split(team) {
            *team = variants.join.combine(base, suffix);
        }
    }
    report.push(format!(
        "opponents fly the player's variant {suffix:?} in their own team's directory \
         (static read of the executable, confidence 55): {}",
        slot_teams.join(", ")
    ));
}

/// Each AI slot's own handling on every title, `None` where it flies the
/// player's (the slot's team is the player's, or its file would not load - said
/// in the report, never invented). The class, pad tunables and `<Special>` are the
/// race's, as they are for the player.
pub(super) fn opponent_handling(
    archives: &mut oag_assets::Archives,
    race: &oag_title::RaceDefaults,
    slot_teams: &[String],
    class: &str,
    pad_tunables: handling::SpeedupPads,
    special: handling::Special,
    report: &mut Vec<String>,
) -> Vec<Option<oag_physics::Handling>> {
    let mut out = vec![None; slot_teams.len()];
    let Some(player) = slot_teams.first() else {
        return out;
    };
    for (slot, team) in slot_teams.iter().enumerate().skip(1) {
        if team == player {
            continue;
        }
        let name = handling::entry_name_in(race.handling_dir_for(team), team);
        let loaded = archives
            .read_name(&name)
            .map_err(|e| e.to_string())
            .and_then(|blob| handling::from_blob(&blob).map_err(|e| e.to_string()))
            .and_then(|stats| {
                let mut handling = oag_gameplay::handling_for(&stats, class, pad_tunables, special)
                    .ok_or_else(|| format!("authors no <Class name=\"{class}\">"))?;
                super::code_laws::apply(&mut handling, &stats, class, race.craft_laws);
                Ok(handling)
            });
        match loaded {
            Ok(handling) => out[slot] = Some(handling),
            Err(error) => report.push(format!(
                "{name}: {error} - slot {slot} flies the player's tuning instead"
            )),
        }
    }
    out
}
