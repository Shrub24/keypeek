# KeyPeek Architecture

KeyPeek connects to keyboards, mirrors their current layers, and draws a live on-screen overlay, with an integrated keymap editor. It is written in Rust and uses the egui immediate-mode GUI stack. This document describes the crate boundaries, the runtime path from process start to overlay, and the ports through which platforms and frontends plug in. It describes current behavior only.

## Crate boundaries

The project is a Cargo workspace. Dependencies point one way only: `keypeek-core` ← `keypeek-protocol` ← `keypeek-presentation` ← frontends. `crates/keypeek-desktop` and `crates/keypeek-web` are the two frontends and depend on nothing else in the workspace; `default-members` is the desktop crate, so `cargo build` builds the native application.

- **`keypeek-core`.** Firmware-agnostic domain, with no I/O and no GUI: physical layout geometry (`layout`, `layout_key`, `key_matrix`), protocol-agnostic key specifications (`key_spec`), the `KeyboardDomain` aggregate and its overlay `VisibilityStateMachine` (`keyboard`, `visibility`), and the `KeyPresenter` and `EditorProfile` traits that firmware adapters implement.
- **`keypeek-protocol`.** Everything that talks to hardware or describes it: transports (`protocols::transport`, `platform::hid` for native, `platform::web_*` for the browser), per-firmware bundles (`firmware::qmk`, `firmware::zmk`), device discovery scanners (`application::device_discovery` plus one scanner per bundle), the connection lifecycle (`application::connection_manager`), per-keyboard background sessions (`application::session`), key legend resolution (`hid_labels`, `key_presenter`, `os_layout`). The `desktop` feature is on by default and gates everything that needs native operating-system APIs; it is off for the browser build.
- **`keypeek-presentation`.** The egui application itself, independent of windowing: `KeyPeekAppCore` holds settings, connection and editor state, and the desktop and web runners share it through differing hosts (`overlay_window`), together with the settings window, the keymap editor UI, key painting and widget helpers. Its `directories` feature gates on-disk configuration paths and is off for the browser build.
- **`keypeek-desktop`.** The native binary `keypeek`. It parses arguments, builds the settings store, runs discovery, and dispatches to a host: on native Wayland a wlr-layer-shell surface driven directly through the EGL/Wayland client stack, otherwise the eframe windowing host, plus the system tray and native file dialogs.
- **`keypeek-web`.** The browser runner over WebHID, Web Serial and Web Bluetooth, with `localStorage`-backed settings and the same `KeyPeekAppCore`.

## Runtime path

1. `keypeek-desktop`'s `main` registers OS-layout support, parses `--config`, and selects a settings store: `FileSettingsStore` for the default location, `ReadOnlySettingsStore` for an explicitly named file.
2. Discovery runs once at start: HID enumeration is shared with the registered firmware scanners (`discover_devices(scan_all_hid())`), and the resulting device list is handed to the frontend.
3. The frontend calls `keypeek_desktop::run(settings_store, devices)`, which picks the host and constructs `DesktopOverlayApp`, whose `KeyPeekAppCore` loads settings and owns a `DeviceConnectionManager`.
4. Connecting a device starts a background session; the session pumps protocol events and applies commands, and wakes the UI through `UiWake` when state changes. The overlay repaints from the domain's layer state rather than from a local simulation.

Devices are not rescanned while running; a device that appears later needs a restart.

## Firmware bundles

Each firmware family is a self-contained bundle implementing `FirmwareBundle`: it provides its discovery scanners, its key presenter, its editor profile, and a `connect` that turns a `ConnectionSpec` into a `KeyboardProtocol`. `bundle_for_spec` selects one, so no application code branches on firmware.

- **QMK / VIA.** RAW HID on usage page `0xFF60`; VIA protocol version 12 or newer. Because VIA exposes no physical layout, the QMK path reads it from a `keyboard_info.json` exported from the user's keymap, and can download and cache the VIA layout catalog.
- **Vial.** RAW HID with the Vial command protocol, fetching layout and keymap from the device. Vial devices are recognized by a `vial:` serial marker.
- **ZMK.** ZMK Studio RPC over a serial (USB-UART) or Bluetooth transport, with live layer updates over a separate RAW HID channel on usage page `0xFF60`. A locked device prompts the user to unlock it with the ZMK Studio key combination.

For QMK and Vial, layer-change events come from a small firmware module that notifies over the RAW HID channel, kept alive by a periodic keepalive; `EventSubscription` releases the background reader thread when dropped.

## Device discovery

Discovery is two-phase. Transport snapshots (the HID interfaces) are collected once into a `DiscoveryContext`; each registered scanner then contributes `DiscoveredDevice` entries and may claim a VID/PID, which suppresses later scanners' entries for the same identity. The ZMK bundle scans first, so a ZMK serial or BLE observation replaces the QMK entry for that identity.

- **QMK and Vial** come from Raw HID interfaces on usage page `0xFF60`, one entry per identity.
- **ZMK serial** comes from every USB serial port, which is currently taken as a ZMK Studio transport without a positive check.
- **ZMK BLE** comes from devices exposing the ZMK Studio service (the Studio client's `BleDiscoveryMode::Any` resolves to the OS's connected-device list filtered by that service) and is attached to a HID identity by product name.

At the 0.11.0 base a claimed identity also suppresses its Vial entry, and a plain UART adapter is listed as a keyboard; both are upstream behaviors the downstream currently accepts.

## Connection lifecycle

`DeviceConnectionManager` is an explicit state machine shared by both frontends: `Disconnected`, `Scanning`, `Connecting`, `RequiresLayoutFile`, `Locked`, `Connected` (the `Keyboard` plus its editor profile), `Reconnecting` with attempt count and next-try instant, and `Error`. Reconnect attempts and the resulting events are dispatched to the app core, which is the only place UI state changes.

## Overlay visibility

`KeyboardDomain` owns a `VisibilityStateMachine`. On every layer-state update it classifies the state as `Selected`, `Excluded` or `Base` from the visible-layer mask, and produces a `VisibilityWindow` (`from`, optional `until`): a held visible layer shows the overlay immediately after the activation delay, a released one keeps it for the configured timeout, and a negative timeout keeps it visible until the layer state changes. Hosts ask the window whether it is visible at a given instant and sleep until `changes_in`. Visibility is therefore driven entirely by the keyboard; there is no manual show/hide control.

## Keymap editor

Opening the editor is a presentation action; the catalog and the write path are firmware-owned. Each bundle provides an `EditorProfile` describing feature support, the catalog of keycodes and behaviors for its firmware, and how to encode a selected `KeySpec`. Writes go through the protocol trait, which reports its `WriteSupport`: QMK and Vial persist every change immediately, ZMK stages changes in a Studio session that is committed when the editor closes, and protocols that cannot write leave keys read-only. The rule is firmware-owned, so the browser build behaves identically.

## Key legends

A firmware adapter converts the device's raw binding into a `KeySpec`; the bundle's `KeyPresenter` then turns that into a visual `LayoutKey` — a base legend, a shifted legend, modifier information and styling. Where a legend depends on the host layout, `os_layout` supplies the character the active OS keyboard layout produces for a usage, with a backend per platform and a fallback to the static HID usage tables. The overlay draws stacked base and shifted legends, a single legend, or a single legend with a live Shift/AltGr preview, and shrinks a label before ellipsizing it when auto-fit is enabled.

## Ports and platform integration

Platform differences are confined to narrow ports:

- **`SettingsStore`.** `load`/`save` over INI. `FileSettingsStore` uses the platform config directory (`dev/srwi/KeyPeek` via `directories`); `MemorySettingsStore` backs tests and the browser; `ReadOnlySettingsStore` reads one explicit file, never falls back to the default path, and drops saves, which is what the declarative Nix configuration relies on.
- **`RawHidTransport` and `Reopener`.** The device I/O seam, implemented by `hidapi` natively and by WebHID in the browser.
- **`OverlayHost`.** Requests of the window host: mouse passthrough and close.
- **`UiWake`.** The single wake-up channel from background sessions to the UI thread.

Native specifics: the Wayland host builds a wlr-layer-shell surface anchored to all edges and sized by the compositor, falling back to eframe on XWayland when layer shell is unavailable (for example on GNOME/Mutter); the tray runs on a dedicated GTK thread on Linux because libappindicator registers its StatusNotifierItem over DBus through the glib main loop, and offers Settings… and Quit.

## Settings

Settings cover window size and position, timeout (including an infinite setting), margin, activation delay, the visible-layer mask, legend mode, font size multiplier, ellipsis behaviour, the font colour, and per-layer theme colours. The settings window edits a draft and commits it to the active settings; persistence goes through the store, so a read-only store makes the window's changes session-only.
