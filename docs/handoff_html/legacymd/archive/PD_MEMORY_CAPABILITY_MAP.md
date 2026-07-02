# PD Memory Range / Capability Map

Status: PD_MAP_DIAGNOSTIC_V1  
Purpose: make zero-copy PDX work safe by exposing current protection-domain ownership, PKU keys, entry addresses, and capability grants.

## Invariant

SexOS is a Single Address Space OS. Virtual addresses are globally meaningful, but access is controlled by:

1. PKEY on mapped pages.
2. Current PKRU mask.
3. Capability grants.
4. PDX slot/call policy.

A pointer alone is never authority.

## Required Per-PD Fields

| Field | Meaning |
|---|---|
| `pd_id` | ProtectionDomain id / registry key |
| `name` | boot module or server name, if tracked |
| `pkey` | Intel MPK/PKU page key assigned to PD |
| `base_pkru_mask` | default allowed/denied key mask |
| `current_pkru_mask` | live runtime PKRU policy |
| `entry` | ELF entry address / task RIP carrier |
| `owned_ranges` | pages mapped for this PD with its pkey |
| `granted_caps` | memory, IPC, interrupt, domain caps |
| `message_ring` | PDX inbound queue pointer/depth |
| `task_state` | runnable/exited/faulted if tracked |
| `last_fault` | latest #PF/#GP if tracked |

## Diagnostic Line Format

```text
[pd.map.begin] reason=boot_post_spawn
[pd.map] id=1 name=sexdisplay pkey=1 base_pkru=0xfffffff3 current_pkru=0xfffffff3 entry=0x... msg_ring=0x... cap_table=0x...
[pd.range] pd=1 kind=elf start=0x... end=0x... pkey=1 rights=rx
[pd.range] pd=1 kind=fb start=0x... end=0x... pkey=1 rights=rw
[pd.cap] pd=1 idx=0 kind=ipc target=kernel rights=call
[pd.map.missing] field=owned_ranges reason=not_tracked_currently
[pd.map.end] count=6
```

## Zero-Copy PDX Interpretation

For any proposed zero-copy handoff:

*   sender owns range?
*   receiver has cap?
*   range pkey can be temporarily granted?
*   grant is bounded?
*   revocation path exists?
*   receiver never receives naked authority from pointer alone?

## Future Extension

Add explicit range tracking only after the diagnostic proves what is missing:

```rust
PdMemoryRange {
    start: u64,
    end: u64,
    pkey: u8,
    owner_pd: u32,
    rights: Rights,
    kind: ElfText | ElfData | Stack | Heap | Framebuffer | Lent | Dma,
}
```

Do not add this until current map output proves the missing fields.
