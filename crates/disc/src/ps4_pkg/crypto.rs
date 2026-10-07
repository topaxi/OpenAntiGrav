//! The primitives a PS4 package needs beyond AES: SHA-256, HMAC-SHA-256, the RSA
//! unwrap and AES-XTS. Written here rather than pulled in: SHA-256 runs over a few
//! hundred bytes per package, and XTS is a loop over the `aes` block cipher this
//! crate already uses for the PS3 layer.

use aes::Aes128;
use aes::cipher::generic_array::GenericArray;
use aes::cipher::{BlockDecrypt, BlockEncrypt, KeyInit};
use num_bigint::BigUint;

const K: [u32; 64] = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
];

/// SHA-256 of `data`.
pub fn sha256(data: &[u8]) -> [u8; 32] {
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let mut msg = data.to_vec();
    msg.push(0x80);
    while msg.len() % 64 != 56 {
        msg.push(0);
    }
    msg.extend_from_slice(&((data.len() as u64) * 8).to_be_bytes());
    for block in msg.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (i, word) in block.chunks_exact(4).enumerate() {
            w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for i in 16..64 {
            let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
            let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
            w[i] = w[i - 16]
                .wrapping_add(s0)
                .wrapping_add(w[i - 7])
                .wrapping_add(s1);
        }
        let mut v = h;
        for i in 0..64 {
            let s1 = v[4].rotate_right(6) ^ v[4].rotate_right(11) ^ v[4].rotate_right(25);
            let ch = (v[4] & v[5]) ^ (!v[4] & v[6]);
            let t1 = v[7]
                .wrapping_add(s1)
                .wrapping_add(ch)
                .wrapping_add(K[i])
                .wrapping_add(w[i]);
            let s0 = v[0].rotate_right(2) ^ v[0].rotate_right(13) ^ v[0].rotate_right(22);
            let maj = (v[0] & v[1]) ^ (v[0] & v[2]) ^ (v[1] & v[2]);
            let t2 = s0.wrapping_add(maj);
            v = [
                t1.wrapping_add(t2),
                v[0],
                v[1],
                v[2],
                v[3].wrapping_add(t1),
                v[4],
                v[5],
                v[6],
            ];
        }
        for (a, b) in h.iter_mut().zip(v) {
            *a = a.wrapping_add(b);
        }
    }
    let mut out = [0u8; 32];
    for (chunk, word) in out.chunks_exact_mut(4).zip(h) {
        chunk.copy_from_slice(&word.to_be_bytes());
    }
    out
}

/// HMAC-SHA-256 of `data` under `key`.
pub fn hmac_sha256(key: &[u8], data: &[u8]) -> [u8; 32] {
    let mut k = [0u8; 64];
    if key.len() > 64 {
        k[..32].copy_from_slice(&sha256(key));
    } else {
        k[..key.len()].copy_from_slice(key);
    }
    let mut inner: Vec<u8> = k.iter().map(|b| b ^ 0x36).collect();
    inner.extend_from_slice(data);
    let mut outer: Vec<u8> = k.iter().map(|b| b ^ 0x5c).collect();
    outer.extend_from_slice(&sha256(&inner));
    sha256(&outer)
}

/// Raw RSA with a big-endian private exponent and modulus, then PKCS#1 v1.5
/// type-2 unpadding (`00 02 <nonzero> 00 payload`). `None` when the padding is
/// wrong, which is how a wrong key or a damaged entry shows.
pub fn rsa_decrypt(ciphertext: &[u8], d: &[u8], n: &[u8]) -> Option<Vec<u8>> {
    let n = BigUint::from_bytes_be(n);
    let m = BigUint::from_bytes_be(ciphertext).modpow(&BigUint::from_bytes_be(d), &n);
    let width = 256;
    let raw = m.to_bytes_be();
    if raw.len() > width {
        return None;
    }
    let mut em = vec![0u8; width - raw.len()];
    em.extend_from_slice(&raw);
    if em[0] != 0 || em[1] != 2 {
        return None;
    }
    let sep = em[2..].iter().position(|&b| b == 0)? + 2;
    if sep < 10 {
        return None;
    }
    Some(em[sep + 1..].to_vec())
}

/// `m^e mod n`, for the self-checks of a transcribed key.
pub fn rsa_public(m: &[u8], n: &[u8]) -> Vec<u8> {
    BigUint::from_bytes_be(m)
        .modpow(&BigUint::from(65_537u32), &BigUint::from_bytes_be(n))
        .to_bytes_be()
}

/// AES-128-CBC decryption, no padding.
pub fn aes_cbc_decrypt(data: &mut [u8], key: &[u8; 16], iv: &[u8; 16]) {
    let cipher = Aes128::new(GenericArray::from_slice(key));
    let mut prev = *iv;
    for block in data.chunks_exact_mut(16) {
        let cipher_block: [u8; 16] = block.try_into().expect("16-byte chunk");
        cipher.decrypt_block(GenericArray::from_mut_slice(block));
        for (b, p) in block.iter_mut().zip(prev) {
            *b ^= p;
        }
        prev = cipher_block;
    }
}

/// One AES-XTS-128 key pair, decrypting 4 KiB sectors the way the PS4's PFS does.
pub struct Xts {
    data: Aes128,
    tweak: Aes128,
}

impl Xts {
    /// `tweak_key` encrypts the sector number, `data_key` the data.
    pub fn new(tweak_key: &[u8; 16], data_key: &[u8; 16]) -> Self {
        Self {
            data: Aes128::new(GenericArray::from_slice(data_key)),
            tweak: Aes128::new(GenericArray::from_slice(tweak_key)),
        }
    }

    /// Decrypts `sector` in place; `number` is the sector's index in the image.
    pub fn decrypt_sector(&self, sector: &mut [u8], number: u64) {
        let mut t = [0u8; 16];
        t[..8].copy_from_slice(&number.to_le_bytes());
        self.tweak
            .encrypt_block(GenericArray::from_mut_slice(&mut t));
        for block in sector.chunks_exact_mut(16) {
            for (b, x) in block.iter_mut().zip(t) {
                *b ^= x;
            }
            self.data.decrypt_block(GenericArray::from_mut_slice(block));
            for (b, x) in block.iter_mut().zip(t) {
                *b ^= x;
            }
            let mut carry = 0u8;
            for byte in t.iter_mut() {
                let next = *byte >> 7;
                *byte = (*byte << 1) | carry;
                carry = next;
            }
            if carry != 0 {
                t[0] ^= 0x87;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ps4_pkg::keys;

    fn hex(b: &[u8]) -> String {
        b.iter().map(|x| format!("{x:02x}")).collect()
    }

    #[test]
    fn sha256_known_vectors() {
        assert_eq!(
            hex(&sha256(b"abc")),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            hex(&sha256(&[b'a'; 1000])),
            "41edece42d63e8d9bf515a9ba6932e1c20cbc9f5a5d134645adb5db1b9737ea3"
        );
    }

    #[test]
    fn hmac_rfc4231_case_2() {
        assert_eq!(
            hex(&hmac_sha256(b"Jefe", b"what do ya want for nothing?")),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[test]
    fn xts_ieee_vector_1() {
        let xts = Xts::new(&[0; 16], &[0; 16]);
        let mut ct = [
            0x91, 0x7c, 0xf6, 0x9e, 0xbd, 0x68, 0xb2, 0xec, 0x9b, 0x9f, 0xe9, 0xa3, 0xea, 0xdd,
            0xa6, 0x92, 0xcd, 0x43, 0xd2, 0xf5, 0x95, 0x98, 0xed, 0x85, 0x8c, 0x02, 0xc2, 0x65,
            0x2f, 0xbf, 0x92, 0x2e,
        ];
        xts.decrypt_sector(&mut ct, 0);
        assert_eq!(ct, [0u8; 32]);
    }

    #[test]
    fn transcribed_keys_round_trip() {
        for (d, n) in [
            (&keys::IMAGE_KEY_D, &keys::IMAGE_KEY_N),
            (&keys::ENTRY_KEY3_D, &keys::ENTRY_KEY3_N),
        ] {
            let m: Vec<u8> = (0..255u32).map(|i| (i * 7 + 3) as u8).collect();
            let c = rsa_public(&m, n);
            let mut padded = vec![0u8; 256 - c.len()];
            padded.extend_from_slice(&c);
            let back = BigUint::from_bytes_be(&padded)
                .modpow(&BigUint::from_bytes_be(d), &BigUint::from_bytes_be(n));
            assert_eq!(back, BigUint::from_bytes_be(&m));
        }
    }
}
