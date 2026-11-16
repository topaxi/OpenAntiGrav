//! The circuit's baked probes: HD's `<track>.probes` and Omega's `<track>.shprobes`, what a hull's
//! ambient is blended from. See [`oag_vex::probes`], [`oag_vex::shprobes`].

use crate::env_probes::Baked;

/// The probes beside `track`: HD's by its own file, Omega's (a `.rcsmodel` of the PS4 kind) by
/// [`super::read_sh_probes`], and neither for any other title.
pub(in crate::load) fn read_probes(
    archives: &mut oag_assets::Archives,
    track: &str,
    geometry: Option<&[u8]>,
    report: &mut Vec<String>,
) -> Baked {
    let ps4 = super::GeometryKind::of(geometry) == super::GeometryKind::Ps4;
    Baked {
        hd: read_hd_probes(archives, track, report),
        sh: super::read_sh_probes(archives, track, ps4, report),
    }
}

/// HD's probes beside `track`, or `None`.
///
/// **Silent where the circuit ships none**, which is every title but Wipeout HD: an
/// absence true of a whole title is not news. A file that is there and does not parse is
/// reported, and the hulls then keep the constant ambient.
fn read_hd_probes(
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
        // Wipeout 2048 ships a `.probes` of its own, little-endian with another header: not a
        // failure of this reader, a file of a layout it does not know.
        Err(oag_vex::probes::Error::Stride(word)) => {
            report.push(format!(
                "{name}: a different layout (second header word {word}, not HD's 16), not read; \
                 hulls keep the constant ambient"
            ));
            None
        }
        Err(e) => {
            report.push(format!("{name}: {e}; hulls keep the constant ambient"));
            None
        }
    }
}
