## 1. Package Integration Assets

- [ ] 1.1 Add the BLE hidraw udev rule and install all integration assets from the default Nix package.
  - refs: `specs/nix-packaging/spec.md`, `design.md`
  - criteria: The package contains the desktop entry, scalable icon, and modalias-based `uaccess` rule in conventional paths.
  - delegate: CoderAgent
  - verify: Build the package and inspect its installed paths and desktop entry references.
- [ ] 1.2 Document Linux USB and BLE hidraw permission handling.
  - refs: `specs/nix-packaging/spec.md`
  - criteria: The documentation distinguishes USB VID/PID rules from BLE modalias rules and explains activation.
  - delegate: DocWriter
  - verify: Check the documented path and modalias against the packaged rule.

## 2. Declarative Configuration Mode

- [ ] 2.1 Add `--config <path>` loading with read-only persistence semantics.
  - refs: `specs/nix-modules/spec.md`, `design.md`
  - criteria: Explicit configuration is loaded without later writes; normal launches retain the current writable settings behavior.
  - delegate: CoderAgent
  - verify: Add focused tests for explicit and default configuration behavior, then run `cargo test` for the affected targets.

## 3. Nix Modules

- [ ] 3.1 Export a Home Manager module for package installation, generated INI settings, and the optional graphical-session service.
  - refs: `specs/nix-modules/spec.md`, `design.md`
  - criteria: `homeManagerModules.default` exposes `programs.keypeek` and the service uses the selected package plus generated read-only config.
  - delegate: CoderAgent
  - verify: Evaluate enabled configurations with and without settings and service options.
- [ ] 3.2 Export a minimal system-manager module for the packaged udev rule.
  - refs: `specs/nix-modules/spec.md`, `design.md`
  - criteria: `systemManagerModules.default` conditionally links the selected package rule into `/etc/udev/rules.d` and configures no daemon or group.
  - delegate: CoderAgent
  - verify: Evaluate the module in enabled and disabled system-manager configurations.

## 4. Validation

- [ ] 4.1 Run scoped Rust and Nix validation.
  - delegate: BuildAgent
  - verify: Run formatting, affected Rust tests, package build, module evaluations, and `nix flake check`.
- [ ] 4.2 Review the complete change for contract compliance and validate OpenSpec.
  - delegate: CodeReviewer
  - verify: Run `openspec validate --change integrate-native-nix-support --strict`.
