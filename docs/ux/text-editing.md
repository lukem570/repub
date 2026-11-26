# Text editing

Specifies text frame modes, cursor, selection, input, formatting shortcuts, columns, frame linking, overflow, navigation, find and replace, spell checking, special characters, and bidirectional text. Text editing follows standard Windows conventions with desktop publishing additions.

## Text frame modes

- Object mode: the frame is selected as an object. Resize and rotate handles show. Text content cannot be edited. The frame can be moved, resized, and rotated.
- Text edit mode: the cursor is inside the frame. An I-beam text cursor shows and no object handles are visible. Text can be typed, selected, and formatted.

| Action | Current mode | Result |
|--------|--------------|--------|
| Click text frame | Object | Select frame |
| Double-click text frame | Object | Enter text edit mode |
| Click inside | Edit | Move cursor |
| Click outside | Edit | Exit to object mode |
| `Escape` | Edit | Exit to object mode |
| `Tab` | Edit | Insert tab, or move to next frame |

## Text cursor

- Blinking vertical line, black (`#000000`), height matches line height.
- Blink rate is the system default (530ms on, 530ms off).
- The cursor sits between characters. A click positions it at the nearest character boundary, using font metrics.

```rust
fn position_to_cursor(
    point: Point,
    text: &StyledText,
    layout: &TextLayout,
) -> CursorPosition {
    let line = layout.line_at_y(point.y);
    let char_index = layout.char_at_x(line, point.x);
    CursorPosition { line, char_index }
}
```

## Text selection

Selected text is inverse: white (`#FFFFFF`) text on an accent blue (`#3B82F6`) background at 100% opacity.

### Mouse selection

| Action | Result |
|--------|--------|
| Click | Position cursor |
| Double-click | Select word |
| Triple-click | Select paragraph |
| Drag | Select range |
| `Shift`+click | Extend selection to click |

### Keyboard selection

| Shortcut | Result |
|----------|--------|
| `Shift+Left`/`Shift+Right` | Extend by character |
| `Shift+Up`/`Shift+Down` | Extend by line |
| `Ctrl+Shift+Left`/`Ctrl+Shift+Right` | Extend by word |
| `Ctrl+Shift+Home`/`Ctrl+Shift+End` | Extend to start/end |
| `Shift+Home`/`Shift+End` | Extend to line start/end |
| `Ctrl+A` | Select all in frame |

### Word boundaries

```rust
fn is_word_boundary(c: char) -> bool {
    c.is_whitespace() || matches!(c,
        '.' | ',' | ';' | ':' | '!' | '?' |
        '(' | ')' | '[' | ']' | '{' | '}' |
        '"' | '\'' | '-' | '/' | '\\'
    )
}
```

## Text input

Standard keyboard input, IME support for international input, and special characters via Alt codes.

| Shortcut | Action |
|----------|--------|
| `Ctrl+C` | Copy |
| `Ctrl+X` | Cut |
| `Ctrl+V` | Paste |
| `Ctrl+Z` | Undo |
| `Ctrl+Y` | Redo |
| `Delete` | Delete forward |
| `Backspace` | Delete backward |
| `Enter` | New paragraph |
| `Shift+Enter` | Line break (soft return) |
| `Tab` | Insert tab |
| `Ctrl+Tab` | Move to next field |

### Paste behavior

| Source | Result |
|--------|--------|
| Plain text | Paste with current style |
| Formatted text | Paste with formatting |
| `Ctrl+Shift+V` | Paste plain (strip formatting) |

## Text formatting

### Character formatting

| Shortcut | Formatting |
|----------|------------|
| `Ctrl+B` | Bold toggle |
| `Ctrl+I` | Italic toggle |
| `Ctrl+U` | Underline toggle |
| `Ctrl+Shift+K` | Small caps |
| `Ctrl+=` | Subscript |
| `Ctrl+Shift+=` | Superscript |
| `Ctrl+Shift+>` | Increase font size |
| `Ctrl+Shift+<` | Decrease font size |

Font size steps:

```rust
const FONT_SIZES: &[f32] = &[
    6.0, 7.0, 8.0, 9.0, 10.0, 10.5, 11.0, 12.0,
    14.0, 16.0, 18.0, 20.0, 22.0, 24.0, 26.0, 28.0,
    36.0, 48.0, 72.0, 96.0
];
```

### Paragraph formatting

| Shortcut | Formatting |
|----------|------------|
| `Ctrl+L` | Align left |
| `Ctrl+E` | Align center |
| `Ctrl+R` | Align right |
| `Ctrl+J` | Justify |
| `Ctrl+1` | Single line spacing |
| `Ctrl+2` | Double line spacing |
| `Ctrl+5` | 1.5 line spacing |
| `Ctrl+M` | Increase indent |
| `Ctrl+Shift+M` | Decrease indent |

## Columns

A text frame can have 1-10 columns, with a configurable gutter width (space between columns) and equal or unequal column widths.

```text
┌───────────────────────────────────────────┐
│ Lorem ipsum    │ Sed ut perspic │ At vero │
│ dolor sit      │ iatis unde     │ eos et  │
│ amet, consec   │ omnis iste     │ accusam │
│ tetur adipis   │ natus error    │ et justo│
│ cing elit.     │ sit voluptat   │ duo     │
└───────────────────────────────────────────┘
      Column 1        Column 2      Column 3
```

## Text frame linking

Text flows between linked frames.

```text
Frame 1              Frame 2              Frame 3
┌──────────┐    ┌──────────┐    ┌──────────┐
│ Lorem    │───→│ dolor sit│───→│ adipis   │
│ ipsum    │    │ amet     │    │ cing     │
└──────────┘    └──────────┘    └──────────┘
     ↑                                ↓
     └────────────────────────────────┘
              Linked frames
```

Link indicators:

- A blue arrow on the frame edge indicates a link: an arrow pointing out is an outgoing link, an arrow pointing in is an incoming link.
- A small red "+" in the corner of a full frame is the overflow indicator.

## Overflow handling

When text does not fit in the frame, a red overflow indicator shows in the lower-right corner. Overflow options:

- Show indicator: red ⊕ in the corner.
- Auto-resize: the frame grows to fit (optional).
- Link to new frame: create a linked frame.
- Shrink text: auto-fit text to the frame.

## Cursor navigation

| Key | Movement |
|-----|----------|
| `Left`/`Right` | One character |
| `Up`/`Down` | One line |
| `Ctrl+Left`/`Ctrl+Right` | One word |
| `Home` | Line start |
| `End` | Line end |
| `Ctrl+Home` | Frame start |
| `Ctrl+End` | Frame end |
| `Page Up` | One page (in linked frames) |
| `Page Down` | One page |

Vertical movement preserves the cursor's preferred x position:

```rust
fn move_cursor_up(cursor: &mut CursorPosition, layout: &TextLayout) {
    if cursor.line > 0 {
        cursor.line -= 1;
        cursor.char_index = layout.char_at_x(cursor.line, cursor.preferred_x);
    }
}
```

## Find and replace

- Find (`Ctrl+F`): a Find field, a Find Next button, and Match case and Whole words only checkboxes.
- Replace (`Ctrl+H`): Find and Replace fields, Find Next, Replace, and Replace All buttons, and Match case and Whole words only checkboxes.

## Spell checking

- Spelling errors show a red wavy underline.
- Grammar errors show a blue wavy underline (future).

Right-clicking a misspelled word opens a context menu:

- Spelling suggestions (for example `quick`, `quack`, `quirk` for `quik`)
- Ignore, Ignore All, Add to Dictionary
- AutoCorrect (submenu)

## Special characters

| Shortcut | Character |
|----------|-----------|
| `Ctrl+-` | En dash (–) |
| `Ctrl+Shift+-` | Em dash (—) |
| `Ctrl+Shift+Space` | Non-breaking space |
| `Ctrl+Shift+Enter` | Column break |
| `Ctrl+Enter` | Frame break |

The Insert > Symbol dialog provides recently used symbols, font-specific symbols, and a Unicode character picker.

## Bidirectional text

- Direction is detected automatically.
- LTR and RTL text can be mixed in the same paragraph.
- Cursor movement follows logical order.

| Shortcut | Action |
|----------|--------|
| `Ctrl+Shift` | Toggle input direction |
| Left `Ctrl+Shift` | LTR input |
| Right `Ctrl+Shift` | RTL input |
