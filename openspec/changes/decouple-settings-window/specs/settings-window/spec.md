## Purpose

Keep the settings window independent of the overlay surface, so it is placed and managed like an ordinary application window.

## ADDED Requirements

### Requirement: Settings opens as its own window
The desktop runner SHALL show the settings window, and the Error and Notice dialogs, in a native window separate from the overlay surface.

#### Scenario: Settings opens on Wayland
- **WHEN** settings is opened on a Wayland session
- **THEN** it appears as an xdg toplevel titled "KeyPeek Settings" with app id `keypeek`, placed by the compositor

#### Scenario: The overlay moves while settings is open
- **WHEN** the monitor setting changes while settings is open
- **THEN** only the overlay surface is rebuilt and the settings window stays open where it was

### Requirement: The overlay surface is never shaded
The overlay surface SHALL stay transparent while any window is open.

#### Scenario: Settings is open
- **WHEN** settings is open
- **THEN** the overlay's output is not dimmed

### Requirement: The overlay takes input only where it draws
While the overlay accepts input, it SHALL do so only over the egui areas it shows.

#### Scenario: Settings is open and the overlay is shown
- **WHEN** settings is open and the overlay is visible
- **THEN** clicks on the overlay reach it and clicks elsewhere on its output reach the windows beneath

### Requirement: Closing settings leaves the instance running
Closing the settings window SHALL NOT exit the process.

#### Scenario: Nothing ever connected
- **WHEN** settings is closed before any keyboard connected
- **THEN** the instance keeps running and `keypeek --settings` reopens settings
