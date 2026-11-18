# oag-ext

The Rust guest SDK a mod's WebAssembly module is built on: implement an
`Extension`, name it in `export!`, build for `wasm32-unknown-unknown`. It also runs
a mod natively for tests.

Its only workspace dependency is `oag-ext-abi`, and no engine crate may depend on
it (dependency rule 2). See
[wasm extensions](../../docs/architecture/wasm-extensions/README.md).
