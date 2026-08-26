# Security

PassPony Desktop reads and writes password stores. If something here is wrong, a secret is readable
by someone who should not read it, or an entry the owner still needs becomes unreadable — so this
document exists.

## Reporting

Report privately, not as a public issue: **NorseHorse@norsehor.se**, or through GitHub's private
vulnerability reporting on this repository.

Encrypt anything sensitive to the release key, which is also the key that signs every artifact:

```
A0CBC8F65AACE56F1C5B767753F9798E4919DE62
```

No bounty, and no fixed response time — this is a free tool with one maintainer. What is promised is
that a credible report gets read and answered.

## Scope

**In scope:** anything that makes an entry readable by someone who should not read it, or unreadable
by someone who should. Wrong ciphertext, encryption to a recipient that does not match the intended
key, plaintext or key material reaching disk where it should not, a parser that can be made to crash
or misbehave on hostile input, a `.gpg-id` written so that pass-core resolves the wrong recipients,
or an interoperability break with PassPony on iOS or Android, with the `pass`/`passage` command-line
tools, or with gpg/age.

**Out of scope:** an attacker who already has code execution as the user. The app decrypts entries
with keys that user holds; nothing here defends against a process that can already read that user's
memory or files.

## What this app does and does not do

- **No network except git sync.** The only outbound connections are to the git remote you configure,
  when you clone, fetch, push or sync. Encryption, decryption, and your keys never leave the machine
  otherwise. The links on the Settings screen open your browser; the app itself does not connect.
- **No telemetry, no accounts, no server.** There is nothing to opt out of.
- **Plaintext is shown deliberately, and held carefully.** PassPony is a password manager, so a
  revealed password and a decrypted entry's fields are shown on screen by design. That plaintext
  lives in a zeroizing buffer and is cleared when you close the entry, change the selection, or close
  the store. A copied secret is cleared from the clipboard after 45 seconds, provided nothing else
  has overwritten it since.
- **A typed passphrase for a locked pass key** is held in memory for five minutes, then cleared, and
  is never written to disk.
- **Git sync backs up entries, not keys.** Sync pushes only the encrypted `.gpg`/`.age` files. Your
  OpenPGP keys and age identities never leave this machine unless you export them yourself (Keys, or
  Settings › Back up).

## The two backends

- **pass** uses OpenPGP, via [rpgp](https://github.com/rpgp/rpgp). PassPony writes standard `.gpg`
  files and a newline-terminated `.gpg-id`, so gpg, the `pass` CLI and the phone apps read them.
- **passage** uses [age](https://github.com/str4d/rage). PassPony writes standard `.age` files, so
  the `passage` CLI, `age`/`rage` and the phone apps read them.

The store model, entry codec, git engine and TOTP come from PassPonyCore — the same Rust core the
phones ship — consumed here as a git submodule. Only the desktop crate is desktop-specific; the
crypto boundary (`CryptoBackend`) is the phones' boundary.

## Keys at rest

Imported OpenPGP keys and generated age identities live in the app's config directory (shown on the
Settings screen), readable by you alone on Unix. A pass secret key can be passphrase-protected so the
key file itself is encrypted. The **panic wipe** on the Settings screen removes every imported key and
identity at once; it does not touch your stores.

## What PassPony cannot do

- It cannot recover a key you have lost, nor the entries encrypted to it. Keep a backup of your keys.
- It cannot protect files after they leave PassPony, or defend a computer already compromised.
- It sends no analytics or telemetry, ever.
