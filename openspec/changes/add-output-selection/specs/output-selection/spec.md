## Purpose

Let the user choose which display the overlay appears on, and keep that choice stable across restarts and display changes.

## ADDED Requirements

### Requirement: The overlay can be bound to a chosen output
The desktop runner SHALL bind the overlay to the output named by the `monitor` setting, and let the display server choose when the setting is `Primary`.

#### Scenario: A named output is configured
- **WHEN** the setting names an output the display server reports
- **THEN** the overlay is created on that output

#### Scenario: Primary is configured
- **WHEN** the setting is `Primary`
- **THEN** the display server chooses the output, as before

#### Scenario: The named output is gone
- **WHEN** the setting names an output that is no longer reported
- **THEN** the display server chooses the output

### Requirement: Changing the setting moves the running overlay
A change to the `monitor` setting SHALL take effect without restarting the process.

#### Scenario: The user picks another display
- **WHEN** the user selects a different output in the settings window and the change is committed
- **THEN** the overlay is recreated on that output within the next frame

### Requirement: The choice is persisted by name
The setting SHALL be stored in the settings file as `Primary` or the output name.

#### Scenario: The process restarts
- **WHEN** KeyPeek starts with a settings file naming an output
- **THEN** the overlay is bound to that output on its first frame

#### Scenario: Outputs are renumbered
- **WHEN** display order changes but names stay the same
- **THEN** the overlay still appears on the named output

### Requirement: The settings window offers the reported outputs
The settings window SHALL list `Primary` plus the outputs the platform reports, and SHALL state when the platform reports none.

#### Scenario: Outputs are reported
- **WHEN** the display server reports output names
- **THEN** each is selectable as an overlay target

#### Scenario: The platform reports no outputs
- **WHEN** the host cannot enumerate outputs (for example the XWayland/eframe path)
- **THEN** only `Primary` is offered and a hint explains that output names are unavailable
