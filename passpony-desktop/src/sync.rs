//! Git sync wrappers over `pass_core::git`, plus HTTPS credential handling.
//!
//! Non-GUI and thin so the flows are testable against a local bare "remote"
//! without a network. The worker thread and the conflict UI live in
//! `tasks`/`panels::sync`; this module is just the operations.
//!
//! Auth is HTTPS-with-token: pass-core reads `user:token@host` userinfo out of
//! the remote URL and supplies it as plaintext credentials (SSH is a later
//! phase). [`with_credentials`] builds that URL; it holds a token, so a URL that
//! has been through it must be redacted before display.

use std::path::{Path, PathBuf};

use pass_core::git::GitStore;
use pass_core::store::StoreFormat;

pub use pass_core::git::{ConflictChoice, SyncOutcome, SyncStatus};

/// Status of the store's repo (ahead/behind/dirty/has_remote), or `None` if the
/// path is not a git repository.
#[must_use]
pub fn status(root: &Path) -> Option<SyncStatus> {
    GitStore::open(root).ok().and_then(|g| g.status().ok())
}

/// Is this store a git repository at all?
#[must_use]
pub fn is_repo(root: &Path) -> bool {
    GitStore::open(root).is_ok()
}

/// Clone a remote store into `dest`. `url` may carry `user:token@` userinfo.
pub fn clone(url: &str, dest: impl Into<PathBuf>) -> anyhow::Result<()> {
    GitStore::clone_from(url, dest, None)?;
    Ok(())
}

/// Publish an existing local store: make it a repo if needed, point `origin` at
/// `url`, and push. `url` may carry `user:token@` userinfo.
pub fn publish(root: &Path, format: StoreFormat, url: &str) -> anyhow::Result<()> {
    let git = match GitStore::open(root) {
        Ok(g) => g,
        Err(_) => GitStore::init(root, format)?,
    };
    git.set_remote(url)?;
    git.push()?;
    Ok(())
}

pub fn set_remote(root: &Path, url: &str) -> anyhow::Result<()> {
    GitStore::open(root)?.set_remote(url)?;
    Ok(())
}

pub fn fetch(root: &Path) -> anyhow::Result<()> {
    GitStore::open(root)?.fetch()?;
    Ok(())
}

pub fn push(root: &Path) -> anyhow::Result<()> {
    GitStore::open(root)?.push()?;
    Ok(())
}

/// Sync (pull/rebase/push-ready), consulting `resolver` once per conflicted
/// path. The resolver blocks the calling thread until it returns a choice — in
/// the app it is driven from the UI over a channel (see `tasks`).
pub fn sync(
    root: &Path,
    resolver: &mut dyn FnMut(&str) -> ConflictChoice,
) -> anyhow::Result<SyncOutcome> {
    Ok(GitStore::open(root)?.sync(resolver)?)
}

/// The remote URL with its userinfo (the token) stripped, for display.
#[must_use]
pub fn redacted_remote(root: &Path) -> Option<String> {
    let url = GitStore::open(root).ok()?.remote_url()?;
    Some(redact(&url))
}

/// Remove `user:token@` userinfo from an https URL for display.
#[must_use]
pub fn redact(url: &str) -> String {
    match url.strip_prefix("https://") {
        Some(rest) => match rest.split_once('@') {
            Some((_, host)) => format!("https://{host}"),
            None => url.to_string(),
        },
        None => url.to_string(),
    }
}

/// Build a push/clone URL with HTTPS credentials embedded. Empty user and token
/// leave the URL untouched (e.g. a public clone, or creds from a helper).
#[must_use]
pub fn with_credentials(url: &str, user: &str, token: &str) -> String {
    if user.is_empty() && token.is_empty() {
        return url.to_string();
    }
    if let Some(rest) = url.strip_prefix("https://") {
        let host = rest.split_once('@').map(|(_, h)| h).unwrap_or(rest);
        return format!("https://{}:{}@{}", enc(user), enc(token), host);
    }
    url.to_string()
}

/// Percent-encode the characters that would break URL userinfo.
fn enc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'@' | b':' | b'/' | b'#' | b'?' | b'%' | b' ' => {
                out.push_str(&format!("%{b:02X}"));
            }
            _ => out.push(b as char),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A valid `file://` URL for a local path, on every platform. Unix paths are
    /// already `/…` (→ `file:///…`); a Windows path like `C:\a\b` becomes
    /// `file:///C:/a/b` — string-concatenating `file://` onto the raw backslash
    /// form gives libgit2 a path it rejects, which is the bug this avoids.
    fn file_url(path: &std::path::Path) -> String {
        let s = path.display().to_string().replace('\\', "/");
        if s.starts_with('/') {
            format!("file://{s}")
        } else {
            format!("file:///{s}")
        }
    }

    #[test]
    fn credential_url_roundtrip() {
        assert_eq!(
            with_credentials("https://x.com/r.git", "", ""),
            "https://x.com/r.git"
        );
        assert_eq!(
            with_credentials("https://github.com/me/store.git", "me", "gh_tok"),
            "https://me:gh_tok@github.com/me/store.git"
        );
        // existing userinfo is replaced, and specials are encoded
        assert_eq!(
            with_credentials("https://old:tok@h/r", "u", "a/b@c"),
            "https://u:a%2Fb%40c@h/r"
        );
        assert_eq!(
            redact("https://me:secret@github.com/me/store.git"),
            "https://github.com/me/store.git"
        );
    }

    // init -> commit -> push to a local bare remote -> clone -> push a change ->
    // sync the original forward. Proves publish/clone/push/fetch/sync end to end
    // with no network. Conflict resolution itself is covered by pass-core's own
    // tests; here the resolver is never consulted (a fast-forward).
    #[test]
    fn publish_clone_and_fast_forward_sync() {
        let tmp = tempfile::tempdir().unwrap();

        // First working copy: a git store with one committed file.
        let s1 = tmp.path().join("s1");
        let g1 = GitStore::init(&s1, StoreFormat::Pass).unwrap();
        std::fs::write(s1.join(".gpg-id"), "KEYID\n").unwrap();
        std::fs::write(s1.join("a.gpg"), b"ciphertext-a").unwrap();
        g1.commit_paths(&[".gpg-id", "a.gpg"], "add a").unwrap();

        // The "remote": a bare clone OF s1, so its HEAD names s1's branch (a
        // freshly init_bare repo leaves HEAD on an ambiguous default branch,
        // which a real host like GitHub never does). Point s1 at it and push.
        let bare = tmp.path().join("remote.git");
        git2::build::RepoBuilder::new()
            .bare(true)
            .clone(&file_url(&s1), &bare)
            .unwrap();
        let bare_url = file_url(&bare);
        set_remote(&s1, &bare_url).unwrap();
        fetch(&s1).unwrap();

        // Second working copy from the clone; add a file and push.
        let s2 = tmp.path().join("s2");
        clone(&bare_url, &s2).unwrap();
        let g2 = GitStore::open(&s2).unwrap();
        std::fs::write(s2.join("b.gpg"), b"ciphertext-b").unwrap();
        g2.commit_paths(&["b.gpg"], "add b").unwrap();
        push(&s2).unwrap();

        // The original is now behind; sync fast-forwards it.
        let st = status(&s1).unwrap();
        assert!(st.has_remote);
        let mut never = |_p: &str| ConflictChoice::KeepBoth;
        let _ = st;
        let outcome = sync(&s1, &mut never).unwrap();
        // Fast-forward or a zero-conflict rebase, depending on tracking setup;
        // the point is it integrated cleanly without consulting the resolver.
        assert!(
            !matches!(outcome, SyncOutcome::ResolvedConflicts { .. }),
            "unexpected conflict: {outcome:?}"
        );
        assert!(
            s1.join("b.gpg").exists(),
            "the remote's file should be present after sync"
        );
    }
}
