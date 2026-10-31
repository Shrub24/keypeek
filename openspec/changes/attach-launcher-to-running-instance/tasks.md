## 1. Argument handling

- [ ] 1.1 Make `--settings` attach-or-start and keep hotkey commands fail-fast.
  - refs: `specs/launcher-attach/spec.md`, `design.md`
  - criteria: `--settings` starts an instance with settings visible when nothing answers; `--toggle` and friends still exit non-zero; `--config` with `--settings` is accepted, with the other commands rejected.
  - delegate: CoderAgent
  - verify: Run each flag against a listener, against no listener, and with `--config`.
- [ ] 1.2 Add `--no-settings`.
  - refs: `specs/launcher-attach/spec.md`
  - criteria: A start with the flag never shows the settings window.
  - delegate: CoderAgent
  - verify: Start with the flag and observe no window.

## 2. Startup visibility

- [ ] 2.1 Open the settings window at start only when nothing was auto-connected.
  - refs: `specs/launcher-attach/spec.md`
  - criteria: An auto-connected start shows no window; a start with nothing remembered shows it.
  - delegate: CoderAgent
  - verify: Unit tests over both connection states.

## 3. Packaging and documentation

- [ ] 3.1 Point the desktop entry at the attaching invocation and add `--no-settings` to the generated service.
  - refs: `specs/launcher-attach/spec.md`
  - criteria: The installed entry runs the settings command; the Home Manager unit passes the suppression flag.
  - delegate: CoderAgent
  - verify: Evaluate the module and inspect `ExecStart`; install the package and read the entry.
- [ ] 3.2 Document launcher behaviour and the read-only configuration caveat.
  - refs: `specs/launcher-attach/spec.md`
  - criteria: The README explains that the launcher attaches, and that settings changes are session-only under a declarative configuration.
  - delegate: DocWriter
  - verify: Read the documentation against the argument parser.

## 4. Validation

- [ ] 4.1 Run scoped Rust and Nix validation.
  - delegate: BuildAgent
  - verify: Run `cargo fmt --check`, the affected Rust tests, the package build, and `nix flake check`.
- [ ] 4.2 Review the change against the specification and validate OpenSpec.
  - delegate: CodeReviewer
  - verify: Run `openspec validate attach-launcher-to-running-instance --strict`.
