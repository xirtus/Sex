# Debugging & Diagnostics Reference

> Referenced from CLAUDE.md (offloaded reference).

---

## Display Bring-up Checklist (Phase 24+)

When the screen is black:
1. Confirm Limine framebuffer request is fulfilled before sexdisplay spawns
2. Pass framebuffer address/width/height/pitch to sexdisplay at spawn time
3. Verify sexdisplay's PKEY (1) is assigned to the framebuffer mapping
4. Verify PKRU allows writes to key 1 when sexdisplay is active
5. Check sexdisplay isn't blocked on IPC recv() waiting for silk-shell
6. Kernel-side sanity check: write `0x00FF00FF` (magenta) directly to framebuffer from init.rs before spawning any PDX — if magenta appears, framebuffer is fine
7. Check for `function_casts_as_integer` warnings in interrupts.rs — stub addresses being cast incorrectly can cause bad handler entry points
8. Confirm `dispatch()` writes `regs.rax = 0` for syscall 0x03 success — otherwise sexdisplay thinks DisplayInfo query failed and enters error spin loop

---

## Interrupts Reading Discipline (kernel/src/interrupts.rs)

**Do not read all of `kernel/src/interrupts.rs`.** It is large (~740 lines). Instead:

1. Use `rg` to find the symbol you need:
   ```
   rg "page_fault_handler|timer_interrupt|switch_to|faulted_task_halt" kernel/src/interrupts.rs -n
   ```
2. Open only ±80 lines around the match:
   ```
   sed -n '460,540p' kernel/src/interrupts.rs
   ```
3. See `docs/INTERRUPTS_QUICKMAP.md` for the full section index with line ranges, critical invariants, and rg patterns for common debug entry points.

### Key Landmarks

| Lines  | What |
|--------|------|
| 48-49  | IDT handler registration (page_fault, GPF, timer) |
| 131-293| `syscall_entry` naked asm |
| 295-336| `page_fault_stub` naked asm (stack layout) |
| 361-456| `timer_interrupt_stub` + `timer_interrupt_handler` |
| 458-465| `faulted_task_halt()` kernel halt trampoline |
| 466-618| `page_fault_handler` (#PF dispatch) |
| 620-725| `general_protection_fault_handler` |

---

## Known Panic Pattern

`KERNEL PANIC: Userland Null Pointer Jump at RIP: 0x0` — page fault at address 0 with RIP=0 means null instruction fetch. Caused by: iretq with RIP=0 in frame (task context.rip=0), OR sysretq with rcx=0 (return addr corrupted), OR null function pointer call in userland.

---

## Diagnostics Reference

### TABLET_LIVENESS_TRACE_V1 (2026-05-04)
8 bounded markers (max 16 each) across 4 servers tracing cursor pipeline:
`sexusb → sexinput → silk-shell → sexdisplay`
**Non-interactive finding:** 15 reports forwarded, all dx=dy=0. QEMU 11.0.0 usb-tablet always reports (0,0) in headless env. Requires interactive SDL test with physical mouse.
See `docs/handoff/TABLET_LIVENESS_TRACE_V1.md`.

### QEMU_INPUT_CONFIG_AUDIT_V1 (2026-05-04)
**Status:** dev.sh audit complete. QEMU 11.0.0 not delivering non-idle coordinates to usb-tablet. Guest pipeline proven healthy. Dead layer is outside guest (QEMU input delivery).
See `docs/handoff/QEMU_INPUT_CONFIG_AUDIT_V1.md`.

### HOST_INPUT_BACKEND_AUDIT_V1 (2026-05-04)
**Status:** QEMU 11.0.0 does not deliver host pointer motion to emulated USB HID. Both usb-mouse and usb-tablet produce only idle reports on real local desktop with physical trackpad movement. GTK and SDL backends both fail. Problem is upstream of guest — cannot fix in SexOS server code.
See `docs/handoff/HOST_INPUT_BACKEND_AUDIT_V1.md`.
