# pdx_reply Syscall Mismatch Audit V1

## Status: STOP FIRST — docs only, no patch

---

## 1. Kernel SYSCALL_PDX_REPLY (syscall 29)

**File**: `kernel/src/syscalls/mod.rs` line 263

```
29 => { // SYSCALL_PDX_REPLY
    let target_pd_id = rdi as u32;
    let val = rsi;
    if crate::ipc::router::send_reply(target_pd_id, val).is_ok() { 0 } else { 1 }
}
```

| Register | Purpose | Type |
|----------|---------|------|
| `rax` | Syscall number | `29` |
| `rdi` | Target PD ID | `u32` |
| `rsi` | Reply value | `u64` |
| Return | Success/failure | `0` on success, `1` on error |

Semantics: pushes `IpcReply { value: val }` to target PD's `incoming_replies` queue,
then unparks the target if it was blocked on listen.

---

## 2. sex_pdx::pdx_reply (current implementation)

**File**: `crates/sex-pdx/src/lib.rs` line 294

```rust
pub fn pdx_reply(target_pd: u64) {
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 1,
            in("rdi") target_pd,
        );
    }
}
```

### Three distinct bugs

| # | Problem | Detail |
|---|---------|--------|
| 1 | Wrong syscall number | Uses `rax = 1` — kernel has no handler for syscall 1; falls through to `_ => u64::MAX` (line 409). No reply is sent. |
| 2 | Missing second argument | Kernel expects `rsi = val` but function takes only `target_pd: u64`. No `val` parameter exists. |
| 3 | Missing clobber declarations | `syscall` instruction clobbers `rcx` and `r11` (returns to saved RIP/RFLAGS). Current asm omits `out("rcx") _, out("r11") _` — can corrupt register state. |

---

## 3. Callsite Inventory

### Workspace servers (built by `./scripts/entrypoint_build.sh`)

| Server | Callsite | Args | Broken? |
|--------|----------|------|---------|
| `silk-shell` | line 9780 | `pdx_reply(0)` | YES — target_pd=0 (invalid), missing val, wrong syscall |
| `silk-shell` | line 9885 | `pdx_reply(0)` | YES — same |
| `silk-shell` | line 10966 | `pdx_reply(0)` | YES — same |
| `sexstore` | bypasses | `kv_reply(target_pd, val)` with syscall 29 | NO — uses own helper correctly |
| `sexbell` | bypasses | `bell_reply(target_pd, val)` with syscall 29 | NO — uses own helper correctly |

### Non-workspace servers (NOT built by entrypoint, dead code)

| Server | Import | Signature mismatch? |
|--------|--------|-------------------|
| `sexgemini` | `sex_pdx::pdx_reply` | Calls `pdx_reply(req.caller_pd, handover.pfn)` — TWO args, but function takes ONE |
| `sext` | `sex_pdx::pdx_reply` | Calls `pdx_reply(req.caller_pd, 0)` — TWO args |
| `sexc` | `libsys::pdx::pdx_reply` | Multiple calls with TWO args |
| `sexnode` | `libsys::pdx::pdx_reply` | Multiple calls with TWO args |
| `sex-ld` | `libsys::pdx::pdx_reply` | Calls `pdx_reply(req.caller_pd, &reply...)` — TWO args |
| `sexfiles` | `sex_pdx::pdx_reply` | Wraps in `vfs_pdx_reply(caller, msg)` — TWO args |
| `sexdrive` (`servers/`, not `apps/`) | `libsys::pdx::pdx_reply` | Calls `pdx_reply(req.caller_pd, CONFIRM_SIG)` — TWO args |
| `silkbar` (`crates/`, not `servers/`) | `sex_pdx::pdx_reply` | Calls `pdx_reply(req.caller_pd, 0)` — TWO args |
| `tatami` (`crates/`) | `sex_pdx::pdx_reply` | Import only (no callsite visible) |

**Note**: Non-workspace servers calling with TWO args would FAIL TO COMPILE against
current `sex_pdx::pdx_reply(target_pd: u64)` — the function only takes one argument.
This confirms these servers are stale/unmaintained.

---

## 4. Risk Classification

| Factor | Risk | Detail |
|--------|------|--------|
| Working servers broken by bug | **Low** | silk-shell's `pdx_reply(0)` already sends to PD 0 (invalid) via syscall 1 (unhandled) — no current behavior depends on replies |
| Patch scope expansion | **High** | Fixing just the syscall number is insufficient — the function signature must also change from 1 arg to 2 args, breaking all callers |
| Hidden dependencies | **Medium** | Any caller that currently compiles despite the bug might rely on the no-op behavior |
| Non-workspace servers | **Low** | Not built by entrypoint; would need individual triage |

---

## 5. STOP Decision

### STOP FIRST — do not patch sex_pdx::pdx_reply in this mission

**Reason**: The register convention for syscall 29 requires TWO arguments (`rdi=target_pd, rsi=val`),
but the current `pdx_reply` takes ONE argument and passes no `val`. Changing just the syscall
number from 1 to 29 would:
1. Send `rsi` = undefined/garbage as the reply value
2. Still have no `val` parameter in the function signature
3. Not fix the missing clobber declarations

A correct fix requires a function signature change from `fn pdx_reply(target_pd: u64)` to
`fn pdx_reply(target_pd: u64, val: u64)`, which breaks all existing callers and requires
individual inspection/fix of each callsite.

This exceeds the scope of "swap syscall number with identical register convention."

---

## 6. Recommended Cleanup Sequence

### Phase 1 (safe, isolated)
1. Fix `sex_pdx::pdx_reply` signature to `fn pdx_reply(target_pd: u64, val: u64)` with syscall 29
2. Add proper clobber declarations: `out("rcx") _, out("r11") _`

### Phase 2 (per-server)
3. Fix `silk-shell` callsites:
   - `pdx_reply(0)` → `pdx_reply(msg.caller_pd, 0)` (each callsite needs correct target_pd)
   - Add missing `caller_pd` storage where not captured
4. Verify kernel's `send_reply` handles PD 0 gracefully (currently likely fails silently)

### Phase 3 (non-workspace)
5. Triage non-workspace servers: either update to new signature, remove dead code, or confirm they still need `pdx_reply`

### Phase 4 (cleanup)
6. Remove local helpers (`bell_reply`, `kv_reply`) — they become identical to fixed `sex_pdx::pdx_reply`
7. Remove duplicative comments about syscall mismatch

---

## 7. Working Servers (No Change Needed)

These servers bypass `sex_pdx::pdx_reply` with local syscall 29 helpers and work correctly:

- **sexbell**: `bell_reply(target_pd: u32, val: u64)` — syscall 29 inline asm
- **sexstore**: `kv_reply(target_pd: u64, val: u64)` — syscall 29 inline asm

Both use the correct register convention (`rax=29, rdi=target_pd, rsi=val`)
with proper clobber declarations.

---

## 8. Kernel Syscall Table (No Change Needed)

The kernel correctly handles syscall 29 at `kernel/src/syscalls/mod.rs:263`.
Syscall 1 is unhandled and falls to `_ => u64::MAX` at line 409.

No kernel changes are required for any phase of the cleanup.
