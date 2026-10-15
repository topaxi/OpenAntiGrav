//! What the GL backend cannot do, asked in one place.
//!
//! wgpu's GL backend (desktop GL through EGL, and WebGL2 in a browser) is its
//! second tier, and it differs from Vulkan, Metal and Dx12 in a few ways the
//! renderer has to know about. Each is answered here once, so a pipeline, a
//! target or a report line asks `is_gl` rather than inventing its own test.
//! Every one of them is `false` on every other backend, which is what keeps the
//! default path byte-identical.
//!
//! [`GAPS`] is the list a run prints when it draws on GL, so nothing a GL
//! player loses is lost silently.

/// Whether `device` is wgpu's GL backend.
#[must_use]
pub fn is_gl(device: &wgpu::Device) -> bool {
    device.adapter_info().backend == wgpu::Backend::Gl
}

/// `(effect, what happens instead, why)` for everything that is off on GL.
///
/// **Chosen, not measured**: each row is a limit of wgpu 30's GL backend found
/// by running the renderer on it (`docs/tools/gl-backend.md`), not a property
/// of any title.
pub const GAPS: &[(&str, &str, &str)] = &[
    (
        "msaa",
        "off",
        "wgpu-hal gives a sampled multisample texture the wrong GL target, and WebGL2 has no multisample sampler",
    ),
    (
        "mapped shadows",
        "no shadow",
        "a depth texture cannot be read by sample or fetch on GL, only compared",
    ),
    (
        "HD hull self-shadow",
        "the hull is lit without its own shadow term",
        "the same depth-texture limit",
    ),
];

/// One line for the loader report naming every effect [`GAPS`] turns off.
#[must_use]
pub fn gaps_report() -> String {
    let rows: Vec<String> = GAPS
        .iter()
        .map(|(effect, instead, why)| format!("{effect}: {instead} ({why})"))
        .collect();
    format!("GL backend, reduced: {}", rows.join("; "))
}
