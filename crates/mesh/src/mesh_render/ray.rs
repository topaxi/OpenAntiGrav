//! The ray tier: hardware ray queries against acceleration structures built from the
//! same buffers the raster draws, for a device that asks for [`FEATURES`].
//!
//! **Off by default and byte-identical when off.** Nothing here runs, and no buffer or
//! limit differs from what the renderer created before this module, unless the tier was
//! requested ([`request`] or `OAG_RAY=1`) *and* the adapter reports the feature. The
//! feature set is fixed at device creation, so the tier is chosen at launch, like the
//! adapter. Design, support table and measured cost: `docs/rendering/ray-tracing.md`.
//!
//! **An enhancement, not a recovery.** No original draws any of it. Everything built on
//! this is labelled "chosen, not measured" and never overrides authored data.
//!
//! The `unsafe` exception is [`experimental_token`] and nothing else, see
//! `docs/architecture/adr/0072-the-ray-tier-takes-wgpus-experimental-features-token.md`.

mod accel;
mod debug;

#[cfg(test)]
mod tests;

pub use accel::{Hulls, MAX_INSTANCES, ModelAccel, Scene, Stats, create_tlas};
pub use debug::{Debug, Frame, MAX_RAY_LENGTH};

use std::sync::atomic::{AtomicBool, Ordering};

/// The device feature the tier needs: `rayQuery*` in a shader and acceleration structures.
pub const FEATURES: wgpu::Features = wgpu::Features::EXPERIMENTAL_RAY_QUERY;

static REQUESTED: AtomicBool = AtomicBool::new(false);

/// Asks every device this process creates from now on for [`FEATURES`], where its adapter
/// has them. Called once at launch, before the first `request_device`.
pub fn request(on: bool) {
    REQUESTED.store(on, Ordering::Relaxed);
}

/// Whether the tier was asked for: [`request`], or `OAG_RAY=1` (a switch for benches and
/// the pictures' comparison, like `OAG_BINDLESS`).
#[must_use]
pub fn requested() -> bool {
    REQUESTED.load(Ordering::Relaxed) || std::env::var("OAG_RAY").is_ok_and(|v| v == "1")
}

/// Whether the tier is asked for **and** `adapter` can give it: the one question
/// `device_descriptor` asks.
#[must_use]
pub fn wanted_for(adapter: &wgpu::Adapter) -> bool {
    requested() && adapter.features().contains(FEATURES)
}

/// Whether `device` was created with the tier: what a buffer's `BLAS_INPUT` usage and every
/// acceleration-structure call is keyed on, never the setting itself.
#[must_use]
pub fn enabled(device: &wgpu::Device) -> bool {
    device.features().contains(FEATURES)
}

/// The buffer usage a vertex or index buffer adds on a device with the tier, so the
/// structures can be built from it. Empty everywhere else, which is what keeps a device
/// that never asked for the tier creating the buffers it always did.
#[must_use]
pub fn blas_input(device: &wgpu::Device) -> wgpu::BufferUsages {
    if enabled(device) {
        wgpu::BufferUsages::BLAS_INPUT
    } else {
        wgpu::BufferUsages::empty()
    }
}

/// `limits` with the acceleration-structure limits raised to what `adapter` has. The
/// defaults are all zero, which would let the request succeed and every `create_blas`
/// then fail validation. Applied only where [`wanted_for`] says so.
#[must_use]
pub fn with_limits(limits: wgpu::Limits, adapter: &wgpu::Adapter) -> wgpu::Limits {
    let own = adapter.limits();
    wgpu::Limits {
        max_blas_primitive_count: own.max_blas_primitive_count,
        max_blas_geometry_count: own.max_blas_geometry_count,
        max_tlas_instance_count: own.max_tlas_instance_count,
        max_acceleration_structures_per_shader_stage: own
            .max_acceleration_structures_per_shader_stage
            .min(1),
        ..limits
    }
}

/// wgpu's token for requesting an `EXPERIMENTAL_*` feature.
///
/// The single `unsafe` of the ray tier, scoped to this function.
#[allow(unsafe_code)]
#[must_use]
pub fn experimental_token() -> wgpu::ExperimentalFeatures {
    // SAFETY: the token's contract is an acknowledgement that wgpu's experimental ray-query
    // support may contain bugs that are undefined behaviour when hit, and that we report
    // them. It is taken only when the adapter reports the feature and the tier was asked for
    // ([`wanted_for`]); every acceleration-structure call this crate makes is checked by
    // wgpu's own validation (sizes inside the descriptor, a TLAS built after its BLASes), and
    // the tier is render-side only, so no simulation state depends on its result.
    unsafe { wgpu::ExperimentalFeatures::enabled() }
}

/// `descriptor` with the tier added when [`wanted_for`] `adapter`: the feature, the limits
/// and the token. The unchanged descriptor otherwise.
#[must_use]
pub fn apply<'a>(
    mut descriptor: wgpu::DeviceDescriptor<'a>,
    adapter: &wgpu::Adapter,
) -> wgpu::DeviceDescriptor<'a> {
    if wanted_for(adapter) {
        descriptor.required_features |= FEATURES;
        descriptor.required_limits = with_limits(descriptor.required_limits, adapter);
        descriptor.experimental_features = experimental_token();
    }
    descriptor
}
