//! The spike's host: a toy one-dimensional race tick that calls a WebAssembly
//! guest from inside the tick, under fuel, and hashes everything the tick owns.
//!
//! `spike-host <engine> <guest.wasm>`, engine `cranelift` or `pulley`. Every line
//! it prints except those starting `time` is meant to be bit-identical across
//! engines and runs; `run.sh` diffs them. Wall-clock timing here is the spike
//! measuring itself; nothing simulated reads it.

use std::time::Instant;

use oag_core::{Rng, StateHasher};
use wasmtime::{Config, Engine, Instance, InstancePre, Linker, Module, Result, Store, Trap};

const IO_SIZE: usize = 256;
const BLOB_SIZE: usize = 64;
const OBS_TICK: usize = 64;
const OBS_SPEED: usize = 68;
const OBS_DISTANCE: usize = 72;
const OBS_BASE_TOP: usize = 76;
const OBS_PAD: usize = 80;
const OUT_SCALE: usize = 96;
const SPIN_COUNTER: usize = 128;

/// Fuel granted to one guest call. Chosen for the spike.
const FUEL_PER_CALL: u64 = 10_000;

const TRACK: f32 = 1000.0;
const PADS: [f32; 4] = [100.0, 350.0, 600.0, 850.0];
const PAD_HALF: f32 = 4.0;
const BASE_TOP: f32 = 100.0;
const DT: f32 = 1.0 / 60.0;

/// Everything the toy race owns: the stand-in for `World`. The mod's blob is in
/// here, so a snapshot of this is a snapshot of the mod.
#[derive(Clone)]
struct Toy {
    tick: u32,
    speed: f32,
    distance: f32,
    pad_current: i32,
    rng: Rng,
    blob: [u8; BLOB_SIZE],
    faulted: bool,
    fuel_used: u64,
}

impl Toy {
    fn new() -> Self {
        Self {
            tick: 0,
            speed: 0.0,
            distance: 0.0,
            pad_current: -1,
            rng: Rng::new(42),
            blob: [0; BLOB_SIZE],
            faulted: false,
            fuel_used: 0,
        }
    }

    fn hash(&self) -> u64 {
        let mut h = StateHasher::new();
        h.write_u32(self.tick);
        h.write_f32(self.speed);
        h.write_f32(self.distance);
        h.write_i32(self.pad_current);
        for word in self.rng.snapshot() {
            h.write_u32(word);
        }
        h.write(&self.blob);
        h.write_u8(u8::from(self.faulted));
        h.write_u64(self.fuel_used);
        h.finish()
    }
}

struct Host {
    rng: Rng,
}

/// How the guest instance lives: one for the whole race, or a fresh one per call.
#[derive(Clone, Copy, PartialEq)]
enum Lifetime {
    Persistent,
    Fresh,
}

struct Guest {
    engine: Engine,
    pre: InstancePre<Host>,
    lifetime: Lifetime,
    live: Option<(Store<Host>, Instance)>,
    tick_export: &'static str,
    fuel: u64,
}

fn engine(kind: &str, canonicalise: bool) -> Result<Engine> {
    engine_with(kind, canonicalise, false)
}

/// `pooled`: wasmtime's pooling allocator, which keeps instance slots and
/// resets memory copy-on-write, the cheap way to give every call a fresh guest.
fn engine_with(kind: &str, canonicalise: bool, pooled: bool) -> Result<Engine> {
    let mut config = Config::new();
    if pooled {
        let mut pool = wasmtime::PoolingAllocationConfig::default();
        pool.total_memories(16).total_core_instances(16).total_tables(16);
        config.allocation_strategy(wasmtime::InstanceAllocationStrategy::Pooling(pool));
    }
    config.consume_fuel(true);
    config.cranelift_nan_canonicalization(canonicalise);
    // No `wasm_threads`: the crate is built without the `threads` feature, so
    // a module declaring shared memory fails to compile.
    config.wasm_relaxed_simd(false);
    config.wasm_simd(false);
    match kind {
        "cranelift" => {}
        "pulley" => {
            config.target("pulley64")?;
        }
        _ => return Err(wasmtime::Error::msg("engine is cranelift or pulley")),
    }
    Engine::new(&config)
}

impl Guest {
    fn new(
        engine: Engine,
        wasm: &[u8],
        lifetime: Lifetime,
        tick_export: &'static str,
    ) -> Result<Self> {
        let module = Module::new(&engine, wasm)?;
        // The only import namespace there is: no WASI, no clock, no files.
        let mut linker = Linker::new(&engine);
        linker.func_wrap(
            "oag",
            "rng_next",
            |mut caller: wasmtime::Caller<'_, Host>| caller.data_mut().rng.next_u32(),
        )?;
        let pre = linker.instantiate_pre(&module)?;
        Ok(Self {
            engine,
            pre,
            lifetime,
            live: None,
            tick_export,
            fuel: FUEL_PER_CALL,
        })
    }

    /// One hook call: copy the blob and observation in, call, copy the blob and
    /// command out. On a trap the race keeps the blob it had and the mod is
    /// faulted for the rest of the race: the deterministic fallback.
    fn call(&mut self, toy: &mut Toy, export: &str) -> Result<Option<f32>> {
        if toy.faulted {
            return Ok(None);
        }
        let host = Host {
            rng: toy.rng.clone(),
        };
        if self.lifetime == Lifetime::Fresh || self.live.is_none() {
            let mut store = Store::new(&self.engine, host);
            store.set_fuel(FUEL_PER_CALL)?;
            let instance = self.pre.instantiate(&mut store)?;
            self.live = Some((store, instance));
        } else if let Some((store, _)) = self.live.as_mut() {
            *store.data_mut() = host;
        }
        let (store, instance) = self.live.as_mut().expect("instantiated above");
        store.set_fuel(FUEL_PER_CALL)?;
        let memory = instance
            .get_memory(&mut *store, "memory")
            .expect("guest exports memory");
        let io = instance
            .get_typed_func::<(), u32>(&mut *store, "oag_io")?
            .call(&mut *store, ())? as usize;
        let mut buf = [0u8; IO_SIZE];
        buf[..BLOB_SIZE].copy_from_slice(&toy.blob);
        buf[OBS_TICK..OBS_TICK + 4].copy_from_slice(&toy.tick.to_le_bytes());
        buf[OBS_SPEED..OBS_SPEED + 4].copy_from_slice(&toy.speed.to_le_bytes());
        buf[OBS_DISTANCE..OBS_DISTANCE + 4].copy_from_slice(&toy.distance.to_le_bytes());
        buf[OBS_BASE_TOP..OBS_BASE_TOP + 4].copy_from_slice(&BASE_TOP.to_le_bytes());
        buf[OBS_PAD..OBS_PAD + 4].copy_from_slice(&toy.pad_current.to_le_bytes());
        memory.write(&mut *store, io, &buf)?;
        store.set_fuel(self.fuel)?;
        let result = instance
            .get_typed_func::<(), u32>(&mut *store, export)?
            .call(&mut *store, ());
        let used = self.fuel - store.get_fuel()?;
        toy.fuel_used += used;
        if result.is_err() {
            toy.faulted = true;
            self.live = None;
            return Ok(None);
        }
        memory.read(&*store, io, &mut buf)?;
        toy.blob.copy_from_slice(&buf[..BLOB_SIZE]);
        toy.rng = store.data().rng.clone();
        if export == "on_pad_enter" {
            // An event hook: it may change the blob and draw from the generator,
            // and returns no command.
            return Ok(None);
        }
        let scale = f32::from_le_bytes(buf[OUT_SCALE..OUT_SCALE + 4].try_into()?);
        // The host's validation: a NaN or an out-of-range command faults the mod.
        if !(0.5..=2.0).contains(&scale) {
            toy.faulted = true;
            return Ok(None);
        }
        Ok(Some(scale))
    }
}

fn pad_at(distance: f32) -> i32 {
    PADS.iter()
        .position(|p| (distance - p).abs() <= PAD_HALF)
        .map_or(-1, |i| i as i32)
}

fn integrate(toy: &mut Toy, throttle: f32, scale: f32) {
    let top = BASE_TOP * scale;
    let accel = throttle * (top - toy.speed) * 0.8;
    toy.speed += accel * DT;
    toy.distance += toy.speed * DT;
    if toy.distance >= TRACK {
        toy.distance -= TRACK;
    }
    toy.tick += 1;
}

/// The toy tick: input from the seeded generator, the guest's
/// `craft.environment` scale, a pad edge calling `pad.enter`. `f32` only, no
/// fused multiply-add.
fn tick(toy: &mut Toy, guest: &mut Guest) -> Result<()> {
    let throttle = if toy.rng.next_f32() > 0.2 { 1.0f32 } else { 0.0 };
    let pad = pad_at(toy.distance);
    let entered = pad != toy.pad_current && pad >= 0;
    toy.pad_current = pad;
    if entered {
        guest.call(toy, "on_pad_enter")?;
    }
    let export = guest.tick_export;
    let scale = guest.call(toy, export)?.unwrap_or(1.0);
    integrate(toy, throttle, scale);
    Ok(())
}

fn race(guest: &mut Guest, ticks: u32) -> Result<Toy> {
    let mut toy = Toy::new();
    for _ in 0..ticks {
        tick(&mut toy, guest)?;
    }
    Ok(toy)
}

/// Ticks 0..n, a snapshot at k, ticks k..n again from the snapshot, as a
/// client's rollback would. Equal hashes mean the guest's state was all in the
/// blob.
fn rollback(guest: &mut Guest, ticks: u32, at: u32) -> Result<(u64, u64)> {
    let mut toy = Toy::new();
    let mut saved = None;
    for t in 0..ticks {
        if t == at {
            saved = Some(toy.clone());
        }
        tick(&mut toy, guest)?;
    }
    let straight = toy.hash();
    let mut toy = saved.expect("at < ticks");
    for _ in at..ticks {
        tick(&mut toy, guest)?;
    }
    Ok((straight, toy.hash()))
}

fn nan_battery(engine: &Engine) -> Result<Vec<u32>> {
    let module = Module::new(engine, include_str!("../../nan_battery.wat"))?;
    let mut store = Store::new(engine, ());
    store.set_fuel(FUEL_PER_CALL)?;
    let instance = Instance::new(&mut store, &module, &[])?;
    let f = instance.get_typed_func::<(f32, f32, f32, f64), ()>(&mut store, "nans")?;
    f.call(&mut store, (0.0, 1.0, f32::from_bits(0x7fa0_0001), 0.0))?;
    let memory = instance.get_memory(&mut store, "memory").expect("exported");
    let mut bytes = [0u8; 48];
    memory.read(&store, 0, &mut bytes)?;
    Ok(bytes
        .chunks(4)
        .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
        .collect())
}

fn blob_u32(b: &[u8; BLOB_SIZE], at: usize) -> u32 {
    u32::from_le_bytes(b[at..at + 4].try_into().unwrap())
}

fn blob_scale(b: &[u8; BLOB_SIZE]) -> f32 {
    let s = f32::from_bits(blob_u32(b, 4));
    if s == 0.0 { 1.0 } else { s }
}

/// The same law in host Rust, for the native-oracle comparison.
fn native_race(ticks: u32) -> Toy {
    let mut toy = Toy::new();
    for _ in 0..ticks {
        let throttle = if toy.rng.next_f32() > 0.2 { 1.0f32 } else { 0.0 };
        let pad = pad_at(toy.distance);
        let entered = pad != toy.pad_current && pad >= 0;
        toy.pad_current = pad;
        if entered {
            let pads = blob_u32(&toy.blob, 0) + 1;
            toy.blob[0..4].copy_from_slice(&pads.to_le_bytes());
            let next = blob_scale(&toy.blob) * 1.05;
            let next = if next > 1.5 { 1.5 } else { next };
            toy.blob[4..8].copy_from_slice(&next.to_bits().to_le_bytes());
            let jitter = (toy.rng.next_u32() >> 8) as f32 / 16_777_216.0;
            let acc = f32::from_bits(blob_u32(&toy.blob, 8)) + jitter;
            toy.blob[8..12].copy_from_slice(&acc.to_bits().to_le_bytes());
        }
        let s = blob_scale(&toy.blob);
        let drag = toy.speed / (BASE_TOP * s);
        let accum = f32::from_bits(blob_u32(&toy.blob, 8));
        let next_accum = accum * 0.999 + 0.0001 * toy.speed;
        toy.blob[8..12].copy_from_slice(&next_accum.to_bits().to_le_bytes());
        let scale = s * (1.0 - 0.01 * drag * drag) + accum.sqrt() * 0.001;
        integrate(&mut toy, throttle, scale);
    }
    toy
}

fn fuel_trap(engine: &Engine, wasm: &[u8], budget: u64) -> Result<String> {
    let module = Module::new(engine, wasm)?;
    let mut linker = Linker::new(engine);
    linker.func_wrap("oag", "rng_next", |_: wasmtime::Caller<'_, ()>| 0u32)?;
    let mut store = Store::new(engine, ());
    store.set_fuel(FUEL_PER_CALL)?;
    let instance = linker.instantiate(&mut store, &module)?;
    let io = instance
        .get_typed_func::<(), u32>(&mut store, "oag_io")?
        .call(&mut store, ())? as usize;
    store.set_fuel(budget)?;
    let r = instance
        .get_typed_func::<u32, u32>(&mut store, "spin")?
        .call(&mut store, 1_000_000);
    let trap = r
        .as_ref()
        .err()
        .and_then(|e| e.downcast_ref::<Trap>())
        .copied();
    let memory = instance.get_memory(&mut store, "memory").expect("exported");
    let mut c = [0u8; 4];
    memory.read(&store, io + SPIN_COUNTER, &mut c)?;
    Ok(format!(
        "fuel budget={budget} trap={trap:?} stopped_at={} left={}",
        u32::from_le_bytes(c),
        store.get_fuel()?
    ))
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let kind = args.get(1).map_or("cranelift", String::as_str);
    let wasm = std::fs::read(args.get(2).expect("path to spike_guest.wasm"))?;
    let ticks = 36_000; // ten minutes of race at 60 Hz
    let canon = engine(kind, true)?;

    let mut fresh = Guest::new(canon.clone(), &wasm, Lifetime::Fresh, "on_tick")?;
    let toy = race(&mut fresh, ticks)?;
    println!(
        "race fresh ticks={ticks} hash={:016x} pads={} speed={:08x} fuel={} faulted={}",
        toy.hash(),
        blob_u32(&toy.blob, 0),
        toy.speed.to_bits(),
        toy.fuel_used,
        toy.faulted
    );
    let mut persistent = Guest::new(canon.clone(), &wasm, Lifetime::Persistent, "on_tick")?;
    let toy = race(&mut persistent, ticks)?;
    println!(
        "race persistent hash={:016x} fuel={}",
        toy.hash(),
        toy.fuel_used
    );

    // Starved. Fuel is checked at function entry and loop back-edges only, so a
    // loop-free hook overruns a small budget without trapping (20 and 100 units
    // both ran the whole race unfaulted, identically on both engines); 1 unit
    // traps at the entry check, the mod is faulted and the race runs on.
    let mut starved = Guest::new(canon.clone(), &wasm, Lifetime::Fresh, "on_tick")?;
    starved.fuel = 1;
    let toy = race(&mut starved, ticks)?;
    println!(
        "race starved fuel-per-call=1 hash={:016x} fuel={} faulted={}",
        toy.hash(),
        toy.fuel_used,
        toy.faulted
    );

    // The native oracle counts no fuel; compare everything else.
    let native = native_race(ticks);
    let mut g = Guest::new(canon.clone(), &wasm, Lifetime::Fresh, "on_tick")?;
    let mut wasm_toy = race(&mut g, ticks)?;
    wasm_toy.fuel_used = 0;
    println!(
        "oracle native={:016x} wasm-without-fuel={:016x} equal={}",
        native.hash(),
        wasm_toy.hash(),
        native.hash() == wasm_toy.hash()
    );

    for (label, lifetime, export) in [
        ("fresh/on_tick", Lifetime::Fresh, "on_tick"),
        ("persistent/on_tick", Lifetime::Persistent, "on_tick"),
        ("persistent/on_tick_leaky", Lifetime::Persistent, "on_tick_leaky"),
        ("fresh/on_tick_leaky", Lifetime::Fresh, "on_tick_leaky"),
    ] {
        let mut g = Guest::new(canon.clone(), &wasm, lifetime, export)?;
        let (a, b) = rollback(&mut g, 6000, 2500)?;
        println!(
            "rollback {label} straight={a:016x} replayed={b:016x} equal={}",
            a == b
        );
    }

    // Out of fuel: a runaway loop under a small budget. Where it stopped is read
    // back from the guest's own counter, so a fuel difference between engines
    // shows as a different count.
    for budget in [1_000u64, 10_000] {
        println!("{}", fuel_trap(&canon, &wasm, budget)?);
    }

    for canonicalise in [true, false] {
        let e = engine(kind, canonicalise)?;
        let bits = nan_battery(&e)?;
        let shown: Vec<String> = bits.iter().map(|b| format!("{b:08x}")).collect();
        println!("nan canon={canonicalise} {}", shown.join(" "));
    }

    // Per-call cost. Not compared across runs.
    let n = 20_000u32;
    for (label, lifetime) in [
        ("persistent", Lifetime::Persistent),
        ("fresh", Lifetime::Fresh),
    ] {
        let mut g = Guest::new(canon.clone(), &wasm, lifetime, "on_tick")?;
        let mut toy = Toy::new();
        let t0 = Instant::now();
        for _ in 0..n {
            g.call(&mut toy, "on_tick")?;
        }
        let per = t0.elapsed().as_nanos() as f64 / f64::from(n);
        println!("time {kind} {label} per-call={per:.0}ns");
    }
    let pooled = engine_with(kind, true, true)?;
    let mut g = Guest::new(pooled, &wasm, Lifetime::Fresh, "on_tick")?;
    let mut toy = Toy::new();
    let t0 = Instant::now();
    for _ in 0..n {
        g.call(&mut toy, "on_tick")?;
    }
    let per = t0.elapsed().as_nanos() as f64 / f64::from(n);
    println!("time {kind} fresh-pooled per-call={per:.0}ns");
    let t0 = Instant::now();
    let _ = Module::new(&canon, &wasm)?;
    println!(
        "time {kind} compile={}us guest-bytes={}",
        t0.elapsed().as_micros(),
        wasm.len()
    );
    Ok(())
}
