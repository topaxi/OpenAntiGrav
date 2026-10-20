//! The circuit's baked environment probes, `<track>.probes`: what a Wipeout HD hull's
//! ambient is blended from. See [`oag_vex::probes`].

/// The probes beside `track`, or `None`.
///
/// **Silent where the circuit ships none**, which is every title but Wipeout HD: an
/// absence true of a whole title is not news. A file that is there and does not parse is
/// reported, and the hulls then keep the constant ambient.
pub(in crate::load) fn read_probes(
    archives: &mut oag_assets::Archives,
    track: &str,
    report: &mut Vec<String>,
) -> Option<oag_vex::probes::Probes> {
    let at = track.len().checked_sub(4)?;
    if !track[at..].eq_ignore_ascii_case(".vex") {
        return None;
    }
    let name = format!("{}.probes", &track[..at]);
    let blob = archives.read_name(&name).ok()?;
    match oag_vex::probes::Probes::parse(&blob) {
        Ok(probes) => {
            report.push(format!(
                "{name}: {} environment probes; hulls lit from them",
                probes.len()
            ));
            Some(probes)
        }
        Err(e) => {
            report.push(format!("{name}: {e}; hulls keep the constant ambient"));
            None
        }
    }
}
