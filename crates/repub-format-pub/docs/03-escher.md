# 03 — The Escher Drawing Streams

Shape geometry, fills, strokes, and image references live in `Escher/EscherStm`
using Microsoft's Office Drawing format (MS-ODRAW, codename "Escher") — the same
record vocabulary modelled by the `repub-escher` crate. A shape's Escher record
is matched to its Contents shape chunk by sequence number (`spid` = `SeqNum`).
Image *bytes* live in a companion stream, `Escher/EscherDelayStm`, covered at the
end of this document.

## Record header (8 bytes)

Every Escher record starts with this header:

```c
typedef struct {
    u16 verAndInstance; // low 4 bits = version; high 12 bits = instance
    u16 recordType;     // see enum below
    u32 length;         // bytes of data AFTER this header
} EscherRecordHeader;
```

```c
// version == 0xF  => this record is a CONTAINER (its data is child records)
// version != 0xF  => this record is an ATOM (its data is raw bytes)
#define ESCHER_IS_CONTAINER(hdr) (((hdr).verAndInstance & 0x000F) == 0x000F)
#define ESCHER_VERSION(hdr)      ((hdr).verAndInstance & 0x000F)
#define ESCHER_INSTANCE(hdr)     (((hdr).verAndInstance >> 4) & 0x0FFF)
```

## Record types

```c
enum EscherRecordType {
    ESCHER_DGG_CONTAINER    = 0xF000, // drawing-group container (document-wide)
    ESCHER_BSTORE_CONTAINER = 0xF001, // BLIP (image) store container
    ESCHER_DG_CONTAINER     = 0xF002, // drawing container (one per drawing/page)
    ESCHER_SPGR_CONTAINER   = 0xF003, // shape-group container
    ESCHER_SP_CONTAINER     = 0xF004, // shape container (one shape)
    ESCHER_SOLVER_CONTAINER = 0xF005, // connector-rule container
    ESCHER_FDGGBLOCK        = 0xF006, // drawing-group record (counts, max shape id)
    ESCHER_FBSE             = 0xF007, // BLIP store entry (one image)
    ESCHER_FDG              = 0xF008, // drawing record (shape count, last shape id)
    ESCHER_FSPGR            = 0xF009, // group-shape record (group bounds)
    ESCHER_FSP              = 0xF00A, // shape record: spid, shape type, flags
    ESCHER_FOPT             = 0xF00B, // shape property table (primary)
    ESCHER_SECONDARY_FOPT   = 0xF121, // property table (secondary range)
    ESCHER_TERTIARY_FOPT    = 0xF122, // property table (tertiary range)
    ESCHER_TEXTBOX          = 0xF00C, // text-box record
    ESCHER_CLIENT_TEXTBOX   = 0xF00D, // host text ref (links to a Quill TextId)
    ESCHER_ANCHOR           = 0xF00E, // shape anchor
    ESCHER_CHILD_ANCHOR     = 0xF00F, // child-shape anchor (bounds within a group)
    ESCHER_CLIENT_ANCHOR    = 0xF010, // client anchor: shape bounds on the page (EMU)
};
```

## FSP — the shape record (0xF00A)

```c
typedef struct {
    u32 spid;   // shape id; matches the Contents chunk SeqNum
    u32 flags;  // bit field; see the mask table below
} EscherFSP;
```

`flags` masks:

| Mask | Name | Meaning |
|---|---|---|
| 0x0001 | fGroup | Shape is a group. |
| 0x0002 | fChild | Shape is a child of a group. |
| 0x0004 | fPatriarch | The top-level group shape for the drawing. |
| 0x0008 | fDeleted | Shape has been deleted. |
| 0x0010 | fOleShape | Shape is an OLE object. |
| 0x0020 | fHaveMaster | Shape has a master (template) shape. |
| 0x0040 | fFlipH | Flipped horizontally. |
| 0x0080 | fFlipV | Flipped vertically. |
| 0x0100 | fConnector | Shape is a connector. |
| 0x0200 | fHaveAnchor | Shape has an anchor record. |
| 0x0400 | fBackground | Shape is the page background. |
| 0x0800 | fHaveSpt | `spid` is valid / shape type is set. |

## FOPT — the property table (0xF00B / 0xF121 / 0xF122)

An FOPT's data is an array of 6-byte property entries. The entry **count** comes
from the record header's `instance` field.

```c
typedef struct {
    u16 opidFlags; // bit 15 = complex, bit 14 = blip id, bits 0..13 = property id
    u32 value;     // simple: the value; complex: byte length of this prop's trailing data
} EscherFOPTEntry; // = 6 bytes
```

```c
#define FOPT_ID(e)         ((e).opidFlags & 0x3FFF)
#define FOPT_IS_BLIP(e)    (((e).opidFlags & 0x4000) != 0)
#define FOPT_IS_COMPLEX(e) (((e).opidFlags & 0x8000) != 0)
```

Read the array first; then the **complex** properties' payloads follow the
array, back to back, in the same order as their entries (each payload is
`value` bytes long).

Property ids a reader interprets include fills, strokes, line dashes, text
margins, shadow, 3D, and the geometry properties below. The full enumerated set
is in `escher.rs`; ids read but not yet mapped are listed in
[04-unresolved.md](./04-unresolved.md).

## Shape geometry: two sources

1. **Preset.** The FSP shape type (`spt`) plus adjust-value properties select
   one of the 202 Office preset templates. `repub-escher` evaluates the template
   (guides + segments) to an outline in the 21600×21600 space, then scales it to
   the shape's EMU bounds. See the `repub-escher` crate.
2. **Custom path.** Two complex FOPT properties give an explicit outline:
   - `pVertices` (opid `0xC145`): the point array.
   - `pSegments` (opid `0xC146`): MSOPATHINFO segment codes (move/line/curve/
     close) over those points.

## Bounds, rotation, flips

- On-page bounds come from `ESCHER_CLIENT_ANCHOR` (0xF010), in EMUs; a grouped
  child uses `ESCHER_CHILD_ANCHOR` (0xF00F) relative to the group.
- Rotation is a FOPT property in 16.16 degrees (doc 00); flips are FSP flag
  bits.

## Images: the BLIP store and EscherDelay

Image bytes are not inline. A picture shape's FOPT holds a *BLIP index* (the
entry with `FOPT_IS_BLIP` set); that index resolves through the BLIP store in
`EscherStm` to a byte range in `Escher/EscherDelayStm`.

### The BLIP store

Inside the drawing-group container (`ESCHER_DGG_CONTAINER`, 0xF000) is a
`ESCHER_BSTORE_CONTAINER` (0xF001) holding one `ESCHER_FBSE` (0xF007) per image,
in index order:

```c
typedef struct {
    u8  btWin32;        // Windows BLIP type
    u8  btMacOS;        // Mac BLIP type
    u8  rgbUid[16];     // MD4 checksum of the image bytes
    u16 tag;            // unused tag
    u32 size;           // size of the BLIP bytes
    u32 cRef;           // reference count
    u32 foDelay;        // offset into EscherDelayStm (0 = inline, rare)
    u8  usage;          // BLIP usage
    u8  cbName;         // length of an optional name that follows
    u8  unused2, unused3;
} EscherFBSE;
```

```c
enum BlipType { // the btWin32/btMacOS values
    BLIP_ERROR = 0, BLIP_EMF = 2, BLIP_WMF = 3, BLIP_PICT = 4,
    BLIP_JPEG = 5, BLIP_PNG = 6, BLIP_DIB = 7, BLIP_TIFF = 17, BLIP_CMYKJPEG = 18,
};
```

The 1-based position of an `FBSE` in the store is the `BlipIndex` that a picture
shape's FOPT references.

### EscherDelay

`Escher/EscherDelayStm` is a concatenation of BLIP records. Seek to the `FBSE`'s
`foDelay`, read a BLIP record header, then the image bytes:

```c
typedef struct {
    EscherRecordHeader hdr; // recordType identifies the blip encoding (0xF01A..0xF01F etc.)
    // For metafile blips (EMF/WMF/PICT): a metafile header, then compressed bytes.
    // For raster blips (PNG/JPEG/DIB/TIFF): one rgbUid[16] (two for some), then raw bytes.
} EscherBlip;
```

The `hdr.instance` distinguishes variants (e.g. one vs. two UID prefixes). A
reader uses the FBSE `btWin32`/`btMacOS` type to decide how to decode the bytes;
this crate preserves the raw bytes and defers decoding to the export/scene
layers.

**Writing images.** Append the bytes as a BLIP record to `EscherDelayStm`, add an
`FBSE` to the BLIP store with the correct type/size/`foDelay`/`rgbUid`, and set
the picture shape's blip-id FOPT property to the store index.
