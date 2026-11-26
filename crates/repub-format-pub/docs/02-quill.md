# 02 — The Quill Text Streams

Publisher's text engine is "Quill". All text lives in
`Quill/QuillSub/CONTENTS` as a list of named *chunks*. A Contents text shape
carries a `TextId` (block id `SHAPE_TEXT_ID`, doc 01); that id indexes the
`SYID` table to find which story belongs to the shape.

## Stream header and chunk list

```c
typedef struct {
    u8  header[0x18];    // 0x00: fixed header; preserve verbatim (breakdown in doc 04)
    // chunk list metadata begins at 0x18:
    u16 _reserved;       // 0x18
    u16 numChunks;       // 0x1A: entries in this list
    u32 nextListOffset;  // 0x1C: offset of the next chunk list, or 0xFFFFFFFF if none
    // ChunkRef chunks[numChunks]; // starting at 0x20
} QuillHeader;
```

Chunk lists can chain via `nextListOffset`; follow the chain until `0xFFFFFFFF`.

## Chunk reference (24 bytes)

```c
typedef struct {            // offsets relative to the entry start
    u16 marker;             // +0x00: always 0x0018 (the entry size)
    u8  name[4];            // +0x02: 4 ASCII chars, e.g. "TEXT", "FDPC"
    u16 id;                 // +0x06: chunk id (distinguishes same-named chunks)
    u8  _flags[4];          // +0x08: constant 0x01000000 observed (preserve; see doc 04)
    u8  name2[4];           // +0x0C: secondary 4-char name/tag
    u32 dataOffset;         // +0x10: absolute offset of the chunk data
    u32 dataLength;         // +0x14: chunk data length in bytes
} ChunkRef;                 // = 0x18 bytes
```

## Chunk names

```c
// 4-char chunk names (space-padded to 4 where shorter):
//   "TEXT"  raw story text, UTF-16LE
//   "STRS"  string-length array: splits TEXT into per-shape strings
//   "SYID"  string ids: maps each story to a shape TextId
//   "FDPC"  character-formatting spans
//   "FDPP"  paragraph-formatting spans
//   "STSH"  style sheet (named default styles; style id 1 is the base)
//   "FONT"  font table (faces referenced by FDPC)
//   "PL  "  color palette
//   "TCD "  table cell definitions
```

## STRS — string lengths

```c
typedef struct {
    u32 count;        // +0x00: number of strings
    u32 arrayOffset;  // +0x04: offset (within this chunk) of the length array
    // ... at arrayOffset:
    // u32 lengths[count]; // each = string length in UTF-16 code units
} Strs;
```

## SYID — story-to-shape ids

```c
typedef struct {
    u32 kind;        // +0x00: record-kind word (preserve)
    u32 count;       // +0x04: number of ids
    u32 ids[count];  // +0x08: TextIds, in story order
} Syid;
```

## FDPC / FDPP — formatting spans

Both are span tables: a list applied in sequence over the story's characters
(`FDPC`) or paragraphs (`FDPP`).

```c
typedef struct {
    u16 count;          // number of spans (read as u16 at the chunk start)
    // followed by `count` records, each:
    //   u32 runLength;  // characters/paragraphs this span covers
    //   <style record>  // character style (FDPC) or paragraph style (FDPP)
} Fdp;
```

A reader maps the common style fields:

- **FDPC**: font reference (into `FONT`), size, bold, italic, underline, color
  (a `ColorRef`, doc 00).
- **FDPP**: alignment, line spacing, space-before/after, left/right/first
  indents.

The remaining bytes of each style record (kerning, scaling, language, tabs,
list/numbering, borders, shading) are read past but not yet mapped; they are
enumerated in [04-unresolved.md](./04-unresolved.md).

## FONT, STSH, TCD

- **FONT**: array of face entries (name as a length-prefixed string + metrics).
- **STSH**: the base style (id 1) and named style definitions; the override
  chain is partially mapped (doc 04).
- **TCD **: table cell definitions (per-cell size and attributes), partially
  mapped (doc 04).
