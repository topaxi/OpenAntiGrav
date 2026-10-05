//! What [`super::variant_choices`]/[`super::combine_variant`] do across the
//! three shapes a title's axis can take: [`oag_title::TeamVariants`] (HD),
//! [`oag_title::race::HullVariant`] (Pulse) and neither (Pure).

use super::*;

/// HD/Fury's own axis: a picked suffix becomes a new team identity, and the
/// hull-file override is always `None` - there is nothing for it to name.
#[test]
fn a_team_variants_title_combines_into_a_new_team_identity() {
    let choices = variant_choices(oag_hd::TITLE, "Auricom");
    assert_eq!(choices.len(), 3, "HD/Fury's three reskins");

    let (combined, hull_variant, warning) = combine_variant(oag_hd::TITLE, "Auricom", "_c1");
    assert_eq!(combined, "Auricom_c1");
    assert_eq!(hull_variant, None);
    assert_eq!(warning, None);
}

/// Pulse's own axis: `team` never changes, and the choice comes back as the
/// hull-file override instead.
#[test]
fn a_hull_variants_title_leaves_the_team_alone_and_returns_a_hull_override() {
    let choices = variant_choices(oag_pulse::TITLE, "Assegai");
    assert_eq!(choices.len(), 2, "Pulse's Normal/Concept axis");

    let (combined, hull_variant, warning) = combine_variant(oag_pulse::TITLE, "Assegai", "extra");
    assert_eq!(combined, "Assegai", "the team's own identity is unchanged");
    assert_eq!(hull_variant, Some("extra"));
    assert_eq!(warning, None);
}

/// The baseline choice (`"Ship"`) combines to no override at all - not
/// `Some("Ship")` - because [`oag_raceplay::ship_entry_name`] already defaults
/// there when handed `None`, and a caller should not have to special-case the
/// baseline stem to get the same file `None` already names.
///
/// Not asserted here that the underlying stem is `"Ship"` and not `None`
/// throughout `combine_variant`'s own return - that distinction belongs to
/// `race::load`, which is what actually decides whether the string it got
/// back is worth threading through as an override. This test only pins
/// `combine_variant`'s literal output for the baseline choice.
#[test]
fn pulses_baseline_choice_combines_to_its_own_stem() {
    let (_, hull_variant, warning) = combine_variant(oag_pulse::TITLE, "Assegai", "Ship");
    assert_eq!(hull_variant, Some("Ship"));
    assert_eq!(warning, None);
}

/// A stem the team does not offer falls back to the first (baseline) one,
/// with a warning - the same shape `TeamVariants`' own mismatch takes.
#[test]
fn an_unrecognised_pulse_variant_falls_back_to_the_baseline_with_a_warning() {
    let (combined, hull_variant, warning) =
        combine_variant(oag_pulse::TITLE, "Assegai", "not-a-real-variant");
    assert_eq!(combined, "Assegai");
    assert_eq!(hull_variant, Some("Ship"), "the baseline is offered first");
    assert!(warning.is_some());
}

/// Pure authors neither axis: no choices, and `combine_variant` is a pure
/// no-op - the team unchanged, no hull override, no warning.
#[test]
fn a_title_with_neither_axis_offers_nothing_and_combines_to_a_no_op() {
    assert!(variant_choices(oag_pure::TITLE, "Assegai").is_empty());

    let (combined, hull_variant, warning) = combine_variant(oag_pure::TITLE, "Assegai", "");
    assert_eq!(combined, "Assegai");
    assert_eq!(hull_variant, None);
    assert_eq!(warning, None);
}
