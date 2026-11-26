# UX philosophy and principles

This file sets the UX principles for Repub, based on Microsoft Publisher 2016+ (Office 365). The goal is a desktop publishing application that feels native, professional, and immediately familiar to Publisher users.

## Design philosophy

### Direct manipulation

Users interact with objects directly on the canvas rather than through dialogs or forms. Anything available through menus can typically be done faster through direct interaction.

- Click to select, drag to move.
- Handles resize and rotate.
- Double-click to edit content (text, images).
- Right-click for contextual actions.
- Drag from the toolbox to create objects.

### WYSIWYG fidelity

What you see is what you print. The canvas:

- Shows actual fonts at proper sizes.
- Displays accurate colors (with color management).
- Renders text flow precisely.
- Shows bleeds and margins clearly.
- Indicates print boundaries.

### Progressive disclosure

Simple actions are obvious; complex features are discoverable:

1. Toolbar for common tools.
2. Context-sensitive Format panel.
3. Right-click context menus.
4. Menu bar for all features.
5. Dialogs for detailed settings.

### Consistent feedback

Every action gives immediate visual feedback:

- Hover states on interactive elements.
- Selection indicators on objects.
- Cursor changes for different operations.
- Status bar information.
- Undo/redo confirmation.

## Visual language

### Colors

Selection and interactive (accent blue):

| Token | Value | Use |
|-------|-------|-----|
| `--accent-primary` | `#3B82F6` | Selection highlight |
| `--accent-hover` | `#60A5FA` | Hover state |
| `--accent-active` | `#2563EB` | Active/pressed |

Workspace:

| Token | Value | Use |
|-------|-------|-----|
| `--workspace-bg` | `#505050` | Canvas background |
| `--page-bg` | `#FFFFFF` | Page |
| `--page-shadow` | rgba(0, 0, 0, 0.24) | Page drop shadow |

UI chrome:

| Token | Value | Use |
|-------|-------|-----|
| `--panel-bg` | `#F3F3F3` | Panel background |
| `--toolbar-bg` | `#FAFAFA` | Toolbar background |
| `--menu-bg` | `#FFFFFF` | Menu background |
| `--border-light` | `#D9D9D9` | Subtle borders |
| `--border-medium` | `#A6A6A6` | Standard borders |

Text:

| Token | Value | Use |
|-------|-------|-----|
| `--text-primary` | `#000000` | Primary text |
| `--text-secondary` | `#666666` | Secondary text |
| `--text-disabled` | `#A6A6A6` | Disabled text |

### Typography

UI fonts:

- Primary: Segoe UI (Windows), SF Pro (macOS), system-ui (Linux).
- Menu/toolbar: 12px regular.
- Panel headers: 12px semibold.
- Status bar: 11px regular.
- Dialog titles: 14px semibold.

Default document fonts:

- Body: Cambria, Calibri.
- Headings: Cambria, Calibri Light.
- Accent: Georgia, Arial.

### Spacing

Spacing uses a 4px grid:

| Value | Use |
|-------|-----|
| 4px | Minimal (between icons) |
| 8px | Tight (within groups) |
| 12px | Standard (between elements) |
| 16px | Medium (section padding) |
| 24px | Large (panel margins) |
| 32px | Extra large (dialog padding) |

### Icons

Icons follow the Fluent Design System:

- Size: 16×16 (toolbar), 24×24 (large toolbar), 32×32 (dialogs).
- Style: outlined, monochrome, with accent colors for active states.
- Stroke: 1px or 2px depending on size.
- Corners: rounded (2px radius for shapes).

## Interface components

### Main window layout

From top to bottom:

- Title bar: document name followed by "- Repub", with window controls.
- Menu bar: File, Edit, View, Insert, Format, Arrange, Help.
- Ribbon/toolbar: tabs Home, Insert, Page Design, Mailings, Review, View. Groups: Clipboard, Font, Paragraph, Styles, Objects, Arrange.
- Middle row, left to right: Pages panel (page thumbnails and an add-page control), document canvas (page centered), Format panel (Position X/Y, Size W/H).
- Status bar: for example "Page 1 of 3 | 100% | 1 object selected".

### Component specifications

| Component | Height | Background | Border |
|-----------|--------|------------|--------|
| Title bar | 32px | System | None |
| Menu bar | 24px | `#FAFAFA` | Bottom 1px |
| Ribbon | 100-120px | `#F3F3F3` | Bottom 1px |
| Toolbar (simple) | 40px | `#FAFAFA` | Bottom 1px |
| Status bar | 22px | `#F3F3F3` | Top 1px |
| Side panel | Varies | `#F5F5F5` | Inside 1px |

### Panels

- Resizable by dragging the border.
- Collapsible by clicking the header.
- Minimum width 180px, maximum 400px, default 240px.

## Interaction states

### Element states

Every interactive element has these states:

1. Normal: default appearance.
2. Hover: subtle highlight (lighter background).
3. Pressed: darker or inset appearance.
4. Selected: accent color highlight.
5. Disabled: grayed out (40% opacity).
6. Focused: keyboard focus ring (2px blue outline).

### Selection states

1. Unselected: normal appearance.
2. Hover: 1px subtle blue outline.
3. Selected: 2px blue outline, 8 resize handles, and a rotation handle (green circle).
4. Multi-selected: combined bounding box.
5. Locked: gray outline, no handles.

## Keyboard navigation

Focus management:

- `Tab` moves between major areas (toolbar, canvas, panels).
- Arrow keys navigate within areas.
- `Enter` activates the focused element.
- `Esc` cancels the operation or deselects.

Modifier keys:

- `Shift`: constrain (proportions, angles); extend selection.
- `Ctrl`/`Cmd`: add to selection; keyboard shortcuts.
- `Alt`/`Option`: duplicate while dragging; draw from center.
- `Space` (hold): temporary pan mode.

## Responsive behavior

Window resizing:

- Minimum window size is 800×600.
- The ribbon collapses groups at smaller sizes.
- Panels hide at very small sizes.
- The canvas always gets priority space.

High DPI:

- All UI scales with system DPI.
- Vector icons scale cleanly.
- Text stays crisp at all sizes.
- Handle sizes stay constant in pixels.

## Accessibility

Visual:

- Minimum text contrast ratio is 4.5:1.
- Selection is visible in high contrast mode.
- Color is never the only indicator.

Motor:

- All features are keyboard accessible.
- Click targets are at least 24×24.
- No action requires hover.

Screen readers:

- All interactive elements are labeled.
- Live regions announce status updates.
- Focus order is logical.

## Performance targets

| Operation | Target time |
|-----------|-------------|
| Selection response | < 16ms (60 FPS) |
| Object move/resize | < 16ms |
| Zoom | < 50ms |
| Pan | < 16ms |
| Menu open | < 100ms |
| Dialog open | < 200ms |
| Page render | < 100ms |
