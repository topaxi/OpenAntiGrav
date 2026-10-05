//! What [`super::resolve`] does across the three shapes an ask can meet:
//! nothing was asked, the title has the axis and offers the stem, or it
//! does not.

use super::*;

#[test]
fn nothing_asked_resolves_to_nothing_and_reports_nothing() {
    let mut report = Vec::new();
    assert_eq!(resolve(None, oag_pulse::TITLE, &mut report), None);
    assert!(report.is_empty());
}

#[test]
fn a_stem_the_title_offers_resolves_to_itself() {
    let mut report = Vec::new();
    assert_eq!(
        resolve(Some("extra"), oag_pulse::TITLE, &mut report),
        Some("extra")
    );
    assert!(report.is_empty(), "a recognised ask is not worth a line");
}

#[test]
fn a_stem_the_title_does_not_offer_falls_back_and_reports_it() {
    let mut report = Vec::new();
    assert_eq!(
        resolve(Some("not-a-real-stem"), oag_pulse::TITLE, &mut report),
        None
    );
    assert_eq!(report.len(), 1);
    assert!(report[0].contains("not-a-real-stem"));
}

#[test]
fn a_title_with_no_axis_at_all_falls_back_and_reports_it() {
    let mut report = Vec::new();
    assert_eq!(resolve(Some("extra"), oag_pure::TITLE, &mut report), None);
    assert_eq!(report.len(), 1);
    assert!(report[0].contains("no hull-variant axis"));
}
