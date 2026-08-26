# Releasing PassPony Desktop

Each release ships eight artifacts — a signed and notarized **`.dmg`** (macOS, **universal**), an
**`.msi`** (Windows, x64), and six for Linux: a **`.deb`**, a portable **`.tar.gz`** and an
**`.AppImage`**, each for both **x86_64** and **ARM64** — plus a detached `.asc` for every one of
them and a signed `SHA256SUMS` covering the lot. Nine signatures.

Both architectures carry an explicit suffix; the names are symmetric.

The work is split because the signing credentials must never reach a hosted runner:

| Where | What |
| --- | --- |
| **CI**, on a tag push | the six Linux artifacts + the `.msi`, into a **draft** release |
| **Your Mac** | the `.dmg`, notarization, every PGP signature, and publishing the draft |

**This repository holds no secrets.** The Developer ID certificate never reaches a hosted runner,
and neither does the PGP release key. A compromised Actions run cannot sign anything.

## The one thing that differs from AgePony's release

The release *shape* is AgePony Desktop's, deliberately — same trigger, same draft flow, same naming,
same signing scheme, the same universal-macOS and `ubuntu-22.04` decisions. The difference that
matters:

- **PassPonyCore is a git submodule.** Every checkout in `release.yml` and `build.yml` is
  `submodules: recursive`. Without it the first `cargo` call fails on a missing path dependency, and
  a fresh clone for a local release must be `git clone --recurse-submodules` (or
  `git submodule update --init --recursive` after the fact).

The CLI checks also match PassPony's own verbs: `selftest` round-trips both crypto backends and
prints `passage (age) : ok` / `pass (openpgp) : ok`; `keys` opens the config directory. There is no
post-quantum reference vector here — that is AgePony's, because age is where ML-KEM lives.

## 0. First release only — the repository does not exist yet

The working tree was assembled from delivered tarballs: it has `.gitmodules` and the
PassPonyCore source under `third_party/passponycore/`, but as **plain files**, not a real
submodule gitlink, and there is no `.git` yet. Wire the submodule properly so CI's
`submodules: recursive` checkout has something to resolve, then create the repo.

```sh
cd ~/Apps/PassPonyDesktop
git init

# Replace the vendored plain files with a real submodule pinned to the phone-release SHA.
rm -rf third_party/passponycore
rm -f .gitmodules
git submodule add https://github.com/norsehorse-dev/PassPonyCore.git third_party/passponycore
git -C third_party/passponycore checkout 692d568085e5616b0984974481e0ff12884d64b2
git add .gitmodules third_party/passponycore

# Everything else.
git add -A
git commit -m "PassPony Desktop 1.0.0"
```

**Check `.gitignore` first** — it already covers `/target`, `/dist`, `__pycache__/` and, the one
that matters, `*-release-env`. `git status` should not list any credential file.

Confirm the submodule landed at the intended commit before going further — this is the pin CI and
every local build resolve:

```sh
git -C third_party/passponycore rev-parse HEAD   # -> 692d568085e5616b0984974481e0ff12884d64b2
git submodule status                             # the SHA should have no leading + or -
cargo build --release                            # proves the gitlink resolves and builds
```

Then create the repo and push `main` before the first tag — the release workflow triggers only on
`v*`, so pushing `main` first gives a free run of `build.yml` in the Actions tab:

```sh
gh repo create norsehorse-dev/PassPonyDesktop --public --source=. --push \
  --description "pass and passage password stores on the desktop — native Rust, interoperable with PassPony on iOS and Android"
```

Homepage and topics are not create-time flags, so set them once the repo exists:

```sh
gh repo edit norsehorse-dev/PassPonyDesktop \
  --homepage "https://passpony.app/desktop" \
  --add-topic rust,egui,password-manager,pass,passage,openpgp,age,gpg,totp,encryption,cross-platform,desktop
```

**The release workflow cannot be dry-run.** Expect the first tag to surface something; AgePony's
first tag took three attempts, and the MSI is the least-exercised artifact.

If you would rather **not** use a submodule, the fallback is to keep `third_party/passponycore/` as
committed plain files and delete `.gitmodules`: the build still works (the path dependency resolves
to the committed files) and CI's submodule step becomes a harmless no-op. You lose the explicit pin
discipline, so this is the lesser option — but it is a valid one if the PassPonyCore repo is not
ready.

## 1. Before tagging

**One number, one place.** `version` in the workspace `Cargo.toml` feeds `passpony version`, the deb,
the MSI and the Info.plist. Nothing else declares it.

```sh
git submodule update --init --recursive
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
cargo fmt --all --check
cargo run --release --bin passpony -- selftest
git status --short
git push
```

A tag on a tree with uncommitted work produces a release that does not contain it. **Confirm the
submodule pin is the one you intend** — `git -C third_party/passponycore rev-parse HEAD` should be a
phone-release SHA, not a drifting `main`.

If the icons changed, regenerate them and commit the result — the master lives in the iOS repo:

```sh
tools/make-icons.py
```

## 2. Tag — CI builds the seven

```sh
git tag v1.0.0
git push origin v1.0.0
sleep 15
gh run watch $(gh run list --workflow=release.yml --limit 1 \
  --json databaseId --jq '.[0].databaseId')
```

The `sleep` matters: `gh run list` fires before the new run registers and hands you the *previous*
run's id, which then reports success for work you are not watching.

Three build jobs — `linux (x86_64)`, `linux (aarch64)` and `windows`. The two Linux legs are one
matrix job deliberately: an ARM lane maintained as a copy of the x86_64 lane drifts, and a lane that
drifts is a lane whose assertions quietly stop covering it. Read the log the first time, not just the
checkmark:

- **the deb's identity** — `Package` must be `passpony`, lowercase; `Architecture` must match the
  lane; `Depends` must be non-empty and mention `libc6`. An empty `Depends` means `$auto` did not
  scan the ELF, and the deb installs onto a machine with no GL and fails at launch.
- **the desktop entry** — passes `desktop-file-validate`, has a real `Categories`, and carries **no**
  `MimeType`. The last one is a tripwire: PassPony implements no handler for an opened file, so if
  associations are added later this fails and forces the `.desktop` and the routing code to land
  together.
- **`selftest` passes in the shipped binary**, on all three platforms and both architectures — both
  backends `ok`, no `FAIL`, exit 0. This is the check that the crypto works in *this* build on *this*
  architecture, which a test suite running only on the build machine cannot see.
- **`keys` opens the config directory** — the only check that touches the config directory outside a
  dev environment; exits 0 with no keys imported.
- **on Windows, both binaries exist, `passpony-cli.exe` prints, and `passpony.exe` is GUI-subsystem.**
  The last one is not redundant: if `windows_subsystem = "windows"` were dropped, the GUI would flash
  a console window on every launch and nothing else would notice.

## 3. On the Mac — the dmg

The four values live in a file outside the repo, `chmod 600`, shared with the other pony desktops
because the Apple credentials are account-level.

```sh
source ~/.pgpony-release-env      # MACOS_SIGN_IDENTITY, NOTARIZATION_{APPLE_ID,PASSWORD,TEAM_ID}
tools/make-dmg.sh
```

The script builds both architectures, `lipo`s them together, writes and **reads back** the
`Info.plist`, signs with the hardened runtime, notarizes, staples, and then verifies the app inside
the mounted image. Expect several minutes of apparent inactivity at the notarization step — that is
Apple's service, not a hang.

Gotchas, all inherited from AgePony and all still current:

- `MACOS_SIGN_IDENTITY` is the certificate **name**, not its SHA-1 hash.
- Only **one** `Developer ID Application` certificate may be in the keychain, or codesign reports
  "multiple matching certificates". List with `security find-identity -v -p codesigning`, delete
  keyless extras with `security delete-certificate -Z <hash>`.
- **The dmg container is deliberately unsigned.** The `.app` is signed and notarized and the ticket
  is stapled to the image. `spctl -a -t open --context context:primary-signature` therefore
  **rejects a perfectly good release** with `source=no usable signature`, because it is asking an
  unsigned container a signature question. `make-dmg.sh` runs the checks that mean something —
  `spctl -a -t exec` on the app inside the mounted image, and then actually running it.

Confirm `lipo -info` reported **both** `x86_64` and `arm64`, and that `selftest` exited 0 with both
backends `ok`.

## 4. Assemble, sign, publish

Pull CI's seven down beside your dmg, then sign all eight together so `SHA256SUMS` covers the exact
bytes that ship.

```sh
mkdir -p ~/passpony-release && cd ~/passpony-release
gh release download v1.0.0 --repo norsehorse-dev/PassPonyDesktop --dir .
cp /Users/kevinstewart/Apps/PassPonyDesktop/dist/PassPony-macOS.dmg .

FILES=(
  PassPony-macOS.dmg
  PassPony-linux-x86_64.deb
  PassPony-linux-x86_64.tar.gz
  PassPony-x86_64.AppImage
  PassPony-linux-arm64.deb
  PassPony-linux-aarch64.tar.gz
  PassPony-aarch64.AppImage
  PassPony-windows.msi
)

shasum -a 256 $FILES > SHA256SUMS
cat SHA256SUMS
for f in $FILES SHA256SUMS; do
  gpg -u A0CBC8F65AACE56F1C5B767753F9798E4919DE62 --armor --detach-sign "$f"
done
for f in $FILES SHA256SUMS; do
  gpg --verify "$f.asc" "$f"
done
shasum -a 256 -c SHA256SUMS
```

The same NorseHorse release key signs AgePony, BurnPony, PGPony and RelayPony — one key to publish,
one fingerprint for users to learn, one revocation to handle if it ever comes to that.

An **array**, not a space-separated string. This shell is zsh, and zsh does not word-split an
unquoted `$var` the way bash does. The explicit `-u` is not decoration either: a bare
`gpg --detach-sign` fails with "no default secret key" when the release key is not the default.
Count **nine** `Good signature` lines before going any further. Verify before publishing, not after.

```sh
gh release upload v1.0.0 PassPony-macOS.dmg *.asc SHA256SUMS \
  --repo norsehorse-dev/PassPonyDesktop
gh release edit v1.0.0 --draft=false --latest --repo norsehorse-dev/PassPonyDesktop
```

`--latest` is load-bearing. `releases/latest/download/…` — the stable, versionless URLs handed out in
a README or used by anyone scripting an install — resolve to whichever release carries the "latest"
flag, not to the newest tag.

## 5. Verification checklist

Artifacts:

- [ ] `gpg --verify` each `.asc`, and `SHA256SUMS.asc` against `SHA256SUMS` — nine in total
- [ ] `shasum -a 256 -c SHA256SUMS` passes (macOS has no `sha256sum`; the format is identical, so
      Linux users can verify later with `sha256sum -c`)
- [ ] the tarball extracts and `PassPony/passpony version` prints, once by hand on a real ARM machine
- [ ] the AppImage is executable and runs, and double-clicking it opens the GUI on a desktop with FUSE
- [ ] `selftest` exits 0 with both backends `ok` from the **installed** artifact on every OS
- [ ] `selftest`'s version line reports the same rustc on all three platforms. The dmg is the
      artifact at risk, because it is the only one CI does not build
- [ ] `keys` prints its header and exits 0 on a machine with no keys imported

macOS:

- [ ] `lipo -info` on the installed binary reports both architectures
- [ ] `xcrun stapler validate` on the dmg
- [ ] `spctl -a -t exec -vv` on the app inside the mounted dmg says `Notarized Developer ID`
- [ ] a quarantined dmg opens with no dialog, and the app launches from `/Applications` without a prompt
- [ ] **the Dock icon is inset and squircle-cornered**, not a hard-edged square sitting larger than
      its neighbours. macOS does not mask, so this is `tools/make-icons.py`'s 824/1024 grid being
      right, and it is only visible in the Dock
- [ ] test on an **Intel** Mac if one is reachable — this is PassPony's first universal build

Windows:

- [ ] the MSI installs on a clean machine and the Start Menu entry appears
- [ ] `passpony.exe` opens the GUI with **no console window flashing behind it**
- [ ] `passpony-cli.exe selftest` prints and exits 0
- [ ] SmartScreen behaviour recorded for the release notes — the MSI is unsigned

Every OS:

- [ ] open a real pass store and a real passage store through the GUI, reveal and copy a secret, and
      watch a TOTP code tick
- [ ] an entry written here opens in PassPony on iOS or Android, and one written there opens here
- [ ] git sync clones, publishes, fetches and pushes; a conflict prompts per file
- [ ] the GUI opens in both light and dark mode and nothing is clipped

## 6. Not set up yet — known gaps

Listed so they are decisions rather than oversights:

- **The site.** `passpony.app/desktop` has no download page yet. Until it does, the GitHub release is
  the only download channel.
- **winget / AUR.** No manifests. When winget starts: `ProductCode` is **not** the `UpgradeCode` in
  `wix/main.wxs` — different GUIDs. An AUR `passpony-bin` PKGBUILD must be pushed **after** the
  release is published, or it points at a draft and every user gets a 404.
- **The MSI is the least-verified artifact.** `wix/main.wxs` was adapted from the `cargo wix init`
  template by hand and has not been through WiX here. CI installs the MSI and runs the binary out of
  it, so a broken source fails the tag rather than a user — but expect the first tag to be where this
  is found.
- **Single-instance forwarding and a native menu bar** are not implemented. They are per-OS and were
  deferred out of the D8 shell; opening a second window is harmless today, and the rail is the app's
  navigation. Revisit if either becomes a real annoyance in use.
