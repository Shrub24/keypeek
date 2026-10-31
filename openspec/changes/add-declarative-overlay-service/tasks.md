## 1. Override state and command handling

- [x] 1.1 Add `OverlayOverride` and `OverlayCommand` with a wire format shared by the socket and the command line.
  - refs: `specs/overlay-control/spec.md`, `design.md`
  - criteria: Commands round-trip through parse and format; an unknown command is rejected without panic.
  - delegate: CoderAgent
  - verify: Unit tests for parsing, formatting, and the toggle transition table.
- [x] 1.2 Apply the override inside the single visibility decision and expose a drain-time command handler.
  - refs: `specs/overlay-control/spec.md`
  - criteria: `Auto` reproduces current behavior, `Shown`/`Hidden` override it, windows still force visibility, no keyboard still shows nothing.
  - delegate: CoderAgent
  - verify: Unit tests for the transition table and the window/no-keyboard cases.

## 2. Control channel

- [x] 2.1 Add the shared command queue and route the tray request through it.
  - refs: `design.md`
  - criteria: The tray has Settings…, Show, Hide, Auto, and Quit entries; `settings_requested` is gone.
  - delegate: CoderAgent
  - verify: Build both desktop hosts and check the tray menu construction.
- [x] 2.2 Add the Unix socket listener and client.
  - refs: `specs/overlay-control/spec.md`
  - criteria: Socket in `$XDG_RUNTIME_DIR`, mode `0600`, stale file replaced, live instance reported, unknown lines ignored.
  - delegate: CoderAgent
  - verify: Integration test that listens on a temporary path, sends a command, and observes it in the queue.
- [x] 2.3 Add the control flags to the binary and reject `--config` combined with a control command.
  - refs: `specs/overlay-control/spec.md`
  - criteria: Control commands exit without constructing the app; usage errors exit with status 2.
  - delegate: CoderAgent
  - verify: Run the binary for each flag and for the contradictory combination.

## 3. Documentation

- [x] 3.1 Document the socket, the commands, and the override semantics in the README and architecture notes.
  - refs: `specs/overlay-control/spec.md`
  - criteria: A user can bind a compositor key from the documentation alone; the non-persistence rule is stated.
  - delegate: DocWriter
  - verify: Follow the documented example against a running instance.

## 4. Validation

- [ ] 4.1 Run scoped Rust and Nix validation.
  - delegate: BuildAgent
  - verify: Run `cargo fmt --check`, the affected Rust tests, and `nix flake check`.
- [ ] 4.2 Review the change against the specification and validate OpenSpec.
  - delegate: CodeReviewer
  - verify: Run `openspec validate add-declarative-overlay-service --strict`.
