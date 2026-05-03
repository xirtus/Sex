# AGENT_README_FIRST

## Revision Backup Plan
Before changing code:
1. `git status --short`
2. `git branch --show-current`
3. Prefer a small branch or checkpoint commit.
4. If anything breaks: revert only your patch, then read handoffs/docs before retrying.
5. Save recurring fixes into the relevant handoff/`COMMON_FAILURES` doc.

## Token Discipline
- Use `rg` first.
- Do not open huge files fully.
- Open only mapped line ranges from QUICKMAP docs.
- Return first fatal build error only.
- No broad refactor.
- No mixed feature+cleanup patch.

## SexOS Reality
This is NOT Linux userspace. No POSIX assumptions.
Strict no_std Rust microkernel:
- no std
- no libc
- no threads
- no sleep/time APIs unless project-local
- no heap unless target crate already uses it
- PDX IPC only
- MPK/PKRU isolation
- cross-domain pointers are invalid
- sexdisplay is sole framebuffer writer

## Stop First If Needed
STOP and explain before changing:
- kernel ABI
- sex-pdx ABI
- syscall ABI
- scheduler/interrupts
- PKRU/domain switching
- framebuffer ownership
- shared-memory/backing-buffer design
- new abstraction across subsystems

## Required First Reads By Task
General runtime bug:
- `docs/AGENT_HANDOFF_GP_CLOCK.md`
- `docs/INTERRUPTS_QUICKMAP.md`
- `docs/COMMON_FAILURES.md`

PDX/server bug:
- `docs/manual_servers.md`
- `docs/PDX_QUICKMAP.md`
- `crates/sex-pdx/src/lib.rs`

Display/SilkBar:
- `docs/SILK_DE_EXECUTION_PLAN.md`
- `crates/silkbar-model/src/lib.rs`
- `servers/silkbar/src/main.rs`
- `servers/sexdisplay/src/main.rs`

Input/USB:
- `docs/INPUT_USB_NEXT.md`
- `crates/sex-pdx/src/lib.rs`
- `servers/sexinput/src/main.rs`
- `servers/silk-shell/src/main.rs`
