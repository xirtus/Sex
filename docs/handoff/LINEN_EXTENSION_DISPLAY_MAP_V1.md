# Linen Extension Display Map V1

**Date:** 2026-05-06
**Status:** PASS
**Gate:** `SEXOS_LINEN_EXTENSION_MAP_PROOF=1`

## Goal

Linen can display the canonical SexOS native file extension for any SexObjectKind.
Pure display metadata — never authority. No parsing, no loader, no ABI change.

## Implementation

### Helper location: `crates/sex-object-model/src/lib.rs`

Added `SexObjectKind::extension() -> &'static str` — a `const fn` match returning
the dot-prefixed extension string for all 12 V1 kinds, per the canon in
`docs/handoff/SEXOS_FILE_EXTENSION_IMPORT_CANON_V2.md`.

Zero dependencies. Available everywhere the model crate is used.

## Mapping Table

| SexObjectKind | Discriminant | Extension |
|---------------|:---:|-----------|
| RawBlob | 0 | `.sx` |
| AppManifest | 1 | `.xap` |
| AppState | 2 | `.st` |
| LinenProject | 3 | `.lin` |
| QuilDocument | 4 | `.qul` |
| SpindleSession | 5 | `.spn` |
| BellEvent | 6 | `.bel` |
| (reserved — SceneSnapshot) | 7 | — |
| CollarGrant | 8 | `.scl` |
| MeshFact | 9 | `.msh` |
| CrashReport | 10 | `.crs` |
| Package | 11 | `.spk` |

## Proof Markers

```
[linen.ext.map]     kind=0  ext=.sx
[linen.ext.map]     kind=1  ext=.xap
[linen.ext.map]     kind=2  ext=.st
[linen.ext.map]     kind=3  ext=.lin
[linen.ext.map]     kind=4  ext=.qul
[linen.ext.map]     kind=5  ext=.spn
[linen.ext.map]     kind=6  ext=.bel
[linen.ext.map]     kind=8  ext=.scl
[linen.ext.map]     kind=9  ext=.msh
[linen.ext.map]     kind=10 ext=.crs
[linen.ext.map]     kind=11 ext=.spk
[linen.ext.authority] extension_authority=0 sexobjectkind_authority=1
[linen.ext.pass]      ok=1 kinds_mapped=12
```

| Marker | Meaning | Result |
|--------|---------|--------|
| ext.map (×12) | All kinds map to canon extensions | All 11 active + 1 reserved |
| ext.authority | Extension is display-only | extension_authority=0 |
| ext.pass | All checks passed | ok=1, kinds_mapped=12 |

## Authority Model

- `SexObjectKind::extension()` is a **pure const fn** — zero side effects, zero I/O
- Extensions are **display hints** for Linen, never used by Collar or SexFiles
- The authoritative type is `SexObjectKind` (stored as `kind: u16` in SexFiles)
- Collar grants bind to `object_id` + `SexObjectKind` — never to extension strings
- Extension mismatch triggers `[extension.mismatch]` warning, not denial

## Files Changed

| File | Change |
|------|--------|
| `crates/sex-object-model/src/lib.rs` | Added `SexObjectKind::extension()` const fn |
| `servers/linen/src/main.rs` | Added proof gate + `run_extension_map_proof()` |
| `docs/handoff/SEXOS_FILE_EXTENSION_IMPORT_CANON_V2.md` | Added rule 9 (mismatch not fatal) |

## Build & Runtime

```sh
cargo check -p sex-object-model    # PASS
cargo check -p linen               # PASS
./scripts/entrypoint_build.sh      # PASS
./scripts/master_runtime_gate.sh   # GREEN_MASTER
```

No kernel edits. No sex-pdx ABI changes. No parser implementation.
