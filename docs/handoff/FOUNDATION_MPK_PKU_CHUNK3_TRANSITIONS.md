# FOUNDATION MPK/PKU — Chunk 3: PKRU Transitions

All transitions verified by static analysis of `interrupts.rs` and `scheduler.rs`.

## Entry Points (all → God Mode)

| Entry | Stub | Before → After | Verified |
|-------|------|----------------|----------|
| Syscall | `syscall_entry` (line 114) | User PKRU → God Mode | ✅ rdpkru save, wrpkru 0 |
| Timer IRQ | `timer_interrupt_stub` (line 235) | User PKRU → God Mode | ✅ wrpkru 0 |
| Page Fault | `page_fault_stub` (line 277) | User PKRU → God Mode | ✅ wrpkru 0 |
| GP Fault | `general_protection_fault_stub` (line 319) | User PKRU → God Mode | ✅ wrpkru 0 |

## Exit Points (all restore user PKRU)

| Exit | Mechanism | Restore source | Verified |
|------|-----------|----------------|----------|
| Syscall return | `syscall_entry` epilogue | Stack (saved at entry) | ✅ pop → wrpkru → sysretq |
| Context switch | `switch_to` (scheduler.rs:310) | `TaskContext.pkru` at offset 0x80 | ✅ `mov eax,[rsi+0x80]` → wrpkru → iretq |

## Save Discipline

| When | What is saved | Source | Why not rdpkru? |
|------|--------------|--------|-----------------|
| Syscall entry | User PKRU saved to stack | `rdpkru` result (live value) | Not overwritten yet |
| Context-switch save | `old_ctx.pkru` | `pd.current_pkru_mask` (software) | Handler is in God Mode; rdpkru would return 0 |
| Timer/#PF handler | `old_ctx.pkru` | `pd.current_pkru_mask` (Relaxed load) | Handlers are in God Mode |

## Critical Invariant

The kernel always runs in God Mode (PKRU=0x00000000). Every entry from user space sets this, and every exit to user space restores the per-domain PKRU. No kernel code path runs with a restrictive PKRU.
