//! The race's scene blocks: one buffer, a block per view and per kind of scene, bound by a
//! dynamic offset.
//!
//! A drawable's bind group 2 holds a [`mesh_render::Scene`] (the fog, the light rig, the
//! clock, the shadow map and the Zone grade, about a kilobyte). Every drawable used to own a
//! buffer for it and the frame wrote the same bytes into each, 37 `queue.write_buffer` calls a
//! view on Pulse's grid. The blocks a frame composes are few - the circuit's, the craft's,
//! the craft's shine pass, the flame surfaces', the behind-the-glass target's alternate
//! fog - so a drawable of one [`SceneKind`] binds that kind's block and the frame writes it
//! once.
//!
//! **A region per view.** Block `(view, kind)` sits at `(view * KINDS + kind) * STRIDE`. Two
//! views of one frame therefore never share a byte: a view's blocks can be written, and its
//! commands recorded, without reading or writing another view's, which is what recording the
//! views of a split on separate threads needs and what a queue write landing before the
//! *submission's* commands (not at its place in the encoder) forbids a shared block. The view
//! is the slot of the craft it follows ([`Race::viewed_slot`]): a craft is followed by one
//! seat, so it is unique within a frame. The offset travels as a [`ViewBlock`] handed to the
//! draw calls, **not stored in the drawable**: a drawable is shared by every view, so any state
//! in it is shared mutable state between views at record time.

use super::*;

/// What a block holds. A drawable is one kind for its life ([`Drawable::share_scene`]), and
/// the frame writes the kind's block with the value every drawable of the kind used to be
/// handed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SceneKind {
    /// The circuit's own scene: the track, its pads, the collision overlay, the start gantry
    /// and the behind-the-glass target's primary.
    Circuit,
    /// The craft's: the circuit's with the Zone half off, for the hulls and the wrecks.
    Ship,
    /// The hulls' and the circuit's shine passes: the craft's with the fog colour black.
    Shine,
    /// The flame surfaces' (flares and boost plumes): `Scene::off` plus the clock.
    Flame,
    /// The behind-the-glass target's alternate fog.
    GlassAlternate,
}

impl SceneKind {
    const COUNT: usize = 5;

    fn index(self) -> u64 {
        self as u64
    }
}

/// Where the blocks of the view being drawn start: the dynamic offsets' view half, one base for
/// the scene blocks (bind group 2) and one for the drawables' matrices (bind group 0).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ViewBlock {
    scene: u32,
    uniforms: u32,
}

impl ViewBlock {
    /// For a draw outside any view (an advert card's own texture): the drawable's own blocks.
    pub(crate) const NONE: Self = Self {
        scene: 0,
        uniforms: 0,
    };
}

/// The buffer behind every shared drawable's bind group 2.
#[derive(Debug)]
pub(crate) struct SceneBlocks {
    buffer: wgpu::Buffer,
    /// Which kinds a drawable was moved onto: an unused kind's block is never written.
    used: [bool; SceneKind::COUNT],
}

impl SceneBlocks {
    /// A view per craft slot, because a view is named by the craft it follows.
    const VIEWS: usize = GRID_SLOTS as usize;

    /// Every block holding [`mesh_render::Scene::off`], so a drawable drawn before its kind is
    /// first written reads what its own buffer held.
    pub(crate) fn new(device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        let blocks = Self::VIEWS * SceneKind::COUNT;
        let size = blocks as u64 * mesh_render::SCENE_STRIDE;
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scene blocks"),
            size,
            usage: wgpu::BufferUsages::UNIFORM
                | wgpu::BufferUsages::COPY_DST
                | wgpu::BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });
        let mut bytes = vec![0u8; size as usize];
        let off = mesh_render::Scene::off();
        for block in 0..blocks {
            let at = block * mesh_render::SCENE_STRIDE as usize;
            bytes[at..at + mesh_render::SCENE_SIZE as usize]
                .copy_from_slice(bytemuck::bytes_of(&off));
        }
        oag_gpu::deferred_upload::write_buffer(queue, &buffer, bytes);
        Self {
            buffer,
            used: [false; SceneKind::COUNT],
        }
    }

    /// The first blocks of the view following craft `slot`.
    pub(crate) fn view(slot: usize) -> ViewBlock {
        let view = slot.min(Self::VIEWS - 1) as u64;
        ViewBlock {
            scene: (view * SceneKind::COUNT as u64 * mesh_render::SCENE_STRIDE) as u32,
            uniforms: (view * UniformBlocks::SLOTS as u64 * mesh_render::UNIFORMS_STRIDE) as u32,
        }
    }

    /// Writes `scene` as `kind`'s block of `view`: one `queue.write_buffer` for every drawable
    /// of the kind. Nothing when no drawable is of the kind.
    pub(crate) fn write(
        &self,
        queue: &wgpu::Queue,
        view: ViewBlock,
        kind: SceneKind,
        scene: &mesh_render::Scene,
    ) {
        if !self.used[kind as usize] {
            return;
        }
        let at = u64::from(view.scene) + kind.index() * mesh_render::SCENE_STRIDE;
        oag_gpu::perfprobe::write_buffer(queue, &self.buffer, at, bytemuck::bytes_of(scene));
    }
}

#[cfg(test)]
impl SceneBlocks {
    /// `kind`'s block of the view following craft `slot`, read back.
    pub(crate) fn read(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        slot: usize,
        kind: SceneKind,
    ) -> mesh_render::Scene {
        let at = u64::from(Self::view(slot).scene) + kind.index() * mesh_render::SCENE_STRIDE;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("scene block readback"),
            size: mesh_render::SCENE_SIZE,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(&self.buffer, at, &readback, 0, mesh_render::SCENE_SIZE);
        queue.submit([encoder.finish()]);
        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("the device polls");
        let mapped = slice.get_mapped_range().expect("the block mapped");
        bytemuck::pod_read_unaligned(&mapped)
    }
}

impl Drawable {
    /// Moves this drawable's bind group 2 onto `blocks` as a `kind` drawable. From here the
    /// frame writes its scene through [`SceneBlocks::write`] and [`Self::write_scene`] is not
    /// to be called; the draws bind [`Self::scene_offset`].
    ///
    /// **Call it straight after the build**, before [`Self::rebind_zone`] has replaced the stage
    /// the group was built with.
    pub(crate) fn share_scene(
        &mut self,
        device: &wgpu::Device,
        blocks: &mut SceneBlocks,
        kind: SceneKind,
    ) {
        blocks.used[kind as usize] = true;
        self.fog_bind = mesh_render::zone::rebind_buffer(&blocks.buffer, device, &self.zone_rebind);
        self.fog = blocks.buffer.clone();
        self.scene_kind = Some(kind);
    }

    /// The dynamic offset of the block this drawable binds in `view`.
    pub(super) fn scene_offset(&self, view: ViewBlock) -> [u32; 1] {
        [self.scene_kind.map_or(0, |kind| {
            view.scene + (kind.index() * mesh_render::SCENE_STRIDE) as u32
        })]
    }
}

/// The buffer behind bind group 0 of every shared drawable: its matrices, a slot of
/// [`mesh_render::UNIFORMS_STRIDE`] bytes each, a region of [`Self::SLOTS`] slots per view.
///
/// A drawable used to own a buffer for them and the frame wrote each with its own
/// `queue.write_buffer` (28 a view on Pulse's grid). A shared drawable stages its block
/// through [`Uploads`] and the view writes the slots it staged in runs of neighbours (about
/// five calls); the draw binds the same buffer at `view region + slot * stride`. **Slots a view
/// did not stage keep what that view's region held**, which is what the old per-drawable buffer
/// did for a drawable nothing wrote that frame.
#[derive(Debug)]
pub(crate) struct UniformBlocks {
    buffer: wgpu::Buffer,
    /// The next slot to hand out.
    next: u32,
}

/// A drawable's view-projection and model matrices as the shader reads them.
#[cfg(test)]
pub(crate) type Matrices = ([[f32; 4]; 4], [[f32; 4]; 4]);

#[cfg(test)]
impl UniformBlocks {
    /// The view-projection and model matrices in `slot` of the view following craft `view`,
    /// read back.
    pub(crate) fn read(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        view: usize,
        slot: u32,
    ) -> Matrices {
        let at = u64::from(SceneBlocks::view(view).uniforms)
            + u64::from(slot) * mesh_render::UNIFORMS_STRIDE;
        let readback = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniform block readback"),
            size: mesh_render::UNIFORMS_SIZE,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
            mapped_at_creation: false,
        });
        let mut encoder = device.create_command_encoder(&Default::default());
        encoder.copy_buffer_to_buffer(&self.buffer, at, &readback, 0, mesh_render::UNIFORMS_SIZE);
        queue.submit([encoder.finish()]);
        let slice = readback.slice(..);
        slice.map_async(wgpu::MapMode::Read, |_| {});
        device
            .poll(wgpu::PollType::wait_indefinitely())
            .expect("the device polls");
        let mapped = slice.get_mapped_range().expect("the block mapped");
        let uniforms: Uniforms = bytemuck::pod_read_unaligned(&mapped);
        (uniforms.view_projection, uniforms.model)
    }
}

impl UniformBlocks {
    /// Slots a view's region holds. A drawable past it keeps a buffer of its own.
    pub(crate) const SLOTS: usize = 384;

    pub(crate) fn new(device: &wgpu::Device) -> Self {
        let size = (SceneBlocks::VIEWS * Self::SLOTS) as u64 * mesh_render::UNIFORMS_STRIDE;
        Self {
            buffer: device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("uniform blocks"),
                size,
                usage: wgpu::BufferUsages::UNIFORM
                    | wgpu::BufferUsages::COPY_DST
                    | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            }),
            next: 0,
        }
    }

    fn claim(&mut self) -> Option<u32> {
        (self.next < Self::SLOTS as u32).then(|| {
            self.next += 1;
            self.next - 1
        })
    }
}

/// What a frame's writes go through: the queue itself (`Deref`, for every upload that is not a
/// drawable's matrices) and the staging of the matrices of drawables on [`UniformBlocks`].
/// [`Self::flush`] writes what was staged, **before the view's scene pass**: a queue write lands
/// before the submission's commands wherever it is made, but a block staged after the flush is
/// written straight away instead (still correct, one call each).
pub(crate) struct Uploads<'a> {
    queue: &'a wgpu::Queue,
    /// `None` for [`Self::immediate`]: nothing is staged, every write goes straight to its buffer.
    blocks: Option<&'a UniformBlocks>,
    view: ViewBlock,
    staged: std::cell::RefCell<Staging>,
    flushed: std::cell::Cell<bool>,
}

/// [`Uploads`]' working memory, kept by the scene so a frame allocates nothing for it.
#[derive(Debug, Default)]
pub(crate) struct Staging {
    blocks: Vec<(u32, Uniforms)>,
    packed: Vec<u8>,
}

impl<'a> Uploads<'a> {
    pub(crate) fn begin(
        queue: &'a wgpu::Queue,
        blocks: &'a UniformBlocks,
        view: ViewBlock,
        mut staging: Staging,
    ) -> Self {
        staging.blocks.clear();
        Self {
            queue,
            blocks: Some(blocks),
            view,
            staged: std::cell::RefCell::new(staging),
            flushed: std::cell::Cell::new(false),
        }
    }

    /// Writes that go straight to the queue, for a drawable drawn outside any view (an advert
    /// card's own texture).
    pub(crate) fn immediate(queue: &'a wgpu::Queue) -> Self {
        Self {
            queue,
            blocks: None,
            view: ViewBlock::NONE,
            staged: std::cell::RefCell::default(),
            flushed: std::cell::Cell::new(true),
        }
    }

    fn stage(&self, slot: u32, uniforms: Uniforms) {
        let Some(blocks) = self.blocks else {
            unreachable!("a drawable on the uniform blocks is written inside a view");
        };
        if self.flushed.get() {
            let at = u64::from(self.view.uniforms) + u64::from(slot) * mesh_render::UNIFORMS_STRIDE;
            oag_gpu::perfprobe::write_buffer(
                self.queue,
                &blocks.buffer,
                at,
                bytemuck::bytes_of(&uniforms),
            );
        } else {
            self.staged.borrow_mut().blocks.push((slot, uniforms));
        }
    }

    /// Writes the staged slots, a call for each run of neighbouring slots. A slot staged twice
    /// keeps its last block.
    pub(crate) fn flush(&self) {
        self.flushed.set(true);
        let (Some(blocks), mut staging) = (self.blocks, self.staged.borrow_mut()) else {
            return;
        };
        let Staging {
            blocks: staged,
            packed,
        } = &mut *staging;
        staged.sort_by_key(|&(slot, _)| slot);
        let stride = mesh_render::UNIFORMS_STRIDE as usize;
        let mut index = 0;
        while index < staged.len() {
            let first = staged[index].0;
            packed.clear();
            let mut next = first;
            while index < staged.len() && staged[index].0 <= next {
                let (slot, uniforms) = &staged[index];
                let at = (slot - first) as usize * stride;
                packed.resize(at + stride, 0);
                packed[at..at + std::mem::size_of::<Uniforms>()]
                    .copy_from_slice(bytemuck::bytes_of(uniforms));
                next = slot + 1;
                index += 1;
            }
            let at =
                u64::from(self.view.uniforms) + u64::from(first) * mesh_render::UNIFORMS_STRIDE;
            oag_gpu::perfprobe::write_buffer(self.queue, &blocks.buffer, at, packed);
        }
        staged.clear();
    }

    /// The working memory back, for the next view.
    pub(crate) fn into_staging(self) -> Staging {
        self.staged.into_inner()
    }
}

impl std::ops::Deref for Uploads<'_> {
    type Target = wgpu::Queue;

    fn deref(&self) -> &wgpu::Queue {
        self.queue
    }
}

impl Drawable {
    /// Moves this drawable's bind group 0 onto `blocks`, in the next slot. Left on its own
    /// buffer when the slots are used up.
    pub(crate) fn share_uniforms(&mut self, device: &wgpu::Device, blocks: &mut UniformBlocks) {
        let Some(slot) = blocks.claim() else {
            return;
        };
        self.uniform_bind = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("race uniforms"),
            layout: &self.pipeline.get_bind_group_layout(0),
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                    buffer: &blocks.buffer,
                    offset: 0,
                    size: std::num::NonZeroU64::new(mesh_render::UNIFORMS_SIZE),
                }),
            }],
        });
        self.uniform_slot = Some(slot);
    }

    /// Which slot of the uniform blocks this drawable reads, if it is on them.
    #[cfg(test)]
    pub(crate) fn uniform_slot(&self) -> Option<u32> {
        self.uniform_slot
    }

    /// The dynamic offset of bind group 0 for this drawable in `view`.
    pub(super) fn uniform_offset(&self, view: ViewBlock) -> [u32; 1] {
        [self.uniform_slot.map_or(0, |slot| {
            view.uniforms + slot * mesh_render::UNIFORMS_STRIDE as u32
        })]
    }

    /// Stages this drawable's matrices on `uploads` when it is on [`UniformBlocks`]; writes its
    /// own buffer otherwise.
    #[track_caller]
    pub(super) fn put_uniforms(&self, uploads: &Uploads<'_>, uniforms: Uniforms) {
        match self.uniform_slot {
            Some(slot) => uploads.stage(slot, uniforms),
            None => oag_gpu::perfprobe::write_buffer(
                uploads.queue,
                &self.uniforms,
                0,
                bytemuck::bytes_of(&uniforms),
            ),
        }
    }
}
