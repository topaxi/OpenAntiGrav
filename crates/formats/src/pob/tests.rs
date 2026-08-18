//! What the `.pob` particle-system parser in [`super`] is asserted to do: the
//! `SYSP` container, the emitter tree inside it, and the fields each emitter
//! carries.
//!
//! Its own file rather than a `#[cfg(test)]` block at the end of
//! `pob.rs`: the tests are 445 lines, well past the 200 an inline test
//! module may hold. See `scripts/check-file-size.py`, which is the rule as a
//! gate.

use super::*;

/// Builds a particle system blob by hand. No game data in any test.
fn pob(name: &str, slots: &[Option<u32>], payload: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&0u32.to_le_bytes()); // patched below
    out.extend_from_slice(&(slots.len() as u16).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes());
    for slot in slots {
        out.extend_from_slice(&slot.unwrap_or(EMPTY_SLOT).to_le_bytes());
    }
    let mut name_field = [0u8; NAME_LEN];
    let bytes = name.as_bytes();
    let take = bytes.len().min(NAME_LEN - 1);
    name_field[..take].copy_from_slice(&bytes[..take]);
    out.extend_from_slice(&name_field);
    out.extend_from_slice(payload);

    let declared = (HEADER_LEN + payload.len()) as u32;
    out[0x04..0x08].copy_from_slice(&declared.to_le_bytes());
    out
}

#[test]
fn a_particle_system_parses_and_its_name_and_payload_survive() {
    let data = pob(
        "WO_TEST_SPARK",
        &[Some(1220), None, Some(2508)],
        &[1, 2, 3, 4, 5],
    );
    let parsed = ParticleSystem::parse(&data).expect("parse");
    assert_eq!(parsed.name, "WO_TEST_SPARK");
    assert_eq!(parsed.slots, vec![Some(1220), None, Some(2508)]);
    assert_eq!(parsed.payload, &[1, 2, 3, 4, 5]);
    assert!(looks_like_particle_system(&data));
}

#[test]
fn a_name_filling_the_field_is_not_null_terminated() {
    let mut data = pob(&"A".repeat(NAME_LEN - 1), &[], &[]);
    // The builder always leaves a NUL at the field's last byte for a name
    // of this length; overwrite it so the field truly has none.
    let last = data.len() - 1;
    data[last] = b'A';
    assert_eq!(ParticleSystem::parse(&data), Err(Error::NameNotTerminated));
}

#[test]
fn an_empty_slot_table_round_trips_with_no_particle_data() {
    let data = pob("WO_EMPTY", &[], &[]);
    let parsed = ParticleSystem::parse(&data).expect("parse");
    assert!(parsed.slots.is_empty());
    assert!(parsed.payload.is_empty());
}

#[test]
fn a_size_disagreement_is_refused() {
    let mut data = pob("WO_TEST", &[Some(1)], &[0; 10]);
    data[0x04..0x08].copy_from_slice(&999u32.to_le_bytes());
    assert!(matches!(
        ParticleSystem::parse(&data),
        Err(Error::SizeDisagreement { .. })
    ));
}

#[test]
fn an_unexpected_header_word_is_refused() {
    let mut data = pob("WO_TEST", &[], &[]);
    data[0x0a..0x0c].copy_from_slice(&2u16.to_le_bytes());
    assert_eq!(
        ParticleSystem::parse(&data),
        Err(Error::UnexpectedHeaderWord {
            field: "+0x0a",
            value: 2
        })
    );
}

#[test]
fn a_truncated_table_is_refused() {
    let data = pob("WO_TEST", &[Some(1), Some(2), Some(3)], &[]);
    assert!(matches!(
        ParticleSystem::parse(&data[..data.len() - 40]),
        Err(Error::TableOutOfRange { .. })
    ));
}

#[test]
fn a_blob_without_the_magic_is_refused() {
    let mut data = pob("WO_TEST", &[], &[]);
    data[0] = b'X';
    assert_eq!(ParticleSystem::parse(&data), Err(Error::NotSysp));
    assert!(!looks_like_particle_system(&data));
}

/// A one-slot blob whose fixup site sits at the very start of `payload`
/// and whose baked value (`40`) resolves 8 bytes further in, where a
/// `MARK` marker sits - the same two-hop shape a live PPSSPP trace
/// confirmed for `WO_SHIP_COLL_SPARK_DAMAGE`'s 26 real slots.
fn pob_with_one_fixup() -> Vec<u8> {
    let mut payload = vec![0u8; 12];
    payload[0..4].copy_from_slice(&40u32.to_le_bytes());
    payload[8..12].copy_from_slice(b"MARK");
    pob("WO_TEST_FIXUP", &[Some(32)], &payload)
}

#[test]
fn a_slot_resolves_through_its_fixup_site_to_the_target() {
    let data = pob_with_one_fixup();
    let parsed = ParticleSystem::parse(&data).expect("parse");
    let resolved = parsed.resolve_slot(&data, 0).expect("resolve");
    assert_eq!(resolved, Some(8));
    assert_eq!(&parsed.payload[8..12], b"MARK");
}

#[test]
fn an_unused_slot_resolves_to_none() {
    let data = pob("WO_TEST", &[None], &[]);
    let parsed = ParticleSystem::parse(&data).expect("parse");
    assert_eq!(parsed.resolve_slot(&data, 0), Ok(None));
}

#[test]
fn an_index_past_the_table_resolves_to_none() {
    let data = pob_with_one_fixup();
    let parsed = ParticleSystem::parse(&data).expect("parse");
    assert_eq!(parsed.resolve_slot(&data, 5), Ok(None));
}

#[test]
fn a_fixup_site_past_the_end_is_refused() {
    let data = pob_with_one_fixup();
    let parsed = ParticleSystem::parse(&data).expect("parse");
    // The fixup site is at table_end(20) + slot(32) = 52; cut well before it.
    let short = &data[..40];
    assert_eq!(
        parsed.resolve_slot(short, 0),
        Err(Error::SlotOutOfRange { index: 0 })
    );
}

/// Builds a `SYSP` blob around a hand-laid resource image, whose first
/// 32 bytes are the name field the container wants *and* the root
/// emitter record's name. No game data in any test.
fn pob_from_resource(slots: &[Option<u32>], resource: &[u8]) -> Vec<u8> {
    assert!(resource.len() >= NAME_LEN);
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&0u32.to_le_bytes()); // patched below
    out.extend_from_slice(&(slots.len() as u16).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes());
    for slot in slots {
        out.extend_from_slice(&slot.unwrap_or(EMPTY_SLOT).to_le_bytes());
    }
    out.extend_from_slice(resource);
    let declared = (HEADER_LEN + resource.len() - NAME_LEN) as u32;
    out[0x04..0x08].copy_from_slice(&declared.to_le_bytes());
    out
}

/// A minimal emitter record: a name, and whatever fields a test writes
/// over it. Everything else is zero, which every field tolerates.
struct Record(Vec<u8>);

impl Record {
    fn new(name: &str) -> Self {
        let mut bytes = vec![0u8; EMITTER_LEN];
        let take = name.len().min(NAME_LEN - 1);
        bytes[..take].copy_from_slice(&name.as_bytes()[..take]);
        Self(bytes)
    }

    fn u32(mut self, at: usize, value: u32) -> Self {
        self.0[at..at + 4].copy_from_slice(&value.to_le_bytes());
        self
    }

    fn i32(self, at: usize, value: i32) -> Self {
        self.u32(at, value as u32)
    }

    fn f32(self, at: usize, value: f32) -> Self {
        self.u32(at, value.to_bits())
    }

    /// The same record, written the way a PS3 build writes it.
    ///
    /// Every field past the 32-byte name is a word, and the name is bytes, so
    /// reversing each four-byte group from [`NAME_LEN`] on is exactly what the
    /// big-endian exporter emits - including the palette, which is `u32`s.
    fn big_endian(mut self) -> Self {
        for word in self.0[NAME_LEN..].chunks_exact_mut(4) {
            word.reverse();
        }
        self
    }

    /// A channel block: mode, bounds, and `(time, value)` keys.
    fn channel(mut self, block: usize, mode: u32, lo: f32, hi: f32, keys: &[(f32, f32)]) -> Self {
        self = self
            .u32(block + 0x04, mode)
            .i32(block + 0x08, keys.len() as i32)
            .f32(block + 0x0c, lo)
            .f32(block + 0x10, hi);
        for (i, &(time, value)) in keys.iter().enumerate() {
            self = self
                .f32(block + 0x14 + i * 8, time)
                .f32(block + 0x18 + i * 8, value);
        }
        self
    }
}

/// Lays records at their own offsets in one resource image, padding
/// between them.
fn resource(records: &[(usize, Record)]) -> Vec<u8> {
    let end = records
        .iter()
        .map(|(offset, _)| offset + EMITTER_LEN)
        .max()
        .unwrap_or(NAME_LEN);
    let mut image = vec![0u8; end];
    for (offset, record) in records {
        image[*offset..*offset + EMITTER_LEN].copy_from_slice(&record.0);
    }
    image
}

/// The shape of every real file: a root whose `+0x94c` chains siblings,
/// each a full record of the same layout at its own offset.
#[test]
fn a_sibling_chain_parses_root_first_with_every_field() {
    const SECOND: usize = 0x1000;
    let image = resource(&[
        (
            0,
            Record::new("WO_TEST_ROOT")
                .u32(0x20, flags::GRAVITY | flags::STREAK_FROM_SPAWN)
                .f32(0x24, 32.0)
                .u32(0x30, 7)
                .f32(0x34, 0.5)
                .u32(0x44, 1)
                .f32(0x48, 1.5)
                .f32(0x4c, 0.25)
                .f32(0x50, 0.75)
                .f32(0x58, 30.0)
                .i32(0x5c, 16)
                .i32(0x60, 6)
                .i32(0x64, 4)
                .i32(0x68, 4)
                .i32(0x6c, 3)
                .i32(0x70, 3)
                .f32(0x74, -0.015)
                .i32(0xa0, 64)
                .u32(0xb8, 5)
                .u32(0xbc, 2)
                .u32(0xc0, 2)
                .u32(0xc4, u32::from_le_bytes([1, 2, 3, 4]))
                .channel(0x4d8, 0, 0.5, 2.5, &[(0.0, 0.0), (1.0, 1.0)])
                .channel(0x5b8, 2, 0.0, 200.0, &[])
                .i32(0x9ac, 1)
                .u32(0x94c, SECOND as u32),
        ),
        (SECOND, Record::new("bits").f32(0x24, 4.0).u32(0xb8, 2)),
    ]);
    let data = pob_from_resource(&[None, None], &image);
    let parsed = ParticleSystem::parse(&data).expect("parse");
    assert_eq!(parsed.name, "WO_TEST_ROOT");

    let emitters = parsed.emitters(&data).expect("emitters");
    assert_eq!(emitters.len(), 2);

    let root = &emitters[0];
    assert_eq!(root.name, "WO_TEST_ROOT");
    assert_eq!(root.offset, 0);
    assert!(root.gravity_enabled());
    assert_eq!(
        root.flags & flags::STREAK_FROM_SPAWN,
        flags::STREAK_FROM_SPAWN
    );
    assert_eq!(root.duration_ticks, 32.0);
    assert_eq!(root.shape, 7);
    assert_eq!(root.extent, 0.5);
    assert_eq!(root.velocity_mode, 1);
    assert_eq!(root.speed_per_tick, (1.5, 0.25));
    assert_eq!(root.elevation, 0.75);
    assert_eq!(root.cone_degrees, 30.0);
    assert_eq!(root.lifetime_ticks, (16, 6));
    assert_eq!(root.interval_ticks, (4, 4));
    assert_eq!(root.per_emission, (3, 3));
    assert_eq!(root.gravity_per_tick2, -0.015);
    assert_eq!(root.live_cap, 64);
    assert_eq!(root.draw_class(), Some(6));
    assert_eq!(root.colour_mode, 2);
    assert_eq!(root.blend_class, 2);
    assert_eq!(root.colours[0], [1, 2, 3, 4]);
    assert_eq!(root.size.mode, ChannelMode::Keyframed);
    assert_eq!(root.size.scaled_at(0.0), 0.5);
    assert_eq!(root.size.scaled_at(1.0), 2.5);
    assert_eq!(root.alpha.mode, ChannelMode::Constant);
    assert_eq!(root.alpha.hi, 200.0);
    assert!(root.death_child.is_none());
    assert!(root.particle_child.is_none());

    assert_eq!(emitters[1].name, "bits");
    assert_eq!(emitters[1].offset, SECOND);
    assert_eq!(emitters[1].draw_class(), Some(3));
    assert!(!emitters[1].gravity_enabled());
}

/// `WO_ROCKET_EXPLO`'s shape: a root whose *particles* each carry a
/// child system, which is why the walk cannot be a flat sibling list.
#[test]
fn a_per_particle_child_is_reachable_from_its_parent() {
    const CHILD: usize = 0x1000;
    const SIBLING: usize = 0x2000;
    let image = resource(&[
        (
            0,
            Record::new("WO_TEST_EXPLO")
                .u32(0x948, CHILD as u32)
                .u32(0x94c, SIBLING as u32),
        ),
        (CHILD, Record::new("SMOKEMUSHROOM").f32(0x4d4, 0.25)),
        (SIBLING, Record::new("DEBRIS")),
    ]);
    let data = pob_from_resource(&[], &image);
    let parsed = ParticleSystem::parse(&data).expect("parse");
    let emitters = parsed.emitters(&data).expect("emitters");

    assert_eq!(emitters.len(), 3);
    let names: Vec<&str> = emitters.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["WO_TEST_EXPLO", "SMOKEMUSHROOM", "DEBRIS"]);
    assert_eq!(emitters[0].particle_child, Some(1));
    assert_eq!(emitters[1].child_spawn_probability, 0.25);
    // The sibling is a peer, not a child.
    assert!(emitters[0].death_child.is_none());
    assert!(emitters[2].particle_child.is_none());
}

/// The type-3 modifier node is where per-axis drag comes from; a list
/// with no type-3 node means no drag at all, not drag of zero.
#[test]
fn the_drag_modifier_is_read_off_the_node_list() {
    const NODE: usize = 0x1000;
    let mut image = resource(&[(0, Record::new("WO_TEST").u32(0x9b4, NODE as u32))]);
    image.resize(NODE + 0x34, 0);
    image[NODE..NODE + 4].copy_from_slice(&0.98f32.to_le_bytes());
    image[NODE + 4..NODE + 8].copy_from_slice(&0.85f32.to_le_bytes());
    image[NODE + 8..NODE + 12].copy_from_slice(&0.5f32.to_le_bytes());
    image[NODE + 0x24..NODE + 0x28].copy_from_slice(&MODIFIER_DRAG.to_le_bytes());

    let data = pob_from_resource(&[], &image);
    let parsed = ParticleSystem::parse(&data).expect("parse");
    let emitters = parsed.emitters(&data).expect("emitters");
    assert_eq!(emitters[0].drag_per_tick(), Some([0.98, 0.85, 0.5]));

    let plain = pob_from_resource(&[], &resource(&[(0, Record::new("WO_TEST"))]));
    let parsed = ParticleSystem::parse(&plain).expect("parse");
    assert_eq!(
        parsed.emitters(&plain).expect("emitters")[0].drag_per_tick(),
        None
    );
}

#[test]
fn a_keyframed_channel_interpolates_and_clamps() {
    let channel = Channel {
        period: 0.0,
        mode: ChannelMode::Keyframed,
        lo: 0.0,
        hi: 200.0,
        keys: vec![(0.0, 1.0), (0.5, 1.0), (1.0, 0.0)],
    };
    assert_eq!(channel.scaled_at(-1.0), 200.0);
    assert_eq!(channel.scaled_at(0.5), 200.0);
    assert_eq!(channel.scaled_at(0.75), 100.0);
    assert_eq!(channel.scaled_at(2.0), 0.0);
}

/// A tree pointer back at an already-visited record would otherwise walk
/// forever off file bytes.
#[test]
fn a_cyclic_tree_is_refused() {
    const SECOND: usize = 0x1000;
    let image = resource(&[
        (0, Record::new("WO_TEST").u32(0x94c, SECOND as u32)),
        (SECOND, Record::new("loop").u32(0x94c, 0)),
    ]);
    // `+0x94c == 0` is read as "no sibling", so point it at itself
    // instead - the same revisit, expressible in the file.
    let mut image = image;
    image[SECOND + 0x94c..SECOND + 0x950].copy_from_slice(&(SECOND as u32).to_le_bytes());
    let data = pob_from_resource(&[], &image);
    let parsed = ParticleSystem::parse(&data).expect("parse");
    assert_eq!(parsed.emitters(&data), Err(Error::EmitterTreeUnbounded));
}

#[test]
fn a_child_past_the_end_is_refused() {
    let image = resource(&[(0, Record::new("WO_TEST").u32(0x94c, 0x9_0000))]);
    let data = pob_from_resource(&[], &image);
    let parsed = ParticleSystem::parse(&data).expect("parse");
    assert_eq!(
        parsed.emitters(&data),
        Err(Error::EmitterOutOfRange { offset: 0x9_0000 })
    );
}

#[test]
fn a_channel_declaring_more_keys_than_fit_is_refused() {
    let image = resource(&[(0, Record::new("WO_TEST").i32(0x4d8 + 0x08, 999))]);
    let data = pob_from_resource(&[], &image);
    let parsed = ParticleSystem::parse(&data).expect("parse");
    assert_eq!(
        parsed.emitters(&data),
        Err(Error::ChannelKeyCount {
            block: 0x4d8,
            count: 999
        })
    );
}

/// The blend table holds eight entries; an index past it has no draw
/// handler, so it must be refusable rather than drawn as something else.
#[test]
fn a_render_mode_past_the_blend_table_has_no_draw_class() {
    let image = resource(&[(0, Record::new("WO_TEST").u32(0xb8, 9))]);
    let data = pob_from_resource(&[], &image);
    let parsed = ParticleSystem::parse(&data).expect("parse");
    assert_eq!(
        parsed.emitters(&data).expect("emitters")[0].draw_class(),
        None
    );
}

#[test]
fn a_target_landing_before_the_payload_is_refused() {
    let mut payload = vec![0u8; 12];
    payload[0..4].copy_from_slice(&0u32.to_le_bytes()); // baked=0 -> target==table_end
    let data = pob("WO_TEST", &[Some(32)], &payload);
    let parsed = ParticleSystem::parse(&data).expect("parse");
    assert_eq!(
        parsed.resolve_slot(&data, 0),
        Err(Error::SlotOutOfRange { index: 0 })
    );
}

/// The container header and slot table, rewritten big-endian around a resource
/// image the caller has already laid out for the same order (see
/// [`Record::big_endian`]). The image itself is not touched: its name fields
/// are byte strings and reversing them would be the very bug
/// [`crate::byte_order`] exists to prevent.
fn big_endian_container(mut blob: Vec<u8>) -> Vec<u8> {
    let slots = usize::from(ByteOrder::Little.u16(&blob, 0x08));
    blob[0x00..0x04].copy_from_slice(MAGIC_BE);
    blob[0x04..0x08].reverse();
    blob[0x08..0x0a].reverse();
    blob[0x0a..0x0c].reverse();
    blob[0x0c..0x10].reverse();
    for i in 0..slots {
        let at = HEADER_LEN + i * SLOT_LEN;
        blob[at..at + SLOT_LEN].reverse();
    }
    blob
}

/// One `.pob` carrying every kind of field the parser reads - flags, floats,
/// signed counts, a palette entry, two channels with keys, a modifier node and
/// a sibling pointer - laid out for `order`.
fn twin(order: ByteOrder) -> Vec<u8> {
    const SECOND: usize = 0x1000;
    const NODE: usize = 0x2000;
    let dress = |record: Record| match order {
        ByteOrder::Big => record.big_endian(),
        ByteOrder::Little => record,
    };
    let mut image = resource(&[
        (
            0,
            dress(
                Record::new("WO_TWIN_ROOT")
                    .u32(0x20, flags::GRAVITY | flags::LOOPING)
                    .f32(0x24, 32.0)
                    .u32(0x30, 7)
                    .f32(0x34, 0.5)
                    .f32(0x48, 1.5)
                    .i32(0x5c, 16)
                    .i32(0x60, -6)
                    .f32(0x74, -0.015)
                    .i32(0xa0, 64)
                    .u32(0xb8, 5)
                    .u32(0xc4, u32::from_le_bytes([1, 2, 3, 4]))
                    .channel(0x4d8, 0, 0.5, 2.5, &[(0.0, 0.0), (1.0, 1.0)])
                    .channel(0x5b8, 2, 0.0, 200.0, &[])
                    .i32(0x9ac, 8)
                    .u32(0x9b4, NODE as u32)
                    .u32(0x94c, SECOND as u32),
            ),
        ),
        (
            SECOND,
            dress(Record::new("bits").f32(0x24, 4.0).u32(0xb8, 2)),
        ),
    ]);
    image.resize(NODE + 0x34, 0);
    let mut put = |at: usize, bits: u32| {
        let bytes = match order {
            ByteOrder::Big => bits.to_be_bytes(),
            ByteOrder::Little => bits.to_le_bytes(),
        };
        image[at..at + 4].copy_from_slice(&bytes);
    };
    put(NODE, 0.98f32.to_bits());
    put(NODE + 4, 0.85f32.to_bits());
    put(NODE + 8, 0.5f32.to_bits());
    put(NODE + 0x24, MODIFIER_DRAG);
    // The atlas grid is two `u16` fields, not one `u32`: on the disc a
    // big-endian record holds `00 08 00 04` for an 8x4 grid, so a four-byte
    // reversal would read it back as 4x8. Written here field by field.
    let mut put16 = |at: usize, value: u16| {
        let bytes = match order {
            ByteOrder::Big => value.to_be_bytes(),
            ByteOrder::Little => value.to_le_bytes(),
        };
        image[at..at + 2].copy_from_slice(&bytes);
    };
    put16(0x9a0, 4);
    put16(0x9a2, 2);

    let blob = pob_from_resource(&[None, Some(0x1a4)], &image);
    match order {
        ByteOrder::Big => big_endian_container(blob),
        ByteOrder::Little => blob,
    }
}

/// The whole claim about Wipeout HD/Fury in one assertion: the PS3 build is
/// the same container with the words the other way round, so the same authored
/// content must parse to the same records either way.
#[test]
fn a_big_endian_blob_parses_to_exactly_what_its_little_endian_twin_does() {
    let little = twin(ByteOrder::Little);
    let big = twin(ByteOrder::Big);
    assert_ne!(little, big, "the two blobs must not be identical bytes");

    let (le, be) = (
        ParticleSystem::parse(&little).expect("little"),
        ParticleSystem::parse(&big).expect("big"),
    );
    assert_eq!(le.order, ByteOrder::Little);
    assert_eq!(be.order, ByteOrder::Big);
    assert_eq!(le.name, be.name);
    assert_eq!(le.slots, be.slots);
    assert_eq!(
        le.emitters(&little).expect("little emitters"),
        be.emitters(&big).expect("big emitters")
    );
}

/// Every field the twin sets, read off the big-endian blob alone - so the test
/// above cannot pass by both sides being wrong in the same way.
#[test]
fn the_big_endian_blob_carries_the_authored_values() {
    let data = twin(ByteOrder::Big);
    let parsed = ParticleSystem::parse(&data).expect("parse");
    assert_eq!(parsed.name, "WO_TWIN_ROOT");
    assert_eq!(parsed.slots, vec![None, Some(0x1a4)]);

    let emitters = parsed.emitters(&data).expect("emitters");
    assert_eq!(emitters.len(), 2);
    let root = &emitters[0];
    assert!(root.gravity_enabled());
    assert!(root.looping());
    assert_eq!(root.duration_ticks, 32.0);
    assert_eq!(root.shape, 7);
    assert_eq!(root.extent, 0.5);
    assert_eq!(root.speed_per_tick.0, 1.5);
    assert_eq!(root.lifetime_ticks, (16, -6));
    assert_eq!(root.gravity_per_tick2, -0.015);
    assert_eq!(root.live_cap, 64);
    assert_eq!(root.draw_class(), Some(6));
    assert_eq!(root.atlas_grid, (4, 2));
    assert_eq!(root.atlas_frames, 8);
    assert_eq!(root.size.mode, ChannelMode::Keyframed);
    assert_eq!(root.size.scaled_at(1.0), 2.5);
    assert_eq!(root.alpha.hi, 200.0);
    assert_eq!(root.drag_per_tick(), Some([0.98, 0.85, 0.5]));
    assert_eq!(emitters[1].name, "bits");
}

/// A palette entry is a `u32`, not four bytes: the one field a big-endian read
/// gets wrong if it is copied straight through.
#[test]
fn a_big_endian_palette_entry_comes_back_in_the_authored_order() {
    let data = twin(ByteOrder::Big);
    let parsed = ParticleSystem::parse(&data).expect("parse");
    let emitters = parsed.emitters(&data).expect("emitters");
    assert_eq!(emitters[0].colours[0], [1, 2, 3, 4]);
    assert_eq!(
        &data[HEADER_LEN + 2 * SLOT_LEN + 0xc4..][..4],
        &[4, 3, 2, 1],
        "on disc it really is stored the other way round"
    );
}

/// A blob's own magic says which order it is; nothing asks what console it
/// came from.
#[test]
fn the_magic_is_the_whole_of_how_a_blob_declares_its_order() {
    assert_eq!(byte_order(MAGIC), ByteOrder::Little);
    assert_eq!(byte_order(MAGIC_BE), ByteOrder::Big);
    assert_eq!(byte_order(b"WOtd"), ByteOrder::Little);
    assert_eq!(byte_order(&[]), ByteOrder::Little);

    assert!(looks_like_particle_system(MAGIC));
    assert!(looks_like_particle_system(MAGIC_BE));
    assert!(!looks_like_particle_system(b"WOtd"));
    assert!(!looks_like_particle_system(b"PSY"));
}
