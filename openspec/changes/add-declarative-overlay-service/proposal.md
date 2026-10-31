## Why

Overlay visibility is driven entirely by held layers, so the user cannot hide the overlay without stopping the process, and no tool outside the process can show, hide, or toggle it. Window managers bind keys to commands, and a declarative setup needs a command to bind; a tray menu is not enough. Upstream PR #17 proposed a CLI toggle but bundled it with unrelated keyboard work and has since conflicted.

## What Changes

- Add a local control socket to the running desktop instance, with one command per line.
- Add a manual visibility override with three states: layer-driven (`Auto`), pinned visible (`Shown`), and hidden (`Hidden`). The override lasts until it is changed again and is never written to settings.
- Add `--toggle`, `--show`, `--hide`, `--auto`, and `--settings` to the `keypeek` binary, which forward the matching command to a running instance and exit.
- Route the existing tray request through the same command channel instead of a dedicated flag.
- Add Show, Hide, and Auto entries to the tray menu.

## Capabilities

### New Capabilities

- `overlay-control`: External show/hide control of the running overlay and its interaction with layer-driven visibility.

### Modified Capabilities

None.

## Impact

The change affects `keypeek-desktop` (argument handling, socket listener, tray menu) and `keypeek-presentation` (override state and command handling). It adds no dependency: the socket uses `std::os::unix::net`. The browser build is unaffected; the override lives in shared presentation state but only the desktop runner feeds it.
