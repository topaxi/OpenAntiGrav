# oag-shader-check

A build-dependency for the render-side crates' `build.rs`: it links a crate's
WESL, parses and validates every final shader with `naga`, and fails `cargo build`
naming the module. It also holds the WESL modules the render crates share.

Not linked into a running program. See
[ADR-0060](../../docs/architecture/adr/0060-every-render-crate-shader-is-wesl-with-one-shared-module-set.md).
