//! The bind groups one frame needs, built together.
//!
//! Its own file rather than the first three hundred lines of `Fsr3::render`,
//! under the 1,000-line rule in `scripts/check-file-size.py` - and it is a real
//! seam: **every ping-pong parity decision in the whole port lives here**.
//! Which half of `luma`, `accumulation`, `luma_history` and
//! `internal_upscaled` is "current" and which is "previous" is decided once,
//! from one frame index, in one place. Spread across the dispatch sequence
//! those eight decisions would be eight chances to read the frame being
//! written.

use super::*;

/// One frame's bind groups, in dispatch order.
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

impl Groups {
    /// Builds all of them against `targets` for the frame at `frame_index`.
    pub(super) fn new(
        fsr3: &Fsr3,
        device: &wgpu::Device,
        targets: &Targets,
        frame: Frame<'_>,
        frame_index: u64,
    ) -> Self {
        let clear_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fsr3 clear"),
            layout: &fsr3.clear_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: targets
                    .reconstructed_previous_nearest_depth
                    .as_entire_binding(),
            }],
        });
        let prepare_inputs_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fsr3 prepare inputs"),
            layout: &fsr3.prepare_inputs_layout,
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
        });
        let luma_pyramid_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("fsr3 luma pyramid"),
            layout: &fsr3.luma_pyramid_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&targets.farthest_depth.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&targets.farthest_depth_mip1.view),
                },
            ],
        });

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
                device.create_bind_group(&wgpu::BindGroupDescriptor {
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
                })
            })
            .collect();

        // The whole pyramid, through a filtering sampler at three explicit mip
        // levels - so this binds the texture's own view rather than one of the
        // per-level ones the chain above writes through.
        let shading_change_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
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
        });

        let prepare_reactivity_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
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
        });

        let luma_instability_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
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
                    resource: wgpu::BindingResource::TextureView(&targets.farthest_depth_mip1.view),
                },
                wgpu::BindGroupEntry {
                    binding: 5,
                    resource: wgpu::BindingResource::TextureView(
                        &targets.luma_history.current(frame_index).view,
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 6,
                    resource: wgpu::BindingResource::TextureView(&targets.luma_instability.view),
                },
            ],
        });

        let accumulate_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
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
                    resource: wgpu::BindingResource::TextureView(&targets.luma_instability.view),
                },
                wgpu::BindGroupEntry {
                    binding: 4,
                    resource: wgpu::BindingResource::TextureView(&targets.farthest_depth_mip1.view),
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
        });

        let rcas_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
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
        });

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
