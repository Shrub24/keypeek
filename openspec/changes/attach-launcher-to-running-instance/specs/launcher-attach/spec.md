## Purpose

Let a launcher invocation configure the KeyPeek that is already running, and keep a service-managed instance from opening windows nobody asked for.

## ADDED Requirements

### Requirement: The launcher attaches to a running instance
A launcher invocation SHALL open the settings window of the instance that owns the control socket.

#### Scenario: An instance is running
- **WHEN** KeyPeek is launched from the application menu while an instance is running
- **THEN** that instance opens its settings window and no second overlay appears

#### Scenario: Nothing is running
- **WHEN** KeyPeek is launched from the application menu with no instance listening
- **THEN** an instance starts with the settings window open

#### Scenario: A configuration is given
- **WHEN** the invocation combines `--config` with `--settings`
- **THEN** a running instance is attached to, and the configuration applies only if an instance had to be started

### Requirement: Hotkey commands never start an instance
`--toggle`, `--show`, `--hide`, and `--auto` SHALL fail with a message when nothing is listening.

#### Scenario: A hotkey is pressed with no instance
- **WHEN** `keypeek --toggle` runs with nothing listening on the control socket
- **THEN** it reports the socket path, exits with a non-zero status, and starts nothing

### Requirement: The settings window opens only when it is useful
A started instance SHALL open its settings window only when it did not connect to a remembered keyboard and was not told to suppress it.

#### Scenario: A remembered keyboard is present
- **WHEN** the instance connects automatically on start
- **THEN** no settings window opens

#### Scenario: Nothing is known yet
- **WHEN** no keyboard is remembered and none connects
- **THEN** the settings window opens

#### Scenario: The window is suppressed
- **WHEN** the instance was started with `--no-settings`
- **THEN** no settings window opens, whatever the connection state

### Requirement: The Nix package's launcher uses the attaching invocation
The desktop entry installed by the flake package SHALL invoke the settings command rather than a bare start. The shared source entry keeps a bare start, so packaging that derives its entrypoint from `Exec` (the AppImage) is unaffected.

#### Scenario: Reading the installed entry
- **WHEN** the desktop entry installed by the flake package is read
- **THEN** it runs the binary with the settings argument

#### Scenario: Building the shared source entry
- **WHEN** `cargo-appimage.desktop` is used directly, without the flake's rewrite
- **THEN** its `Exec` names only the binary
