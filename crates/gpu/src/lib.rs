//! The GPU vocabulary the renderer's crates share.
//!
//! `oag-mesh` draws the scene and `oag-post` reads it back, and neither may
//! depend on the other: the scene target's format is something the mesh
//! pipeline chooses and the bloom gate reads, and the profiler both passes
//! write timestamps into is neither's own. Those few things live here, below
//! both.
//!
//! - [`formats`] are the two offscreen target formats the scene pass and the
//!   post chain have to agree on.
//! - [`perfprobe`] is the off-by-default `perf-probe` instrumentation.
//! - [`init_buffer`] is a small constant buffer created holding its contents,
//!   by mapping on native and through the queue in a browser.
//! - [`timing`] is whether the GPU can be asked how long it took.

pub mod formats;
pub mod init_buffer;
pub mod perfprobe;
pub mod timing;
