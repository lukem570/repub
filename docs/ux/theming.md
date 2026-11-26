# Theming

Visual theming of Repub: colors, typography, spacing, radii, shadows, icons, component styles, and accessibility. The theme feels native on each platform while staying consistent.

## Light theme (default)

### Backgrounds

```css
--bg-window:        #F5F5F5;    /* Window */
--bg-panel:         #FFFFFF;    /* Panel */
--bg-toolbar:       #FAFAFA;    /* Toolbar */
--bg-menu:          #FFFFFF;    /* Menu */
--bg-dialog:        #FFFFFF;    /* Dialog */
--bg-input:         #FFFFFF;    /* Input field */

--bg-workspace:     #505050;    /* Workspace/artboard */
--bg-page:          #FFFFFF;    /* Document page */
--bg-pasteboard:    #E5E5E5;    /* Area outside page */

--bg-hover:         #E5F3FF;    /* Hover */
--bg-active:        #CCE8FF;    /* Active/pressed */
--bg-selected:      #E5F3FF;    /* Selected item */
```

### Borders

```css
--border-light:     #E5E5E5;    /* Subtle */
--border-medium:    #D9D9D9;    /* Standard */
--border-dark:      #A6A6A6;    /* Strong */
--border-focus:     #3B82F6;    /* Focus ring */
```

### Text

```css
--text-primary:     #000000;
--text-secondary:   #666666;
--text-tertiary:    #999999;    /* Placeholder */
--text-disabled:    #A6A6A6;
--text-inverse:     #FFFFFF;    /* On dark backgrounds */
```

### Accents

```css
--accent-primary:   #3B82F6;    /* Selection, links, focus */
--accent-hover:     #2563EB;
--accent-active:    #1D4ED8;

--success:          #22C55E;    /* Green */
--warning:          #F59E0B;    /* Amber */
--error:            #EF4444;    /* Red */
--info:             #3B82F6;    /* Blue */
```

### Selection

```css
--selection-handle-fill:    #FFFFFF;
--selection-handle-border:  #000000;
--selection-outline:        #3B82F6;
--rotation-handle:          #22C55E;
--marquee-fill:             rgba(59, 130, 246, 0.1);
--marquee-border:           #3B82F6;
```

## Dark theme

```css
--bg-window:        #1E1E1E;
--bg-panel:         #252526;
--bg-toolbar:       #2D2D2D;
--bg-menu:          #2D2D2D;
--bg-dialog:        #2D2D2D;
--bg-input:         #3C3C3C;

--bg-workspace:     #1E1E1E;
--bg-page:          #FFFFFF;    /* Page stays white */
--bg-pasteboard:    #2D2D2D;

--bg-hover:         #3C3C3C;
--bg-active:        #4C4C4C;
--bg-selected:      #094771;

--border-light:     #3C3C3C;
--border-medium:    #4C4C4C;
--border-dark:      #666666;
--border-focus:     #3B82F6;

--text-primary:     #E5E5E5;
--text-secondary:   #A0A0A0;
--text-tertiary:    #808080;
--text-disabled:    #606060;
--text-inverse:     #000000;
```

Define all variables under `:root` and override them under `[data-theme="dark"]`.

## Typography

### Font stacks

```css
--font-ui:
    -apple-system,          /* macOS */
    BlinkMacSystemFont,     /* macOS Chrome */
    "Segoe UI",             /* Windows */
    "Roboto",               /* Android/Chrome OS */
    "Oxygen",               /* KDE */
    "Ubuntu",               /* Ubuntu */
    "Cantarell",            /* GNOME */
    "Fira Sans",            /* Firefox OS */
    "Droid Sans",           /* Old Android */
    "Helvetica Neue",       /* Fallback */
    sans-serif;

--font-mono:
    "SF Mono",              /* macOS */
    "Consolas",             /* Windows */
    "Liberation Mono",      /* Linux */
    "Menlo",                /* macOS fallback */
    monospace;
```

### Sizes, weights, line heights

```css
/* Base size: 12px */
--text-xs:      10px;       /* Labels, captions */
--text-sm:      11px;       /* Status bar */
--text-base:    12px;       /* Body text, menus */
--text-md:      13px;       /* Panel headers */
--text-lg:      14px;       /* Dialog titles */
--text-xl:      16px;       /* Section headers */
--text-2xl:     18px;       /* Dialog headings */
--text-3xl:     20px;       /* Large headings */

--weight-normal:    400;
--weight-medium:    500;
--weight-semibold:  600;
--weight-bold:      700;

--leading-tight:    1.25;   /* Compact UI */
--leading-normal:   1.5;    /* Body text */
--leading-relaxed:  1.75;   /* Readable paragraphs */
```

## Spacing

Spacing uses a 4px grid.

```css
--space-0:      0px;
--space-1:      4px;
--space-2:      8px;
--space-3:      12px;
--space-4:      16px;
--space-5:      20px;
--space-6:      24px;
--space-8:      32px;
--space-10:     40px;
--space-12:     48px;
--space-16:     64px;
```

| Component | Padding | Gap |
|-----------|---------|-----|
| Button | 8px 16px | 8px between icons/text |
| Menu item | 0 24px 0 12px | none |
| Panel section | 12px | 8px between items |
| Dialog | 24px | 16px between sections |
| Toolbar | 4px 8px | 4px between items |
| Input field | 4px 8px | none |

## Border radius

```css
--radius-none:  0px;
--radius-sm:    2px;        /* Small elements, inputs */
--radius-md:    4px;        /* Buttons, cards */
--radius-lg:    8px;        /* Dialogs, panels */
--radius-xl:    12px;       /* Large cards */
--radius-full:  9999px;     /* Pills, circles */
```

## Shadows

```css
--shadow-sm:    0 1px 2px rgba(0, 0, 0, 0.05);
--shadow-md:    0 2px 4px rgba(0, 0, 0, 0.10);
--shadow-lg:    0 4px 8px rgba(0, 0, 0, 0.15);
--shadow-xl:    0 8px 16px rgba(0, 0, 0, 0.20);

--shadow-menu:      0 2px 8px rgba(0, 0, 0, 0.15);
--shadow-dialog:    0 4px 16px rgba(0, 0, 0, 0.20);
--shadow-tooltip:   0 2px 4px rgba(0, 0, 0, 0.15);
--shadow-page:      0 2px 8px rgba(0, 0, 0, 0.15);
```

## Icons

```css
--icon-xs:      12px;       /* Inline indicators */
--icon-sm:      16px;       /* Toolbar, menus */
--icon-md:      20px;       /* Panel icons */
--icon-lg:      24px;       /* Large toolbar */
--icon-xl:      32px;       /* Dialog icons */
--icon-2xl:     48px;       /* Empty states */
```

Icon style:

- Outlined, not filled.
- Stroke: 1.5px for 16px icons, 2px for 24px and larger.
- Corners: rounded, 2px radius.
- Color: inherited from text color.

Required icons:

- Toolbar (16×16 or 24×24): Select (arrow), Text (T cursor), Rectangle, Ellipse, Line, Polygon, Star, Image, Pan (hand), Zoom (magnifier).
- Menu (16×16): New document, Open, Save, Export, Print, Undo/Redo, Cut/Copy/Paste, Zoom in/out, Bold/Italic/Underline, Align left/center/right.
- Panel (16×16): Expand/collapse arrow, Close (×), Settings gear, Plus, Minus, Eye (visibility), Lock, Layers.

## Components

### Primary button

```css
background: var(--accent-primary);
color: white;
padding: 8px 16px;
border-radius: 4px;
font-weight: 500;

:hover  { background: var(--accent-hover); }
:active { background: var(--accent-active); }
```

### Secondary button

```css
background: transparent;
color: var(--text-primary);
border: 1px solid var(--border-medium);
padding: 8px 16px;
border-radius: 4px;

:hover { background: var(--bg-hover); }
```

### Icon button

```css
width: 28px;
height: 28px;
padding: 4px;
border-radius: 4px;

:hover  { background: var(--bg-hover); }
:active { background: var(--bg-active); }
```

### Input field

```css
height: 28px;
padding: 4px 8px;
border: 1px solid var(--border-medium);
border-radius: 2px;
background: var(--bg-input);

:focus {
    border-color: var(--accent-primary);
    outline: none;
    box-shadow: 0 0 0 2px rgba(59, 130, 246, 0.2);
}
```

### Dropdown

An input field with an arrow icon on the right.

```css
height: 28px;
padding: 4px 28px 4px 8px;
border: 1px solid var(--border-medium);
border-radius: 2px;
background: var(--bg-input);
```

### Checkbox

```css
width: 16px;
height: 16px;
border: 1px solid var(--border-medium);
border-radius: 2px;

:checked {
    background: var(--accent-primary);
    border-color: var(--accent-primary);
    /* Checkmark icon */
}
```

### Slider

```css
height: 4px;
background: var(--border-medium);
border-radius: 2px;

.thumb {
    width: 14px;
    height: 14px;
    background: var(--accent-primary);
    border-radius: 50%;
}
```

## Transitions

```css
--duration-fast:    100ms;
--duration-normal:  200ms;
--duration-slow:    300ms;

--ease-out:         cubic-bezier(0, 0, 0.2, 1);
--ease-in:          cubic-bezier(0.4, 0, 1, 1);
--ease-in-out:      cubic-bezier(0.4, 0, 0.2, 1);
```

## Z-index scale

```css
--z-base:       0;
--z-dropdown:   1000;
--z-sticky:     1100;
--z-fixed:      1200;
--z-overlay:    1300;
--z-modal:      1400;
--z-popover:    1500;
--z-tooltip:    1600;
```

## Platform adaptations

- Windows: Segoe UI font, squared corners (2px radius), higher contrast borders, native title bar integration.
- macOS: SF Pro font, rounded corners (4-6px), subtle shadows, optional vibrancy effects.
- Linux: system font, matches the GTK/Qt theme when possible, respects the dark mode preference.

## Accessibility

- Color contrast: text on background at least 4.5:1; large text at least 3:1; interactive elements at least 3:1.
- Focus: visible 2px focus ring, high contrast mode support, no focus trapping.
- Motion: respect `prefers-reduced-motion` and provide static alternatives.
