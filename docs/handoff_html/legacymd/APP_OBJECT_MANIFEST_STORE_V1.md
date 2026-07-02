# APP_OBJECT_MANIFEST_STORE_V1

- date: 2026-05-06
- git commit: pending
- target: `SEXOS_APP_MANIFEST_STORE_PROOF=1`
- qemu: same as MASTER_RUNTIME_GATE_V1

## Purpose
Make app manifests first-class SexFiles-backed objects. Defines a minimal,
bounded, versioned `AppManifestRecord` content schema. Stores manifests
via DiskFS object entries (kind=1 = AppManifest) with serialized content
in a bounded content store. Provides deterministic validation rejecting
bad versions, unknown capability bits, oversized titles, and malformed
records.

## Schema Shape

### AppManifestRecord (64 bytes, repr(C, packed))

| Offset | Size | Field | Description |
|--------|------|-------|-------------|
| 0 | 4 | magic | `0x4E41_4D41` ("AMAN" = App MANifest) |
| 4 | 1 | manifest_version | Schema version (1) |
| 5 | 2 | app_id | Bounded app discriminator (16-bit) |
| 7 | 1 | title_len | Title byte count (max 24) |
| 8 | 24 | title | Title text, 0-padded |
| 32 | 1 | capability_bits | Capability mask (BELL=0x01, SEXFILES=0x02) |
| 33 | 1 | _pad | Must be 0 |
| 34 | 8 | surface_id | Requested surface ID |
| 42 | 8 | entry_opcode | Launch PDX opcode (e.g., 0xFA = OP_APP_SURFACE_REQ) |
| 50 | 8 | state_object_id | Optional linked app state object (0 = none) |
| 58 | 4 | checksum | Xor-based integrity over bytes 0..57 |
| 62 | 2 | _reserved | Must be zero |

### Validation Rules
| Condition | Error |
|-----------|-------|
| magic != MANIFEST_MAGIC | `ERR_NOT_FOUND` |
| manifest_version != 1 | `ERR_PERM_DENIED` ("bad version") |
| title_len > 24 | `ERR_NAME_TOO_LONG` |
| capability_bits & !KNOWN != 0 | `ERR_PERM_DENIED` ("bad caps") |
| _pad != 0 or _reserved != 0 | `ERR_OVERFLOW` ("malformed") |
| checksum mismatch | `ERR_OVERFLOW` |

### Storage Model
- **Metadata**: DiskFS object entry, kind = 1 (`SexObjectKind::AppManifest`)
- **Content**: Bounded static content store (16 slots, keyed by object_id)
- In the persistent path (M3), content moves to SexFiles block storage;
  the proof gate exercises the logical store→validate→reject surface
  with synthetic fixtures only.

### Mapping to silk-shell AppManifest
The durable `AppManifestRecord` schema mirrors the transient silk-shell
`AppManifest` (packed in PDX message args):
- `app_id` ↔ silk-shell's 16-bit app discriminator
- `capability_bits` ↔ `AppCapabilityBits` (BELL=0x01, SEXFILES=0x02)
- `surface_id` ↔ silk-shell's surface_id (arg0)
- `entry_opcode` ↔ `OP_APP_SURFACE_REQ` (0xFA)

## Proof Markers

| Marker | Status | Description |
|--------|--------|-------------|
| `[app.manifest.proof.start]` | EMITTED | Proof gate entry |
| `[app.manifest.proof.create]` | ok=1 | Manifest stored as SexFiles object (object_id=1, kind=1) |
| `[app.manifest.proof.read]` | ok=1 | Manifest read back, fields match stored values |
| `[app.manifest.proof.match]` | ok=1 | Full roundtrip match (magic, version, app_id, caps, surface, entry) |
| `[app.manifest.proof.bad_version]` | ok=1 | Version byte corrupted → `ERR_PERM_DENIED` |
| `[app.manifest.proof.bad_caps]` | ok=1 | Unknown capability bits injected → `ERR_PERM_DENIED` |
| `[app.manifest.proof.bounds]` | ok=1 | Oversized title_len injected → `ERR_NAME_TOO_LONG` |
| `[app.manifest.proof.done]` | EMITTED | All checks passed |

## Files Changed

| File | Change |
|------|--------|
| `servers/sexfiles/src/manifest.rs` | **NEW** — AppManifestRecord schema, content store, store/load API, proof injection helpers |
| `servers/sexfiles/src/lib.rs` | +`pub mod manifest;` |
| `servers/sexfiles/src/main.rs` | +`mod manifest;` (bin target) |
| `servers/sexfiles/src/proof.rs` | +`run_app_manifest_store_proofs()` (6 sub-tests) |
| `servers/sexfiles/src/trampoline.rs` | +gate hook for `SEXOS_APP_MANIFEST_STORE_PROOF` |

## Build/Runtime

- `cargo check -p sexfiles`: **PASS**
- `./scripts/entrypoint_build.sh`: **PASS**
- `SEXOS_APP_MANIFEST_STORE_PROOF=1 ./scripts/master_runtime_gate.sh --probe 25 --keep-log`: **PASS (GREEN_MASTER)**

## Non-Goals Kept
- No kernel edits
- No `sex-pdx` ABI edits
- No POSIX package/install semantics
- No loader/process model redesign
- No app gets framebuffer/raw disk authority
- No unbounded manifest fields
- No broad refactor
- No shared-memory/backing-buffer redesign

## Contract Alignment
- `SexObjectKind::AppManifest = 1` — matches the existing `sex-object-model` enum
- `AppCapabilityBits::{BELL=0x01, SEXFILES=0x02}` — matches silk-shell's capability contract
- `OP_APP_SURFACE_REQ = 0xFA` — matches silk-shell's app surface request opcode
- Schema fits within RamFS 4096-byte content bound (64 bytes per manifest)

## Remaining Manifest/Runtime Gaps

1. **No real persistent content store** — Content lives in a bounded static array.
   The M3 block-device persistence path (SexFiles→SexDrive) is not yet wired
   for object content. Manifests are proven logically correct but not
   crash-durable.

2. **No Collar→Manifest capability binding** — `capability_bits` are stored
   but not enforced by Collar at runtime. When an app launches with
   `OP_APP_SURFACE_REQ`, silk-shell validates caps from the PDX message args,
   not from the persisted manifest. The manifest is a durable record but
   not yet the authoritative source for Collar enforcement.

3. **No app-state object linking** — `state_object_id` field exists but
   there is no mechanism to create/link app state objects. The field is
   reserved for future app state persistence.

4. **No launch-from-manifest path** — The manifest stores `entry_opcode` but
   there is no runtime code path that reads the manifest and uses it to
   spawn/launch the app. The silk-shell currently receives manifest data
   via PDX message args, not from SexFiles.

5. **No title→object_id index** — Manifests are looked up by object_id only.
   There is no name-based or app_id-based index for fast lookup.
