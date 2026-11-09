# KeyPeek Roadmap and External-Change Policy

KeyPeek is a Linux/Nix-first personal downstream that moves ahead of upstream in atomic, reviewable changes. This document records the approved feature sequence and the policy for evaluating upstream pull requests and forks. It describes the plan and policy, not current state; [development.md](development.md) records the engineering posture this plan builds on.

The downstream currently tracks upstream 0.11.0 (`78a2274`), which restructured the project into a five-crate workspace and added a browser target. All local work is expressed against that layout.

## Feature roadmap

Features land as one atomic change per concern, in the approved sequence below. The sequence is ordered but not scheduled: each change starts only when the work it depends on is complete. Later changes are specified by their own OpenSpec proposal when they start.

- **Foundation — `establish-downstream-foundation`.** Record the architecture, engineering posture, workflow, and roadmap before independent feature work begins.
- **First feature change — `integrate-native-nix-support`.** Complete desktop and BLE permission packaging, then export Home Manager and NixOS modules.
- **Subsequent changes, in order:**
  - `add-declarative-overlay-service` — landed: local show/hide control over a command socket, plus tray entries and the `--toggle` family.
  - `add-output-selection` — landed: the overlay is bound to a chosen output and retargets live.
  - `add-device-profiles-and-autoconnect`

## Withdrawn and re-scoped changes

- **`render-custom-zmk-behaviors` — withdrawn.** Upstream #28 landed custom ZMK behavior rendering, with OS-layout-aware labels (#35) and a single-legend mode (#38) alongside it.
- **`make-zmk-discovery-authoritative` — withdrawn.** The change was justified by coexistence with QMK/Vial keyboards: it stopped an unverified USB serial port from suppressing a Raw HID entry, and preserved Vial entries against ZMK claims. The downstream is used exclusively with ZMK keyboards, where the upstream scanner's unconditional serial enumeration already produces the right result, and the local benefit was limited to uncluttering the device list of UART adapters. Rather than carry a probe and evidence model for a case that does not arise locally, the change is dropped; the upstream behaviors it targeted (a Vial board exposing a serial port loses its VIA-page entry, and a UART adapter is listed as a keyboard) are recorded in [architecture.md](architecture.md#device-discovery) and left to upstream.

## External-change evaluation

Upstream current behavior is the baseline for comparison; local functionality may lead upstream master. Useful concepts from upstream and forks are reimplemented as small local changes rather than merged wholesale, so the downstream is not held to stale or broad diffs. A planned change that upstream absorbs before it starts is withdrawn instead of reimplemented.

Decisions by source:

- **Upstream issue #28, pull requests #35 and #38.** Custom behavior rendering, OS-layout-aware labels, and single-legend mode are all present, so `render-custom-zmk-behaviors` is withdrawn.
- **dan-dr / issue #25 (exact identity, saved connection, auto-connect/reconnect).** Adopted as concepts and reimplemented in the later `add-device-profiles-and-autoconnect` change. The full branch is rejected — its merge base predates the keymap editor and its discovery work is fuzzy.
- **PR #49 (overlay monitor selection).** Open against master but conflicting as of 0.11.0 (it predates the crate split; a Linux/macOS variant, #47, and a Windows variant, #51, are closed). `add-output-selection` takes the concepts — bind the layer surface to a named output, persist the choice by display name, prime the surface so the compositor reports scale — and adds live retargeting, which the pull request lacks for Wayland. Its eframe/winit monitor handling is not ported: the layer-shell path is the one in use, and those hosts keep showing the overlay on the primary display.
- **PR #17 (IPC/CLI control).** Open and conflicting, bundling `--toggle`/`--settings` with unrelated keyboard work. Only the toggle and settings-request concepts are reimplemented, in `add-declarative-overlay-service`.
- **PR #29 (Vial layout options).** Deferred until a local need appears; upstream progress is tracked in the meantime.
- **PR #30 (layout-aware labels).** Closed, with its behaviour superseded upstream by #35.
- **Upstream browser target and Windows/macOS platform work.** Not a downstream concern. Changes must keep the workspace compiling under `cargo nextest --workspace` and, where they touch shared crates, keep the `wasm32` build available; no downstream feature may target the browser or those platforms.

## Intake rule for external changes

Before adopting anything from upstream or a fork, run this check:

1. **Verify current upstream state.** Check the latest upstream master, not the PR or branch as originally opened; the diff may be stale.
2. **Identify the root cause.** Understand the problem the change solves before deciding how to adopt it.
3. **Prefer existing patterns.** Use local helpers or standard library approaches where they already cover the need.
4. **Split into atomic changes.** Land the useful concepts as small, independent jj/OpenSpec changes, one concern each.
5. **Record provenance.** Note the source PR, branch, or issue in the change description so the origin stays traceable.
