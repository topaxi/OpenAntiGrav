use super::*;

#[test]
fn parses_hex_in_the_forms_a_player_pastes() {
    let want = DiscKey::parse(b"00112233445566778899aabbccddeeff").unwrap();
    assert_eq!(want.to_hex(), "00112233445566778899aabbccddeeff");
    for form in [
        "  00112233445566778899AABBCCDDEEFF\n",
        "0x00112233445566778899aabbccddeeff",
        "00112233 44556677 8899aabb ccddeeff",
        "00-11-22-33-44-55-66-77-88-99-aa-bb-cc-dd-ee-ff",
    ] {
        assert_eq!(
            DiscKey::parse(form.as_bytes()),
            Some(want.clone()),
            "{form}"
        );
    }
}

#[test]
fn parses_a_raw_sixteen_byte_dkey_and_refuses_the_rest() {
    let raw: Vec<u8> = (0xf0..=0xff).collect();
    assert_eq!(
        DiscKey::parse(&raw).unwrap().to_hex(),
        "f0f1f2f3f4f5f6f7f8f9fafbfcfdfeff"
    );
    assert!(DiscKey::parse(b"").is_none());
    assert!(DiscKey::parse(b"0011").is_none());
    assert!(DiscKey::parse(b"zz112233445566778899aabbccddeeff").is_none());
    assert!(DiscKey::parse(&[0u8; 17]).is_none());
}

#[test]
fn debug_never_prints_the_key() {
    let key = DiscKey::parse(b"00112233445566778899aabbccddeeff").unwrap();
    let shown = format!("{key:?}");
    assert!(!shown.contains("0011") && !shown.contains("17"), "{shown}");
}

fn table(bounds: &[u32]) -> Vec<u8> {
    let mut s = vec![0u8; SECTOR_SIZE];
    s[..4].copy_from_slice(&u32::try_from(bounds.len() / 2).unwrap().to_be_bytes());
    for (i, b) in bounds.iter().enumerate() {
        s[8 + 4 * i..12 + 4 * i].copy_from_slice(&b.to_be_bytes());
    }
    s
}

#[test]
fn regions_follow_the_hd_layout() {
    let r = Regions::parse(
        &table(&[0, 0x75f, 0xecac0, 0xecb00, 0xed040, 0xed4df]),
        0xed4e0,
    )
    .unwrap();
    assert!(!r.is_encrypted(0x75f));
    assert!(r.is_encrypted(0x760));
    assert!(r.is_encrypted(0xecabf));
    assert!(!r.is_encrypted(0xecac0));
    assert!(r.has_encrypted());
}

#[test]
fn a_table_that_does_not_tile_the_image_is_refused() {
    assert!(Regions::parse(&table(&[0, 99]), 200).is_none());
    assert!(Regions::parse(&table(&[5, 99]), 100).is_none());
    assert!(Regions::parse(&table(&[0, 50, 40, 99]), 100).is_none());
    assert!(Regions::parse(&[0u8; SECTOR_SIZE], 100).is_none());
    assert!(
        !Regions::parse(&table(&[0, 99]), 100)
            .unwrap()
            .has_encrypted()
    );
}

/// AES-128 FIPS-197 appendix C.1, through the real sector routine's block step:
/// a one-block CBC with an all-zero IV is plain ECB.
#[test]
fn the_block_cipher_matches_the_fips_197_vector() {
    let key: Vec<u8> = (0u8..16).collect();
    let cipher = Aes128::new(GenericArray::from_slice(&key));
    let mut block = GenericArray::clone_from_slice(&hex16("00112233445566778899aabbccddeeff"));
    cipher.encrypt_block(&mut block);
    assert_eq!(block.as_slice(), hex16("69c4e0d86a7b0430d8cdb78070b4c55a"));
}

fn hex16(s: &str) -> Vec<u8> {
    (0..16)
        .map(|i| u8::from_str_radix(&s[2 * i..2 * i + 2], 16).unwrap())
        .collect()
}

/// Encrypts a sector with the documented scheme, independently of
/// `decrypt_sector`, so the round trip proves the IV rule (absolute LBA,
/// chained within the sector only) rather than restating the code.
fn encrypt_sector(cipher: &Aes128, lba: u32, plain: &[u8]) -> Vec<u8> {
    let mut previous = [0u8; 16];
    previous[12..].copy_from_slice(&lba.to_be_bytes());
    let mut out = Vec::new();
    for block in plain.chunks_exact(16) {
        let mut b = GenericArray::clone_from_slice(block);
        for (x, p) in b.iter_mut().zip(previous) {
            *x ^= p;
        }
        cipher.encrypt_block(&mut b);
        previous.copy_from_slice(&b);
        out.extend_from_slice(&b);
    }
    out
}

#[test]
fn a_sector_round_trips_with_the_absolute_lba_as_iv() {
    let cipher = Aes128::new(GenericArray::from_slice(&[7u8; 16]));
    let plain: Vec<u8> = (0..SECTOR_SIZE).map(|i| (i * 31 % 251) as u8).collect();
    let mut sector = encrypt_sector(&cipher, 0xed040, &plain);
    assert_ne!(sector, plain);
    decrypt_sector(&cipher, 0xed040, &mut sector);
    assert_eq!(sector, plain);
    let mut wrong_iv = encrypt_sector(&cipher, 0xed040, &plain);
    decrypt_sector(&cipher, 0x1, &mut wrong_iv);
    assert_ne!(wrong_iv, plain, "a region-relative IV must not decrypt");
}
