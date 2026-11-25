//! `OAG_SKIP_PASSES`: leave named passes out of the frame, to measure what each costs
//! where no profiler sees inside the GPU (a phone: `docs/tools/android.md`, "Where an HD
//! frame goes on the S21").
//!
//! A measuring switch, not a setting: while a pass is left out the picture is wrong. Unset,
//! [`skipped`] is `false` for every name and nothing changes.
//!
//! `OAG_SKIP_PASSES=hud,glass` leaves those passes out of every frame.
//! `OAG_SKIP_PASSES_TOGGLE=10` leaves them out only in every other 10 second window (the
//! odd ones, counted from the first call), so one run carries both arms side by side and a
//! phone's temperature, which moves a whole run by half, moves both alike.
//! `scripts/android-skip-windows.py` splits a `--frame-log` by window.
//!
//! Two names price a render pass rather than leave one out: `advert-draws` keeps the
//! advert cards' passes and clears and drops their draws (pixels against passes), and
//! `extra-passes` adds [`EXTRA_PASSES`] one-pixel clear passes to the frame
//! (`docs/tools/android.md`, "What a render pass costs").
//!
//! Three names look inside the scene pass: `track-plain` and `track-rich` leave out the
//! shading half of a prepassed circuit's draws that name no material row and that name one
//! (its depth half stays), and `mips` binds every material texture from two mip levels down
//! (`docs/tools/android.md`, "Inside the scene pass").
//!
//! `writes` drops every [`crate::writes::Writes`] write made on the queue itself once a race
//! has drawn its first frame ([`arm_writes`]), so the frame's upload copies leave the
//! submission and every buffer keeps what it last held: whether frame N's copies wait on
//! frame N-1 (`docs/tools/android.md`, "Freezing the frame's writes"). A load's writes are
//! never dropped. `writes-same` is its control: every write is still made, but with the bytes
//! that buffer range was given when the window began, so the picture freezes as under
//! `writes` while every copy, and every hazard it carries, stays in the submission.
//!
//! With a toggle period set, every change of window prints one line to stderr
//! (`OAG_SKIP_PASSES: window N on|off`), which a phone's Perfetto trace carries with its own
//! timestamps under `android.log`, so a GPU timeline splits by arm.

use std::sync::OnceLock;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// The names a call site asks about. Anything else in the variable is reported once.
pub const NAMES: [&str; 13] = [
    "hud",
    "bloom",
    "adverts",
    "glass",
    "resolve",
    "composite",
    "advert-draws",
    "extra-passes",
    "writes",
    "writes-same",
    "track-plain",
    "track-rich",
    "mips",
];

/// How many one-pixel passes `extra-passes` adds to a frame.
pub const EXTRA_PASSES: usize = 16;

struct Switch {
    names: Vec<String>,
    period: Option<f32>,
    #[cfg(not(target_arch = "wasm32"))]
    start: std::time::Instant,
}

fn switch() -> &'static Switch {
    static SWITCH: OnceLock<Switch> = OnceLock::new();
    SWITCH.get_or_init(|| {
        let names: Vec<String> = std::env::var("OAG_SKIP_PASSES")
            .map(|v| {
                v.split(',')
                    .map(str::trim)
                    .filter(|n| !n.is_empty())
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default();
        let period = std::env::var("OAG_SKIP_PASSES_TOGGLE")
            .ok()
            .and_then(|v| v.trim().parse::<f32>().ok())
            .filter(|p| *p > 0.0);
        if !names.is_empty() {
            let unknown: Vec<&String> = names
                .iter()
                .filter(|n| !NAMES.contains(&n.as_str()))
                .collect();
            // Stderr rather than a log line: this crate logs nothing, and the phone's
            // `RustStdoutStderr` tag carries it beside the `--frame-log` lines.
            eprintln!(
                "OAG_SKIP_PASSES: {names:?} left out {}; unknown names {unknown:?} (known: {NAMES:?})",
                period.map_or("every frame".to_owned(), |p| {
                    format!("in every other {p} s window from now, the odd ones")
                }),
            );
        }
        Switch {
            names,
            period,
            #[cfg(not(target_arch = "wasm32"))]
            start: std::time::Instant::now(),
        }
    })
}

/// Whether `OAG_SKIP_PASSES` names `name` at all, toggled or not: what a caller asks before
/// it builds something only that arm uses.
#[must_use]
pub fn named(name: &str) -> bool {
    switch().names.iter().any(|n| n == name)
}

/// Records [`EXTRA_PASSES`] passes that each clear `target` (a one-pixel texture the
/// caller built because [`named`] said `extra-passes`) while `extra-passes` is on.
pub fn extra_passes(encoder: &mut wgpu::CommandEncoder, target: Option<&wgpu::TextureView>) {
    let Some(target) = target else { return };
    if !skipped("extra-passes") {
        return;
    }
    for _ in 0..EXTRA_PASSES {
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("extra pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    }
}

static WRITES_ARMED: AtomicBool = AtomicBool::new(false);

/// Lets `writes` drop the queue's writes from now on: the race stage calls it every frame it
/// draws, so nothing a load uploads is ever dropped.
pub fn arm_writes() {
    WRITES_ARMED.store(true, Ordering::Relaxed);
}

/// Whether `writes` drops a write made now: named, armed and in an odd window.
#[must_use]
pub fn writes_frozen() -> bool {
    WRITES_ARMED.load(Ordering::Relaxed) && skipped("writes")
}

/// Whether `writes-same` replays held bytes for a write made now: named, armed and in an odd
/// window.
#[must_use]
pub fn writes_replayed() -> bool {
    WRITES_ARMED.load(Ordering::Relaxed) && skipped("writes-same")
}

/// Whether `OAG_SKIP_PASSES` leaves `name` out of this frame.
#[must_use]
pub fn skipped(name: &str) -> bool {
    let switch = switch();
    if !switch.names.iter().any(|n| n == name) {
        return false;
    }
    match switch.period {
        // The browser build has no monotonic clock here; a toggle there leaves the pass
        // out throughout.
        #[cfg(not(target_arch = "wasm32"))]
        Some(period) => {
            let window = (switch.start.elapsed().as_secs_f32() / period) as u64;
            announce(window);
            window % 2 == 1
        }
        _ => true,
    }
}

/// Prints the window a toggle run has entered, once per window.
#[cfg(not(target_arch = "wasm32"))]
fn announce(window: u64) {
    static LAST: AtomicU64 = AtomicU64::new(u64::MAX);
    if LAST.swap(window, Ordering::Relaxed) != window {
        let arm = if window % 2 == 1 { "on" } else { "off" };
        eprintln!("OAG_SKIP_PASSES: window {window} {arm}");
    }
}

#[cfg(test)]
mod tests {
    /// Unset, nothing is left out: the frame a run without the variable draws is the
    /// frame it always drew. (The test runner sets no `OAG_SKIP_PASSES`.)
    #[test]
    fn with_the_variable_unset_no_pass_is_left_out() {
        for name in super::NAMES {
            assert!(!super::skipped(name), "{name}");
            assert!(!super::named(name), "{name}");
        }
        super::arm_writes();
        assert!(!super::writes_frozen());
        assert!(!super::writes_replayed());
    }
}
