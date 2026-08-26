//! The pass (OpenPGP) crypto backend, over rpgp.
//!
//! Validated in the Phase 0 spike against real gpg-produced stores across RSA,
//! Cv25519, passphrase-locked, and v6/RFC 9580 keys, both directions. Two
//! behaviours the spike proved necessary and that live here from day one:
//!
//! 1. Decryption goes through `TheRing` with
//!    `DecryptionOptions::enable_gnupg_aead()`. Modern gpg emits its
//!    non-standard AEAD packet (tag 20) whenever the recipient key advertises
//!    AEAD, which every key recent gpg generates does. rpgp reads it, but only
//!    when asked; the bare `Message::decrypt` fails on real-world entries.
//! 2. Encryption picks the SEIPD version from the recipient set: SEIPD v2 for a
//!    v6-only set, SEIPD v1 + AES256 (what a v4 gpg recipient expects) otherwise.

use pass_core::crypto::{CryptoBackend, CryptoError, Recipient};
use pgp::composed::{DecryptionOptions, Message, MessageBuilder, SignedPublicKey, TheRing};
use pgp::crypto::aead::{AeadAlgorithm, ChunkSize};
use pgp::crypto::sym::SymmetricKeyAlgorithm;
use pgp::packet::PublicSubkey;
use pgp::types::{KeyDetails, KeyVersion, Password};
use rand::rngs::OsRng;
use zeroize::Zeroizing;

use super::keystore::PgpKeyStore;

/// pass backend: a keystore plus the passphrase used to unlock secret keys.
pub struct PgpBackend {
    store: PgpKeyStore,
    password: Password,
}

impl PgpBackend {
    #[must_use]
    pub fn new(store: PgpKeyStore, passphrase: Option<&str>) -> Self {
        let password = match passphrase {
            Some(p) => Password::from(p),
            None => Password::empty(),
        };
        Self { store, password }
    }

    #[must_use]
    pub fn store(&self) -> &PgpKeyStore {
        &self.store
    }
}

fn encryption_subkey(pk: &SignedPublicKey) -> Option<&PublicSubkey> {
    pk.public_subkeys.first().map(|s| &s.key)
}

impl CryptoBackend for PgpBackend {
    fn encrypt(&self, plaintext: &[u8], recipients: &[Recipient]) -> Result<Vec<u8>, CryptoError> {
        if recipients.is_empty() {
            // pass always resolves a non-empty .gpg-id; empty means uninitialized.
            return Err(CryptoError::NoUsableKey);
        }

        // Resolve every recipient spec to an imported key's encryption subkey.
        let mut targets: Vec<&PublicSubkey> = Vec::with_capacity(recipients.len());
        let mut all_v6 = true;
        for spec in recipients {
            let pk = self
                .store
                .resolve(spec)
                .ok_or_else(|| CryptoError::Unavailable {
                    reason: "no imported key matches a .gpg-id recipient".into(),
                })?;
            if pk.version() != KeyVersion::V6 {
                all_v6 = false;
            }
            let sub = encryption_subkey(pk).ok_or(CryptoError::NoUsableKey)?;
            targets.push(sub);
        }

        // The builder is a typestate: SEIPD v1 and v2 are distinct types, so the
        // two paths cannot share a variable. v2 (AEAD) for a v6-only recipient
        // set; v1 + AES256 (what a v4 gpg recipient expects) otherwise.
        let mut rng = OsRng;
        if all_v6 {
            let mut builder = MessageBuilder::from_bytes("", plaintext.to_vec()).seipd_v2(
                &mut rng,
                SymmetricKeyAlgorithm::AES256,
                AeadAlgorithm::Ocb,
                ChunkSize::default(),
            );
            for sub in &targets {
                builder
                    .encrypt_to_key(&mut rng, *sub)
                    .map_err(|_| CryptoError::EncryptionFailed)?;
            }
            builder
                .to_vec(rng)
                .map_err(|_| CryptoError::EncryptionFailed)
        } else {
            let mut builder = MessageBuilder::from_bytes("", plaintext.to_vec())
                .seipd_v1(&mut rng, SymmetricKeyAlgorithm::AES256);
            for sub in &targets {
                builder
                    .encrypt_to_key(&mut rng, *sub)
                    .map_err(|_| CryptoError::EncryptionFailed)?;
            }
            builder
                .to_vec(rng)
                .map_err(|_| CryptoError::EncryptionFailed)
        }
    }

    fn decrypt(&self, ciphertext: &[u8]) -> Result<Zeroizing<Vec<u8>>, CryptoError> {
        if self.store.secret_keys.is_empty() {
            return Err(CryptoError::NoUsableKey);
        }
        let msg = Message::from_bytes(ciphertext).map_err(|_| CryptoError::DecryptionFailed)?;

        let secret_keys = self.store.secret_keys.iter().collect::<Vec<_>>();
        let passwords = vec![&self.password; secret_keys.len()];
        let ring = TheRing {
            secret_keys,
            key_passwords: passwords,
            // Modern gpg's tag-20 AEAD packet is non-standard; opt in or real
            // entries fail. (See the module note.)
            decrypt_options: DecryptionOptions::new().enable_gnupg_aead(),
            ..Default::default()
        };

        let (mut dec, _) = msg
            .decrypt_the_ring(ring, true)
            .map_err(|_| CryptoError::DecryptionFailed)?;
        if dec.is_compressed() {
            dec = dec
                .decompress()
                .map_err(|_| CryptoError::DecryptionFailed)?;
        }
        let data = dec
            .as_data_vec()
            .map_err(|_| CryptoError::DecryptionFailed)?;
        Ok(Zeroizing::new(data))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pgp::ser::Serialize as _;

    // Generate a v4 ECC key with rpgp, wire it into the backend, and confirm a
    // pass-shaped round trip through the CryptoBackend seam. Real gpg interop is
    // covered by the Phase 0 spike; this pins the wiring in the tree.
    fn gen_key() -> pgp::composed::SignedSecretKey {
        use pgp::composed::{EncryptionCaps, KeyType, SecretKeyParamsBuilder, SubkeyParamsBuilder};
        let rng = OsRng;
        let params = SecretKeyParamsBuilder::default()
            .key_type(KeyType::Ed25519)
            .can_sign(true)
            .primary_user_id("Test <t@t>".into())
            .passphrase(None)
            .subkey(
                SubkeyParamsBuilder::default()
                    .key_type(KeyType::X25519)
                    .can_encrypt(EncryptionCaps::All)
                    .passphrase(None)
                    .build()
                    .unwrap(),
            )
            .build()
            .unwrap();
        params.generate(rng).unwrap()
    }

    #[test]
    fn round_trips_through_pass_core_seam() {
        let key = gen_key();
        let fpr: String = {
            use pgp::composed::SignedPublicKey;
            let pk = SignedPublicKey::from(key.clone());
            pk.fingerprint()
                .as_bytes()
                .iter()
                .map(|b| format!("{b:02X}"))
                .collect()
        };
        let mut store = PgpKeyStore::new();
        store
            .add_secret_key_bytes(&key.to_bytes().unwrap())
            .unwrap();
        let backend = PgpBackend::new(store, None);

        let plaintext = b"correct horse battery staple\n";
        let ct = backend.encrypt(plaintext, &[fpr]).unwrap();
        let pt = backend.decrypt(&ct).unwrap();
        assert_eq!(&*pt, plaintext);
    }
}
