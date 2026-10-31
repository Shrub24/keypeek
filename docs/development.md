# KeyPeek Development Standards

KeyPeek is a Linux/Nix-first, fully featured personal downstream of the upstream project. It is intended to move ahead of upstream: local functionality wins, but atomic changes should remain upstreamable where practical. This document records the downstream product posture, code style, validation expectations, and the Jujutsu change workflow. It describes policy, not current state.

## Product posture

- **Linux/Nix first.** Linux is the primary target and the Nix flake is the primary build and validation path. Windows and macOS builds are maintained but secondary; features are developed and tested on Linux first.
- **Local-first, upstream-aware.** The project serves personal needs and may diverge from upstream without waiting for acceptance. Where a change is atomic and self-contained, keep it in a shape that could be upstreamed later; this constraint never blocks local progress.
- **Fully featured.** There is no feature ceiling set by upstream. If the downstream wants it, the downstream builds it.

## Code style

- **Modern, idiomatic, declarative Rust and Nix.** Use current stable Rust idioms and the existing flake conventions (crane, fenix). Prefer data-driven, declarative forms over imperative glue.
- **Concise, consistent patterns.** Reuse existing helpers and patterns before adding new ones; match the surrounding code.
- **YAGNI.** Build only what the task needs. No speculative abstraction, configuration, or scaffolding.
- **Minimal comments.** Comments explain why, not what; prefer self-documenting code.

## Validation

Validation is scoped: run the checks that cover the change rather than the whole suite, and run the full gate before considering work complete.

- `cargo fmt` and alejandra for formatting.
- `cargo nextest run` for Rust tests (or `cargo test` when nextest is unavailable).
- deadnix and statix for Nix hygiene.
- `nix flake check` as the full gate: it builds the package, runs the test suite, and runs the pre-commit hooks.

## Jujutsu workflow

Changes are made as anonymous Jujutsu changes, one concern per change. Branches and bookmarks are not required workflow state; a change is identified by its description, not a name. Jujutsu operates on this repository's git history directly.

Practical flow:

1. `jj new` starts a change on top of the current parent.
2. Make the edit; `jj describe -m "<one concern>"` when the change is complete.
3. Validate the change, then publish it if needed with `jj git push --change @`; Jujutsu creates the required remote bookmark from its change ID.
4. `jj new` starts the next concern. Keep each change small enough to review, test, and upstream on its own.

The working copy is itself an anonymous change; `jj squash` folds it into the change below when an edit continues an earlier concern.
