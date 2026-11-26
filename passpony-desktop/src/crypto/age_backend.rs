//! The passage (age) crypto backend, over the `age` crate.
//!
//! Semantics match the phones' engine (pass-ffi `age_engine.rs`, iOS
//! `AgePonyEngine.swift`) exactly: identities are one `AGE-SECRET-KEY-1…` per
//! line with blank lines and `#` comments skipped; encrypt to the resolved
//! recipient set, or fall back to the identities' own public keys when it is
//! empty (`age -e -i identities`); decrypt tries every loaded identity. Errors
//! never carry entry names or plaintext.
//!
//! Recipients may also be YubiKey keys made by age-plugin-yubikey
//! (`age1yubikey1…`): encrypting to them needs neither the plugin nor the key
//! (PassPonyCore's `pass-age-piv`). Decrypting with one goes through the
//! plugin itself: the identities file's `AGE-PLUGIN-YUBIKEY-1…` stub lines are
//! kept, and when no plain identity opens a file, `age-plugin-yubikey` (on
//! PATH) is asked, with its PIN and touch prompts sent to a
//! [`PluginPrompter`]. Without a prompter the stubs are ignored.

use std::collections::HashSet;
use std::io::{Read, Write};
use std::str::FromStr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use age::secrecy::{ExposeSecret, SecretString};
use age::x25519::{Identity, Recipient as X25519Recipient};
use age::{Decryptor, Encryptor};
use age_core::format::{FileKey, Stanza};
use pass_core::crypto::{CryptoBackend, CryptoError, Recipient};
use zeroize::Zeroizing;

/// Where an age plugin's prompts go while it decrypts: called on the thread
/// that is decrypting, and each call may block until the user answers.
pub trait PluginPrompter: Send + Sync {
    /// Something to show, such as "touch your YubiKey".
    fn message(&self, text: &str);
    /// A yes/no question (age-plugin-yubikey: is the YubiKey plugged in, or
    /// skip it). `None` when the user dismissed it.
    fn confirm(&self, text: &str, yes: &str, no: Option<&str>) -> Option<bool>;
    /// A secret, such as the PIV PIN. `None` when the user cancelled.
    fn secret(&self, description: &str) -> Option<SecretString>;
}

/// passage backend: the age identities this machine holds.
pub struct AgeBackend {
    identities: Vec<Identity>,
    plugin_identities: Vec<age::plugin::Identity>,
    prompter: Option<Arc<dyn PluginPrompter>>,
}

impl AgeBackend {
    /// Parse an identities file's text. Malformed lines are skipped, matching
    /// the phones' loader; an engine with zero identities is not an error until
    /// it is actually used. Plugin identity lines (`AGE-PLUGIN-…`) are kept
    /// for [`AgeBackend::with_prompter`].
    #[must_use]
    pub fn from_identities_text(text: &str) -> Self {
        let lines: Vec<&str> = text
            .lines()
            .map(str::trim)
            .filter(|l| !l.is_empty() && !l.starts_with('#'))
            .collect();
        let identities = lines
            .iter()
            .filter_map(|l| Identity::from_str(l).ok())
            .collect();
        let plugin_identities = lines
            .iter()
            .filter(|l| l.starts_with("AGE-PLUGIN-"))
            .filter_map(|l| age::plugin::Identity::from_str(l).ok())
            .collect();
        Self {
            identities,
            plugin_identities,
            prompter: None,
        }
    }

    /// Lets decrypt use the plugin identities, sending their prompts to
    /// `prompter`.
    #[must_use]
    pub fn with_prompter(mut self, prompter: Arc<dyn PluginPrompter>) -> Self {
        self.prompter = Some(prompter);
        self
    }

    /// Whether the identities include any plugin (YubiKey) identity: then a
    /// decrypt may block on the user, so it belongs off the UI thread.
    #[must_use]
    pub fn has_plugin_identities(&self) -> bool {
        !self.plugin_identities.is_empty()
    }

    /// The plugin identities' plugins, one per plugin name, sharing `prompts`.
    /// YubiKeys that are plugged in go alone when there are any, so the plugin
    /// doesn't ask for the others. A plugin whose binary is not on PATH is left
    /// out and named in the second list.
    fn plugins(
        &self,
        prompts: &Prompts,
    ) -> (Vec<age::plugin::IdentityPluginV1<Prompts>>, Vec<String>) {
        let identities = match connected_yubikey_stubs() {
            Some(connected) => prefer_connected(&self.plugin_identities, &connected),
            None => self.plugin_identities.clone(),
        };
        let mut names: Vec<&str> = identities.iter().map(|i| i.plugin()).collect();
        names.sort_unstable();
        names.dedup();
        let mut plugins = Vec::new();
        let mut missing = Vec::new();
        for name in names {
            match age::plugin::IdentityPluginV1::new(name, &identities, prompts.clone()) {
                Ok(p) => plugins.push(p),
                Err(_) => missing.push(format!("age-plugin-{name}")),
            }
        }
        (plugins, missing)
    }
}

/// A [`PluginPrompter`] as the age crate's plugin callbacks, noting whether
/// the user cancelled a prompt.
#[derive(Clone)]
struct Prompts {
    prompter: Arc<dyn PluginPrompter>,
    cancelled: Arc<AtomicBool>,
}

impl age::Callbacks for Prompts {
    fn display_message(&self, message: &str) {
        self.prompter.message(message);
    }

    fn confirm(&self, message: &str, yes_string: &str, no_string: Option<&str>) -> Option<bool> {
        let answer = self.prompter.confirm(message, yes_string, no_string);
        if answer.is_none() {
            self.cancelled.store(true, Ordering::Relaxed);
        }
        answer
    }

    fn request_public_string(&self, _description: &str) -> Option<String> {
        None
    }

    fn request_passphrase(&self, description: &str) -> Option<SecretString> {
        let answer = self.prompter.secret(description);
        if answer.is_none() {
            self.cancelled.store(true, Ordering::Relaxed);
        }
        answer
    }
}

impl CryptoBackend for AgeBackend {
    fn encrypt(&self, plaintext: &[u8], recipients: &[Recipient]) -> Result<Vec<u8>, CryptoError> {
        if self.identities.is_empty() && recipients.is_empty() {
            return Err(CryptoError::NoUsableKey);
        }
        let targets: Vec<Box<dyn age::Recipient>> = if recipients.is_empty() {
            self.identities
                .iter()
                .map(|i| Box::new(i.to_public()) as Box<dyn age::Recipient>)
                .collect()
        } else {
            recipients
                .iter()
                .map(|spec| parse_recipient(spec))
                .collect::<Result<_, _>>()?
        };

        let refs: Vec<&dyn age::Recipient> = targets.iter().map(AsRef::as_ref).collect();
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
        let use_plugins = self.prompter.is_some() && !self.plugin_identities.is_empty();
        if self.identities.is_empty() && !use_plugins {
            return Err(CryptoError::NoUsableKey);
        }
        let prompts = self.prompter.clone().map(|prompter| Prompts {
            prompter,
            cancelled: Arc::new(AtomicBool::new(false)),
        });
        // Plain identities first, on their own: the YubiKey (and its plugin) is
        // asked only when nothing on this machine opens the file.
        if !self.identities.is_empty() {
            let decryptor =
                Decryptor::new(ciphertext).map_err(|_| CryptoError::DecryptionFailed)?;
            let refs = self.identities.iter().map(|i| i as &dyn age::Identity);
            match decryptor.decrypt(refs) {
                Ok(mut reader) => {
                    let mut plaintext = Zeroizing::new(Vec::new());
                    reader
                        .read_to_end(&mut plaintext)
                        .map_err(|_| CryptoError::DecryptionFailed)?;
                    return Ok(plaintext);
                }
                Err(_) if use_plugins => {}
                Err(_) => return Err(CryptoError::DecryptionFailed),
            }
        }
        let (plugins, missing) = match &prompts {
            Some(p) => self.plugins(p),
            None => (Vec::new(), Vec::new()),
        };
        let decryptor = Decryptor::new(ciphertext).map_err(|_| CryptoError::DecryptionFailed)?;
        let refs: Vec<&dyn age::Identity> =
            plugins.iter().map(|p| p as &dyn age::Identity).collect();
        let mut reader = decryptor.decrypt(refs.into_iter()).map_err(|_| {
            if prompts
                .as_ref()
                .is_some_and(|p| p.cancelled.load(Ordering::Relaxed))
            {
                CryptoError::Cancelled
            } else if plugins.is_empty() && !missing.is_empty() {
                CryptoError::Unavailable {
                    reason: format!("{} is not installed", missing.join(", ")),
                }
            } else {
                CryptoError::DecryptionFailed
            }
        })?;
        let mut plaintext = Zeroizing::new(Vec::new());
        reader
            .read_to_end(&mut plaintext)
            .map_err(|_| CryptoError::DecryptionFailed)?;
        Ok(plaintext)
    }
}

/// The stubs of the YubiKeys plugged in now, as age-plugin-yubikey lists them
/// (`--identity` reads the cards' slots and needs no PIN or touch). `None`
/// when the plugin can't be run.
fn connected_yubikey_stubs() -> Option<Vec<String>> {
    let out = std::process::Command::new("age-plugin-yubikey")
        .arg("--identity")
        .stdin(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    Some(
        text.lines()
            .map(str::trim)
            .filter(|l| l.starts_with("AGE-PLUGIN-YUBIKEY-"))
            .map(str::to_owned)
            .collect(),
    )
}

/// The YubiKey identities among `identities` that are in `connected`, plus
/// every other plugin's identities; all of them unchanged when none of the
/// YubiKeys is connected, so the plugin asks for one to be inserted.
fn prefer_connected(
    identities: &[age::plugin::Identity],
    connected: &[String],
) -> Vec<age::plugin::Identity> {
    let is_connected = |i: &age::plugin::Identity| {
        let s = i.to_string();
        connected.iter().any(|c| c.eq_ignore_ascii_case(&s))
    };
    if !identities
        .iter()
        .any(|i| i.plugin() == "yubikey" && is_connected(i))
    {
        return identities.to_vec();
    }
    identities
        .iter()
        .filter(|i| i.plugin() != "yubikey" || is_connected(i))
        .cloned()
        .collect()
}

/// One `.age-recipients` line: an X25519 key or a YubiKey key. Any parse
/// failure is a generic `EncryptionFailed`, as on the phones.
fn parse_recipient(spec: &str) -> Result<Box<dyn age::Recipient>, CryptoError> {
    if pass_age_piv::is_piv_recipient(spec) {
        pass_age_piv::PivRecipient::from_str(spec)
            .map(|r| Box::new(YubiKeyRecipient(r)) as Box<dyn age::Recipient>)
            .map_err(|_| CryptoError::EncryptionFailed)
    } else {
        X25519Recipient::from_str(spec)
            .map(|r| Box::new(r) as Box<dyn age::Recipient>)
            .map_err(|_| CryptoError::EncryptionFailed)
    }
}

/// [`pass_age_piv::PivRecipient`] as an age recipient: one `piv-p256` stanza,
/// no labels (it mixes with X25519, as with the plugin).
struct YubiKeyRecipient(pass_age_piv::PivRecipient);

impl age::Recipient for YubiKeyRecipient {
    fn wrap_file_key(
        &self,
        file_key: &FileKey,
    ) -> Result<(Vec<Stanza>, HashSet<String>), age::EncryptError> {
        let s = self.0.wrap(file_key.expose_secret());
        Ok((
            vec![Stanza {
                tag: pass_age_piv::STANZA_TAG.to_owned(),
                args: s.args(),
                body: s.body(),
            }],
            HashSet::new(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use p256::elliptic_curve::sec1::ToEncodedPoint;

    /// A store whose recipients mix a YubiKey and this machine: saving works,
    /// and this machine still reads what it saved. The stanza itself is checked
    /// against age-plugin-yubikey in pass-age-piv's tests.
    #[test]
    fn encrypts_to_yubikey_and_x25519_recipients() {
        let id = Identity::generate();
        let sk = p256::SecretKey::random(&mut p256::elliptic_curve::rand_core::OsRng);
        let yk = pass_age_piv::PivRecipient::from_compressed(
            sk.public_key().to_encoded_point(true).as_bytes(),
        )
        .unwrap();
        let backend = AgeBackend::from_identities_text(id.to_string().expose_secret());

        let plaintext = b"shared with the yubikey\n";
        let ct = backend
            .encrypt(plaintext, &[yk.to_string(), id.to_public().to_string()])
            .unwrap();
        assert!(ct.windows(12).any(|w| w == b"-> piv-p256 "));
        assert_eq!(&*backend.decrypt(&ct).unwrap(), plaintext);
    }

    /// A YubiKey (recipient, stub) for a software key, with a made-up serial.
    fn fake_yubikey(serial: u32) -> (pass_age_piv::PivRecipient, String) {
        let sk = p256::SecretKey::random(&mut p256::elliptic_curve::rand_core::OsRng);
        let r = pass_age_piv::PivRecipient::from_sec1(
            sk.public_key().to_encoded_point(false).as_bytes(),
        )
        .unwrap();
        let stub = pass_age_piv::PivStub::new(serial, 0x82, &r)
            .unwrap()
            .to_string();
        (r, stub)
    }

    /// Records prompts and answers every one with "no thanks".
    #[derive(Default)]
    struct Dismissing {
        asked: std::sync::Mutex<Vec<String>>,
    }

    impl PluginPrompter for Dismissing {
        fn message(&self, _text: &str) {}
        fn confirm(&self, text: &str, _yes: &str, _no: Option<&str>) -> Option<bool> {
            self.asked.lock().unwrap().push(text.to_owned());
            None
        }
        fn secret(&self, description: &str) -> Option<SecretString> {
            self.asked.lock().unwrap().push(description.to_owned());
            None
        }
    }

    #[test]
    fn keeps_yubikey_stubs_but_needs_a_prompter() {
        let (yk, stub) = fake_yubikey(12_345_678);
        let writer =
            AgeBackend::from_identities_text(Identity::generate().to_string().expose_secret());
        let ct = writer.encrypt(b"x", &[yk.to_string()]).unwrap();

        let backend = AgeBackend::from_identities_text(&format!("# yk\n{stub}\n"));
        assert!(backend.has_plugin_identities());
        assert!(matches!(
            backend.decrypt(&ct).unwrap_err(),
            CryptoError::NoUsableKey
        ));
    }

    #[test]
    fn plain_identity_opens_without_asking() {
        let (yk, stub) = fake_yubikey(12_345_678);
        let id = Identity::generate();
        let text = format!("{stub}\n{}\n", id.to_string().expose_secret());
        let prompter = Arc::new(Dismissing::default());
        let backend = AgeBackend::from_identities_text(&text).with_prompter(prompter.clone());
        let ct = backend
            .encrypt(b"pw\n", &[yk.to_string(), id.to_public().to_string()])
            .unwrap();
        assert_eq!(&*backend.decrypt(&ct).unwrap(), b"pw\n");
        assert!(prompter.asked.lock().unwrap().is_empty());
    }

    #[test]
    fn a_missing_plugin_is_unavailable() {
        let stub = age::plugin::Identity::default_for_plugin("passpony-test-nosuchplugin")
            .unwrap()
            .to_string();
        let (yk, _) = fake_yubikey(1);
        let writer =
            AgeBackend::from_identities_text(Identity::generate().to_string().expose_secret());
        let ct = writer.encrypt(b"x", &[yk.to_string()]).unwrap();
        let backend =
            AgeBackend::from_identities_text(&stub).with_prompter(Arc::new(Dismissing::default()));
        match backend.decrypt(&ct).unwrap_err() {
            CryptoError::Unavailable { reason } => {
                assert!(
                    reason.contains("age-plugin-passpony-test-nosuchplugin"),
                    "{reason}"
                );
            }
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }

    /// Through the real age-plugin-yubikey when it is installed: a YubiKey
    /// that isn't plugged in is asked for, and dismissing that is a cancel.
    /// Without the plugin, the same file is "not installed".
    #[test]
    fn yubikey_plugin_prompts_reach_the_prompter() {
        let (yk, stub) = fake_yubikey(4_000_000_001);
        let writer =
            AgeBackend::from_identities_text(Identity::generate().to_string().expose_secret());
        let ct = writer.encrypt(b"x", &[yk.to_string()]).unwrap();
        let prompter = Arc::new(Dismissing::default());
        let backend = AgeBackend::from_identities_text(&stub).with_prompter(prompter.clone());
        let installed = std::process::Command::new("sh")
            .args(["-c", "command -v age-plugin-yubikey"])
            .stdout(std::process::Stdio::null())
            .status()
            .map(|s| s.success())
            .unwrap_or(false);
        let err = backend.decrypt(&ct).unwrap_err();
        if installed {
            assert!(matches!(err, CryptoError::Cancelled), "{err:?}");
            let asked = prompter.asked.lock().unwrap();
            assert!(asked.iter().any(|q| q.contains("4000000001")), "{asked:?}");
        } else {
            assert!(matches!(err, CryptoError::Unavailable { .. }), "{err:?}");
        }
    }

    #[test]
    fn plugged_in_yubikeys_go_alone() {
        let (_, a) = fake_yubikey(1);
        let (_, b) = fake_yubikey(2);
        let other = age::plugin::Identity::default_for_plugin("other").unwrap();
        let ids: Vec<age::plugin::Identity> = [a.as_str(), b.as_str()]
            .iter()
            .map(|s| age::plugin::Identity::from_str(s).unwrap())
            .chain(std::iter::once(other.clone()))
            .collect();

        let only_b = prefer_connected(&ids, &[b.to_lowercase()]);
        let names: Vec<String> = only_b.iter().map(ToString::to_string).collect();
        assert_eq!(names, vec![b.clone(), other.to_string()]);

        // None of ours plugged in (or someone else's): ask for all of them.
        let (_, stranger) = fake_yubikey(3);
        assert_eq!(prefer_connected(&ids, &[stranger]).len(), 3);
        assert_eq!(prefer_connected(&ids, &[]).len(), 3);
    }

    #[test]
    fn malformed_yubikey_recipient_is_encryption_failed() {
        let id = Identity::generate();
        let backend = AgeBackend::from_identities_text(id.to_string().expose_secret());
        let err = backend
            .encrypt(b"x", &["age1yubikey1qqqqqq".to_owned()])
            .unwrap_err();
        assert!(matches!(err, CryptoError::EncryptionFailed));
    }

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
