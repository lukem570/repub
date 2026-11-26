# Architecture

Repub is a Rust workspace of small crates arranged in layers. A crate depends only on crates in lower layers. The workspace members in the root `Cargo.toml` are listed in the same order.

## Layers

| Layer | Crates | Responsibility |
|-------|--------|----------------|
| 0 | `repub-geometry`, `repub-color` | Points, rectangles, transforms, unit conversion, color spaces, gradients, patterns |
| 1 | `repub-model`, `repub-escher` | The pure document model; Office legacy (Escher/MS-ODRAW) preset shape geometry |
| 2 | `repub-fonts`, `repub-text`, `repub-layout` | Font loading and metrics; text runs, Unicode segmentation, line breaking; text and object layout |
| 3 | `repub-render`, `repub-scene`, `repub-hit` | `Renderer` trait and options; scene composition; hit testing |
| 4 | `repub-raster`, `repub-gpu` | CPU rasterizer (tiny-skia); windowless wgpu executor |
| 5 | `repub-format-pub`, `repub-format-native`, `repub-export-vector`, `repub-export-raster`, `repub-format` | File formats and exporters; `repub-format` dispatches by file extension |
| 6 | `repub-edit`, `repub-templates` | Editing engine; template library |
| 7 | `repub-ui`, `repub`, `repub-cli`, `repub-mcp`, `repub-web`, `repub-compare` | Frontends and tools |

## Document model

`repub-model` holds the document with no serialization, rendering, or UI dependencies. A document contains pages. A page contains `Shape` values. Each `Shape` has common placement data, a fill, an optional outline, effects, an opacity, and a `ShapeKind`:

- `AutoShape`: primitive outline or custom path
- `TextFrame`: styled text
- `Table`
- `Picture`
- `TextEffect`: WordArt and text on a path
- `Line`: lines and connectors
- `Group`

The model has no `serde` derives. `repub-format-native` mirrors it with data-transfer structs that do.

## Rendering

Rendering goes through a scene, a flat display list of resolved drawing primitives.

```
Document page → compose (repub-scene) → Scene → Renderer (repub-render)
                                                 ├── repub-raster  (CPU, tiny-skia)
                                                 └── repub-gpu     (wgpu)
```

- `repub-scene` composes a page into a `Scene`.
- `repub-hit` hit-tests a `Scene`.
- `repub-export-vector` writes PDF and SVG from a `Scene`.
- `repub-export-raster` rasterizes a `Scene` to PNG, JPEG, and other raster formats, and writes HTML.

## File formats

`repub-format` loads and saves by extension.

| Extension | Direction | Backend |
|-----------|-----------|---------|
| `.repub`, `.json` | load, save | `repub-format-native` (JSON) |
| `.repubz` | load, save | `repub-format-native` (zipped) |
| `.pub` | load, save | `repub-format-pub` |
| `.pdf`, `.svg` | save | `repub-export-vector` |
| `.png`, `.jpg`, `.jpeg`, `.tiff`, `.bmp`, `.webp` | save | `repub-export-raster` |
| `.html`, `.htm` | save | `repub-export-raster` |

`.pub` export is incomplete. See `crates/repub-format-pub/docs/`.

## Frontends

- `repub-ui`: egui desktop GUI. It carries its own editing state, commands, tools, and hit testing, and renders through wgpu with SDF text.
- `repub`: desktop application binary that runs `repub-ui`.
- `repub-cli`: command-line interface (binary `repub-cli`) with `new`, `info`, `export`, `convert`, `add`, `list`, `validate`, `template`, and `completions`.
- `repub-mcp`: Model Context Protocol server library. It provides tools to create, open, save, and close documents, add and delete pages and objects, list pages and objects, render previews, export, and work with templates.
- `repub-web`: WebAssembly build. Build it with `wasm-pack build --target web --release`.
- `repub-compare`: tools for comparing rendering output and imported files.

`repub-edit` is a framework-free editing engine (commands, undo and redo, selection, text editing, snapping). `repub-ui` does not currently use it.

## UI specification

Behavior of the desktop UI is specified in `docs/ux/`.
