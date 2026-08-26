# Packaging

The icon master and the installer inputs for every platform. Regenerate the
icons with `tools/make-icons.py` (run from the repo root); build the artifacts
with the release workflow (`.github/workflows/release.yml`) or, for the macOS
disk image, `tools/make-dmg.sh`. `RELEASING.md` is the full procedure.

```
packaging/
├── passpony-master-1024.png   the 1024px master (mirror of the iOS icon)
├── passpony.png               Linux — cargo-deb and the AppImage (512px, squircle)
├── passpony.ico               Windows — AppImage-adjacent copy (7 sizes)
├── passpony.icns              macOS — the .app bundle (inset to the 824/1024 grid)
└── appimage/
    ├── AppRun                 the AppImage entry point (execs usr/bin/passpony)
    └── passpony.desktop       the freedesktop entry (no MimeType by design)
```

The Windows MSI icon lives at `../passpony-desktop/wix/Product.ico`, written by
the same `make-icons.py` run so `wix/main.wxs` needs no path edit. The deb and
MSI metadata are in `../passpony-desktop/Cargo.toml` (`[package.metadata.deb]`
and `[package.metadata.wix]`).

Icons are generated artifacts: safe to delete and re-run. The master is the iOS
app icon; if it changes there, refresh `passpony-master-1024.png` and re-run
`tools/make-icons.py`.
