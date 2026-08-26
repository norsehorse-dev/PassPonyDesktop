//! Store-mutating operations: add/edit/remove/move/copy and re-encrypt, each
//! wrapping the pass-core write and then committing with the same message the
//! `pass`/`passage` CLIs use, so a PassPony-written store stays interoperable.
//!
//! Non-GUI so it is testable against a temp git store. Commits are best-effort:
//! a store that is not a git repository is written but not committed, reported
//! back as [`Commit::NotARepo`] rather than an error.

use std::path::Path;

use pass_core::crypto::CryptoBackend;
use pass_core::entry::Entry;
use pass_core::git::{messages, GitStore};
use pass_core::store::Store;

/// What happened to the git side of an operation.
#[derive(Debug, PartialEq, Eq)]
pub enum Commit {
    Committed,
    NotARepo,
    Failed(String),
}

fn entry_rel(store: &Store, name: &str) -> String {
    format!("{name}.{}", store.format().entry_extension())
}

fn commit(root: &Path, paths: &[&str], message: &str) -> Commit {
    match GitStore::open(root) {
        Ok(git) => match git.commit_paths(paths, message) {
            Ok(_) => Commit::Committed,
            Err(e) => Commit::Failed(e.to_string()),
        },
        Err(_) => Commit::NotARepo,
    }
}

/// Write a new entry or replace an existing one, then commit. `is_new` and
/// `generated` only pick the commit message; the write is the same.
pub fn save_entry(
    store: &Store,
    name: &str,
    entry: &Entry,
    backend: &dyn CryptoBackend,
    is_new: bool,
    generated: bool,
) -> anyhow::Result<Commit> {
    store.write_entry(name, entry, backend)?;
    let message = match (is_new, generated) {
        (true, true) => messages::add_generated(name),
        (true, false) => messages::add(name),
        (false, true) => messages::replace_generated(name),
        (false, false) => messages::edit(name),
    };
    let rel = entry_rel(store, name);
    Ok(commit(store.root(), &[&rel], &message))
}

/// Remove an entry, then commit.
pub fn remove_entry(store: &Store, name: &str) -> anyhow::Result<Commit> {
    store.remove_entry(name)?;
    let rel = entry_rel(store, name);
    Ok(commit(store.root(), &[&rel], &messages::remove(name)))
}

/// Rename (move) an entry, re-encrypting to the destination's recipients.
pub fn move_entry(
    store: &Store,
    from: &str,
    to: &str,
    backend: &dyn CryptoBackend,
) -> anyhow::Result<Commit> {
    store.move_entry(from, to, backend)?;
    let from_rel = entry_rel(store, from);
    let to_rel = entry_rel(store, to);
    Ok(commit(
        store.root(),
        &[&from_rel, &to_rel],
        &messages::rename(from, to),
    ))
}

/// Copy an entry, re-encrypting to the destination's recipients.
pub fn copy_entry(
    store: &Store,
    from: &str,
    to: &str,
    backend: &dyn CryptoBackend,
) -> anyhow::Result<Commit> {
    store.copy_entry(from, to, backend)?;
    let to_rel = entry_rel(store, to);
    Ok(commit(store.root(), &[&to_rel], &messages::copy(from, to)))
}

/// Re-encrypt a subtree to its current recipients (after a `.gpg-id` change),
/// then commit every file that was rewritten.
pub fn reencrypt(
    store: &Store,
    subpath: &str,
    backend: &dyn CryptoBackend,
) -> anyhow::Result<(usize, Commit)> {
    let targets = store.reencrypt_subtree(subpath, backend)?;
    let rels: Vec<String> = targets.iter().map(|n| entry_rel(store, n)).collect();
    let refs: Vec<&str> = rels.iter().map(String::as_str).collect();
    let outcome = commit(store.root(), &refs, &messages::reencrypt(subpath));
    Ok((targets.len(), outcome))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::{PgpBackend, PgpKeyStore};
    use pass_core::store::StoreFormat;

    fn gen_backend() -> (PgpBackend, String) {
        use pgp::composed::{
            EncryptionCaps, KeyType, SecretKeyParamsBuilder, SignedPublicKey, SubkeyParamsBuilder,
        };
        use pgp::ser::Serialize as _;
        use pgp::types::KeyDetails;
        let rng = rand::rngs::OsRng;
        let params = SecretKeyParamsBuilder::default()
            .key_type(KeyType::Ed25519)
            .can_sign(true)
            .primary_user_id("edit <e@e>".into())
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
        let mut store = PgpKeyStore::new();
        store
            .add_secret_key_bytes(&key.to_bytes().unwrap())
            .unwrap();
        (PgpBackend::new(store, None), fpr)
    }

    #[test]
    fn add_read_move_remove_in_a_git_store_commits() {
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("store");
        // A real git-backed pass store initialized through pass-core.
        GitStore::init(&root, StoreFormat::Pass).unwrap();
        let (backend, fpr) = gen_backend();
        // A real .gpg-id ends with a newline; pass-core (like the CLI) drops a
        // final unterminated line.
        std::fs::write(root.join(".gpg-id"), format!("{fpr}\n")).unwrap();

        let store = Store::open(&root, StoreFormat::Pass).unwrap();

        // add
        let entry = Entry::from_bytes(b"hunter2\nlogin: kevin\n".to_vec());
        assert_eq!(
            save_entry(&store, "web/github.com", &entry, &backend, true, false).unwrap(),
            Commit::Committed
        );
        // read back
        let got = store.read_entry("web/github.com", &backend).unwrap();
        assert_eq!(got.to_bytes(), entry.to_bytes());

        // move
        assert_eq!(
            move_entry(&store, "web/github.com", "web/gh", &backend).unwrap(),
            Commit::Committed
        );
        assert!(store.has_entry("web/gh"));
        assert!(!store.has_entry("web/github.com"));

        // reencrypt (no-op recipients, but exercises the path + commit)
        let (n, c) = reencrypt(&store, "", &backend).unwrap();
        assert!(n >= 1);
        assert_eq!(c, Commit::Committed);

        // remove
        assert_eq!(remove_entry(&store, "web/gh").unwrap(), Commit::Committed);
        assert!(!store.has_entry("web/gh"));
    }
}
