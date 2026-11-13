#![allow(unsafe_code)]
//! **Debug instrumentation, behind the off-by-default `perf-probe` feature.**
//!
//! Without the feature every function here is a no-op the optimiser deletes:
//! [`on`] returns `false` before it touches anything, and each counter is
//! behind a `cfg!` rather than a `#[cfg]`, so the bodies still compile and
//! cannot rot while the flag is off. [`bind_group`] and [`texture_view`] are
//! passthroughs either way. Build with
//! `cargo run -p oag-game --features perf-probe`.
//!
//! Three things the frame path does that nothing else counts:
//!
//! - heap allocations and bytes, through [`Counting`], a wrapper the binary
//!   installs as its global allocator;
//! - wgpu objects created per frame - bind groups and texture views - counted
//!   by hand at each site;
//! - redundant `set_bind_group` and `set_pipeline` calls inside
//!   `Drawable::draw`.
//!
//! Everything is a relaxed atomic and everything reports through
//! [`report_frame`], which prints only when `OAG_RENDER_PERF` is set.

pub mod marks;

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};

pub static ALLOCS: AtomicU64 = AtomicU64::new(0);
pub static ALLOC_BYTES: AtomicU64 = AtomicU64::new(0);
pub static BIND_GROUPS: AtomicU64 = AtomicU64::new(0);
pub static TEXTURE_VIEWS: AtomicU64 = AtomicU64::new(0);
pub static TEXTURE_BINDS: AtomicU64 = AtomicU64::new(0);
pub static TEXTURE_BINDS_REDUNDANT: AtomicU64 = AtomicU64::new(0);
pub static PIPELINE_SETS: AtomicU64 = AtomicU64::new(0);
pub static ARRAY_BINDS: AtomicU64 = AtomicU64::new(0);
pub static ARRAY_ELEMENTS: AtomicU64 = AtomicU64::new(0);
pub static WRITE_BUFFERS: AtomicU64 = AtomicU64::new(0);
pub static WRITE_BUFFER_BYTES: AtomicU64 = AtomicU64::new(0);

/// A `System` that counts every allocation that reaches it.
#[derive(Debug)]
pub struct Counting;

static CAPTURE: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
static SITES: std::sync::Mutex<Vec<(usize, std::backtrace::Backtrace)>> =
    std::sync::Mutex::new(Vec::new());
thread_local! {
    static GUARD: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

fn capture(size: usize) {
    if !CAPTURE.load(Relaxed) {
        return;
    }
    if GUARD.try_with(|g| g.replace(true)).unwrap_or(true) {
        return;
    }
    let bt = std::backtrace::Backtrace::force_capture();
    if let Ok(mut sites) = SITES.lock() {
        sites.push((size, bt));
    }
    let _ = GUARD.try_with(|g| g.set(false));
}

fn dump_sites() {
    let sites = std::mem::take(&mut *SITES.lock().unwrap());
    let mut ours: std::collections::BTreeMap<String, (u64, u64)> = Default::default();
    for (size, bt) in &sites {
        let text = bt.to_string();
        let lines: Vec<&str> = text.lines().collect();
        let mut first_any: Option<String> = None;
        let mut first_oag: Option<String> = None;
        for l in &lines {
            let l = l.trim();
            let Some(at) = l.strip_prefix("at ") else { continue };
            if at.starts_with("/rustc/") || at.contains("perfprobe.rs") {
                continue;
            }
            let short = at.rsplit_once(":").map_or(at, |(f, _)| f).to_owned();
            let short = short.split("/src/").last().map(str::to_owned).unwrap_or(short);
            let crate_ = at.split('/').skip_while(|c| *c != "registry" && *c != "crates").nth(if at.contains("registry") { 3 } else { 1 }).unwrap_or("").to_owned();
            first_any.get_or_insert(format!("{crate_}/{}", at.rsplit('/').next().unwrap_or("")));
            if at.contains("/crates/") && !at.contains("registry") {
                first_oag.get_or_insert(format!("{crate_}/{}", at.rsplit('/').next().unwrap_or("")));
                break;
            }
            let _ = short;
        }
        let k = format!("{}  <=  {}", first_oag.unwrap_or_default(), first_any.unwrap_or_default());
        let e = ours.entry(k).or_default();
        e.0 += 1;
        e.1 += *size as u64;
    }
    let mut v: Vec<_> = ours.into_iter().collect();
    v.sort_by_key(|(_, (n, _))| std::cmp::Reverse(*n));
    println!("ALLOC SITES ({} allocations)", sites.len());
    for (k, (n, b)) in v {
        println!("SITE {n:5} {b:8} {k}");
    }
}

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        capture(layout.size());
        if counting() {
            ALLOCS.fetch_add(1, Relaxed);
            ALLOC_BYTES.fetch_add(layout.size() as u64, Relaxed);
        }
        unsafe { System.alloc(layout) }
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) }
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        capture(new_size);
        if counting() {
            ALLOCS.fetch_add(1, Relaxed);
            ALLOC_BYTES.fetch_add(new_size as u64, Relaxed);
        }
        unsafe { System.realloc(ptr, layout, new_size) }
    }
}

/// Whether the probe should print at all: the `perf-probe` feature *and*
/// `OAG_RENDER_PERF` in the environment.
///
/// The environment half is read once. `env::var_os` allocates an `OsString`
/// on every call, and this is asked once per [`mark`] - about thirteen times
/// a frame - so reading it live would have this module allocating in the
/// frame path it exists to count.
#[must_use]
#[inline]
pub fn on() -> bool {
    if !cfg!(feature = "perf-probe") {
        return false;
    }
    static ON: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ON.get_or_init(|| std::env::var_os("OAG_RENDER_PERF").is_some())
}

/// Whether the counters below record anything.
///
/// Separate from [`on`]: a counter costs one relaxed atomic and is wanted
/// whenever the feature is built, where printing is wanted only when asked
/// for. Both are constant-folded away without the feature.
#[inline]
const fn counting() -> bool {
    cfg!(feature = "perf-probe")
}

/// Every counter, zeroed.
#[inline]
pub fn reset() {
    if !counting() {
        return;
    }
    if let Ok(mut sites) = WRITE_SITES.lock() {
        sites.clear();
    }
    for counter in [
        &ALLOCS,
        &ALLOC_BYTES,
        &BIND_GROUPS,
        &TEXTURE_VIEWS,
        &TEXTURE_BINDS,
        &TEXTURE_BINDS_REDUNDANT,
        &PIPELINE_SETS,
        &ARRAY_BINDS,
        &ARRAY_ELEMENTS,
        &WRITE_BUFFERS,
        &WRITE_BUFFER_BYTES,
    ] {
        counter.store(0, Relaxed);
    }
}

/// One line per frame, on `frame`.
pub fn report_frame(frame: u64) {
    if !on() {
        return;
    }
    println!(
        "perf frame {frame}: allocs {} ({} KiB)  bind_groups {}  texture_views {}  \
         tex_binds {} (redundant {})  array_binds {} ({} elements)  pipeline_sets {}  \
         write_buffer {} ({} KiB)",
        ALLOCS.load(Relaxed),
        ALLOC_BYTES.load(Relaxed) / 1024,
        BIND_GROUPS.load(Relaxed),
        TEXTURE_VIEWS.load(Relaxed),
        TEXTURE_BINDS.load(Relaxed),
        TEXTURE_BINDS_REDUNDANT.load(Relaxed),
        ARRAY_BINDS.load(Relaxed),
        ARRAY_ELEMENTS.load(Relaxed),
        PIPELINE_SETS.load(Relaxed),
        WRITE_BUFFERS.load(Relaxed),
        WRITE_BUFFER_BYTES.load(Relaxed) / 1024,
    );
    if std::env::var_os("OAG_RENDER_PERF_SITES").is_some() {
        print!("{}", write_sites());
    }
}

/// The counted writes of the frame by call site, most first, as `file:line count`.
fn write_sites() -> String {
    let Ok(sites) = WRITE_SITES.lock() else {
        return String::new();
    };
    let mut sorted: Vec<_> = sites.iter().collect();
    sorted.sort_by_key(|(_, count)| std::cmp::Reverse(*count));
    sorted
        .iter()
        .map(|(at, count)| format!("    {count:4} x {}:{}\n", at.file(), at.line()))
        .collect()
}

/// `device.create_bind_group`, counted.
#[must_use]
pub fn bind_group(device: &wgpu::Device, desc: &wgpu::BindGroupDescriptor<'_>) -> wgpu::BindGroup {
    if counting() {
        BIND_GROUPS.fetch_add(1, Relaxed);
    }
    device.create_bind_group(desc)
}

/// `texture.create_view`, counted.
#[must_use]
pub fn texture_view(
    texture: &wgpu::Texture,
    desc: &wgpu::TextureViewDescriptor<'_>,
) -> wgpu::TextureView {
    if counting() {
        TEXTURE_VIEWS.fetch_add(1, Relaxed);
    }
    texture.create_view(desc)
}

/// Where each counted `write_buffer` came from, by call site: what `report_frame` prints, so a
/// frame's uploads can be told apart without a profiler.
static WRITE_SITES: std::sync::Mutex<Vec<(&'static std::panic::Location<'static>, u64)>> =
    std::sync::Mutex::new(Vec::new());

/// `queue.write_buffer`, counted, and tallied by the caller's source line.
#[track_caller]
pub fn write_buffer(
    queue: &dyn crate::writes::Writes,
    buffer: &wgpu::Buffer,
    offset: u64,
    data: &[u8],
) {
    if counting() {
        WRITE_BUFFERS.fetch_add(1, Relaxed);
        WRITE_BUFFER_BYTES.fetch_add(data.len() as u64, Relaxed);
        let site = std::panic::Location::caller();
        if let Ok(mut sites) = WRITE_SITES.lock() {
            match sites
                .iter_mut()
                .find(|(at, _)| at.line() == site.line() && at.file() == site.file())
            {
                Some((_, count)) => *count += 1,
                None => sites.push((site, 1)),
            }
        }
    }
    queue.write_buffer(buffer, offset, data);
}

/// A named allocation checkpoint: prints the allocations and bytes since the
/// last call, so one frame can be broken into stages.
///
/// **Debug instrumentation for the render-performance review. Not for merge.**
pub fn mark(label: &str) {
    if !on() {
        return;
    }
    use std::sync::atomic::AtomicU64;
    static LAST_ALLOCS: AtomicU64 = AtomicU64::new(0);
    static LAST_BYTES: AtomicU64 = AtomicU64::new(0);
    // Per thread: a split screen records its views on threads of their own, and a span is
    // the time since the last mark *of the same thread*. The allocation counts stay global.
    thread_local! {
        static LAST_TIME: std::cell::Cell<Option<std::time::Instant>> =
            const { std::cell::Cell::new(None) };
    }
    if label == "t-submit" {
        static N: AtomicU64 = AtomicU64::new(0);
        let n = N.fetch_add(1, Relaxed) + 1;
        let at: u64 = std::env::var("OAG_ALLOC_SITES_AT").ok().and_then(|v| v.parse().ok()).unwrap_or(0);
        if at > 0 && n == at {
            CAPTURE.store(true, Relaxed);
        } else if at > 0 && n == at + 1 {
            CAPTURE.store(false, Relaxed);
            dump_sites();
        }
    }
    let allocs = ALLOCS.load(Relaxed);
    let bytes = ALLOC_BYTES.load(Relaxed);
    let now = std::time::Instant::now();
    let micros = LAST_TIME
        .with(|last| last.replace(Some(now)))
        .map_or(0, |last| now.duration_since(last).as_micros());
    println!(
        "  perf mark {label}: +{micros} us, +{} allocs, +{} bytes",
        allocs.saturating_sub(LAST_ALLOCS.swap(allocs, Relaxed)),
        bytes.saturating_sub(LAST_BYTES.swap(bytes, Relaxed)),
    );
}

/// One `Drawable::draw`'s worth of texture bind-group bookkeeping: how many
/// were issued, and how many named the slot already bound.
///
/// Here rather than in `oag-game` so its state disappears with the feature -
/// without `perf-probe` the field is written and never read, and the whole
/// struct folds away.
#[derive(Debug, Default)]
pub struct Binds {
    last: Option<usize>,
}

impl Binds {
    /// Records that the caller is about to bind `slot`.
    #[inline]
    pub fn record(&mut self, slot: usize) {
        if !counting() {
            return;
        }
        TEXTURE_BINDS.fetch_add(1, Relaxed);
        if self.last == Some(slot) {
            TEXTURE_BINDS_REDUNDANT.fetch_add(1, Relaxed);
        }
        self.last = Some(slot);
    }
}

/// Records one `set_bind_group` of a binding-array group carrying `elements`
/// descriptors, each of which wgpu tracks per bind.
#[inline]
pub fn array_bind(elements: usize) {
    if counting() {
        ARRAY_BINDS.fetch_add(1, Relaxed);
        ARRAY_ELEMENTS.fetch_add(elements as u64, Relaxed);
    }
}

/// Records one `set_pipeline`.
#[inline]
pub fn pipeline_set() {
    if counting() {
        PIPELINE_SETS.fetch_add(1, Relaxed);
    }
}
