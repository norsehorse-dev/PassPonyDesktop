//! The passage (age) crypto backend, over the `age` crate.
//!
//! Semantics match the phones' engine (pass-ffi `age_engine.rs`, iOS
//! `AgePonyEngine.swift`) exactly: identities are one `AGE-SECRET-KEY-1…` per
//! line with blank lines and `#` comments skipped; encrypt to the resolved
//! recipient set, or fall back to the identities' own public keys when it is
//! empty (`age -e -i identities`); decrypt tries every loaded identity. Errors
//! never carry entry names or plaintext.

use std::io::{Read, Write};
use std::str::FromStr;

use age::x25519::{Identity, Recipient as X25519Recipient};
use age::{Decryptor, Encryptor};
use pass_core::crypto::{CryptoBackend, CryptoError, Recipient};
use zeroize::Zeroizing;

/// passage backend: the age identities this machine holds.
pub struct AgeBackend {
    identities: Vec<Identity>,
}

impl AgeBackend {
    /// Parse an identities file's text. Malformed lines are skipped, matching
    /// the phones' loader; an engine with zero identities is not an error until
    /// it is actually used.
    #[must_use]
    pub fn from_identities_text(text: &str) -> Self {
        let identities = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .filter_map(|l| Identity::from_str(l).ok())
            .collect();
        Self { identities }
    }
}

impl CryptoBackend for AgeBackend {
    fn encrypt(&self, plaintext: &[u8], recipients: &[Recipient]) -> Result<Vec<u8>, CryptoError> {
        if self.identities.is_empty() && recipients.is_empty() {
            return Err(CryptoError::NoUsableKey);
        }
        let targets: Vec<X25519Recipient> = if recipients.is_empty() {
            self.identities.iter().map(Identity::to_public).collect()
        } else {
            let mut parsed = Vec::with_capacity(recipients.len());
            for spec in recipients {
                match X25519Recipient::from_str(spec) {
                    Ok(r) => parsed.push(r),
                    Err(_) => return Err(CryptoError::EncryptionFailed),
                }
            }
            parsed
        };

        let refs: Vec<&dyn age::Recipient> =
            targets.iter().map(|r| r as &dyn age::Recipient).collect();
        let encryptor = Encryptor::with_recipients(refs.into_iter())
            .map_err(|_| CryptoError::EncryptionFailed)?;
        let mut ciphertext = Vec::with_capacity(plaintext.len());
        let mut writer = encryptor
            .wrap_output(&mut ciphertext)
            .map_err(|_| CryptoError::EncryptionFailed)?;
        writer
            .write_all(plaintext)
            .map_err(|_| CryptoError::EncryptionFailed)?;
        writer.finish().map_err(|_| CryptoError::EncryptionFailed)?;
        Ok(ciphertext)
    }

    fn decrypt(&self, ciphertext: &[u8]) -> Result<Zeroizing<Vec<u8>>, CryptoError> {
        if self.identities.is_empty() {
            return Err(CryptoError::NoUsableKey);
        }
        let decryptor = Decryptor::new(ciphertext).map_err(|_| CryptoError::DecryptionFailed)?;
        let refs: Vec<&dyn age::Identity> = self
            .identities
            .iter()
            .map(|i| i as &dyn age::Identity)
            .collect();
        let mut reader = decryptor
            .decrypt(refs.into_iter())
            .map_err(|_| CryptoError::DecryptionFailed)?;
        let mut plaintext = Zeroizing::new(Vec::new());
        reader
            .read_to_end(&mut plaintext)
            .map_err(|_| CryptoError::DecryptionFailed)?;
        Ok(plaintext)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use age::secrecy::ExposeSecret;

    #[test]
    fn round_trips_through_pass_core_seam() {
        let id = Identity::generate();
        let recipient = id.to_public().to_string();
        let backend = AgeBackend::from_identities_text(id.to_string().expose_secret());

        let plaintext = b"web/github.com\nlogin: kevin\n";
        let ct = backend.encrypt(plaintext, &[recipient]).unwrap();
        let pt = backend.decrypt(&ct).unwrap();
        assert_eq!(&*pt, plaintext);
    }
}
