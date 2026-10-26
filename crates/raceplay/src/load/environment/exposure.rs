//! Wipeout 2048's exposure and bloom keys, read for
//! [`oag_post::wo_composite::Params`].

use super::*;

/// The circuit's `Lighting.Exposure*` and `Lighting.Bloom*` keys, or `None`
/// with the reason reported.
///
/// **An add-on pack's copy of the file wins over the base archive's.** The
/// base `data.psarc` ships the HD Campaign circuits' paths too, and its copies
/// disagree with the pack's: Vineta K's base copy says `ExposureScale` 0.6,
/// `ExposureMax` 12 and `BloomGate` 0.2, its `dlc1.psarc` copy 0.5, 8 and 0.3,
/// and the setters read the pack's live on that circuit (exposure.md, "The
/// setters carry the circuit's own file"). The base copies of Vineta K's and
/// Moa Therma's reversed files author none of the five keys at all. Scoped to
/// these keys: the light and fog readers still take
/// [`oag_assets::Archives::read_name`]'s copy.
///
/// The exposure always adapts here; a comparison's hold is laid over it by the
/// caller (`Options::exposure_hold`).
pub(in crate::load) fn envsettings_exposure(
    archives: &mut oag_assets::Archives,
    track: &str,
    title: &'static oag_title::Title,
    report: &mut Vec<String>,
) -> Option<oag_post::wo_composite::Params> {
    use oag_tables::envsettings::{
        BLOOM_FACTOR, BLOOM_GATE, EXPOSURE_MAX, EXPOSURE_SCALE, EXPOSURE_SPEED,
    };
    let name = envsettings_name(track)?;
    let copies = archives.read_every_name(&name);
    let from_pack = |label: &str| {
        let file = file_name(label);
        title
            .archives
            .add_ons
            .iter()
            .any(|add_on| file.eq_ignore_ascii_case(file_name(add_on.archive)))
    };
    let Some((label, blob)) = copies
        .iter()
        .find(|(label, _)| from_pack(label))
        .or_else(|| copies.first())
    else {
        report.push(format!(
            "{name}: not in the archive set; the race draws without 2048's composite"
        ));
        return None;
    };
    let env = String::from_utf8(blob.clone())
        .ok()
        .and_then(|text| EnvSettings::parse(&text).ok());
    let keys = env.as_ref().map(|env| {
        [
            EXPOSURE_SCALE,
            EXPOSURE_MAX,
            EXPOSURE_SPEED,
            BLOOM_FACTOR,
            BLOOM_GATE,
        ]
        .map(|key| env.scalar(key))
    });
    let Some(
        [
            Some(scale),
            Some(max),
            Some(speed),
            Some(factor),
            Some(gate),
        ],
    ) = keys
    else {
        report.push(format!(
            "{name} ({label}): no complete Lighting.Exposure/Bloom keys; the race draws \
             without 2048's composite"
        ));
        return None;
    };
    report.push(format!(
        "{name} ({label}): 2048's composite - exposure adapting to {scale} / mean \
         luminance at {speed} a frame, at most {max}; bloom x{factor} above a gate of \
         {gate} - wo_bloom_gate_fp, wo_blur_fp and wo_composite_nocc_fp as the \
         executable's programs compute them; vignette, screen tint and alphaTex unread"
    ));
    Some(oag_post::wo_composite::Params {
        exposure_scale: scale,
        exposure_max: max,
        exposure_speed: speed,
        bloom_factor: factor,
        bloom_gate: gate,
        exposure: oag_post::wo_composite::Exposure::Adapting,
    })
}

/// The last path component of an archive label or name, either separator.
fn file_name(path: &str) -> &str {
    path.rsplit(['/', '\\']).next().unwrap_or(path)
}
