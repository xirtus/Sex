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

## Build Enforcement
- Only valid build root: `./scripts/entrypoint_build.sh`
- Build graph source of truth: `sexos_build_spec.toml`
