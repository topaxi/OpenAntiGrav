//! Building Wipeout HD's post chain for [`super::Scene::new`].
//!
//! Split out for the 1,000-line rule in `scripts/check-file-size.py`; a
//! move, with no behaviour change - `Scene::new` had this inline before the
//! Cannon's own two quads needed a field, a parameter and a construction
//! call there and left it with nowhere to put them.

use super::*;

/// Wipeout HD's post chain: a linear float scene target, the read
/// FunkLayerBloom passes and the encode. Present exactly when the circuit
/// authors an `HDR and Bloom` block - see `oag_post::hd_bloom` for
/// what of it is the microcode's. A failure is reported and dropped the way
/// the PSP bloom's is: a race without it is the pre-HDR picture, not a
/// broken one.
///
/// **The glow always draws.** There is no bloom setting: the original offers
/// none in any title, so the only faithful strength is the title's own
/// (maintainer, 2026-10-02). `Glow::Suppressed` stays in `oag_render` for its
/// own tests; nothing here asks for it.
///
/// Returns the chain itself (or `None`), the caller's own surface format -
/// **the format is the statement about colour space**: with the chain in
/// place every pipeline built after this call switches to the linear float
/// target and its own linear output, see `mesh_render::is_linear_target` -
/// and that caller format again, unshadowed, for the one pass that runs
/// after every chain has already encoded into the caller's view (see
/// `motion_blur` in `Scene::new`).
pub(super) fn build(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    size: (u32, u32),
    hd_bloom: Option<oag_post::hd_bloom::Params>,
) -> (
    Option<oag_post::hd_bloom::Chain>,
    wgpu::TextureFormat,
    wgpu::TextureFormat,
) {
    let glow = oag_post::hd_bloom::Glow::Drawn;
    let hd = match hd_bloom
        .map(|params| oag_post::hd_bloom::Chain::new(device, format, size, params, glow))
        .transpose()
    {
        Ok(hd) => hd,
        Err(e) => {
            warn!("hd post chain unavailable ({e}) - the frame draws without it");
            None
        }
    };
    let caller_format = format;
    let format = if hd.is_some() {
        oag_gpu::formats::SCENE_FORMAT
    } else {
        format
    };
    (hd, format, caller_format)
}

/// Omega's tone map (`oag_post::omega_tonemap`), built when the
/// circuit authors a `Tonemap` block and no HD chain already owns the scene
/// target. Returns it and the scene format, switched to the linear float one
/// when it is there - the same statement [`build`] makes. A failure is
/// reported and dropped: the race then draws the pre-tonemap picture.
pub(super) fn build_omega(
    device: &wgpu::Device,
    format: wgpu::TextureFormat,
    caller_format: wgpu::TextureFormat,
    size: (u32, u32),
    params: Option<oag_post::omega_tonemap::Params>,
) -> (Option<oag_post::omega_tonemap::Chain>, wgpu::TextureFormat) {
    let omega = match params
        .filter(|_| format == caller_format)
        .map(|params| oag_post::omega_tonemap::Chain::new(device, caller_format, size, params))
        .transpose()
    {
        Ok(omega) => omega,
        Err(e) => {
            warn!("omega tone map unavailable ({e}) - the frame draws without it");
            None
        }
    };
    let format = if omega.is_some() {
        oag_gpu::formats::SCENE_FORMAT
    } else {
        format
    };
    (omega, format)
}

impl Scene {
    /// The linear float target the race pass draws into under either post
    /// chain, or `None` when it draws straight into the caller's view.
    pub(super) fn linear_scene_view(&self) -> Option<&wgpu::TextureView> {
        match (&self.hd, &self.omega) {
            (Some(hd), _) => Some(hd.scene_view()),
            (None, Some(omega)) => Some(omega.scene_view()),
            (None, None) => None,
        }
    }

    /// Whether the scene's pipelines were built against the linear float
    /// target - see [`build`] and [`build_omega`].
    pub(super) fn draws_linear(&self) -> bool {
        self.hd.is_some() || self.omega.is_some()
    }
}

/// The borrowed zoom-streak ring (`graphics.boost_zoom`): built at the first frame that
/// asks for it, in the caller's frame format, so a race that borrows nothing builds
/// nothing. See [`oag_post::hd_zoom::Host`].
#[derive(Debug)]
pub(super) struct BoostZoom {
    host: std::cell::RefCell<Option<oag_post::hd_zoom::Host>>,
    format: wgpu::TextureFormat,
}

impl BoostZoom {
    pub(super) fn new(format: wgpu::TextureFormat) -> Self {
        Self {
            host: std::cell::RefCell::new(None),
            format,
        }
    }
}

impl Scene {
    /// Wipeout HD's chain over this frame, with the zoom-streak ring's pulse (stepped
    /// once a tick; `None` draws none).
    pub(super) fn run_hd(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        zoom: Option<oag_post::hd_zoom::Frame>,
        timestamps: Option<oag_post::hd_bloom::ChainTimestamps<'_>>,
    ) {
        let Some(hd) = &self.hd else {
            return;
        };
        hd.set_zoom_frame(zoom);
        hd.run(
            queue,
            encoder,
            view,
            (viewport.0, viewport.1),
            (viewport.2 as u32, viewport.3 as u32),
            timestamps,
        );
    }

    /// Runs the borrowed zoom-streak ring over `view`, building its host at the first
    /// frame that has a pulse to draw. Nothing is built, bound or encoded for a race
    /// that borrows no ring, which is every race with `graphics.boost_zoom` off.
    pub(super) fn run_boost_zoom(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        viewport: (f32, f32, f32, f32),
        race: &Race,
    ) {
        let Some(tuning) = race.borrowed_zoom_tuning() else {
            return;
        };
        let Some(frame) = race.hd_zoom_frame() else {
            return;
        };
        let mut host = self.boost_zoom.host.borrow_mut();
        let host = host
            .get_or_insert_with(|| oag_post::hd_zoom::Host::new(tuning, self.boost_zoom.format));
        let size = self.depth.size();
        host.run(
            device,
            queue,
            encoder,
            view,
            (size.width, size.height),
            (viewport.0 as u32, viewport.1 as u32),
            (viewport.2 as u32, viewport.3 as u32),
            &frame,
        );
    }
}
