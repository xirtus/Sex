# IPC/PKU Canonical Map (Post ABI-Drift Closure)

This file is canonical for slot and PKU routing references used by runtime code.
Build flow canonical source is `sexos_build_spec.toml`.

## PKEY Boundaries
- **PKEY 0**: Kernel/Supervisor
- **PKEY 1**: `sexdisplay`
- **PKEY 2**: `sext` or service-assigned runtime domain
- **PKEY 3**: `silk-shell`
- **PKEY 4+**: dynamically assigned runtime domains/apps

## Capability Slots (Canonical)
- **Slot 1**: `SLOT_STORAGE`
- **Slot 2**: `SLOT_SEXT`
- **Slot 3**: `SLOT_INPUT`
- **Slot 4**: `SLOT_AUDIO`
- **Slot 5**: `SLOT_DISPLAY`
- **Slot 6**: `SLOT_SHELL`

## ABI Closure Notes (2026-04-28)
- Core live IPC paths no longer use `PdxListenResult`.
- Core live IPC listen/call paths no longer use `r9` pointer marshalling.
- `pdx_call` register return contract: `RAX=status`, `RSI=value`.
- `pdx_listen` register decode contract: `RAX=type_id`, `RSI=caller_pd`, `RDX/R10/R8=args`.

## Shell-Local Namespaces (PKEY 3 — silk-shell only)

The following ID tiers exist within silk-shell but are NOT part of the IPC/PKU
canonical map. They are shell-defined conventions, not ABI contracts. Surface
IDs are communicated to sexdisplay at call time (0xEC/0xEF/0xEE) but sexdisplay
treats them as opaque identifiers.

| Namespace | Range | Currently Assigned | Spec Doc |
|-----------|-------|-------------------|----------|
| Linen object IDs | 1-16 | 1-6 (seeds) | `docs/handoff/K2B_NAMESPACE_SPEC_DOC_V1.md` §3.1 |
| Quil seed buffer IDs | 1-6 | 1-6 (six seeds) | `docs/handoff/K2B_NAMESPACE_SPEC_DOC_V1.md` §3.2 |
| Quil dynamic buffer IDs | 1001-1016 | `QUIL_DYNAMIC_BUFFER_ID_BASE + object_id` | `docs/handoff/K2B_NAMESPACE_SPEC_DOC_V1.md` §3.3 |
| Surface IDs — OS panels | 0x90-0x97 | CURSOR, LAUNCHER, STATUS, CLOCK, BELL, SCENE_SETTINGS, ATLAS | `docs/handoff/K2B_NAMESPACE_SPEC_DOC_V1.md` §3.4 |
| Surface IDs — app | 100-103 | APP, STATIC, TEST3, TEST4 | `docs/handoff/K2B_NAMESPACE_SPEC_DOC_V1.md` §3.4 |
| Surface IDs — workstation | 200-204 | LINEN, QUIL, MESH, COLLAR, BELL_PLACEHOLDER | `docs/handoff/K2B_NAMESPACE_SPEC_DOC_V1.md` §3.4 |
| grant_ref | 0 | All current usage (no real Collar grant) | `docs/handoff/K2B_NAMESPACE_SPEC_DOC_V1.md` §3.5 |

**Rules (full details in K2B doc):**
- Dynamic buffer IDs must never collide with seed buffer IDs or surface IDs.
- All these namespaces are shell-local; they do NOT appear in PDX opcodes or ABI contracts.
- grant_ref = 0 means placeholder (no real Collar grant). Non-zero requires STOP FIRST.
- Shell-local enums (CollarOperation, CollarDecision, BellEventKind) are match-branched only.

## Build Enforcement
- Only valid build root: `./scripts/entrypoint_build.sh`
- Build graph source of truth: `sexos_build_spec.toml`
