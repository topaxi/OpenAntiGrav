//! The one switch a circuit has over Wipeout HD's SPU vertex lights.
//!
//! `Environment_RegisterLightingSchema` (`0x003a83d8` in `ps3-hdfury-eu`)
//! registers `"Lighting.Enable spu vertex lights"` onto the settings byte
//! `Enable_spu_vertex_light` (`+0x5a3`) **after writing `1` to it**, so a
//! circuit that never mentions the key runs with the lights on. Two do
//! mention it, and both turn it off: `02_track` and `12_sol_2`, in their
//! `track.envsettings` and `track_reversed.envsettings` alike - measured over
//! every `.envsettings` on the Fury disc. See
//! `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`, "`Enable_spu_vertex_light`
//! is read at 14 sites" (confidence 75 on the gate) and
//! `docs/formats/envsettings.md` for the key's integer formatting.
//!
//! Read off the circuit's own file rather than the staged front-end merge:
//! `/data/fe/fe.track.envsettings` does not author the key, so there is no
//! carried value to inherit and the registrar's default is the only fallback.

/// Whether this circuit's `.envsettings` leaves the SPU vertex lights on.
///
/// `true` on every title but Wipeout HD (the mechanism is HD's alone and the
/// light list is empty elsewhere anyway), on an HD circuit with no
/// `.envsettings`, and on one that does not author the key; `false` only on
/// an authored `0`.
pub(super) fn spu_vertex_lights_enabled(
    archives: &mut oag_assets::Archives,
    track: &str,
    ps3_geometry: bool,
    report: &mut Vec<String>,
) -> bool {
    if !ps3_geometry {
        return true;
    }
    let Some(name) = super::environment::envsettings_name(track) else {
        return true;
    };
    let Some(env) = super::environment::read_envsettings(archives, &name) else {
        return true;
    };
    match env.flag(KEY) {
        Some(false) => {
            report.push(format!(
                "{name}: \"{KEY}\"=0 - this circuit switches the engines' SPU vertex lights \
                 off, as 02_track and 12_sol_2 do"
            ));
            false
        }
        Some(true) => true,
        None => {
            report.push(format!(
                "{name}: no \"{KEY}\" key - the registrar's default leaves the engines' SPU \
                 vertex lights on"
            ));
            true
        }
    }
}

/// The key, as `docs/formats/envsettings.md` lists it.
const KEY: &str = "Lighting.Enable spu vertex lights";
