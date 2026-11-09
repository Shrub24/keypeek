# KeyPeek Technology Stack

KeyPeek is a Rust workspace (edition 2021, version 0.11.0, GPL-3.0-only) producing a native desktop application and a browser application from the same core. This document records the language, dependencies, and build/package tooling as they exist today.

## Workspace

`crates/keypeek-core`, `keypeek-protocol`, `keypeek-presentation`, `keypeek-desktop`, `keypeek-web`; shared versions and dependencies are declared once in the root `[workspace.dependencies]`, and `default-members` is `keypeek-desktop`. See [architecture.md](architecture.md) for the boundary between them.

Two features cross the layer boundary, both off for the browser build: `desktop` (native operating-system APIs — `hidapi`, `serialport`, Bluetooth enumeration) and `directories` (on-disk configuration paths).

## Language and dependencies

Rust on the stable toolchain. Notable dependencies:

- **GUI**: `egui` / `eframe` (glow backend), plus `egui_extras`, `egui-phosphor`, `egui-file-dialog`, `image` for icons and textures.
- **Keyboard protocols**: `qmk-via-api` for QMK/VIA and Vial over RAW HID; `zmk-studio-api` (serial and BLE features) for ZMK; `hidapi` for HID enumeration and access; `serialport` for serial transports; `prost` for ZMK Studio protobufs; `lzma-rs` for decompressing XZ-compressed Vial keyboard definitions. Both protocol crates are published libraries maintained alongside this project.
- **Async plumbing**: `futures-util` / `futures-channel` for the event streams that connect background readers to sessions.
- **Platform integration**: `tray-icon` for the system tray; on Linux `gtk`, `winit` (X11), `smithay-client-toolkit`, `wayland-client`, `wayland-egl` and `khronos-egl` for the native Wayland overlay, plus `xkbcommon` and `x11rb` for reading the active keyboard layout; on macOS `winit` and `objc2-app-kit`; on Windows the `windows` crate (Bluetooth/radios, DWM, and keyboard-layout APIs) and `winreg`.
- **Data and misc**: `serde` / `serde_json` for JSON definitions, `rust-ini` for the settings file, `directories` for config paths, `web-time` instead of `std::time` so the same code compiles to `wasm32`.
- **Browser build**: `wasm-bindgen`, `wasm-bindgen-futures`, `web-sys` and the WebHID, Web Serial and Web Bluetooth APIs; `Trunk` bundles it (`Trunk.toml`, `index.html`).

## Build and packaging

- **Nix flake** is the primary Linux build. It uses `crane` with a `fenix` Rust toolchain and `flake-utils`. The flake exposes the `keypeek` package (wrapped with the required `LD_LIBRARY_PATH`, plus the desktop entry, scalable icon, and the BLE udev rule under `lib/udev/rules.d`), the `homeManagerModules.default` and `nixosModules.default` outputs, `nix flake check` checks (package build, `cargo-nextest` over the workspace, pre-commit Nix hooks, and evaluations of both modules), a `keypeek-llvm-coverage` package, and a dev shell carrying the same library path. The flake defines alejandra, deadnix, and statix hooks; `direnv` loads the development shell via `.envrc`.
- **Linux AppImage** is produced by `cargo-appimage` with `appimagetool`, using the desktop entry in `cargo-appimage.desktop` and `resources/icon.png`.
- **Windows** builds a plain release executable; `keypeek-desktop`'s `build.rs` embeds the icon via `winresource`.
- **macOS** builds a `.app` bundle with `cargo-bundle`; bundle metadata lives in `crates/keypeek-desktop/Cargo.toml` (`package.metadata.bundle`) with an Info.plist extension in `resources/macos-info-plist-ext.xml`.
- **Browser** build is a `wasm32-unknown-unknown` bundle published to GitHub Pages.

## CI

Three GitHub Actions workflows:

- **Build and release** (`workflow.yml`) builds the Linux AppImage, the Windows executable, and macOS bundles for both `aarch64` and `x86_64`, then drafts a GitHub release with the artifacts. Tagged releases are signed with build provenance attestations.
- **Evaluate Nix** (`evaluate-nix.yml`) installs Nix and runs `nix flake check` to validate the flake and build the package.
- **Deploy pages** (`deploy-pages.yml`) builds and publishes the browser app.

The workspace test gate runs `cargo nextest --workspace`, so the browser crate must keep compiling for the host target even though it is only shipped as WebAssembly.
