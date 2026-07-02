# Bell Local Helpers — pdx_reply Migration V1

## Status: Merged

## Changes
Replaced Bell's local `bell_reply()` inline-asm helper with the shared
`sex_pdx::pdx_reply(target_pd, value)`.

### Removed
`servers/sexbell/src/main.rs` lines 7–22:
```rust
/// Reply to caller via kernel syscall 29 (SYSCALL_PDX_REPLY).
/// sex-pdx's pdx_reply() uses syscall 1 — unhandled in current kernel. Use 29 directly.
...
unsafe fn bell_reply(target_pd: u32, val: u64) { ... }
```

Replaced with:
```rust
// Replaced local bell_reply helper with shared sex_pdx::pdx_reply(target_pd, value).
```

### Updated import
Added `pdx_reply` to existing `use sex_pdx::{...}`.

### Updated callsites
| Line | Old | New |
|------|-----|-----|
| 578 | `bell_reply(caller_pd, u64::MAX)` | `pdx_reply(caller_pd, u64::MAX)` |
| 685 | `bell_reply(caller_pd, packed)` | `pdx_reply(caller_pd, packed)` |

## Equivalence proof
Both helpers used identical register convention:
- `rax=29` (SYSCALL_PDX_REPLY)
- `rdi=target_pd` (u32)
- `rsi=val` (u64)
- `rcx/r11` clobbers
- `options(nostack)`

Only difference: shared helper returns `u64` status (discarded with `;`).

## Files changed
- `servers/sexbell/src/main.rs`

## Build
✅ `./scripts/entrypoint_build.sh` passes

## Runtime proof
All three Bell presence markers present:
```
[bell.list.reply]  total=0 lanes=[0 0 0 0 0 0] redacted=0
[silkbar.bell.poll.reply]  total=0 redacted=0 flags=0x0
[sexdisplay.bell.render]  total=0 redacted=0 flags=0x0
```

## Remaining local helper to migrate
**sexstore's `kv_reply`** (`servers/sexstore/src/main.rs` lines 127–137):
- Same syscall 29 convention
- Signature: `unsafe fn kv_reply(target_pd: u64, val: u64)`
- Minor type difference: `target_pd: u64` vs shared helper's `target_pd: u32`
- 12 callsites
- Can be migrated identically — just needs `target_pd as u32` cast or accept the widening
