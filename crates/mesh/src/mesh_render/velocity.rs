//! [`Velocity`]: whether a mesh pipeline draws into the race's velocity
//! attachment as well as its colour one.
//!
//! Split out of `mesh_render.rs` under the 1,000-line rule in
//! `scripts/check-file-size.py`; a move, with no behaviour change.

/// Whether a pipeline draws into the race's velocity attachment as well as
/// its colour one.
///
/// An enum rather than a `bool` for the reason [`Depth`] is: a missing
/// velocity write is invisible in the final image, so a call site has to say
/// which it means. The race scene passes `Write` for everything it draws;
/// the asset viewer and the pipeline tests pass `None` and get exactly the
/// single-target pipelines that existed before the velocity buffer.
///
/// Under `Write`, the depth-writing pipelines (opaque and alpha-test) output
/// their surface's real screen-space motion - `shaders/velocity.wesl`'s `velocity_of` -
/// and the blended pipelines carry the second target **with an empty write
/// mask**: they write no depth, so the velocity at their pixels belongs to
/// the surface behind them, and masking the write is what keeps velocity and
/// depth describing the same surface at every pixel. See
/// `docs/rendering/motion-blur.md`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Velocity {
    /// One colour target, no velocity - every path outside a race.
    #[default]
    None,
    /// The race's two attachments: colour, then [`oag_gpu::formats::VELOCITY_FORMAT`].
    Write,
}

impl Velocity {
    /// The second colour target this choice adds, if any. `masked` empties
    /// the write mask - the blended-pipeline case above.
    pub fn target(self, masked: bool) -> Option<Option<wgpu::ColorTargetState>> {
        match self {
            Self::None => None,
            Self::Write => Some(Some(wgpu::ColorTargetState {
                format: oag_gpu::formats::VELOCITY_FORMAT,
                blend: None,
                write_mask: if masked {
                    wgpu::ColorWrites::empty()
                } else {
                    wgpu::ColorWrites::ALL
                },
            })),
        }
    }

    /// The entry point and second target of a blended pass. With `coverage`, the
    /// pass writes zero motion weighted by its own alpha (source-alpha over)
    /// through its `_velocity` twin, so the glow does not take the screen motion
    /// of the surface behind it; without, the write mask stays empty.
    pub fn blended(
        self,
        base: &'static str,
        twin: &'static str,
        coverage: bool,
    ) -> (&'static str, Option<Option<wgpu::ColorTargetState>>) {
        if !coverage {
            return (base, self.target(true));
        }
        let target = self.target(false).map(|target| {
            target.map(|mut target| {
                target.blend = Some(wgpu::BlendState::ALPHA_BLENDING);
                target
            })
        });
        (self.entry(base, twin), target)
    }

    /// The fragment entry point for the depth-writing pipelines: `base`, or
    /// its `_velocity` twin when this pipeline also writes the buffer.
    pub(crate) fn entry(self, base: &'static str, velocity: &'static str) -> &'static str {
        match self {
            Self::None => base,
            Self::Write => velocity,
        }
    }
}

/// The one or two colour targets a mesh pipeline draws into: the colour
/// target always, and the velocity target [`Velocity::Write`] adds.
pub(crate) fn velocity_targets(
    colour: wgpu::ColorTargetState,
    velocity: Option<Option<wgpu::ColorTargetState>>,
) -> Vec<Option<wgpu::ColorTargetState>> {
    let mut targets = vec![Some(colour)];
    targets.extend(velocity);
    targets
}
