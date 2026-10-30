use super::*;
use crate::test::{CraftRecord, Race};

fn slot(index: usize) -> Slot {
    Slot::new(index).unwrap()
}

crate::blob! {
    /// Energy Turbo's per-slot record, as it was laid out by hand before the SDK.
    #[derive(Debug, Clone, Copy, PartialEq, Default)]
    struct Record {
        spent: f32,
        burst: u16,
        boosting: bool,
        spark: u8,
        rest: u16,
        _spare: u16,
    }
}

crate::blob! {
    #[derive(Debug, Clone, Copy, PartialEq, Default)]
    struct Nested {
        records: [Record; 2],
        tail: i8,
    }
}

#[test]
fn a_blob_is_its_fields_in_order_little_endian_without_padding() {
    assert_eq!(Record::BYTES, 12);
    let record = Record {
        spent: 1.5,
        burst: 0x0102,
        boosting: true,
        spark: 7,
        rest: 0x0304,
        _spare: 0,
    };
    let mut bytes = [0xAA; 12];
    record.to_bytes(&mut bytes);
    // The offsets Energy Turbo wrote by hand: 0, 4, 6, 7, 8, 2 spare.
    let mut want = [0; 12];
    want[..4].copy_from_slice(&1.5_f32.to_le_bytes());
    want[4..].copy_from_slice(&[0x02, 0x01, 1, 7, 0x04, 0x03, 0, 0]);
    assert_eq!(bytes, want);
    assert_eq!(Record::from_bytes(&bytes), record);

    assert_eq!(Nested::BYTES, 25);
    let nested = Nested {
        records: [Record::default(), record],
        tail: -2,
    };
    let mut bytes = [0; 25];
    nested.to_bytes(&mut bytes);
    assert_eq!(bytes[12..24], want);
    assert_eq!(bytes[24], 0xFE);
    assert_eq!(Nested::from_bytes(&bytes), nested);
    // Short input reads as zeros, never out of bounds.
    assert_eq!(Record::from_bytes(&[]), Record::default());
}

crate::blob! {
    #[derive(Debug, Default)]
    struct Seen {
        ticks: u32,
        walls: [u8; 4],
        last_shield: f32,
    }
}

/// Counts ticks and wall contacts, remembers slot 1's shield, and scales the thrust
/// of every craft that touched a wall.
struct Probe;

impl Extension for Probe {
    type State = Seen;

    fn tick(obs: &Observation<'_>, state: &mut Seen, reply: &mut Reply<'_>) {
        state.ticks += 1;
        for event in obs.events() {
            if let Event::WallContact { slot } = event
                && let Some(count) = state.walls.get_mut(slot.index())
            {
                *count += 1;
                if let Some(craft) = obs.craft(slot) {
                    reply.scale_thrust(&craft, 0.5).unwrap();
                }
            }
        }
        if let Some(craft) = obs.craft(slot(1)) {
            state.last_shield = craft.shield();
        }
    }
}

fn racer(shield: f32) -> CraftRecord {
    CraftRecord {
        racing: true,
        shield,
        shield_max: 100.0,
        ..CraftRecord::default()
    }
}

#[test]
fn an_extension_runs_natively_through_the_hosts_writer_and_reader() {
    let mut race = Race::<Probe>::new()
        .with_craft(slot(0), racer(10.0))
        .with_craft(slot(1), racer(42.0));
    race.event(Event::WallContact { slot: slot(1) })
        .event(Event::WallContact { slot: slot(3) });
    let commands = race.tick();
    // Slot 3 holds no craft: the contact is counted, no command names it.
    assert_eq!(
        commands,
        [Command::ScaleThrust {
            slot: slot(1),
            factor: 0.5
        }]
    );
    let state = race.state();
    assert_eq!(
        (state.ticks, state.walls, state.last_shield),
        (1, [0, 1, 0, 1], 42.0)
    );
    // Events are one call's; the state carries.
    assert!(race.tick().is_empty());
    assert_eq!(race.state().ticks, 2);
    *race.craft_mut(slot(1)) = None;
    race.tick();
    assert_eq!(race.state().last_shield, 42.0);
}

#[test]
fn the_observation_lists_active_crafts_and_skips_unknown_events() {
    let mut io = Box::new([0_u8; IO_LEN]);
    let crafts = [racer(1.0), racer(2.0), racer(3.0)];
    let events = [
        Event::SpeedPad {
            slot: slot(2),
            pad: 1,
        },
        Event::PickupFire { slot: slot(0) },
    ];
    oag_ext_abi::write_observation(&mut io, 77, 0b101, &events, &crafts, &[]);
    // An event kind from a later revision.
    io[oag_ext_abi::EVENTS_AT + oag_ext_abi::EVENT_KIND] = 200;
    let sections = Sections::split(&mut io);
    let obs = Observation::read(sections.head, sections.crafts, &[]);
    assert_eq!(obs.tick(), 77);
    let shields: Vec<_> = obs.crafts().map(|c| (c.slot(), c.shield())).collect();
    assert_eq!(shields, [(slot(0), 1.0), (slot(2), 3.0)]);
    assert!(obs.craft(slot(1)).is_none());
    let seen: Vec<_> = obs.events().collect();
    assert_eq!(seen, [Event::PickupFire { slot: slot(0) }]);
    assert_eq!(obs.track().speed_pads(), 0);
}

#[test]
fn the_reply_refuses_what_the_host_would_fault_on() {
    let mut buffer = [0_u8; 4 + oag_ext_abi::COMMAND_LEN * oag_ext_abi::MAX_COMMANDS];
    let mut reply = Reply::new(&mut buffer, 2);
    let record = CraftRecord {
        pickup: Pickup::Ours,
        ..racer(50.0)
    }
    .encode();
    let held = Craft {
        slot: slot(2),
        record: &record,
    };
    for bad in [2.5, -0.1, f32::NAN, f32::INFINITY] {
        assert_eq!(reply.scale_thrust(&held, bad), Err(Refused::Range));
    }
    assert_eq!(reply.scale_thrust(&held, 2.0), Ok(()));
    assert_eq!(reply.scale_thrust(&held, 1.0), Err(Refused::Repeated));
    assert_eq!(reply.shield_delta(&held, -100.5), Err(Refused::Range));
    assert_eq!(reply.shield_delta(&held, f32::NAN), Err(Refused::Range));
    assert_eq!(reply.shield_delta(&held, -100.0), Ok(()));
    assert_eq!(reply.shield_delta(&held, 1.0), Err(Refused::Repeated));
    assert_eq!(reply.give_pickup(&held), Err(Refused::Pickup));
    assert_eq!(reply.clear_pickup(&held), Ok(()));
    assert_eq!(reply.pickup_roll(Weapon::Turbo, 0), Err(Refused::Range));
    for bad in [-0.1, 1.1, f32::NAN, f32::INFINITY] {
        assert_eq!(
            reply.set_weather(WeatherKind::Rain, bad),
            Err(Refused::Range)
        );
        assert_eq!(reply.set_sun(bad), Err(Refused::Range));
        assert_eq!(reply.grade_sky(bad), Err(Refused::Range));
    }
    for bad in [2.5, -0.1, f32::NAN] {
        assert_eq!(reply.scale_pad(0, bad), Err(Refused::Range));
        assert_eq!(reply.scale_slot_pads(&held, bad), Err(Refused::Range));
    }
    assert_eq!(
        reply.scale_pad(2, 1.0),
        Err(Refused::Range),
        "the track has two"
    );
    assert_eq!(reply.scale_pad(1, 2.0), Ok(()));
    assert_eq!(reply.scale_pad(1, 1.0), Err(Refused::Repeated));
    assert_eq!(reply.scale_slot_pads(&held, 0.0), Ok(()));
    assert_eq!(reply.scale_slot_pads(&held, 1.0), Err(Refused::Repeated));
    assert_eq!(reply.len(), 5);
    while reply.len() < MAX_COMMANDS_FOR_TEST {
        reply.pickup_tint(&held, Rgba::default()).unwrap();
    }
    assert_eq!(reply.play_cue(&held, Cue::Turbo), Err(Refused::Full));
    reply.finish();
    assert_eq!(buffer[..4], 64_u32.to_le_bytes());
}

const MAX_COMMANDS_FOR_TEST: usize = oag_ext_abi::MAX_COMMANDS;

#[test]
fn a_state_length_other_than_the_blobs_is_a_failed_tick() {
    let mut io = Box::new([0_u8; IO_LEN]);
    oag_ext_abi::write_observation(&mut io, 0, 0, &[], &[], &[0; 8]);
    assert_eq!(run::<Probe>(&mut io, &[]), 1);
    oag_ext_abi::write_observation(&mut io, 0, 0, &[], &[], &[0; 12]);
    assert_eq!(run::<Probe>(&mut io, &[]), 0);
}

crate::blob! {
    #[derive(Debug, Default)]
    struct PadLog {
        boost: [f32; 2],
        found: [bool; 2],
        speed: u8,
        entered: u8,
        left: u8,
        weapon: u8,
        whole: f32,
    }
}

/// Reads `boost` off speed pads 0 and 1 and the track's `laps`, logs the last pad and
/// trigger events, and on a speed pad doubles that pad for the race.
struct PadReader;

impl Extension for PadReader {
    type State = PadLog;

    fn tick(obs: &Observation<'_>, state: &mut PadLog, reply: &mut Reply<'_>) {
        let track = obs.track();
        for pad in 0..2 {
            let boost = track
                .speed_pad(pad)
                .and_then(|p| p.prop(crate::prop!("boost")));
            state.found[usize::from(pad)] = boost.is_some();
            state.boost[usize::from(pad)] = boost.unwrap_or(0.0);
        }
        state.whole = track.whole().prop(crate::prop!("laps")).unwrap_or(0.0);
        for event in obs.events() {
            match event {
                Event::SpeedPad { pad, .. } => {
                    state.speed = pad as u8;
                    reply.scale_pad(pad, 2.0).unwrap();
                }
                Event::WeaponPad { pad, .. } => state.weapon = pad as u8,
                Event::TriggerEnter { trigger, .. } => state.entered = trigger as u8,
                Event::TriggerExit { trigger, .. } => state.left = trigger as u8,
                _ => {}
            }
        }
    }
}

#[test]
fn a_guest_reads_a_pads_property_by_its_compile_time_key_and_hears_pads_and_triggers() {
    use crate::test::TrackRecord;
    let boost = crate::prop!("boost");
    assert_eq!(boost, prop_key("boost"));
    let track = TrackRecord {
        props: vec![(prop_key("laps"), 5.0)],
        speed_pads: vec![vec![(prop_key("lit"), 1.0)], vec![(boost, 1.5)]],
        weapon_pads: vec![vec![], vec![], vec![]],
        triggers: vec![vec![], vec![(prop_key("gate"), 0.0)], vec![]],
    };
    let mut race = Race::<PadReader>::new()
        .with_craft(slot(4), racer(50.0))
        .with_track(&track);
    race.event(Event::SpeedPad {
        slot: slot(4),
        pad: 1,
    })
    .event(Event::WeaponPad {
        slot: slot(4),
        pad: 2,
    })
    .event(Event::TriggerEnter {
        slot: slot(4),
        trigger: 2,
    })
    .event(Event::TriggerExit {
        slot: slot(4),
        trigger: 1,
    });
    let commands = race.tick();
    assert_eq!(
        commands,
        [Command::ScalePad {
            pad: 1,
            factor: 2.0
        }]
    );
    let log = race.state();
    assert_eq!(
        (log.found, log.boost[1]),
        ([false, true], 1.5),
        "pad 0 has no boost, pad 1 has 1.5"
    );
    assert_eq!((log.speed, log.weapon, log.entered, log.left), (1, 2, 2, 1));
    assert_eq!(log.whole, 5.0);
}

mod armor;
