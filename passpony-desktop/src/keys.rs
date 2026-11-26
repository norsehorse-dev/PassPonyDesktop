//! On-disk key and identity material, and the loaders that turn it into
//! backends.
//!
//! Everything lives in the OS config directory, owner-only on Unix. Mirrors the
//! phones' split: imported OpenPGP key files (`PgpKeyStore.kt` /
//! `PgpKeyStore.swift`) for pass, and a single age identities file
//! (`AgeIdentityStore`) for passage.
//!
//! - `<config>/pgp-keys/*`   one OpenPGP key per file (public or secret,
//!   armored or binary). Every file is offered to the keystore on load.
//! - `<config>/identities`   age identities, one `AGE-SECRET-KEY-1…` per line,
//!   blank lines and `#` comments ignored.

use std::fs;
use std::path::{Path, PathBuf};

use age::secrecy::ExposeSecret;
use age::x25519::Identity;

use crate::crypto::{AgeBackend, PgpBackend, PgpKeyStore};

/// Where PassPony keeps its keys. `None` only if no config dir can be located.
#[must_use]
pub fn config_dir() -> Option<PathBuf> {
    directories::ProjectDirs::from("dev", "NorseHorse", "PassPony")
        .map(|d| d.config_dir().to_path_buf())
}

fn require_config_dir() -> anyhow::Result<PathBuf> {
    config_dir().ok_or_else(|| anyhow::anyhow!("cannot locate a config directory"))
}

/// `<config>/pgp-keys`.
pub fn pgp_keys_dir() -> anyhow::Result<PathBuf> {
    Ok(require_config_dir()?.join("pgp-keys"))
}

/// `<config>/identities`.
pub fn identities_file() -> anyhow::Result<PathBuf> {
    Ok(require_config_dir()?.join("identities"))
}

// --- pass (OpenPGP) ---------------------------------------------------------

/// Import an OpenPGP key file (public or secret, armored or binary) into the
/// keys directory. The bytes are validated by round-tripping them through the
/// keystore before they are written, so an unreadable file is rejected here
/// rather than silently on the next load.
pub fn import_pgp_key(path: &Path) -> anyhow::Result<PathBuf> {
    let bytes = fs::read(path)?;
    // Validate: it must parse as either a secret or a public key.
    let mut probe = PgpKeyStore::new();
    if probe.add_secret_key_bytes(&bytes).is_err() {
        probe
            .add_public_key_bytes(&bytes)
            .map_err(|_| anyhow::anyhow!("not a readable OpenPGP key file"))?;
    }

    let dir = pgp_keys_dir()?;
    fs::create_dir_all(&dir)?;
    restrict_dir(&dir)?;
    let name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "imported.key".into());
    let dest = unique_path(&dir, &name);
    fs::write(&dest, &bytes)?;
    restrict_file(&dest)?;
    Ok(dest)
}

/// Load every key in the keys directory into one keystore. Missing directory is
/// not an error: it yields an empty keystore, which fails only when used.
pub fn load_pgp_keystore() -> anyhow::Result<PgpKeyStore> {
    let mut store = PgpKeyStore::new();
    let dir = pgp_keys_dir()?;
    if !dir.exists() {
        return Ok(store);
    }
    for entry in fs::read_dir(&dir)? {
        let path = entry?.path();
        if !path.is_file() {
            continue;
        }
        let bytes = fs::read(&path)?;
        // A file is a secret key or a public key; try secret first.
        if store.add_secret_key_bytes(&bytes).is_err() {
            let _ = store.add_public_key_bytes(&bytes);
        }
    }
    Ok(store)
}

/// Build the pass backend from disk with an optional passphrase.
pub fn load_pgp_backend(passphrase: Option<&str>) -> anyhow::Result<PgpBackend> {
    Ok(PgpBackend::new(load_pgp_keystore()?, passphrase))
}

/// Build whichever backend a store format needs, as the `Backend` enum that
/// hands pass-core a `&dyn CryptoBackend`.
pub fn load_backend(
    format: pass_core::store::StoreFormat,
    passphrase: Option<&str>,
) -> anyhow::Result<crate::crypto::Backend> {
    use pass_core::store::StoreFormat;
    Ok(match format {
        StoreFormat::Pass => crate::crypto::Backend::Pass(load_pgp_backend(passphrase)?),
        StoreFormat::Passage => crate::crypto::Backend::Passage(load_age_backend()?),
    })
}

/// Names of the imported key files, for listing in the UI/CLI.
pub fn list_pgp_key_files() -> anyhow::Result<Vec<String>> {
    let dir = pgp_keys_dir()?;
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut names = Vec::new();
    for entry in fs::read_dir(&dir)? {
        let path = entry?.path();
        if path.is_file() {
            if let Some(n) = path.file_name() {
                names.push(n.to_string_lossy().into_owned());
            }
        }
    }
    names.sort();
    Ok(names)
}

// --- passage (age) ----------------------------------------------------------

/// Load the age backend from the identities file. Missing file yields an empty
/// backend (usable only once identities exist), matching the phones.
pub fn load_age_backend() -> anyhow::Result<AgeBackend> {
    let file = identities_file()?;
    let text = if file.exists() {
        fs::read_to_string(&file)?
    } else {
        String::new()
    };
    Ok(AgeBackend::from_identities_text(&text))
}

/// Generate a new age identity and append it to the identities file, creating
/// the file (owner-only) on first use. Returns the public recipient string.
pub fn generate_age_identity() -> anyhow::Result<String> {
    generate_age_identity_at(&identities_file()?)
}

fn generate_age_identity_at(file: &Path) -> anyhow::Result<String> {
    let identity = Identity::generate();
    let recipient = identity.to_public().to_string();

    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
        restrict_dir(parent)?;
    }
    let mut text = if file.exists() {
        fs::read_to_string(file)?
    } else {
        String::from("# PassPony age identities\n")
    };
    if !text.ends_with('\n') {
        text.push('\n');
    }
    text.push_str(&format!("# public key: {recipient}\n"));
    text.push_str(identity.to_string().expose_secret());
    text.push('\n');
    fs::write(file, text)?;
    restrict_file(file)?;
    Ok(recipient)
}

/// Add the age identities in `source` (an identities file: X25519
/// `AGE-SECRET-KEY-1…` lines and plugin stubs such as age-plugin-yubikey's
/// `AGE-PLUGIN-YUBIKEY-1…`) to the identities file, skipping ones already
/// there. Returns how many were added; other lines are ignored.
pub fn import_age_identities(source: &Path) -> anyhow::Result<usize> {
    import_age_identities_at(source, &identities_file()?)
}

fn import_age_identities_at(source: &Path, file: &Path) -> anyhow::Result<usize> {
    use std::str::FromStr;
    let incoming = zeroize::Zeroizing::new(fs::read_to_string(source)?);
    if let Some(parent) = file.parent() {
        fs::create_dir_all(parent)?;
        restrict_dir(parent)?;
    }
    let mut text = zeroize::Zeroizing::new(if file.exists() {
        fs::read_to_string(file)?
    } else {
        String::from("# PassPony age identities\n")
    });
    let mut added = 0;
    for line in incoming.lines().map(str::trim) {
        let usable = Identity::from_str(line).is_ok()
            || (line.starts_with("AGE-PLUGIN-") && age::plugin::Identity::from_str(line).is_ok());
        if !usable || text.lines().any(|l| l.trim() == line) {
            continue;
        }
        if !text.ends_with('\n') {
            text.push('\n');
        }
        text.push_str(&format!("# imported from {}\n{line}\n", source.display()));
        added += 1;
    }
    if added > 0 {
        fs::write(file, text.as_bytes())?;
        restrict_file(file)?;
    }
    Ok(added)
}

/// Generate an age identity only if the identities file has none yet, matching
/// the phones' auto-generate-on-first-run. Returns the new recipient string if
/// one was created, or `None` if identities already existed.
pub fn ensure_age_identity() -> anyhow::Result<Option<String>> {
    ensure_age_identity_at(&identities_file()?)
}

fn ensure_age_identity_at(file: &Path) -> anyhow::Result<Option<String>> {
    let has_identity = file.exists()
        && std::fs::read_to_string(file)?
            .lines()
            .any(|l| l.trim().starts_with("AGE-SECRET-KEY-"));
    if has_identity {
        return Ok(None);
    }
    let recipient = generate_age_identity_at(file)?;
    Ok(Some(recipient))
}

/// Whether an age identity exists on disk.
#[must_use]
pub fn has_age_identity() -> bool {
    identities_file()
        .ok()
        .filter(|f| f.exists())
        .and_then(|f| std::fs::read_to_string(f).ok())
        .map(|t| t.lines().any(|l| l.trim().starts_with("AGE-SECRET-KEY-")))
        .unwrap_or(false)
}

/// Copy the age identities file to a user-chosen destination, for backup.
pub fn export_identities(dest: &Path) -> anyhow::Result<()> {
    let src = identities_file()?;
    if !src.exists() {
        anyhow::bail!("no age identity to export yet");
    }
    std::fs::copy(&src, dest)?;
    restrict_file(dest)?;
    Ok(())
}

/// Primary fingerprint of the first imported pass key, for initializing a store.
pub fn first_pgp_fingerprint() -> anyhow::Result<Option<String>> {
    Ok(load_pgp_keystore()?
        .primary_fingerprints()
        .into_iter()
        .next())
}

/// Initialize a new pass store at `root`: make it a git repo, write a
/// newline-terminated `.gpg-id` (pass-core drops a final unterminated line), and
/// commit it.
pub fn init_pass_store(root: &Path, recipient: &str) -> anyhow::Result<()> {
    use pass_core::git::{messages, GitStore};
    use pass_core::store::StoreFormat;
    std::fs::create_dir_all(root)?;
    let git = GitStore::init(root, StoreFormat::Pass)?;
    std::fs::write(root.join(".gpg-id"), format!("{recipient}\n"))?;
    git.commit_paths(&[".gpg-id"], &messages::configure())?;
    Ok(())
}

/// Initialize a new passage store at `root`: a git repo with no
/// `.age-recipients`, so it encrypts to the loaded identities (the
/// identities-fallback branch), matching the phones.
pub fn init_passage_store(root: &Path) -> anyhow::Result<()> {
    use pass_core::git::GitStore;
    use pass_core::store::StoreFormat;
    std::fs::create_dir_all(root)?;
    GitStore::init(root, StoreFormat::Passage)?;
    Ok(())
}

// --- first-run markers ------------------------------------------------------

fn marker(name: &str) -> anyhow::Result<PathBuf> {
    Ok(require_config_dir()?.join(name))
}

fn marker_set(name: &str) {
    if let Ok(path) = marker(name) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(&path, b"");
    }
}

fn marker_exists(name: &str) -> bool {
    marker(name).map(|p| p.exists()).unwrap_or(false)
}

/// Has the first-run tour been completed?
#[must_use]
pub fn is_onboarded() -> bool {
    marker_exists("onboarded")
}
pub fn mark_onboarded() {
    marker_set("onboarded");
}

/// Has the user exported/backed up their age identity at least once?
#[must_use]
pub fn identity_backed_up() -> bool {
    marker_exists("identity_backed_up")
}
pub fn mark_identity_backed_up() {
    marker_set("identity_backed_up");
}

// --- persisted preferences --------------------------------------------------

fn read_pref(name: &str) -> Option<String> {
    let raw = std::fs::read_to_string(marker(name).ok()?).ok()?;
    let v = raw.trim().to_string();
    (!v.is_empty()).then_some(v)
}

fn write_pref(name: &str, value: &str) {
    if let Ok(path) = marker(name) {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let _ = std::fs::write(path, value);
    }
}

/// The saved language code, if any (e.g. "en", "pt-BR").
#[must_use]
pub fn load_language() -> Option<String> {
    read_pref("language")
}
pub fn save_language(code: &str) {
    write_pref("language", code);
}

/// The saved theme choice ("auto" | "light" | "dark"), if any.
#[must_use]
pub fn load_theme() -> Option<String> {
    read_pref("theme")
}
pub fn save_theme(choice: &str) {
    write_pref("theme", choice);
}

/// Delete all key material: the imported pass keys and the age identities.
/// Preferences (language, theme, onboarded) are left alone.
pub fn panic_wipe() -> anyhow::Result<()> {
    if let Ok(dir) = pgp_keys_dir() {
        if dir.exists() {
            std::fs::remove_dir_all(&dir)?;
        }
    }
    if let Ok(file) = identities_file() {
        if file.exists() {
            std::fs::remove_file(&file)?;
        }
    }
    // The identity is gone, so its "backed up" marker no longer means anything.
    if let Ok(m) = marker("identity_backed_up") {
        let _ = std::fs::remove_file(m);
    }
    Ok(())
}

// --- permissions ------------------------------------------------------------

#[cfg(unix)]
fn restrict_file(path: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(unix)]
fn restrict_dir(path: &Path) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o700))?;
    Ok(())
}

#[cfg(not(unix))]
fn restrict_file(_path: &Path) -> anyhow::Result<()> {
    Ok(())
}

#[cfg(not(unix))]
fn restrict_dir(_path: &Path) -> anyhow::Result<()> {
    Ok(())
}

fn unique_path(dir: &Path, name: &str) -> PathBuf {
    let candidate = dir.join(name);
    if !candidate.exists() {
        return candidate;
    }
    for n in 1..1000 {
        let alt = dir.join(format!("{n}-{name}"));
        if !alt.exists() {
            return alt;
        }
    }
    dir.join(format!("dup-{name}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use pass_core::crypto::CryptoBackend;

    // Point the config dir at a temp HOME and drive the whole on-disk key layer:
    // generate an age identity, import a generated PGP key, load both backends,
    // and round-trip through each. This is the only test that touches process
    // env; nothing else in the crate does, so it is safe under the test harness.
    #[test]
    fn key_layer_round_trips_from_disk() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::remove_var("XDG_CONFIG_HOME");
        std::env::set_var("HOME", tmp.path());

        // passage: generate an identity, then self-encrypt/decrypt through it.
        let _recipient = generate_age_identity().unwrap();
        let age = load_age_backend().unwrap();
        let msg = b"age from disk\n";
        let ct = age.encrypt(msg, &[]).unwrap();
        assert_eq!(&*age.decrypt(&ct).unwrap(), msg);

        // pass: generate a key, write it to a file, import it, load, round-trip.
        let (key_bytes, fpr) = gen_pgp_key();
        let key_file = tmp.path().join("k.pgp");
        fs::write(&key_file, &key_bytes).unwrap();
        let dest = import_pgp_key(&key_file).unwrap();
        assert!(dest.exists());
        assert_eq!(list_pgp_key_files().unwrap().len(), 1);

        let pgp = load_pgp_backend(None).unwrap();
        let pmsg = b"pgp from disk\n";
        let pct = pgp.encrypt(pmsg, &[fpr]).unwrap();
        assert_eq!(&*pgp.decrypt(&pct).unwrap(), pmsg);
    }

    fn gen_pgp_key() -> (Vec<u8>, String) {
        use pgp::composed::{
            EncryptionCaps, KeyType, SecretKeyParamsBuilder, SignedPublicKey, SubkeyParamsBuilder,
        };
        use pgp::ser::Serialize as _;
        use pgp::types::KeyDetails;
        let rng = rand::rngs::OsRng;
        let params = SecretKeyParamsBuilder::default()
            .key_type(KeyType::Ed25519)
            .can_sign(true)
            .primary_user_id("disk <d@d>".into())
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
        let key = params.generate(rng).unwrap();
        let fpr = SignedPublicKey::from(key.clone())
            .fingerprint()
            .as_bytes()
            .iter()
            .map(|b| format!("{b:02X}"))
            .collect();
        (key.to_bytes().unwrap(), fpr)
    }

    // Env-free: these hit explicit paths, not the global config dir, so they do
    // not race the HOME-mutating test above.

    #[test]
    fn imports_plain_and_yubikey_identities_once() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("identities");
        let src = dir.path().join("from-passage");
        let id = Identity::generate();
        let stub = age::plugin::Identity::default_for_plugin("yubikey")
            .unwrap()
            .to_string();
        std::fs::write(
            &src,
            format!(
                "# a comment\n{}\n\nnot an identity\n{stub}\n",
                id.to_string().expose_secret()
            ),
        )
        .unwrap();
        assert_eq!(import_age_identities_at(&src, &file).unwrap(), 2);
        assert_eq!(import_age_identities_at(&src, &file).unwrap(), 0);
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains(&stub));
        assert!(!text.contains("not an identity"));
        let backend = AgeBackend::from_identities_text(&text);
        assert!(backend.has_plugin_identities());
    }

    #[test]
    fn ensure_age_identity_generates_once() {
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("identities");
        let first = ensure_age_identity_at(&file).unwrap();
        assert!(first.is_some(), "first call generates");
        assert!(file.exists());
        let second = ensure_age_identity_at(&file).unwrap();
        assert!(second.is_none(), "second call is a no-op");
    }

    #[test]
    fn init_pass_store_is_usable_and_gpg_id_is_terminated() {
        use crate::crypto::{PgpBackend, PgpKeyStore};
        use pass_core::entry::Entry;
        use pass_core::store::{Store, StoreFormat};

        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("store");
        let (key_bytes, fpr) = gen_pgp_key();

        init_pass_store(&root, &fpr).unwrap();
        let gpg_id = fs::read_to_string(root.join(".gpg-id")).unwrap();
        assert!(gpg_id.ends_with('\n'), ".gpg-id must be newline-terminated");

        let mut ks = PgpKeyStore::new();
        ks.add_secret_key_bytes(&key_bytes).unwrap();
        let backend = PgpBackend::new(ks, None);
        let store = Store::open(&root, StoreFormat::Pass).unwrap();
        let entry = Entry::from_password(b"hunter2");
        store.write_entry("web/x", &entry, &backend).unwrap();
        let got = store.read_entry("web/x", &backend).unwrap();
        assert_eq!(got.password(), b"hunter2");
    }

    #[test]
    fn primary_fingerprints_lists_imported_keys() {
        use crate::crypto::PgpKeyStore;
        let (key_bytes, fpr) = gen_pgp_key();
        let mut ks = PgpKeyStore::new();
        ks.add_secret_key_bytes(&key_bytes).unwrap();
        assert_eq!(ks.primary_fingerprints(), vec![fpr]);
    }
}
