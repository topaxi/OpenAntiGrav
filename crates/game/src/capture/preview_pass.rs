//! The 3-D pass a selection screen's capture owes its frame once the draw
//! list is rendered: [`PreviewRequest`] says what to draw and [`draw_preview`]
//! draws it. Split out of [`super::menu_page`] under the 1,000-line rule in
//! `scripts/check-file-size.py`.

use anyhow::Result;
use oag_mesh::mesh_render::Anisotropy;

/// What `--menu-page track-select`/`ship-select` still owes the frame once
/// its draw list is rendered: the preview mesh, which is a 3D pass rather
/// than a draw - see [`crate::preview`].
pub(super) struct PreviewRequest {
    /// The mesh's archive entry name.
    pub entry: String,
    /// The skin `.dat` to paint it in, when the settings name one the team
    /// declares.
    pub skin: Option<String>,
    /// Where it goes, in the screen's grid - [`oag_ui_screens::picker::Layout::preview`].
    pub rect: [f32; 4],
    pub kind: oag_ui_screens::picker::Kind,
    /// The selected circuit's own `<Mode3D><Model>` pose, when its
    /// `screen.xml` authors one - see
    /// [`oag_game::preview::mode3d_view_projection`]. `None` falls back to
    /// [`oag_game::preview::orbit_for`], same as the live screen.
    pub mode3d: Option<oag_ui::screen::Model>,
    /// The craft is the race hull of a title with no `ship_FE.vex`, drawn at
    /// the fixed [`crate::preview::hull_orbit`] - see
    /// [`oag_title::FrontEnd::ship_preview_hull`].
    pub hull_only: bool,
    /// HD's `TrackModel` widget: places the circuit model on its own camera,
    /// see [`crate::preview::track_model`]. Takes the place of `mode3d` and
    /// the orbit.
    pub track_model: Option<oag_ui_screens::picker::hd::track::TrackModel>,
    /// How far into the screen the capture is, for the circuit model's
    /// turntable: `--menu-picker-seconds`, or `0.0` settled.
    pub seconds: f32,
}

/// The source a capture's race options name, opened with its packs, for the
/// selection screens' own reads.
pub(super) fn open_for_previews(race: &oag_raceplay::Options) -> Result<oag_assets::Archives> {
    let (packs, pure_packs, problems) = oag_source::dlc::packs_from_defaults(
        &race.dlc,
        &oag_source::cache::default_dlc_cache_dir(),
    );
    for problem in problems {
        log::warn!("{problem}");
    }
    Ok(oag_source::title::open_source(&race.source, packs, pure_packs)?.archives)
}

/// The selection screen's preview mesh, over the finished draw list - the
/// same pass `MenuStage::render` runs live, at the screen's first tick.
///
/// A mesh that will not read or build is logged and draws nothing, per the
/// rule for an asset that will not play; the rest of the capture stands.
#[allow(
    clippy::too_many_arguments,
    reason = "one call site, each a separate fact of the frame"
)]
pub(super) fn draw_preview(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    encoder: &mut wgpu::CommandEncoder,
    view: &wgpu::TextureView,
    viewport: (f32, f32, f32, f32),
    target_size: (u32, u32),
    space: oag_display::space::Space,
    race: &oag_raceplay::Options,
    request: &PreviewRequest,
    anisotropy: Anisotropy,
) {
    let built = open_for_previews(race).and_then(|mut archives| {
        let mut model = if request.track_model.is_some() {
            crate::preview::psp2_scene::circuit_model(&mut archives, &request.entry)?
        } else {
            crate::preview::model(&mut archives, &request.entry)?
        };
        // The chosen paint, the same swap the live screen and a race make;
        // a skin that will not read leaves the hull's own and says so.
        if let Some(entry) = &request.skin {
            for line in crate::preview::paint(&mut archives, entry, &request.entry, &mut model) {
                log::debug!("preview {}: {line}", request.entry);
            }
        }
        if request.hull_only {
            crate::preview::frame_hull(&mut model);
        }
        let ramp = if request.track_model.is_some() {
            Some(crate::preview::track_model::Ramp::of(
                &mut archives,
                &request.entry,
                &mut model,
            )?)
        } else {
            None
        };
        let preview = crate::preview::Preview::new(
            device,
            queue,
            wgpu::TextureFormat::Rgba8Unorm,
            anisotropy,
            model,
        )?;
        Ok(match ramp {
            Some(ramp) => preview.with_ramp(ramp),
            None => preview,
        })
    });
    match built {
        Ok(mut preview) if request.track_model.is_some() => {
            let widget = request.track_model.as_ref().expect("checked by the guard");
            preview.draw_track_model(
                device,
                queue,
                encoder,
                view,
                viewport,
                target_size,
                space,
                widget,
                request.seconds,
            );
        }
        Ok(mut preview) => preview.draw_auto(
            device,
            queue,
            encoder,
            view,
            viewport,
            target_size,
            space,
            request.mode3d.as_ref(),
            request.rect,
            crate::preview::capture_orbit(request.hull_only, request.kind),
            0.0,
        ),
        Err(error) => log::warn!("{}: {error:#} - the preview draws nothing", request.entry),
    }
}
