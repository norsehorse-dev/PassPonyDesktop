//! Imported OpenPGP key material and the recipient-resolution rule.
//!
//! The resolution rule is ported verbatim from the phones' `PgpEngine.resolve()`
//! (Android) / `PGPonyEngine.resolve()` (iOS): a `.gpg-id` spec matches a key
//! by uppercase-hex fingerprint (primary or subkey) or 16-hex key ID, tolerating
//! a `0x` prefix and inner spaces, with a fingerprint-suffix match at 16+ chars.
//! It must stay byte-identical to the phones so a store made on one device opens
//! on another.

use pgp::composed::{Deserializable, SignedPublicKey, SignedSecretKey};
use pgp::types::KeyDetails;

/// The keys PassPony holds: secret keys for decrypt, public keys for encrypt.
/// Every secret key's own public half is also available as a recipient, so
/// encrypting a copy to yourself never means importing your own public key
/// separately.
#[derive(Default)]
pub struct PgpKeyStore {
    pub secret_keys: Vec<SignedSecretKey>,
    pub public_keys: Vec<SignedPublicKey>,
}

impl PgpKeyStore {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Import a secret key (binary or armored). Its public half becomes an
    /// available recipient too.
    pub fn add_secret_key_bytes(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
        let key = SignedSecretKey::from_bytes(bytes)?;
        self.public_keys.push(SignedPublicKey::from(key.clone()));
        self.secret_keys.push(key);
        Ok(())
    }

    /// Import a public key (binary or armored) as an encryption recipient.
    pub fn add_public_key_bytes(&mut self, bytes: &[u8]) -> anyhow::Result<()> {
        let key = SignedPublicKey::from_bytes(bytes)?;
        self.public_keys.push(key);
        Ok(())
    }

    /// Resolve a `.gpg-id` spec to an imported public key, by the phones' rule.
    #[must_use]
    pub fn resolve(&self, spec: &str) -> Option<&SignedPublicKey> {
        let needle = spec.to_uppercase().replace("0X", "").replace(' ', "");
        if needle.is_empty() {
            return None;
        }
        self.public_keys
            .iter()
            .find(|pk| key_identifiers(pk).iter().any(|id| ids_match(id, &needle)))
    }

    /// Uppercase-hex primary fingerprint of each imported public key, for
    /// writing a `.gpg-id` when initializing a store.
    #[must_use]
    pub fn primary_fingerprints(&self) -> Vec<String> {
        self.public_keys
            .iter()
            .map(|pk| hex_up(pk.fingerprint().as_bytes()))
            .collect()
    }
}

/// Uppercase-hex fingerprints of the primary key and every subkey. A 16-hex key
/// ID is the fingerprint's tail for v4 keys, so the suffix rule in
/// [`ids_match`] subsumes it.
fn key_identifiers(pk: &SignedPublicKey) -> Vec<String> {
    let mut ids = vec![hex_up(pk.fingerprint().as_bytes())];
    for sub in &pk.public_subkeys {
        ids.push(hex_up(sub.key.fingerprint().as_bytes()));
    }
    ids
}

fn ids_match(id: &str, needle: &str) -> bool {
    id == needle
        || (id.len() == 16 && needle.ends_with(id))
        || (needle.len() >= 16 && id.ends_with(needle))
}

fn hex_up(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02X}")).collect()
}
