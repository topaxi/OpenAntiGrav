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

/// Where the blocks of the view being drawn start: the dynamic offset's view half.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct ViewBlock(u32);

impl ViewBlock {
    /// For a draw outside any view (an advert card's own texture): the drawable's own block.
    pub(crate) const NONE: Self = Self(0);
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

    /// The first block of the view following craft `slot`.
    pub(crate) fn view(slot: usize) -> ViewBlock {
        let view = slot.min(Self::VIEWS - 1) as u64;
        ViewBlock((view * SceneKind::COUNT as u64 * mesh_render::SCENE_STRIDE) as u32)
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
        let at = u64::from(view.0) + kind.index() * mesh_render::SCENE_STRIDE;
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
        let at = u64::from(Self::view(slot).0) + kind.index() * mesh_render::SCENE_STRIDE;
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
            view.0 + (kind.index() * mesh_render::SCENE_STRIDE) as u32
        })]
    }
}
