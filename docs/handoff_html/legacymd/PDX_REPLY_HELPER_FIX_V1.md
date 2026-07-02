# pdx_reply Helper Fix V1

## Status: Merged

---

## 1. Changes

### Old `sex_pdx::pdx_reply` (broken)
```rust
pub fn pdx_reply(target_pd: u64) {
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 1,         // ← wrong: syscall 1 unhandled
            in("rdi") target_pd, // ← missing rsi=value
        );                       // ← missing rcx/r11 clobbers
    }
}
```

### New `sex_pdx::pdx_reply` (fixed)
```rust
pub fn pdx_reply(target_pd: u32, value: u64) -> u64 {
    let ret: u64;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 29u64,
            in("rdi") target_pd as u64,
            in("rsi") value,
            out("rcx") _,
            out("r11") _,
            lateout("rax") ret,
            options(nostack),
        );
    }
    ret
}
```

## 2. Kernel Syscall Convention

| Register | Purpose | Type |
|----------|---------|------|
| `rax` | Syscall number | `29` (SYSCALL_PDX_REPLY) |
| `rdi` | Target PD ID | `u32` |
| `rsi` | Reply value | `u64` |
| Return (rax) | Status | `0` success, `1` error |

Kernel handler at `kernel/src/syscalls/mod.rs:263`:
```rust
29 => { // SYSCALL_PDX_REPLY
    let target_pd_id = rdi as u32;
    let val = rsi;
    if crate::ipc::router::send_reply(target_pd_id, val).is_ok() { 0 } else { 1 }
}
```

## 3. Bugs Fixed

| # | Old | New |
|---|-----|-----|
| 1 | `rax=1` (unhandled → `_ => u64::MAX`) | `rax=29` (SYSCALL_PDX_REPLY) |
| 2 | Missing `value` parameter | `value: u64` passed in `rsi` |
| 3 | No return value | Returns `u64` (kernel status in rax) |
| 4 | Missing `rcx`, `r11` clobbers | `out("rcx") _, out("r11") _` |
| 5 | `target_pd: u64` | `target_pd: u32` (matches kernel) |

## 4. Callsite Audit (pre-patch)

| Server | Old calls | Status |
|--------|-----------|--------|
| `silk-shell` | 3× `pdx_reply(0)` | Removed in prior cleanup (were dead) |
| `sexshop` | `pdx_reply(event.caller_pd, found)` | Not in build spec (stale) |
| `sexgemini`, `sext`, `sexc`, etc. | 2-arg calls | Not in workspace (dead code) |

**No active workspace server calls `pdx_reply`.** The `use` import in `silk-shell` line 8 and comments in `sexbell`/`sexstore` are the only references — none break with the new signature.

## 5. Local Helpers (Not Changed, Migration Planned)

Both are correct implementations of syscall 29. They will be replaced with the shared `pdx_reply` in a future cleanup.

### `bell_reply` (servers/sexbell/src/main.rs:12)
```rust
unsafe fn bell_reply(target_pd: u32, val: u64) { ... }
```
Difference from fixed `pdx_reply`: `unsafe`, no return value. Identical convention.

### `kv_reply` (servers/sexstore/src/main.rs:127)
```rust
unsafe fn kv_reply(target_pd: u64, val: u64) { ... }
```
Difference from fixed `pdx_reply`: `unsafe`, `target_pd: u64` instead of `u32`, no return value. Identical convention (u64→u32 cast is safe).

## 6. Migration Plan

### Phase 1 (this patch) ✅
- Fix `sex_pdx::pdx_reply` signature and implementation
- Update ABI version hash in build spec
- Build passes

### Phase 2 (next cleanup)
- Replace `bell_reply` with `sex_pdx::pdx_reply` in sexbell
- Replace `kv_reply` with `sex_pdx::pdx_reply` in sexstore
- Remove the local helpers
- Update comments

### Phase 3 (future)
- Remove `pdx_reply` from silk-shell `use` import if no future handler needs it
- Remove `pdx_reply` from `sex_pdx::*` wildcard re-exports if warranted

## 7. Build Result

`./scripts/entrypoint_build.sh` — **passes**.

## 8. Files Changed

| File | Change |
|------|--------|
| `crates/sex-pdx/src/lib.rs` | Fixed `pdx_reply` signature + implementation |
| `sexos_build_spec.toml` | Updated `abi_version_hash` to match new sex-pdx hash |
| `docs/handoff/PDX_REPLY_HELPER_FIX_V1.md` | This document |
