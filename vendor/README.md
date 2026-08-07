# Vendored dependencies

Third-party crate source, copied in and patched, pinned via `[patch.crates-io]`
in the root `Cargo.toml`. This is a last resort, not a default: prefer an
upstream release or a `path`/`git` dependency over adding here. A crate lands
here only when it needs a small, mechanical fix to build against something on
the host (a system library's current headers, in this case) that the published
version doesn't yet account for, and vendoring is cheaper than carrying the
same patch as an out-of-tree diff nobody remembers to reapply.

Each entry keeps its own upstream license and README untouched. The patch
itself is a normal code comment at the change site explaining what was
touched and why, with a pointer to the upstream issue/repo so the entry can be
dropped once a real release picks up the fix.

## `cros-libva-0.0.13`

[chromeos/cros-libva](https://github.com/chromeos/cros-libva), BSD-3-Clause.
Safe Rust bindings over `libva` (VA-API), used for the Linux platform-native
H.264 decode path - see
[ADR-0016](../docs/architecture/adr/0016-platform-native-h264-decode.md).

0.0.13 (last published December 2024) does not compile as-is against libva
1.24: `VAEncPictureParameterBufferVP9` gained two fields (`seg_id_block_size`,
`va_reserved8`) in a libva release since. Patched in
`src/buffer/vp9.rs::EncPictureParameterBufferVP9::new` to zero-fill them -
nothing in this codebase calls VP9 encode, the fix exists purely so the crate
builds. Drop this vendored copy for a plain `cros-libva.workspace = true`
dependency once upstream publishes a release that includes the fix.
