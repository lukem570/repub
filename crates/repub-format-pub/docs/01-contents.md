# 01 — The Contents Stream

`Contents` is the structural stream: the document tree of pages, shapes, tables,
groups, fonts, and the palette. Its header, trailer, and chunk directory are
given in [doc 00](./00-overview.md#organization-of-a-publisher-file). This
document specifies the universal *block* encoding that the whole stream is built
from, then every chunk reached through the directory.

`Contents` is a tree of length-prefixed blocks; *chunks* are groups of blocks
reached through the chunk directory. You never scan the stream linearly — you
follow offsets from the trailer.

## Block encoding

A *block* is the universal unit: a 1-byte **id**, a 1-byte **type**, then data
whose length is determined entirely by `type`.

```c
typedef struct {
    u8 id;    // field name within the containing chunk's namespace (see Chunks below)
    u8 type;  // selects the data length (table below)
    // u8 data[N];  // N depends on `type`
} BlockHeader;
```

The exact `type` → length rule (authoritative — `get_block_data_length`):

```c
// N = data length in bytes for a given block `type`:
//   0x05, 0x08, 0x0A, 0x78                          -> 0   (marker / dummy)
//   0x07, 0x10, 0x12, 0x18, 0x1A                    -> 2   (u16)
//   0x20, 0x22, 0x58, 0x68, 0x70, 0xB8              -> 4   (u32: offset, id, or EMU dim)
//   0x28                                            -> 8
//   0x38                                            -> 16
//   0x48                                            -> 24
//   0x80, 0x82, 0x88, 0x8A, 0x90, 0x98, 0xA0, 0xC0  -> variable (see below)
```

For a **variable** block, the data begins with its own length:

```c
typedef struct {
    u8  id;
    u8  type;    // one of 0x80,0x82,0x88,0x8A,0x90,0x98,0xA0,0xC0
    u32 length;  // number of bytes of `data` INCLUDING these 4 length bytes
    // u8 data[length - 4];
} VarBlock;
```

- `0xC0` is a **string container**: `data[length-4]` is UTF-16LE text.
- The others are **containers**: `data[length-4]` is a sequence of nested blocks.

Named block types used structurally:

```c
enum BlockType {
    BT_TRAILER_DIRECTORY = 0x90, // or 0x8A: holds the chunk directory
    BT_GENERAL_CONTAINER = 0x88, // a chunk, or one chunk record
    BT_STRING_CONTAINER  = 0xC0, // UTF-16LE string (e.g. master-page name)
    BT_SHAPE_SEQNUM      = 0x70, // a u32 sequence number inside a shape list
    BT_DUMMY             = 0x78, // zero-length placeholder
};
```

### The chunk directory, in blocks

The trailer and chunk directory shown as structs in doc 00 are built from these
blocks. The directory (a variable block of type `0x90`/`0x8A`) holds one
`BT_GENERAL_CONTAINER` (`0x88`) per chunk — one `ChunkRecord` — whose child
blocks carry these ids:

```c
enum ChunkRecordBlockId {
    CR_CHUNK_TYPE          = 0x02, // u32: the chunk's type (enum ChunkType below)
    CR_CHUNK_OFFSET        = 0x04, // u32: absolute offset of the chunk's own blocks
    CR_CHUNK_PARENT_SEQNUM = 0x05, // u32: parent chunk's SeqNum (0 = top level)
};
```

A record's **SeqNum** is its index in the directory. To resolve a chunk: read
its `CR_CHUNK_OFFSET`, seek there, and read that chunk's blocks, interpreting
each block `id` in the namespace of the chunk's `CR_CHUNK_TYPE`.

## Chunks

Each chunk has a type, and within a chunk a block `id` names a field in that
chunk's namespace — the same id means different things in different chunk types.

```c
enum ChunkType {
    CT_UNKNOWN    = 0x00, // untyped; skip
    CT_SHAPE      = 0x01, // a shape (geometry/fill/stroke resolved from Escher)
    CT_TABLE      = 0x10, // a table; cells in a CELLS chunk
    CT_ALTSHAPE   = 0x20, // alternate/placeholder shape; not rendered
    CT_GROUP      = 0x30, // a group; children reference it via parent seqnum
    CT_LOGO       = 0x31, // logo building block (treat as a group)
    CT_PAGE       = 0x43, // a page
    CT_DOCUMENT   = 0x44, // document-level properties
    CT_BORDER_ART = 0x46, // BorderArt definition
    CT_PALETTE    = 0x5C, // color palette
    CT_CELLS      = 0x63, // table cell data
    CT_FONT       = 0x6C, // a font-list entry
};
```

The parent/child tree is built from each chunk's `CR_CHUNK_PARENT_SEQNUM`: pages
are children of the document, shapes are children of a page, group members are
children of a group, and a table's CELLS chunk is a child of the table.

Each table below lists the block ids a reader interprets inside that chunk. A
block carries its value per the block encoding above. Any id seen in a chunk but
not listed here is recorded in [04-unresolved.md](./04-unresolved.md), never
silently dropped.

### Document chunk (`CT_DOCUMENT`, 0x44)

```c
enum DocumentBlockId {
    DOC_WIDTH     = 0x01, // u32 EMU: page width
    DOC_HEIGHT    = 0x02, // u32 EMU: page height (also the page-list id in context)
    DOC_SIZE      = 0x12, // document size record
};
```

### Page chunk (`CT_PAGE`, 0x43)

```c
enum PageBlockId {
    PAGE_SHAPES         = 0x02, // container: list of shape SeqNums on this page
    PAGE_BG_SHAPE       = 0x0A, // u32: background shape SeqNum
    PAGE_APPLIED_MASTER = 0x0D, // string/ref: name of the applied master page
    PAGE_THIS_MASTER    = 0x0E, // string: this page's own master name (if a master)
};
```

`PAGE_SHAPES` is a container of `BT_SHAPE_SEQNUM` (0x70) blocks, each a `u32`
shape sequence number, in z-order.

### Shape chunk (`CT_SHAPE`, 0x01)

```c
enum ShapeBlockId {
    SHAPE_DONT_STRETCH_BA = 0x07, // u16 flag: don't stretch BorderArt
    SHAPE_BORDER_IMAGE_ID = 0x09, // u32: BorderArt image id
    SHAPE_TEXT_ID         = 0x27, // u32: TextId into Quill (doc 02)
    SHAPE_VALIGN          = 0x35, // u16: vertical text alignment
    SHAPE_WIDTH           = 0xAA, // u32 EMU: width
    SHAPE_HEIGHT          = 0xAB, // u32 EMU: height
    SHAPE_CROP            = 0xB7, // picture crop rectangle
};
```

A shape's geometry, fill, stroke, rotation, flips, and image reference come from
its **Escher** record (doc 03), matched by sequence number; the Contents shape
chunk mainly carries size, the text link, and a few shape-level flags.

### Table chunk (`CT_TABLE`, 0x10)

```c
enum TableBlockId {
    TABLE_NUM_ROWS      = 0x66, // u16: row count
    TABLE_NUM_COLS      = 0x67, // u16: column count
    TABLE_CELLS_SEQNUM  = 0x6B, // u32: SeqNum of the CELLS chunk
    TABLE_ROWCOL_ARRAY  = 0x6D, // container: row heights then column widths (EMU)
};
```

### Font chunk (`CT_FONT`, 0x6C) and Palette chunk (`CT_PALETTE`, 0x5C)

The font chunk is one entry in the document font list (face name as a string
container). The palette chunk carries the document's RGB palette entries indexed
by scheme/system color references (doc 00). Field-level ids beyond the font name
and palette array are listed in [04-unresolved.md](./04-unresolved.md).
