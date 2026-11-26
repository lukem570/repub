# 04 — Unresolved Bytes Ledger

This is the "no unknown blocks" ledger. Everything the format reader consumes
but does not yet fully interpret is listed here with its exact stream,
offset/record, and size. Nothing is described as "unknown" anywhere else in this
guide; this file is the single source of remaining reverse-engineering work.
**The format is fully specified when this list is empty.**

Each item states where it is, its size, what a reader currently does, and the RE
needed to resolve it. The `MSPUB.EXE` Ghidra project at
`reverse/ghidra_projects/mspub` and libmspub are the sources.

## Container

### U1 — `Contents` header `signature[0x1A]` (doc 00), 26 bytes
- Now: preserved verbatim; only `trailerOffset` at 0x1A is interpreted.
- Needed: break down the signature/version fields (expected: a magic value and
  a format-version word). RE: find Publisher's Contents-writer in `MSPUB.EXE`
  and read the constant it emits.

### U2 — `Quill` header `header[0x18]` (doc 02), 24 bytes
- Now: skipped; the chunk list is read from 0x18.
- Needed: identify the fields (likely magic + list offset/count).

### U3 — `ChunkRef._flags` (doc 02), +0x08, 4 bytes
- Now: read and preserved; constant `0x01000000` in all observed files.
- Needed: confirm whether any bit varies (compression? chunk kind?).

### U4 — `Internal` stream (doc 00)
- Now: present but its bytes are not interpreted.
- Needed: enumerate its flag fields (small and fixed in observed files).

### U5 — `Envelope` stream (doc 00)
- Now: not interpreted (empty/fixed in observed files).
- Needed: document the envelope-settings layout when non-empty.

## Contents

### U6 — Unrecognized block `type` (doc 01)
- Now: a block whose `type` is not in the size table is logged and skipped.
- Needed: none expected for 2k2+ files; if one appears, add its size rule here
  with the offending file. Target: this never fires.

### U7 — Unrecognized block `id` within a chunk (doc 01)
- Now: a block id not in its chunk's id enum is preserved but unmapped.
- Needed: map the remaining ids per chunk (Document/Page/Shape/Table) by diffing
  files that toggle one property at a time.

## Quill

### U8 — FDPC character-style record, unmapped tail (doc 02)
- Now: font ref, size, bold, italic, underline, color are mapped; the rest is
  read past.
- Needed: map kerning, scaling, language, and character effects.

### U9 — FDPP paragraph-style record, unmapped tail (doc 02)
- Now: alignment, spacing, and indents are mapped; the rest is read past.
- Needed: map tabs, list/numbering, borders, and shading.

### U10 — `STSH` override chain and `TCD ` cell attributes (doc 02)
- Now: `STSH` base style is read; `TCD ` is partially mapped.
- Needed: the full style-override chain and complete table-cell attributes.

## Escher

### U11 — Unmapped FOPT property ids (doc 03)
- Now: fills, strokes, geometry, text margins, shadow, and 3D are read; other
  opids are preserved but not applied.
- Needed: enumerate and map the remaining opids (text-effect, line-dash detail,
  advanced fill, perspective) against MS-ODRAW and the `escher.rs` warnings.

## Out of scope (deliberately excluded, not unknown)

### X1 — Publisher 97/98/2000 Contents format (doc 00)
- The pre-2002 chunk layout is a different format this guide does not cover. It
  is excluded by design. Supporting it would be a separate parser.
