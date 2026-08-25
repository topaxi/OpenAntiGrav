//! [`MotionState`]: the previous tick's matrices, and the velocity
//! attachment they give meaning to.
//!
//! Split out of `frame.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

use super::super::*;

/// The previous tick's camera and model matrices - what every drawable's
/// `prev_mvp` is measured against, and therefore what the velocity buffer
/// means.
///
/// Keyed by the simulation tick, not the frame, which is the design's
/// non-obvious decision (`docs/rendering/motion-blur.md`): frames can outrun
/// the fixed 60 Hz simulation, and consecutive frames of one tick re-render
/// identical state. Snapshotting per frame would make those repeats read as
/// zero motion and pulse the blur off; promoting the snapshot only when the
/// tick advances makes every frame of a tick measure the same one-tick
/// travel. A camera or craft that genuinely holds still simply snapshots
/// equal matrices - zero velocity with no special case.
#[derive(Debug)]
pub(super) struct MotionState {
    tick: u64,
    prev: Snapshot,
    cur: Snapshot,
}

/// One tick's worth of the matrices anything draws with.
#[derive(Clone, Debug)]
pub(super) struct Snapshot {
    pub(super) view_projection: Mat4,
    /// The camera's translation alone, for the sky's own `prev_mvp` - see
    /// the sky write in [`Scene::render`].
    pub(super) camera_translation: Mat4,
    ships: Vec<Mat4>,
    pub(super) rockets: Vec<Mat4>,
}

impl Snapshot {
    fn take(race: &Race, view_projection: Mat4, drawn: usize) -> Self {
        Self {
            view_projection,
            camera_translation: Mat4::from_translation(race.camera_position()),
            ships: (0..drawn)
                .map(|slot| race.ship_model_matrix_of(slot))
                .collect(),
            rockets: race.rocket_model_matrices(),
        }
    }

    /// The slot's previous model matrix, or its current one when the
    /// previous tick had no entry for it - zero object velocity, never a
    /// smear from stale or missing state.
    pub(super) fn ship(&self, slot: usize, race: &Race) -> Mat4 {
        self.ships
            .get(slot)
            .copied()
            .unwrap_or_else(|| race.ship_model_matrix_of(slot))
    }
}

impl MotionState {
    /// Folds this frame in and returns the snapshot to measure against.
    pub(super) fn advance(
        cell: &std::cell::RefCell<Option<Self>>,
        race: &Race,
        view_projection: Mat4,
        drawn: usize,
    ) -> Snapshot {
        let mut state = cell.borrow_mut();
        match state.as_mut() {
            // A scene's first frame measures against itself: zero velocity,
            // not a smear from an uninitialised matrix.
            None => {
                let now = Snapshot::take(race, view_projection, drawn);
                *state = Some(Self {
                    tick: race.world.tick,
                    prev: now.clone(),
                    cur: now.clone(),
                });
                now
            }
            Some(state) if state.tick != race.world.tick => {
                let now = Snapshot::take(race, view_projection, drawn);
                state.prev = std::mem::replace(&mut state.cur, now);
                state.tick = race.world.tick;
                state.prev.clone()
            }
            // The same tick re-rendered - a frame rate above 60 Hz - keeps
            // measuring against the same previous tick.
            Some(state) => state.prev.clone(),
        }
    }
}

/// The scene's velocity attachment - see `Scene::velocity`.
pub(super) fn velocity_texture(
    device: &wgpu::Device,
    size: (u32, u32),
    sample_count: u32,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("race velocity"),
        size: wgpu::Extent3d {
            width: size.0.max(1),
            height: size.1.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count,
        dimension: wgpu::TextureDimension::D2,
        format: oag_render::mesh_render::VELOCITY_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}
