use super::*;

/// A guest in WAT: the buffer at 1024, `tick` running `$body`.
fn guest(body: &str) -> Vec<u8> {
    wat::parse_str(format!(
        r#"(module
            (memory (export "memory") 1)
            (func (export "oag_abi") (result i32) i32.const 1)
            (func (export "oag_io") (result i32) i32.const 1024)
            (func (export "tick") (result i32) {body}))"#
    ))
    .unwrap()
}

const STATE: i32 = 1024 + abi::STATE_AT as i32;
const REPLY: i32 = 1024 + abi::REPLY_AT as i32;

/// Counts ticks in the blob's first word and asks slot 1 for `factor`.
fn counter(factor: &str) -> Vec<u8> {
    guest(&format!(
        "(i32.store (i32.const {STATE}) (i32.add (i32.load (i32.const {STATE})) (i32.const 1)))
         (i32.store (i32.const {REPLY}) (i32.const 1))
         (i32.store8 (i32.const {cmd}) (i32.const 1))
         (i32.store8 (i32.const {slot}) (i32.const 1))
         (f32.store (i32.const {f}) {factor})
         i32.const 0",
        cmd = REPLY + 4,
        slot = REPLY + 5,
        f = REPLY + 8,
    ))
}

fn observe(tick: u64) -> Observation<'static> {
    Observation {
        tick,
        active: 0b11,
        events: &[],
    }
}

#[test]
fn a_tick_returns_the_blob_and_a_checked_command() {
    let module = Module::compile(&counter("(f32.const 0.25)"), Limits::default()).unwrap();
    let mut state = vec![0; 8];
    let mut commands = Vec::new();
    for tick in 0..3 {
        module.tick(&observe(tick), &mut state, &mut commands).unwrap();
    }
    assert_eq!(&state[..4], &3_u32.to_le_bytes());
    assert_eq!(
        commands,
        [Command::ScaleThrust {
            slot: 1,
            factor: 0.25
        }]
    );
    assert_eq!(engines_created(), 1);
}

#[test]
fn a_factor_outside_zero_to_one_or_nan_is_a_fault_not_a_clamp() {
    for factor in ["(f32.const 1.5)", "(f32.const -0.1)", "(f32.div (f32.const 0) (f32.const 0))"] {
        let module = Module::compile(&counter(factor), Limits::default()).unwrap();
        let mut state = vec![7; 8];
        let mut commands = Vec::new();
        let (fault, _) = module
            .tick(&observe(0), &mut state, &mut commands)
            .unwrap_err();
        assert!(matches!(fault, Fault::Reply(_)), "{factor}: {fault:?}");
        assert_eq!(state, [7; 8], "{factor}: the blob keeps its value from before the call");
        assert!(commands.is_empty());
    }
}

#[test]
fn a_loop_runs_out_of_fuel_at_the_same_point_every_time() {
    let spin = guest("(loop $l (br $l)) i32.const 0");
    let limits = Limits {
        fuel: 5_000,
        ..Limits::default()
    };
    let module = Module::compile(&spin, limits).unwrap();
    let costs: Vec<_> = (0..2)
        .map(|_| {
            let (fault, cost) = module
                .tick(&observe(0), &mut [0; 4], &mut Vec::new())
                .unwrap_err();
            assert_eq!(fault, Fault::OutOfFuel);
            cost
        })
        .collect();
    assert_eq!(costs[0], costs[1]);
    assert_eq!(costs[0].fuel_used, 5_000);
}

#[test]
fn an_instance_is_fresh_every_tick() {
    // A counter in the guest's own memory, not the blob: it must read 1 every tick.
    let leaky = guest(&format!(
        "(i32.store (i32.const 64) (i32.add (i32.load (i32.const 64)) (i32.const 1)))
         (i32.store (i32.const {STATE}) (i32.load (i32.const 64)))
         i32.const 0"
    ));
    let module = Module::compile(&leaky, Limits::default()).unwrap();
    let mut state = vec![0; 4];
    for tick in 0..3 {
        module.tick(&observe(tick), &mut state, &mut Vec::new()).unwrap();
        assert_eq!(state, 1_u32.to_le_bytes());
    }
}

#[test]
fn a_module_is_refused_at_load_by_name() {
    let import = wat::parse_str(
        r#"(module (import "wasi_snapshot_preview1" "clock_time_get" (func))
            (memory (export "memory") 1)
            (func (export "oag_abi") (result i32) i32.const 1)
            (func (export "oag_io") (result i32) i32.const 0)
            (func (export "tick") (result i32) i32.const 0))"#,
    )
    .unwrap();
    assert_eq!(
        Module::compile(&import, Limits::default()).unwrap_err(),
        LoadError::Import("`wasi_snapshot_preview1.clock_time_get`".to_string())
    );
    let no_tick = wat::parse_str(
        r#"(module (memory (export "memory") 1)
            (func (export "oag_abi") (result i32) i32.const 1)
            (func (export "oag_io") (result i32) i32.const 0))"#,
    )
    .unwrap();
    assert!(matches!(
        Module::compile(&no_tick, Limits::default()),
        Err(LoadError::Export(e)) if e.contains("tick")
    ));
    let simd = guest("(drop (v128.const i32x4 0 0 0 0)) i32.const 0");
    assert!(matches!(
        Module::compile(&simd, Limits::default()),
        Err(LoadError::Compile(_))
    ));
    let big = wat::parse_str(
        r#"(module (memory (export "memory") 32)
            (func (export "oag_abi") (result i32) i32.const 1)
            (func (export "oag_io") (result i32) i32.const 0)
            (func (export "tick") (result i32) i32.const 0))"#,
    )
    .unwrap();
    assert!(matches!(
        Module::compile(&big, Limits::default()),
        Err(LoadError::Start(_))
    ));
}

#[test]
fn a_slot_with_no_craft_is_refused() {
    let module = Module::compile(&counter("(f32.const 0.5)"), Limits::default()).unwrap();
    let only_slot_0 = Observation {
        tick: 0,
        active: 0b1,
        events: &[],
    };
    let (fault, _) = module
        .tick(&only_slot_0, &mut [0; 4], &mut Vec::new())
        .unwrap_err();
    assert!(matches!(fault, Fault::Reply(e) if e.contains("slot 1")));
}

#[test]
fn events_reach_the_guest() {
    // Writes the event count and the first event's slot into the blob.
    let echo = guest(&format!(
        "(i32.store (i32.const {STATE}) (i32.load (i32.const {count})))
         (i32.store8 (i32.const {s4}) (i32.load8_u (i32.const {slot})))
         i32.const 0",
        count = 1024 + 24,
        s4 = STATE + 4,
        slot = 1024 + abi::EVENTS_AT as i32 + 1,
    ));
    let module = Module::compile(&echo, Limits::default()).unwrap();
    let events = [Event::WallContact { slot: 5 }, Event::WallContact { slot: 2 }];
    let mut state = vec![0; 8];
    module
        .tick(
            &Observation {
                tick: 9,
                active: 0xff,
                events: &events,
            },
            &mut state,
            &mut Vec::new(),
        )
        .unwrap();
    assert_eq!(state, [2, 0, 0, 0, 5, 0, 0, 0]);
}
