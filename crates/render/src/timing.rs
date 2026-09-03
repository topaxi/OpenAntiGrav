//! Whether the GPU can be asked how long it took.
//!
//! A wall-clock interval between frame-loop iterations is not a GPU cost.
//! Under `Vsync::On` it is pinned to the refresh and under any `FrameLimit` it
//! is pinned to the limit, because the loop sleeps to it - so it tracks real
//! work only with vsync off *and* no limit, which is a diagnostic
//! configuration rather than a shipping one. Anything that wants to *control*
//! on frame cost - dynamic resolution first
//! ([ADR-0037](../../../docs/architecture/adr/0037-dynamic-resolution-varies-a-viewport-not-an-allocation.md),
//! [dynamic-resolution.md](../../../docs/rendering/dynamic-resolution.md)) -
//! needs a timestamp written on the device instead.
//!
//! That is an optional wgpu feature, and asking a device for a feature it
//! lacks fails the request outright rather than degrading. So this module is
//! the *probe*: what one adapter offers, gathered before a device is asked for
//! anything, so a caller can degrade rather than fail to boot - the fallback
//! discipline
//! [ADR-0012](../../../docs/architecture/adr/0012-wgsl-upscalers-not-native-fidelityfx.md)
//! already mandates for the upscalers.
//!
//! The consumer exists now: [`PassTimer`] is the ring of timestamp pairs a
//! frame loop brackets one pass with, and `mesh_render::optional_features`
//! asks for what [`Timing::features`] reports because of it. This module is
//! still only the probe - it reports what an adapter offers, and
//! [`PassTimer::new`] then checks what the *device* actually came back with,
//! which are two different questions.

/// What one adapter offers for timing work on the GPU.
///
/// Three separate bits rather than one, because they are three separate
/// permissions and a controller only needs the first:
///
/// - [`Timing::passes`] writes a timestamp at the beginning and end of a
///   render or compute pass, through the pass descriptor's own
///   `timestamp_writes`. This is the WebGPU-portable one and is all that
///   bracketing the scene passes requires.
/// - [`Timing::encoders`] writes one at an arbitrary point in a command
///   encoder, between passes.
/// - [`Timing::inside_passes`] writes one at an arbitrary point *within* a
///   pass, which is what a per-draw breakdown would want and no controller
///   does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timing {
    /// [`wgpu::Features::TIMESTAMP_QUERY`].
    pub passes: bool,
    /// [`wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS`].
    pub encoders: bool,
    /// [`wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES`].
    pub inside_passes: bool,
}

impl Timing {
    /// Nothing at all - what a caller gets on an adapter that cannot be timed,
    /// and what [`Timing::features`] then asks for.
    pub const NONE: Self = Self {
        passes: false,
        encoders: false,
        inside_passes: false,
    };

    /// What `adapter` can do, without requesting a device.
    #[must_use]
    pub fn of(adapter: &wgpu::Adapter) -> Self {
        let has = |feature| adapter.features().contains(feature);
        Self {
            passes: has(wgpu::Features::TIMESTAMP_QUERY),
            encoders: has(wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS),
            inside_passes: has(wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES),
        }
    }

    /// Whether a pass can be bracketed at all, which is the only question a
    /// frame-cost controller asks.
    #[must_use]
    pub fn can_time_a_pass(self) -> bool {
        self.passes
    }

    /// The features a `request_device` would have to name to get this.
    ///
    /// Derived from what was probed rather than demanded, so it can be `or`ed
    /// into a descriptor's `required_features` on any adapter without turning
    /// a missing feature into a failed device request. The two native bits are
    /// only ever asked for alongside [`wgpu::Features::TIMESTAMP_QUERY`],
    /// which wgpu requires and which the probe cannot report separately
    /// anyway.
    #[must_use]
    pub fn features(self) -> wgpu::Features {
        let mut features = wgpu::Features::empty();
        if !self.passes {
            return features;
        }
        features |= wgpu::Features::TIMESTAMP_QUERY;
        if self.encoders {
            features |= wgpu::Features::TIMESTAMP_QUERY_INSIDE_ENCODERS;
        }
        if self.inside_passes {
            features |= wgpu::Features::TIMESTAMP_QUERY_INSIDE_PASSES;
        }
        features
    }

    /// One line for a log or a probe run, naming what is there rather than
    /// what is not.
    #[must_use]
    pub fn describe(self) -> String {
        if !self.passes {
            return "no GPU timestamps".to_string();
        }
        let mut extra = Vec::new();
        if self.encoders {
            extra.push("encoders");
        }
        if self.inside_passes {
            extra.push("inside passes");
        }
        if extra.is_empty() {
            "timestamps around passes".to_string()
        } else {
            format!(
                "timestamps around passes, and inside {}",
                extra.join(" and ")
            )
        }
    }
}

mod timer;
pub use timer::{Half, PassTimer, Reading};

#[cfg(test)]
mod tests;
