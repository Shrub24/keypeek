## Purpose

Give a running KeyPeek instance an external, local control surface for showing, hiding, and resetting the overlay, without turning the overlay's layer-driven behavior into global state that has to be persisted.

## ADDED Requirements

### Requirement: A running instance accepts local control commands
The desktop runner SHALL listen on a Unix domain socket in the user's runtime directory for the lifetime of the process and accept one command per connection.

#### Scenario: A command is delivered to a running instance
- **WHEN** a client connects to the control socket and sends `toggle`
- **THEN** the running overlay applies the toggle and repaints

#### Scenario: No instance is running
- **WHEN** a control command is issued and nothing is listening on the socket
- **THEN** the command fails with a message naming the socket and the exit status is non-zero

#### Scenario: A stale socket is left behind
- **WHEN** an instance starts and finds a socket file that refuses connections
- **THEN** it removes the file and listens on it

#### Scenario: An unknown command is received
- **WHEN** a client sends a command that is not one of `toggle`, `show`, `hide`, `auto`, `settings`
- **THEN** the instance ignores it and keeps running

### Requirement: The overlay has a manual visibility override
The overlay SHALL support `Auto`, `Shown`, and `Hidden` states, where `Auto` is the existing layer-driven behavior.

#### Scenario: The overlay is pinned visible
- **WHEN** the override is `Shown` and a keyboard is connected
- **THEN** the overlay is visible regardless of held layers and the configured timeout

#### Scenario: The overlay is hidden
- **WHEN** the override is `Hidden`
- **THEN** the overlay is not visible, even when a layer that keeps it visible is held

#### Scenario: A window needs the overlay
- **WHEN** the override is `Hidden` and the settings or key editor window is open
- **THEN** the overlay stays visible so keys remain clickable

#### Scenario: Layer-driven behavior is restored
- **WHEN** the override is `Auto`
- **THEN** visibility follows held layers, the visible-layer mask, activation delay, and timeout exactly as before

#### Scenario: No keyboard is connected
- **WHEN** no keyboard is connected and the override is `Shown`
- **THEN** nothing is shown, because the overlay draws the connected keyboard

### Requirement: The override is not persisted
The override SHALL NOT be written to the settings store, so a read-only declarative configuration remains authoritative.

#### Scenario: The process restarts
- **WHEN** KeyPeek is restarted after any override was set
- **THEN** it starts in `Auto`

### Requirement: The binary forwards control commands
`keypeek` SHALL accept `--toggle`, `--show`, `--hide`, `--auto`, and `--settings`, forward the matching command to the running instance, and exit without starting the application.

#### Scenario: A command is bound to a key
- **WHEN** the user binds a compositor key to `keypeek --toggle`
- **THEN** each invocation toggles the overlay of the running instance and exits immediately

#### Scenario: Toggle is resolved against current visibility
- **WHEN** `--toggle` arrives while the overlay is visible
- **THEN** it becomes hidden, and the next `--toggle` shows it again

#### Scenario: Arguments are contradictory
- **WHEN** `--config` is combined with a control command
- **THEN** the invocation is rejected with a usage message and exit status 2

### Requirement: The tray uses the same channel
The tray SHALL offer Show, Hide, and Auto alongside Settings… and Quit, and route its requests through the same command channel as the socket.

#### Scenario: The overlay is hidden from the tray
- **WHEN** the user chooses Hide in the tray menu
- **THEN** the overlay is hidden exactly as if `--hide` had been sent
