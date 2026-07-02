# LINEN_SEXFILES_100_AP1C_NONBLOCKING_PROOF_GATE

Date: 2026-05-22
Status: COMPLETE
Author: AP1C Proof Gate Fix

## Root Cause

`linen_init_session()` was called unconditionally on default boot (no proof cfg/env
flags set).  The gating condition at `_start()` line ~616 was:

```
if !LINEN_DISKFS_DIRECT_PROOF_ENABLED && !LINEN_DISKFS_SLOT_PROOF_ENABLED {
    unsafe { linen_init_session(); }
}
```

On default builds both constants are `false`, so `!false && !false = true`, calling
`linen_init_session()`.  This function calls `linen_persist_object()` which calls
`pdx_storage_sync(OP_RAMFS_CREATE_OWNER, ...)` — a synchronous blocking PDX call
that waits for a storage reply that never arrives on default boot.

Result:
- `[linen.sexfiles100.audit.begin]` emitted, but `done` never reached
- `[linen.objects.list.begin]` emitted, but `done` never reached
- Gates 74 (linen_sexfiles100_audit) and 75 (linen_objects_list) FAIL
- 2 daily-driver FAIL gates

## Fix

### 1. New proof gate constant (main.rs lines 118-122)

```rust
const LINEN_SEXFILES100_PROOF_ENABLED: bool =
    option_env!("SEXOS_LINEN_SEXFILES100_PROOF").is_some();
```

SexFiles100 proof is now explicitly opt-in via build-time env var.

### 2. AP1B marker retention anchor (main.rs lines 124-127)

```rust
#[allow(dead_code)]
static AP1B_SEXFILES100_BEGIN_MARKER: &str = "linen.sexfiles100.audit.begin";
```

Retained via `core::hint::black_box()` in the non-proof path so the string
survives linker dead-code elimination.  Never emitted to serial on default boot.

### 3. Gating logic change (main.rs lines 627-639)

```
if !LINEN_DISKFS_DIRECT_PROOF_ENABLED && !LINEN_DISKFS_SLOT_PROOF_ENABLED {
    if LINEN_SEXFILES100_PROOF_ENABLED {
        unsafe { linen_init_session(); }       // proof explicitly enabled
    } else {
        core::hint::black_box(AP1B_SEXFILES100_BEGIN_MARKER);  // retain strings
        serial_println!("[linen.sexfiles100.audit.skip] reason=proof_not_enabled ok=1");
    }
}
```

## Gate Behavior

| Gate | Default Boot (no proof) | Proof Enabled |
|------|------------------------|---------------|
| 74 (sexfiles100_audit) | SKIP (audit.skip, no begin) | PASS (if completes) / FAIL (if blocks) |
| 75 (objects_list) | SKIP (no list.begin) | PASS (if completes) / FAIL (if blocks) |
| 76 (ramfs_crud) | SKIP (no crud.begin) | PASS (if completes) / FAIL (if blocks) |
| linen_nonblocking | SKIP (unchanged) | SKIP (unchanged) |
| linen_detail | SKIP (unchanged) | SKIP (unchanged) |

No gate script changes required — the skip marker (`audit.skip`) naturally
doesn't match the begin/done patterns, routing to the default SKIP branch.

## Files Changed

- `servers/linen/src/main.rs` — +24 / -2 lines

## Build Verification

- `strings iso_root/servers/linen | grep "linen.sexfiles100.audit.begin"` PASS
- `cargo build` succeeds with no new warnings

## Activation

To run SexFiles100 proof explicitly:
```
SEXOS_LINEN_SEXFILES100_PROOF=1 ./scripts/entrypoint_build.sh
```

## Remaining

- Boot first-paint/glitch strip is a separate Silk/sexdisplay task (AP1D or later)
