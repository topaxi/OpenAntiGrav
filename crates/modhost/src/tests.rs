use super::*;
use oag_ext_abi::Slot;

/// A whole module in WAT, `{ABI_WORD}` standing for this host's `oag_abi` value.
fn module(src: &str) -> Vec<u8> {
    wat::parse_str(src.replace("{ABI_WORD}", &ABI_WORD.to_string())).unwrap()
}

/// A guest in WAT: the buffer at 1024, `tick` running `$body`.
fn guest(body: &str) -> Vec<u8> {
    wat::parse_str(format!(
        r#"(module
            (memory (export "memory") 1)
            (func (export "oag_abi") (result i32) i32.const {ABI_WORD})
            (func (export "oag_io") (result i32) i32.const 1024)
            (func (export "tick") (result i32) {body}))"#
    ))
    .unwrap()
}

const STATE: i32 = 1024 + oag_ext_abi::STATE_AT as i32;
const REPLY: i32 = 1024 + oag_ext_abi::REPLY_AT as i32;

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
        crafts: &[],
        track: &[],
        speed_pads: 0,
    }
}

#[test]
fn a_tick_returns_the_blob_and_a_checked_command() {
    let module = Module::compile(&counter("(f32.const 0.25)"), Limits::default()).unwrap();
    let mut state = vec![0; 8];
    let mut commands = Vec::new();
    for tick in 0..3 {
        module
            .tick(&observe(tick), &mut state, &mut commands)
            .unwrap();
    }
    assert_eq!(&state[..4], &3_u32.to_le_bytes());
    assert_eq!(
        commands,
        [Command::ScaleThrust {
            slot: Slot::new(1).unwrap(),
            factor: 0.25
        }]
    );
    assert_eq!(engines_created(), 1);
}

#[test]
fn a_factor_outside_zero_to_two_or_nan_is_a_fault_not_a_clamp() {
    for factor in [
        "(f32.const 2.5)",
        "(f32.const -0.1)",
        "(f32.div (f32.const 0) (f32.const 0))",
    ] {
        let module = Module::compile(&counter(factor), Limits::default()).unwrap();
        let mut state = vec![7; 8];
        let mut commands = Vec::new();
        let (fault, _) = module
            .tick(&observe(0), &mut state, &mut commands)
            .unwrap_err();
        assert!(matches!(fault, Fault::Reply(_)), "{factor}: {fault:?}");
        assert_eq!(
            state, [7; 8],
            "{factor}: the blob keeps its value from before the call"
        );
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
        module
            .tick(&observe(tick), &mut state, &mut Vec::new())
            .unwrap();
        assert_eq!(state, 1_u32.to_le_bytes());
    }
}

#[test]
fn a_module_is_refused_at_load_by_name() {
    let import = module(
        r#"(module (import "wasi_snapshot_preview1" "clock_time_get" (func))
            (memory (export "memory") 1)
            (func (export "oag_abi") (result i32) i32.const {ABI_WORD})
            (func (export "oag_io") (result i32) i32.const 0)
            (func (export "tick") (result i32) i32.const 0))"#,
    );
    assert_eq!(
        Module::compile(&import, Limits::default()).unwrap_err(),
        LoadError::Import("`wasi_snapshot_preview1.clock_time_get`".to_string())
    );
    let no_tick = module(
        r#"(module (memory (export "memory") 1)
            (func (export "oag_abi") (result i32) i32.const {ABI_WORD})
            (func (export "oag_io") (result i32) i32.const 0))"#,
    );
    assert!(matches!(
        Module::compile(&no_tick, Limits::default()),
        Err(LoadError::Export(e)) if e.contains("tick")
    ));
    let simd = guest("(drop (v128.const i32x4 0 0 0 0)) i32.const 0");
    assert!(matches!(
        Module::compile(&simd, Limits::default()),
        Err(LoadError::Compile(_))
    ));
    let big = module(
        r#"(module (memory (export "memory") 32)
            (func (export "oag_abi") (result i32) i32.const {ABI_WORD})
            (func (export "oag_io") (result i32) i32.const 0)
            (func (export "tick") (result i32) i32.const 0))"#,
    );
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
        crafts: &[],
        track: &[],
        speed_pads: 0,
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
        slot = 1024 + oag_ext_abi::EVENTS_AT as i32 + 1,
    ));
    let module = Module::compile(&echo, Limits::default()).unwrap();
    let events = [
        Event::WallContact {
            slot: Slot::new(5).unwrap(),
        },
        Event::WallContact {
            slot: Slot::new(2).unwrap(),
        },
    ];
    let mut state = vec![0; 8];
    module
        .tick(
            &Observation {
                tick: 9,
                active: 0xff,
                events: &events,
                crafts: &[],
                track: &[],
                speed_pads: 0,
            },
            &mut state,
            &mut Vec::new(),
        )
        .unwrap();
    assert_eq!(state, [2, 0, 0, 0, 5, 0, 0, 0]);
}

/// A module whose `oag_abi` returns `word`, otherwise well-formed.
fn answering(word: u32, extra: &str) -> Vec<u8> {
    module(&format!(
        r#"(module (memory (export "memory") 1)
            (func (export "oag_abi") (result i32) i32.const {word})
            (func (export "oag_io") (result i32) i32.const 0)
            (func (export "tick") (result i32) i32.const 0)
            {extra})"#
    ))
}

#[test]
fn another_layout_revision_is_refused_at_load_naming_both() {
    let other = REVISION + 1;
    let error = Module::compile(&answering((ABI << 8) | other, ""), Limits::default()).unwrap_err();
    assert_eq!(
        error,
        LoadError::Abi {
            major: ABI,
            revision: Some(other)
        }
    );
    let text = error.to_string();
    assert!(
        text.contains(&format!("rev {other}")) && text.contains(&format!("rev {REVISION}")),
        "{text}"
    );
    // A module from before revisions returns the bare major.
    let legacy = Module::compile(&answering(ABI, ""), Limits::default()).unwrap_err();
    assert_eq!(
        legacy,
        LoadError::Abi {
            major: ABI,
            revision: None
        }
    );
    assert!(
        legacy.to_string().contains("without a layout revision"),
        "{legacy}"
    );
    assert!(Module::compile(&answering(ABI_WORD, ""), Limits::default()).is_ok());
}

#[test]
fn a_state_length_export_must_agree_with_the_manifest() {
    let says_16 = r#"(func (export "oag_state_len") (result i32) i32.const 16)"#;
    let limits = |state_bytes| Limits {
        state_bytes,
        ..Limits::default()
    };
    assert!(Module::compile(&answering(ABI_WORD, says_16), limits(16)).is_ok());
    let error = Module::compile(&answering(ABI_WORD, says_16), limits(96)).unwrap_err();
    assert_eq!(
        error,
        LoadError::State {
            module: 16,
            manifest: 96
        }
    );
    assert!(error.to_string().contains("state_bytes = 96"), "{error}");
}

/// A guest whose track buffer is at 8192: copies the region's first property value
/// into the blob and sends the reply `$reply` writes.
fn track_reader(reply: &str) -> Vec<u8> {
    wat::parse_str(format!(
        r#"(module
            (memory (export "memory") 1)
            (func (export "oag_abi") (result i32) i32.const {ABI_WORD})
            (func (export "oag_io") (result i32) i32.const 1024)
            (func (export "oag_track") (result i32) i32.const 8192)
            (func (export "tick") (result i32)
              (i32.store (i32.const {STATE}) (i32.load (i32.const {value})))
              {reply}
              i32.const 0))"#,
        value = 8192 + oag_ext_abi::track::PROPS_AT + 4,
    ))
    .unwrap()
}

/// One race-wide `scale-pad` for `pad`.
fn scale_pad(pad: u16) -> String {
    format!(
        "(i32.store (i32.const {REPLY}) (i32.const 1))
         (i32.store8 (i32.const {cmd}) (i32.const 13))
         (i32.store16 (i32.const {arg}) (i32.const {pad}))
         (f32.store (i32.const {f}) (f32.const 2.0))",
        cmd = REPLY + 4,
        arg = REPLY + 6,
        f = REPLY + 8,
    )
}

#[test]
fn the_track_region_reaches_a_guest_that_exports_its_buffer() {
    let mut region = vec![0_u8; oag_ext_abi::track::TRACK_LEN];
    let props = [(oag_ext_abi::track::prop_key("boost"), 1.5_f32)];
    let used = oag_ext_abi::track::write(&mut region, &props, &[], &[], &[]).unwrap();
    let module = Module::compile(&track_reader(""), Limits::default()).unwrap();
    let mut state = vec![0; 4];
    let observation = Observation {
        track: &region[..used],
        ..observe(0)
    };
    module
        .tick(&observation, &mut state, &mut Vec::new())
        .unwrap();
    assert_eq!(state, 1.5_f32.to_le_bytes());
}

#[test]
fn a_scale_pad_for_a_pad_the_track_lacks_is_refused() {
    let module = Module::compile(&track_reader(&scale_pad(3)), Limits::default()).unwrap();
    let mut commands = Vec::new();
    let with = |speed_pads| Observation {
        speed_pads,
        ..observe(0)
    };
    module.tick(&with(4), &mut [0; 4], &mut commands).unwrap();
    assert_eq!(
        commands,
        [Command::ScalePad {
            pad: 3,
            factor: 2.0
        }]
    );
    let (fault, _) = module
        .tick(&with(3), &mut [0; 4], &mut commands)
        .unwrap_err();
    assert!(matches!(fault, Fault::Reply(e) if e.contains("speed pad 3")));
}

#[test]
fn two_shield_commands_for_one_slot_in_a_tick_are_refused() {
    let one = |kinds: &[u8]| {
        let body = format!(
            "(i32.store (i32.const {REPLY}) (i32.const {count}))\n{commands}",
            count = kinds.len(),
            commands = kinds
                .iter()
                .enumerate()
                .map(|(i, kind)| {
                    let at = i as i32 * oag_ext_abi::COMMAND_LEN as i32;
                    format!(
                        "(i32.store8 (i32.const {c}) (i32.const {kind}))
                         (i32.store8 (i32.const {s}) (i32.const 1))",
                        c = REPLY + 4 + at,
                        s = REPLY + 5 + at,
                    )
                })
                .collect::<String>(),
        );
        let module = Module::compile(&track_reader(&body), Limits::default()).unwrap();
        let mut commands = Vec::new();
        module
            .tick(&observe(0), &mut [0; 4], &mut commands)
            .map(|_| commands)
    };
    assert_eq!(
        one(&[25]).unwrap(),
        [Command::HoldShield {
            slot: oag_ext_abi::Slot::new(1).unwrap()
        }]
    );
    for pair in [[24, 25], [25, 26], [26, 26]] {
        let (fault, _) = one(&pair).unwrap_err();
        assert!(
            matches!(&fault, Fault::Reply(e) if e.contains("two shield commands")),
            "{pair:?}: {fault}"
        );
    }
}
