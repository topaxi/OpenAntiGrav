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
    let mut reply = Reply::new(&mut buffer, 2, 1);
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
    assert_eq!(
        reply.prefer_route(&held, 1),
        Err(Refused::Range),
        "the track has one route"
    );
    assert_eq!(reply.prefer_route(&held, 0), Ok(()));
    assert_eq!(reply.prefer_route(&held, 0), Err(Refused::Repeated));
    assert_eq!(reply.len(), 6);
    while reply.len() < MAX_COMMANDS_FOR_TEST {
        reply
            .hud_tint(&held, oag_ext_abi::HudElement::Lap, Rgba::default())
            .unwrap();
    }
    assert_eq!(reply.play_cue(&held, Cue::Turbo), Err(Refused::Full));
    reply.finish();
    assert_eq!(buffer[..4], (MAX_COMMANDS_FOR_TEST as u32).to_le_bytes());
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
        ..TrackRecord::default()
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

crate::blob! {
    #[derive(Debug, Default)]
    struct NoState {
        spare: u8,
    }
}

/// Puts a lap time and a sprite on every craft's HUD, then floods one craft past the
/// cap.
struct HudDrawer;

impl Extension for HudDrawer {
    type State = NoState;

    fn tick(obs: &Observation<'_>, _: &mut NoState, reply: &mut Reply<'_>) {
        for craft in obs.crafts() {
            let place = HudPlace {
                anchor: Anchor::BottomRight,
                quarters: 4,
                x: -8,
                y: -6,
            };
            let mut line = Line::new();
            line.time(3725).unwrap();
            reply
                .hud_text(&craft, place, Rgba::new(255, 255, 255, 255), line.as_str())
                .unwrap();
            reply
                .hud_icon(&craft, place, Rgba::default(), "TimeDiffIcon")
                .unwrap();
        }
    }
}

#[test]
fn a_guest_draws_text_and_an_icon_and_the_cap_refuses_by_name() {
    let mut race = Race::<HudDrawer>::new()
        .with_craft(slot(0), racer(10.0))
        .with_craft(slot(2), racer(10.0));
    let commands = race.tick();
    assert_eq!(commands.len(), 4);
    assert!(matches!(
        commands[0],
        Command::HudText { slot: s, text, place, .. }
            if s == slot(0) && text.as_str() == "1:02.0" && place.anchor == Anchor::BottomRight
    ));
    assert!(matches!(
        commands[3],
        Command::HudIcon { slot: s, sprite, .. } if s == slot(2) && sprite.as_str() == "TimeDiffIcon"
    ));

    let mut buffer = [0_u8; oag_ext_abi::CRAFTS_AT - oag_ext_abi::REPLY_AT];
    let mut reply = Reply::new(&mut buffer, 0, 0);
    let held = Craft {
        slot: slot(1),
        record: &[0; oag_ext_abi::CRAFT_LEN],
    };
    let place = HudPlace {
        anchor: Anchor::Top,
        quarters: 4,
        x: 0,
        y: 0,
    };
    for _ in 0..MAX_HUD_ITEMS {
        reply.hud_text(&held, place, Rgba::default(), "x").unwrap();
    }
    assert_eq!(
        reply.hud_text(&held, place, Rgba::default(), "x"),
        Err(Refused::HudItems)
    );
    let long = "x".repeat(MAX_HUD_TEXT + 1);
    assert_eq!(
        reply.hud_text(&held, place, Rgba::default(), &long),
        Err(Refused::Range)
    );
    let far = HudPlace { x: 2048, ..place };
    assert_eq!(
        reply.hud_icon(&held, far, Rgba::default(), "x"),
        Err(Refused::Range)
    );
}

/// Names the mod's own string and icon.
struct HudNamer;

impl Extension for HudNamer {
    type State = NoState;

    fn tick(obs: &Observation<'_>, _: &mut NoState, reply: &mut Reply<'_>) {
        for craft in obs.crafts() {
            let place = HudPlace {
                anchor: Anchor::Top,
                quarters: 4,
                x: 0,
                y: 0,
            };
            reply
                .hud_string(&craft, place, Rgba::default(), "boost")
                .unwrap();
            reply
                .hud_mod_icon(&craft, place, Rgba::default(), "badge")
                .unwrap();
        }
    }
}

#[test]
fn a_guest_names_its_own_string_and_icon_by_key() {
    let mut race = Race::<HudNamer>::new().with_craft(slot(0), racer(10.0));
    let commands = race.tick();
    assert!(matches!(
        commands[0],
        Command::HudText { text, own: true, .. } if text.as_str() == "boost"
    ));
    assert!(matches!(
        commands[1],
        Command::HudIcon { sprite, own: true, .. } if sprite.as_str() == "badge"
    ));
}

mod armor;

#[test]
fn a_guest_finds_a_route_by_a_property_of_a_path_it_drives() {
    let pit = crate::prop!("pit");
    let track = crate::test::TrackRecord {
        paths: vec![vec![], vec![(pit, 1.0)], vec![]],
        routes: vec![vec![(pit, 1.0)]],
        ..crate::test::TrackRecord::default()
    };
    let region = track.region();
    let mut io = Box::new([0_u8; IO_LEN]);
    oag_ext_abi::write_observation(&mut io, 0, 0b1, &[], &[racer(1.0)], &[]);
    let sections = Sections::split(&mut io);
    let obs = Observation::read(sections.head, sections.crafts, &region);
    let track = obs.track();
    assert_eq!((track.paths(), track.routes()), (3, 1));
    assert_eq!(track.path(1).and_then(|p| p.prop(pit)), Some(1.0));
    assert_eq!(track.path(0).and_then(|p| p.prop(pit)), None);
    let found =
        (0..track.routes() as u16).find(|&r| track.route(r).and_then(|e| e.prop(pit)).is_some());
    assert_eq!(found, Some(0));
    assert!(track.route(1).is_none());
}
#[test]
fn a_pickup_look_is_one_record_and_a_mod_icon_rides_the_pool() {
    let mut buffer = [0_u8; 4 + oag_ext_abi::COMMAND_LEN * oag_ext_abi::MAX_COMMANDS];
    let mut reply = Reply::new(&mut buffer, 0, 0);
    let record = CraftRecord {
        pickup: Pickup::Ours,
        ..racer(50.0)
    }
    .encode();
    let held = Craft {
        slot: slot(2),
        record: &record,
    };
    let red = Colour::Rgba(Rgba::new(0xE0, 0x20, 0x20, 0x80));
    assert_eq!(reply.pickup_look(&held, icon::title::TURBO, red), Ok(()));
    let bolt = Icon::Mod(ModIcon::new("bolt").unwrap());
    assert_eq!(
        reply.pickup_look(&held, bolt, colour::title::SHIELD),
        Ok(())
    );
    assert_eq!(
        reply.hud_tint(&held, HudElement::Pickup, Rgba::default()),
        Err(Refused::PickupTint)
    );
    assert_eq!(reply.len(), 2);
}

crate::blob! {
    #[derive(Debug, Clone, Copy, PartialEq, Default)]
    struct Heard {
        from: u32,
        channel: u32,
        slot: u8,
        value: f32,
        count: u8,
    }
}

struct Pinger;

impl Extension for Pinger {
    type State = Heard;
    fn tick(obs: &Observation<'_>, _: &mut Heard, reply: &mut Reply<'_>) {
        if let Some(craft) = obs.craft(slot(2)) {
            reply
                .emit(&craft, crate::channel!("oag.test-a/ping"), 7.5)
                .unwrap();
        }
    }
}

struct Listener;

impl Extension for Listener {
    type State = Heard;
    fn tick(obs: &Observation<'_>, state: &mut Heard, _: &mut Reply<'_>) {
        for event in obs.events() {
            if let Event::Mod {
                slot,
                from,
                channel,
                value,
            } = event
            {
                *state = Heard {
                    from,
                    channel,
                    slot: slot.index() as u8,
                    value,
                    count: state.count + 1,
                };
            }
        }
    }
}

#[test]
fn an_emit_reaches_a_listener_as_a_mod_event_with_everything_intact() {
    let mut a = Race::<Pinger>::new().with_craft(slot(2), racer(1.0));
    let mut b = Race::<Listener>::new().with_craft(slot(2), racer(1.0));
    let sent = a.tick();
    assert_eq!(
        sent,
        [Command::Emit {
            slot: slot(2),
            channel: crate::channel_key("oag.test-a/ping"),
            value: 7.5
        }]
    );
    b.hear(crate::channel!("oag.test-a"), &sent[0]);
    b.tick();
    assert_eq!(
        b.state(),
        Heard {
            from: crate::channel_key("oag.test-a"),
            channel: crate::channel_key("oag.test-a/ping"),
            slot: 2,
            value: 7.5,
            count: 1
        }
    );
}

#[test]
fn the_reply_refuses_an_emit_past_the_cap_and_a_non_finite_value() {
    let mut io = [0_u8; oag_ext_abi::IO_LEN];
    let mut reply = Reply::new(&mut io[oag_ext_abi::REPLY_AT..], 0, 0);
    let record = racer(1.0).encode();
    let craft = Craft {
        slot: slot(0),
        record: &record,
    };
    for _ in 0..crate::MAX_EMITS_PER_TICK {
        assert_eq!(reply.emit(&craft, 1, 0.0), Ok(()));
    }
    assert_eq!(reply.emit(&craft, 1, 0.0), Err(Refused::Emits));
    let mut other = Reply::new(&mut io[oag_ext_abi::REPLY_AT..], 0, 0);
    assert_eq!(other.emit(&craft, 1, f32::NAN), Err(Refused::Range));
}

#[test]
fn the_reply_refuses_a_fire_effect_past_the_cap_counting_both_kinds() {
    let mut io = [0_u8; oag_ext_abi::IO_LEN];
    let mut reply = Reply::new(&mut io[oag_ext_abi::REPLY_AT..], 0, 0);
    let record = racer(1.0).encode();
    let craft = Craft {
        slot: slot(0),
        record: &record,
    };
    for i in 0..oag_ext_abi::MAX_FIRE_EFFECTS_PER_TICK {
        let sent = if i % 2 == 0 {
            reply.fire_effect(&craft, oag_ext_abi::Effect::HitSpark)
        } else {
            reply.fire_mod_effect(&craft, 0)
        };
        assert_eq!(sent, Ok(()));
    }
    assert_eq!(
        reply.fire_effect(&craft, oag_ext_abi::Effect::HitSpark),
        Err(Refused::Effects)
    );
    assert_eq!(reply.fire_mod_effect(&craft, 0), Err(Refused::Effects));
}
