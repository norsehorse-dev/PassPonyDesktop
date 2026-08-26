# PassPony Desktop — Phase 0 findings

The one decision that blocks scaffolding: **which Rust OpenPGP library backs the
`pass` (OpenPGP) store format.** Everything else about the port is settled by the
existing code. This document pins the requirement, evaluates the options against
it, and recommends a path plus the spike that closes the question for good.

Written to mirror the family pattern (AgePony Desktop's `PHASE0_FINDINGS.md`):
answer the blocking crypto question first, in prose, before a line of the app
is scaffolded.

## The shape of the project

PassPony Desktop is a cross-platform native Rust app, built the way AgePony
Desktop is built: a Cargo workspace with a headless logic layer and an
egui/eframe GUI plus CLI binaries on top. AgePony Desktop is the structural
template. PassPonyCore is the shared logic, consumed as a git submodule
(`third_party/passponycore`), the same way the iOS and Android apps consume it.

What that buys us, and what it leaves to build, is not a guess. It is readable
straight out of `pass-core`.

### Reused wholesale from `pass-core` (all pure Rust, all cross-platform today)

- `store.rs` — the store model, `StoreFormat`, `.gpg-id` / `.age-recipients`
  resolution (nearest-first), `read_entry`.
- `entry.rs` — the entry codec (password line plus fields, including the
  `otpauth://` line that feeds TOTP).
- `git.rs` — the sync engine over `git2`, with pass-CLI-compatible commit
  messages.
- `totp.rs` — RFC 6238 TOTP over `hmac` / `sha1` / `sha2`.
- `crypto.rs` — the crypto seam itself (below).

The desktop links these directly, Rust to Rust. No UniFFI, no JNA, no FFI of any
kind. The mobile apps cross the `CryptoBackend` boundary with a UniFFI callback
because their crypto lives in Kotlin and Swift. Ours lives in the same language
as the core, so the boundary is an ordinary trait impl.

### The crypto seam, verbatim from `pass-core/src/crypto.rs`

```rust
pub trait CryptoBackend: Send + Sync {
    fn encrypt(&self, plaintext: &[u8], recipients: &[Recipient]) -> Result<Vec<u8>, CryptoError>;
    fn decrypt(&self, ciphertext: &[u8]) -> Result<Zeroizing<Vec<u8>>, CryptoError>;
}
```

Two methods. `Recipient` is a `String` (a `.gpg-id` line for pass, an age
recipient line for passage). Errors carry no entry names and no plaintext, by
design. And the core's own module doc already names the intended desktop plan:

```text
//! - Desktop: sequoia-openpgp + the age crate, natively in Rust
```

That line is the reason this document exists rather than just doing what it says.
It was written before rpgp shipped RFC 9580, and it is worth a fresh look now.

### The two backends the desktop must supply

**Passage (age).** Low risk, effectively already written. The phone's
`age_engine.rs` is a thin wrapper over the `age` crate: parse an identities file
(one `AGE-SECRET-KEY-1…` per line, `#` comments skipped), encrypt to the resolved
recipient set or fall back to the identities' own public keys, decrypt by trying
every identity. It is only tied to the phones by its UniFFI macros. The desktop
reimplements the same forty lines as a native `CryptoBackend` over the `age`
crate (the same crate, same version, AgePony Desktop already depends on it). No
open question here.

**Pass (OpenPGP).** This is the decision. Below.

## What the pass backend actually has to do

The surface is much smaller than PGPony's, because a password store is not a
messaging app. Pinned from `PgpEngine.kt` (Android) and `store.rs`:

- **Encrypt** plaintext to the recipients named in `.gpg-id`, producing a
  **binary** `.gpg` OpenPGP message (not armored — pass entries are binary).
- **Decrypt** a `.gpg` entry with whatever secret keys the backend holds, into a
  zeroizing buffer. Passphrase prompting and caching are the backend's job.
- **Resolve** a `.gpg-id` spec against imported keys: uppercase-hex fingerprint
  (primary or subkey) or 16-hex key ID, tolerating a `0x` prefix and inner
  spaces, with a fingerprint-suffix match at 16+ chars. This rule is identical
  across iOS and Android and must match here so a store made on a phone opens on
  the desktop and vice versa.

What is explicitly **out of scope** for the pass backend, because pass never
uses it: detached signatures, signing at all, PGP/MIME, keyservers, WKD, trust
management. PGPony Desktop needs all of that. PassPony Desktop needs none of it.

**Key types that must interoperate:**

- **RSA** — the common case in existing `pass` stores in the wild. Android's
  engine deliberately accepts RSA recipients (BouncyCastle does it in software);
  the desktop must too, or it cannot open a normal `pass` user's store.
- **ECDH Curve25519 (Cv25519), v4** — PGPony's and modern gpg's default.
- **X25519 / Ed25519, v6 (RFC 9580)** — the keys PGPony itself generates now.

**Smartcard is deferred for v1.** iOS does not have it yet either (its README
lists it under "Not yet"). It is a real future want, and it influences the
choice below, but it is not a v1 gate.

## The options

### Option A — rpgp (the `pgp` crate)

Pure Rust, no C dependencies. Latest `0.20.0` (June 2026). Supports RSA, ECDH
Curve25519, Ed25519, v4 and v6 keys (RFC 9580), encryption and decryption, and
passphrase-protected secret keys. In production in Delta Chat, himalaya, and the
`rsop` / `rpgpie` tool family.

- **For:** It is exactly the AgePony ethos — pure Rust, one self-contained binary
  per platform, nothing to link and nothing to install on the user's machine. It
  covers the entire key-type matrix pass needs. Its narrow, message-oriented API
  is a clean fit for a two-method backend. Delta Chat is a serious, adversarial
  production user, which is the kind of proving pass's needs actually lean on.
- **Against:** Smaller, lower-level API than sequoia; we own more of the
  key-store and recipient-matching glue (which we are writing regardless, cf.
  `PgpKeyStore.kt`). Smartcard, when it comes, is not in rpgp; it would be a
  separate PC/SC path or a gpg hand-off.

### Option B — sequoia-openpgp (the core's documented choice)

Latest `2.3.0`, full RFC 9580 and RFC 4880. The most complete and rigorously
correct OpenPGP implementation in Rust, and what `crypto.rs` names.

- **For:** Correctness on strange real-world keys is its whole reputation, which
  matters when we cannot predict what a random `pass` user's keyring looks like.
  Full standard coverage with room to grow into signing or WKD if PassPony ever
  wants them (it currently does not).
- **Against:** The default crypto backend is **Nettle, a C library**. That is the
  opposite of a self-contained cross-platform binary: it means a build-time C
  toolchain and platform-specific linking, exactly the friction AgePony Desktop
  was built to avoid. Sequoia does offer other backends (OpenSSL, Windows CNG,
  and a pure-Rust `crypto-rust`), but the pure-Rust one has historically shipped
  with caveats (not all primitives constant-time; flagged as not for production).
  Its current status must be verified before it could be the answer here, and if
  it does not hold up we are back to per-OS C backends and the CI matrix that
  implies.

### Option C — gpg shell-out

What the real `pass` CLI does: hand the bytes to the user's `gpg`.

- **For:** Maximal interop by definition (it *is* gpg), and smartcards "just
  work" through the user's existing gpg-agent, for free, today.
- **Against:** Not self-contained. Requires a configured GnuPG on the machine,
  which is a poor assumption on Windows and not guaranteed on macOS. Wrong as the
  primary backend for an app whose selling point is that it is one binary with no
  dependencies. Worth keeping in mind as an **optional** backend later for users
  who specifically want their smartcard and already live in gpg.

## Recommendation

**Build the pass backend on rpgp, and prove it with the spike below before
committing the rest of the app to it.**

The reasoning: the pass backend's job is narrow (encrypt and decrypt, no
signing), the key-type matrix is fully covered by rpgp today, and rpgp is the
only option that keeps the "pure Rust, one binary, nothing to install" property
that is the entire reason this app is native Rust instead of the JVM/Compose
route we set aside. Sequoia's correctness edge is real, but its cost is a C
dependency or an experimental pure-Rust backend, and pass does not exercise the
exotic corners of OpenPGP where that edge pays off. The core's documented
sequoia preference predates rpgp's RFC 9580 support and is worth revisiting on
that basis.

Keep **sequoia as the named fallback**: if the spike turns up a concrete interop
or key-parsing gap in rpgp that sequoia handles, switch, and accept the build
cost. Keep **gpg shell-out on the roadmap** as an optional backend for the
smartcard story rather than as a v1 dependency.

## The spike that closes this (do this first, in a throwaway crate)

Acceptance is interop, not "it compiles." Before scaffolding the app, stand up a
tiny binary that implements `CryptoBackend` over rpgp and passes all of:

1. **Decrypt real `pass` entries.** Take a store created by the actual `pass` CLI
   with a plain **RSA** key and with a **Cv25519** key, and decrypt entries from
   both. This is the interop floor.
2. **Round-trip.** Encrypt to those recipients and decrypt back, byte-for-byte.
3. **Cross-tool.** Decrypt a `.gpg` produced by `gpg` for a **v6 (RFC 9580)**
   key, and confirm `gpg` can decrypt what rpgp produced for the same key.
4. **Cross-device.** Decrypt an entry a **PGPony phone** wrote (against the shared
   fixtures the phones already carry), and confirm a phone can read what the
   desktop wrote. This is the promise the whole family rests on.
5. **Passphrase-locked secret keys** unlock and decrypt.
6. **Recipient resolution** matches `PgpEngine.resolve()` exactly, including the
   16-char fingerprint-suffix case, verified against the same inputs the phones
   test.

If all six pass, rpgp is the answer and we scaffold on it. If 1, 3, or 4 fail in
a way that is rpgp's and not ours, we fall back to sequoia and pay the build
cost. Either way the decision is made on evidence, against real keys, not on a
crate's feature table.

## Open questions for Kevin

1. **rpgp over the core's documented sequoia?** This doc recommends it; the core
   comment says sequoia. Your call, and I would make it after the spike, not
   before.
2. **Smartcard in v1 or deferred?** iOS defers it. Deferring keeps rpgp clean; if
   it is a v1 must-have, that pulls gpg shell-out (or a PC/SC path) forward and
   changes the shape of the backend.
3. **Repo home.** `~/Apps/PassPonyDesktop` exists and is empty. Confirm this is
   the repo root and it publishes to `norsehorse-dev/PassPonyDesktop` like the
   rest of the family.
4. **PassPonyCore submodule pin.** Which commit of PassPonyCore the desktop
   pins on first scaffold (latest `main`, or the same SHA the current phone
   release pins).
