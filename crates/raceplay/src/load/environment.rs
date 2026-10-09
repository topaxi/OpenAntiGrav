//! The circuit's staging: its sky, its light rig and its distance fog.
//!
//! Split out of `load.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change. The three
//! readers share one shape - a sibling-path rewrite from the track `.vex`,
//! every failure reported, every absence honest rather than substituted.

use super::*;
use oag_tables::envsettings::EnvSettings;

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
///
/// **A Zone race draws [`oag_title::RaceDefaults::zone_sky`] instead**, where
/// the title has one located. That is a replacement rather than a tint, and it
/// is what the original does: its per-race loader picks between the circuit's
/// sibling `sky.gtf` and `Data/Tex/ZoneSky.gtf` on one byte - set for four mode
/// ids, `Mode::Zone` being this port's stand-in for it - and the two branches
/// converge on the same loader call, the same handle and the same sampler
/// state, so the picture is the only thing that changes. The rotation is still
/// the circuit's. See `docs/ghidra/functions/ps3-hdfury-eu/zone-sky.md`.
pub(super) fn hd_sky_model(
    archives: &mut oag_assets::Archives,
    track: &str,
    race: &'static oag_title::RaceDefaults,
    mode: oag_race::Mode,
    report: &mut Vec<String>,
) -> Option<mesh::Model> {
    let zone_sky = (mode == oag_race::Mode::Zone)
        .then_some(race.zone_sky)
        .flatten();
    let name = match zone_sky {
        Some(zone) => zone.to_string(),
        None => sky_gtf_name(track)?,
    };
    let Ok(blob) = archives.read_name(&name) else {
        report.push(format!(
            "{name}: not in the archive set - the sky stays black"
        ));
        return None;
    };
    let env = envsettings_name(track)
        .and_then(|name| archives.read_name(&name).ok())
        .and_then(|blob| String::from_utf8(blob).ok())
        .and_then(|text| oag_tables::envsettings::EnvSettings::parse(&text).ok());
    let rotation = env
        .as_ref()
        .and_then(|env| env.scalar(oag_tables::envsettings::SKY_ROTATION))
        .unwrap_or(0.0);
    // The Zone branch of the original's sky draw does not take the circuit's
    // colour (renderer.md, "Lighting.Sky colour's consumer"), so a Zone sky
    // and a circuit with no authored colour draw untinted.
    let tint = match (zone_sky, env.as_ref()) {
        (None, Some(env)) => env
            .rgba8(oag_tables::envsettings::SKY_COLOUR)
            .map_or([1.0; 3], |c| {
                std::array::from_fn(|i| f32::from(c[i]) / mesh::sky_cube::SKY_COLOUR_FULL)
            }),
        _ => [1.0; 3],
    };
    match mesh::sky_cube::build(&name, &blob, rotation, tint) {
        Ok(sky) => {
            let (width, height) = sky
                .textures
                .first()
                .and_then(Option::as_ref)
                .map_or((0, 0), |t| (t.width, t.height));
            let whose = if zone_sky.is_some() {
                "the Zone sky, drawn in place of the circuit's own"
            } else {
                "the circuit's sky"
            };
            report.push(format!(
                "{name}: {whose}, six {width}x{height} cubemap face(s) on a \
                 camera-centred cube, turned {rotation:.0} degree(s) by the authored Sky \
                 rotation (unit read off the corpus; sign and axis this project's), tinted \
                 x{:.2}/{:.2}/{:.2} by Sky colour (byte / 255, the vertex colour the original's sky draw carries)",
                tint[0], tint[1], tint[2]
            ));
            Some(sky)
        }
        Err(error) => {
            report.push(format!("{name}: {error:#} - the sky stays black"));
            None
        }
    }
}

/// `SkyCube/skycube.rcsmodel`'s sibling name beside a Wipeout 2048 `track.vex`.
///
/// **Beside the track's own directory, not the track's own stem.** Unlike
/// [`sky_gtf_name`], the sky file's basename is fixed (`skycube.rcsmodel`
/// regardless of whether the track file is `track.vex` or
/// `track_reversed.vex`) and it is not inside a `SkyCube/` subdirectory
/// despite the `.rcsmaterial` and `.gxt` beside it being - measured off the
/// shipped archive layout, not assumed from the directory name. See
/// [2048-sky.md](../../../docs/formats/2048-sky.md).
fn psp2_sky_sibling_name(track: &str) -> Option<String> {
    let at = track.rfind(['/', '\\'])?;
    Some(format!(
        "{}{}skycube.rcsmodel",
        &track[..at],
        &track[at..=at]
    ))
}

/// The camera-centred radius every Wipeout 2048 sky is rescaled to.
///
/// **Appearance-invariant, the same argument [`mesh::sky_cube::HALF_EXTENT`]
/// makes for Wipeout HD's generated cube** - the picture a camera-centred
/// shape draws does not depend on its absolute size, only on clearing the
/// near plane (`1.0`, `race::camera`'s own constant). What differs here is
/// that the *authored* dome ships small: measured at radius 1.83 units on
/// every base-game circuit (`altima` among them), not the tens-of-units scale
/// Wipeout HD's cube or Pulse's authored skies use. Left at that size, a
/// visible point at angle `theta` off the view axis sits at view-space depth
/// `radius * cos(theta)` - so anything past roughly 57 degrees off-axis
/// (`arccos(1.0 / 1.83)`) would clip into the near plane, which a merely wide
/// field of view reaches at the screen corners. Rescaled to the same 32-unit
/// convention Wipeout HD's cube uses, the same corner clears past ~88
/// degrees, comfortably past any FOV this project drives the camera at.
const PSP2_SKY_TARGET_RADIUS: f32 = 32.0;

/// Wipeout 2048's sky, or `None` with the reason in the report.
///
/// **An authored dome mesh, not a generated cube.** Wipeout HD's sky is
/// `sky.gtf`, a bare cubemap with no geometry of its own -
/// [`hd_sky_model`] supplies the cube. Wipeout 2048's is `skycube.rcsmodel`
/// beside the track, a small camera-centred dome mesh in the same `.rcsmodel`
/// container [`geometry::track_model`] already reads the track and craft
/// through - see `oag_rcs::rcsmodel::psp2` and
/// [2048-sky.md](../../../docs/formats/2048-sky.md) for the format finding
/// that made this readable at all (the dome's own submesh record uses a
/// second, previously-unrecognised buffer-pointer gap,
/// `psp2::SKY_BUFFER_POINTER_GAP`).
///
/// **The circuit's own `Lighting.Sky rotation` turns the dome**, in degrees
/// about the vertical, the way [`hd_sky_model`] turns HD's cube. The file is
/// 2048's `track.EnvSettings` and carries the key under HD's spelling; the
/// unit is read off the two titles' shared circuits (Anulpha Pass authors 230
/// in HD and -130 in 2048, the same angle). The *sign* is HD's choice
/// (positive about `+Y`) and **chosen, not measured**. `Sky brightness` and
/// `Sky height offset` are authored too and left unread: their consumer is
/// unlocated.
///
/// **Zone's sky swap is not wired.** `data/Tex/zoneSky.gxt` exists on the
/// disc, the same file HD's `Data/Tex/ZoneSky.gtf` names, but nothing here
/// reads it - a Zone race draws the circuit's own dome, same as every other
/// mode. Recorded as located-and-unwired rather than chased, the same
/// `CLAUDE.md` line this whole function follows: draw what is read, say what
/// is not.
pub(super) fn psp2_sky_model(
    archives: &mut oag_assets::Archives,
    track: &str,
    report: &mut Vec<String>,
) -> Option<mesh::Model> {
    let name = psp2_sky_sibling_name(track)?;
    let Ok(blob) = archives.read_name(&name) else {
        report.push(format!(
            "{name}: not in the archive set - the sky stays black"
        ));
        return None;
    };
    if !mesh::rcs::psp2::is_psp2(&blob) {
        report.push(format!(
            "{name}: not a Wipeout 2048 .rcsmodel - the sky stays black"
        ));
        return None;
    }
    let (mut model, built) = match mesh::rcs::psp2::build(&name, &blob, None, &mut |path| {
        archives.read_name(path).ok()
    }) {
        Ok(built) => built,
        Err(error) => {
            report.push(format!("{name}: {error:#} - the sky stays black"));
            return None;
        }
    };
    if model.indices.is_empty() {
        report.push(format!(
            "{name}: decoded to no triangles - the sky stays black"
        ));
        return None;
    }
    // Camera-centred like every other sky source (`mesh::sky_cube::build`'s
    // own doc comment), but rescaled from the authored dome's own small
    // radius up to `PSP2_SKY_TARGET_RADIUS` - see that constant's doc for why
    // the authored size alone risks a near-plane clip. A uniform scale about
    // the model's own centre, which `finish_bounds` inside `psp2::build`
    // already measured as the dome's origin.
    let authored_radius = model.radius;
    let rotation = envsettings_name(track)
        .and_then(|name| archives.read_name(&name).ok())
        .and_then(|blob| String::from_utf8(blob).ok())
        .and_then(|text| EnvSettings::parse(&text).ok())
        .and_then(|env| env.scalar(oag_tables::envsettings::SKY_ROTATION))
        .unwrap_or(0.0);
    let (sin, cos) = rotation.to_radians().sin_cos();
    if authored_radius > 0.0 {
        let scale = PSP2_SKY_TARGET_RADIUS / authored_radius;
        for vertex in &mut model.vertices {
            for a in vertex.position.iter_mut() {
                *a *= scale;
            }
            let [x, y, z] = vertex.position;
            vertex.position = [x * cos + z * sin, y, z * cos - x * sin];
            // A sky is a picture of light, not a surface - the same rule
            // `mesh::sky_cube::build` states for HD's generated cube, and
            // Pulse's own skies (authored `_nolight`) confirm independently.
            // Left at `psp2::build`'s default of `1.0`, the stand-in light
            // rig would shade one side of the dome as if it were geometry.
            vertex.lit = 0.0;
        }
        model.radius *= scale;
        for draw in &mut model.draws {
            for a in draw.bounds.centre.iter_mut() {
                *a *= scale;
            }
            let [x, y, z] = draw.bounds.centre;
            draw.bounds.centre = [x * cos + z * sin, y, z * cos - x * sin];
            draw.bounds.radius *= scale;
        }
    }
    report.push(format!(
        "{name}: the circuit's sky dome, {}, rescaled from its authored radius of \
         {authored_radius:.2} unit(s) to {PSP2_SKY_TARGET_RADIUS:.0} to clear the near \
         plane at any field of view, turned {rotation} degree(s) by `Sky rotation` (sign \
         chosen, not measured)",
        built.describe(),
    ));
    Some(model)
}

/// The name of the `.envsettings` beside a track `.vex`, or `None`.
///
/// The rewrite the disc's own pairing uses:
/// `/data/environments/talons_junction/track.vex` sits beside
/// `track.envsettings`. Case-insensitive on the suffix for the same reason
/// [`mesh::rcs::sibling_name`] is - the game's own spelling varies between
/// containers.
pub(super) fn envsettings_name(vex_name: &str) -> Option<String> {
    let at = vex_name.len().checked_sub(4)?;
    vex_name[at..]
        .eq_ignore_ascii_case(".vex")
        .then(|| format!("{}.envsettings", &vex_name[..at]))
}

/// Wipeout HD's own front-end `.envsettings` - what the settings registrar
/// starts from, before a circuit's own file overlays whichever keys it
/// declares. See [`staged_envsettings`].
///
/// **Always this path, never Fury's own `/data/fe/fe.fury.track.envsettings`.**
/// The two author byte-identical `HDR and Bloom` blocks - all ten keys,
/// checked against `hdfury-ps3-eu-dec.iso` directly - so which of them the
/// real engine actually loads ahead of a Fury/DLC race makes no difference to
/// this carry; `docs/ghidra/functions/ps3-hdfury-eu/renderer.md`'s "exact
/// wiring for the next lane" paragraph says the real boot order between the
/// two is unresolved, and picks this one for the same reason. **Chosen, not
/// measured - no confidence score**, same as every value this project fills
/// in without reading the executable's own registrar function.
///
/// `Archives::read_name` resolves this to `DATA00.PSARC`'s copy even though
/// `DATA02.PSARC` ships a byte-different entry at the same path (the two
/// disagree on `Lighting`/`Fog`, not `HDR and Bloom`): the bulk archive is
/// searched ahead of the DLC one, per `oag_assets::source::Archives::holder_of`.
const FRONT_END_ENVSETTINGS: &str = "/data/fe/fe.track.envsettings";

/// Reads and parses one `.envsettings` file off the archive set, or `None`
/// for any reason at all - not found, not UTF-8, not this format. Callers
/// that need to tell those apart for the report read the archive themselves;
/// this is for a base layer a caller falls back to silently either way.
pub(super) fn read_envsettings(
    archives: &mut oag_assets::Archives,
    name: &str,
) -> Option<EnvSettings> {
    let blob = archives.read_name(name).ok()?;
    let text = String::from_utf8(blob).ok()?;
    EnvSettings::parse(&text).ok()
}

/// The circuit's own `.envsettings`, laid over [`FRONT_END_ENVSETTINGS`] -
/// the registrar [`envsettings_bloom`] reads through, replacing its own lone
/// read of the circuit's file.
///
/// **This is the registrar's own persistence, read live rather than
/// invented.** `renderer.md`'s "The resolve's Fury circuits carry the front
/// end's own Tone triple" section (2026-09-13) found Sol 2's race running its
/// exposure resolve at `(20, 3, 4)` although Sol 2's own file authors only the
/// first of the three - because the front end's file loads first and a
/// circuit's own file only overwrites the keys it declares, never resetting
/// the rest. Confidence 85 on the mechanism, cited from that page; this
/// function is the mechanism, not a new reading of it.
///
/// `None` only when *neither* file is readable at all - every Pulse, Pure, PS2
/// and Wipeout 2048 circuit, none of which ship a front-end `.envsettings` at
/// this path, and a Wipeout HD/Fury circuit whose own file is also missing
/// falls through to the front end's alone rather than here.
///
/// **[`envsettings_fog`] and [`envsettings_light`] deliberately do not read
/// through this**, even though the brief that asked for this carry named them
/// as worth checking for the same shape. Checked: a disc-wide survey of every
/// `.envsettings` that exists (all 13 circuit files with one) found every
/// single one authoring a complete `Fog`/`Lighting` block on its own - the
/// partial-key gap `envsettings_bloom` has is not a gap either reader has.
/// The only place the carry *would* reach for them is `zone_2`/`zone_3`/
/// `zone_4`, which ship no `.envsettings` at all - a different case from a
/// partial override, and one this session has no live read or reference frame
/// to check: `fe.track.envsettings` is visibly a menu backdrop's rig (a cyan
/// `Sun color` of `0.09/0.84/0.97`, ambient up to `3.0`), and wiring it into
/// three Zone circuits' light and fog would be this project drawing a picture
/// nothing measured, which is exactly what `CLAUDE.md`'s "never invent what
/// the assets already author" exists to stop. Left as the pre-existing
/// silent stand-in/unfogged fallback instead - a known, named absence rather
/// than an unverified substitution.
struct StagedEnvSettings {
    /// The table every reader now reads from: the front end's file with the
    /// circuit's own laid over it.
    merged: EnvSettings,
    /// The circuit's own file, unmerged - kept only to tell a carried key
    /// (present in `merged` because the front end authors it, absent from
    /// this) apart from the circuit's own for the report.
    own: Option<EnvSettings>,
    /// The circuit's own `.envsettings` name, attempted whether or not it
    /// existed - `None` only when `track` is not a `.vex` at all.
    own_name: Option<String>,
}

impl StagedEnvSettings {
    /// Which of `keys` are not the circuit's own - so came from the front
    /// end's carried value instead - in the order given, for a report line.
    fn carried<'a>(&self, keys: &[&'a str]) -> Vec<&'a str> {
        keys.iter()
            .copied()
            .filter(|key| {
                !self
                    .own
                    .as_ref()
                    .is_some_and(|own| own.entries.contains_key(*key))
            })
            .collect()
    }

    /// The label a report line should name for this circuit: its own file's
    /// name where it has one, the front end's otherwise.
    fn label(&self) -> &str {
        self.own_name.as_deref().unwrap_or(FRONT_END_ENVSETTINGS)
    }
}

fn staged_envsettings(
    archives: &mut oag_assets::Archives,
    track: &str,
    report: &mut Vec<String>,
) -> Option<StagedEnvSettings> {
    let front_end = read_envsettings(archives, FRONT_END_ENVSETTINGS);
    let own_name = envsettings_name(track);
    let own = match own_name
        .as_deref()
        .map(|name| (name, archives.read_name(name)))
    {
        Some((name, Ok(blob))) => match String::from_utf8(blob) {
            Ok(text) => match EnvSettings::parse(&text) {
                Ok(env) => Some(env),
                Err(error) => {
                    report.push(format!(
                        "{name}: {error}; using only the front end's own settings"
                    ));
                    None
                }
            },
            Err(_) => {
                report.push(format!(
                    "{name}: not text; using only the front end's own settings"
                ));
                None
            }
        },
        _ => None,
    };
    let merged = match (&front_end, &own) {
        (Some(front_end), Some(own)) => {
            let mut merged = front_end.clone();
            merged.entries.extend(own.entries.clone());
            merged
        }
        (Some(front_end), None) => front_end.clone(),
        (None, Some(own)) => own.clone(),
        (None, None) => return None,
    };
    Some(StagedEnvSettings {
        merged,
        own,
        own_name,
    })
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
///
/// **Does not read through [`staged_envsettings`].** Checked for the same
/// partial-key gap `envsettings_bloom` had (`lane-envsettings-carry`): every
/// circuit that ships a `.envsettings` at all authors both `Fog` keys itself,
/// so there is nothing here for the front end's file to carry. See
/// [`staged_envsettings`]'s own doc for why the remaining case -
/// `zone_2`/`zone_3`/`zone_4`, which ship no file at all - is left as the
/// pre-existing unfogged fallback rather than wired to the front end's.
///
/// **`psp2` reads Wipeout 2048's own spelling instead** - one
/// `"Lighting.Fog colour"` key of four numbers, colour then a fourth that is
/// HD's `Fog Density`'s order of magnitude. **The curve there is inherited,
/// not measured**: 2048's fragment microcode is USSE and its fog term is
/// unread, so HD's `exp(-(density * view_depth)^2)` is drawn with the fourth
/// component as the coefficient, on the strength of the shared `fogColour`
/// `float4` and the shared lineage alone. The `Fog Region Colour Override`
/// ladder and `Depth Fog Offset RecipRange` are authored and left unread:
/// what selects a region is unlocated.
pub(super) fn envsettings_fog(
    archives: &mut oag_assets::Archives,
    track: &str,
    psp2: bool,
    report: &mut Vec<String>,
) -> Option<mesh_render::Fog> {
    use oag_tables::envsettings::{FOG_COLOUR, FOG_DENSITY, PSP2_FOG_COLOUR};
    let name = envsettings_name(track)?;
    let blob = archives.read_name(&name).ok()?;
    let text = String::from_utf8(blob).ok()?;
    let env = EnvSettings::parse(&text).ok()?;
    let authored = if psp2 {
        env.vec4(PSP2_FOG_COLOUR)
            .map(|v| ([v[0], v[1], v[2]], v[3]))
    } else {
        env.vec3(FOG_COLOUR).zip(env.scalar(FOG_DENSITY))
    };
    let Some((colour, density)) = authored else {
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
        "{name}: fog [{:.2}, {:.2}, {:.2}] at density {density}, on {}: \
         exp(-(density * view_depth)^2)",
        colour[0],
        colour[1],
        colour[2],
        if psp2 {
            "the curve INHERITED from Wipeout HD's microcode (2048's own is unread)"
        } else {
            "the curve read from the circuit's own fragment microcode"
        },
    ));
    Some(mesh_render::Fog::authored_exp2(colour, density))
}

/// The circuit's **alternate** fog pair, `Fog.Alternate Fog Color` and
/// `Alternate Fog Density`, on [`envsettings_fog`]'s curve, or `None` with the
/// reason reported.
///
/// The fog the original gives a chunk whose render flag has `0x20`
/// (`oag_rcs::rcsmodel::RENDER_ALTERNATE_FOG`): on Vineta K's capture the
/// patched `fogColour` of every such draw is this pair,
/// `{0, 0.031373, 0.031373, 0.0045}`, and of every other draw the primary
/// one. Drawn on the behind-the-glass target's chunks, 198 of whose 210 per
/// direction carry the bit; the 5 main-view chunks that do keep the primary
/// pair (`scene::behind_glass`).
pub(super) fn envsettings_alternate_fog(
    archives: &mut oag_assets::Archives,
    track: &str,
    report: &mut Vec<String>,
) -> Option<mesh_render::Fog> {
    use oag_tables::envsettings::{ALTERNATE_FOG_COLOUR, ALTERNATE_FOG_DENSITY};
    let name = envsettings_name(track)?;
    let text = String::from_utf8(archives.read_name(&name).ok()?).ok()?;
    let env = EnvSettings::parse(&text).ok()?;
    let pair = env
        .vec3(ALTERNATE_FOG_COLOUR)
        .zip(env.scalar(ALTERNATE_FOG_DENSITY))
        .filter(|&(_, density)| density > 0.0);
    let Some((colour, density)) = pair else {
        report.push(format!(
            "{name}: no usable alternate fog; the behind-the-glass chunks that select it \
             draw with the primary fog"
        ));
        return None;
    };
    report.push(format!(
        "{name}: alternate fog [{:.3}, {:.3}, {:.3}] at density {density}, for the chunks \
         flagged 0x20",
        colour[0], colour[1], colour[2],
    ));
    Some(mesh_render::Fog::authored_exp2(colour, density))
}

mod bloom;
mod light;
pub(super) use bloom::envsettings_bloom;
pub(super) use light::{envsettings_light, envsettings_tonemap};

/// Everything the four environment readers found, in one value.
///
/// Bundled because they are one concern - the staging this module's own docs
/// name - and because `load.rs` has a 1,000-line ceiling that four separate
/// call sites plus their reasoning were pushing at. No behaviour of any reader
/// changed in the move.
pub(super) struct Staging {
    /// The circuit's own light rig, or [`mesh_render::Light::stand_in`].
    pub(super) light: mesh_render::Light,
    /// The circuit's authored distance fog, where it authors one.
    pub(super) authored_fog: Option<mesh_render::Fog>,
    /// The circuit's `HDR and Bloom` block, where it authors one.
    pub(super) hd_bloom: Option<oag_post::hd_bloom::Params>,
    /// Omega's `Tonemap` block, where the circuit authors one.
    pub(super) omega_tonemap: Option<oag_post::omega_tonemap::Params>,
    /// The Zone stage grade this title lays over the two above, in a Zone
    /// race on a title that ships a table. See [`zone_grade`].
    pub(super) zone_grade: Option<crate::zone_grade::ZoneGrade>,
}

/// Which title's `.rcsmodel` container [`staging`] is reading a light rig
/// beside, where there is one.
///
/// Every non-`None` variant sets the flag `ps3_geometry` used to be - the two
/// readers that only a `.rcsmodel`-backed circuit answers stay gated on
/// "either", per [`staging`]'s own doc. Only the light rig cares which one,
/// because only its key spelling differs - see [`envsettings_light`]. A
/// third bool argument would have pushed [`staging`] over
/// `clippy::too_many_arguments`; this replaces the one it already had rather
/// than adding to it.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum GeometryKind {
    /// No `.rcsmodel` sibling - Pulse, Pure, PS2, or a title falling back to
    /// the derived ribbon.
    None,
    /// Wipeout HD's own container.
    Hd,
    /// Wipeout 2048's own container - the same extension, an unrelated binary
    /// shape (see `oag_rcs::rcsmodel::psp2`).
    Psp2,
    /// Wipeout: Omega Collection's: 2048's container with eight-byte offsets
    /// ([`oag_rcs::rcsmodel::psp2::is_ps4`]). **A kind of its own because its
    /// pixel shaders are not 2048's**: Omega's lightmapped surfaces are lit by
    /// the `Lighting.Nova prelit scale bias power` triple - see
    /// [`mesh_render::Light::with_nova_prelit`] - which the Vita files author
    /// (as `1 0 1 0`) and whose Vita shader is unread.
    Ps4,
}

impl GeometryKind {
    /// Classifies a `.rcsmodel` sibling already read off the archive, or its
    /// absence - the same test `load.rs` already made to decide the sky and
    /// pad readers, kept in one place rather than repeated at each call site.
    pub(super) fn of(rcsmodel: Option<&[u8]>) -> Self {
        match rcsmodel {
            None => Self::None,
            Some(geometry) if mesh::rcs::psp2::is_psp2(geometry) => {
                if oag_rcs::rcsmodel::psp2::is_ps4(geometry) {
                    Self::Ps4
                } else {
                    Self::Psp2
                }
            }
            Some(_) => Self::Hd,
        }
    }
}

/// Reads the circuit's staging: its rig, its fog, its bloom and its Zone
/// grade.
///
/// `geometry` gates the two readers that only a `.rcsmodel`-backed circuit
/// answers - a Pulse circuit fogs through its `fogCube` volumes and authors no
/// `.envsettings` at all - and the Zone grade is gated on the mode and on the
/// title shipping a table, not on the geometry. Which of the two non-`None`
/// variants it is additionally picks the light rig's key spelling - see
/// [`envsettings_light`].
pub(super) fn staging(
    archives: &mut oag_assets::Archives,
    track: &str,
    geometry: GeometryKind,
    // The Zone axes and the lit-output scale all come off the same title
    // record, so this takes the record rather than loose fields - which is
    // also what keeps the argument count under `clippy::too_many_arguments`.
    title: &'static oag_title::Title,
    mode: oag_race::Mode,
    zone_stage: Option<u32>,
    report: &mut Vec<String>,
) -> Staging {
    let ps3_geometry = geometry != GeometryKind::None;
    // **The circuit's own light rig, where it authors one.** Wipeout HD does:
    // `track.envsettings` sits beside `track.vex` and states a sun direction, a
    // sun colour and a constant ambient. `mesh.wgsl`'s two-light rig is a
    // stand-in for exactly this and says so, and `CLAUDE.md`'s rule about not
    // inventing what the assets already author is why it is read here rather
    // than approximated. Wipeout 2048 ships the same shape under its own
    // `track.EnvSettings`, keyed differently - see [`envsettings_light`].
    //
    // A sibling-path rewrite, the same shape as `mesh::rcs::sibling_name`, and
    // the same failure behaviour: no file, an unparsable one, or a degenerate
    // sun direction all fall back to the stand-in and say so. Nothing is
    // substituted for a value the file does not carry.
    let light = envsettings_light(
        archives,
        track,
        matches!(geometry, GeometryKind::Psp2 | GeometryKind::Ps4),
        geometry == GeometryKind::Ps4,
        report,
    );
    // Every lit program of a title whose `lit_output_halved` rule applies ends
    // on a `/2` output scale (HD's: `oag_title::Looks::lit_output_halved`).
    let light = if title.looks.lit_output_halved.applies_everywhere() {
        light.with_output_scale(0.5)
    } else {
        light
    };
    // The circuit's authored distance fog, on the same file. **The curve is no
    // longer a guess**: every fogged fragment variant of an HD circuit
    // `.rcsmaterial` computes `exp(-(coefficient * view_depth)^2)` and lerps a
    // patched `fogColour` in by it - read out of the microcode itself, see
    // `mesh_render::Fog::curve`.
    let authored_fog = ps3_geometry
        .then(|| {
            let psp2 = matches!(geometry, GeometryKind::Psp2 | GeometryKind::Ps4);
            envsettings_fog(archives, track, psp2, report)
        })
        .flatten();
    // The circuit's `HDR and Bloom` block, which is what turns the HD race
    // onto the linear float scene target and the read FunkLayerBloom chain -
    // see `oag_post::hd_bloom` for what of that is the microcode's
    // and what is this project's. Gated to the PS3 path like the fog above.
    let hd_bloom = ps3_geometry
        .then(|| envsettings_bloom(archives, track, report))
        .flatten()
        .map(|params| oag_post::hd_bloom::Params {
            zoom: title.race.zoom_ring.map(crate::zoom::tuning),
            ..params
        });
    let omega_tonemap = (geometry == GeometryKind::Ps4)
        .then(|| envsettings_tonemap(archives, track, report))
        .flatten();
    Staging {
        light,
        authored_fog,
        hd_bloom,
        omega_tonemap,
        zone_grade: zone_grade(archives, title.race, mode, zone_stage, track, report),
    }
}

/// Both fifteen-entry per-stage texture sets, `(track, general)`, one slot
/// per stage each.
///
/// **Both, because the original binds both and the chunk picks.** The track
/// set carries the art and is sampled by a chunk whose render-block flags
/// carry the track bit; the general set, fifteen byte-identical flat whites,
/// by every other chunk, whose surface is then its `Scene.Texture Colour`
/// flat. See `oag_mesh::mesh_render::zone::StageArt` and
/// [`oag_title::ZoneStageTextures`] for the trap the general set used to be.
/// A stage whose entry is missing or will not decode leaves its slot `None`
/// and says so in the report rather than substituting a neighbour's.
fn zone_stage_art(
    archives: &mut oag_assets::Archives,
    textures: &'static oag_title::ZoneStageTextures,
    report: &mut Vec<String>,
) -> (oag_mesh::mesh::TextureSlots, oag_mesh::mesh::TextureSlots) {
    let mut load = |set: &str, entry: &dyn Fn(u32) -> String| {
        let mut decoded = 0_u32;
        let mut art = Vec::with_capacity(textures.stages as usize);
        for stage in textures.stages() {
            let name = entry(stage);
            let slot = archives
                .read_name(&name)
                .ok()
                .and_then(|blob| oag_mesh::mesh::ModelTexture::from_gtf(&name, &blob))
                .map(std::sync::Arc::new);
            if slot.is_some() {
                decoded += 1;
            } else {
                report.push(format!(
                    "{name}: no per-stage Zone {set} texture for stage {stage}"
                ));
            }
            art.push(slot);
        }
        report.push(format!(
            "{}0{}..{}: {decoded}/{} per-stage Zone {set} textures decoded",
            entry(0)
                .trim_end_matches(textures.extension)
                .trim_end_matches('0'),
            textures.extension,
            textures.stages - 1,
            textures.stages,
        ));
        art
    };
    let track = load("track", &|stage| textures.track_entry(stage));
    let general = load("general", &|stage| textures.general_entry(stage));
    (track, general)
}

/// The stage a Zone race opens on where the title's own ladder is unrecovered.
///
/// **Inferred, not measured** - see [`zone_grade`] for the three sources that
/// converge on it. A recovered trigger would replace this, not adjust it.
const ZONE_OPENING_STAGE: u32 = 1;

pub(super) fn zone_grade(
    archives: &mut oag_assets::Archives,
    race: &'static oag_title::RaceDefaults,
    mode: oag_race::Mode,
    forced_stage: Option<u32>,
    track: &str,
    report: &mut Vec<String>,
) -> Option<crate::zone_grade::ZoneGrade> {
    use oag_tables::effectsettings::EffectSettings;

    if mode != oag_race::Mode::Zone {
        return None;
    }
    let name = race.zone_palette?.entry_for(track)?;
    let Ok(blob) = archives.read_name(&name) else {
        report.push(format!(
            "{name}: not in the archive set - the Zone race keeps the circuit's own colours"
        ));
        return None;
    };
    let text = match String::from_utf8(blob) {
        Ok(text) => text,
        Err(_) => {
            report.push(format!("{name}: not text, so it is not this format"));
            return None;
        }
    };
    let table = match EffectSettings::parse(&text) {
        Ok(table) => table,
        Err(error) => {
            report.push(format!("{name}: {error}; no stage grade"));
            return None;
        }
    };
    let (art, scene_art) = match race.zone_stage_textures {
        Some(textures) => zone_stage_art(archives, textures, report),
        None => (Vec::new(), Vec::new()),
    };
    let mut grade =
        crate::zone_grade::ZoneGrade::new(name.clone(), table, race.zone_stages, art, scene_art)
            // How a stage step sweeps the world, where the title's own law is read -
            // HD/Fury's sphere out of the craft; `None` on 2048, whose rate is
            // unread. See `oag_title::ZoneTransition`.
            .map(|grade| grade.with_transition(race.zone_transition));
    // The stage a race *starts* on, which is not stage `0`: **`Start` is the
    // pre-race state, not the opening lap's.** Three converging sources, one of
    // them a direct observation of the original:
    //
    // - 2048's own recovered threshold table matches zone `0` against its last
    //   record and so shows stage `1` from the first frame.
    // - HD's Detonator counter is likewise constructed holding `1`.
    // - The original, played: a Zone race on Moa Therma is already showing
    //   `Sub Venom`'s cyan at the start line. That stage authors
    //   `Track.Base Colour` = `0.003922 0.847059 1.000000`; `Start` authors a
    //   flat black, which is what this port drew before this and what the
    //   maintainer reported as wrong.
    //
    // `show_zone` does this on a title whose ladder is recovered, and **is a
    // no-op on one whose is not** - which is every title but 2048, HD
    // included, so the comment above used to describe an intent this branch
    // never carried out. The opening stage is applied explicitly for those.
    //
    // **This is the opening stage, not the trigger.** What *advances* the
    // stage during an HD race is still unrecovered; see
    // `crate::zone_grade`'s module docs.
    if let Some(grade) = grade.as_mut() {
        if grade.stage_for_zone(0).is_some() {
            grade.show_zone(0);
        } else {
            grade.request_stage(ZONE_OPENING_STAGE);
            grade.commit();
            // `commit` zeroes the weight and starts the transition sphere,
            // verbatim as the traced store does, which would leave the
            // opening stage showing `Start` underneath and outside it. An
            // opening stage is shown whole.
            grade.show_whole();
        }
    }
    // **The development override, applied after the title's own ladder and
    // pinned rather than merely committed once.** Both titles with a
    // recovered ladder (2048, and HD/Fury since 2026-08-31) drive
    // `show_zone` from the scene every frame, so a plain commit here would be
    // overwritten on the very next call - see
    // `crate::zone_grade::ZoneGrade::pin_stage` and
    // `crate::Options::zone_stage`.
    if let (Some(grade), Some(stage)) = (grade.as_mut(), forced_stage) {
        grade.pin_stage(stage);
        report.push(format!(
            "zone: stage pinned to {} by --zone-stage; the title's own ladder will not \
             move it",
            grade.blend().current,
        ));
    }
    match &grade {
        Some(grade) => report.push(grade.describe()),
        None => report.push(format!("{name}: names no stage at all; no stage grade")),
    }
    grade
}

/// The shared texture every `cloudCube` sprite draws with - `oag_vex::cloud`
/// names it; present in `Data.wad` per `cloud_ground_truth.rs`.
const CLOUD_TEXTURE: &str = "Data\\Tex\\Cloud\\Wipeout_Clouds_D_128x64x4.mip";

/// `05_Track`'s cloud puffs, and their shared texture - `None` for every
/// other circuit and for a ribbon build. See `crates/fx/src/cloud.rs`'s
/// module doc for what this draws and what it deliberately does not.
pub(super) fn cloud_layer(
    archives: &mut oag_assets::Archives,
    track_blob: &[u8],
    vex_geometry: bool,
    report: &mut Vec<String>,
) -> Option<(oag_fx::cloud::Layer, FlareTexture)> {
    if !vex_geometry {
        return None;
    }
    let nodes = vex::nodes(track_blob).unwrap_or_default();
    let layer = oag_fx::cloud::Layer::from_groups(track_blob, &nodes, &oag_fx::cloud::CHOSEN_SEEDS);
    if layer.is_empty() {
        return None;
    }
    let blob = match archives.read_name(CLOUD_TEXTURE) {
        Ok(blob) => blob,
        Err(error) => {
            report.push(format!(
                "{CLOUD_TEXTURE}: not in the archive set ({error}) - {} cloud sprite(s) \
                 decoded but not drawn",
                layer.len()
            ));
            return None;
        }
    };
    match oag_texture::texture::Texture::parse(&blob) {
        Ok(texture) => {
            report.push(format!(
                "{CLOUD_TEXTURE}: {}x{} .mip - drawing {} cloud sprite(s), each \
                 group's field built from a chosen seed (oag_fx::cloud::CHOSEN_SEEDS; \
                 see docs/ghidra/functions/psp-pulse-usa/clouds.md)",
                texture.width,
                texture.height,
                layer.len()
            ));
            Some((
                layer,
                FlareTexture {
                    width: u32::from(texture.width),
                    height: u32::from(texture.height),
                    rgba: texture.to_rgba(),
                },
            ))
        }
        Err(error) => {
            report.push(format!(
                "{CLOUD_TEXTURE}: {error:#} - {} cloud sprite(s) decoded but not drawn",
                layer.len()
            ));
            None
        }
    }
}
