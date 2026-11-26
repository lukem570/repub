# 00 — Microsoft Publisher `.pub` File

A Microsoft Publisher file contains the data for a publication — pages, shapes,
text, tables, and images — stored across several byte streams inside an OLE
Compound File. This guide specifies that data, in struct format, for anyone
implementing a reader or writer for the format (Publisher 2002–2016).

It is grounded in the parser in this crate (`repub-format-pub`), cross-checked
against libmspub and Ghidra analysis of `MSPUB.EXE`. The aim is that **every
byte has a named meaning**. Bytes read but not yet interpreted are listed in
[`04-unresolved.md`](./04-unresolved.md) with exact location and size; the format
is fully specified when that list is empty.

The remaining documents specify each stream in detail; read them in order:

| Doc | Topic |
|---|---|
| [`01-contents.md`](./01-contents.md) | The Contents stream: block encoding and every chunk |
| [`02-quill.md`](./02-quill.md) | The Quill text streams: text, string tables, formatting |
| [`03-escher.md`](./03-escher.md) | The Escher drawing streams: records, properties, images |
| [`04-unresolved.md`](./04-unresolved.md) | The "no unknown blocks" ledger |

## Data Types

All structures in this guide are **packed** (no padding, `#pragma pack(1)`) and
all multi-byte integers are **little-endian**. `//` comments give a field's
meaning; where a value is stated as a constant (e.g. `marker = 0x0018`) it is
fixed for the versions this guide covers. Fields marked *reserved* are preserved
verbatim on write and must not be interpreted. Variable-length and repeated
regions are described in prose right after each struct, since C cannot express
them inline.

| Data type | Description |
|---|---|
| u8 | 8-bit unsigned integer. |
| u16 | 16-bit unsigned integer. |
| u32 | 32-bit unsigned integer. |
| i32 | 32-bit signed integer. |
| EMU | u32 or i32 in English Metric Units (see Units below). |
| Fixed16_16 | 32-bit signed 16.16 fixed-point; used for angles (`degrees = raw / 65536`). |
| ColorRef | 4-byte color reference (see Color References below). |
| SeqNum | u32 identifying a Contents chunk; matches an Escher shape id (`spid`). |
| TextId | u32 on a Contents text shape; selects a Quill story via the `SYID` table. |
| BlipIndex | 1-based index into the Escher BLIP store; selects image bytes. |

## Units

Positions and sizes are in **EMUs** (English Metric Units):

```
914400 EMU = 1 inch
 12700 EMU = 1 point (1/72 inch)
360000 EMU = 1 cm
```

A reader converts to points with `points = emu / 12700.0`. Escher preset shapes
use a separate fixed **21600 × 21600** design space (see [03](./03-escher.md)),
scaled into the shape's EMU bounds. Angles are 16.16 fixed-point degrees
(`degrees = raw / 65536`). Page coordinates are relative to the page's top-left
origin, y increasing downward.

## Color References

A `ColorRef` is a 4-byte value: a `kind` byte that selects the encoding, and
three payload bytes whose meaning depends on `kind`.

```c
typedef struct {
    u8 payload[3]; // interpreted per `kind` (see tables below)
    u8 kind;       // encoding selector
} ColorRef;
```

`kind` values:

| kind | Encoding | `payload[0..3]` |
|---|---|---|
| 0x00 | Direct RGB | blue, green, red |
| 0x08 | Scheme color | `payload[0]` = scheme slot index; `payload[1..3]` unused |
| 0x10 | System color | `payload[0]` = system palette index; `payload[1..3]` unused |
| 0xFE | No color | ignored (used for "no fill" / "no line") |
| 0xFF | Sentinel | ignored |

Scheme slot indices (`kind` 0x08):

| Index | Slot |
|---|---|
| 0 | Background |
| 1 | Text |
| 2–6 | Accent 1–5 |
| 7 | Hyperlink |
| 8 | Followed hyperlink |

A reader preserves scheme and system references (rather than resolving them) so
the document's color scheme stays editable; they are resolved to concrete RGBA
later, at compose time. The document palette (Publisher 2000's built-in 10-entry
base: black, white, red, green, blue, yellow, cyan, magenta, gray, light-gray)
is the fallback when a file omits its own; the palette itself is the `PL  `
Quill chunk or the `PALETTE` Contents chunk.

**Legacy line-width byte (Publisher 2000 shapes only).** Publisher 2002+ stores
line width as an EMU `u32` in the Escher line properties. The older per-shape
byte encoding is:

```
0x81        -> no line
> 0x81      -> quarter_points = ((v - 0x81) / 3) * 4 + ((v - 0x81) % 3) + 1
< 0x81      -> quarter_points = v * 4
```

## Organization of a Publisher File

A `.pub` file is a Microsoft Compound File (MS-CFB): a FAT-like container of
named byte *streams* in one file. The CFB layer itself (header, FAT, directory,
mini-stream) is specified in [MS-CFB] and handled by a CFB library (this crate
uses the `cfb` crate); this guide specifies the *contents* of the streams.

### Streams

| Stream path | Required | Holds | Spec |
|---|---|---|---|
| `Contents` | yes | Document structure: pages, shapes, tables, groups, fonts, palette | [01](./01-contents.md) |
| `Quill/QuillSub/CONTENTS` | with text | Text stories and formatting | [02](./02-quill.md) |
| `Escher/EscherStm` | yes | Geometry, fills, strokes, image refs | [03](./03-escher.md) |
| `Escher/EscherDelayStm` | with images | Raw image (BLIP) bytes | [03](./03-escher.md) |
| `Internal` | yes | Small internal flag block (opaque; see [04](./04-unresolved.md)) | — |
| `Envelope` | yes | Envelope/print settings (usually a fixed empty block) | — |
| `\x01CompObj` | yes | Standard OLE component-object record (app identity) | — |
| `\x05SummaryInformation` | optional | Standard OLE property set: title, author, … | — |
| `\x05DocumentSummaryInformation` | optional | Standard OLE property set: company, category, … | — |

The `\x01` / `\x05` prefixes are the standard OLE control-byte prefixes for
those well-known streams.

### The Contents stream header

The **`Contents`** stream is the root of the document structure; everything else
is reached from it. It begins with a fixed header whose only interpreted field is
the trailer offset:

```c
typedef struct {
    u8  signature[0x1A]; // 0x00: fixed Publisher signature/version (breakdown in doc 04)
    u32 trailerOffset;   // 0x1A: absolute offset of the Trailer within this stream
} ContentsHeader;        // chunk block data follows, reached only via the Trailer
```

### Trailer and chunk directory

At `trailerOffset` is the Trailer: a length followed by three sub-blocks, exactly
one of which (block type `0x90` or `0x8A`) is the **chunk directory**.

```c
typedef struct {
    u32 length;          // total Trailer length in bytes
    // three sub-blocks follow (encoded as Blocks, doc 01);
    // the one with type 0x90 or 0x8A is the chunk directory.
} Trailer;
```

The chunk directory lists every top-level piece of the document — the document
record, each page, each shape, each table, groups, fonts, the palette — as a
sequence of *chunk records*. A record gives the chunk's type, where its data
lives, and its parent, forming the document tree. A record's position in the
directory is its `SeqNum`.

```c
typedef struct {
    u32 type;            // ChunkType: DOCUMENT, PAGE, SHAPE, TABLE, GROUP, ... (doc 01)
    u32 offset;          // absolute offset of this chunk's block data in the stream
    u32 parentSeqNum;    // SeqNum of the parent chunk (0 = top level)
} ChunkRecord;           // one per chunk; index in the directory = this chunk's SeqNum
```

> The `ContentsHeader`, `Trailer`, and `ChunkRecord` are all built from the
> universal *block* encoding specified at the top of [doc 01](./01-contents.md);
> the chunk payloads reached through `offset` are specified in the rest of that
> document.

### How the streams join

Having read the chunk directory, a reader walks the tree and resolves each
chunk's cross-stream references:

```
Contents chunk (SeqNum, TextId) ──TextId──▶ Quill story + formatting   (doc 02)
                │
                └────SeqNum────▶ Escher shape (geometry, fill, stroke)  (doc 03)
                                        │
                                        └──BlipIndex──▶ EscherDelay image bytes (doc 03)
```

## Implementation order

Read the streams in dependency order:

1. **Quill** — build the text stories and formatting tables, keyed by `TextId`.
2. **Contents** — read the header → trailer → chunk directory; build the
   page/shape/table/group tree; resolve `TextId`s.
3. **EscherDelay** — index the embedded image bytes.
4. **Escher** — resolve each shape's geometry/fill/stroke/image, joining to
   Contents by `SeqNum` and to EscherDelay by `BlipIndex`.

Writing reverses this: lay out the Quill stories, assign `TextId`s and
`SeqNum`s, write Escher geometry, then write the Contents tree and the OLE
directory. Export completeness is tracked in [`TODO.md`](./TODO.md).

## Version scope

This guide covers Publisher 2002–2016 (the "2k2+" Contents layout). Publisher
97/98/2000 used an older, simpler chunk format that is **out of scope** (noted in
[04](./04-unresolved.md) as excluded, not unknown).
