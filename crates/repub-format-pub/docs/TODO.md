# `.pub` support — roadmap to 100%

A checklist toward complete, lossless Microsoft Publisher support. "100%" means
two things: **strict import reads every item in any `.pub` file with no
`UnreadItems` error**, and **export → re-import round-trips a document
losslessly**.

Import is strict by default (see `lib.rs`): it fails with
`FormatError::UnreadItems` on anything it cannot fully read.

## Status

Import is in good shape: **every well-formed file in `reverse/test-files/`
(29 of 32; the other 3 are deliberately-corrupt fuzzer inputs that fail at the
OLE layer) imports cleanly under strict mode** — locked by the
`every_well_formed_fixture_imports_strictly` test. The reader distinguishes
*intentional* skips (ALTSHAPE alternates and confirmed-invisible shapes, which
Publisher itself would not render — not data loss) from *genuine* failures (a
page referencing a shape that is absent from the file — data loss, which does
fail strict import).

Export now round-trips the essentials: every shape's kind, position, size,
fill, and stroke survive an export → re-import, and a text frame no longer
inherits a neighbouring shape's fill (locked by
`export::tests::roundtrip_preserves_shape_kind_fill_and_stroke`). The remaining
export work is richer shape geometry (custom paths, lines, tables, WordArt,
pictures) and real Quill formatting.

## Import — read every item (strict)

### Correctness
- [x] Fail with `UnreadItems` on any genuine data-loss item
- [x] `import_lenient` best-effort fallback
- [x] Intentional skips (ALTSHAPE, invisible shapes) are not counted as data loss
- [x] Zero data-loss failures on every well-formed file in `reverse/test-files/`
- [ ] Genuinely-absent referenced shapes report the referencing page + seqnum in the error

### Field coverage (deepen what is interpreted; not currently failing imports)
These do not block strict import on the current corpus, but are needed before
the format can be called fully specified (see `04-unresolved.md`).
- [ ] Interpret the `Contents` header signature (ledger U1)
- [ ] Interpret the Quill stream header (ledger U2)
- [ ] Confirm the `ChunkRef` flags field (ledger U3)
- [ ] Interpret the `Internal` stream (ledger U4)
- [ ] Interpret the `Envelope` stream when non-empty (ledger U5)
- [ ] Map the full FDPC character-style record (ledger U8)
- [ ] Map the full FDPP paragraph-style record (ledger U9)
- [ ] Map the STSH style-override chain and `TCD ` cell attributes (ledger U10)
- [ ] Map every FOPT property id used by Publisher (ledger U11)
- [ ] Map any remaining per-chunk block ids (ledger U7)
- [ ] Read OLE `SummaryInformation` / `DocumentSummaryInformation` metadata

## Export — write a Publisher-faithful file

The exporter writes a valid OLE structure, a Contents chunk directory, a Quill
text story, and per-shape Escher records. A re-import now recovers each shape's
kind, bounds, fill, and stroke. The richer items below are still outstanding.

### Contents
- [x] Per-shape type / position / size blocks
- [x] Fill, line per shape (via Escher FOPT; see Escher below)
- [x] Rotation per shape (via Escher FOPT ROTATION)
- [x] Flips per shape (via FSP flip flags)
- [x] Text-frame columns and internal insets (via Escher FOPT)
- [ ] Text-frame vertical alignment, overflow/links
- [x] Table grid (rows/cols, row-col array, cells seqnum) and per-cell text
- [x] Group containers with correct parent sequence numbers
- [x] Document size and page list
- [ ] Master-page references
- [ ] BorderArt, palette, and font chunks

### Quill
- [x] Write character-formatting (FDPC) spans (bold, italic, underline, size)
- [x] Write paragraph boundaries (FDPP) so paragraphs/table cells round-trip
- [x] Write paragraph-formatting (FDPP style content: alignment, spacing)
- [ ] Write the STSH style sheet
- [ ] Write the FONT table and `PL  ` palette
- [ ] Write `TCD ` table-cell definitions
- [ ] Reconstruct the correct chunk-list / PLC structures

### Escher
- [x] AutoShape preset type (`spt`) written per shape (FSP record)
- [x] Shape rotation (FOPT ROTATION 0x0004)
- [x] AutoShape adjust values (ADJUST1, e.g. rounded-rect corner radius)
- [x] Custom path geometry (`pVertices` / `pSegments`)
- [x] Line geometry (spt 20 + bounds) and line-ending arrow heads
- [x] Solid fills and strokes as FOPT properties (fill colour/type, line colour/width)
- [x] Drop shadows as FOPT properties (type, colour, offsets)
- [x] Linear/radial gradient fills as FOPT properties (shade type, colours, angle)
- [ ] Glows, 3D as FOPT properties
- [x] Text-frame `TEXT_ID` join written (so text frames bind to their own record)
- [x] WordArt / text-effect records (spt + GTEXT_UNICODE complex property)
- [ ] Table cell-fill shapes (tables reconstruct from Contents, not Escher)
- [x] Picture BLIP records (PNG/JPEG) in `EscherDelayStm` + blip store / `pib`

### Round-trip
- [x] Shape kind, position, size, fill, and stroke survive export → re-import
- [x] A text frame keeps its own fill when mixed with other shapes
- [x] Groups (incl. nesting), pictures, tables, multi-paragraph text, WordArt
- [ ] `compat::RoundtripTest` reports full fidelity on every fixture (not just page count)
- [ ] Byte-level stability where Publisher's own output is deterministic

## Out of scope (not counted toward 100%)

- [ ] Publisher 97/98/2000 Contents format (ledger X1) — a separate parser.

---

When every box above is checked, delete `docs/04-unresolved.md` (it will be
empty) and update this file to say the format is complete.
