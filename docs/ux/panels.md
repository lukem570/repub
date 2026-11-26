# Panels

Dockable panels (task panes) provide contextual tools and information. This file specifies panel behavior, the contents of each panel, and the shared color picker and number input components.

## Panel system

Left panels (Navigation, Pages, Master Pages, Layers) sit left of the central document canvas. Right panels (Format, Properties, Styles) sit right of it.

| Property | Value |
|----------|-------|
| Default width | 240px |
| Minimum width | 180px |
| Maximum width | 400px |
| Resize handle | 4px drag area |
| Collapse | Click header or double-click handle |

Each panel has a header with a collapse arrow, the panel title, and a close button (`×`), followed by the panel content.

### Collapsed state

A collapsed panel shows a narrow strip with an expand indicator (`◀`) and the panel icon or initial. Clicking the strip expands the panel.

### Auto-collapse (future)

- Window narrower than 1200px: right panels collapse.
- Window narrower than 1000px: left panels collapse.
- Hovering a collapsed indicator temporarily expands the panel.

### Keyboard shortcuts

| Shortcut | Action |
|----------|--------|
| `F5` | Toggle Pages panel |
| `F6` | Toggle Format panel |
| `Ctrl+Shift+L` | Toggle Layers panel |
| `Escape` | Close active panel |

## Pages panel

Located on the left side, top priority. Shows page thumbnails in a two-column grid, with a `+ Add Page` button below and a status line (`Pages: 4  |  Master: Default`) at the bottom. The current page is highlighted.

| Property | Value |
|----------|-------|
| Thumbnail size | 80×100px (aspect maintained) |
| Thumbnail gap | 8px |
| Border | 1px #D9D9D9 |
| Selected border | 2px #3B82F6 |
| Background | White |
| Shadow | 1px 2px 4px rgba(0,0,0,0.1) |

Each thumbnail contains a miniature render of the page content, the page number below it, and a master page indicator (icon or text).

| Action | Result |
|--------|--------|
| Click thumbnail | Navigate to page |
| Double-click | Rename page (future) |
| Right-click | Context menu |
| Drag thumbnail | Reorder pages |
| Click `+ Add Page` | Insert new page |

Context menu, in order, with separators between groups:

- Insert Page Before
- Insert Page After
- (separator)
- Duplicate Page
- (separator)
- Delete Page
- (separator)
- Apply Master Page (submenu)

## Format panel

Located on the right side, top priority. Content changes with the selection. Sections are collapsible.

### No selection

Shows "No objects selected" and "Select an object to see its formatting options."

### Shape selected

| Section | Controls |
|---------|----------|
| Position & Size | X, Y, W, H in pt; Rotation in degrees; Lock aspect ratio checkbox |
| Fill | Radio: Solid, Gradient, None; Color swatch with hex value (e.g. #3B82F6); Opacity % |
| Stroke | Color swatch with hex value (e.g. #000000); Width in pt (e.g. 1.0); Style dropdown |
| Effects | Checkboxes: Shadow, Reflection, Glow |

### Text frame selected

| Section | Controls |
|---------|----------|
| Position & Size | Same as shape |
| Text Frame | Columns dropdown (1); Margins Top/Bottom/Left/Right (4pt each); Vertical align dropdown (Top) |
| Character | Font dropdown (Calibri); Size (11pt); Toggle buttons Bold, Italic, Underline, Strikethrough, Font color; Color swatch |
| Paragraph | Four alignment buttons; Line spacing (1.0×); Before and After spacing (0pt each) |

### Multiple objects selected

Shows a count ("3 objects selected") and these sections:

| Section | Controls |
|---------|----------|
| Align | Horizontal: left, center, right; Vertical: top, middle, bottom |
| Distribute | Horizontally, Vertically |
| Common Properties | Fill and Stroke dropdowns, showing "Mixed" when values differ |

## Layers panel (future)

Lists layers top to bottom from foreground (Layer 3) to background (Layer 1). Each row has a visibility toggle, a lock toggle, and the layer name. The footer has `+ Add Layer`, `Delete`, and `Options` buttons.

| Action | Result |
|--------|--------|
| Click visibility | Toggle layer visibility |
| Click lock | Toggle layer lock |
| Click name | Select layer |
| Double-click name | Rename layer |
| Drag layer | Reorder layers |

## Styles panel (future)

- Text Styles section: Normal, Heading 1, Heading 2, Quote, Caption.
- Shape Styles section: grid of swatches, six per row.

## Master Pages panel (future)

Shows a `None` thumbnail, then master thumbnails (Master A, Master B) in a two-column grid, with a `+ Add Master` button.

## Color panel (future)

Sections, top to bottom:

- Scheme Colors: one row of eight accent color swatches.
- Standard Colors: two rows of ten swatches.
- Recent Colors: one row of five swatches.
- `More Colors...` button.

## Color picker component

Used in the Fill and Stroke sections. Contains:

- Saturation/brightness field with a circular picker.
- Hue slider.
- Alpha slider.
- RGB inputs (e.g. 255, 128, 64) and a hex field (#FF8040).
- Buttons: `OK`, `Cancel`, `Add to Custom Colors`.

## Number input component

Used for dimension and position inputs. A text input with spin buttons and a unit label.

- Direct text input with validation.
- Spin buttons: ±1pt per click.
- `Shift`+spin: ±10pt per click.
- Mouse wheel: ±1pt per tick.
- Accepts fractions (0.5pt).
- Units: pt, in, mm, px.
