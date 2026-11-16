//! The loader report's `<StartBoost>` line: whose launch law this race flies.

/// Pushes the line for a title whose `<StartBoost>` rule has `origin` (`None`: the title flies no
/// authored launch boost) when the race has `boost` to fly.
pub(super) fn say(origin: Option<oag_title::Origin>, boost: bool, report: &mut Vec<String>) {
    match (origin, boost) {
        (Some(oag_title::Origin::Measured), true) => {
            report.push("<StartBoost>: the launch law is measured on this title".to_string());
        }
        (Some(oag_title::Origin::InheritedFrom(from)), true) => {
            report.push(format!(
                "<StartBoost>: the launch law is inherited from {from}, not measured on this title"
            ));
        }
        (Some(oag_title::Origin::Chosen), true) => {
            report.push("<StartBoost>: the launch law is chosen, not measured".to_string());
        }
        (None, _) => {
            report.push("<StartBoost>: this title flies no authored launch boost".to_string());
        }
        (Some(_), false) => {}
    }
}
