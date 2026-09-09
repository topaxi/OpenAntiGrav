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
    let rotation = envsettings_name(track)
        .and_then(|name| archives.read_name(&name).ok())
        .and_then(|blob| String::from_utf8(blob).ok())
        .and_then(|text| oag_tables::envsettings::EnvSettings::parse(&text).ok())
        .and_then(|env| env.scalar(oag_tables::envsettings::SKY_ROTATION))
        .unwrap_or(0.0);
    match mesh::sky_cube::build(&name, &blob, rotation) {
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
/// through - see `oag_formats::rcsmodel::psp2` and
/// [2048-sky.md](../../../docs/formats/2048-sky.md) for the format finding
/// that made this readable at all (the dome's own submesh record uses a
/// second, previously-unrecognised buffer-pointer gap,
/// `psp2::SKY_BUFFER_POINTER_GAP`).
///
/// **No rotation is applied.** Wipeout HD's `Lighting.Sky rotation` comes off
/// `.envsettings`, and that file does not parse for this title at all - see
/// `docs/formats/2048-status.md` - so there is nothing this reading could turn
/// the dome by even if the mesh authored an asymmetric horizon (it does not
/// measurably: every sampled circuit's dome is close to a uniform sphere).
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
    let (mut model, built) =
        match mesh::rcs::psp2::build(&name, &blob, &mut |path| archives.read_name(path).ok()) {
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
    if authored_radius > 0.0 {
        let scale = PSP2_SKY_TARGET_RADIUS / authored_radius;
        for vertex in &mut model.vertices {
            for a in vertex.position.iter_mut() {
                *a *= scale;
            }
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
            draw.bounds.radius *= scale;
        }
    }
    report.push(format!(
        "{name}: the circuit's sky dome, {}, rescaled from its authored radius of \
         {authored_radius:.2} unit(s) to {PSP2_SKY_TARGET_RADIUS:.0} to clear the near \
         plane at any field of view",
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
    use oag_tables::envsettings::{EnvSettings, FOG_COLOUR, FOG_DENSITY};
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
    use oag_tables::envsettings::{
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
///
/// **`psp2` picks the key spelling, not the file.** Wipeout 2048 ships the
/// same `"Key.Subkey"=float [float...]` syntax beside its own `track.vex`, but
/// its registrar spells the ambient and sun-diffuse terms differently from
/// HD's - see `oag_tables::envsettings`'s "Wipeout 2048 authors the same
/// shape under different key names". Reading HD's keys against a 2048 file
/// resolves nothing and falls back to the stand-in rig silently wrong about
/// why; this is what tells the two schemas apart.
pub(super) fn envsettings_light(
    archives: &mut oag_assets::Archives,
    track: &str,
    psp2: bool,
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
    let env = match oag_tables::envsettings::EnvSettings::parse(&text) {
        Ok(env) => env,
        Err(e) => {
            report.push(format!("{name}: {e}; lighting with the stand-in rig"));
            return mesh_render::Light::stand_in();
        }
    };
    use oag_tables::envsettings::{
        AMBIENT_COLOUR, PRELIT_POWER, PRELIT_SCALE, PSP2_AMBIENT_COLOUR, PSP2_SUN_DIFFUSE_COLOUR,
        SUN_COLOUR, SUN_DIRECTION, SUN_SPECULAR_SCALE,
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
    let combination = if psp2 {
        "2048 authors no equivalent prelit or specular keys, so both stay at the identity"
    } else {
        "the combination is the microcode's own; the render target's saturation stands in \
         for HD's tonemap"
    };
    report.push(format!(
        "{name}: sun [{:.2}, {:.2}, {:.2}] colour [{:.2}, {:.2}, {:.2}] over ambient \
         [{:.2}, {:.2}, {:.2}], prelit {:.1}*lightmap^{:.1}, specular x{:.2} - {combination}",
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
    pub(super) hd_bloom: Option<oag_render::post::hd_bloom::Params>,
    /// The Zone stage grade this title lays over the two above, in a Zone
    /// race on a title that ships a table. See [`zone_grade`].
    pub(super) zone_grade: Option<crate::race::zone_grade::ZoneGrade>,
}

/// Which title's `.rcsmodel` container [`staging`] is reading a light rig
/// beside, where there is one.
///
/// Both non-`None` variants set the flag `ps3_geometry` used to be - the two
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
    /// shape (see `oag_formats::rcsmodel::psp2`).
    Psp2,
}

impl GeometryKind {
    /// Classifies a `.rcsmodel` sibling already read off the archive, or its
    /// absence - the same test `load.rs` already made to decide the sky and
    /// pad readers, kept in one place rather than repeated at each call site.
    pub(super) fn of(rcsmodel: Option<&[u8]>) -> Self {
        match rcsmodel {
            None => Self::None,
            Some(geometry) if mesh::rcs::psp2::is_psp2(geometry) => Self::Psp2,
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
    // The three Zone axes travel together and all come off the same title
    // record, so this takes the record rather than three loose fields - which
    // is also what keeps the argument count under `clippy::too_many_arguments`.
    race: &'static oag_title::RaceDefaults,
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
    let light = envsettings_light(archives, track, geometry == GeometryKind::Psp2, report);
    // The circuit's authored distance fog, on the same file. **The curve is no
    // longer a guess**: every fogged fragment variant of an HD circuit
    // `.rcsmaterial` computes `exp(-(coefficient * view_depth)^2)` and lerps a
    // patched `fogColour` in by it - read out of the microcode itself, see
    // `mesh_render::Fog::curve`.
    let authored_fog = ps3_geometry
        .then(|| envsettings_fog(archives, track, report))
        .flatten();
    // The circuit's `HDR and Bloom` block, which is what turns the HD race
    // onto the linear float scene target and the read FunkLayerBloom chain -
    // see `oag_render::post::hd_bloom` for what of that is the microcode's
    // and what is this project's. Gated to the PS3 path like the fog above.
    let hd_bloom = ps3_geometry
        .then(|| envsettings_bloom(archives, track, report))
        .flatten();
    Staging {
        light,
        authored_fog,
        hd_bloom,
        zone_grade: zone_grade(archives, race, mode, zone_stage, track, report),
    }
}

/// The Zone colour grade a title lays over the circuit, stage by stage, or
/// `None` with the reason reported.
///
/// **Zone only, and only where the title ships a table.** Pulse and Pure ship
/// none - see [`oag_title::ZonePalette`] for how thoroughly each disc was
/// searched before that was written down - so this is silent on them rather
/// than reporting an absence that is true of a whole title, the same rule
/// [`envsettings_light`] follows. A missing entry on a title that *claims* one
/// is reported, because that is a gap in this reading rather than in the data.
///
/// **Nothing this returns advances during a race.** The grade rests on stage
/// `0`, which is where HD's own loader leaves it, and what would move it is
/// unrecovered on both titles - see `crate::race::zone_grade`'s module docs
/// and `docs/formats/effectsettings.md`'s `## Open`.
/// The "track" set's per-stage textures, decoded, one slot per stage.
///
/// **The track set, never the general one.** The original binds both to the
/// same two shader parameters from different publishers, and only this one is
/// a picture - see [`oag_title::ZoneStageTextures`] for the trap. A stage
/// whose entry is missing or will not decode leaves its slot `None` and says
/// so in the report rather than substituting a neighbour's.
fn zone_stage_art(
    archives: &mut oag_assets::Archives,
    textures: &'static oag_title::ZoneStageTextures,
    report: &mut Vec<String>,
) -> Vec<Option<std::sync::Arc<oag_render::mesh::ModelTexture>>> {
    let mut decoded = 0_u32;
    let mut art = Vec::with_capacity(textures.stages as usize);
    for stage in textures.stages() {
        let name = textures.track_entry(stage);
        let slot = archives
            .read_name(&name)
            .ok()
            .and_then(|blob| oag_render::mesh::ModelTexture::from_gtf(&name, &blob))
            .map(std::sync::Arc::new);
        if slot.is_some() {
            decoded += 1;
        } else {
            report.push(format!(
                "{name}: no per-stage Zone texture for stage {stage}"
            ));
        }
        art.push(slot);
    }
    report.push(format!(
        "{}0{}..{}: {decoded}/{} per-stage Zone textures decoded; nothing draws them yet, \
         because HD's own Zone fragment program is unread",
        textures.track,
        textures.extension,
        textures.stages - 1,
        textures.stages,
    ));
    art
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
) -> Option<crate::race::zone_grade::ZoneGrade> {
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
    let art = match race.zone_stage_textures {
        Some(textures) => zone_stage_art(archives, textures, report),
        None => Vec::new(),
    };
    let mut grade =
        crate::race::zone_grade::ZoneGrade::new(name.clone(), table, race.zone_stages, art);
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
    // `crate::race::zone_grade`'s module docs.
    if let Some(grade) = grade.as_mut() {
        if grade.stage_for_zone(0).is_some() {
            grade.show_zone(0);
        } else {
            grade.request_stage(ZONE_OPENING_STAGE);
            grade.commit();
            // `commit` zeroes the weight, verbatim as the traced store does,
            // which would leave the opening stage showing `Start` underneath
            // it. An opening stage is shown whole.
            grade.set_weight(1.0);
        }
    }
    // **The development override, applied after the title's own ladder.** It
    // exists because HD's stage trigger is unrecovered, so nothing else can put
    // an HD Zone race on a rung to compare against the original - see
    // `crate::race::Options::zone_stage`.
    if let (Some(grade), Some(stage)) = (grade.as_mut(), forced_stage) {
        grade.request_stage(stage);
        grade.commit();
        // The commit zeroes the weight, verbatim as the traced store does,
        // which would leave the forced stage showing its predecessor. A forced
        // stage is asked for whole.
        grade.set_weight(1.0);
        report.push(format!(
            "zone: stage forced to {} by --zone-stage; the title's own ladder is not \
             driving this",
            grade.blend().current,
        ));
    }
    match &grade {
        Some(grade) => report.push(grade.describe()),
        None => report.push(format!("{name}: names no stage at all; no stage grade")),
    }
    grade
}
