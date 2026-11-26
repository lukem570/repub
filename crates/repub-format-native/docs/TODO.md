# repub-format-native — TODO

The DTO layer currently achieves a **lossless round trip** for the whole
`repub-model` document graph: a `Document` saved and loaded compares `==` to the
original (object/page/layer/master-page IDs are preserved via `Id::from_raw`).

Every model field reachable from `Document` is serialized today:

- Document: metadata, settings, default page size/orientation, pages, master
  pages.
- Page / MasterPage: id, name, size, margins, master-page reference, layers,
  guides, background fill.
- Layer: id, name, visible/locked/printable flags, shapes.
- Shape: common placement (id, bounds, rotation, transform, visible, locked,
  name, z-index, text wrap, flips), fill, line, effects, opacity, kind.
- ShapeKind payloads: AutoShape (geometry + Office preset + adjust values),
  TextFrame, Table (cells, borders, defaults), Picture, TextEffect (WordArt /
  text-on-path, 3D rotation, extrusion, path settings), Line (endings,
  connector type, control points), Group (recursive children + bounds).
- Styles: Fill (all variants incl. gradients, pattern, texture), Stroke,
  Effects (every sub-effect), FontSpec, CharacterStyle, ParagraphStyle
  (incl. list style, line spacing).
- Text: StyledText → Paragraph → InlineElement (Text/Field/Image/Link/
  LineBreak/Tab) with full character styling.
- Foundation: Point, Size, Rect, Transform2D, Color, gradients, pattern types.

## Not yet serialized / known gaps

- **Named style sheets.** `repub_model::styles::StyleSheet`,
  `NamedCharacterStyle`, and `NamedParagraphStyle` are not part of the
  `Document` graph today (the model does not store a stylesheet on `Document`),
  so they are not persisted. If a stylesheet is added to `Document` later, add a
  `StyleSheetDto` and wire it in. The `StyleId` type would need a raw-value DTO
  like the other IDs.
- **CMYK colors.** `repub_color::CmykColor` is defined in the color layer but
  the model's `Color` is RGBA only and no model field uses CMYK, so no CMYK DTO
  exists. Add `CmykColorDto` if a CMYK-carrying field appears.
- **ID remapping on import.** IDs are preserved verbatim via `Id::from_raw`.
  There is no option to renumber on load (e.g. to merge two documents). A
  future `LoadOptions { renumber_ids: bool }` could remap and fix up
  `linked_frame`/`master_page` references.
- **Forward/older versions.** Only the `2.x` version series is accepted. There
  is no migration path from the legacy `1.x` (`repub-core`) native format;
  loading one returns `VersionMismatch`. A converter could be added.
- **Compression tuning.** The `.repubz` container uses DEFLATE at the zip
  crate's default level; the `compression_level` knob from the old format was
  dropped. Re-expose it if needed.
- **Schema/`serde` representation stability.** Enum DTOs use serde's default
  (externally tagged) representation. This is fine for a private format but is
  not a documented, stable schema; if interop is ever needed, pin explicit tags
  and field names.
