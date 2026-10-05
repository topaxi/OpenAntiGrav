//! The bind groups one frame needs, built together - and cached, because there
//! are only ever two distinct sets of them.
//!
//! Its own file rather than the first three hundred lines of `Fsr3::render`,
//! under the 1,000-line rule in `scripts/check-file-size.py` - and it is a real
//! seam: **every ping-pong parity decision in the whole port lives here**.
//! Which half of `luma`, `accumulation`, `luma_history` and
//! `internal_upscaled` is "current" and which is "previous" is decided once,
//! from one frame index, in one place. Spread across the dispatch sequence
//! those eight decisions would be eight chances to read the frame being
//! written.
//!
//! # Two sets, not one per frame
//!
//! Building these per frame was fifteen validated `wgpu` objects and a `Vec`
//! allocation in the frame path, and **nothing in them varies per frame**:
//! every resource is either a [`Targets`] member, which moves only when an
//! allocation does, or one of the three scene views, which move only on a
//! resize. What is left is the ping-pong parity, and a parity has two values.
//!
//! So [`Cache`] holds both and hands back the one this frame wants. It is
//! rebuilt when the scene views change identity, when MSAA changes which build
//! of `prepare_inputs` runs, or when `Fsr3::resize` throws the targets away -
//! the three things that can actually invalidate a binding. A `wgpu::TextureView`
//! is an `Arc` handle that compares by identity, so "are these the same three"
//! is three pointer comparisons rather than a rebuild.
//!
//! Every `create_bind_group` here goes through
//! [`oag_gpu::perfprobe::bind_group`], as the five sites in [`crate`] do.
//! Bypassing it was why this port - the renderer's largest bind-group producer
//! by a wide margin - reported zero under `OAG_RENDER_PERF`.

use super::*;

/// One frame's bind groups, in dispatch order.
#[derive(Debug)]
pub(super) struct Groups {
    pub clear: wgpu::BindGroup,
    pub prepare_inputs: wgpu::BindGroup,
    pub luma_pyramid: wgpu::BindGroup,
    /// One per pyramid level: level 0 computes, the rest reduce.
    pub pyramid: Vec<wgpu::BindGroup>,
    pub shading_change: wgpu::BindGroup,
    pub prepare_reactivity: wgpu::BindGroup,
    pub luma_instability: wgpu::BindGroup,
    pub accumulate: wgpu::BindGroup,
    pub rcas: wgpu::BindGroup,
}

/// Both parities of [`Groups`], and what they were built against.
///
/// See the module documentation for why two sets cover every frame.
#[derive(Debug)]
pub(super) struct Cache {
    /// The scene's colour, depth and velocity views, by clone. A
    /// `wgpu::TextureView` is an `Arc` handle and compares by identity, so
    /// holding them costs three refcounts and buys the invalidation test.
    scene: [wgpu::TextureView; 3],
    /// Which build of `prepare_inputs` these bound - `Dispatch::sample_count`
    /// is the only other thing that changes their *shape*.
    multisampled: bool,
    /// Index 0 for an even frame index, 1 for an odd one.
    parity: [Groups; 2],
}

impl Cache {
    /// Both parities, built against `targets` and `frame`'s scene views.
    pub(super) fn build(
        fsr3: &Fsr3,
        device: &wgpu::Device,
        targets: &Targets,
        frame: Frame<'_>,
    ) -> Self {
        Self {
            scene: [
                frame.colour.clone(),
                frame.depth.clone(),
                frame.velocity.clone(),
            ],
            multisampled: frame.dispatch.sample_count > 1,
            parity: [
                Groups::new(fsr3, device, targets, frame, 0),
                Groups::new(fsr3, device, targets, frame, 1),
            ],
        }
    }

    /// Whether these are still the right bind groups for `frame`.
    ///
    /// **Nothing about `Targets` is checked here**, deliberately: a `Targets`
    /// only ever changes by being replaced wholesale in `Fsr3::resize`, which
    /// drops the cache in the same statement. Testing it again would be
    /// testing the same fact twice and inviting the two to disagree.
    pub(super) fn fits(&self, frame: Frame<'_>) -> bool {
        self.multisampled == (frame.dispatch.sample_count > 1)
            && self.scene[0] == *frame.colour
            && self.scene[1] == *frame.depth
            && self.scene[2] == *frame.velocity
    }

    /// The set this frame dispatches with.
    pub(super) fn groups(&self, frame_index: u64) -> &Groups {
        &self.parity[usize::from(!frame_index.is_multiple_of(2))]
    }
}

impl Groups {
    /// Builds all of them against `targets` for a frame of the given parity.
    ///
    /// `frame_index` is only ever read for its evenness - it is the ping-pong
    /// parity and nothing else - so [`Cache::build`] passes a plain 0 and 1.
    pub(super) fn new(
        fsr3: &Fsr3,
        device: &wgpu::Device,
        targets: &Targets,
        frame: Frame<'_>,
        frame_index: u64,
    ) -> Self {
        let clear_group = oag_gpu::perfprobe::bind_group(
            device,
            &wgpu::BindGroupDescriptor {
                label: Some("fsr3 clear"),
                layout: &fsr3.clear_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: targets
                        .reconstructed_previous_nearest_depth
                        .as_entire_binding(),
                }],
            },
        );
        let prepare_inputs_group = oag_gpu::perfprobe::bind_group(
            device,
            &wgpu::BindGroupDescriptor {
                label: Some("fsr3 prepare inputs"),
                layout: &fsr3.prepare_inputs_layout[usize::from(frame.dispatch.sample_count > 1)],
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(frame.velocity),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(frame.depth),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(frame.colour),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(
                            &targets.dilated_motion_vectors.view,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(&targets.dilated_depth.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: targets
                            .reconstructed_previous_nearest_depth
                            .as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: wgpu::BindingResource::TextureView(&targets.farthest_depth.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 7,
                        resource: wgpu::BindingResource::TextureView(
                            &targets.luma.current(frame_index).view,
                        ),
                    },
                ],
            },
        );
        let luma_pyramid_group = oag_gpu::perfprobe::bind_group(
            device,
            &wgpu::BindGroupDescriptor {
                label: Some("fsr3 luma pyramid"),
                layout: &fsr3.luma_pyramid_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&targets.farthest_depth.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(
                            &targets.farthest_depth_mip1.view,
                        ),
                    },
                ],
            },
        );

        // The pyramid's chain: one group per level, each reading the level below
        // and writing its own. Level 0 reads the render-resolution inputs
        // instead, and binds level 0 as its (unused) source so that one layout
        // serves both entry points.
        let pyramid_levels = targets.spd_mips.mips.len();
        let pyramid_groups: Vec<wgpu::BindGroup> = (0..pyramid_levels)
            .map(|level| {
                // **Level 0's source must not be level 0.** It never reads
                // binding 3 - it computes from the render-resolution inputs -
                // but wgpu forbids a texture being a storage-write and a
                // sampled resource in the same dispatch whether or not the
                // shader touches it, and one layout serving both entry points
                // means the binding has to hold *something*. The current luma is
                // already read-only in this dispatch, so pointing at it aliases
                // nothing.
                let source = match level {
                    0 => &targets.luma.current(frame_index).view,
                    _ => &targets.spd_mips.mips[level - 1],
                };
                oag_gpu::perfprobe::bind_group(
                    device,
                    &wgpu::BindGroupDescriptor {
                        label: Some("fsr3 shading change pyramid"),
                        layout: &fsr3.shading_change_pyramid_layout,
                        entries: &[
                            wgpu::BindGroupEntry {
                                binding: 0,
                                resource: wgpu::BindingResource::TextureView(
                                    &targets.luma.current(frame_index).view,
                                ),
                            },
                            wgpu::BindGroupEntry {
                                binding: 1,
                                resource: wgpu::BindingResource::TextureView(
                                    &targets.luma.previous(frame_index).view,
                                ),
                            },
                            wgpu::BindGroupEntry {
                                binding: 2,
                                resource: wgpu::BindingResource::TextureView(
                                    &targets.dilated_motion_vectors.view,
                                ),
                            },
                            wgpu::BindGroupEntry {
                                binding: 3,
                                resource: wgpu::BindingResource::TextureView(source),
                            },
                            wgpu::BindGroupEntry {
                                binding: 4,
                                resource: wgpu::BindingResource::TextureView(
                                    &targets.spd_mips.mips[level],
                                ),
                            },
                            wgpu::BindGroupEntry {
                                binding: 5,
                                resource: fsr3.levels[level].as_entire_binding(),
                            },
                        ],
                    },
                )
            })
            .collect();

        // The whole pyramid, through a filtering sampler at three explicit mip
        // levels - so this binds the texture's own view rather than one of the
        // per-level ones the chain above writes through.
        let shading_change_group = oag_gpu::perfprobe::bind_group(
            device,
            &wgpu::BindGroupDescriptor {
                label: Some("fsr3 shading change"),
                layout: &fsr3.shading_change_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(&targets.spd_mips.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&targets.shading_change.view),
                    },
                ],
            },
        );

        let prepare_reactivity_group = oag_gpu::perfprobe::bind_group(
            device,
            &wgpu::BindGroupDescriptor {
                label: Some("fsr3 prepare reactivity"),
                layout: &fsr3.prepare_reactivity_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(
                            &targets.dilated_motion_vectors.view,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&targets.dilated_depth.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: targets
                            .reconstructed_previous_nearest_depth
                            .as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(
                            &targets.luma.current(frame_index).view,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(&targets.shading_change.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::TextureView(
                            &targets.accumulation.previous(frame_index).view,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: wgpu::BindingResource::TextureView(
                            &targets.dilated_reactive_masks.view,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 7,
                        resource: wgpu::BindingResource::TextureView(
                            &targets.accumulation.current(frame_index).view,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 8,
                        resource: wgpu::BindingResource::TextureView(&targets.new_locks.view),
                    },
                ],
            },
        );

        let luma_instability_group = oag_gpu::perfprobe::bind_group(
            device,
            &wgpu::BindGroupDescriptor {
                label: Some("fsr3 luma instability"),
                layout: &fsr3.luma_instability_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(
                            &targets.dilated_motion_vectors.view,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(
                            &targets.dilated_reactive_masks.view,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(
                            &targets.luma.current(frame_index).view,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(
                            &targets.luma_history.previous(frame_index).view,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(
                            &targets.farthest_depth_mip1.view,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::TextureView(
                            &targets.luma_history.current(frame_index).view,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: wgpu::BindingResource::TextureView(
                            &targets.luma_instability.view,
                        ),
                    },
                ],
            },
        );

        let accumulate_group = oag_gpu::perfprobe::bind_group(
            device,
            &wgpu::BindGroupDescriptor {
                label: Some("fsr3 accumulate"),
                layout: &fsr3.accumulate_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(frame.colour),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(
                            &targets.dilated_motion_vectors.view,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: wgpu::BindingResource::TextureView(
                            &targets.dilated_reactive_masks.view,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: wgpu::BindingResource::TextureView(
                            &targets.luma_instability.view,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 4,
                        resource: wgpu::BindingResource::TextureView(
                            &targets.farthest_depth_mip1.view,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 5,
                        resource: wgpu::BindingResource::TextureView(
                            &targets.internal_upscaled.previous(frame_index).view,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 6,
                        resource: wgpu::BindingResource::TextureView(&targets.new_locks.view),
                    },
                    wgpu::BindGroupEntry {
                        binding: 7,
                        resource: wgpu::BindingResource::TextureView(
                            &targets.internal_upscaled.current(frame_index).view,
                        ),
                    },
                ],
            },
        );

        let rcas_group = oag_gpu::perfprobe::bind_group(
            device,
            &wgpu::BindGroupDescriptor {
                label: Some("fsr3 rcas"),
                layout: &fsr3.rcas_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: wgpu::BindingResource::TextureView(
                            &targets.internal_upscaled.current(frame_index).view,
                        ),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: wgpu::BindingResource::TextureView(&targets.upscaled_output.view),
                    },
                ],
            },
        );

        Self {
            clear: clear_group,
            prepare_inputs: prepare_inputs_group,
            luma_pyramid: luma_pyramid_group,
            pyramid: pyramid_groups,
            shading_change: shading_change_group,
            prepare_reactivity: prepare_reactivity_group,
            luma_instability: luma_instability_group,
            accumulate: accumulate_group,
            rcas: rcas_group,
        }
    }
}

#[cfg(test)]
mod tests;
