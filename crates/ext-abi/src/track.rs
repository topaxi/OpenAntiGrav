//! The track region: the race's track as a guest reads it, written by the host into
//! the guest's `oag_track() -> u32` buffer before each call. Read-only by contract, the
//! same every tick of a race, and no state: nothing in it is hashed.
//!
//! Every entity is one record: the track itself, then each speed pad, each weapon pad
//! and each trigger, in that order and in the track's own order within a kind. So the
//! `i`-th speed pad is entity `1 + i`. Each entity carries its box and the
//! author-defined properties of a track file (`mod_*` in the Blender add-on), as
//! `(key hash, f32)` pairs; the engine gives none of them a meaning. A disc circuit
//! carries its pads with no properties.
//!
//! | Offset | Size | Field |
//! | --- | --- | --- |
//! | [`SPEED_PADS_AT`] | 2 | speed pads, `u16` |
//! | [`WEAPON_PADS_AT`] | 2 | weapon pads, `u16` |
//! | [`TRIGGERS_AT`] | 2 | triggers, `u16` |
//! | 6 | 2 | zero |
//! | [`ENTITIES_AT`] | [`ENTITY_LEN`] x [`MAX_ENTITIES`] | entities: kind `u8`, zero `u8`, index `u16`, first property `u16`, property count `u16`, local-to-world 16 x `f32` (row-major, translation in row 3), box minimum 3 x `f32`, box maximum 3 x `f32` (local space) |
//! | [`PROPS_AT`] | [`PROP_LEN`] x [`MAX_PROPS`] | properties: key `u32` ([`prop_key`]), value `f32` |
//!
//! The caps are chosen. What was counted: Pulse PSP's 40 track files carry 544 speed
//! and 218 weapon pads in all, HD's Talon's Junction 18 speed pads; no per-circuit
//! maximum was taken, and 2048 and Omega were not counted. A track over a cap is
//! refused by name at load.

/// Entities in one region: the track, its pads and its triggers.
pub const MAX_ENTITIES: usize = 256;
/// Triggers one track may carry: one bit each in the engine's per-craft mask.
pub const MAX_TRIGGERS: usize = 64;
/// Properties over every entity of one region.
pub const MAX_PROPS: usize = 1024;

/// Where the speed-pad count is written.
pub const SPEED_PADS_AT: usize = 0;
/// Where the weapon-pad count is written.
pub const WEAPON_PADS_AT: usize = 2;
/// Where the trigger count is written.
pub const TRIGGERS_AT: usize = 4;
/// Where the entity records start.
pub const ENTITIES_AT: usize = 8;
/// One entity record.
pub const ENTITY_LEN: usize = 96;
/// Where the property records start.
pub const PROPS_AT: usize = ENTITIES_AT + ENTITY_LEN * MAX_ENTITIES;
/// One property record.
pub const PROP_LEN: usize = 8;
/// The region's length. A guest's buffer must be at least this long.
pub const TRACK_LEN: usize = PROPS_AT + PROP_LEN * MAX_PROPS;

/// Within an entity record: the kind code.
pub const ENTITY_KIND: usize = 0;
/// Within an entity record: the index within its kind, `u16`.
pub const ENTITY_INDEX: usize = 2;
/// Within an entity record: its first property record, `u16`.
pub const ENTITY_FIRST_PROP: usize = 4;
/// Within an entity record: how many property records are its, `u16`.
pub const ENTITY_PROP_COUNT: usize = 6;
/// Within an entity record: local to world, 16 `f32`.
pub const ENTITY_TO_WORLD: usize = 8;
/// Within an entity record: the box minimum, 3 `f32`.
pub const ENTITY_MIN: usize = 72;
/// Within an entity record: the box maximum, 3 `f32`.
pub const ENTITY_MAX: usize = 84;
const _: () = assert!(ENTITY_MAX + 12 == ENTITY_LEN);
const _: () = assert!(MAX_PROPS <= u16::MAX as usize);

/// The FNV-1a 32 hash of a property's name, its key in the region: computed at compile
/// time by the guest SDK's `prop!` and at load by the host, from the name's UTF-8 bytes
/// (the `mod_` prefix already stripped).
#[must_use]
pub const fn prop_key(name: &str) -> u32 {
    let bytes = name.as_bytes();
    let mut hash = 0x811c_9dc5_u32;
    let mut i = 0;
    while i < bytes.len() {
        hash ^= bytes[i] as u32;
        hash = hash.wrapping_mul(0x0100_0193);
        i += 1;
    }
    hash
}

/// What an entity is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Kind {
    /// The track itself: its box is empty, its properties are the track's.
    Track,
    /// A speed pad.
    SpeedPad,
    /// A weapon pad.
    WeaponPad,
    /// A trigger volume of a track file.
    Trigger,
}

impl Kind {
    /// Its code in an entity record, from 1.
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8 + 1
    }

    /// The kind a code names, if any.
    #[must_use]
    pub const fn from_code(code: u8) -> Option<Self> {
        match code {
            1 => Some(Self::Track),
            2 => Some(Self::SpeedPad),
            3 => Some(Self::WeaponPad),
            4 => Some(Self::Trigger),
            _ => None,
        }
    }
}

/// A box with properties, as the host hands it to [`write`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Entity<'a> {
    /// Local to world, row-major, translation in row 3.
    pub to_world: [f32; 16],
    /// The box minimum, local space.
    pub min: [f32; 3],
    /// The box maximum, local space.
    pub max: [f32; 3],
    /// `(prop_key, value)` pairs.
    pub props: &'a [(u32, f32)],
}

/// The identity matrix: the track entity's placement.
pub const IDENTITY: [f32; 16] = [
    1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0,
];

/// Why a track does not fit the region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TooBig {
    /// More entities than [`MAX_ENTITIES`].
    Entities(usize),
    /// More triggers than [`MAX_TRIGGERS`].
    Triggers(usize),
    /// More properties than [`MAX_PROPS`].
    Props(usize),
    /// Two properties of one entity share a key hash.
    Clash {
        /// The entity's kind.
        kind: Kind,
        /// Its index within the kind.
        index: usize,
        /// The shared hash.
        key: u32,
    },
}

impl core::fmt::Display for TooBig {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::Entities(n) => write!(
                f,
                "{n} entities (the track, its pads and triggers), a mod reads at most {MAX_ENTITIES}"
            ),
            Self::Triggers(n) => write!(f, "{n} triggers, a mod reads at most {MAX_TRIGGERS}"),
            Self::Props(n) => write!(f, "{n} properties, a mod reads at most {MAX_PROPS}"),
            Self::Clash { kind, index, key } => write!(
                f,
                "{kind:?} {index} has two properties whose names hash to {key:#010x}"
            ),
        }
    }
}

/// Writes a track into `out` (at least [`TRACK_LEN`] long, zeroed past what this
/// writes) and returns how many bytes it used.
///
/// # Errors
///
/// [`TooBig`]: a cap passed, or a key hash twice on one entity.
///
/// # Panics
///
/// If `out` is shorter than [`TRACK_LEN`].
pub fn write(
    out: &mut [u8],
    track: &[(u32, f32)],
    speed_pads: &[Entity<'_>],
    weapon_pads: &[Entity<'_>],
    triggers: &[Entity<'_>],
) -> Result<usize, TooBig> {
    let out = &mut out[..TRACK_LEN];
    let entities = 1 + speed_pads.len() + weapon_pads.len() + triggers.len();
    if triggers.len() > MAX_TRIGGERS {
        return Err(TooBig::Triggers(triggers.len()));
    }
    if entities > MAX_ENTITIES {
        return Err(TooBig::Entities(entities));
    }
    let whole = Entity {
        to_world: IDENTITY,
        min: [0.0; 3],
        max: [0.0; 3],
        props: track,
    };
    let all = [
        (Kind::Track, core::slice::from_ref(&whole)),
        (Kind::SpeedPad, speed_pads),
        (Kind::WeaponPad, weapon_pads),
        (Kind::Trigger, triggers),
    ];
    let props: usize = all
        .iter()
        .flat_map(|(_, list)| list.iter())
        .map(|e| e.props.len())
        .sum();
    if props > MAX_PROPS {
        return Err(TooBig::Props(props));
    }
    put_u16(out, SPEED_PADS_AT, speed_pads.len());
    put_u16(out, WEAPON_PADS_AT, weapon_pads.len());
    put_u16(out, TRIGGERS_AT, triggers.len());
    let (mut at, mut prop) = (0, 0);
    for (kind, list) in all {
        for (index, entity) in list.iter().enumerate() {
            for (i, (key, _)) in entity.props.iter().enumerate() {
                if entity.props[..i].iter().any(|(k, _)| k == key) {
                    return Err(TooBig::Clash {
                        kind,
                        index,
                        key: *key,
                    });
                }
            }
            let record = ENTITIES_AT + at * ENTITY_LEN;
            out[record + ENTITY_KIND] = kind.code();
            put_u16(out, record + ENTITY_INDEX, index);
            put_u16(out, record + ENTITY_FIRST_PROP, prop);
            put_u16(out, record + ENTITY_PROP_COUNT, entity.props.len());
            let floats = entity.to_world.iter().chain(&entity.min).chain(&entity.max);
            for (i, value) in floats.enumerate() {
                put_f32(out, record + ENTITY_TO_WORLD + 4 * i, *value);
            }
            for (key, value) in entity.props {
                let p = PROPS_AT + prop * PROP_LEN;
                out[p..p + 4].copy_from_slice(&key.to_le_bytes());
                put_f32(out, p + 4, *value);
                prop += 1;
            }
            at += 1;
        }
    }
    Ok(if prop == 0 {
        ENTITIES_AT + at * ENTITY_LEN
    } else {
        PROPS_AT + prop * PROP_LEN
    })
}

/// A region as a guest reads it. A short or zeroed region reads as a track with no
/// pads, no triggers and no properties.
#[derive(Debug, Clone, Copy)]
pub struct Region<'a> {
    bytes: &'a [u8],
}

impl<'a> Region<'a> {
    /// A view over the guest's buffer.
    #[must_use]
    pub const fn new(bytes: &'a [u8]) -> Self {
        Self { bytes }
    }

    /// How many entities of `kind` the track has (1 for [`Kind::Track`]).
    #[must_use]
    pub fn count(&self, kind: Kind) -> usize {
        match kind {
            Kind::Track => 1,
            Kind::SpeedPad => get_u16(self.bytes, SPEED_PADS_AT),
            Kind::WeaponPad => get_u16(self.bytes, WEAPON_PADS_AT),
            Kind::Trigger => get_u16(self.bytes, TRIGGERS_AT),
        }
    }

    /// The `index`-th entity of `kind`, if there is one.
    #[must_use]
    pub fn entity(&self, kind: Kind, index: usize) -> Option<EntityView<'a>> {
        if index >= self.count(kind) {
            return None;
        }
        let before = match kind {
            Kind::Track => 0,
            Kind::SpeedPad => 1,
            Kind::WeaponPad => 1 + self.count(Kind::SpeedPad),
            Kind::Trigger => 1 + self.count(Kind::SpeedPad) + self.count(Kind::WeaponPad),
        };
        let at = ENTITIES_AT + (before + index) * ENTITY_LEN;
        let record = self.bytes.get(at..at + ENTITY_LEN)?;
        Some(EntityView {
            record,
            props: self.bytes.get(PROPS_AT..).unwrap_or(&[]),
        })
    }
}

/// One entity of a [`Region`].
#[derive(Debug, Clone, Copy)]
pub struct EntityView<'a> {
    record: &'a [u8],
    props: &'a [u8],
}

impl EntityView<'_> {
    /// Its kind; `None` for a code this revision does not know.
    #[must_use]
    pub fn kind(&self) -> Option<Kind> {
        Kind::from_code(self.record[ENTITY_KIND])
    }

    /// Its index within its kind.
    #[must_use]
    pub fn index(&self) -> usize {
        get_u16(self.record, ENTITY_INDEX)
    }

    /// Local to world, row-major, translation in row 3.
    #[must_use]
    pub fn to_world(&self) -> [f32; 16] {
        core::array::from_fn(|i| get_f32(self.record, ENTITY_TO_WORLD + 4 * i))
    }

    /// The box minimum, local space.
    #[must_use]
    pub fn min(&self) -> [f32; 3] {
        core::array::from_fn(|i| get_f32(self.record, ENTITY_MIN + 4 * i))
    }

    /// The box maximum, local space.
    #[must_use]
    pub fn max(&self) -> [f32; 3] {
        core::array::from_fn(|i| get_f32(self.record, ENTITY_MAX + 4 * i))
    }

    /// Its properties, `(prop_key, value)`, in the order authored.
    pub fn props(&self) -> impl Iterator<Item = (u32, f32)> + '_ {
        let first = get_u16(self.record, ENTITY_FIRST_PROP);
        let count = get_u16(self.record, ENTITY_PROP_COUNT);
        (first..first + count).filter_map(|i| {
            let at = i * PROP_LEN;
            let bytes = self.props.get(at..at + PROP_LEN)?;
            Some((get_u32(bytes, 0), get_f32(bytes, 4)))
        })
    }

    /// The property whose key is `key`, if it has one.
    #[must_use]
    pub fn prop(&self, key: u32) -> Option<f32> {
        self.props().find(|(k, _)| *k == key).map(|(_, v)| v)
    }
}

fn put_u16(out: &mut [u8], at: usize, value: usize) {
    out[at..at + 2].copy_from_slice(&(value as u16).to_le_bytes());
}

fn put_f32(out: &mut [u8], at: usize, value: f32) {
    out[at..at + 4].copy_from_slice(&value.to_bits().to_le_bytes());
}

fn get_u16(bytes: &[u8], at: usize) -> usize {
    match bytes.get(at..at + 2) {
        Some(&[a, b]) => usize::from(u16::from_le_bytes([a, b])),
        _ => 0,
    }
}

fn get_u32(bytes: &[u8], at: usize) -> u32 {
    match bytes.get(at..at + 4) {
        Some(&[a, b, c, d]) => u32::from_le_bytes([a, b, c, d]),
        _ => 0,
    }
}

fn get_f32(bytes: &[u8], at: usize) -> f32 {
    f32::from_bits(get_u32(bytes, at))
}
