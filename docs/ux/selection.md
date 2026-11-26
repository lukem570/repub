# Selection

Selection behavior in Publisher: handles, selection modes, multi-selection display, hit testing, cursors, persistence, groups, and locked objects.

## Selection handles

Selected objects show white square handles with black borders, a blue selection outline, and a green rotation handle.

```text
                    ┌─── Rotation handle (green circle)
                    │    Connected by green line
                    ▼
                    ◉
                    │
        ┌───────────┬───────────┐
        │■          ■          ■│ ← Top handles
        │                       │
        │■                     ■│ ← Middle handles
        │                       │
        │■          ■          ■│ ← Bottom handles
        └───────────┴───────────┘
```

| Property | Value |
|----------|-------|
| Handle size | 8×8 px |
| Handle fill | White (`#FFFFFF`) |
| Handle border | Black (`#000000`), 1px solid |
| Selection outline | Accent blue (`#3B82F6`), 2px |

Rotation handle:

| Property | Value |
|----------|-------|
| Diameter | 10 px |
| Fill | Green (`#22C55E`) |
| Border | Dark green (`#16A34A`), 1px |
| Connection line | Green (`#22C55E`), 1px |
| Distance | 20 px above the top-center handle |

Handle positions: `TopLeft`, `TopCenter`, `TopRight`, `MiddleLeft`, `MiddleRight`, `BottomLeft`, `BottomCenter`, `BottomRight`, and `Rotation` (above top-center).

## Selection modes

### Click

Clicking an object selects it and deselects others.

- Clicking empty space deselects all.
- Clicking a selected object keeps it selected, allowing a drag.
- Clicking through transparent areas checks object bounds, not pixels.

### Shift+click (extend)

`Shift`+click adds an unselected object to the selection or removes a selected object from it. `Ctrl`+click behaves identically in Publisher.

### Marquee

Dragging on empty canvas draws a selection rectangle.

- Objects fully enclosed are selected (default).
- Objects only partially touching are not selected.
- `Alt`+drag selects touching objects.

| Property | Value |
|----------|-------|
| Fill | `rgba(59, 130, 246, 0.1)` (10% opacity blue) |
| Border | 1px `#3B82F6`, dashed or solid |

### Select all

`Ctrl+A` selects all unlocked objects on the current page.

## Multi-selection display

- Each selected object gets a 1px light blue (`#93C5FD`) outline.
- The combined bounding box gets a 2px accent blue outline and handles.
- The rotation handle appears on the combined bounds.
- Dragging a handle affects all selected objects proportionally.

## Hit testing

Objects are tested in reverse z-order (top to bottom). Layers are iterated in reverse, skipping layers that are not visible or are locked. Within each layer, objects are iterated in reverse and the first object whose `hit_test(point)` succeeds is returned.

Hit regions:

| Region | Meaning | Tolerance |
|--------|---------|-----------|
| `None` | Miss | n/a |
| `Interior` | Inside the fill | Within bounds |
| `Edge` | On the stroke | ±3 px from stroke |
| `Handle(HandlePosition)` | On a resize handle | ±4 px from handle center |
| `RotationHandle` | On the rotation handle | ±5 px from center |
| `Text` | Inside editable text | n/a |

## State machine

| State | Event | Next state |
|-------|-------|------------|
| Idle (no selection) | Click on object | Selected |
| Selected (1+ objects) | Click empty space | Idle |
| Selected | Drag on handle | Manipulating (resize/rotate) |
| Manipulating | Mouse up | Idle |

## Cursors

| Context | Cursor | Source |
|---------|--------|--------|
| Over empty space | Default arrow | System |
| Over selectable object | Default arrow | System |
| Over selected object body | Move (four-way arrow) | Custom |
| Over top-left or bottom-right handle | NWSE resize | Custom or system |
| Over top-right or bottom-left handle | NESW resize | Custom or system |
| Over top-center or bottom-center handle | NS resize | System |
| Over middle-left or middle-right handle | EW resize | System |
| Over rotation handle | Rotate (circular arrow) | Custom |
| Dragging marquee | Crosshair (+) | System |

## Persistence

- Selection persists through zoom, pan, and page navigation.
- Selection clears on page change (unless grouped across pages), document close, and the deselect action.
- Selection changes are not recorded in undo history.
- After undo or redo, the selection is cleared.

## Groups

- Clicking any group member selects the group.
- A selected group shows a single bounding box with handles and no individual outlines.
- Double-clicking enters group edit mode, where individual members can be selected. The group boundary shows a dashed outline.
- Clicking outside the group or pressing `Escape` exits group edit mode.

## Locked objects

Locked objects are visible on the page but not interactive.

- Appearance: 1px dashed gray (`#A6A6A6`) outline, no handles, optional lock icon in the corner.
- Cannot be selected by click or by marquee.
- Selectable only through the Page Navigator or the Layers panel.
- Clicking one shows "Object locked" in the status bar.
