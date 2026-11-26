# Menu structure

Specifies the menu bar, every menu and submenu, menu styling, and conditional enablement for Repub. The structure follows Microsoft Publisher's menu organization, adapted for cross-platform use.

Each bullet below is one group of items; groups are separated by a divider in the rendered menu. Items ending in `...` open a dialog. Items marked "submenu" open a submenu.

## Menu bar

```text
File   Edit   View   Insert   Format   Arrange   Table   Help
```

## File menu

- New... (`Ctrl+N`), New from Template...
- Open... (`Ctrl+O`), Open Recent (submenu)
- Close (`Ctrl+W`)
- Save (`Ctrl+S`), Save As... (`Ctrl+Shift+S`), Save a Copy...
- Export... (`Ctrl+E`), Export as PDF..., Export as Image...
- Page Setup..., Print... (`Ctrl+P`)
- Properties...
- Exit (`Ctrl+Q`)

### Open Recent submenu

- Numbered list of recent files (for example `1. Newsletter.pcd`, `2. Business Card.pcd`, `3. Flyer Design.pcd`, `4. Annual Report.pcd`, `5. Wedding Invitation.pcd`)
- Clear Recent Files

## Edit menu

- Undo (`Ctrl+Z`), Redo (`Ctrl+Y` or `Ctrl+Shift+Z`)
- Cut (`Ctrl+X`), Copy (`Ctrl+C`), Paste (`Ctrl+V`), Paste Special... (`Ctrl+Shift+V`), Delete (`Del`)
- Duplicate (`Ctrl+D`)
- Select All (`Ctrl+A`), Select All Text
- Find... (`Ctrl+F`), Find and Replace... (`Ctrl+H`)
- Preferences... (`Ctrl+,`)

## View menu

- Zoom (submenu)
- Rulers (`Ctrl+R`, checkable, shown checked), Guides (checkable, shown checked), Snap to Guides, Grid, Snap to Grid
- Boundaries and Guides, Special Characters, Text Frame Links
- Pages Panel (`F5`, shown checked), Format Panel (`F6`, shown checked), Layers Panel, Styles Panel
- Master Page (submenu), Two-Page Spread
- Full Screen (`F11`)

### Zoom submenu

- Zoom In (`Ctrl++`), Zoom Out (`Ctrl+-`)
- 400%, 200%, 150%, 100% (`Ctrl+1`, current level marked with a bullet), 75%, 50%, 25%
- Fit to Window (`Ctrl+0`), Fit Page Width, Actual Size (`Ctrl+1`)
- Custom Zoom...

### Master Page submenu

- View Master Page, Edit Master Page
- None, Master A, Master B

## Insert menu

- Text Frame, Image..., Online Image...
- Shape (submenu), Line (submenu), WordArt
- Table...
- Page (submenu), Section...
- Header and Footer..., Page Numbers..., Date and Time...
- Hyperlink... (`Ctrl+K`), Bookmark...
- Symbol..., Building Blocks (submenu)

### Shape submenu

- Rectangle (`R`), Rounded Rectangle, Ellipse (`E`)
- Triangle, Right Triangle, Polygon... (`P`), Star... (`S`)
- Arrow (submenu), Callout (submenu), Flowchart (submenu)
- More Shapes...

### Line submenu

- Line (`L`), Arrow, Double Arrow
- Connector: Straight, Connector: Elbow, Connector: Curved
- Freeform, Scribble

### Page submenu

- Insert Blank Page, Insert Duplicate Page
- Insert Page Before..., Insert Page After...

## Format menu

- Font..., Paragraph...
- Bullets and Numbering..., Tabs..., Drop Cap...
- Text Styles (submenu), Shape Styles (submenu)
- Shape (submenu), Picture (submenu)
- Columns..., Hyphenation...
- Format Painter (`Ctrl+Shift+C`)

### Shape submenu

- Fill (submenu), Stroke (submenu), Effects (submenu)
- Change Shape (submenu)
- Format Shape...

### Fill submenu

- No Fill
- Scheme color swatches (Scheme Color 1, Scheme Color 2, Scheme Color 3, and so on)
- More Colors..., Gradient..., Texture..., Pattern...

### Effects submenu

Shadow, Reflection, Glow, Soft Edges, 3-D Rotation, 3-D Format. Each is a submenu.

## Arrange menu

- Group (`Ctrl+G`), Ungroup (`Ctrl+Shift+G`)
- Order (submenu)
- Align (submenu), Distribute (submenu)
- Rotate (submenu), Flip (submenu)
- Wrap Text (submenu)
- Layout Guides..., Ruler Guides (submenu)
- Lock, Unlock

### Order submenu

- Bring to Front (`Ctrl+Shift+]`)
- Bring Forward (`Ctrl+]`)
- Send Backward (`Ctrl+[`)
- Send to Back (`Ctrl+Shift+[`)

### Align submenu

- Align Left, Align Center, Align Right
- Align Top, Align Middle, Align Bottom
- Relative to Margin (shown checked), Relative to Page, Relative to Selection

### Distribute submenu

Distribute Horizontally, Distribute Vertically.

### Rotate submenu

- Rotate Right 90°, Rotate Left 90°, Rotate 180°
- Free Rotate, Custom Rotation...

### Flip submenu

Flip Horizontal, Flip Vertical.

## Table menu

- Insert Table...
- Insert (submenu), Delete (submenu)
- Select (submenu)
- Merge Cells, Split Cells...
- Distribute Rows, Distribute Columns
- Table Properties...

### Insert submenu (table)

Insert Row Above, Insert Row Below, Insert Column Left, Insert Column Right.

### Delete submenu (table)

Delete Row, Delete Column, Delete Table.

## Help menu

- Documentation (`F1`), Keyboard Shortcuts...
- Check for Updates...
- Report a Bug..., Request a Feature...
- About Repub

## Menu styling

### Menu container

| Property | Value |
|----------|-------|
| Background | `#FFFFFF` |
| Border | 1px `#D9D9D9` |
| Shadow | 2px 4px 8px rgba(0,0,0,0.15) |
| Border radius | 4px |
| Padding | 4px 0 |

### Menu item

| Property | Value |
|----------|-------|
| Height | 24px |
| Padding | 0 24px 0 12px |
| Font | Segoe UI 12px |
| Text color | `#000000` |
| Shortcut color | `#666666` |

### Menu item states

| State | Background | Text |
|-------|------------|------|
| Normal | Transparent | `#000000` |
| Hover | `#E5F3FF` | `#000000` |
| Pressed | `#CCE8FF` | `#000000` |
| Disabled | Transparent | `#A6A6A6` |

## Conditional states

### Clipboard-dependent

| Item | Enabled when |
|------|--------------|
| Cut | Has selection |
| Copy | Has selection |
| Paste | Clipboard has content |
| Delete | Has selection |

### Selection-dependent

| Item | Enabled when |
|------|--------------|
| Group | 2+ objects selected |
| Ungroup | Group selected |
| Order items | Has selection |
| Align items | Has selection |
| Distribute | 3+ objects selected |

### Document-dependent

| Item | Enabled when |
|------|--------------|
| Save | Document dirty |
| Close | Document open |
| Export | Document open |
| Undo | Has undo history |
| Redo | Has redo history |
