//! A player's key pair: what a player who hosts a race signs their registration with, and
//! what the lobby knows them by
//! (`docs/architecture/adr/0092-a-player-hosted-race-is-a-room-whose-owner-runs-the-race-host.md`,
//! decision 3). Ed25519, through `ring`.

use anyhow::{Result, anyhow};
use ring::signature::{ED25519, Ed25519KeyPair, KeyPair, UnparsedPublicKey};

use crate::wire::online::HostOffer;

/// A player's Ed25519 key pair.
pub struct PlayerKey {
    pair: Ed25519KeyPair,
    pkcs8: Vec<u8>,
}

impl PlayerKey {
    /// A fresh key pair.
    ///
    /// # Errors
    ///
    /// The system's random source failed.
    pub fn generate() -> Result<Self> {
        let rng = ring::rand::SystemRandom::new();
        let document = Ed25519KeyPair::generate_pkcs8(&rng)
            .map_err(|_| anyhow!("the system's random source failed"))?;
        Self::from_pkcs8(document.as_ref())
    }

    /// The key pair [`Self::pkcs8`] wrote.
    ///
    /// # Errors
    ///
    /// The bytes are not an Ed25519 key pair in PKCS#8.
    pub fn from_pkcs8(bytes: &[u8]) -> Result<Self> {
        let pair = Ed25519KeyPair::from_pkcs8(bytes)
            .map_err(|e| anyhow!("not an Ed25519 key pair: {e}"))?;
        Ok(Self {
            pair,
            pkcs8: bytes.to_vec(),
        })
    }

    /// The key pair as PKCS#8, to keep on disk. Secret.
    #[must_use]
    pub fn pkcs8(&self) -> &[u8] {
        &self.pkcs8
    }

    /// The public half: what the lobby knows this player by.
    #[must_use]
    pub fn public(&self) -> [u8; 32] {
        let mut key = [0; 32];
        key.copy_from_slice(self.pair.public_key().as_ref());
        key
    }

    /// The signed offer to host on `port` under the certificate `cert_hash`, for the lobby
    /// whose certificate hashes to `lobby` and whose welcome issued `token`.
    #[must_use]
    pub fn offer(
        &self,
        lobby: &[u8; 32],
        token: &str,
        port: u16,
        cert_hash: [u8; 32],
    ) -> HostOffer {
        let statement = HostOffer::statement(lobby, token, port, &cert_hash);
        let mut signature = [0; 64];
        signature.copy_from_slice(self.pair.sign(&statement).as_ref());
        HostOffer {
            port,
            cert_hash,
            key: self.public(),
            signature,
        }
    }
}

impl std::fmt::Debug for PlayerKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PlayerKey")
            .field("public", &crate::wt::hex(&self.public()))
            .finish_non_exhaustive()
    }
}

/// Whether `offer` is signed by its own key for this lobby and this session.
#[must_use]
pub fn verify(offer: &HostOffer, lobby: &[u8; 32], token: &str) -> bool {
    let statement = HostOffer::statement(lobby, token, offer.port, &offer.cert_hash);
    UnparsedPublicKey::new(&ED25519, &offer.key)
        .verify(&statement, &offer.signature)
        .is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_offer_verifies_only_for_the_lobby_session_port_and_certificate_it_was_signed_for() {
        let key = PlayerKey::generate().expect("a key");
        let again = PlayerKey::from_pkcs8(key.pkcs8()).expect("the key reads back");
        assert_eq!(key.public(), again.public());
        let offer = key.offer(&[1; 32], "token", 4435, [2; 32]);
        assert!(verify(&offer, &[1; 32], "token"));
        assert!(!verify(&offer, &[9; 32], "token"), "another lobby");
        assert!(!verify(&offer, &[1; 32], "other"), "another session");
        let mut moved = offer.clone();
        moved.port = 4436;
        assert!(!verify(&moved, &[1; 32], "token"), "another port");
        let mut other = offer.clone();
        other.cert_hash = [3; 32];
        assert!(!verify(&other, &[1; 32], "token"), "another certificate");
        let mut stolen = offer;
        stolen.key = PlayerKey::generate().expect("a key").public();
        assert!(!verify(&stolen, &[1; 32], "token"), "another player's key");
        assert!(PlayerKey::from_pkcs8(b"not a key").is_err());
    }
}
