# SexOS File Extension & Import Canon V2

**Date:** 2026-05-06
**Status:** CANON LOCKED
**Supersedes:** implicit extension usage in ramfs filenames (.lo prefix, quil_doc_01)
**Gate:** Documentation only — no code changes

## Design Rules (authority model)

1. **SexObjectKind is the real type.** Extension strings are display hints, never
   type authority. The authoritative type discriminant lives in the SexFiles
   object table (`SexfilesObjectEntry.kind: u16`).
2. **SexObjectRef is the real identity.**  `{ object_id (global), generation }`.
   Extension never determines object_id or Collar grant scope.
3. **Collar grants govern access.** Extension has no bearing on capability
   checks, operation masks, or revocation.
4. **Linen may display extensions** as part of the display name, but display
   name is bounded to 24 bytes and extension is informational only.
5. **Extension never grants authority.** No POSIX `chmod`-style permission
   derived from extension. No `#!` shebang execution policy. No MIME-type
   dispatch based on extension alone.
6. **Imported bytes become or attach to a SexObject.** An importer PD creates
   a SexObject (kind = RawBlob or media-kinded in future), stores content
   bytes via RamFS/SexFiles, and optionally creates a Linen metadata record.
7. **Parser/decoder runs as isolated PD with least privilege.** Decoder PD
   receives SLOT_STORAGE for read, no write to source object. Output goes
   to a new SexObject. Failed decode must not corrupt original bytes/object
   metadata.
8. **No POSIX path semantics.** Extensions are NOT part of a file path tree.
   SexFiles RamFS is a flat namespace. Extensions are stored in the display
   name field and optionally as a separate metadata byte.
9. **Extension mismatch is not fatal.** If a file's extension disagrees with
   its SexObjectKind (e.g. `.qul` on a RawBlob, or `.txt` on a QuilDocument):
   - Linen emits a warning marker `[extension.mismatch]` with the claimed
     extension and actual SexObjectKind.
   - Import review is triggered (pause, inspect, confirm or override).
   - The authoritative SexObjectKind always wins. Extension is metadata only.
   - Collar grants are unaffected — they bind to SexObjectKind + object_id,
     never to extension strings.
   - No import is blocked solely on extension mismatch; user may override.

## Native SexOS Extensions

| Extension | SexObjectKind | Linen Kind | Description | Status |
|-----------|---------------|------------|-------------|--------|
| `.sex` | RawBlob (0) | ImportPlaceholder (7) | Universal SexObject bundle/export (archive of object + metadata + caps) | Future |
| `.sx` | RawBlob (0) | ImportPlaceholder (7) | Compact native SexObject blob (single-object wire format) | Future |
| `.qul` | QuilDocument (4) | Document (0) / QuilWorkspaceReference (9) | Quil text surface / document | ✅ Active |
| `.spn` | SpindleSession (5) | — | Spindle session/log (terminal session recording) | ✅ Bound (M9) |
| `.lin` | LinenProject (3) | Project (0) | Linen project / object collection metadata | Future |
| `.bel` | BellEvent (6) | BellEventReference (8) | Bell event/notification archive | ✅ Bound (M8) |
| `.msh` | MeshFact (9) | MeshDiagnosticReference (10) | Mesh graph/facts export | ✅ Bound (M7) |
| `.scl` | CollarGrant (8) | — | Collar grant/policy bundle | Future |
| `.slk` | RawBlob (0) | — | Silk scene/layout (appearance tokens + surface topology) | Deferred |
| `.fab` | RawBlob (0) | — | Fabric/theme/design-token pack | Future |
| `.xap` | AppManifest (1) | — | App manifest/bundle (manifest + ELF + assets) | ✅ Active (manifest store) |
| `.spk` | Package (11) | — | Package / dependency bundle | Future |
| `.st`  | AppState (2) | — | App state snapshot (save/restore) | Future |
| `.crs` | CrashReport (10) | — | Crash report / diagnostic dump | Future |

### Conflict Avoidance

Three extensions were deliberately chosen over more obvious alternatives:

| Chosen | Avoided | Reason |
|--------|---------|--------|
| `.xap` | `.app` | `.app` is the macOS Application Bundle — Finder would attempt to launch it |
| `.spk` | `.pkg` | `.pkg` is macOS Installer, FreeBSD pkg(8), and Solaris SVR4 |
| `.scl` | `.col` | `.col` conflicts with HTML color tables and legacy COBOL (minor, preempted) |

`.msh` and `.st` are kept despite legacy conflicts (GNU mesh viewer, Smalltalk/Atari ST)
because those formats are effectively extinct and the SexOS-owned semantics dominate.

### Extension Display Rules

Linen display name is bounded to 24 bytes (RamFS max name). Native extensions
consume 3-4 bytes (dot + 2-3 chars). The extension is the last N bytes of the
display name, separated by `.`:

```
"README.qul"     → kind=QuilDocument, display_name="README.qul" (10 bytes)
"session_01.spn" → kind=SpindleSession, display_name="session_01.spn" (15 bytes)
"mesh_export.msh"→ kind=MeshFact, display_name="mesh_export.msh" (15 bytes)
"crash_001.crs"  → kind=CrashReport, display_name="crash_001.crs" (13 bytes)
```

## Standard Import/View Extensions

These are external formats SexOS should be able to import, view, or play
without reinventing the codec. All imported content becomes or attaches to
a SexObject.

### Text / Markup
| Extension | Import As | Decoder Needed | Priority |
|-----------|-----------|----------------|----------|
| `.txt` | RawBlob → QuilDocument (text surface) | Trivial (UTF-8 passthrough) | P0 |
| `.md` | RawBlob → QuilDocument (rendered markup) | Markdown→CP437 renderer | P1 |

### Image (Raster)
| Extension | Import As | Decoder Needed | Priority |
|-----------|-----------|----------------|----------|
| `.png` | RawBlob → MediaAsset (framebuffer) | PNG decoder PD | P0 |
| `.jpg` / `.jpeg` | RawBlob → MediaAsset | JPEG decoder PD | P1 |
| `.gif` | RawBlob → MediaAsset (animated) | GIF decoder PD | P2 |
| `.webp` | RawBlob → MediaAsset | WebP decoder PD | P2 |
| `.bmp` | RawBlob → MediaAsset | BMP decoder PD | P2 |

### Audio
| Extension | Import As | Decoder Needed | Priority |
|-----------|-----------|----------------|----------|
| `.wav` | RawBlob → AudioAsset | WAV decoder PD | P1 |
| `.mp3` | RawBlob → AudioAsset | MP3 decoder PD | P1 |
| `.flac` | RawBlob → AudioAsset | FLAC decoder PD | P2 |
| `.ogg` | RawBlob → AudioAsset | Ogg/Vorbis decoder PD | P2 |

### Video
| Extension | Import As | Decoder Needed | Priority |
|-----------|-----------|----------------|----------|
| `.mp4` | RawBlob → VideoAsset | MP4/H.264 decoder PD | P2 |
| `.webm` | RawBlob → VideoAsset | WebM/VP9 decoder PD | P2 |
| `.mkv` | RawBlob → VideoAsset | MKV decoder PD | P3 |

### Structured Data
| Extension | Import As | Decoder Needed | Priority |
|-----------|-----------|----------------|----------|
| `.json` | RawBlob → QuilDocument (formatted text) | JSON→text formatter | P1 |
| `.toml` | RawBlob → QuilDocument | TOML→text formatter | P0 (build config) |
| `.yaml` | RawBlob → QuilDocument | YAML→text formatter | P2 |
| `.xml` | RawBlob → QuilDocument | XML→text formatter | P2 |

### Executable / Font / Archive
| Extension | Import As | Decoder Needed | Priority |
|-----------|-----------|----------------|----------|
| `.wasm` | RawBlob → AppManifest (with manifest) | WASM validator PD | P2 |
| `.ttf` / `.otf` | RawBlob → FontAsset | Font parser PD | P1 |
| `.zip` | RawBlob → Package (extracted tree) | ZIP decoder PD | P1 |
| `.tar` | RawBlob → Package | TAR decoder PD | P2 |

## Parser/Decoder Isolation Rules

1. **Every decoder runs as its own PD** (protection domain) with MPK/PKU
   isolation. A crash in the PNG decoder must not affect the JPEG decoder
   or any other server.
2. **Least privilege capability set:**
   - `SLOT_STORAGE` (read source object, write output object)
   - `SLOT_DISPLAY` only if the decoder is also a viewer (sexdisplay sole
     framebuffer writer — STOP FIRST before granting).
   - No `SLOT_LINEN`, no `SLOT_BELL`, no `SLOT_QUIL` beyond what's needed.
3. **Input validation before decode.** The importer PD reads raw bytes,
   validates the header/magic bytes match the claimed extension, then
   passes validated bytes to the decoder PD.
4. **Failed decode → no corruption.** If the decoder PD panics or returns
   error, the original source bytes are untouched. The output object is
   marked failed/tombstoned, not partially written.
5. **Decoder output is a new SexObject.** The decoded content (rendered
   framebuffer, text surface, audio buffer) is a new SexObject with its
   own global `sexfiles_object_id`. The source object is preserved.
6. **No decoder PD may spawn child PDs** unless that capability is
   explicitly granted via Collar.

## Import / Export Rules

### Import
1. User selects a standard-format file via Linen or Spindle.
2. Importer PD reads raw bytes from RamFS or external media.
3. Extension is checked for known import format.
4. Content is validated (magic bytes, header integrity).
5. If known format: appropriate decoder PD is spawned, decodes content,
   creates output SexObject (e.g., QuilDocument for .md, MediaAsset for .png).
6. If unknown format: stored as RawBlob with extension preserved in name.
7. Linen object is created pointing to the output SexObject.

### Export
1. User selects a SexObject for export.
2. Exporter PD reads object metadata + content from RamFS.
3. Extension is chosen based on SexObjectKind:
   - QuilDocument → `.qul` or `.txt`
   - SpindleSession → `.spn`
   - RawBlob → original extension or `.sex`
4. Content is serialized to portable format if needed.
5. Export artifact is stored as a new RamFS file.

## Forbidden Assumptions

| Assumption | Why Forbidden |
|-----------|---------------|
| Extension determines file type authority | SexObjectKind is canonical |
| Extension implies Collar grant scope | Collar is operation-based, not path-based |
| Extension is part of file path tree | SexFiles is flat namespace |
| Extension maps 1:1 to MIME type | No MIME registry; decoder dispatch is explicit |
| `.exe` / `.dll` / `.so` exist in SexOS | No PE/ELF/DLL semantics; `.app` + `.wasm` replace them |
| Decoder runs in same PD as caller | Isolation rule: each decoder is a separate PD |
| Imported bytes are mutable in place | Imports create new SexObjects; source is preserved |
| Extension implies POSIX `chmod +x` | No POSIX permission model |
| Extension length is unrestricted | Display name is 24 bytes max (RamFS) |

## Existing Conflicts Resolved

| Item | Resolution |
|------|-----------|
| RamFS metadata files use `lo.{id:016x}` prefix | Internal Linen metadata; no user-visible extension. Unchanged. |
| Quil uses `quil_doc_01` (no extension) | Now canonical: `.qul` extension for Quil documents |
| Linen seed objects have no extensions | Seeds are boot objects; extension field is 0 (unset) |
| Mesh facts have no file representation | `.msh` extension reserved for export; not used in memory |

## Future Implementation Sequence

1. **P0:** Add extension byte to Linen object metadata record (1 byte in the
   48-byte metadata layout, currently byte 22 is `flags: u8` — repurpose a
   reserved flag bit for "extension_present", add 4 bytes for extension at
   offset 48-51, extending record to 52 bytes).
2. **P0:** `.txt` import (trivial: UTF-8 passthrough to QuilDocument).
3. **P1:** `.md` renderer PD (Markdown → CP437 text surface).
4. **P1:** `.png` decoder PD + `.jpg` decoder PD.
5. **P1:** `.json` / `.toml` formatter PD.
6. **P2:** Audio/video decoders, WASM validator.

## Next Safe Steps

1. **Extension metadata field** — Add 1-byte extension kind + 4-byte extension
   string to Linen object metadata record. Backward-compatible: old records
   have extension=0 (unset).
2. **`.txt` import proof** — Create a RawBlob SexObject, spawn a trivial
   decoder PD that copies bytes to a new QuilDocument, verify roundtrip.
3. **Decoder PD spawning contract** — Define the PDX protocol for
   `spawn_decoder(source_oid, output_kind)` with capability isolation.
