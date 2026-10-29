//! What Pulse on the PSP draws differently from every other source, measured
//! live on PPSSPP. The bloom's stencil-stamped glow mask is its own rule
//! (`Looks::psp_glow_mask`: Pulse and Pure on the PSP, unmeasured on the PS2 port or any
//! later title); the rest stays behind [`is_pulse_psp`]. The hull's
//! GE lights and the Quake's road ripple are read off the PSP executable and
//! carried to the PS2 by the `hull_lights` and `road_ripple` rules. The hull rig
//! is built for the PS2 too but draws nothing there: a PS2 hull vertex carries a
//! colour, which `mesh.wesl` reads as already lit
//! (`docs/ghidra/functions/ps2-pulse-eu/hull-lights.md`).
//!
//! See `docs/ghidra/functions/psp-pulse-usa/scene-light.md` and
//! `docs/rendering/glow-mask.md`.

#[cfg(feature = "view")]
use oag_mesh::mesh_render;

#[cfg(feature = "view")]
use super::Loaded;

/// Whether this race is Pulse off a PSP disc.
pub(super) fn is_pulse_psp(title: &oag_title::Title, archives: &oag_assets::Archives) -> bool {
    title.looks.measured_draws.applies(archives.layout.platform)
}

#[cfg(feature = "view")]
/// Marks every `.vex` model the race draws as stamping the glow mask when
/// `psp_glow_mask` says so. The circuit's own placed effects, weather, road
/// ripple and hull lights follow their own rules, which the PS2 shares.
pub(super) fn finish(
    loaded: &mut Loaded,
    (circuit_fx, road_ripple, hull_lights): (bool, bool, bool),
    psp_glow_mask: bool,
    track_shine: bool,
    track_blob: &[u8],
    (archives, track): (&mut oag_assets::Archives, &str),
) {
    let nodes = oag_vex::vex::nodes(track_blob).unwrap_or_default();
    if circuit_fx {
        place_scenery_fx(loaded, track_blob, &nodes);
        place_weather(loaded, track_blob, &nodes, archives, track);
    }
    if road_ripple {
        loaded.ripples = super::ripple::build(
            track_blob,
            &loaded.setup.ai,
            loaded.setup.course.as_ref(),
            [
                Some(&loaded.track_model),
                loaded.pad_model.as_ref(),
                loaded.weapon_pad_model.as_ref(),
            ],
            &mut loaded.report,
        );
    }
    // A foreign track has no blob; its file's own light is `foreign::art`'s.
    if hull_lights && !track_blob.is_empty() {
        light_hulls(loaded, track_blob, &nodes);
    }
    if track_shine {
        loaded
            .report
            .push(match oag_render::shine::build_track(&loaded.track_model) {
                Some(pass) => format!(
                    "track shine: {} chrome-map batches drawn, {} left undrawn (animated nodes or \
                 outside the opaque list)",
                    pass.model.draws.len(),
                    pass.skipped
                ),
                None if loaded.track_model.shine_draws.is_empty() => {
                    "track shine: off, or the circuit authors none".into()
                }
                None => format!(
                    "track shine: none drawn, {} batches left undrawn (animated nodes or outside \
                 the opaque list)",
                    loaded.track_model.shine_draws.len()
                ),
            });
    }
    if psp_glow_mask {
        super::glow_mask::stamp_models(loaded, false);
        loaded.report.push(
            "glow mask: the track, sky, pads, hulls, plumes, shields and weapon bodies stamp it as \
             the original's stencil does, a blended batch with the glow bits through a second draw"
                .into(),
        );
    }
}

#[cfg(feature = "view")]
/// Lights the hulls with the circuit's own light nodes, built at the bare
/// colour bytes: the craft's own scale is applied in the shader. See
/// `oag_mesh::mesh_render::HullLights`.
fn light_hulls(loaded: &mut Loaded, track_blob: &[u8], nodes: &[oag_vex::vex::Node]) {
    match mesh_render::HullLights::from_track(track_blob, nodes, 256) {
        Some(hull) => {
            loaded.report.push(format!(
                "hull lights: the circuit's own, ambient {:?}, diffuse {:?}",
                hull.ambient, hull.diffuse
            ));
            let directionals = hull.direction.iter().filter(|d| d[3] > 0.5).count();
            loaded.setup.hull_points =
                mesh_render::HullPointLights::from_track(track_blob, nodes, directionals);
            loaded.report.push(format!(
                "hull point lights: {} wopoint/wospot nodes, {} free slot(s) after {directionals} directional(s)",
                loaded.setup.hull_points.len(),
                4 - directionals.min(4),
            ));
            loaded.light.hull = hull;
        }
        None => loaded
            .report
            .push("hull lights: the circuit authors none; hulls keep the stand-in rig".into()),
    }
}

#[cfg(feature = "view")]
/// The circuit's own `ParticleSystem` nodes, each played from load on its node,
/// as `race::scenery_fx` describes. Pulse PSP only: `PsysNode_Init` was read
/// and confirmed live there and nowhere else.
fn place_scenery_fx(loaded: &mut Loaded, track_blob: &[u8], nodes: &[oag_vex::vex::Node]) {
    let placed = oag_vex::placed_psys::placed(track_blob, nodes);
    let mut names: Vec<&str> = placed.iter().map(|p| p.name.as_str()).collect();
    names.sort_unstable();
    names.dedup();
    for name in names {
        let count = placed.iter().filter(|p| p.name == name).count();
        let state = if loaded.setup.effects.get(name).is_some() {
            "played from load on its node"
        } else {
            "not loaded, so not drawn"
        };
        loaded
            .report
            .push(format!("placed effect: {count} x {name}, {state}"));
    }
    loaded.setup.scenery_fx.placed = placed;
}

#[cfg(feature = "view")]
/// The circuit's `<Weather>` element and the `weatherPos` anchors it is
/// played against, as `race::scenery_fx::weather` describes. Pulse PSP only:
/// `Weather_Update` was read there, and the PS2 disc's three circuits that
/// author the element are not played by extension.
///
/// Nothing in Zone: `TrackStartup_Parse` does not build the weather there.
///
/// Outpost 7's `WO_SNOW` plays too. It was withheld until 2026-10-04 because
/// ours drew no flakes past its first frames while the original kept 64: the
/// emitter sets the immortal flag `0x800`, which `oag_fx::psys` did not
/// honour. With it honoured, its faint speckle matches the original's A/B at
/// the same spot. See `docs/ghidra/functions/psp-pulse-usa/weather.md`.
fn place_weather(
    loaded: &mut Loaded,
    track_blob: &[u8],
    nodes: &[oag_vex::vex::Node],
    archives: &mut oag_assets::Archives,
    track: &str,
) {
    let Some(config) =
        oag_sound::sfx::circuit_manifest(archives, track).and_then(|manifest| manifest.weather)
    else {
        return;
    };
    if loaded.setup.zone.is_some() {
        loaded
            .report
            .push("weather: authored, and not built in Zone, as the original skips it".into());
        return;
    }
    let anchors = oag_vex::weather::anchors(track_blob, nodes);
    loaded.report.push(format!(
        "weather: {} on {} covered section(s), the effect ridden on the camera in the open and \
         held at the section's anchor under cover",
        config.env_psys.as_deref().unwrap_or("no EnvPsys"),
        anchors.len()
    ));
    let mist_texture = mist_texture(archives, config.tex.as_deref(), &mut loaded.report);
    loaded.setup.scenery_fx.weather = Some(crate::scenery_fx::weather::Setup {
        config,
        anchors,
        mist_texture,
    });
}

#[cfg(feature = "view")]
/// The mist overlay's `Tex`, decoded, or `None` with the reason in the report:
/// an absent or undecodable texture draws no mist rather than a stand-in.
fn mist_texture(
    archives: &mut oag_assets::Archives,
    tex: Option<&str>,
    report: &mut Vec<String>,
) -> Option<oag_fx::exhaust::FlareTexture> {
    let name = tex?;
    let blob = match archives.read_name(name) {
        Ok(blob) => blob,
        Err(error) => {
            report.push(format!(
                "mist: {name}: not in the archive set ({error}), not drawn"
            ));
            return None;
        }
    };
    let decoded = match oag_texture::texture::Texture::parse(&blob) {
        Ok(texture) => Ok((texture.width, texture.height, texture.to_rgba())),
        Err(error) => oag_texture::ps2_texture::parse(&blob)
            .map(|texture| (texture.width, texture.height, texture.to_rgba()))
            .map_err(|_| error),
    };
    match decoded {
        Ok((width, height, rgba)) => {
            report.push(format!(
                "mist: {name}, {width}x{height}, two additive screen layers under the HUD"
            ));
            Some(oag_fx::exhaust::FlareTexture {
                width: u32::from(width),
                height: u32::from(height),
                rgba,
            })
        }
        Err(error) => {
            report.push(format!("mist: {name}: {error:#}, not drawn"));
            None
        }
    }
}
