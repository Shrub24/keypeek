## Why

The upstream Nix package installs only the executable, leaving KeyPeek absent from application launchers and BLE hidraw access unmanaged on the target Arch/Nix system. Native flake outputs should provide complete desktop packaging and small, composable modules without relying on downstream patches.

## What Changes

- Install the existing desktop entry and scalable icon from the default Nix package.
- Package a modalias-based BLE hidraw udev rule using `uaccess`, while documenting why the existing USB rules do not cover BLE HID.
- Export a Home Manager module for package installation, generated INI configuration, and an optional graphical-session systemd user service.
- Export a minimal system-manager module that installs the packaged udev rule on non-NixOS systems.
- Keep the system module compatible with a future NixOS integration without introducing a system daemon or custom access group.

## Capabilities

### New Capabilities

- `nix-packaging`: Complete Nix package assets and BLE hidraw integration data.
- `nix-modules`: Declarative Home Manager and system-manager integration for KeyPeek.

### Modified Capabilities

None.

## Impact

The change affects `flake.nix`, packaged desktop assets, Nix module outputs, Linux permissions documentation, and Nix validation. It adds no application runtime dependency and does not modify the separate `~/.dotfiles` repository.
