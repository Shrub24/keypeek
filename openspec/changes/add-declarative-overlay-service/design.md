## Context

`KeyPeekAppCore::is_overlay_visible` currently returns `is_any_window_open() || keyboard.overlay_is_visible(now)`, where the keyboard side is the layer-driven `VisibilityStateMachine`. The desktop hosts call it once per frame to decide whether to reveal the overlay window, so the decision point is already single and shared. The only external request today is the tray's Settings… entry, which sets an `Arc<AtomicBool>` that `DesktopOverlayApp::update` polls once per frame.

KeyPeek is started either by the user or by a systemd user service. In the declarative case the process is long-lived and the settings store is read-only, so state that must survive a restart belongs in the generated INI and state that describes the current sitting does not.

## Goals / Non-Goals

**Goals:**

- Let anything on the machine toggle, show, hide, or reset the overlay of a running instance.
- Keep the override out of persisted settings so a declarative configuration stays authoritative.
- One command channel for every external request, including the tray.
- No new dependency and no protocol that needs a daemon or session bus.

**Non-Goals:**

- Persist the override across restarts, or reconfigure the overlay from the command channel.
- Control the overlay from a different user, over the network, or over D-Bus.
- Register global shortcuts; the compositor owns key bindings and now has a command to bind.
- Change layer-driven behavior when the override is `Auto`.

## Decisions

### A line protocol on a Unix socket in the runtime directory

The instance listens on `$XDG_RUNTIME_DIR/keypeek.sock` (falling back to the temporary directory), mode `0600`, and reads one command per line from each connection and closes it. The client is the same binary, so no second executable, no D-Bus dependency, and any compositor can bind a key to `keypeek --toggle`.

Alternatives: D-Bus would add `zbus` and interface paperwork for five methods; a separate `keypeekctl` binary duplicates argument parsing and packaging for no gain. A `--toggle` that writes state to the settings file and asks the running instance to reload would break under the read-only store.

Binding behavior: the socket is created at startup. A leftover file from a killed process is removed after a failed connection attempt, and a second instance that finds a live socket reports that instead of silently replacing it.

### Tri-state override, applied at the single visibility decision

`OverlayOverride` has `Auto` (layer-driven), `Shown` (visible while a keyboard is connected), and `Hidden`. It lives in `UiState` and is consulted inside `is_overlay_visible`, so both desktop hosts and the overlay drawing path pick it up without knowing it exists. `Shown` still requires a connected keyboard, since the overlay draws that keyboard; `Hidden` never hides the overlay while a settings or editor window is open, because the key editor needs the overlay to click keys.

`Toggle` is resolved at drain time against the effective visibility: visible becomes `Hidden`, invisible becomes `Shown`. That is what a hotkey should mean, and it keeps the client stateless.

### One queue for external requests

`DesktopOverlayApp` gains an `Arc<CommandQueue>` of `OverlayCommand` values, drained at the top of `update`. The socket listener thread pushes into it and calls `UiWake`, and the tray pushes `OverlayCommand::Settings` instead of setting its own flag, which removes `settings_requested`.

Alternative: store the override itself in an atomic shared with the listener. Rejected because `Toggle` depends on the current layer state, which only the UI thread knows.

### Control commands exit without starting the application

`main` parses arguments into either a run invocation (optional `--config`) or a control command. A control command connects, writes one line, and exits; it never constructs the app, discovers devices, or creates a window. Combining `--config` with a control command is rejected, so a typo cannot silently start a second instance with the wrong configuration.

## Risks

- **Socket left behind by a crash.** Handled by the connect-then-unlink sequence; a stale socket refuses connections immediately, so `--toggle` fails fast with a message rather than hanging.
- **Two instances.** The second one finds a live socket and reports it; it still runs, since running two overlays is a legitimate way to watch two keyboards, but it does not take over the control socket.
- **Override forgotten.** `Shown` keeps the overlay on screen past the configured timeout. The tray menu shows the current state and offers `Auto` as the way back, and the override dies with the process.
