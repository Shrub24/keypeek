## Context

`main` parses either a run invocation (`--config <path>` optional) or a control command, and control commands send one line to the socket and exit. `--config` combined with a control command is rejected.

The desktop entry's `Exec` is rewritten to the store binary at package time, so it currently launches a full instance. A second instance binds no control socket (the first one owns it) and prints a warning, but it still runs an overlay and connects to the keyboard, which upstream's auto-connect now does without being asked.

`KeyPeekAppCore::new` receives `settings_visible` from its callers, which pass `true` unconditionally.

## Goals / Non-Goals

**Goals:**

- Clicking the launcher configures the instance that is already running.
- No second overlay when a service instance exists.
- No settings window at login for a service-managed machine.

**Non-Goals:**

- Multi-instance management, instance naming, or a daemon that owns several overlays. One instance owns the control socket; the rest run uncontrolled, as today.
- Making the settings window able to write configuration that a declarative store has made read-only. Session-only changes are the accepted behaviour.
- Attach-or-start behaviour for `--toggle`/`--show`/`--hide`/`--auto`: a hotkey that silently starts an overlay would be worse than one that does nothing.

## Decisions

### `--settings` attaches, and starts only if nothing answers

The command already exists on the wire; only the failure path changes. When the socket refuses the connection, the same process starts normally with the settings window visible instead of exiting non-zero. That is what a launcher click should mean, and it needs no second binary or a separate flag.

Hotkey commands keep their current behaviour — fail with a message and a non-zero exit — because they are bound to keys, not clicked. This split is why `--settings` may be combined with `--config` (attach to whatever runs; if starting, use that configuration) while the other commands still reject it.

### Two conditions for the startup window

`settings_visible` at start becomes `requested && !auto_connected`, where `requested` is true unless `--no-settings` was passed. So:

- Fresh install, nothing remembered: settings opens, which is where the device list lives.
- Remembered keyboard present: connects quietly, no window.
- Service with the keyboard switched off at boot: `--no-settings` in the unit keeps the window closed; the overlay appears when the keyboard does.

`--no-settings` is a flag rather than a config key because it describes how the *process* was started, not user preference, and it must not be persisted.

### The desktop entry uses the same path a user would

`Exec=keypeek --settings` in `cargo-appimage.desktop`, so the .deb/AppImage and the flake package behave identically, and the flake's rewrite of `Exec` only has to substitute the binary path. The Home Manager service gets `--no-settings` appended to its `ExecStart`.

## Risks

- **Race at login.** The service may still be starting when the launcher is clicked; the send fails and a second instance starts. The window for this is the service's first second.
- **Two instances anyway.** If the service crashes and the user starts KeyPeek from the menu, that instance owns the socket; when the service restarts it runs uncontrolled. `Restart=on-failure` narrows the window, and the existing "another instance is listening" warning makes the state visible.
- **Read-only configuration.** Settings opened from the launcher change the running instance only, because its store is the generated file. Documented rather than worked around.
