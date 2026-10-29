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
fn every_command() -> [Command; 9] {
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
        Command::PadMode { slot: slot(5) },
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
    ]
}

fn every_event() -> [Event; 4] {
    [
        Event::WallContact { slot: slot(0) },
        Event::PickupFire { slot: slot(1) },
        Event::PickupAbsorbed { slot: slot(2) },
        Event::PadEnergy { slot: slot(15) },
    ]
}

fn a_craft() -> Craft {
    Craft {
        pickup: Pickup::Weapon(Weapon::Disruptor),
        person: true,
        fire_held: false,
        racing: true,
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
        COMMAND_KIND as u32,
        COMMAND_SLOT as u32,
        COMMAND_ARG as u32,
        COMMAND_VALUE as u32,
        COMMAND_WORD as u32,
        CRAFT_PICKUP as u32,
        CRAFT_FLAGS as u32,
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
    let commands = every_command().into_iter().flat_map(|c| c.encode());
    let events = every_event().into_iter().flat_map(Event::encode);
    let craft = a_craft().encode();
    fnv(consts
        .into_iter()
        .chain(commands.chain(events).chain(craft).map(u32::from)))
}

/// The revision and the layout it names. Edit the layout and this fails: bump
/// [`REVISION`] and record the new fold beside it, in the same change.
const REVISION_AND_FOLD: (u32, u64) = (1, 0xf160_519e_04b4_b616);

#[test]
fn the_layout_is_the_revision_it_says() {
    let fold = layout_fold();
    assert_eq!(
        (REVISION, fold),
        REVISION_AND_FOLD,
        "the ABI 1 layout changed ({fold:#018x}): bump REVISION and record the fold"
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
    let mut far = Command::PadMode { slot: slot(0) }.encode();
    far[COMMAND_SLOT] = MAX_CRAFTS as u8;
    assert_eq!(Command::decode(&far), Err(Malformed::Slot(16)));
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
    let events: [Option<Event>; 4] =
        core::array::from_fn(|i| Sections::event_record(sections.head, i).and_then(Event::decode));
    assert_eq!(events, every_event().map(Some));
    assert_eq!(Sections::event_count(sections.head), 4);
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
