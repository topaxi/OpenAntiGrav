//! What a ship screen writes into `race.skin`, `race.variant` and `race.variant_chosen`.
//!
//! **Nothing, on a row the mode fixes** ([`oag_game::livery_row::fixed_row`]): Zone's one hull and
//! an Eliminator race's forced skin are not the player's pick, so opening the screen, stepping
//! teams or confirming there must not overwrite the livery they saved for their next race (a Zone
//! race would otherwise save Zone's placeholder row as a deliberate model pick, and the next Single
//! Race would open on the classic hull instead of their Fury Concept). The team is still written:
//! that is the point of the screen.
//!
//! **Chosen, not measured.** The original's `TeamSelection_Update` registers the mode's own model
//! as `FE_TeamModel` (Zone's `zone01`, `Registry_Register` in `FUN_088ea7bc`); what that leaves
//! behind for the next normal race is untraced.

use oag_game::settings::Race;

use crate::picker_stage::LiveryAxis;

/// The screen opened on `variant`, the selected row's id: a bare Confirm races it, until the
/// player has picked a model of their own.
pub(crate) fn opened(race: &mut Race, axis: LiveryAxis, variant: Option<&str>, fixed: bool) {
    if fixed || axis != LiveryAxis::Variant || race.variant_chosen {
        return;
    }
    if let Some(id) = variant {
        race.variant = id.to_string();
    }
}

/// The player stepped to another team. Wipeout HD/Fury's screen keeps the model row across the
/// step (`across`, `Picker::land_on`) and every other picker restarts the livery.
pub(crate) fn stepped(
    race: &mut Race,
    axis: LiveryAxis,
    kept: Option<String>,
    across: bool,
    fixed: bool,
) {
    if fixed {
        return;
    }
    match (axis, kept) {
        (LiveryAxis::Skin, _) => race.skin = String::new(),
        (LiveryAxis::Variant, Some(id)) if across => race.variant = id,
        (LiveryAxis::Variant, _) => race.variant = String::new(),
    }
}

/// The player confirmed: the livery on screen is now their pick.
pub(crate) fn confirmed(race: &mut Race, fixed: bool) {
    if !fixed {
        race.variant_chosen = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A profile that picked its Fury Concept model and a Pulse skin before the Zone race.
    fn saved() -> Race {
        Race {
            variant: "_c1".to_string(),
            variant_chosen: true,
            skin: "Alternative".to_string(),
            ..Race::default()
        }
    }

    /// **The regression this module is for.** One Zone race through Ship Select must leave the
    /// saved model and skin alone, on both axes: the open, a team step and the confirm.
    #[test]
    fn a_fixed_row_leaves_the_saved_livery_alone() {
        for axis in [LiveryAxis::Variant, LiveryAxis::Skin] {
            for across in [false, true] {
                let mut race = saved();
                opened(&mut race, axis, Some(""), true);
                stepped(&mut race, axis, Some(String::new()), across, true);
                confirmed(&mut race, true);
                assert_eq!(race.variant, "_c1", "{axis:?} across {across}");
                assert_eq!(race.skin, "Alternative", "{axis:?} across {across}");
                assert!(race.variant_chosen);
            }
        }
        let mut fresh = Race::default();
        confirmed(&mut fresh, true);
        assert!(!fresh.variant_chosen, "a fixed row is not a model pick");
    }

    /// And the open row's own rules still hold where the player picks the livery.
    #[test]
    fn a_picked_row_writes_as_it_always_did() {
        let mut fresh = Race::default();
        opened(&mut fresh, LiveryAxis::Variant, Some("_n1"), false);
        assert_eq!(fresh.variant, "_n1", "what the screen opened on");
        let mut chosen = saved();
        opened(&mut chosen, LiveryAxis::Variant, Some("_n1"), false);
        assert_eq!(chosen.variant, "_c1", "a model the player picked wins");

        let mut stepping = saved();
        stepped(&mut stepping, LiveryAxis::Variant, Some("_n1".to_string()), true, false);
        assert_eq!(stepping.variant, "_n1", "HD keeps the model row");
        stepped(&mut stepping, LiveryAxis::Variant, Some("_n1".to_string()), false, false);
        assert_eq!(stepping.variant, "", "every other picker restarts it");
        stepped(&mut stepping, LiveryAxis::Skin, None, false, false);
        assert_eq!(stepping.skin, "");

        let mut confirming = Race::default();
        confirmed(&mut confirming, false);
        assert!(confirming.variant_chosen);
    }
}
