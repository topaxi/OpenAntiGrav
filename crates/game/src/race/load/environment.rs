//! The circuit's staging: its sky, its light rig and its distance fog.
//!
//! Split out of `load.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. The three
//! readers share one shape - a sibling-path rewrite from the track `.vex`,
//! every failure reported, every absence honest rather than substituted.

use super::*;

/// The `sky.gtf` beside a circuit's `.vex` - Wipeout HD's sky.
///
/// One per circuit *directory*, not per `.vex`: `track.vex` and
/// `track_reversed.vex` both resolve to the same file, which is how the disc
/// ships it (16 environments, 16 `sky.gtf`).
fn sky_gtf_name(vex_name: &str) -> Option<String> {
    let at = vex_name.rfind(['/', '\\'])?;
    Some(format!("{}{}sky.gtf", &vex_name[..at], &vex_name[at..=at]))
}

/// Wipeout HD's sky, or `None` with the reason in the report.
///
/// The rotation applied is the circuit's own `Lighting.Sky rotation`, read as
/// degrees - `mesh::sky_cube::build` records why that unit is the corpus's
/// rather than a guess, and why the sign is still this project's choice. A
/// missing or unreadable `.envsettings` leaves the sky unrotated rather than
/// undrawn: the picture is data, the turn is presentation.
pub(super) fn hd_sky_model(
    archives: &mut oag_assets::Archives,
    track: &str,
    report: &mut Vec<String>,
) -> Option<mesh::Model> {
    let name = sky_gtf_name(track)?;
    let Ok(blob) = archives.read_name(&name) else {
        report.push(format!(
            "{name}: not in the archive set - the sky stays black"
        ));
        return None;
    };
    let rotation = envsettings_name(track)
        .and_then(|name| archives.read_name(&name).ok())
        .and_then(|blob| String::from_utf8(blob).ok())
        .and_then(|text| oag_formats::envsettings::EnvSettings::parse(&text).ok())
        .and_then(|env| env.scalar(oag_formats::envsettings::SKY_ROTATION))
        .unwrap_or(0.0);
    match mesh::sky_cube::build(&name, &blob, rotation) {
        Ok(sky) => {
            let (width, height) = sky
                .textures
                .first()
                .and_then(Option::as_ref)
                .map_or((0, 0), |t| (t.width, t.height));
            report.push(format!(
                "{name}: the circuit's sky, six {width}x{height} cubemap face(s) on a \
                 camera-centred cube, turned {rotation:.0} degree(s) by the authored Sky \
                 rotation (unit read off the corpus; sign and axis this project's)"
            ));
            Some(sky)
        }
        Err(error) => {
            report.push(format!("{name}: {error:#} - the sky stays black"));
            None
        }
    }
}

/// The name of the `.envsettings` beside a track `.vex`, or `None`.
///
/// The rewrite the disc's own pairing uses:
/// `/data/environments/talons_junction/track.vex` sits beside
/// `track.envsettings`. Case-insensitive on the suffix for the same reason
/// [`mesh::rcs::sibling_name`] is - the game's own spelling varies between
/// containers.
fn envsettings_name(vex_name: &str) -> Option<String> {
    let at = vex_name.len().checked_sub(4)?;
    vex_name[at..]
        .eq_ignore_ascii_case(".vex")
        .then(|| format!("{}.envsettings", &vex_name[..at]))
}

/// The distance fog a Wipeout HD circuit authors, or `None` with the reason
/// reported.
///
/// Reads `Fog.Fog Color` and `Fog.Fog Density` from the same `.envsettings`
/// [`envsettings_light`] reads. What is the disc's and what is not:
///
/// - **The curve is the disc's**, read out of the circuit materials' own
///   fragment microcode - see [`mesh_render::Fog::curve`].
/// - **The density reaching the shader unscaled is this project's reading.**
///   The microcode takes its coefficient from a patched `fogColour.w`; how the
///   engine fills that from `Fog.Fog Density` is unread, so the authored value
///   is passed through and judged against an rpcs3 reference frame.
/// - `Alternate Fog Color`/`Density` exist on every circuit and what selects
///   them is unread; the primary pair is used and the alternates are not.
pub(super) fn envsettings_fog(
    archives: &mut oag_assets::Archives,
    track: &str,
    report: &mut Vec<String>,
) -> Option<mesh_render::Fog> {
    use oag_formats::envsettings::{EnvSettings, FOG_COLOUR, FOG_DENSITY};
    let name = envsettings_name(track)?;
    let blob = archives.read_name(&name).ok()?;
    let text = String::from_utf8(blob).ok()?;
    let env = EnvSettings::parse(&text).ok()?;
    let (Some(colour), Some(density)) = (env.vec3(FOG_COLOUR), env.scalar(FOG_DENSITY)) else {
        report.push(format!(
            "{name}: no usable fog colour and density; the race draws unfogged"
        ));
        return None;
    };
    if density <= 0.0 {
        report.push(format!(
            "{name}: fog density {density}; the race draws unfogged"
        ));
        return None;
    }
    report.push(format!(
        "{name}: fog [{:.2}, {:.2}, {:.2}] at density {density}, on the curve read from the \
         circuit's own fragment microcode: exp(-(density * view_depth)^2)",
        colour[0], colour[1], colour[2],
    ));
    Some(mesh_render::Fog::authored_exp2(colour, density))
}

/// The `HDR and Bloom` values a Wipeout HD circuit authors, or `None` with
/// the reason reported.
///
/// These are the parameters the engine patches into the read
/// `FunkLayerBloom` gate and blur programs - the formulas live in
/// `oag_render::post::hd_bloom`, every one of them the microcode's own. A
/// file without the whole set draws without the chain rather than with a
/// guessed half of it, and says so.
pub(super) fn envsettings_bloom(
    archives: &mut oag_assets::Archives,
    track: &str,
    report: &mut Vec<String>,
) -> Option<oag_render::post::hd_bloom::Params> {
    use oag_formats::envsettings::{
        BLOOM_ADAPTION_BOOST, BLOOM_ADAPTION_RATE, BLOOM_ALPHA_CONTRIBUTION,
        BLOOM_FRAME_CONTRIBUTION, BLOOM_FRAME_EXPONENT, BLOOM_HORIZONTAL_SIZE, BLOOM_VERTICAL_SIZE,
        EnvSettings, TONE_ADAPTION_BOOST, TONE_DARKENING_CLAMP, TONE_MAXIMUM_BRIGHTNESS,
    };
    let name = envsettings_name(track)?;
    let blob = archives.read_name(&name).ok()?;
    let text = String::from_utf8(blob).ok()?;
    let env = EnvSettings::parse(&text).ok()?;
    let (
        Some(alpha_contribution),
        Some(frame_contribution),
        Some(frame_exponent),
        Some(horizontal_size),
        Some(vertical_size),
        Some(adaption_rate),
        Some(adaption_boost),
        Some(tone_adaption_boost),
        Some(tone_darkening_clamp),
        Some(tone_maximum_brightness),
    ) = (
        env.scalar(BLOOM_ALPHA_CONTRIBUTION),
        env.scalar(BLOOM_FRAME_CONTRIBUTION),
        env.scalar(BLOOM_FRAME_EXPONENT),
        env.scalar(BLOOM_HORIZONTAL_SIZE),
        env.scalar(BLOOM_VERTICAL_SIZE),
        env.scalar(BLOOM_ADAPTION_RATE),
        env.scalar(BLOOM_ADAPTION_BOOST),
        env.scalar(TONE_ADAPTION_BOOST),
        env.scalar(TONE_DARKENING_CLAMP),
        env.scalar(TONE_MAXIMUM_BRIGHTNESS),
    )
    else {
        report.push(format!(
            "{name}: no complete HDR and Bloom block; the race draws without the read \
             bloom chain"
        ));
        return None;
    };
    report.push(format!(
        "{name}: bloom gate alpha x{alpha_contribution}, lum^{frame_exponent} \
         x{frame_contribution} faded by adaptation (rate {adaption_rate}, boost \
         {adaption_boost}), blur steps {horizontal_size}/{vertical_size}, exposure \
         {tone_maximum_brightness} - min(adapted x{tone_adaption_boost}, \
         {tone_darkening_clamp}) - formulas read from the executable's own \
         FunkLayerBloom microcode and its PPU chain runner"
    ));
    Some(oag_render::post::hd_bloom::Params {
        alpha_contribution,
        frame_contribution,
        frame_exponent,
        horizontal_size,
        vertical_size,
        adaption_rate,
        adaption_boost,
        tone_adaption_boost,
        tone_darkening_clamp,
        tone_maximum_brightness,
    })
}

/// The light rig a circuit authors, or [`mesh_render::Light::stand_in`].
///
/// **Every way this can fail leaves the stand-in and says so in the report.**
/// A track that authors no settings file - every Pulse, Pure and PS2 circuit -
/// is silent, because an absence that is true of a whole title is not news; a
/// file that is there and does not yield a rig is reported, because that is a
/// gap in this reading rather than in the data.
pub(super) fn envsettings_light(
    archives: &mut oag_assets::Archives,
    track: &str,
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
    let env = match oag_formats::envsettings::EnvSettings::parse(&text) {
        Ok(env) => env,
        Err(e) => {
            report.push(format!("{name}: {e}; lighting with the stand-in rig"));
            return mesh_render::Light::stand_in();
        }
    };
    use oag_formats::envsettings::{
        AMBIENT_COLOUR, PRELIT_POWER, PRELIT_SCALE, SUN_COLOUR, SUN_DIRECTION, SUN_SPECULAR_SCALE,
    };
    let (Some(direction), Some(colour), Some(ambient)) = (
        env.direction(SUN_DIRECTION),
        env.vec3(SUN_COLOUR),
        env.vec3(AMBIENT_COLOUR),
    ) else {
        report.push(format!(
            "{name}: no usable sun direction, colour and ambient; lighting with the \
             stand-in rig"
        ));
        return mesh_render::Light::stand_in();
    };
    // The prelit curve and the specular weight feed terms whose *combination*
    // is read out of the circuit's own fragment microcode - see
    // `mesh_render::Light`. A file missing one of them gets the identity for
    // that term rather than the stand-in rig: the sun and ambient above are
    // still the circuit's.
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
    report.push(format!(
        "{name}: sun [{:.2}, {:.2}, {:.2}] colour [{:.2}, {:.2}, {:.2}] over ambient \
         [{:.2}, {:.2}, {:.2}], prelit {:.1}*lightmap^{:.1}, specular x{:.2} - the \
         combination is the microcode's own; the render target's saturation stands in \
         for HD's tonemap",
        direction[0],
        direction[1],
        direction[2],
        light.sun[0],
        light.sun[1],
        light.sun[2],
        light.ambient[0],
        light.ambient[1],
        light.ambient[2],
        prelit_scale[0],
        prelit_power[0],
        specular_scale,
    ));
    light
}
