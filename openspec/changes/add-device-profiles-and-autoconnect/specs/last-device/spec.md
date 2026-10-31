## Purpose

Remember which keyboard was used and reconnect to it on the next start, without touching configuration and without requiring user interaction.

## ADDED Requirements

### Requirement: The last used keyboard is remembered
A successful connection SHALL record the identity of the connected keyboard in a state file outside the configuration directory.

#### Scenario: A connection succeeds
- **WHEN** a keyboard connects successfully
- **THEN** its driver, VID/PID, and name are written to the state file

#### Scenario: A connection never completes
- **WHEN** a device is selected but the connection fails, is locked, or needs a layout file
- **THEN** the state file is unchanged

#### Scenario: Configuration is read-only
- **WHEN** KeyPeek runs with an explicit read-only configuration
- **THEN** the state file is still written, and the configuration file is not

### Requirement: The remembered keyboard is reconnected on start
Startup SHALL connect automatically to the remembered keyboard when it is present in the discovered devices.

#### Scenario: The remembered keyboard is present
- **WHEN** the remembered identity matches a discovered device
- **THEN** KeyPeek selects and connects to it without user interaction

#### Scenario: The remembered keyboard is absent
- **WHEN** no discovered device matches the remembered identity
- **THEN** KeyPeek starts disconnected and does not connect to another device

#### Scenario: Nothing has been remembered
- **WHEN** the state file is missing or unreadable
- **THEN** KeyPeek starts disconnected

### Requirement: Identity survives transport changes
Matching SHALL rely on driver, VID/PID, and name only.

#### Scenario: The keyboard moves to another port
- **WHEN** a remembered keyboard appears on a different USB port or BLE address than last time
- **THEN** it still matches and is reconnected
