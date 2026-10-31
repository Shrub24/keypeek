## 1. Canonical Documentation

- [x] 1.1 Document the current architecture and technology stack under `docs/`.
  - criteria: The documents describe the runtime boundaries, protocol integrations, UI stack, and build/package tooling without duplicating implementation details.
  - delegate: DocWriter
  - verify: Check every technical claim against the repository.
- [x] 1.2 Document the downstream product posture and development standards.
  - criteria: The documentation defines Linux/Nix priority, local-first upstream policy, concise Rust/Nix style, validation expectations, and anonymous Jujutsu changes.
  - delegate: DocWriter
  - verify: Confirm downstream doctrine is not added to `CONTRIBUTING.md`.
- [x] 1.3 Document the feature roadmap and external-change evaluation policy.
  - criteria: The roadmap records the approved atomic change sequence and distinguishes adopted, deferred, and rejected fork/PR approaches.
  - delegate: DocWriter
  - verify: Compare the roadmap with `proposal.md` and the approved planning decisions.

## 2. Documentation Integration

- [x] 2.1 Link the canonical documents from `README.md`.
  - criteria: A reader can find the architecture, stack, development posture, and roadmap from the project entry point.
  - verify: Resolve each added relative link.
- [x] 2.2 Review the documentation set for consistency and validate the OpenSpec change.
  - delegate: CodeReviewer
  - verify: Run `openspec validate establish-downstream-foundation --strict`.
