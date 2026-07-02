# LINEN_OBJECT_KIND_SCHEMA_V1 — Handoff

## Goal
Define and emit local object kind, status, and tag schema taxonomy markers
for Linen objects.  No storage changes, no protocol changes.

## Files Changed
| File | Change | Lines |
|------|--------|-------|
| `servers/linen/src/main.rs` | Schema proof gate, function, wiring in _start | +36 |

## Schema Taxonomy

### Object Kinds
| kind | Name     | Description |
|------|----------|-------------|
| 0    | Document | Text/document object |
| 1    | Session  | Interactive session object |
| 2    | Unknown  | Catch-all / invalid kind |

### Object Statuses
| status | Name       | Description |
|--------|------------|-------------|
| 0      | local_only | Created in session, not persisted |
| 1      | persisted  | Written to SexFiles RamFS/DiskFS |
| 2      | tagged     | Has one or more tags in tag table |
| 3      | orphan     | Owned but no tags, no persistence |

### Tag Table
| Property    | Value |
|-------------|-------|
| max_tags    | 16    |
| max_tag_len | 16    |
| storage     | static BSS |

## Markers (serial)
```
[linen.schema.kind] kind=N name=NAME ok=N
[linen.schema.status] status=N name=NAME ok=N
[linen.schema.tag] max_tags=N max_tag_len=N table=static_bss
[linen.schema.proof.done] ok=N
```

## Proof Env Var
```
SEXOS_LINEN_OBJECT_SCHEMA_PROOF=1
```

## Build + Proof Result
- `entrypoint_build.sh` PASS
- `run_daily_driver_proof.sh` gate `linen_object_schema`: PASS (3 kinds, 4 statuses)

## Safety / STOP FIRST
- ❌ No kernel / ABI / USB / input / pointer / display changes
- ❌ No storage changes — markers only
- ✅ Existing object create/tag/search/persist proofs unchanged
- ✅ Schema taxonomy matches existing KIND_DOCUMENT/KIND_SESSION/KIND_UNKNOWN constants

## Known Limitations
- Schema markers are synthetic — not read back from session state
- Status taxonomy not enforced in code (no status field on LinenObject struct)
- Tag table is separate from object struct (not integrated)
- No runtime status tracking (persisted/tagged/orphan detection)

## Future Follow-up
- Add status field to LinenObject struct with runtime tracking
- Integrate tag table into LinenObject metadata
- Schema export via PDX opcode for silk-shell consumption
- Auto-detect orphan objects during session init
