use super::*;

/// FNV-1a over little-endian words: no dependency, and every byte counts.
fn fnv(words: impl IntoIterator<Item = u32>) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for word in words {
        for byte in word.to_le_bytes() {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(0x0100_0000_01b3);
        }
    }
    hash
}

fn slot(index: usize) -> Slot {
    Slot::new(index).unwrap()
}

/// One of every command, with arguments that touch every field.
fn every_command() -> [Command; 16] {
    [
        Command::ScaleThrust {
            slot: slot(1),
            factor: 0.25,
        },
        Command::ShieldDelta {
            slot: slot(2),
            amount: -3.5,
        },
        Command::GivePickup { slot: slot(3) },
        Command::ClearPickup { slot: slot(4) },
        Command::PickupRoll {
            weapon: Weapon::Turbo,
            every: 2,
        },
        Command::ScalePad {
            pad: 300,
            factor: 1.5,
        },
        Command::ScaleSlotPads {
            slot: slot(5),
            factor: 0.0,
        },
        Command::PlayCue {
            slot: slot(6),
            cue: Cue::Absorb,
        },
        Command::FireEffect {
            slot: slot(7),
            effect: Effect::HitSpark,
        },
        Command::PickupTint {
            slot: slot(15),
            rgba: Rgba::new(0xE0, 0x20, 0x21, 0xFF),
        },
        Command::SetWeather {
            kind: WeatherKind::Rain,
            strength: 0.75,
        },
        Command::SetSun { intensity: 0.5 },
        Command::GradeSky { grey: 0.25 },
        Command::GiveShield { slot: slot(8) },
        Command::HoldShield { slot: slot(9) },
        Command::ClearShield { slot: slot(10) },
    ]
}

fn every_event() -> [Event; 8] {
    [
        Event::WallContact { slot: slot(0) },
        Event::PickupFire { slot: slot(1) },
        Event::PickupAbsorbed { slot: slot(2) },
        Event::SpeedPad {
            slot: slot(15),
            pad: 258,
        },
        Event::WeaponPad {
            slot: slot(3),
            pad: 7,
        },
        Event::TriggerEnter {
            slot: slot(4),
            trigger: 63,
        },
        Event::TriggerExit {
            slot: slot(5),
            trigger: 1,
        },
        Event::Hit {
            slot: slot(6),
            amount: 17.5,
            weapon: Weapon::Cannon,
        },
    ]
}

fn a_craft() -> Craft {
    Craft {
        pickup: Pickup::Weapon(Weapon::Disruptor),
        person: true,
        fire_held: false,
        racing: true,
        shielded: true,
        lap: 3,
        laps_target: 5,
        shield: 12.5,
        shield_max: 100.0,
        speed_kmh: 401.25,
    }
}

/// Every offset, size, cap, code and flag of the layout, and the bytes of one of
/// every record.
fn layout_fold() -> u64 {
    let consts = [
        ABI,
        IO_LEN as u32,
        MAX_SLOTS,
        MAX_EVENTS as u32,
        MAX_STATE_BYTES as u32,
        MAX_COMMANDS as u32,
        MAX_CRAFTS as u32,
        ABI_AT as u32,
        TICK_AT as u32,
        SLOTS_AT as u32,
        ACTIVE_AT as u32,
        EVENT_COUNT_AT as u32,
        STATE_LEN_AT as u32,
        EVENTS_AT as u32,
        EVENT_LEN as u32,
        STATE_AT as u32,
        REPLY_AT as u32,
        COMMAND_LEN as u32,
        CRAFTS_AT as u32,
        CRAFT_LEN as u32,
        EVENT_KIND as u32,
        EVENT_SLOT as u32,
        EVENT_INDEX as u32,
        MAX_PAD_FACTOR.to_bits(),
        track::MAX_ENTITIES as u32,
        track::MAX_TRIGGERS as u32,
        track::MAX_PROPS as u32,
        track::SPEED_PADS_AT as u32,
        track::WEAPON_PADS_AT as u32,
        track::TRIGGERS_AT as u32,
        track::ENTITIES_AT as u32,
        track::ENTITY_LEN as u32,
        track::PROPS_AT as u32,
        track::PROP_LEN as u32,
        track::TRACK_LEN as u32,
        track::ENTITY_KIND as u32,
        track::ENTITY_INDEX as u32,
        track::ENTITY_FIRST_PROP as u32,
        track::ENTITY_PROP_COUNT as u32,
        track::ENTITY_TO_WORLD as u32,
        track::ENTITY_MIN as u32,
        track::ENTITY_MAX as u32,
        u32::from(track::Kind::Track.code()),
        u32::from(track::Kind::SpeedPad.code()),
        u32::from(track::Kind::WeaponPad.code()),
        u32::from(track::Kind::Trigger.code()),
        track::prop_key("boost"),
        COMMAND_KIND as u32,
        COMMAND_SLOT as u32,
        COMMAND_ARG as u32,
        COMMAND_VALUE as u32,
        COMMAND_WORD as u32,
        CRAFT_PICKUP as u32,
        CRAFT_FLAGS as u32,
        CRAFT_LAP as u32,
        CRAFT_LAPS_TARGET as u32,
        CRAFT_SHIELD as u32,
        CRAFT_SHIELD_MAX as u32,
        CRAFT_SPEED as u32,
        u32::from(CRAFT_PERSON),
        u32::from(CRAFT_FIRE_HELD),
        u32::from(CRAFT_RACING),
        MAX_THRUST_FACTOR.to_bits(),
        MAX_ROLL_EVERY,
        Weapon::ALL.len() as u32,
        Cue::ALL.len() as u32,
        Effect::ALL.len() as u32,
        u32::from(Pickup::Ours.code()),
    ];
    // Each code by name: a reordered variant moves a byte's meaning on the wire.
    let codes = [
        Weapon::Rocket,
        Weapon::Missile,
        Weapon::Quake,
        Weapon::Cannon,
        Weapon::Turbo,
        Weapon::Shield,
        Weapon::Autopilot,
        Weapon::Plasma,
        Weapon::Bomb,
        Weapon::Mine,
        Weapon::LeachBeam,
        Weapon::Repulser,
        Weapon::Shuriken,
        Weapon::Disruptor,
    ]
    .map(|weapon| u32::from(weapon.code()))
    .into_iter()
    .chain(
        [
            Cue::SpeedupPad,
            Cue::Turbo,
            Cue::Collision,
            Cue::Absorb,
            Cue::Shield,
            Cue::ShieldActive,
        ]
        .map(|cue| u32::from(cue.code())),
    )
    .chain([Effect::CollisionSpark, Effect::HitSpark].map(|effect| u32::from(effect.code())))
    .chain([WeatherKind::Rain].map(|kind| u32::from(kind.code())));
    let commands = every_command().into_iter().flat_map(|c| c.encode());
    let events = every_event().into_iter().flat_map(Event::encode);
    let craft = a_craft().encode();
    fnv(consts
        .into_iter()
        .chain(codes)
        .chain(commands.chain(events).chain(craft).map(u32::from)))
}

/// The revision and the layout it names. The ABI changes in place before release (no
/// revision bump); the fold is recorded again with each layout edit.
const REVISION_AND_FOLD: (u32, u64) = (3, 0xb0fe_1175_4317_d134);

#[test]
fn the_layout_is_the_revision_it_says() {
    let fold = layout_fold();
    assert_eq!(
        (REVISION, fold),
        REVISION_AND_FOLD,
        "the ABI 1 layout changed ({fold:#018x}): record the new fold beside the revision"
    );
}

#[test]
fn the_abi_word_carries_major_and_revision_and_a_bare_major_has_none() {
    assert_eq!(split_abi_word(ABI_WORD), (ABI, Some(REVISION)));
    assert_eq!(split_abi_word(1), (1, None));
    assert_eq!(split_abi_word((1 << 8) | 7), (1, Some(7)));
}

#[test]
fn every_record_decodes_to_what_was_encoded() {
    for command in every_command() {
        assert_eq!(Command::decode(&command.encode()), Ok(command));
    }
    for event in every_event() {
        assert_eq!(Event::decode(&event.encode()), Some(event));
    }
    assert_eq!(Craft::decode(&a_craft().encode()), a_craft());
    for pickup in [Pickup::None, Pickup::Ours]
        .into_iter()
        .chain(Weapon::ALL.iter().copied().map(Pickup::Weapon))
    {
        assert_eq!(Pickup::from_code(pickup.code()), Some(pickup));
    }
}

#[test]
fn the_codes_are_the_ones_the_examples_were_built_against() {
    // The pre-revision examples wrote these numbers by hand.
    assert_eq!(Weapon::Turbo.code(), 5);
    assert_eq!(Pickup::Ours.code(), 255);
    assert_eq!(Cue::Turbo.code(), 1);
    assert_eq!(Cue::Absorb.code(), 3);
    assert_eq!(Effect::CollisionSpark.code(), 0);
    assert_eq!(Rgba::new(0xE0, 0x20, 0x20, 0xFF).to_u32(), 0xFF20_20E0);
    let thrust = Command::ScaleThrust {
        slot: slot(3),
        factor: 0.25,
    }
    .encode();
    assert_eq!(thrust[..4], [1, 3, 0, 0]);
    assert_eq!(thrust[4..8], 0.25_f32.to_le_bytes());
}

#[test]
fn a_record_breaking_its_own_rules_is_malformed() {
    let mut thrust = Command::ScaleThrust {
        slot: slot(0),
        factor: 1.0,
    }
    .encode();
    for bad in [2.5_f32, -0.1, f32::NAN] {
        thrust[COMMAND_VALUE..COMMAND_VALUE + 4].copy_from_slice(&bad.to_le_bytes());
        assert!(matches!(
            Command::decode(&thrust),
            Err(Malformed::Factor { .. })
        ));
    }
    let mut shield = Command::ShieldDelta {
        slot: slot(0),
        amount: 1.0,
    }
    .encode();
    shield[COMMAND_VALUE..COMMAND_VALUE + 4].copy_from_slice(&f32::INFINITY.to_le_bytes());
    assert!(matches!(
        Command::decode(&shield),
        Err(Malformed::Amount { .. })
    ));
    let mut far = Command::ScaleSlotPads {
        slot: slot(0),
        factor: 1.0,
    }
    .encode();
    far[COMMAND_SLOT] = MAX_CRAFTS as u8;
    assert_eq!(Command::decode(&far), Err(Malformed::Slot(16)));
    for bad in [2.5_f32, -0.1, f32::NAN] {
        for command in [
            Command::ScalePad {
                pad: 0,
                factor: bad,
            },
            Command::ScaleSlotPads {
                slot: slot(1),
                factor: bad,
            },
        ] {
            assert!(matches!(
                Command::decode(&command.encode()),
                Err(Malformed::PadFactor { .. })
            ));
        }
    }
    let mut pad = Command::ScalePad {
        pad: 0,
        factor: 1.0,
    }
    .encode();
    pad[COMMAND_SLOT] = 2;
    assert!(matches!(
        Command::decode(&pad),
        Err(Malformed::PadFactor { slot: 2, .. })
    ));
    let mut cue = Command::PlayCue {
        slot: slot(0),
        cue: Cue::Turbo,
    }
    .encode();
    cue[COMMAND_ARG] = 6;
    assert_eq!(Command::decode(&cue), Err(Malformed::Cue(6)));
    let mut roll = Command::PickupRoll {
        weapon: Weapon::Turbo,
        every: 2,
    }
    .encode();
    roll[COMMAND_WORD] = 0;
    assert!(matches!(
        Command::decode(&roll),
        Err(Malformed::Roll { .. })
    ));
    for bad in [-0.1, 1.5, f32::NAN, f32::INFINITY] {
        for command in [
            Command::SetWeather {
                kind: WeatherKind::Rain,
                strength: bad,
            },
            Command::SetSun { intensity: bad },
            Command::GradeSky { grey: bad },
        ] {
            assert!(matches!(
                Command::decode(&command.encode()),
                Err(Malformed::Unit { .. })
            ));
        }
    }
    let mut sun = Command::SetSun { intensity: 0.5 }.encode();
    sun[COMMAND_SLOT] = 1;
    assert!(matches!(Command::decode(&sun), Err(Malformed::Unit { .. })));
    let mut weather = Command::SetWeather {
        kind: WeatherKind::Rain,
        strength: 1.0,
    }
    .encode();
    weather[COMMAND_ARG] = 1;
    assert_eq!(Command::decode(&weather), Err(Malformed::Weather(1)));
    let mut unknown = [0; COMMAND_LEN];
    unknown[COMMAND_KIND] = 200;
    assert_eq!(Command::decode(&unknown), Err(Malformed::Kind(200)));
}

#[test]
fn the_observation_reads_back_through_the_guest_sections() {
    let mut io = [0; IO_LEN];
    let crafts = [Craft::default(), a_craft()];
    write_observation(
        &mut io,
        0x1_0000_0002,
        0b11,
        &every_event(),
        &crafts,
        &[9, 8, 7],
    );
    let sections = Sections::split(&mut io);
    assert_eq!(Sections::tick(sections.head), 0x1_0000_0002);
    assert_eq!(Sections::active(sections.head), 0b11);
    assert_eq!(Sections::state_len(sections.head), 3);
    assert_eq!(&sections.state[..3], &[9, 8, 7]);
    let events: [Option<Event>; 8] =
        core::array::from_fn(|i| Sections::event_record(sections.head, i).and_then(Event::decode));
    assert_eq!(events, every_event().map(Some));
    assert_eq!(Sections::event_count(sections.head), 8);
    let craft = Sections::craft_record(sections.crafts, slot(1)).map(Craft::decode);
    assert_eq!(craft, Some(a_craft()));
    Sections::put_command(sections.reply, 0, &every_command()[4]);
    Sections::put_count(sections.reply, 1);
    assert_eq!(reply_count(&io), Ok(1));
    assert_eq!(
        Command::decode(&command_record(&io, 0)),
        Ok(every_command()[4])
    );
}

#[test]
fn a_prop_key_is_fnv_1a_32_of_the_name() {
    assert_eq!(track::prop_key(""), 0x811c_9dc5);
    assert_eq!(track::prop_key("a"), 0xe40c_292c);
    assert_eq!(track::prop_key("foobar"), 0xbf9c_f968);
}

#[test]
fn a_track_region_reads_back_entity_by_entity() {
    use track::{Entity, Kind, Region, TRACK_LEN, prop_key, write};
    let boost = [(prop_key("boost"), 1.5), (prop_key("lit"), 1.0)];
    let gate = [(prop_key("gate"), 0.1)];
    let pad = |x: f32, props| Entity {
        to_world: core::array::from_fn(|i| if i == 12 { x } else { track::IDENTITY[i] }),
        min: [-1.0, -2.0, -3.0],
        max: [1.0, 8.0, 3.0],
        props,
    };
    let mut out = [0_u8; TRACK_LEN];
    let used = write(
        &mut out,
        &[(prop_key("laps"), 3.0)],
        &[pad(10.0, &[]), pad(20.0, &boost)],
        &[pad(30.0, &[])],
        &[pad(40.0, &gate)],
    )
    .unwrap();
    assert_eq!(used, track::PROPS_AT + 4 * track::PROP_LEN);
    let region = Region::new(&out);
    assert_eq!(
        [Kind::SpeedPad, Kind::WeaponPad, Kind::Trigger].map(|k| region.count(k)),
        [2, 1, 1]
    );
    let second = region.entity(Kind::SpeedPad, 1).unwrap();
    assert_eq!((second.kind(), second.index()), (Some(Kind::SpeedPad), 1));
    assert_eq!(second.to_world()[12], 20.0);
    assert_eq!(
        (second.min(), second.max()),
        ([-1.0, -2.0, -3.0], [1.0, 8.0, 3.0])
    );
    assert_eq!(second.prop(prop_key("boost")), Some(1.5));
    assert_eq!(
        region
            .entity(Kind::SpeedPad, 0)
            .unwrap()
            .prop(prop_key("boost")),
        None
    );
    assert!(region.entity(Kind::SpeedPad, 2).is_none());
    assert_eq!(
        region.entity(Kind::WeaponPad, 0).unwrap().to_world()[12],
        30.0
    );
    let trigger = region.entity(Kind::Trigger, 0).unwrap();
    assert_eq!(trigger.prop(prop_key("gate")), Some(0.1));
    assert_eq!(
        region
            .entity(Kind::Track, 0)
            .unwrap()
            .prop(prop_key("laps")),
        Some(3.0)
    );
    // A zeroed region is a track with nothing on it.
    let empty = Region::new(&[0; 8]);
    assert!(empty.entity(Kind::SpeedPad, 0).is_none());
    assert_eq!(empty.entity(Kind::Track, 0).map(|t| t.prop(1)), None);
}

#[test]
fn a_track_past_a_cap_or_with_a_key_twice_does_not_fit() {
    use track::{Entity, Kind, TRACK_LEN, TooBig, write};
    let mut out = [0_u8; TRACK_LEN];
    let none = Entity {
        to_world: track::IDENTITY,
        min: [0.0; 3],
        max: [0.0; 3],
        props: &[],
    };
    let many = [none; track::MAX_ENTITIES];
    assert_eq!(
        write(&mut out, &[], &many, &[], &[]),
        Err(TooBig::Entities(track::MAX_ENTITIES + 1))
    );
    assert_eq!(
        write(&mut out, &[], &[], &[], &many[..track::MAX_TRIGGERS + 1]),
        Err(TooBig::Triggers(track::MAX_TRIGGERS + 1))
    );
    let props = [(1_u32, 0.0_f32); track::MAX_PROPS + 1];
    assert_eq!(
        write(&mut out, &props, &[], &[], &[]),
        Err(TooBig::Props(track::MAX_PROPS + 1))
    );
    let twice = [(7, 1.0), (7, 2.0)];
    let clash = Entity {
        props: &twice,
        ..none
    };
    assert_eq!(
        write(&mut out, &[], &[none], &[clash], &[]),
        Err(TooBig::Clash {
            kind: Kind::WeaponPad,
            index: 0,
            key: 7
        })
    );
}
