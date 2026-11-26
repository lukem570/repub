# Context menus

Context menus (right-click menus) provide quick access to actions relevant to the clicked target. This file specifies the contents, ordering, styling, and enable/show conditions of every context menu in Repub.

## Principles

- Show only applicable actions.
- Give similar objects similar menus.
- Put the most common actions at the top.
- Use submenus for related options.
- Show keyboard shortcuts.
- Open submenus after a 200ms hover delay.

## Notation

Menus are listed top to bottom. A blank line separates groups divided by a separator line. `>` marks a submenu. Shortcuts follow the item name. `(checked)` and `(selected)` mark the default state of check and radio items.

The edit group is the following set of items, used at the top of most object menus:

```text
Cut             Ctrl+X
Copy            Ctrl+C
Paste           Ctrl+V
Duplicate       Ctrl+D
Delete          Del
```

## Canvas (no selection)

Right-click on an empty canvas area.

```text
Paste               Ctrl+V
Paste Special...    Ctrl+Shift+V

Select All          Ctrl+A

View >
Zoom >

Insert >

Guides >
Grid >

Page Setup...
```

View submenu:

```text
Rulers (checked)
Guides (checked)
Grid

Boundaries
Text Frame Links
```

Insert submenu:

```text
Text Frame
Image...

Rectangle
Ellipse
Line

Table...
```

## Single object

Right-click on a selected object.

```text
(edit group)

Order >

Group               Ctrl+G

Rotate >
Flip >

Format Shape...

Hyperlink...        Ctrl+K

Lock
```

Order submenu:

```text
Bring to Front      Ctrl+Shift+]
Bring Forward       Ctrl+]
Send Backward       Ctrl+[
Send to Back        Ctrl+Shift+[
```

Rotate submenu:

```text
Rotate Right 90°
Rotate Left 90°
Rotate 180°

Free Rotate
Custom Rotation...
```

Flip submenu:

```text
Flip Horizontal
Flip Vertical
```

## Multiple objects

Right-click when multiple objects are selected.

```text
(edit group)

Group               Ctrl+G

Order >

Align >
Distribute >

Rotate >
Flip >

Format Objects...
```

Align submenu:

```text
Align Left
Align Center
Align Right

Align Top
Align Middle
Align Bottom

Relative to Selection (selected)
Relative to Page
Relative to Margin
```

Distribute submenu:

```text
Distribute Horizontally
Distribute Vertically
```

## Group

Right-click on a group.

```text
(edit group)

Ungroup             Ctrl+Shift+G
Edit Group          (enters group edit mode)

Order >

Rotate >
Flip >

Format Group...
```

## Text frame

Object mode (frame selected):

```text
(edit group)

Edit Text           (enters text edit mode)

Order >

Create Text Box Link    (starts linking)
Break Text Box Links    (shown only if linked)

Format Text Box...

Hyperlink...        Ctrl+K
Lock
```

Edit mode (cursor in text):

```text
Cut                 Ctrl+X
Copy                Ctrl+C
Paste               Ctrl+V
Paste Plain Text    Ctrl+Shift+V

Font...
Paragraph...

Bullets and Numbering...

Hyperlink...        Ctrl+K
Remove Hyperlink    (shown only on a hyperlink)
```

On a misspelled word:

```text
(spelling suggestions, e.g. quick, quack, quirk)

Ignore
Ignore All
Add to Dictionary

AutoCorrect >

Cut                 Ctrl+X
Copy                Ctrl+C
Paste               Ctrl+V
```

## Image frame

```text
(edit group)

Change Picture...
Reset Picture
Remove Picture

Order >

Fit >
Crop

Format Picture...

Save as Picture...
```

Fit submenu:

```text
Fill Frame
Fit to Frame
Stretch to Fill

Original Size
```

## Table

On a table cell:

```text
Cut                 Ctrl+X
Copy                Ctrl+C
Paste               Ctrl+V

Insert >
Delete >

Select >

Merge Cells
Split Cells...

Cell Properties...
Table Properties...
```

Insert submenu: `Row Above`, `Row Below`, `Column Left`, `Column Right`.

Delete submenu: `Row`, `Column`, `Table`.

Select submenu: `Cell`, `Row`, `Column`, `Table`.

## Line

```text
(edit group)

Order >

Add Arrow >

Format Line...

Lock
```

Add Arrow submenu:

```text
No Arrows

End Arrow
Start Arrow
Both Arrows

More Arrows...
```

## Page panel

Right-click on a page thumbnail.

```text
Insert Page Before
Insert Page After

Duplicate Page
Rename Page...

Delete Page

Apply Master Page >
```

## Ruler

Right-click on a ruler.

```text
Points (checked)
Inches
Millimeters
Centimeters
Picas

Add Guide
Clear All Guides

Hide Rulers
```

## Guide

Right-click on a guide.

```text
Delete Guide

Position...         (sets exact position)

Lock Guide
Guide Properties...
```

## Locked object

Right-click on a locked object.

```text
Unlock

View Properties...  (read-only view)
```

## Styling

| Property | Value |
|----------|-------|
| Background | `#FFFFFF` |
| Border | 1px `#D9D9D9` |
| Shadow | 2px 4px 8px rgba(0,0,0,0.15) |
| Border radius | 4px |
| Item height | 24px |
| Padding | 4px 0 |
| Min width | 180px |
| Max width | 300px |

Item states:

| State | Background | Text |
|-------|------------|------|
| Normal | Transparent | `#000000` |
| Hover | `#E5F3FF` | `#000000` |
| Pressed | `#CCE8FF` | `#000000` |
| Disabled | Transparent | `#A6A6A6` |

Separator: 1px high, `#E5E5E5`, 4px 0 margin.

## Conditional items

Enable conditions:

| Item | Enabled when |
|------|--------------|
| Cut, Copy | Has selection |
| Paste | Clipboard has content |
| Duplicate | Has selection |
| Group | 2+ objects selected |
| Ungroup | Group selected |
| Align | Selection exists |
| Distribute | 3+ objects selected |
| Merge Cells | Multiple cells selected |

Show conditions:

| Item | Shown when |
|------|------------|
| Edit Group | Group selected |
| Break Text Links | Frame has links |
| Remove Hyperlink | On hyperlink |
| Spelling suggestions | On misspelled word |
