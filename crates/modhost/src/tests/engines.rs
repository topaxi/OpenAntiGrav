//! Which engine runs the modules, and that native Cranelift code and the Pulley
//! interpreter agree on every byte a tick returns (`docs/architecture/wasm-extensions/README.md`,
//! section 5).

use super::*;

#[test]
fn the_engine_is_the_one_the_build_asked_for() {
    Module::compile(&counter("(f32.const 0.25)"), Limits::default()).unwrap();
    let want = if cfg!(feature = "pulley") {
        "Pulley"
    } else {
        "Cranelift"
    };
    assert_eq!(engine_name(), Some(want));
}

#[cfg(feature = "pulley")]
/// A guest that makes NaNs of odd payloads and stores their bits, loops on integer and
/// float arithmetic, and asks slot 1 for a factor derived from them: the places the
/// two backends could differ (NaN bits, float rounding, fuel at loop headers).
fn arithmetic() -> Vec<u8> {
    guest(&format!(
        "(local $i i32) (local $x f32) (local $n f32)
         (local.set $n (f32.reinterpret_i32 (i32.const 0x7fa00001)))
         (local.set $x (f32.const 1.0))
         (loop $l
           (local.set $x (f32.add (f32.mul (local.get $x) (f32.const 1.0001)) (f32.const 0.0003)))
           (local.set $n (f32.add (local.get $n) (f32.const 1.0)))
           (local.set $i (i32.add (local.get $i) (i32.const 1)))
           (br_if $l (i32.lt_u (local.get $i) (i32.const 500))))
         (i32.store (i32.const {STATE}) (i32.reinterpret_f32 (local.get $x)))
         (i32.store (i32.const {n}) (i32.reinterpret_f32 (local.get $n)))
         (i32.store (i32.const {div}) (i32.reinterpret_f32 (f32.div (f32.const 0) (f32.const 0))))
         (i32.store (i32.const {sqrt}) (i32.reinterpret_f32 (f32.sqrt (f32.const -1))))
         (i32.store (i32.const {sub}) (i32.reinterpret_f32 (f32.sub (f32.const inf) (f32.const inf))))
         (i32.store (i32.const {trunc}) (i32.trunc_sat_f32_s (f32.mul (local.get $x) (f32.const 1e10))))
         (i32.store (i32.const {demote}) (i32.reinterpret_f32 (f32.demote_f64 (f64.promote_f32 (local.get $n)))))
         (i32.store (i32.const {REPLY}) (i32.const 1))
         (i32.store8 (i32.const {cmd}) (i32.const 1))
         (i32.store8 (i32.const {slot}) (i32.const 1))
         (f32.store (i32.const {f}) (f32.const 0.5))
         i32.const 0",
        n = STATE + 4,
        div = STATE + 8,
        sqrt = STATE + 12,
        sub = STATE + 16,
        trunc = STATE + 20,
        demote = STATE + 24,
        cmd = REPLY + 4,
        slot = REPLY + 5,
        f = REPLY + 8,
    ))
}

#[cfg(feature = "pulley")]
/// What one run of `ticks` ticks returned: the blob, the commands and the cost of each tick.
fn run(engine: &Engine, wasm: &[u8], ticks: u64) -> (Vec<u8>, Vec<Vec<Command>>, Vec<Cost>) {
    let module = Module::compile_on(engine, wasm, Limits::default()).unwrap();
    let mut state = vec![0; 32];
    let (mut replies, mut costs) = (Vec::new(), Vec::new());
    for tick in 0..ticks {
        let mut commands = Vec::new();
        costs.push(
            module
                .tick(&observe(tick), &mut state, &mut commands)
                .unwrap(),
        );
        replies.push(commands);
    }
    (state, replies, costs)
}

#[cfg(feature = "pulley")]
#[test]
fn pulley_and_native_code_return_the_same_bytes_fuel_and_commands() {
    let native = build_engine(false).unwrap();
    let pulley = build_engine(true).unwrap();
    assert!(!native.is_pulley() && pulley.is_pulley());
    for (name, wasm) in [
        ("counter", counter("(f32.const 0.25)")),
        ("arithmetic", arithmetic()),
    ] {
        let a = run(&native, &wasm, 200);
        let b = run(&pulley, &wasm, 200);
        assert_eq!(a, b, "{name}: native code and Pulley disagree");
    }
    let (state, ..) = run(&pulley, &arithmetic(), 1);
    for at in [8, 12, 16] {
        let bits = u32::from_le_bytes(state[at..at + 4].try_into().unwrap());
        assert_eq!(bits, 0x7fc0_0000, "NaN at {at} is the canonical quiet NaN");
    }
}

/// The wall time of 2,000 ticks of each guest on each engine, printed for a person
/// comparing the two (`--no-capture`); nothing is asserted, wall time is not a result.
#[cfg(feature = "pulley")]
#[test]
fn per_tick_wall_time_of_each_engine() {
    let native = build_engine(false).unwrap();
    let pulley = build_engine(true).unwrap();
    for (name, wasm) in [
        ("counter", counter("(f32.const 0.25)")),
        ("arithmetic", arithmetic()),
    ] {
        for (engine_name, engine) in [("Cranelift", &native), ("Pulley", &pulley)] {
            let start = std::time::Instant::now();
            let (_, _, costs) = run(engine, &wasm, 2_000);
            let per_tick = start.elapsed().as_secs_f64() * 1e6 / 2_000.0;
            eprintln!(
                "{name} on {engine_name}: {per_tick:.1} us per tick, {} fuel",
                costs[0].fuel_used
            );
        }
    }
}
