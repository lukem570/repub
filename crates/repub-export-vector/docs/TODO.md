# repub-export-vector — TODO

Both exporters are Scene-driven: a page is composed into a
`repub_scene::Scene` and its primitives are translated. This replaces the old
`repub-formats` PDF exporter, which walked the document tree and only drew
placeholder outlines — the new PDF export produces a valid, viewable file with
real fills, strokes and selectable text (confirmed with `pdfinfo`/`pdftotext`).

## PDF (`pdf` module)

Working today:

- Fills (nonzero / even-odd) and strokes with per-primitive color, width and
  dash pattern.
- Glyph runs rendered as selectable text in the standard base-14 Helvetica
  family (regular / bold / oblique / bold-oblique chosen by weight and italic).
- Per-page y-flip so the top-left scene origin maps correctly; text is
  individually positioned by a text matrix.
- Multi-page output, one PDF page per document page; document info (title,
  author, subject, keywords).

Simplifications / not yet done:

- **Font embedding.** The document's actual font is not embedded; all text uses
  Helvetica metrics via a base-14 font. Characters outside Latin-1 are replaced
  with `?` by the WinAnsi encoder. Embedding the real TrueType/OpenType face
  (via `FontProvider` bytes, a CID/Type0 font and a `ToUnicode` CMap) is the
  biggest remaining item; it would also fix glyph advances and non-Latin text.
- **Gradients.** Linear and radial gradients are approximated by a single
  representative color (`Paint::primary_color`). Real output needs PDF axial
  (type 2) / radial (type 3) shadings via `function_shading`/`shading_pattern`.
- **Images.** `Primitive::Image` draws a gray placeholder rectangle. Real
  embedding needs image XObjects: JPEG sources via a `DCTDecode`-filtered
  stream, and PNG/other via decoded RGB(A) samples (with an `SMask` for alpha).
  pdf-writer's `image_xobject` does not expose the stream filter directly, so
  JPEG passthrough needs the lower-level `stream().filter(Filter::DctDecode)`
  path plus manual XObject dictionary keys.
- **Clipping.** Primitive `clip` rectangles are ignored. Add `W n` clip paths
  around clipped primitives.
- **Effects.** `SceneEffect`s (shadow, glow, reflection, bevel, soft edges) are
  not realized in PDF. They would need rasterized soft masks or approximations.
- **Text advances / kerning.** Each glyph is placed by absolute x from the
  scene, so runs are correct positionally but not kerned as a single `TJ` array.
- **Hyperlinks, crop marks, bleed, PDF/A.** The old `PdfOptions` had
  `include_links`, `crop_marks`, `bleed` and `pdf_a`; these are dropped for now.
  Re-add link annotations, crop-mark drawing, a bleed box and PDF/A output
  intents.
- **Opacity / blend.** Primitive opacity is not applied to fills/strokes (only
  images in SVG). Add `ExtGState` alpha for PDF.

## SVG (`svg` module)

Working today:

- `<path>` for fills (with `fill-rule`) and strokes (width, dash, opacity).
- `<text>` for glyph runs with per-glyph absolute x positions, family, size,
  weight and style.
- `<image>` for file/url/embedded (data-URI) picture sources; a placeholder
  `<rect>` for unresolved sources.
- `<linearGradient>` / `<radialGradient>` definitions in `<defs>` for gradient
  paints, in user-space coordinates.

Simplifications / not yet done:

- **Pattern fills** collapse to their foreground color (no SVG `<pattern>`).
- **Clipping** rectangles are ignored (no `<clipPath>`).
- **Effects** (`SceneEffect`) are not emitted as SVG filters.
- **Transforms on text/images** are emitted as `matrix(...)`, but glyph x-lists
  are in pre-transform coordinates; complex per-glyph transforms (e.g. rotated
  runs) may need per-glyph `<tspan>` or path rendering.
