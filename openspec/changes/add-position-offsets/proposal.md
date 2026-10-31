## Why

Placement is an anchor plus a single margin that has to serve both axes, so an overlay in the bottom-right corner cannot sit closer to the edge horizontally than vertically — the two insets are welded together. Offsets are also clamped to inward movement, and the smallest non-zero inset is one pixel for both axes at once.

## What Changes

- Replace the single `margin` setting with `offset_x` and `offset_y`, measured inward from the anchored edge of each axis, where a negative value moves past that edge.
- Migrate existing configurations: a file with `margin` and no offsets keeps its current appearance.
- Offer both offsets in the settings window instead of the single distance field.

## Capabilities

### New Capabilities

- `overlay-placement`: Where the overlay sits on its output.

### Modified Capabilities

None.

## Impact

Affects `keypeek-presentation` (settings model, serialization, settings window, the anchor-to-position mapping). The map is shared by both desktop hosts and the browser runner, so all three follow. Placement stays global rather than per-output.
