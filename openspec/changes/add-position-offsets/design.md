## Context

`WindowPosition` provides seven anchors and `anchor_and_offset(margin)` turns each into an egui alignment plus an offset vector, with `margin` applied to both axes and always inward. The desktop hosts call it once per frame with `settings.active.margin`, and the settings window exposes a single "Distance from screen edge" field.

The overlay is a small floating panel, so the useful placement vocabulary is "which corner or edge" plus "how far in". Arbitrary placement was considered and rejected: it needs a drag-to-place UI or fractional coordinates nobody can read, while per-axis offsets cover the practical cases with two integers.

## Goals / Non-Goals

**Goals:**

- Independent horizontal and vertical insets from the anchored edge.
- Allow deliberately pushing the overlay past an edge (negative offset).
- Keep existing configuration files looking the same.

**Non-Goals:**

- Fractional or absolute placement, drag-to-place, or snapping.
- Per-output placement; placement stays one global value.
- Changing the anchor set, the size, or the fade behaviour.

## Decisions

### Inward offsets per axis, signed

`offset_x`/`offset_y` mean "distance inward from the edge this anchor uses", so `BottomRight` with `(40, 12)` sits 40 px from the right and 12 px from the bottom. A negative value moves outward, past the edge, which is how a partially off-screen overlay is expressed.

The alternative — raw screen-relative displacement — would make the same numbers mean different things per anchor and invert when the user switches corners; the whole point of an anchor is that it owns the direction.

For the three anchors without a horizontal edge (`Top`, `Bottom`, `Center`), `offset_x` is a plain horizontal displacement, and for `Center` both are displacements. This is documented in the settings tooltips rather than modelled as a separate type.

### `margin` is read, never written

Lexing keeps `margin` as a fallback when `offset_x`/`offset_y` are absent, so a settings file from an earlier version keeps its appearance. Writing emits only the two offsets. A build that only understands `margin` therefore falls back to its default placement rather than inheriting the tuned values — acceptable, since the downstream is the consumer of its own configuration.

### Two fields in the settings window, not a combined pair

Each axis gets its own row with a px unit, so a value can be nudged without touching the other. The rows are labelled by axis ("Horizontal offset", "Vertical offset") with a tooltip explaining the relation to the anchor, which keeps the window usable when the anchor changes.

## Risks

- **Sign confusion.** An offset that moves outward in a corner anchor is still "inward" internally; the tooltip is the only guard. Getting it wrong is visible immediately, since the overlay is on screen while the settings window is open.
- **Off-screen overlay.** Negative offsets can hide the overlay entirely, and a saved value survives a restart. The settings window remains reachable from the tray, so this is recoverable.
