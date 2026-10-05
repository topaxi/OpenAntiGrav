//! What this renderer asks of a device: the optional features it uses and the
//! limits it raises, in the one descriptor every `request_device` shares.

/// The optional device features this renderer uses when the adapter has them.
///
/// **Intersected with the adapter's own, never demanded.** Wipeout HD's
/// textures are DXT blocks on the disc and binding them as such is worth ~250
/// MiB in a race, but `TEXTURE_COMPRESSION_BC` is not universal - the GL
/// backend and WebGL do not have it - and asking a device for a feature it
/// lacks fails the request outright rather than degrading. So this is what
/// every `request_device` in the workspace asks for, and `texture::upload`
/// decodes back to RGBA8 for whatever comes back without it.
///
/// `TIMESTAMP_QUERY` rides along on the same terms, for
/// [`oag_gpu::timing::PassTimer`] - how long the scene pass took, which is the
/// signal dynamic resolution is controlled on
/// ([dynamic-resolution.md](../../../../docs/rendering/dynamic-resolution.md)).
/// **Only the portable bit**, and deliberately not
/// [`oag_gpu::timing::Timing::features`], which would bring
/// `TIMESTAMP_QUERY_INSIDE_ENCODERS` and `TIMESTAMP_QUERY_INSIDE_PASSES` with
/// it: neither is WebGPU-portable, both are a driver behaviour change, and
/// bracketing a pass through its own descriptor needs neither. A device that
/// comes back without the bit gets no timer at all - [`oag_gpu::timing::PassTimer::new`]
/// checks the device rather than trusting the probe.
///
/// This reaches **every** `request_device` in the workspace, the captures and
/// the viewer included, which is why it is one line here rather than a
/// per-site decision: a device descriptor that differs between the window and
/// the capture is how a screenshot stops being comparable with what a player
/// sees. Byte-identity of the `--presented` captures was checked either side
/// of adding it.
#[must_use]
pub fn optional_features(adapter: &wgpu::Adapter) -> wgpu::Features {
    // Inside-pass timestamps and fragment counts only for `perfprobe::marks`, so a default build
    // asks a device for exactly what it always did.
    let probe = if cfg!(feature = "perf-probe") {
        wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES | wgpu::Features::PIPELINE_STATISTICS_QUERY
    } else {
        wgpu::Features::empty()
    };
    adapter.features()
        & (wgpu::Features::TEXTURE_COMPRESSION_BC | wgpu::Features::TIMESTAMP_QUERY | probe)
}

/// The device descriptor every `request_device` in this workspace uses.
///
/// One place that knows what this renderer wants of a device, rather than
/// eight that have to be kept agreeing - which they were not: the block-texture
/// feature had to reach the window, both offscreen captures, the loading
/// screen and all three of the viewer's paths, and a site that missed it would
/// have quietly decoded every HD texture back to RGBA8 with nothing to say so.
/// A caller with a further requirement of its own spreads this and overrides
/// that one field, as `oag_game::main::gpu` does with `memory_hints`.
#[must_use]
pub fn device_descriptor<'a>(
    label: &'a str,
    adapter: &wgpu::Adapter,
) -> wgpu::DeviceDescriptor<'a> {
    wgpu::DeviceDescriptor {
        label: Some(label),
        required_features: optional_features(adapter),
        required_limits: required_limits(adapter),
        ..Default::default()
    }
}

/// wgpu's default limits with the **texture-dimension limits raised to the
/// adapter's own**.
///
/// The default `max_texture_dimension_2d` is 8,192, and Wipeout Omega's Talon's
/// Junction ships `ds_floor_cs.gnf` 16,384 texels wide: `create_texture` on it
/// was a validation error and, with wgpu's errors fatal, a panic that ended the
/// race load. Desktop adapters and lavapipe report 16,384 or more, so asking for
/// the adapter's number is enough there. Only the resolution limits move
/// (`Limits::using_resolution`), so nothing else about what the device promises
/// changes. An adapter that is still below a texture's size is handled where
/// the texture is uploaded - see `texture::first_fitting_level`.
///
/// It also raises the ceiling `oag_game::upscale::target_size` clamps a render
/// target to, because that reads the device's own limit. `Scale::RANGE` tops out
/// at 200 %, so only a display wider than 4,096 pixels at that scale (an 8K
/// panel) could ever have been clamped at 8,192, and it is now allowed what the
/// adapter allows.
#[must_use]
pub fn required_limits(adapter: &wgpu::Adapter) -> wgpu::Limits {
    wgpu::Limits::default().using_resolution(adapter.limits())
}
