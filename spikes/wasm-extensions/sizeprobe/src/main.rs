//! Binary-size probe: the same trivial program built with no wasmtime, with the
//! runtime and Pulley only (what an iOS build needs), and with Cranelift too.
//! `cargo build --release -p spike-sizeprobe [--features pulley|jit]`.

fn main() {
    #[cfg(feature = "runtime")]
    {
        let mut config = wasmtime::Config::new();
        config.consume_fuel(true);
        #[cfg(all(feature = "pulley", not(feature = "jit")))]
        config.target("pulley64").expect("pulley target");
        let engine = wasmtime::Engine::new(&config).expect("engine");
        #[cfg(feature = "jit")]
        {
            let wasm = std::env::args().nth(1).map(std::fs::read);
            if let Some(Ok(wasm)) = wasm {
                let module = wasmtime::Module::new(&engine, &wasm).expect("module");
                let mut store = wasmtime::Store::new(&engine, ());
                store.set_fuel(1_000).expect("fuel");
                let _ = wasmtime::Instance::new(&mut store, &module, &[]);
            }
        }
        #[cfg(not(feature = "jit"))]
        {
            // Runtime only: modules arrive precompiled (`Engine::precompile_module`
            // on a machine with Cranelift). Deserialising is the runtime's job.
            if let Some(path) = std::env::args().nth(1) {
                // SAFETY: a spike measuring size; the file is our own precompile.
                let module = unsafe { wasmtime::Module::deserialize_file(&engine, path) };
                if let Ok(module) = module {
                    let mut store = wasmtime::Store::new(&engine, ());
                    store.set_fuel(1_000).expect("fuel");
                    let _ = wasmtime::Instance::new(&mut store, &module, &[]);
                }
            }
        }
    }
    println!("probe");
}
