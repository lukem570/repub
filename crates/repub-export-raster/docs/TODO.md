# repub-export-raster — TODO

Two Scene/model-driven exporters: raster images (via `repub-raster`) and HTML.

## Raster (`image` module)

Working today:

- PNG / JPEG / TIFF / WebP / BMP via `repub_raster::ExportFormat`, at any DPI
  and extra scale, with antialiasing and an optional opaque background color.
- Pixel size follows the page size × pixels-per-point (verified: a Letter page
  at 72 DPI is 612×792 px).
- Everything the CPU rasterizer realizes (fills, strokes, gradients, glyph
  runs, embedded/file images, most effects) is included, since it draws the
  composed scene.

Simplifications / not yet done:

- **Format from extension.** `export_png`/`export_image` use
  `options.format`, not the file extension. A convenience that infers the
  format from the path would be nice.
- **Bleed / crop marks.** The old `ImageExportOptions` had `include_bleed` and
  `bleed`; these are not carried over. Add a bleed margin around the rendered
  page if print output is needed.
- **Pattern tiling / `Url` images / some effects** inherit `repub-raster`'s own
  limitations: pattern fills are approximated by their foreground color, `Url`
  image sources are not fetched, and inner glow / bevel are approximated. Those
  are tracked in the `repub-raster` crate, not here.
- **Multi-page sheets.** Only single-page export exists; a contact-sheet or
  per-page batch helper could be added.

## HTML (`html` module)

Working today:

- One self-contained `<html>` document for the whole document, a positioned
  `.page` div per page (sized in points), master-page layers drawn under the
  page's own.
- Text frames as live, selectable text `<div>`s with their text color and
  inset padding; pictures as `<img>` (file/url/data-URI) or a placeholder box;
  auto shapes as styled boxes (solid/gradient/pattern fill approximations, and
  border from the stroke); lines as thin rotated boxes; groups recurse; text
  effects as bold text boxes; rotation and opacity via CSS.

Simplifications / not yet done:

- **Tables** render only as a single bordered bounding box. Real output needs a
  `<table>` (or a grid of positioned cells) using `TableData`'s rows, columns,
  per-cell fills, borders and cell text. This is the biggest HTML gap.
- **Rich text.** Only the plain text of a frame is emitted, with the first
  run's color. Per-run font family/size/weight/style, underline, strikethrough,
  links, fields, inline images, paragraph alignment/spacing, columns and
  vertical alignment are not mapped to CSS yet.
- **Gradients** use a fixed 90°/radial direction with only the first and last
  stops (no intermediate stops, no gradient angle).
- **Effects** (shadow, glow, reflection, bevel, soft edges) are not translated
  to CSS filters/box-shadow.
- **Auto-shape geometry.** Non-rectangular shapes (ellipse, polygon, star,
  custom paths, Office presets) render as plain rectangles. Mapping to
  `border-radius`, `clip-path` or inline SVG is a TODO.
- **Text-on-path / WordArt warps** are flattened to a plain bold text box.
- **Page backgrounds** are not emitted (the page is always white).
