//! Wipeout Pure's PSN DLC decryption.
//!
//! Unlike Pulse's `PACKn.edat` ([`crate::wad`] under a misleading extension,
//! not encrypted), Pure's `pi.wad` is ciphertext: an 8-round XTEA stream
//! cipher, keyed per pack, wrapping an ordinary [`wad`](crate::wad) payload plus
//! a 256-byte per-region trailer this project does not read. See
//! `docs/formats/dlc-pack.md#pures-packs-decrypt-with-an-external-key-table`
//! for the evidence and confidence score.
//!
//! # No keys are shipped in this crate
//!
//! The algorithm is public, hand-rolled from a published, independently verified
//! source. The **keys** are not shipped: redistribution is not established as
//! permitted, so, like the Vita zRIF table and every disc image, they are
//! user-supplied under `data/keys/`, gitignored. [`parse_keys`] reads that
//! file's format; nothing here or in its tests embeds a real key. See
//! `data/keys/README.md`.

pub const SIGNATURE_LEN: usize = 256;

const DELTA: u32 = 0x9e37_79b9;
const SEED: u32 = 0x1234_5678;

/// 8-round XTEA on one 64-bit block.
///
/// Not the usual 32/64-round TEA family: the upstream tool uses 8, and a
/// shipped `pi.wad` only decrypts to a valid WAD header under that count.
fn xtea8(mut v0: u32, mut v1: u32, key: [u32; 4]) -> (u32, u32) {
    let mut sum: u32 = 0;
    for _ in 0..8 {
        let t0 = (v1 << 4 ^ v1 >> 5).wrapping_add(v1);
        v0 = v0.wrapping_add(t0 ^ key[(sum & 3) as usize].wrapping_add(sum));
        sum = sum.wrapping_add(DELTA);
        let t1 = (v0 << 4 ^ v0 >> 5).wrapping_add(v0);
        v1 = v1.wrapping_add(t1 ^ key[((sum >> 11) & 3) as usize].wrapping_add(sum));
    }
    (v0, v1)
}

/// XORs a keystream over `buf` in place, one 8-byte block at a time.
///
/// Block `i`'s keystream is `xtea8(0x12345678, i, key)`, so this is symmetric
/// (one call encrypts and decrypts) and can start at any 8-byte-aligned offset;
/// callers start at 0, which a shipped pack needs.
///
/// Public because a fixture needing "ciphertext" builds one by calling this on
/// plaintext, the operation [`decrypt_pack`] uses to undo it.
#[must_use]
pub fn crypt_with_key(buf: &[u8], key: [u32; 4]) -> Vec<u8> {
    let mut buf = buf.to_vec();
    crypt_with_key_in_place(&mut buf, key);
    buf
}

fn crypt_with_key_in_place(buf: &mut [u8], key: [u32; 4]) {
    for (index, chunk) in buf.chunks_mut(8).enumerate() {
        let (v0, v1) = xtea8(SEED, index as u32, key);
        let keystream = [v0.to_le_bytes(), v1.to_le_bytes()].concat();
        for (byte, stream) in chunk.iter_mut().zip(keystream.iter()) {
            *byte ^= stream;
        }
    }
}

/// One row of the external key table: a PSN content id and its 128-bit key.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DlcKey {
    /// The content id this key belongs to, e.g. `UCES00001DGAMMAPAK`, read from
    /// that pack's `PARAM.sfo` but **not** reliable as a lookup against the
    /// containing folder name; see [`decrypt_pack`].
    pub name: String,
    pub key: [u32; 4],
}

/// Parses `data/keys/pure-dlc-keys.txt`: one `NAME 0xHH ... (x16)` row per line,
/// `#` comments and blank lines ignored. A malformed row is dropped, as
/// [`crate::wad`] drops an unparseable directory entry.
#[must_use]
pub fn parse_keys(text: &str) -> Vec<DlcKey> {
    text.lines().filter_map(parse_key_line).collect()
}

fn parse_key_line(line: &str) -> Option<DlcKey> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    let mut fields = line.split_whitespace();
    let name = fields.next()?.to_string();
    let mut bytes = [0u8; 16];
    for byte in &mut bytes {
        let token = fields.next()?;
        *byte = u8::from_str_radix(token.strip_prefix("0x").unwrap_or(token), 16).ok()?;
    }
    let key = [
        u32::from_le_bytes(bytes[0..4].try_into().ok()?),
        u32::from_le_bytes(bytes[4..8].try_into().ok()?),
        u32::from_le_bytes(bytes[8..12].try_into().ok()?),
        u32::from_le_bytes(bytes[12..16].try_into().ok()?),
    ];
    Some(DlcKey { name, key })
}

/// Decrypts a Pure `pi.wad`, or `None` if `data` is too short or no key in
/// `keys` fits.
///
/// Tries every key against the first 8 bytes and keeps whichever gives a
/// `version == 1` WAD header, as the upstream tool does - **deliberately not**
/// a lookup by folder or file name: the Gamma pack's zip unpacks to
/// `UCES00001DGAMMAPACK`, one letter off the `GAMMAPAK` content id its
/// `PARAM.sfo` and this key table use.
///
/// Returns the decrypted **payload only** (without the [`SIGNATURE_LEN`]-byte
/// trailer) and the key that worked, so a caller can log which pack it found.
#[must_use]
pub fn decrypt_pack<'k>(data: &[u8], keys: &'k [DlcKey]) -> Option<(Vec<u8>, &'k DlcKey)> {
    let payload_len = data.len().checked_sub(SIGNATURE_LEN)?;
    if payload_len < 8 {
        return None;
    }
    let dlc_key = keys.iter().find(|candidate| {
        let mut probe: [u8; 8] = data[..8].try_into().expect("checked len above");
        crypt_with_key_in_place(&mut probe, candidate.key);
        u32::from_le_bytes(probe[..4].try_into().expect("8 bytes sliced to 4")) == 1
    })?;
    let mut payload = data[..payload_len].to_vec();
    crypt_with_key_in_place(&mut payload, dlc_key.key);
    Some((payload, dlc_key))
}

#[cfg(test)]
mod tests {
    use super::{DlcKey, SIGNATURE_LEN, crypt_with_key, decrypt_pack, parse_keys};

    /// A key that appears in no real key table: hand-picked, not shipped bytes,
    /// per [ADR-0006](../../../docs/architecture/adr/0006-no-copyrighted-content.md).
    const TEST_KEY: [u32; 4] = [0x1122_3344, 0x5566_7788, 0x99aa_bbcc, 0xddee_ff00];

    fn wrap_as_pack(wad: Vec<u8>, key: [u32; 4]) -> Vec<u8> {
        let mut pack = crypt_with_key(&wad, key);
        pack.resize(pack.len() + SIGNATURE_LEN, 0xaa); // an unread trailer
        pack
    }

    #[test]
    fn round_trips_through_encryption_and_decryption() {
        let mut plain = vec![1, 0, 0, 0, 4, 0, 0, 0]; // version=1, nitems=4
        plain.extend(std::iter::repeat_n(0x42u8, 57)); // an odd length on purpose
        let pack = wrap_as_pack(plain.clone(), TEST_KEY);

        let keys = vec![DlcKey {
            name: "TEST00000DTESTPACK".to_string(),
            key: TEST_KEY,
        }];
        let (decrypted, found) = decrypt_pack(&pack, &keys).expect("the only key fits");

        assert_eq!(decrypted, plain);
        assert_eq!(found.name, "TEST00000DTESTPACK");
    }

    #[test]
    fn the_wrong_key_never_matches() {
        let plain = vec![1, 0, 0, 0, 0, 0, 0, 0];
        let pack = wrap_as_pack(plain, TEST_KEY);

        let wrong_key = [0u32; 4];
        let keys = vec![DlcKey {
            name: "WRONG".to_string(),
            key: wrong_key,
        }];
        assert!(decrypt_pack(&pack, &keys).is_none());
    }

    #[test]
    fn too_short_for_a_trailer_is_rejected_before_trying_any_key() {
        let keys = vec![DlcKey {
            name: "ANY".to_string(),
            key: TEST_KEY,
        }];
        assert!(decrypt_pack(&[0u8; SIGNATURE_LEN], &keys).is_none());
    }

    #[test]
    fn parses_the_shipped_key_table_format() {
        let text = "\
# a comment, and a blank line follow

UCES00001DGAMMAPAK 0x10 0x70 0x53 0xaf 0xaa 0xd9 0x76 0x88 0x72 0x3e 0x13 0xcb 0xf1 0x19 0xa4 0xcb
";
        let keys = parse_keys(text);
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].name, "UCES00001DGAMMAPAK");
        // Little-endian per 4-byte group, matching the algorithm's own key indexing.
        assert_eq!(keys[0].key[0], 0xaf53_7010);
    }

    #[test]
    fn a_malformed_row_is_dropped_not_fatal() {
        let text = "GOOD 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00 0x00\nBAD too short\n";
        let keys = parse_keys(text);
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].name, "GOOD");
    }
}
