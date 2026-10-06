//! What Pulse on the PSP draws differently from every other source, measured
//! live on PPSSPP: the hull's GE lights and the bloom's stencil-stamped glow
//! mask. Neither has been measured on the PS2 port, Pure or any later title,
//! so both stay behind [`is_pulse_psp`]. The Quake's road ripple joins them:
//! it is read off the PSP executable and moves PSP-shaped GE batches.
//!
//! See `docs/ghidra/functions/psp-pulse-usa/scene-light.md` and
//! `docs/rendering/glow-mask.md`.

use oag_mesh::mesh_render;

use super::Loaded;

/// Whether this race is Pulse off a PSP disc.
pub(super) fn is_pulse_psp(title: &oag_title::Title, archives: &oag_assets::Archives) -> bool {
    title.looks.measured_draws.applies(archives.layout.platform)
}

/// Lights the hulls with the circuit's own `AmbientLight`/`DirectionalLight`
/// nodes and marks every `.vex` model the race draws as stamping the glow
/// mask. A no-op when `pulse_psp` is false.
pub(super) fn finish(
    loaded: &mut Loaded,
    pulse_psp: bool,
    track_blob: &[u8],
    (archives, track): (&mut oag_assets::Archives, &str),
) {
    if !pulse_psp {
        return;
    }
    let nodes = oag_vex::vex::nodes(track_blob).unwrap_or_default();
    place_scenery_fx(loaded, track_blob, &nodes);
    place_weather(loaded, track_blob, &nodes, archives, track);
    match mesh_render::HullLights::from_track(track_blob, &nodes, 255) {
        Some(hull) => {
            loaded.report.push(format!(
                "hull lights: the circuit's own, ambient {:?}, diffuse {:?}",
                hull.ambient, hull.diffuse
            ));
            loaded.light.hull = hull;
        }
        None => loaded
            .report
            .push("hull lights: the circuit authors none; hulls keep the stand-in rig".into()),
    }

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

    super::glow_mask::stamp_models(loaded, false);
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
    loaded.report.push(
        "glow mask: the track, sky, pads, hulls, plumes, shields and weapon bodies stamp it as the \
         original's stencil does, a blended batch with the glow bits through a second draw"
            .into(),
    );
}

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
    match oag_texture::texture::Texture::parse(&blob) {
        Ok(texture) => {
            report.push(format!(
                "mist: {name}, {}x{}, two additive screen layers under the HUD",
                texture.width, texture.height
            ));
            Some(oag_fx::exhaust::FlareTexture {
                width: u32::from(texture.width),
                height: u32::from(texture.height),
                rgba: texture.to_rgba(),
            })
        }
        Err(error) => {
            report.push(format!("mist: {name}: {error:#}, not drawn"));
            None
        }
    }
}
