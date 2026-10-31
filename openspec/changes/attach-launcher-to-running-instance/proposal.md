## Why

With the systemd user service enabled, KeyPeek already runs. Launching it from the application menu then starts a *second* process: two overlays, and the newcomer cannot own the control socket because the service instance already holds it. Clicking the launcher should open the running instance's settings, and only start a process when nothing is running.

The service instance also opens its settings window on every start, because the settings visibility is passed as `true` unconditionally — a window appearing at login on a machine nobody is sitting at.

## What Changes

- Make `--settings` mean "open the settings of the running instance, or start one with settings if none is running".
- Open the settings window at start only when nothing was auto-connected, so a normal start connects quietly.
- Add `--no-settings` for the service, so a machine with the keyboard switched off at boot does not get a settings window either.
- Point the packaged desktop entry at `keypeek --settings`.

## Capabilities

### New Capabilities

- `launcher-attach`: What a launcher invocation does when an instance is already running.

### Modified Capabilities

None.

## Impact

Affects `keypeek-desktop` (argument handling and the control send path), `keypeek-presentation` (startup window visibility), `cargo-appimage.desktop`, the flake's Home Manager service, and the documentation. The read-only configuration interaction is unchanged; the running instance keeps whatever configuration it was started with.
