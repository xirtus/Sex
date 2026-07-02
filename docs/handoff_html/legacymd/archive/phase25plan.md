Current Plan
/home/xirtus_arch/.claude/plans/you-are-writing-rust-transient-moore.md

Context

Phase 25 v2 canonical spec (docs/phase25-compositor.md) is the single source of truth.
Prior implementation accumulated hybrid IPC semantics: register-based returns coexisted
with a PdxListenResult r9-pointer write path. That struct was removed from userspace but
the kernel still references sex_pdx::PdxListenResult (compile error), and a vestigial
in("r9") 0u64 in pdx_listen left the contract implicit rather than explicit.

This plan locks the IPC contract cleanly across kernel, transport layer, and compositor.

---
Root Cause (Current Desync)

Two compile-blocking / semantic issues after last session:

1. kernel/src/syscalls/mod.rs syscall 28 still holds:
- let resp_ptr = r9 as *mut sex_pdx::PdxListenResult; (line 89)
- if !resp_ptr.is_null() { ... } write block (lines 135–148)
PdxListenResult was removed from sex-pdx — this is now a compile error.
2. pdx_listen still had in("r9") 0u64 — vestigial signal to the now-deleted kernel
path. With no struct contract, r9 usage in syscall 28 is undefined. Drop entirely.

Explicit register contract (stated in code comments going forward):
Syscall 28 → kernel writes: rax=type_id (0x00=empty), rsi=caller_pd,
rdx=arg0, r10=arg1, r8=arg2. Kernel is sole authority. No r9 semantics.

---
Changes

1. crates/sex-pdx/src/lib.rs — full convergence to v2

Remove (not in spec):
- Rect struct
- MessageType enum
- PdxEvent stub
- Old blocking pdx_listen() -> PdxMessage spin loop
- PdxListenResult struct + pdx_try_listen()
- in("r9") 0u64 constraint (vestigial)
- OP_WINDOW_DESTROY, OP_MOVE_WINDOW, OP_RESIZE_WINDOW, OP_SET_BG

Add/change:
- PdxMessage._pad → pub _pad (spec requires public)
- pdx_listen(_slot: u64) -> Option<PdxMessage> — pure register contract, no r9
- SLOT_SELF: u64 = 0
- OP_COMMIT: u64 = 0xDD

pub fn pdx_listen(_slot: u64) -> Option<PdxMessage> {
// Syscall 28 ABI: kernel writes rax=type_id (0=empty), rsi=caller_pd,
// rdx=arg0, r10=arg1, r8=arg2. No r9 contract. Kernel is sole authority.
let type_id: u64;
let caller_pd: u64;
let arg0: u64;
let arg1: u64;
let arg2: u64;
unsafe {
core::arch::asm!(
"syscall",
in("rax") 28u64,
lateout("rax") type_id,
lateout("rsi") caller_pd,
lateout("rdx") arg0,
lateout("r10") arg1,
lateout("r8")  arg2,
out("rcx") _,
out("r11") _,
);
}
if type_id == 0 { None } else {
Some(PdxMessage { type_id, arg0, arg1, arg2, caller_pd: caller_pd as u32, _pad:
0 })
}
}

2. kernel/src/syscalls/mod.rs — remove dead PdxListenResult path

Delete from syscall 28 arm:
- let resp_ptr = r9 as *mut sex_pdx::PdxListenResult;
- if !resp_ptr.is_null() { ... } block (entire conditional)

Keep: tuple computation + regs.rsi/rdx/r10/r8 assignments + type_id return.
regs.rax = type_id is set by dispatch() bottom. Kernel is now explicitly the
sole register-based IPC authority for syscall 28.

3. servers/sexdisplay/src/main.rs — update to v2 API

- Import: pdx_listen only (drop pdx_try_listen)
- Pre-FB wait: loop { if let Some(m) = pdx_listen(SLOT_SELF) { if m.type_id == 0x11 {
break; } } sys_yield(); }
- Main loop per spec Section 6:

loop {
if let Some(m) = pdx_listen(SLOT_SELF) {
match m.type_id {
0xDE => { /* fill bg 0xFF1E1E2E, pdx_reply */ }
0xDF => { /* blit window area */ }
0x101 => { /* render silkbar top 48px in 0xFF0A0A14 */ }
0xDD => { /* commit — no-op */ }
other => { /* red error strip + serial_println */ }
}
} else {
/* idle: 0xFF1A1A2E base + 1px 0xFF00FFCC stripe, sys_yield */
}
}

---
Files Modified

┌────────────────────────────────┬──────────────────────────────────────────────────────
┐
│              File              │                        Change
│
├────────────────────────────────┼──────────────────────────────────────────────────────
┤
│ crates/sex-pdx/src/lib.rs      │ Full v2 convergence — remove legacy types, fix
│
│                                │ pdx_listen, add spec constants
│
├────────────────────────────────┼──────────────────────────────────────────────────────
┤
│ kernel/src/syscalls/mod.rs     │ Remove dead PdxListenResult write path from syscall
│
│                                │ 28 arm
│
├────────────────────────────────┼──────────────────────────────────────────────────────
┤
│ servers/sexdisplay/src/main.rs │ Update to pdx_listen(SLOT_SELF) API, idle + error
│
│                                │ frame paths
│
└────────────────────────────────┴──────────────────────────────────────────────────────
┘

Apply steps 1 and 2 together — kernel won't compile until sex_pdx::PdxListenResult is
gone.

Verification

./build_payload.sh && make iso && make run-sasos

Expected:
- No compile errors referencing PdxListenResult
- Serial: [sexdisplay] LISTENING
- Screen: 0x1A1A2E base on idle, panel strip on RENDER_BAR, never black
