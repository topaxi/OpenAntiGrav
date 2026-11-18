# oag-modhost

The WebAssembly extension host: wasmtime set up for determinism, one fresh pooled
instance per tick, a fuel budget, and one fault path for a trap or a refused reply.

Knows no game and no title. No gameplay crate may depend on it. Design:
[wasm extensions](../../docs/architecture/wasm-extensions/README.md).
