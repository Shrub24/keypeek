# nix-packaging Specification

## Purpose
Provide a complete Linux package that integrates KeyPeek with desktop launchers and supplies the system data needed for BLE hidraw access.

## Requirements

### Requirement: Desktop integration is installed
The default Nix package SHALL install the KeyPeek desktop entry and scalable application icon in their standard freedesktop locations.

#### Scenario: Package is added to a user environment
- **WHEN** the default Nix package is installed
- **THEN** a desktop launcher can resolve the `keypeek` executable and `keypeek` icon from the package

### Requirement: BLE hidraw rule is packaged
The default Nix package SHALL include a udev rule that grants the active local user access to BLE HID devices matching modalias `hid:b0005g0001v00001D50p0000615E` through `uaccess`.

#### Scenario: System integration consumes the package rule
- **WHEN** a system installs and activates the packaged udev rule
- **THEN** the matching BLE hidraw device receives the `uaccess` tag without requiring a KeyPeek-specific group

### Requirement: BLE permissions are documented
The repository SHALL explain that USB VID/PID udev rules do not cover BLE HID and SHALL document how to activate the packaged modalias rule.

#### Scenario: User configures BLE access
- **WHEN** a user follows the Linux permissions documentation
- **THEN** they can distinguish USB and BLE permission handling and locate the packaged rule
