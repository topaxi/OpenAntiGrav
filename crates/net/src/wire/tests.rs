use super::*;
use oag_gameplay::input::Button;

fn held(buttons: u32, x: f32) -> Held {
    Held {
        buttons,
        stick_x: x,
        stick_y: -0.0,
        airbrake_left: 0.25,
        airbrake_right: 1.0,
    }
}

fn params() -> RaceParams {
    RaceParams {
        title: "Wipeout Pulse".into(),
        track: "01_Track".into(),
        team: "Feisar".into(),
        teams: vec!["Feisar".into(), String::new(), "Qirex".into()],
        class: "Venom".into(),
        mode: "single_race".into(),
        difficulty: "elite".into(),
        scheme: "veteran".into(),
        seed: 0xDEAD_BEEF_0123_4567,
        people: 2,
        opponents: true,
        weapons: false,
    }
}

#[test]
fn every_control_message_round_trips() {
    let messages = [
        Control::Hello {
            protocol: PROTOCOL,
            name: "client".into(),
            team: "Qirex".into(),
            token: "abc123".into(),
        },
        Control::Welcome {
            seat: 1,
            params: params(),
            fingerprint: 0x0123_4567_89AB_CDEF,
        },
        Control::Reject {
            reason: "full".into(),
        },
        Control::Ready { fingerprint: 7 },
        Control::Start { lead_ticks: 6 },
        Control::Resend { from: 4000 },
        Control::Ticks(Confirmed {
            first: 12,
            ticks: vec![
                TickRecord {
                    hash: 1,
                    inputs: vec![held(1, 0.5), held(2, -1.0)],
                },
                TickRecord {
                    hash: u64::MAX,
                    inputs: vec![held(3, 0.0), held(0, 1.0e-7)],
                },
            ],
        }),
        Control::End {
            reason: "finished".into(),
        },
    ];
    for message in messages {
        assert_eq!(Control::decode(&message.encode()), Ok(message.clone()));
    }
}

#[test]
fn every_datagram_round_trips_with_the_exact_bits() {
    let inputs = Datagram::Inputs {
        seat: 3,
        first: 99,
        inputs: vec![held(5, f32::MIN_POSITIVE), held(6, -0.3333333)],
    };
    let decoded = Datagram::decode(&inputs.encode()).expect("parses");
    assert_eq!(decoded, inputs);
    let Datagram::Inputs { inputs: back, .. } = decoded else {
        unreachable!()
    };
    // -0.0 == 0.0 under PartialEq; the bits must survive too.
    assert_eq!(back[0].stick_y.to_bits(), (-0.0f32).to_bits());
    let confirmed = Datagram::Confirmed {
        lead: -3,
        confirmed: Confirmed {
            first: 0,
            ticks: vec![TickRecord {
                hash: 42,
                inputs: vec![held(1, 0.1)],
            }],
        },
    };
    assert_eq!(Datagram::decode(&confirmed.encode()), Ok(confirmed));
}

#[test]
fn a_bad_message_is_refused_by_name() {
    let hello = Control::Hello {
        protocol: PROTOCOL,
        name: "x".into(),
        team: String::new(),
        token: String::new(),
    }
    .encode();
    assert_eq!(
        Control::decode(&hello[..hello.len() - 1]),
        Err(Malformed("truncated"))
    );
    let mut wrong = hello.clone();
    wrong[1] = b'X';
    assert_eq!(
        Control::decode(&wrong),
        Err(Malformed("hello without the OAGN magic"))
    );
    let mut long = hello;
    long.push(0);
    assert_eq!(Control::decode(&long), Err(Malformed("trailing bytes")));
    assert_eq!(
        Datagram::decode(&[0x7f]),
        Err(Malformed("unknown datagram"))
    );
}

/// The server and the client derive edges from the held stream the same way, and
/// it is the way a device's own snapshot has them.
#[test]
fn edges_are_derived_from_the_held_stream() {
    let cross = Button::Cross.bit();
    let mut edges = Input::new();
    let first = held(cross, 0.0).snapshot(&mut edges);
    assert!(first.buttons.is_pressed(Button::Cross));
    let second = held(cross, 0.0).snapshot(&mut edges);
    assert!(second.buttons.is_held(Button::Cross));
    assert!(!second.buttons.is_pressed(Button::Cross));
    let mut device = Input::new();
    device.begin_frame(cross);
    let snapshot = InputSnapshot {
        buttons: device,
        stick_x: 2.0,
        ..InputSnapshot::default()
    };
    // `of` clamps, so the wire never carries a value the sim would not apply.
    assert_eq!(Held::of(&snapshot).stick_x, 1.0);
    // The held mask crosses as the device has it (the synthetic Any bit included),
    // and the far end's derivation from it holds the same buttons.
    let wire = Held::of(&snapshot);
    assert_eq!(wire.buttons, device.held_mask());
    let mut far = Input::new();
    assert_eq!(
        wire.snapshot(&mut far).buttons.held_mask(),
        device.held_mask()
    );
}

#[test]
fn a_full_pack_of_confirmed_ticks_fits_a_datagram() {
    // 1,413 bytes is what both QUIC stacks offered on localhost in the spike.
    let per_tick = Confirmed::record_bytes(8);
    assert!(per_tick * 7 + 16 <= 1_413, "{per_tick}");
}
