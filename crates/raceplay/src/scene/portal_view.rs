//! The race's one secondary view: the scene from the camera through a mod's portal pair
//! (`Race::portal_shot`), drawn into one reused texture that every mod shader reads as
//! `engine_texture`.
//!
//! **It is a view like a split's, with no window**: [`Scene::prepare_view`] and
//! [`Recorder::record`] draw it, on a scene block of its own ([`SceneBlocks::portal`]) and a
//! camera history of its own, into attachments of its own. Three things differ.
//!
//! - **A submission of its own, ahead of the view that shows it.** A view's writes land before
//!   the next submission, not at their place in an encoder (`viewports`' module doc), and the
//!   effect pipelines hold one buffer: sharing a submission would draw one of the two pictures
//!   with the other's matrices.
//! - **It reads the frame's work, it does not do it**: last frame's animation tables and cards,
//!   except for the very first frame of a race, which has none to read (a `--screenshot`
//!   draws only that one). No post chain, no shadows, no jitter, and no PVS: both sets are keyed
//!   to the craft and the camera on this side of the portal.
//! - **No mod shader draws in it**, or the texture it fills would be bound while it is a target,
//!   and a portal would show itself: that is the recursion the design leaves out.
//!
//! **One extra scene a frame at most, under a split too.** The first seat of the frame that asks
//! and has the portal in its view gets the texture. Every other view of that frame reads
//! `engine_texture_bound()` as false and draws the shader's own fallback, which is what keeps the
//! cost of a portal to one scene whatever the number of seats. A later window of a multi-window
//! race is one of those other views. `docs/architecture/wasm-extensions/mod-shaders.md`.

use oag_ext_abi::ViewUpdate;

use super::frame::{PortalPass, ViewSettings};
use super::viewports::Layer;
use super::*;
use crate::extensions::PortalRequest;

/// The texture's size, whatever the window's: a portal is a small part of the picture.
const SIZE: (u32, u32) = (960, 540);

/// The snapshot a [`ViewUpdate::Once`] holds.
#[derive(Debug)]
struct Snapshot {
    slot: usize,
    request: PortalRequest,
    engine_view_projection: Mat4,
}

/// The reused target. Built the first time a view is drawn, so a race with no portal has none.
#[derive(Debug)]
pub(super) struct Target {
    colour: wgpu::TextureView,
    attachments: motion::Attachments,
    snapshot: Option<Snapshot>,
}

impl Target {
    fn new(device: &wgpu::Device, format: wgpu::TextureFormat, sample_count: u32) -> Self {
        let colour = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("portal view"),
            size: wgpu::Extent3d {
                width: SIZE.0,
                height: SIZE.1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                | wgpu::TextureUsages::TEXTURE_BINDING
                | motion::readable_in_tests(),
            view_formats: &[],
        });
        let depth = frame::depth_texture(device, SIZE, sample_count);
        let velocity = motion::velocity_texture(device, SIZE, sample_count);
        let msaa = frame::msaa_color_texture(device, format, SIZE, sample_count);
        Self {
            colour: colour.create_view(&wgpu::TextureViewDescriptor::default()),
            attachments: motion::Attachments::new(&depth, &velocity, msaa.as_ref()),
            snapshot: None,
        }
    }
}

impl Scene {
    /// Draws the frame's secondary view, if a view of `views` (slot and rectangle, in view order)
    /// wants one, into a submission of its own. Nothing at all, not a pass, when none does.
    /// Before the views themselves, and only by the view that does the frame's work.
    pub(super) fn draw_portal_view(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        race: &Race,
        views: &[(usize, (f32, f32, f32, f32))],
        settings: &ViewSettings<'_>,
    ) {
        self.portal_engine.set(None);
        if self.mod_shaders.borrow().is_none() {
            return;
        }
        let drawing = race.viewed_slot();
        let wanted = views.iter().find_map(|&(slot, rect)| {
            let request = race.extension_portal_view(slot)?;
            if let Some(held) = self.held_snapshot(slot, &request) {
                self.portal_engine.set(Some((slot, held)));
                return None;
            }
            race.set_viewpoint(slot);
            let aspect = rect.2.max(1.0) / rect.3.max(1.0);
            let projection = race.projection(aspect, self.far, settings.fov);
            let shot = race.portal_shot(&request, projection)?;
            Some((slot, request, shot, aspect, projection))
        });
        if self.portal_engine.get().is_none()
            && let Some((slot, request, shot, aspect, projection)) = wanted
        {
            race.set_viewpoint(slot);
            self.encode_portal_view(device, queue, race, &shot, aspect, projection, settings);
            self.portal_engine
                .set(Some((slot, shot.engine_view_projection)));
            if let Some(target) = self.portal.borrow_mut().as_mut() {
                target.snapshot = (request.update == ViewUpdate::Once).then_some(Snapshot {
                    slot,
                    request,
                    engine_view_projection: shot.engine_view_projection,
                });
            }
        }
        race.set_viewpoint(drawing);
    }

    /// The matrix of the snapshot that answers `request`, if the target holds it.
    fn held_snapshot(&self, slot: usize, request: &PortalRequest) -> Option<Mat4> {
        let portal = self.portal.borrow();
        let snapshot = portal.as_ref()?.snapshot.as_ref()?;
        (request.update == ViewUpdate::Once
            && snapshot.slot == slot
            && snapshot.request == *request)
            .then_some(snapshot.engine_view_projection)
    }

    fn encode_portal_view(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        race: &Race,
        shot: &crate::portal_view::PortalShot,
        aspect: f32,
        projection: Mat4,
        settings: &ViewSettings<'_>,
    ) {
        let mut portal = self.portal.borrow_mut();
        let target = portal.get_or_insert_with(|| {
            let target = Target::new(device, self.pass_format, self.sample_count());
            if let Some(stage) = self.mod_shaders.borrow_mut().as_mut() {
                stage.set_engine_texture(device, Some(&target.colour));
            }
            target
        });
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("portal view"),
        });
        let uploads = Uploads::begin(
            queue,
            &self.uniform_blocks,
            SceneBlocks::portal(),
            std::mem::take(&mut *self.upload_staging.borrow_mut()),
        );
        let (layer, reuse, history) = (
            self.layer.replace(Layer::Window),
            // The very first frame has no main view's work to read.
            self.reuse_frame_work
                .replace(self.frame_work.get().scenery_clocks > 0),
            self.history.replace(motion::PORTAL_HISTORY),
        );
        race.set_portal_camera(Some(shot.view));
        let pass_settings = ViewSettings {
            pvs_cull: false,
            shadows: oag_display::display::Shadows::Off,
            camera_jitter: None,
            portal: Some(PortalPass {
                aspect,
                projection,
                exit_plane: shot.exit_plane,
            }),
            ..*settings
        };
        let rect = (0.0, 0.0, SIZE.0 as f32, SIZE.1 as f32);
        let pass = self
            .prepare_view(
                &uploads,
                &mut encoder,
                &target.colour,
                race,
                rect,
                &pass_settings,
                None,
            )
            .without_view_surfaces();
        self.recorder()
            .into_attachments(&target.attachments)
            .record(&pass, &mut encoder);
        drop(pass);
        race.set_portal_camera(None);
        self.layer.set(layer);
        self.reuse_frame_work.set(reuse);
        self.history.set(history);
        *self.upload_staging.borrow_mut() = uploads.into_staging();
        queue.submit(Some(encoder.finish()));
        self.frame_work.update(|w| FrameWork {
            portal_views: w.portal_views + 1,
            ..w
        });
    }

    /// Whether a target was ever built.
    #[cfg(test)]
    pub(crate) fn has_portal_target(&self) -> bool {
        self.portal.borrow().is_some()
    }

    /// The matrix `engine_uv` projects with in the view being drawn, `None` where the frame's
    /// texture is not this view's.
    pub(crate) fn portal_engine_of(&self, race: &Race) -> Option<[[f32; 4]; 4]> {
        let (slot, matrix) = self.portal_engine.get()?;
        (slot == race.viewed_slot()).then(|| matrix.to_cols_array_2d())
    }
}
