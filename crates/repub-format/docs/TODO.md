# repub-format — TODO

A thin facade dispatching load/save by file extension to the Layer-5 backends
(`repub-format-native`, `repub-export-vector`, `repub-export-raster`).

## Dispatch table (current)

| Extension | load | save |
|-----------|------|------|
| `.repub` | native JSON | native JSON (pretty) |
| `.repubz` | native zip | native zip |
| `.json` | native JSON | native JSON (compact) |
| `.pdf` | — | vector PDF |
| `.svg` | — | vector SVG (page 0) |
| `.png`/`.jpg`/`.jpeg`/`.tiff`/`.tif`/`.bmp`/`.webp` | — | raster (page 0) |
| `.html`/`.htm` | — | HTML (whole document) |
| `.pub` | `PubNotWired` | `PubNotWired` |

## Wiring `.pub` (the documented seam)

`repub-format-pub` is deliberately **not** a dependency yet, so this crate
builds while that crate is developed in parallel. To wire it in:

1. Add `repub-format-pub = { workspace = true }` to `Cargo.toml`.
2. In `load`, replace the `"pub" => Err(FormatError::PubNotWired)` arm with a
   call into the importer (see the `TODO(pub)` comment there).
3. In `save`, do the same for the exporter.
4. Optionally add a `FormatError::Pub(#[from] repub_format_pub::PubError)`
   variant and drop `PubNotWired`.

## Other gaps

- **Page selection for single-page exports.** `.svg` and raster exports always
  use page 0. Add an API that takes a page index (and perhaps a page range for
  multi-file output), or a naming template for per-page files.
- **Options passthrough.** `save` uses each backend's default options
  (`PdfOptions`, `SvgOptions`, `RasterExportOptions`, `HtmlOptions`,
  `SaveOptions`). There is no way to pass custom options (title/author, DPI,
  quality, compression) through the single `save` entry point. Add a
  `SaveConfig`/`save_with` variant, or per-format `save_*` helpers.
- **Loading non-native formats.** Only native formats load. PDF/SVG/HTML are
  write-only; a round-trip importer for them is out of scope but could be noted
  for users.
- **Extension vs. content detection.** Dispatch is purely by extension. There
  is no sniffing of file contents (e.g. a `.json` that is actually a `.repubz`
  zip), and `.repubz` compression is keyed off the extension by the native
  backend.
- **Bytes-level facade.** Only path-based `load`/`save` exist. In-memory
  `save_bytes(doc, format, fonts)` / `load_bytes(bytes, format)` helpers would
  round out the API; the backends already expose byte variants.
