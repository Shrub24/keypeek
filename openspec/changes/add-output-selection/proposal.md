## Why

The overlay is created without naming an output, so the compositor decides which display it appears on, and there is no way to choose another one — the surface has to be recreated. On a multi-monitor desk that means the overlay can land on a display the user is not looking at. Upstream issue #24 is open; PR #49 implements monitor selection but is written against the pre-0.11.0 layout, leaves the Wayland case at startup-only, and bundles unrelated Windows/macOS window handling.

## What Changes

- Add a `monitor` setting (`Primary` or a named output), persisted by name because output indices shift.
- Offer the display server's outputs in the settings window and show the current choice.
- Bind the Wayland layer surface to the chosen output, after a roundtrip so output names are known before the surface is created.
- Rebuild the layer surface when the choice changes, so the overlay moves without restarting.

## Capabilities

### New Capabilities

- `output-selection`: Choosing the display the overlay appears on.

### Modified Capabilities

None.

## Impact

Affects `keypeek-presentation` (setting, settings UI, available-output state) and `keypeek-desktop` (Wayland surface binding and recreation). The eframe/XWayland path keeps displaying on the primary monitor, and the settings window says so when the platform reports no output names. No new dependency.
