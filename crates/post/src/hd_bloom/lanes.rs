//! The adaptation a picture keeps, and the chain run over one view of a split.
//!
//! **A lane is everything the chain keeps from one frame to the next**: the 1x1 adapted
//! luminance (two textures, written by turns), which of them the next frame writes, and
//! whether the very first frame's rate-1 jump is still owed. The passes that read it
//! (adapt, gate, resolve) are bound to it. The window's own picture has one lane; a split
//! screen keeps one **per seat** (`Chain::run_view`), so a view in a dark tunnel and a view
//! in daylight each settle on their own exposure. The adaptation law is the single-view
//! one in every lane: same rate, same clamps, only the pixels that feed it are the
//! view's own. **The per-seat lane is this project's, chosen, not measured**: the original
//! has no split screen.
//!
//! Every lane shares the chain's scratch levels (the half and quarter buffers, the
//! reduction ladder): a view's chain runs to its end, in its own submission, before the
//! next view's starts, so nothing in them outlives the view that wrote it.

use std::cell::{Cell, RefCell};

use super::{Chain, Pass, Target};

/// Seats a split can hold (`oag_race` allows 8), and so the most lanes besides the window's.
pub const MAX_SEATS: usize = 8;

/// What a lane's passes read that is not its own: the chain's shared views and buffers.
#[derive(Debug, Clone)]
pub(super) struct LaneInputs {
    pub scene: wgpu::TextureView,
    pub quarter_a: wgpu::TextureView,
    pub quarter_b: wgpu::TextureView,
    /// The last reduction level, whose texels the adaptation averages.
    pub mean: wgpu::TextureView,
    /// The constants of every pass that reads scratch alone.
    pub still: wgpu::Buffer,
    /// The constants of the two passes that read the scene target: the first downsample's
    /// and the resolve's.
    pub scene_read: wgpu::Buffer,
    /// The scene target's size and the quarter level's.
    pub extent: (u32, u32),
    pub quarter: (u32, u32),
}

/// One picture's adaptation state and the passes bound to it.
#[derive(Debug)]
pub(super) struct Lane {
    #[expect(dead_code, reason = "held so the views stay valid")]
    adapted: [Target; 2],
    /// `adapt[i]` writes ping-pong texture `i`, reading the other as the previous state.
    pub adapt: [Pass; 2],
    /// `gate[i]` and `encode[i]` run after `adapt[i]` and read the state it wrote.
    pub gate: [Pass; 2],
    pub encode: [Pass; 2],
    /// Which ping-pong texture the next frame writes.
    pub current: Cell<usize>,
    /// Whether the adaptation still holds its zero-initialised value, so the next frame
    /// jumps to its own luminance instead of lerping toward it from black.
    pub fresh: Cell<bool>,
}

impl Lane {
    pub(super) fn new(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        sampler: &wgpu::Sampler,
        inputs: &LaneInputs,
    ) -> Self {
        let adapted = [
            Target::new(device, "hd bloom adapted a", (1, 1)),
            Target::new(device, "hd bloom adapted b", (1, 1)),
        ];
        let pass = |label: &str,
                    source: &wgpu::TextureView,
                    state: &wgpu::TextureView,
                    bloom: &wgpu::TextureView,
                    buffer: &wgpu::Buffer,
                    target: &wgpu::TextureView,
                    level: (u32, u32)| Pass {
            group: super::bind_group(
                device,
                layout,
                sampler,
                label,
                [source, state, bloom],
                buffer,
            ),
            target: target.clone(),
            level,
        };
        let adapt = [
            pass(
                "hd bloom adapt a",
                &inputs.mean,
                &adapted[1].view,
                &inputs.mean,
                &inputs.still,
                &adapted[0].view,
                (1, 1),
            ),
            pass(
                "hd bloom adapt b",
                &inputs.mean,
                &adapted[0].view,
                &inputs.mean,
                &inputs.still,
                &adapted[1].view,
                (1, 1),
            ),
        ];
        let gate = [
            pass(
                "hd bloom gate a",
                &inputs.quarter_a,
                &adapted[0].view,
                &inputs.quarter_a,
                &inputs.still,
                &inputs.quarter_b,
                inputs.quarter,
            ),
            pass(
                "hd bloom gate b",
                &inputs.quarter_a,
                &adapted[1].view,
                &inputs.quarter_a,
                &inputs.still,
                &inputs.quarter_b,
                inputs.quarter,
            ),
        ];
        let encode = [
            pass(
                "hd encode a",
                &inputs.scene,
                &adapted[0].view,
                &inputs.quarter_b,
                &inputs.scene_read,
                &inputs.scene,
                inputs.extent,
            ),
            pass(
                "hd encode b",
                &inputs.scene,
                &adapted[1].view,
                &inputs.quarter_b,
                &inputs.scene_read,
                &inputs.scene,
                inputs.extent,
            ),
        ];
        Self {
            adapted,
            adapt,
            gate,
            encode,
            current: Cell::new(0),
            fresh: Cell::new(true),
        }
    }
}

/// The seats' lanes, built when a split first asks for them.
#[derive(Debug, Default)]
pub(super) struct SeatLanes(RefCell<Vec<Option<Lane>>>);

impl SeatLanes {
    pub(super) fn built(&self) -> usize {
        self.0.borrow().iter().flatten().count()
    }

    pub(super) fn clear(&self) {
        self.0.borrow_mut().clear();
    }

    pub(super) fn get_or_build(
        &self,
        seat: usize,
        build: impl FnOnce() -> Lane,
    ) -> std::cell::Ref<'_, Lane> {
        {
            let mut lanes = self.0.borrow_mut();
            if lanes.len() <= seat {
                lanes.resize_with(seat + 1, || None);
            }
            if lanes[seat].is_none() {
                lanes[seat] = Some(build());
            }
        }
        std::cell::Ref::map(self.0.borrow(), |lanes| {
            lanes[seat].as_ref().expect("built above")
        })
    }
}

/// One view of a split, as the chain runs over it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    /// The seat, whose lane keeps this view's adaptation from frame to frame.
    pub seat: usize,
    /// The view's rectangle, `x, y, width, height`: where it was drawn in the scene target
    /// and where its picture goes in the frame (the two are the same place).
    pub rect: (u32, u32, u32, u32),
    /// The whole picture the views were cut from, in pixels. The blur's tap spacing is
    /// authored against a whole frame, so it is measured against this and not against the
    /// view.
    pub area: (u32, u32),
    /// How far the glow reaches across and down against what [`Params`](super::Params)
    /// authors. **Chosen, not measured.** `(1.0, 1.0)` for a whole picture.
    pub reach: (f32, f32),
    /// Whether this is the first view of the frame, which clears the whole frame to opaque
    /// black before it resolves (the gap between views, and any stale frame under it); every
    /// later view loads what the ones before it resolved.
    pub first: bool,
}

impl Chain {
    /// Runs the chain over one view of a split: its own half-size ladder, its own
    /// adaptation (the seat's lane), its glow clamped to its rectangle, and its resolve into
    /// the same rectangle of `view`. **Call it once per view with its own encoder, and
    /// submit each before the next** - the constants are written with the queue, which
    /// lands before the submission that follows.
    ///
    /// Reads no zoom streak (it holds the primary craft's pulse).
    pub fn run_view(
        &self,
        queue: &wgpu::Queue,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        view: &wgpu::TextureView,
        run: View,
    ) {
        let seat = run.seat.min(MAX_SEATS - 1);
        let lane = self.seats.get_or_build(seat, || {
            Lane::new(device, &self.layout, &self.sampler, &self.sized.inputs)
        });
        let (x, y, w, h) = run.rect;
        self.run_placed(
            queue,
            encoder,
            view,
            &lane,
            super::Placement {
                source: (x, y),
                dest: (x as f32, y as f32),
                viewport: (w, h),
                reference: run.area,
                reach: run.reach,
                load: if run.first {
                    wgpu::LoadOp::Clear(wgpu::Color::BLACK)
                } else {
                    wgpu::LoadOp::Load
                },
                zoom: false,
            },
            None,
        );
    }

    /// How many seats hold an adaptation of their own right now.
    #[must_use]
    pub fn seat_lanes(&self) -> usize {
        self.seats.built()
    }

    /// Forgets every seat's adaptation, for the first whole-picture frame after a split: a
    /// seat that comes back later settles from its own first frame again instead of from
    /// the luminance it had before the gap.
    pub fn retire_seats(&self) {
        self.seats.clear();
    }
}
