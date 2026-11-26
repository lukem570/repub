# Object manipulation

Specifies how objects are moved, resized, rotated, and snapped through direct manipulation.

## Moving objects

Dragging a selected object (not a handle) moves it. The cursor changes to the Move cursor, the object moves in real time with its outline following the mouse, and it snaps to its final position on release.

| Modifier | Behavior |
|----------|----------|
| None | Free movement |
| `Shift` | Constrain to horizontal or vertical |
| `Alt` | Duplicate object while moving |
| `Shift+Alt` | Constrain and duplicate |

The `Shift` constraint compares the absolute drag distance on each axis from the start point. If the horizontal distance is greater, the object moves horizontally only (y stays at the start value). Otherwise it moves vertically only.

When multiple objects are selected, all move by the same delta, relative positions are preserved, and snapping uses the combined bounding box.

## Resizing objects

Each corner and edge has a resize handle:

```text
TL──────TC──────TR
│                │
ML              MR
│                │
BL──────BC──────BR
```

| Handle | X behavior | Y behavior |
|--------|------------|------------|
| TL | Resize from left | Resize from top |
| TC | No change | Resize from top |
| TR | Resize from right | Resize from top |
| ML | Resize from left | No change |
| MR | Resize from right | No change |
| BL | Resize from left | Resize from bottom |
| BC | No change | Resize from bottom |
| BR | Resize from right | Resize from bottom |

Dragging a handle applies the drag delta to the edges that handle controls. A resize that would make the width or height smaller than the minimum size is not applied. The minimum object size is 1×1 pixel (0.5 pt).

| Modifier | Behavior |
|----------|----------|
| None | Free resize |
| `Shift` | Maintain aspect ratio |
| `Alt` | Resize from center |
| `Shift+Alt` | Maintain aspect ratio and resize from center |

Resizing from center keeps the object center fixed and applies the delta to both sides, so the size changes by twice the delta on each axis.

### Resizing multiple objects

When multiple objects are selected, each object resizes proportionally and the gaps between objects scale proportionally. Corner handles are recommended. Each object's position relative to the group bounds is preserved as a fraction of the group size, and its width and height are multiplied by the group's x and y scale factors.

## Rotating objects

The rotation handle is a green circle above the top-center of the selection, joined to it by a green 1px line.

Dragging the handle changes the cursor to the rotation cursor, rotates the object around its center, and shows an angle tooltip with the current rotation (for example, "45°"). The angle is the change in the angle (`atan2`) from the object center to the handle's start and current positions.

| Modifier | Behavior |
|----------|----------|
| None | Free rotation |
| `Shift` | Snap to 15° increments (0°, 15°, 30°, 45°, 60°, 75°, 90°, and so on) |

Rotation is around the object center.

## Skewing objects

Skew is available only through the Format Shape dialog (horizontal skew and vertical skew), not through direct manipulation handles.

## Snapping

Snap types:

- Grid: align to the document grid.
- Guide: align to ruler guides.
- Object: align to other object edges and centers.
- Margin: align to page margins.

The snap tolerance is 8 pixels. A position snaps to the first snap point closer than the tolerance.

An object's snap points are its left, center, and right x-positions (vertical alignment) and its top, center, and bottom y-positions (horizontal alignment).

### Smart guides

When moving or resizing, alignment guides appear between aligned objects.

| Property | Value |
|----------|-------|
| Color | Accent blue (`#3B82F6`) |
| Style | Dashed (4px dash, 4px gap) |
| Width | 1px |

## Status bar feedback

| Operation | Status bar display |
|-----------|--------------------|
| Move | "X: 100.5 pt  Y: 200.0 pt" |
| Resize | "W: 150.0 pt  H: 100.0 pt" |
| Rotate | "Rotation: 45.0°" |
| Multi-move | "Moving 3 objects" |

## Undo and redo

Each manipulation is one undoable command. Multi-object operations are grouped into a single command.

| Command | Data |
|---------|------|
| Move | Object IDs and the move delta. Execute adds the delta to each object's origin; undo subtracts it. |
| Resize | Object ID, old bounds, new bounds |
| Rotate | Object IDs, rotation center, angle delta |

## Precision input

| Key | Movement |
|-----|----------|
| Arrow | 1 point |
| `Shift+Arrow` | 10 points |
| `Ctrl+Arrow` | 1 pixel (at current zoom) |

The format panel accepts numeric entry for position (X, Y in pt), size (W, H in pt), and rotation (degrees):

```text
Position: X [100.0] pt  Y [200.0] pt
Size:     W [150.0] pt  H [100.0] pt
Rotation: [45.0]°
```
