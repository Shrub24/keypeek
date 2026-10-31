# nix-modules Specification

## Purpose
Expose concise Nix modules that integrate KeyPeek at the correct user and system privilege boundaries without coupling the project to a downstream configuration repository.

## Requirements

### Requirement: Home Manager integration is available
The flake SHALL export a Home Manager module under `homeManagerModules.default` with `programs.keypeek` options for the package, INI settings, and graphical-session user service.

#### Scenario: KeyPeek is enabled without its service
- **WHEN** `programs.keypeek.enable` is true and the service is disabled
- **THEN** Home Manager installs the selected package and does not start KeyPeek automatically

#### Scenario: Declarative settings are configured
- **WHEN** `programs.keypeek.settings` is non-empty
- **THEN** Home Manager generates an INI configuration and KeyPeek reads it as authoritative without attempting to modify it

#### Scenario: User service is enabled
- **WHEN** the KeyPeek service option is enabled
- **THEN** a systemd user service starts the selected package in the graphical session and restarts it after failure

### Requirement: System-manager integration is available
The flake SHALL export a system-manager module under `systemManagerModules.default` that can install the packaged BLE udev rule without configuring a system daemon or custom group.

#### Scenario: BLE permissions are enabled
- **WHEN** the system-manager KeyPeek option is enabled
- **THEN** the packaged rule is linked into `/etc/udev/rules.d`

#### Scenario: BLE permissions are disabled
- **WHEN** the system-manager KeyPeek option is disabled
- **THEN** the module makes no KeyPeek-specific changes under `/etc`
