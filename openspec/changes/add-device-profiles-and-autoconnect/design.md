## Context

`KeyPeekAppCore::new` loads settings, builds a `DeviceConnectionManager` over the discovered devices, and starts disconnected. A connection begins when the user selects a device in the settings window and presses Connect, which records a `ConnectionSpec` for the session; if the device drops, `ConnectionState::Reconnecting` retries that same spec every couple of seconds. Nothing survives the process.

`SettingsStore` is the only persistence seam today, and it is deliberately made read-only under `--config` so a declarative configuration stays authoritative. Last-used state is not configuration: it changes as a consequence of use, and a generated Nix file must not be rewritten.

## Goals / Non-Goals

**Goals:**

- Connect on start, without user interaction, to the keyboard used last.
- Keep the record out of the configuration file entirely.
- Match a keyboard that moved to a different port or BLE address.

**Non-Goals:**

- Multiple saved profiles, per-device settings, or a profile manager.
- Distinguishing two physically identical keyboards connected at the same time.
- Changing what happens on a dropped connection; the retry loop stays as upstream wrote it.
- Auto-scanning for devices that appear after start.

## Decisions

### A second store, separate from `SettingsStore`

`SessionStateStore` mirrors the shape of `SettingsStore` — a trait, a file-backed implementation, an in-memory one for tests and the browser — but writes to the state directory (`$XDG_STATE_HOME/keypeek/last-device`), not the config directory. Using the settings store would either be silently dropped under `--config` or would rewrite a Nix-generated file; both are worse than a second small seam.

State is a single line, `driver:vid:pid:name`. It is not configuration the user edits, so a structured format buys nothing over one human-readable line.

### Identity is driver, VID/PID and name

`DeviceIdentity` captures what stays true across sessions. The serial port path and BLE address identify a connection, not a keyboard, and change between reboots and re-pairings — recording them would make the entry stop matching exactly when it is needed. Two identical boards are therefore not distinguished; the first match in discovery order wins.

The name is included because a board's VID/PID often matches a whole family, and the discovery name is what the user sees in the list.

### Auto-connect happens once, at construction

`KeyPeekAppCore::new` looks up the remembered identity, selects the matching device if one is present, and connects. Doing it there means every frontend gets the behavior with no additional wiring, and the connection is a normal one — same task, same events, same retry — so nothing downstream needs to know it was automatic.

If no remembered device matches, nothing happens: no fallback to "the first available device", because connecting to a keyboard the user did not choose is worse than starting disconnected.

### The record is written on success, not on click

`update_connection` already handles `ConnectionEvent::Connected`. Saving the identity there means a device that appears in the list but fails to connect, is locked, or needs a layout file never becomes the remembered one.

## Risks

- **Stale record.** The device was renamed or its VID/PID changed by a firmware update, so it never matches again. The outcome is today's behavior — start disconnected — and the record is overwritten on the next successful connection.
- **Record written from a read-only configuration.** Deliberate: the state file is not configuration. It lives under the state directory, so nothing in the declarative setup is rewritten.
- **Unwanted connection on start.** The service starts, sees the remembered board, and connects; that is the point of the feature and the overlay follows its own visibility rules.
