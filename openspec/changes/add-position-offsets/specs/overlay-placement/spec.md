## Purpose

Position the overlay relative to an anchor with independent control over each axis, so it can be placed in a corner, along an edge, or just past one.

## ADDED Requirements

### Requirement: Each axis has its own offset
The overlay SHALL accept a horizontal and a vertical offset, each measured inward from the edge chosen by the anchor on that axis.

#### Scenario: Different insets per axis
- **WHEN** the anchor is `BottomRight` and the offsets are 40 and 12
- **THEN** the overlay sits 40 px from the right edge and 12 px from the bottom edge

#### Scenario: An edge anchor without a horizontal edge
- **WHEN** the anchor is `Top`, `Bottom`, or `Center`
- **THEN** the horizontal offset moves the overlay along the axis without flipping direction

#### Scenario: Offset past the edge
- **WHEN** an offset is negative
- **THEN** the overlay moves outward, past the anchored edge

### Requirement: Existing configuration keeps its placement
A settings file that specifies a single `margin` SHALL be read as both offsets.

#### Scenario: Loading an older file
- **WHEN** the file has `margin = 25` and no offsets
- **THEN** the overlay is placed with a horizontal and vertical offset of 25

#### Scenario: Saving
- **WHEN** settings are written
- **THEN** the file contains the two offsets and no `margin`

### Requirement: Both offsets are editable in the settings window
The settings window SHALL offer a horizontal and a vertical offset instead of the single distance field.

#### Scenario: Adjusting one axis
- **WHEN** the user changes the horizontal offset
- **THEN** only horizontal placement changes, and the vertical offset keeps its value
