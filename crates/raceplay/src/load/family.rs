//! An opponent flies the player's variant family, on its own variant's tuning.
//!
//! The family is [`oag_title::TeamVariants::opponent_families`], title data:
//! HD/Fury declares HD and Fury as two, so a player on an HD hull faces HD
//! hulls and a player on `_c1`/`_n1` faces Fury ones. A title that declares
//! none is untouched by everything here. **Chosen, not measured.**

use super::handling;
use oag_core::rng::Rng;

/// Salt for the Concept-or-Nitro draw, so it never shares a stream with the
/// roster shuffle.
const FAMILY_SALT: u64 = 0x4641_4d49_4c59_5f31;

/// The title's variant table when it declares families, else `None`.
fn table(race: &oag_title::RaceDefaults) -> Option<&'static oag_title::TeamVariants> {
    race.team_variants
        .filter(|variants| !variants.opponent_families.is_empty())
}

/// `available` without the player's own team in any of its variants, so a
/// player on `Feisar_c1` never faces `Feisar`. Unchanged for a title with no
/// families (its own exact-id filter in `oag_livery::teams_for_slots` stands).
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

/// Moves every AI slot onto the player's family: its base team kept, its
/// suffix drawn from the family (the one member, or Concept or Nitro off the
/// race seed and the slot). A slot the table does not recognise is left alone.
pub(super) fn follow_player(
    race: &oag_title::RaceDefaults,
    slot_teams: &mut [String],
    seed: u64,
    report: &mut Vec<String>,
) {
    let Some(variants) = table(race) else { return };
    let Some(player) = slot_teams.first() else {
        return;
    };
    let Some((_, suffix)) = variants.split(player) else {
        return;
    };
    let Some(family) = variants.family_of(suffix) else {
        return;
    };
    for (slot, team) in slot_teams.iter_mut().enumerate().skip(1) {
        let Some((base, _)) = variants.split(team) else {
            continue;
        };
        let mut rng = Rng::new(seed ^ FAMILY_SALT ^ slot as u64);
        let pick = family[rng.below(family.len() as u32) as usize];
        *team = variants.join.combine(base, pick);
    }
    report.push(format!(
        "opponents follow the player's variant family {family:?} (chosen, not measured): {}",
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
                oag_gameplay::handling_for(&stats, class, pad_tunables, special)
                    .ok_or_else(|| format!("authors no <Class name=\"{class}\">"))
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
