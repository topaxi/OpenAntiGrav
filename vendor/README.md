# Vendored dependencies

Third-party crate source, copied in and patched, pinned via `[patch.crates-io]`
in the root `Cargo.toml`. This is a last resort, not a default: prefer an
upstream release or a `path`/`git` dependency over adding here. A crate lands
here only when the published version needs a small, mechanical fix this
codebase depends on - a build failure against something on the host the
published version doesn't yet account for, or a small missing accessor onto
something the crate already computes internally - and vendoring is cheaper
than carrying the same patch as an out-of-tree diff nobody remembers to
reapply.

Each entry keeps its own upstream license and README untouched. The patch
itself is a normal code comment at the change site explaining what was
touched and why, with a pointer to the upstream issue/repo so the entry can be
dropped once a real release picks up the fix.

**`atrac9dec/`** (0.9.1, MIT, a Rust port of the MIT LibAtrac9). Not a small
fix, which this page otherwise asks for: the published crate refused every 5.1
and 7.1 stream and departed from LibAtrac9 on stereo too, and five functions
are ported from LibAtrac9 in full (`docs/formats/atrac9.md`, "What the
published crate got wrong"). Each change carries an "OpenAntiGrav patch"
comment. Upstream ships no licence file; the manifest says MIT, and
LibAtrac9's own licence sits beside it as `LICENSE-LibAtrac9` and ships in
`licences/LibAtrac9-MIT.txt`. Drop the entry
once a release decodes everything `crates/music/tests/atrac9_ground_truth.rs`
and `omega_wem_ground_truth.rs` read, sample-identical to LibAtrac9.

**`wgpu-hal/`** (30.0.2, MIT OR Apache-2.0, licence files kept). The Vulkan
backend keeps each pipeline stage's SPIR-V per device (the Vulkan counterpart of
upstream's D3D12 shader cache, gfx-rs/wgpu#7729), so pipelines built from one
module with override constants stop re-resolving and re-writing identical stages: a
race launch's user instructions fall by about 21 %. `NOTICE.md` lists the changed
files; why, how it is tested and when to drop it are in
`docs/architecture/vendored-wgpu-hal.md`. The workspace's `exclude = ["vendor"]`
lets the crate's own tests run standalone. Drop the entry once a `wgpu` release
carries an equivalent cache.

Earlier entries: the Linux platform-native H.264 decode path
(see [ADR-0017](../docs/architecture/adr/0017-gstreamer-native-video.md))
used to vendor patched copies of `cros-libva` and `h264-reader` for a
hand-rolled VA-API decoder; that decoder was dropped in favor of GStreamer
(`gstreamer`/`gstreamer-app`, both plain crates.io dependencies, no patching
needed), so both entries and their `vendor/` directories were removed. See
ADR-0017 for why.
