//! `.pob` particle systems: a `SYSP` container around an undecoded payload.
//!
//! These are the `Data\Psys\*.POB` blobs the front-end and weapon effects
//! load by name (`Data\Psys\%s.POB`, built at `FUN_089156a0` in the PSP
//! `BOOT.BIN`). 35 exist in `Data.wad` on the PSP disc, one per authored
//! effect - `WO_SHIP_COLL_SPARK_DAMAGE`, `WO_MISSILE_EXPLO`, `WO_RAIN`, and so
//! on. See `docs/formats/pob.md` for the evidence and open questions.
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
//! # What the slot table holds is not known
//!
//! The values are not per-file byte offsets: slot 0 is exactly 1220 in every
//! one of the 35 files regardless of table length or total file size, which a
//! self-relative offset could not be. They read more like a fixed vocabulary,
//! a small heavily reused set of values, which fits the `ParticleAgeMapper`,
//! `ParticleColorMapper`, `ParticleIncandecenceMapper` and
//! `ParticleTransparencyMapper` class names findable elsewhere in the
//! executable's string table: one plausible reading is a fixed slot per
//! attribute-mapper class, present or absent (`None`) per particle system.
//! That is a hypothesis, not a finding, see the format page.

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
    /// (the raw `0xffffffff`), `Some` otherwise. See the module documentation
    /// for what the values are not yet known to mean.
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
}
