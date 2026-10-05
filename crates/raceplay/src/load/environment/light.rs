//! The light rig a circuit's `.envsettings` authors. Split out of
//! `environment.rs` for its length; no behaviour moved.

use super::*;

/// The light rig a circuit authors, or [`mesh_render::Light::stand_in`].
///
/// **Every way this can fail leaves the stand-in and says so in the report.**
/// A track that authors no settings file - every Pulse, Pure and PS2 circuit -
/// is silent, because an absence that is true of a whole title is not news; a
/// file that is there and does not yield a rig is reported, because that is a
/// gap in this reading rather than in the data.
///
/// **`psp2` picks the key spelling, not the file.** Wipeout 2048 ships the
/// same `"Key.Subkey"=float [float...]` syntax beside its own `track.vex`, but
/// its registrar spells the ambient and sun-diffuse terms differently from
/// HD's - see `oag_tables::envsettings`'s "Wipeout 2048 authors the same
/// shape under different key names". Reading HD's keys against a 2048 file
/// resolves nothing and falls back to the stand-in rig silently wrong about
/// why; this is what tells the two schemas apart.
///
/// **Does not read through [`staged_envsettings`].** Checked for the same
/// partial-key gap `envsettings_bloom` had (`lane-envsettings-carry`): every
/// circuit that ships a `.envsettings` at all authors a complete sun
/// direction, colour and ambient itself, so there is nothing here for the
/// front end's file to carry. See [`staged_envsettings`]'s own doc for why the
/// remaining case, `zone_2`/`zone_3`/`zone_4` (which ship no file at all), is
/// left with the pre-existing stand-in rig rather than wired to the front
/// end's.
pub(in crate::load) fn envsettings_light(
    archives: &mut oag_assets::Archives,
    track: &str,
    psp2: bool,
    ps4: bool,
    report: &mut Vec<String>,
) -> mesh_render::Light {
    let Some(name) = envsettings_name(track) else {
        return mesh_render::Light::stand_in();
    };
    let Ok(blob) = archives.read_name(&name) else {
        return mesh_render::Light::stand_in();
    };
    let text = match String::from_utf8(blob) {
        Ok(text) => text,
        Err(_) => {
            report.push(format!("{name}: not text; lighting with the stand-in rig"));
            return mesh_render::Light::stand_in();
        }
    };
    let env = match EnvSettings::parse(&text) {
        Ok(env) => env,
        Err(e) => {
            report.push(format!("{name}: {e}; lighting with the stand-in rig"));
            return mesh_render::Light::stand_in();
        }
    };
    use oag_tables::envsettings::{
        AMBIENT_COLOUR, NOVA_PRELIT, NOVA_PRELIT_DEFAULT, PRELIT_POWER, PRELIT_SCALE,
        PSP2_AMBIENT_COLOUR, PSP2_SUN_DIFFUSE_COLOUR, SUN_COLOUR, SUN_DIRECTION,
        SUN_SPECULAR_SCALE,
    };
    let ambient_key = if psp2 {
        PSP2_AMBIENT_COLOUR
    } else {
        AMBIENT_COLOUR
    };
    let colour_key = if psp2 {
        PSP2_SUN_DIFFUSE_COLOUR
    } else {
        SUN_COLOUR
    };
    let (Some(direction), Some(colour), Some(ambient)) = (
        env.direction(SUN_DIRECTION),
        env.vec3(colour_key),
        env.vec3(ambient_key),
    ) else {
        report.push(format!(
            "{name}: no usable sun direction, colour and ambient; lighting with the \
             stand-in rig"
        ));
        return mesh_render::Light::stand_in();
    };
    // The prelit curve and the specular weight feed terms whose *combination*
    // is read out of the circuit's own fragment microcode on HD - see
    // `mesh_render::Light`. 2048 authors no equivalent keys at all (a genuine
    // RGB `Sun specular colour` rather than a scalar - see the module docs for
    // why that is left unwired rather than reduced to a scalar), so both
    // terms take the identity there, same as a file missing one of them: the
    // sun and ambient above are still the circuit's.
    let prelit_scale = env.vec3(PRELIT_SCALE).unwrap_or([1.0; 3]);
    let prelit_power = env.vec3(PRELIT_POWER).unwrap_or([1.0; 3]);
    let specular_scale = env.scalar(SUN_SPECULAR_SCALE).unwrap_or(0.0);
    let light = mesh_render::Light::authored(
        direction,
        colour,
        ambient,
        prelit_scale,
        prelit_power,
        specular_scale,
    );
    // **Omega's lightmap combination**, read out of its pixel shaders
    // (`docs/ghidra/functions/ps4-omega-eu/lightmap-prelit.md`): the authored
    // `Lighting.Nova prelit scale bias power` triple, or the executable's own
    // static default `(1.4, 0.2, 1.5)` for a file that omits it - 9 of the
    // title's 97 files do, none of them a circuit's.
    let nova = ps4.then(|| env.vec3(NOVA_PRELIT).unwrap_or(NOVA_PRELIT_DEFAULT));
    let light = match nova {
        Some([scale, bias, power]) => light.with_nova_prelit(scale, bias, power),
        None => light,
    };
    let prelit = match nova {
        Some([scale, bias, power]) => format!("{scale:.1}*lightmap^{power:.1} + {bias:.2}"),
        None => format!("{:.1}*lightmap^{:.1}", prelit_scale[0], prelit_power[0]),
    };
    let combination = if ps4 {
        "Omega's own: pow(lightmap, power) * scale + bias on the raw atlas, no constant \
         ambient on a lightmapped draw"
    } else if psp2 {
        "2048 authors no equivalent prelit or specular keys, so both stay at the identity"
    } else {
        "the combination is the microcode's own; the render target's saturation stands in \
         for HD's tonemap"
    };
    report.push(format!(
        "{name}: sun [{:.2}, {:.2}, {:.2}] colour [{:.2}, {:.2}, {:.2}] over ambient \
         [{:.2}, {:.2}, {:.2}], prelit {prelit}, specular x{:.2} - {combination}",
        direction[0],
        direction[1],
        direction[2],
        light.sun[0],
        light.sun[1],
        light.sun[2],
        light.ambient[0],
        light.ambient[1],
        light.ambient[2],
        specular_scale,
    ));
    light
}

/// Omega's `Tonemap` block, read into [`oag_post::omega_tonemap::Params`].
///
/// Its consumer is `ToneMap_ApplyEnvSettings` (`0x01620980`), which reads the
/// `Tonemap.*` keys, or `TonemapHDR.*` under HDR video out; this port draws
/// SDR, so it reads `Tonemap`. A circuit's own file only - every Omega circuit
/// file authors the block; a file without one draws without the curve, and
/// says so.
pub(in crate::load) fn envsettings_tonemap(
    archives: &mut oag_assets::Archives,
    track: &str,
    report: &mut Vec<String>,
) -> Option<oag_post::omega_tonemap::Params> {
    let name = envsettings_name(track)?;
    let blob = archives.read_name(&name).ok()?;
    let env = EnvSettings::parse(&String::from_utf8(blob).ok()?).ok()?;
    let Some(t) = env.tonemap("Tonemap") else {
        report.push(format!(
            "{name}: no complete Tonemap block; the race draws without Omega's curve"
        ));
        return None;
    };
    let params = oag_post::omega_tonemap::Params {
        luminance_a: t.luminance_a,
        luminance_b: t.luminance_b,
        exposure_minimum: t.exposure_minimum,
        exposure_maximum: t.exposure_maximum,
        exposure_response: t.exposure_response,
        exposure_time: t.exposure_time,
        source_end_a: t.source_colour_end_a,
        source_end_b: t.source_colour_end_b,
        start_angle: t.start_angle,
        end_angle: t.end_angle,
    };
    report.push(format!(
        "{name}: Tonemap applied - exposure ({:.3} + {:.3} LAvg) / LAvg in {:.2}..{:.2}, \
         LAvg the mean luma of the last {} frames stepped {:.4} a frame, cubic from 0 to \
         t1 = {:.3} + {:.3} LAvg with angles {:.1}/{:.1} deg - the executable's own law \
         (ToneMap_ApplyEnvSettings and its coefficient shader)",
        params.luminance_a,
        params.luminance_b,
        params.exposure_minimum,
        params.exposure_maximum,
        params.history_frames(),
        params.step(),
        params.source_end_a,
        params.source_end_b,
        params.start_angle,
        params.end_angle,
    ));
    Some(params)
}
