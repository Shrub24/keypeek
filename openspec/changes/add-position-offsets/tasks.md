## 1. Placement model

- [x] 1.1 Replace `margin` with `offset_x`/`offset_y` in the settings model and the anchor mapping.
  - refs: `specs/overlay-placement/spec.md`, `design.md`
  - criteria: Each anchor applies its own axis direction; negative offsets move outward; constant values are used where an axis has no edge.
  - delegate: CoderAgent
  - verify: Unit tests over all anchors covering sign and magnitude.
- [x] 1.2 Serialize the offsets and read `margin` as a fallback.
  - refs: `specs/overlay-placement/spec.md`
  - criteria: Writing emits only offsets; reading an older file keeps the previous placement.
  - delegate: CoderAgent
  - verify: Round-trip test plus a legacy-file test.

## 2. Settings window

- [x] 2.1 Replace the single distance field with one row per axis.
  - refs: `specs/overlay-placement/spec.md`
  - criteria: Each field edits only its own axis and explains the inward direction.
  - delegate: CoderAgent
  - verify: Change one field and confirm the other keeps its value.

## 3. Documentation

- [x] 3.1 Document the offsets and the legacy `margin` behaviour.
  - refs: `specs/overlay-placement/spec.md`
  - criteria: The README names the keys and their sign convention.
  - delegate: DocWriter
  - verify: Read the documented keys against the serializer.

## 4. Validation

- [x] 4.1 Run scoped Rust and Nix validation.
  - delegate: BuildAgent
  - verify: Run `cargo fmt --check`, the affected Rust tests, and `nix flake check`.
- [ ] 4.2 Review the change against the specification and validate OpenSpec.
  - delegate: CodeReviewer
  - verify: Run `openspec validate add-position-offsets --strict`.
