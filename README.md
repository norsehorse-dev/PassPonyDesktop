# PassPony Desktop

Pure Rust. egui/eframe. One binary per platform. The same `pass` and `passage`
stores, the same keys, the same git remotes as PassPony on iOS and Android.

**Status:** D1–D8 done. The app is a fixed left rail of four destinations —
Passwords, Keys, Sync, Settings — over a content area, the design ported from
AgePony Desktop and recoloured to PassPony's orange (the approved app-shell
overhaul). **Passwords**: pick pass or passage, open a store, search, and browse
a master–detail list; opening an entry shows a card with reveal/hide, copy to a
45-second auto-clearing clipboard, the `key: value` fields, and a live TOTP code
with a countdown ring (plaintext in a zeroizing buffer); add/edit/remove with a
password generator, each committed with pass-CLI-compatible messages.
**Keys**: import an OpenPGP key, enter a passphrase for a locked key (held five
minutes), and generate or back up an age identity. **Sync**: the working tree's
status against its remote, clone/publish/fetch/push/sync with per-file conflict
resolution on a worker thread so the UI never blocks (HTTPS token auth).
**Settings**: theme switcher (Auto/Light/Dark), language picker, storage,
re-encrypt, key backup, About, Help, Security, licenses, panic wipe, and the
family links — localized into seven languages (en, es, fr, de, zh-Hans, pt-BR,
ru) with a live switch. Plus the first-run tour, and **D8 desktop integration**:
drag a key file onto the window to import it or a store folder to open it,
⌘1–4 / ⌘F / ⌘, keyboard shortcuts, and remembered window geometry, screen, and
last store per format. The design system uses PassPony's orange palette,
embedded Inter / JetBrains Mono / Lucide / Noto Sans SC faces (the last so the
Simplified-Chinese UI renders rather than showing tofu boxes), a styled surface ladder,
and the real PassPony mark (horse and keys) as the window and rail icon. Much of
the core is also provable headlessly (`passpony show`, `generate`, `rm`), and a
headless smoke test paints every screen in every language. **D9 release
engineering** is in place: the icon set from a 1024 master
(`tools/make-icons.py`), deb/msi/dmg/AppImage inputs, a tagged `release.yml` that
builds the six Linux artifacts and the Windows MSI into a draft release and
asserts `version`/`selftest`/`keys` on each, and `RELEASING.md` for the signed,
notarized macOS half. The backend decision is in `PHASE0_FINDINGS.md` and
`PHASE0_SPIKE_RESULTS.md`; the release procedure is in `RELEASING.md` and the
security posture in `SECURITY.md`.

## Not a rewrite

The store model, entry codec, git sync engine and TOTP come from
[PassPonyCore](https://github.com/norsehorse-dev/PassPonyCore), the exact Rust
core the phones ship, vendored here as a git submodule under
[`third_party/`](third_party/) and consumed directly, Rust to Rust, with no FFI.
`pass-core` is crypto-free by design: it hands ciphertext and a resolved
recipient set to a `CryptoBackend`. The phones cross that boundary with a UniFFI
callback into Kotlin/Swift; the desktop implements it natively:

- **passage (age)** over the `age` crate, mirroring the phones' `age_engine`.
- **pass (OpenPGP)** over [rpgp](https://github.com/rpgp/rpgp), chosen and
  proven against real gpg stores in Phase 0. It interoperates both directions
  across RSA, Cv25519, passphrase-locked and v6/RFC 9580 keys, and does software
  RSA decrypt (which the iOS core cannot).

Only `passpony-desktop` under [`src/`](passpony-desktop/src/) is desktop-specific.

## First build

Requires a Rust toolchain (see `rust-toolchain.toml`) and, on Linux, the usual
GUI dev libraries for egui (X11/Wayland, GL). PassPonyCore is a submodule:

```
git clone --recurse-submodules https://github.com/norsehorse-dev/PassPonyDesktop.git
cd PassPonyDesktop
cargo run
```

After a plain clone, `git submodule update --init --recursive` first.

```
cargo run                         # open the app
cargo run -- selftest             # round-trip both crypto backends on this build
cargo run -- version              # version and the compiler that built it
cargo run -- list pass            # list entries in ~/.password-store (no decrypt)
cargo run -- import-key key.asc   # import an OpenPGP key (public or secret)
cargo run -- gen-identity         # generate an age identity for passage
cargo run -- show web/github.com  # decrypt and print an entry
cargo test --workspace            # unit suite
cargo clippy --workspace --all-targets
```

The same binary is both the GUI and the verbs: a bare launch opens the app; a
verb runs the command line. On Windows the console tool is a second executable,
`passpony-cli.exe`, because a GUI-subsystem binary there has no stdout.

## Layout

```
PassPonyDesktop/
├── Cargo.toml                     workspace, pinned deps, release profile
├── rust-toolchain.toml            one compiler for local builds and CI
├── PHASE0_FINDINGS.md             the backend question and the recommendation
├── PHASE0_SPIKE_RESULTS.md        the interop evidence that settled it
├── PLANNING_DESKTOP_1_0_0.md      the phase plan to phone parity
├── RELEASING.md                   the release procedure (CI + the Mac half)
├── SECURITY.md                    reporting, scope, and the security posture
├── tools/                         make-icons.py (icon set), make-dmg.sh (macOS)
├── packaging/                     the icon master/outputs, AppImage bits, .desktop
├── third_party/passponycore/      submodule: the shared Rust core
└── passpony-desktop/
    └── src/
        ├── main.rs                the GUI/CLI entry (verbs, else window)
        ├── lib.rs                 shared by the GUI and console binaries
        ├── cli.rs                 read-only verbs: version, selftest, list
        ├── app.rs                 the App struct, rail shell, drops, shortcuts
        ├── theme.rs               design system — palette, fonts, rail/card/buttons
        ├── appicon.rs             the real app icon (window + rail), from PNG
        ├── tasks.rs               worker-thread seam for git sync
        ├── panels/                the four screens + editor + onboarding
        ├── bin/passpony-cli.rs    the Windows console twin
        └── crypto/
            ├── pgp_backend.rs     pass (OpenPGP) over rpgp
            ├── age_backend.rs     passage (age) over the age crate
            └── keystore.rs        imported keys + recipient resolution
```

## License

Apache-2.0. See LICENSE.
