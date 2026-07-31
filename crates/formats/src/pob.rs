//! `.pob` particle systems: a `SYSP` container around an undecoded payload.
//!
//! These are the `Data\Psys\*.POB` blobs the front-end and weapon effects
//! load by name (`Data\Psys\%s.POB`, built at `FUN_089156a0` in the PSP
//! `BOOT.BIN`). 35 exist in `Data.wad` on the PSP disc, one per authored
//! effect - `WO_SHIP_COLL_SPARK_DAMAGE`, `WO_MISSILE_EXPLO`, `WO_RAIN`, and so
//! on. The PS2 port ships the identical container: 41 `SYSP` blobs in
//! `WADS2.WAD`, 32 sharing a name hash (and so a path) with PSP, parsed and
//! resolved by this same module with no code changes. See
//! `docs/formats/pob.md` for the evidence and open questions.
//!
//! # Container
//!
//! ```text
//! +0x00  char[4]  "SYSP"
//! +0x04  u32      header_len(16) + payload.len()
//! +0x08  u16      slot count, entries in the table at +0x10
//! +0x0a  u16      1 in every file seen
//! +0x0c  u32      1 in every file seen
//! +0x10  slot[count], 4 bytes each: u32, or 0xffffffff for an unused slot
//!        ...      32-byte NUL-terminated name, immediately after the table
//!        ...      payload, undecoded
//! ```
//!
//! The `+0x04` field is redundant with the file's own length - it is validated
//! on parse rather than exposed - and the name field's position (immediately
//! after the table, not the fixed `+0x80` a single sample first suggested) is
//! confirmed by hashing the recovered name as a WAD entry path and finding it
//! agrees with the blob's own entry hash, on all 35 files in the corpus.
//!
//! # The slot table is a pointer-fixup table
//!
//! Confirmed by both decompilation and a live PPSSPP trace (`FUN_088f8e38`,
//! called from the generic resource loader `FUN_088f3540` on every fresh
//! load): a slot's raw value is not the offset of its data. It is the offset
//! of a **fixup site** - a 4-byte field, relative to the resource's own base
//! (`HEADER_LEN + slots.len() * SLOT_LEN`, i.e. right where the slot table
//! ends) - holding a second, baked offset from the same base. The loader adds
//! the base to that stored value in place, turning it from an on-disk
//! relative offset into a live pointer:
//!
//! ```text
//! fixup_site = resource_base + slots[i]
//! target     = resource_base + read_u32(fixup_site)   // what the site holds *before* the add
//! ```
//!
//! [`ParticleSystem::resolve_slot`] replays exactly this, entirely from the
//! file's own bytes - no runtime needed to compute it, only to discover it.
//! Verified live: all 26 real slots of `WO_SHIP_COLL_SPARK_DAMAGE` matched
//! this arithmetic exactly, `fixup site value + resource_base == the address
//! PPSSPP wrote there`, and the same two-hop resolution lands in bounds on
//! **all 436 real slots across all 35 files** with zero exceptions.
//!
//! # Many resolved targets are readable strings
//!
//! 187 of those 436 (43%) are themselves NUL-terminated ASCII: developer
//! texture paths (`Z:\WipeoutPSP\X2\Data\Psys\Tex\orange_glow2.tga`,
//! `Z:\Art_Resources\Psys\Tex\pointglow_32x32.tga`) and short authored layer
//! names (`GLOW`, `debris`, `thin_streaks`, `RINGS`). So a slot is a
//! per-channel record: sometimes a texture reference, sometimes a name,
//! sometimes (the non-string targets) a small run of floats whose layout is
//! still unread. Neither the record's full field layout nor what a *shared*
//! target (several slots resolving to the identical offset - the corpus's
//! most common case, five slots to one target) means beyond "these channels
//! fall back to the same default" is known yet - see the format page.

/// The container magic.
pub const MAGIC: &[u8; 4] = b"SYSP";

/// Bytes before the slot table: magic, the size field, the count, and the two
/// constant header words.
pub const HEADER_LEN: usize = 0x10;

/// Bytes per slot-table entry.
pub const SLOT_LEN: usize = 4;

/// Bytes of the fixed name field following the slot table.
pub const NAME_LEN: usize = 32;

/// The slot value meaning "unused".
pub const EMPTY_SLOT: u32 = 0xffff_ffff;

/// Something wrong with a particle system blob.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// Fewer bytes than the fixed header needs.
    TooShort {
        /// Bytes supplied.
        got: usize,
    },
    /// The blob does not start with [`MAGIC`].
    NotSysp,
    /// One of the two header words seen as a constant 1 on every real file
    /// was something else.
    UnexpectedHeaderWord {
        /// The field, named by its byte offset.
        field: &'static str,
        /// The value found.
        value: u32,
    },
    /// The slot table and the fixed name field run past the end of the blob.
    TableOutOfRange {
        /// The declared slot count.
        count: u16,
    },
    /// No NUL byte in the 32-byte name field.
    NameNotTerminated,
    /// `+0x04` disagreed with `HEADER_LEN + payload.len()`, computed from the
    /// blob's own length. Every one of 35 real files agrees exactly, so a
    /// disagreement means this is not really a `.pob` payload rather than a
    /// damaged one.
    SizeDisagreement {
        /// What `+0x04` says.
        declared: u32,
        /// `HEADER_LEN + payload.len()`.
        computed: u32,
    },
    /// A slot's fixup site, or the target the site resolves to, runs past
    /// the end of the blob. Never observed on any of the 436 real slots
    /// across all 35 files.
    SlotOutOfRange {
        /// The slot's index in the table.
        index: usize,
    },
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TooShort { got } => write!(f, "need at least {HEADER_LEN} bytes, got {got}"),
            Self::NotSysp => write!(f, "the blob does not begin with SYSP"),
            Self::UnexpectedHeaderWord { field, value } => {
                write!(f, "header word {field} is {value}, expected 1")
            }
            Self::TableOutOfRange { count } => {
                write!(f, "the {count}-slot table and name field run past the end")
            }
            Self::NameNotTerminated => write!(f, "no NUL in the 32-byte name field"),
            Self::SizeDisagreement { declared, computed } => write!(
                f,
                "+0x04 declares {declared}, but header length plus the payload is {computed}"
            ),
            Self::SlotOutOfRange { index } => {
                write!(f, "slot {index}'s fixup site or target runs past the end")
            }
        }
    }
}

impl std::error::Error for Error {}

/// Result alias for this module.
pub type Result<T> = std::result::Result<T, Error>;

/// A parsed particle system.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParticleSystem<'a> {
    /// The particle system's own name, e.g. `WO_SHIP_COLL_SPARK_DAMAGE`.
    ///
    /// `Data\Psys\<name>.POB` hashes to the blob's own WAD entry on all 35
    /// files in the corpus - see [`crate::wad::hash_name`].
    pub name: String,
    /// The slot table, one entry per declared slot: `None` for an unused slot
    /// (the raw `0xffffffff`), `Some(fixup_site)` otherwise. A slot's value is
    /// not its target's offset - it is a *fixup site* that must be resolved
    /// with [`Self::resolve_slot`]. See the module documentation.
    pub slots: Vec<Option<u32>>,
    /// Bytes after the name field, undecoded.
    pub payload: &'a [u8],
}

fn word(data: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([data[at], data[at + 1], data[at + 2], data[at + 3]])
}

fn half(data: &[u8], at: usize) -> u16 {
    u16::from_le_bytes([data[at], data[at + 1]])
}

/// Whether `data` looks like a particle system, without parsing it.
#[must_use]
pub fn looks_like_particle_system(data: &[u8]) -> bool {
    data.len() >= 4 && &data[0..4] == MAGIC
}

impl<'a> ParticleSystem<'a> {
    /// Parses a particle system blob.
    ///
    /// # Errors
    ///
    /// Fails when the blob is too short, does not start with [`MAGIC`], one of
    /// the two constant header words is not 1, the slot table and name field
    /// do not fit, the name field has no NUL, or `+0x04` disagrees with the
    /// blob's own length. Every one of those is the blob contradicting itself,
    /// not a judgement call about the data.
    pub fn parse(data: &'a [u8]) -> Result<Self> {
        if data.len() < HEADER_LEN {
            return Err(Error::TooShort { got: data.len() });
        }
        if &data[0..4] != MAGIC {
            return Err(Error::NotSysp);
        }
        let declared = word(data, 0x04);
        let count = half(data, 0x08);
        let header_a = u32::from(half(data, 0x0a));
        let header_c = word(data, 0x0c);
        if header_a != 1 {
            return Err(Error::UnexpectedHeaderWord {
                field: "+0x0a",
                value: header_a,
            });
        }
        if header_c != 1 {
            return Err(Error::UnexpectedHeaderWord {
                field: "+0x0c",
                value: header_c,
            });
        }

        let table_end = HEADER_LEN + usize::from(count) * SLOT_LEN;
        let name_end = table_end + NAME_LEN;
        if data.len() < name_end {
            return Err(Error::TableOutOfRange { count });
        }

        let slots = data[HEADER_LEN..table_end]
            .chunks_exact(SLOT_LEN)
            .map(|chunk| {
                let value = u32::from_le_bytes([chunk[0], chunk[1], chunk[2], chunk[3]]);
                (value != EMPTY_SLOT).then_some(value)
            })
            .collect();

        let name_field = &data[table_end..name_end];
        let nul = name_field
            .iter()
            .position(|&byte| byte == 0)
            .ok_or(Error::NameNotTerminated)?;
        let name = String::from_utf8_lossy(&name_field[..nul]).into_owned();

        let payload = &data[name_end..];
        let computed = (HEADER_LEN + payload.len()) as u32;
        if declared != computed {
            return Err(Error::SizeDisagreement { declared, computed });
        }

        Ok(Self {
            name,
            slots,
            payload,
        })
    }

    /// Resolves slot `index` to its target, as an offset into [`Self::payload`].
    ///
    /// Replays the pointer fixup the game's loader performs at load time (see
    /// the module documentation): reads the raw value stored at the slot's
    /// fixup site, adds the resource's own base to it, and returns that
    /// address as an offset into `payload` - so `payload[result..]` is the
    /// target. `data` must be the same bytes originally passed to
    /// [`Self::parse`].
    ///
    /// Returns `Ok(None)` for an unused slot, and also for an `index` past
    /// the end of [`Self::slots`] - there is no fixup site to resolve either
    /// way.
    ///
    /// # Errors
    ///
    /// [`Error::SlotOutOfRange`] if the fixup site, or the target it names,
    /// runs past the end of `data`. Never observed on any of the 436 real
    /// slots across all 35 files in the corpus, but `data` here is
    /// caller-supplied and not re-validated against `self`.
    pub fn resolve_slot(&self, data: &[u8], index: usize) -> Result<Option<usize>> {
        let Some(Some(slot)) = self.slots.get(index).copied() else {
            return Ok(None);
        };
        let resource_base = HEADER_LEN + self.slots.len() * SLOT_LEN;
        let site = resource_base + slot as usize;
        let Some(fixup_bytes) = data.get(site..site + SLOT_LEN) else {
            return Err(Error::SlotOutOfRange { index });
        };
        let baked = u32::from_le_bytes([
            fixup_bytes[0],
            fixup_bytes[1],
            fixup_bytes[2],
            fixup_bytes[3],
        ]);
        let target = resource_base + baked as usize;
        let name_end = resource_base + NAME_LEN;
        if target < name_end || data.len() <= target {
            return Err(Error::SlotOutOfRange { index });
        }
        Ok(Some(target - name_end))
    }
}

#[cfg(test)]
mod tests {
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
}
