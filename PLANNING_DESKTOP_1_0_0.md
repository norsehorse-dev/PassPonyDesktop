# PassPony Desktop — plan to 1.0.0 (phone parity)

The v1 target is feature parity with PassPony iOS and Android: both store formats,
browse and search, entry detail with live TOTP, full editing, git sync with
per-entry conflict resolution, an unlock gate, settings, onboarding, and the seven
shared languages. Native Rust throughout, AgePony Desktop as the structural
template. Phases are cut so each ends on something runnable and testable, the way
the phone repo's P-packets and AgePony's parity plan were.

Smartcard is explicitly deferred past 1.0.0 (iOS defers it too). rpgp has no card
path; when it lands it is a gpg-agent hand-off or a PC/SC path, tracked separately.

## D0 — scaffold and the backend decision — DONE

Cargo workspace, PassPonyCore as a submodule (pinned to the phone-release SHA),
the `passpony-desktop` crate with both `CryptoBackend`s implemented natively, a
store-browser skeleton, the CLI verbs, CI, and repo hygiene. The pass backend is
rpgp, chosen and proven in `PHASE0_FINDINGS.md` / `PHASE0_SPIKE_RESULTS.md`. Two
findings from the spike are already baked into `pgp_backend.rs`: decrypt through
`TheRing` with `enable_gnupg_aead()`, and pick the SEIPD version by recipient.

## D1 — keys, identities, and a store that decrypts

Import pass OpenPGP keys (public and secret, armored or binary) and an age
identities file, held in the OS config directory with owner-only permissions.
A Settings/keys panel to add, list, and remove them. Passphrase entry and a cache
with a TTL (the backend already owns the policy). Wire a loaded backend to the
open store so `read_entry` actually returns plaintext. Port the phones' key-store
tests, including the recipient-resolution cases already mirrored in `keystore.rs`.

## D2 — entry detail

Decrypt and show an entry: password reveal/hide, one-click copy to an
auto-clearing clipboard (45 s, matching the phones), the parsed `key: value`
fields, and a live TOTP countdown ring driven by `pass_core::totp`. Plaintext
stays in zeroizing buffers and is cleared on leaving the view.

## D3 — editing

Add, edit, move, copy, and delete entries through `Store::write_entry` /
`move_entry` / `remove_entry`, with a password generator, and re-encrypt a subtree
when its recipients change (`reencrypt_targets` / `reencrypt_subtree`). Every
mutation commits with the pass-CLI-compatible message from `pass_core::git`.

## D4 — git sync

Clone, publish (init + set remote + first push), fetch, push, and sync with
per-entry conflict resolution, over `pass_core::git` (`SyncOutcome`,
`ConflictChoice`, `SyncStatus`). A conflict sheet mirroring the phones'. Long
operations move to the `tasks` worker thread with progress and cancellation.

## D5 — onboarding

First-run tour: pick a language, choose a format, clone an existing store or
import keys, add a first entry. Mirrors the phones' onboarding flow.

## D6 — settings and localization

The seven phone languages (en, es, fr, de, zh-Hans, pt-BR, ru) over the shared
string catalog, switchable live with no restart. Settings for clipboard timeout,
passphrase-cache TTL, theme, and store locations.

## D7 — design system  ✓

Ported AgePony Desktop's `theme.rs`: the NorseHorse palette and components
recoloured to PassPony's orange, Inter / JetBrains Mono / Lucide embedded, the
styled surface ladder. The window/rail mark is the shipped PassPony artwork
(horse and keys), decoded from a PNG (`appicon.rs`) — the earlier shield mesh was
AgePony's shape and has been removed.

## D8 — desktop integration  ✓

The app-shell overhaul (a fixed left rail of four destinations over a content
area, from the approved mockup) plus: drag-and-drop (a key file imports, a store
folder opens, with a drop overlay), ⌘1–4 / ⌘F / ⌘, shortcuts, a `logic`/`ui`
split, and remembered window geometry (eframe) plus screen, format, and last
store per format (`Prefs` via eframe storage). A headless smoke test paints every
screen in every language. Still open: single-instance forwarding and a native
menu bar / file associations, folded into D9's platform packaging where they are
per-OS anyway.

## D9 — release engineering  ✓

Icons from the 1024 master via `tools/make-icons.py` (Linux `.png`, Windows
`.ico` + `wix/Product.ico`, macOS `.icns` inset to the 824/1024 Dock grid with a
squircle corner). `.deb` (cargo-deb) and `.msi` (cargo-wix, `wix/main.wxs` with
committed GUIDs) metadata in the crate `Cargo.toml`; `tools/make-dmg.sh` builds a
**universal** signed + notarized `.dmg`; the AppImage from `packaging/appimage/`.
`.github/workflows/release.yml` on a `v*` tag builds the six Linux artifacts and
the MSI into a draft release, checking out the PassPonyCore **submodule
recursively** and asserting `version`/`selftest`/`keys` on each packaged artifact
(both crypto backends round-trip; the deb identity and desktop entry; the Windows
console/GUI subsystems). The Mac half — dmg, nine PGP signatures, `SHA256SUMS`,
publish — is `RELEASING.md`, adapted from AgePony's. `SECURITY.md` updated for a
password manager that reveals secrets in the UI. Release profile gains
`panic = "abort"`.

Deferred (documented in RELEASING.md §6): single-instance forwarding and a native
menu bar — per-OS niceties, harmless to omit while the rail is the app's
navigation; and the download site, winget/AUR manifests.

## D9 — release engineering (original brief)

Icons from a 1024 master, `.deb` (cargo-deb), `.msi` (cargo-wix), `.dmg` (signed
+ notarized), and AppImage; detached PGP signatures and a signed `SHA256SUMS`;
`release.yml` on a `v*` tag; the `version`/`selftest` verbs asserted on each
packaged artifact. A `RELEASING.md` adapted from AgePony's.

## Cross-cutting

- Shared fixtures: round-trip the phones' own ciphertext fixtures in CI (the one
  interop check the Phase 0 spike could not run here), plus a gpg-2.6 v6
  cross-decrypt check on a runner that has it.
- Keep PassPonyCore's submodule pin moving deliberately, alongside a phone
  release, not drifting onto `main`.
- No AI attribution in any commit, tag, or release note; all commits authored as
  norsehorse-dev.
