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

Nothing is vendored right now. The Linux platform-native H.264 decode path
(see [ADR-0017](../docs/architecture/adr/0017-gstreamer-native-video.md))
used to vendor patched copies of `cros-libva` and `h264-reader` for a
hand-rolled VA-API decoder; that decoder was dropped in favor of GStreamer
(`gstreamer`/`gstreamer-app`, both plain crates.io dependencies, no patching
needed), so both entries and their `vendor/` directories were removed. See
ADR-0017 for why.
