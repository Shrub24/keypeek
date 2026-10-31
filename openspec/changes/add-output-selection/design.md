## Context

The Wayland host creates the overlay as a wlr-layer-shell surface with `create_layer_surface(..., output = None)`, anchors it to all four edges, and lets the compositor size and place it. The output argument is fixed when the surface is created: there is no request to move a layer surface, so any change of target means replacing the surface. The host already handles "first configure of a fresh surface" by creating the EGL state lazily, which is exactly the path a replacement surface takes.

The settings window edits a draft copy of `Settings` and commits it to the active settings while the user watches; the host reads active settings every frame.

## Goals / Non-Goals

**Goals:**

- Name the target output explicitly, and move the running overlay when that choice changes.
- Persist the choice so it survives the process, by output name rather than index.
- Tell the user when the platform cannot report outputs.

**Non-Goals:**

- Port PR #49's eframe/winit monitor handling (X11 sizing, Windows/macOS maximization, DWM alpha). The layer-shell path is the one in use; those hosts keep showing the overlay on the primary monitor.
- Follow the pointer, the focused window, or keyboard focus across outputs — Wayland exposes no global focus to a layer surface.
- React to a display being unplugged by moving the overlay elsewhere and back again.

## Decisions

### Bind the surface to a named output

`create_layer_surface` takes the `wl_output`; passing the one whose `OutputState` name matches the setting binds the overlay there. `Primary` passes `None` and keeps today's compositor-chosen behaviour, since Wayland has no primary-output flag. A named output that is no longer present also passes `None`, so an unplugged monitor degrades to the current behaviour instead of leaving no overlay.

### Rebuild the surface instead of persisting a restart requirement

PR #49's Wayland path reads the setting once, at surface creation. Since the declarative setup runs the overlay as a long-lived systemd user service, a setting that only applies after `systemctl --user restart` is not usable. `bind_overlay` drops the EGL state, painter and viewport, destroys the old surface, and creates a new layer surface with the requested output; the existing configure path then rebuilds EGL and the painter. The loop compares the active setting with the bound output once per iteration, so a change made in the settings window moves the overlay in the next frame.

### Roundtrip before the first bind

Output names only exist after `OutputState` has seen the output events, so `run` performs one `event_queue.roundtrip` before creating the surface. The same pass means the overlay appears on the configured display from its first frame instead of starting on the default one and jumping.

### Prime the surface once

A layer surface gets its `configure` and `preferred_scale` only after a buffer has been committed. The first frame after binding therefore clears a transparent buffer and returns, letting the compositor report scale before the UI is laid out. Without this, a surface on a fractional-scale output paints one frame at the wrong pixel ratio.

### Persist by name

`MonitorSelection` serializes as `Primary` or the raw output name. Output indices are not stable across replugs and reordering, so a numeric choice would silently move the overlay. An unknown name in the config parses as itself and falls back to compositor choice when absent.

## Risks

- **Surface replacement churn.** Each retarget destroys and recreates the surface, so a rapid string of changes causes flicker. The setting only changes when the settings window commits, so this is bounded in practice.
- **Lost inputs.** The keyboard and pointer objects belong to the seat, not the surface, so a retarget does not re-pair inputs; the input region is re-applied on the next passthrough sync.
- **Untested platform paths.** The X11/Windows/macOS behaviour is deliberately untouched; those hosts report no outputs, and the settings window says "This display server does not report output names."
