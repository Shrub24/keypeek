## Why

On Wayland every egui window shares the overlay's full-output layer surface. The settings window therefore opens on whichever output the overlay is bound to, not where the user launched it; a dim shade covers that output while any window is open; and changing the monitor setting rebuilds the surface the settings window lives on, so the window vanishes with it. Settings is part of operating the daemon, not part of the overlay, and should behave like an ordinary application window.

## What Changes

- Show settings, with its Error and Notice dialogs, as an egui viewport of its own; the eframe host already opens viewports as native windows.
- Teach the Wayland host to open egui viewports as xdg toplevels drawn with the overlay's shared EGL context and painter, with input routed by surface, so the compositor places settings like any other new window.
- Remove the dim shade; the overlay surface is always transparent.
- Limit the overlay's input region to the egui areas it shows (overlay, key editor) instead of the whole output while a window is open.
- Closing settings no longer quits a process that never connected; the instance keeps running, reachable from the tray, `keypeek --settings`, or the control socket.

## Capabilities

### New Capabilities

- `settings-window`: How the settings window is placed relative to the overlay.

### Modified Capabilities

None.

## Impact

Affects `keypeek-presentation` (settings viewport, `OverlayHost` loses `request_close`), `keypeek-desktop` (Wayland host: shared EGL context, per-surface canvases, xdg windows; eframe host: no shade) and `keypeek-web` (host trait). The key editor still opens from the overlay. No new dependency.
