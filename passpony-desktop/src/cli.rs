//! Read-only verbs, for verifying a packaged build and quick store inspection.
//!
//! These exist for the same reason AgePony's do: a packaged GUI binary can
//! build, install, and still fail to start because a shared library is missing
//! on the target or the glibc floor is too high, and nothing in `cargo test`
//! catches that. Something that runs the shipped binary on the target can.
//!
//! | verb | what it proves |
//! |---|---|
//! | `version` | the binary starts, links, and reports the toolchain that built it |
//! | `selftest` | both crypto backends round-trip in this build, not just in tests |
//! | `list` | a real store directory opens and enumerates outside a dev tree |

use pass_core::store::{Store, StoreFormat};

use crate::crypto::{AgeBackend, PgpBackend, PgpKeyStore};

/// The compiler that built this binary, from build.rs.
pub const RUSTC: &str = env!("PASSPONY_RUSTC");
/// The target triple this binary was built for.
pub const TARGET: &str = env!("PASSPONY_TARGET");
/// The crate version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Run a verb. Returns `None` if the GUI should start, or `Some(exit_code)`.
#[must_use]
pub fn run(args: &[String]) -> Option<i32> {
    let verb = args.first()?.as_str();
    Some(match verb {
        "version" | "--version" | "-V" => {
            println!("{}", version_line());
            0
        }
        "selftest" => selftest(),
        "list" | "ls" => list(args.get(1).map(String::as_str)),
        "keys" => keys_list(),
        "import-key" => import_key(args.get(1).map(String::as_str)),
        "import-identities" => import_identities(args.get(1).map(String::as_str)),
        "gen-identity" => gen_identity(),
        "show" | "cat" => show(
            args.get(1).map(String::as_str),
            args.get(2).map(String::as_str),
        ),
        "gen-password" => gen_password(args.get(1).map(String::as_str)),
        "generate" => generate_entry(
            args.get(1).map(String::as_str),
            args.get(2).map(String::as_str),
        ),
        "rm" | "remove" => remove_entry_cli(args.get(1).map(String::as_str)),
        "help" | "--help" | "-h" => {
            println!("{}", usage());
            0
        }
        other => {
            eprintln!("passpony: unknown verb '{other}'\n\n{}", usage());
            2
        }
    })
}

#[must_use]
pub fn version_line() -> String {
    format!("passpony {VERSION} ({TARGET}, built with {RUSTC})")
}

fn usage() -> String {
    "\
passpony — pass and passage on the desktop

USAGE:
    passpony                 open the app
    passpony version         print version and build info
    passpony selftest        round-trip both crypto backends in this build
    passpony list [pass|passage]
                             list entries in the default store (no decryption)
    passpony show <entry> [pass|passage]
                             decrypt and print an entry (PASSPONY_PASSPHRASE for
                             a passphrase-locked pass key; a YubiKey identity
                             asks for its PIN, or takes PASSPONY_PIV_PIN)
    passpony keys            list imported pass keys and age identities
    passpony import-key <file>
                             import an OpenPGP key file (public or secret)
    passpony import-identities <file>
                             add the age identities in an identities file (such
                             as passage's, with age-plugin-yubikey stubs)
    passpony gen-identity    generate a new age identity for passage
    passpony gen-password [len]
                             print a generated password (default 20)
    passpony generate <entry> [len]
                             generate a password, save it to the pass store, commit
    passpony rm <entry>      remove an entry from the pass store and commit
    passpony help            this text"
        .to_string()
}

fn gen_password(len: Option<&str>) -> i32 {
    let len = len.and_then(|s| s.parse::<usize>().ok()).unwrap_or(20);
    println!(
        "{}",
        crate::pwgen::generate(&crate::pwgen::PwOptions {
            len,
            ..Default::default()
        })
    );
    0
}

/// Open the default pass store and build its backend (PASSPONY_PASSPHRASE for a
/// locked key). Pass format only; the GUI covers passage editing.
fn pass_store_and_backend() -> Option<(Store, crate::crypto::Backend)> {
    let home = directories::UserDirs::new().map(|u| u.home_dir().to_path_buf())?;
    let root = home.join(StoreFormat::Pass.default_store_dir());
    let store = match Store::open(&root, StoreFormat::Pass) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("passpony: cannot open {}: {e}", root.display());
            return None;
        }
    };
    let passphrase = std::env::var("PASSPONY_PASSPHRASE").ok();
    match crate::keys::load_backend(StoreFormat::Pass, passphrase.as_deref()) {
        Ok(b) => Some((store, b)),
        Err(e) => {
            eprintln!("passpony: {e}");
            None
        }
    }
}

fn report_commit(action: &str, name: &str, outcome: &crate::edit::Commit) {
    match outcome {
        crate::edit::Commit::Committed => println!("{action} {name} (committed)"),
        crate::edit::Commit::NotARepo => {
            println!("{action} {name} (saved; store is not a git repo)")
        }
        crate::edit::Commit::Failed(e) => println!("{action} {name} (saved; commit failed: {e})"),
    }
}

fn generate_entry(name: Option<&str>, len: Option<&str>) -> i32 {
    let Some(name) = name else {
        eprintln!("passpony: generate needs an entry name");
        return 2;
    };
    let len = len.and_then(|s| s.parse::<usize>().ok()).unwrap_or(20);
    let Some((store, backend)) = pass_store_and_backend() else {
        return 1;
    };
    let pw = crate::pwgen::generate(&crate::pwgen::PwOptions {
        len,
        ..Default::default()
    });
    let entry = pass_core::entry::Entry::from_password(pw.as_bytes());
    let is_new = !store.has_entry(name);
    match crate::edit::save_entry(&store, name, &entry, backend.as_crypto(), is_new, true) {
        Ok(outcome) => {
            report_commit("generated", name, &outcome);
            0
        }
        Err(e) => {
            eprintln!("passpony: {e}");
            1
        }
    }
}

fn remove_entry_cli(name: Option<&str>) -> i32 {
    let Some(name) = name else {
        eprintln!("passpony: rm needs an entry name");
        return 2;
    };
    let Some((store, _backend)) = pass_store_and_backend() else {
        return 1;
    };
    match crate::edit::remove_entry(&store, name) {
        Ok(outcome) => {
            report_commit("removed", name, &outcome);
            0
        }
        Err(e) => {
            eprintln!("passpony: {e}");
            1
        }
    }
}

fn keys_list() -> i32 {
    match crate::keys::list_pgp_key_files() {
        Ok(files) => {
            println!("pass keys ({}):", files.len());
            for f in files {
                println!("  {f}");
            }
        }
        Err(e) => {
            eprintln!("passpony: {e}");
            return 1;
        }
    }
    match crate::keys::load_age_backend() {
        Ok(_) => match crate::keys::identities_file() {
            Ok(path) => println!(
                "age identities: {}",
                if path.exists() {
                    path.display().to_string()
                } else {
                    "none".into()
                }
            ),
            Err(e) => eprintln!("passpony: {e}"),
        },
        Err(e) => eprintln!("passpony: {e}"),
    }
    0
}

fn import_key(path: Option<&str>) -> i32 {
    let Some(path) = path else {
        eprintln!("passpony: import-key needs a file path");
        return 2;
    };
    match crate::keys::import_pgp_key(std::path::Path::new(path)) {
        Ok(dest) => {
            println!("imported {}", dest.display());
            0
        }
        Err(e) => {
            eprintln!("passpony: {e}");
            1
        }
    }
}

fn gen_identity() -> i32 {
    match crate::keys::generate_age_identity() {
        Ok(recipient) => {
            println!("generated age identity");
            println!("public recipient: {recipient}");
            0
        }
        Err(e) => {
            eprintln!("passpony: {e}");
            1
        }
    }
}

/// Decrypt and print an entry from the default store. This is the D1 proof that
/// the whole pipeline — keystore, backend, pass-core — works end to end.
fn show(entry: Option<&str>, which: Option<&str>) -> i32 {
    use pass_core::crypto::CryptoBackend;
    let Some(name) = entry else {
        eprintln!("passpony: show needs an entry name");
        return 2;
    };
    let format = match which {
        None | Some("pass") => StoreFormat::Pass,
        Some("passage") => StoreFormat::Passage,
        Some(other) => {
            eprintln!("passpony: unknown store '{other}' (expected pass or passage)");
            return 2;
        }
    };
    let Some(home) = directories::UserDirs::new().map(|u| u.home_dir().to_path_buf()) else {
        eprintln!("passpony: cannot locate home directory");
        return 1;
    };
    let root = home.join(format.default_store_dir());
    let store = match Store::open(&root, format) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("passpony: cannot open {}: {e}", root.display());
            return 1;
        }
    };

    let passphrase = std::env::var("PASSPONY_PASSPHRASE").ok();
    let result = match format {
        StoreFormat::Pass => {
            let backend = match crate::keys::load_pgp_backend(passphrase.as_deref()) {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("passpony: {e}");
                    return 1;
                }
            };
            store.read_entry(name, &backend as &dyn CryptoBackend)
        }
        StoreFormat::Passage => {
            let backend = match crate::keys::load_age_backend() {
                Ok(b) => b.with_prompter(std::sync::Arc::new(TerminalPrompter)),
                Err(e) => {
                    eprintln!("passpony: {e}");
                    return 1;
                }
            };
            store.read_entry(name, &backend as &dyn CryptoBackend)
        }
    };

    match result {
        Ok(entry) => {
            print!("{}", String::from_utf8_lossy(entry.to_bytes()));
            0
        }
        Err(e) => {
            eprintln!("passpony: cannot read '{name}': {e}");
            1
        }
    }
}

/// Prove both backends work in this build, without touching the user's keys.
fn selftest() -> i32 {
    // passage: a generated age identity round-trips.
    let id = age::x25519::Identity::generate();
    let recipient = id.to_public().to_string();
    let age_backend = {
        use age::secrecy::ExposeSecret;
        AgeBackend::from_identities_text(id.to_string().expose_secret())
    };
    use pass_core::crypto::CryptoBackend;
    let msg = b"passpony selftest\n";
    let age_ok = age_backend
        .encrypt(msg, &[recipient])
        .and_then(|ct| age_backend.decrypt(&ct))
        .map(|pt| *pt == msg)
        .unwrap_or(false);

    // pass: a generated OpenPGP key round-trips through the rpgp backend.
    let pgp_ok = pgp_selftest(msg).unwrap_or(false);

    println!("passage (age)     : {}", if age_ok { "ok" } else { "FAIL" });
    println!("pass (openpgp)    : {}", if pgp_ok { "ok" } else { "FAIL" });
    i32::from(!(age_ok && pgp_ok))
}

fn pgp_selftest(msg: &[u8]) -> anyhow::Result<bool> {
    use pass_core::crypto::CryptoBackend;
    use pgp::composed::{
        EncryptionCaps, KeyType, SecretKeyParamsBuilder, SignedPublicKey, SubkeyParamsBuilder,
    };
    use pgp::ser::Serialize as _;
    use pgp::types::KeyDetails;
    let rng = rand::rngs::OsRng;
    let params = SecretKeyParamsBuilder::default()
        .key_type(KeyType::Ed25519)
        .can_sign(true)
        .primary_user_id("selftest <s@s>".into())
        .passphrase(None)
        .subkey(
            SubkeyParamsBuilder::default()
                .key_type(KeyType::X25519)
                .can_encrypt(EncryptionCaps::All)
                .passphrase(None)
                .build()
                .map_err(anyhow::Error::msg)?,
        )
        .build()
        .map_err(anyhow::Error::msg)?;
    let key = params.generate(rng)?;
    let fpr: String = SignedPublicKey::from(key.clone())
        .fingerprint()
        .as_bytes()
        .iter()
        .map(|b| format!("{b:02X}"))
        .collect();
    let mut store = PgpKeyStore::new();
    store.add_secret_key_bytes(&key.to_bytes()?)?;
    let backend = PgpBackend::new(store, None);
    let ct = backend.encrypt(msg, &[fpr]).map_err(anyhow::Error::msg)?;
    let pt = backend.decrypt(&ct).map_err(anyhow::Error::msg)?;
    Ok(*pt == msg)
}

/// List entry names in the default store for a format. No decryption.
fn list(which: Option<&str>) -> i32 {
    let format = match which {
        None | Some("pass") => StoreFormat::Pass,
        Some("passage") => StoreFormat::Passage,
        Some(other) => {
            eprintln!("passpony: unknown store '{other}' (expected pass or passage)");
            return 2;
        }
    };
    let Some(home) = directories::UserDirs::new().map(|u| u.home_dir().to_path_buf()) else {
        eprintln!("passpony: cannot locate home directory");
        return 1;
    };
    let root = home.join(format.default_store_dir());
    match Store::open(&root, format) {
        Ok(store) => match store.entries() {
            Ok(entries) => {
                println!("{} ({} entries)", format.ls_header(), entries.len());
                for e in entries.iter().filter(|e| !e.hidden) {
                    println!("  {}", e.name);
                }
                0
            }
            Err(e) => {
                eprintln!("passpony: {e}");
                1
            }
        },
        Err(e) => {
            eprintln!("passpony: cannot open {}: {e}", root.display());
            1
        }
    }
}

/// YubiKey (age plugin) prompts on the terminal: messages and questions on
/// stderr, answers from stdin, the PIN without echo (or from
/// `PASSPONY_PIV_PIN`, the way `PASSPONY_PASSPHRASE` gives the pass one).
struct TerminalPrompter;

impl crate::crypto::age_backend::PluginPrompter for TerminalPrompter {
    fn message(&self, text: &str) {
        eprintln!("{text}");
    }

    fn confirm(&self, text: &str, yes: &str, no: Option<&str>) -> Option<bool> {
        match no {
            Some(no) => eprint!("{text}\n  [1] {yes}\n  [2] {no}\n> "),
            None => eprint!("{text}\n  [1] {yes}\n> "),
        }
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).ok()?;
        match line.trim() {
            "1" => Some(true),
            "2" if no.is_some() => Some(false),
            _ => None,
        }
    }

    fn secret(&self, description: &str) -> Option<age::secrecy::SecretString> {
        if let Ok(pin) = std::env::var("PASSPONY_PIV_PIN") {
            return Some(pin.into());
        }
        rpassword::prompt_password(format!("{description}: "))
            .ok()
            .filter(|pin| !pin.is_empty())
            .map(Into::into)
    }
}

fn import_identities(path: Option<&str>) -> i32 {
    let Some(path) = path else {
        eprintln!("passpony: import-identities needs an age identities file");
        return 2;
    };
    match crate::keys::import_age_identities(std::path::Path::new(path)) {
        Ok(n) => {
            println!(
                "imported {n} age identit{}",
                if n == 1 { "y" } else { "ies" }
            );
            0
        }
        Err(e) => {
            eprintln!("passpony: {e}");
            1
        }
    }
}
