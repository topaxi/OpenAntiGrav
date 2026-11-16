//! The circuit's `track.shprobes` and the two weights beside it: what an Omega hull's ambient
//! is blended from. See [`crate::sh_probes`].

use super::{EnvSettings, envsettings_name, read_circuit_envsettings};
use crate::sh_probes::HullSh;
use oag_tables::envsettings::{OMEGA_SHIP_DIFFUSE_BRIGHTNESS, OMEGA_SHIP_DIFFUSE_SHADOW};

/// The probes beside `track` and the circuit's `Ship diffuse brightness` pair, or `None`.
///
/// **Omega only** (`ps4`): 2048 ships no `.shprobes` and HD's hull is lit from `.probes`. Silent
/// where the circuit ships none (the Zone circuits do not; their hulls keep the constant ambient,
/// and say so). A file that is there and does not parse, or a circuit that authors no
/// `Ship diffuse brightness` to weight it with, is reported: the executable's default for the
/// key is unread, so nothing is invented for it.
pub(in crate::load) fn read_sh_probes(
    archives: &mut oag_assets::Archives,
    track: &str,
    ps4: bool,
    report: &mut Vec<String>,
) -> Option<HullSh> {
    if !ps4 {
        return None;
    }
    let at = track.len().checked_sub(4)?;
    if !track[at..].eq_ignore_ascii_case(".vex") {
        return None;
    }
    let name = format!("{}.shprobes", &track[..at]);
    let blob = archives.read_name(&name).ok()?;
    let probes = match oag_vex::shprobes::ShProbes::parse(&blob) {
        Ok(probes) => probes,
        Err(e) => {
            report.push(format!("{name}: {e}; hulls keep the constant ambient"));
            return None;
        }
    };
    let settings = envsettings_name(track).and_then(|settings| {
        let (_, blob) = read_circuit_envsettings(archives, &settings)?;
        EnvSettings::parse(&String::from_utf8(blob).ok()?).ok()
    });
    let Some(k) = settings
        .as_ref()
        .and_then(|env| env.scalar(OMEGA_SHIP_DIFFUSE_BRIGHTNESS))
    else {
        report.push(format!(
            "{name}: {} spherical-harmonic probes, but the circuit authors no Ship diffuse \
             brightness to weight them; hulls keep the constant ambient",
            probes.len()
        ));
        return None;
    };
    let ks = settings
        .as_ref()
        .and_then(|env| env.scalar(OMEGA_SHIP_DIFFUSE_SHADOW))
        .unwrap_or(k);
    report.push(format!(
        "{name}: {} spherical-harmonic probes, weights K {k:.2} / Ks {ks:.2}; hulls lit from them",
        probes.len()
    ));
    Some(HullSh { probes, k, ks })
}
