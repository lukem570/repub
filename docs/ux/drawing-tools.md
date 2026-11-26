# Drawing tools

Behavior of each toolbar tool in Publisher. Creation tools make objects on the canvas through drag interactions.

## Tools

| Tool | Shortcut | Purpose |
|------|----------|---------|
| Select | `V` | Select and manipulate objects |
| Pan | `H` | Pan/scroll the canvas |
| Zoom | `Z` | Zoom the view |
| Text | `T` | Create text frames |
| Image | none | Insert images |
| Rectangle | `R` | Draw rectangles |
| Ellipse | `E` | Draw ellipses and circles |
| Line | `L` | Draw straight lines |
| Polygon | `P` | Draw regular polygons |
| Star | `S` | Draw stars |

## Select tool

Cursors:

- Over empty space: default arrow
- Over an object: arrow (move cursor if the object is already selected)
- Dragging a marquee: crosshair

| Action | Result |
|--------|--------|
| Click empty space | Deselect all |
| Click object | Select the object and deselect others |
| `Shift`+click object | Add to or remove from selection |
| Drag on empty space | Marquee selection |
| Drag on selected object | Move selected objects |
| Drag on handle | Resize selected objects |
| Drag on rotation handle | Rotate selected objects |
| Double-click text | Enter text edit mode |
| Double-click group | Enter group edit mode |

See [selection.md](selection.md) for handles, hit testing, and group behavior.

## Text tool

Cursors: I-beam normally, crosshair while drawing.

Creating a text frame:

1. Select the Text tool (`T`) or choose it from the toolbar. The cursor becomes a crosshair.
2. Drag on the canvas to define the frame bounds. The drag outline is a 1px solid `#3B82F6` border with no fill.
3. On release, the frame is created with the dragged dimensions and selected, and the cursor enters text edit mode (I-beam inside the frame). The tool remains Text until the user changes it.

A click without a drag creates a default-sized text frame (100×50 pt) or shows an insertion point for an auto-sizing frame.

Default text frame properties:

| Property | Value |
|----------|-------|
| Minimum size | 10×10 pt |
| Fill | None (transparent) |
| Stroke | None |
| Margins | 4 pt on all sides |
| Font | Calibri 11 pt (or system default) |

## Rectangle tool

Cursor: crosshair (+).

1. Select the Rectangle tool (`R`).
2. Drag from one corner to the opposite corner. The rectangle previews during the drag with fill `#E5E5E5` (light gray) and a 1px solid `#333333` border.
3. On release, the rectangle is created and selected, and the tool switches to Select.

| Modifier | Behavior |
|----------|----------|
| None | Free rectangle |
| `Shift` | Constrain to square |
| `Alt` | Draw from center (the start point is the center) |
| `Shift+Alt` | Square from center |

Default properties:

- Fill: accent color 1 from the scheme (usually blue)
- Stroke: 1 pt black
- Corner radius: 0

Publisher has a separate rounded rectangle with an adjustable corner radius. The default radius is 10 pt.

## Ellipse tool

Cursor: crosshair (+).

1. Select the Ellipse tool (`E`).
2. Drag to define the bounding box. The ellipse inscribed in the box previews during the drag with fill `#E5E5E5` and a 1px solid `#333333` border.
3. On release, the ellipse is created and selected, and the tool switches to Select.

| Modifier | Behavior |
|----------|----------|
| None | Free ellipse |
| `Shift` | Constrain to circle |
| `Alt` | Draw from center |
| `Shift+Alt` | Circle from center |

Default properties:

- Fill: accent color 1
- Stroke: 1 pt black

## Line tool

Cursor: crosshair (+).

1. Select the Line tool (`L`).
2. Drag from the start point to the end point. The end point follows the mouse.
3. On release, the line is created and selected, and the tool switches to Select.

| Modifier | Behavior |
|----------|----------|
| None | Free angle |
| `Shift` | Constrain to 15° increments (0°, 15°, 30°, ... 165°, 180°) |

With `Shift`, the angle of the drag vector, `atan2(dy, dx)`, is rounded to the nearest multiple of 15° and the line length is preserved.

Default properties:

- Stroke: 1 pt black
- Start cap: none
- End cap: none
- Arrow style: none

Predefined line types:

- Plain line
- Arrow (end)
- Double arrow (both ends)
- Connector lines (elbow, curved)

## Polygon tool

Cursor: crosshair (+).

Creates regular polygons inscribed in a circle.

1. Select the Polygon tool (`P`).
2. Drag to define the bounding circle. The polygon previews during the drag.
3. On release, the polygon is created and selected, and the tool switches to Select.

| Modifier | Behavior |
|----------|----------|
| None | Free bounding ellipse |
| `Shift` | Constrain to circle (regular polygon) |
| `Alt` | Draw from center |

The default is 6 sides (hexagon). The side count is set in the tool options bar, typically 3-12.

Default properties:

- Fill: accent color 1
- Stroke: 1 pt black
- Sides: 6

## Star tool

Cursor: crosshair (+).

1. Select the Star tool (`S`).
2. Drag to define the bounding box. The star previews during the drag.
3. On release, the star is created and selected, and the tool switches to Select.

Parameters:

- Points: number of star points (default 5)
- Inner radius ratio: how deep the indents go (default 0.5)

The star path has `points * 2` vertices at equal angular steps of `TAU / (points * 2)`. The first vertex is at the top (angle -π/2). Even-indexed vertices lie at the outer radius and odd-indexed vertices at the inner radius (`outer_radius * inner radius ratio`). The path is a closed polyline.

## Pan tool

Cursors: open hand (grab) normally, closed hand (grabbing) while dragging.

| Action | Result |
|--------|--------|
| Click | Nothing |
| Drag | Pan the canvas by the drag delta |

Other ways to pan:

- `Space`+drag: temporary pan mode with any tool
- Middle mouse drag: pan with any tool
- Scroll wheel: vertical pan
- `Shift`+scroll: horizontal pan

## Zoom tool

Cursors: magnifying glass with + normally, with - while `Alt` is held.

| Action | Result |
|--------|--------|
| Click | Zoom in, centered on the click |
| `Alt`+click | Zoom out, centered on the click |
| Drag | Zoom to the rectangle |

Zoom levels: 10%, 25%, 33%, 50%, 67%, 75%, 100%, 125%, 150%, 200%, 300%, 400%, 600%, 800%.

| Shortcut | Action |
|----------|--------|
| `Ctrl++` | Zoom in |
| `Ctrl+-` | Zoom out |
| `Ctrl+0` | Fit to window |
| `Ctrl+1` | 100% (actual size) |

## Image tool

Cursor: crosshair with an image icon.

Drag to define size:

1. Select the Image tool.
2. Drag on the canvas to define the image frame.
3. The file picker opens.
4. The image is placed and fitted to the frame.

Click to place:

1. Select the Image tool. The file picker opens immediately.
2. Click on the canvas to place the image's top-left corner.
3. The image is placed at its natural size.

Fit modes:

| Mode | Behavior |
|------|----------|
| Fill | Image fills the frame and may crop |
| Fit | Image fits entirely in the frame and may letterbox |
| Stretch | Image stretches to fill the frame exactly |
| Tile | Image tiles to fill the frame |

## Tool options bar

Tools with options show a secondary toolbar:

| Tool | Controls |
|------|----------|
| Rectangle | Corners (pt), Fill, Stroke |
| Polygon | Sides, Fill, Stroke |
| Star | Points, Indent (%), Fill |

## Tool switching

After creating an object, Rectangle, Ellipse, Line, Polygon, and Star switch back to Select. Exceptions:

- Text: stays in text edit mode
- Image: stays active for multiple images (option)
