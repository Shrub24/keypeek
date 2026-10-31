## Context

The flake currently exposes only per-system package, app, check, and shell outputs. Its package wraps the executable but omits the existing desktop entry and icon. KeyPeek always loads and saves its default INI path, which conflicts with immutable Nix-generated configuration. Home Manager can own user configuration and a graphical-session service, while system-manager is required for `/etc` integration on the target Arch system.

## Goals / Non-Goals

**Goals:**

- Keep package assets and module defaults tied to the selected KeyPeek package.
- Separate user-session configuration from privileged udev installation.
- Make generated configuration authoritative and non-writable.
- Preserve the existing INI representation and avoid new dependencies.

**Non-Goals:**

- Add a NixOS module, system daemon, custom access group, or automatic device connection.
- Modify or migrate the separate `~/.dotfiles` packaging.
- Generalize the application command line beyond selecting a configuration file.

## Decisions

### Export system-independent modules beside per-system outputs

Merge `homeManagerModules.default` and `systemManagerModules.default` with the existing `eachDefaultSystem` result. Each module captures the flake package as its default while allowing callers to override `package`. This avoids adding Home Manager or system-manager as flake inputs because the module interfaces are evaluated only by their consumers.

Alternative: define modules inside `eachDefaultSystem`. Rejected because it produces non-standard system-qualified module outputs.

### Package all integration assets with KeyPeek

Install the desktop entry, icon, and udev rule in conventional package paths. The system-manager module links the rule from the selected package rather than duplicating its contents. A future NixOS module can consume the same packaged rule through `services.udev.packages`.

Alternative: embed the udev rule in each module. Rejected because the rule would drift from the package documentation and future NixOS integration.

### Keep user and system modules separate

The Home Manager module owns `home.packages`, generated INI configuration, and the systemd user service. The system-manager module owns only the `/etc/udev/rules.d` link. The service remains a user service because the overlay requires the graphical session.

Alternative: run KeyPeek as a system service. Rejected because it would need to cross display-session and user IPC boundaries without adding useful capability.

### Treat an explicit configuration file as read-only

Add a minimal `--config <path>` application option. Normal launches retain the current writable platform config. Launches with `--config` load that file but skip persistence, allowing the Home Manager service to pass a store-backed generated INI without save failures. No command-line parsing dependency is needed for one option.

Alternative: symlink generated INI into the default writable path. Rejected because the existing UI would repeatedly attempt to replace an immutable Nix store file.

## Risks / Trade-offs

- [The system-manager module uses a less common flake output] → Export it explicitly as `systemManagerModules.default` and document the import path.
- [A newly installed udev rule may not affect an existing device immediately] → Document replugging the device and reloading udev rules; avoid an activation service until runtime evidence requires one.
- [The initial service can start before auto-connect exists] → Keep service enablement explicit and extend it in the later auto-connect change.
- [Manual argument parsing will not scale to the planned IPC CLI] → Limit it to `--config`; replace it only when the later CLI genuinely needs a parser.

## Migration Plan

1. Build and validate the enriched default package.
2. Evaluate the Home Manager and system-manager modules with minimal test configurations.
3. Validate the read-only generated configuration path.
4. Leave downstream consumers unchanged until they opt into the new module outputs.

Rollback removes the module imports and returns consumers to the existing package output; no mutable data migration is required.
