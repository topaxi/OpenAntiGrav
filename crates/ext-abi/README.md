# oag-ext-abi

Version 1 of the extension ABI, defined once: the I/O buffer's layout, its
records and the vocabulary the host and the guest share. `no_std` and without
dependencies.

`oag-modhost` and `oag-ext` both compile against it, so the layout cannot drift.
See [wasm extensions](../../docs/architecture/wasm-extensions/README.md).
