## 1. Device identity

- [ ] 1.1 Add `DeviceIdentity` (driver, VID/PID, name) with matching and a single-line wire form.
  - refs: `specs/last-device/spec.md`, `design.md`
  - criteria: A device round-trips through the wire form; a different transport with the same identity matches; a different VID/PID or name does not.
  - delegate: CoderAgent
  - verify: Unit tests for round-trip and matching.

## 2. State store

- [ ] 2.1 Add `SessionStateStore` with a file-backed implementation in the state directory and an in-memory one.
  - refs: `specs/last-device/spec.md`, `design.md`
  - criteria: The file store creates its directory, ignores unreadable content, and lives outside the configuration directory.
  - delegate: CoderAgent
  - verify: Round-trip through a temporary path, plus a missing-file and a garbage-file case.

## 3. Startup behaviour

- [ ] 3.1 Auto-connect on construction and record the identity on a successful connection.
  - refs: `specs/last-device/spec.md`
  - criteria: A matching device connects without interaction, a missing one leaves KeyPeek disconnected, and only a successful connection updates the record.
  - delegate: CoderAgent
  - verify: Unit tests with a memory store for the match, no-match, and record-on-success cases.
- [ ] 3.2 Supply the stores from both runners.
  - refs: `design.md`
  - criteria: The desktop runners use the file store; the browser runner uses the in-memory one.
  - delegate: CoderAgent
  - verify: Build the desktop and wasm-facing crates.

## 4. Documentation

- [ ] 4.1 Document the behaviour, the state file location, and how to disable it.
  - refs: `specs/last-device/spec.md`
  - criteria: The README states that the last keyboard is reconnected and where the state lives.
  - delegate: DocWriter
  - verify: Read the documented path against the implementation.

## 5. Validation

- [ ] 5.1 Run scoped Rust and Nix validation.
  - delegate: BuildAgent
  - verify: Run `cargo fmt --check`, the affected Rust tests, and `nix flake check`.
- [ ] 5.2 Review the change against the specification and validate OpenSpec.
  - delegate: CodeReviewer
  - verify: Run `openspec validate add-device-profiles-and-autoconnect --strict`.
