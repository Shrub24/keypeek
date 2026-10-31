## 1. Setting and settings window

- [x] 1.1 Add the `monitor` setting with `Primary` and named outputs, serialized by name.
  - refs: `specs/output-selection/spec.md`, `design.md`
  - criteria: The setting round-trips through INI, an empty value means `Primary`, and any other value parses as a name.
  - delegate: CoderAgent
  - verify: Unit tests for the INI round-trip and name parsing.
- [x] 1.2 Offer the reported outputs in the appearance section and report when none exist.
  - refs: `specs/output-selection/spec.md`
  - criteria: `Primary` plus each reported output is selectable; without reported outputs a hint replaces the list.
  - delegate: CoderAgent
  - verify: Build the settings window and check the row against a host that reports outputs and one that does not.

## 2. Wayland output binding

- [x] 2.1 Bind the layer surface to the configured output after a roundtrip.
  - refs: `specs/output-selection/spec.md`, `design.md`
  - criteria: The surface is created with the named `wl_output`; `Primary` and unknown names pass `None`; a missing output falls back to compositor choice.
  - delegate: CoderAgent
  - verify: Inspect the surface creation path and the fallback branch.
- [x] 2.2 Rebuild the surface when the setting changes and prime the first frame.
  - refs: `specs/output-selection/spec.md`, `design.md`
  - criteria: A changed setting recreates the surface without restarting; the bound output is tracked so no recreation loop occurs; one transparent frame is committed before the first UI frame.
  - delegate: CoderAgent
  - verify: Reason through the loop comparison and confirm one bind per change.
- [x] 2.3 Report the compositor's outputs to the presentation layer, including on hotplug.
  - refs: `specs/output-selection/spec.md`
  - criteria: Available outputs are refreshed at startup and on output events.
  - delegate: CoderAgent
  - verify: Confirm the `OutputHandler` callbacks update the list.

## 3. Documentation

- [x] 3.1 Document the monitor setting and note which hosts support it.
  - refs: `specs/output-selection/spec.md`
  - criteria: The README mentions the setting and the Wayland-only support; the architecture notes describe binding and retargeting.
  - delegate: DocWriter
  - verify: Read the documented behaviour against the code.

## 4. Validation

- [ ] 4.1 Run scoped Rust and Nix validation.
  - delegate: BuildAgent
  - verify: Run `cargo fmt --check`, the affected Rust tests, and `nix flake check`.
- [ ] 4.2 Review the change against the specification and validate OpenSpec.
  - delegate: CodeReviewer
  - verify: Run `openspec validate add-output-selection --strict`.
