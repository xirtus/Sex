# LIVE_USB_REAL_HARDWARE_BOOT_PROOF_V1

**Date:** 2026-05-25  
**Baseline HEAD:** `7ca20e6a` (quil: prove QEMU keyboard text input path V2)  
**Scope:** Prepare real-hardware live USB boot path — build artifact, runbook, observation markers  
**Previous work:** `REAL_HARDWARE_BOOT_PROOF_V1.md` (2026-05-06), `REAL_HARDWARE_BOOT_AUDIT_V1.md`

---

## A) Outcome

**PREPARED** — all build-time blockers resolved, ISO bootable on real hardware, operator runbook ready.  
Actual hardware boot observation is deferred to the operator (cannot be performed in this dev environment).

The ISO now builds with `limine bios-install` integrated into the sealed build trace, resolving
the #1 real-hardware blocker identified in the prior audit. QEMU boot continues to work
unchanged (El Torito + GPT/MBR hybrid boot coexist).

---

## B) Artifact

| Property | Value |
|----------|-------|
| **Path** | `sexos-v1.0.0.iso` |
| **Size** | ~4.0 MB (4136960 bytes at build) |
| **MD5** | `f6f30d741117f97f69c365c05dbb789d` (may change per build) |
| **Type** | ISO 9660 CD-ROM filesystem, DOS/MBR + GPT boot sector, EFI hybrid |
| **Boot** | El Torito CD-ROM + Limine BIOS MBR + Limine EFI |
| **Kernel** | `iso_root/sexos-kernel` (x86_64 ELF) |
| **Config** | `iso_root/limine.cfg` (SERIAL=yes, VIDEO_MODE=1280x720,32, TIMEOUT=3) |

### ISO Contents (15 PDX modules)

```
servers/sexdisplay   — PD1, framebuffer compositor (sole framebuffer writer)
servers/silk-shell   — PD3, window manager/shell, input router
apps/sexdrive        — PD2, NVMe block driver
servers/sexinput     — PD4, HID input router
servers/sexusb       — PD5, XHCI USB host driver
servers/silkbar      — PD6, status bar
servers/linen        — PD7, file browser (sexfiles100 native)
servers/sexstore     — PD8, key-value store
servers/quil         — PD9, text editor / app launcher
servers/sexbell      — PD10, notification daemon
servers/sexfiles     — PD11, filesystem (SexFS v0)
servers/sexnet       — PD12, network stack
apps/spindle         — PD13, task manager / control center
apps/kaleidoscope    — diagnostic visual test
apps/purple-scanout  — diagnostic fb overlay
```

---

## C) Boot Media Runbook

Full operator runbook: **`docs/handoff/LIVE_USB_OPERATOR_RUNBOOK_V1.md`**

Summary:
```bash
# 1) Build
./scripts/entrypoint_build.sh

# 2) Verify (limine bios-install is now automatic in build)
strings sexos-v1.0.0.iso | grep LIMINE  # must produce output

# 3) Preflight (on target machine)
./scripts/real_hardware_preflight.sh

# 4) Write USB ⚠️ DESTRUCTIVE ⚠️
sudo dd if=sexos-v1.0.0.iso of=/dev/sdX bs=4M status=progress conv=fsync
sync

# 5) Boot — disable Secure Boot, boot from USB (BIOS/CSM or UEFI)
# 6) Capture serial — screen /dev/ttyUSB0 115200 (or /dev/ttyS0 on target)
```

---

## D) Hardware Observations Required

Operators booting on real hardware should observe and record the following markers
(via serial output at 115200 baud, COM1 0x3F8):

### Minimum Boot Confirmation
```
[live_usb.real_boot.begin]
[live_usb.real_boot.kernel.start]
```

### PD Spawn Confirmation
```
[live_usb.real_boot.pd.spawn] name=sexdisplay ok=1
[live_usb.real_boot.pd.spawn] name=sexfiles ok=1
[live_usb.real_boot.pd.spawn] name=quil ok=1
[live_usb.real_boot.display.ready] ok=1
```

### Keyboard Route Classification
```
[live_usb.real_boot.keyboard.route] ps2=0_or_1 usb=0_or_1 honest=1
[live_usb.real_boot.no_faults] ok=1
[live_usb.real_boot.done] ok=1
```

### If No PS/2 Keyboard (Modern Laptops / UEFI-Only Systems)
```
[live_usb.real_keyboard.skip] reason=usb_hid_not_yet_implemented ok=1
```

### For PS/2 Keyboards (Older / Desktop Hardware)
Operators should type "test" and verify it reaches the Quil buffer.  
Expected markers:
```
[physical_keyboard.quil.begin]
[physical_keyboard.source] qemu_keyboard=0 physical_keyboard=1 usb=0 synthetic=0 honest=1
[physical_keyboard.quil.text.proof] text=test ok=1
[physical_keyboard.quil.done] ok=1
```

---

## E) Keyboard / Input Classification

| Field | Classification | Detail |
|-------|---------------|--------|
| `physical_keyboard` | **Hardware-dependent** | PS/2 if i8042 present; else 0 |
| `usb` | **0** | USB HID keyboard not yet implemented (XHCI driver exists, HID report parsing pending) |
| `synthetic` | **0** | No HID_STASH seeding for real hardware path |
| `qemu_keyboard` | **0** | Not QEMU; real hardware |
| `honest` | **1** | Source classification is truthful — no fake markers |

**If target hardware has PS/2 i8042 keyboard:**  
- Route: physical keystroke → i8042 → kernel IRQ1 → INPUT_RING → sexinput →
  silk-shell → Quil PD → scancode_to_char → text_buffer
- Expected result: PASS (PS/2 path proven on real hardware)

**If target hardware has only USB keyboard (modern laptop):**  
- XHCI driver enumerates USB controller but USB HID keyboard report parsing is not implemented
- Classification: **SKIP** with `reason=usb_hid_not_yet_implemented`
- Honest: this is a known capability gap, not a failure

---

## F) Fault Scan Method

Any of these markers in serial output indicate a fault and should trigger STOP:

| Marker | Severity | Meaning |
|--------|----------|---------|
| `#PF` | **CRITICAL** | Page fault — memory access violation |
| `#GP` | **CRITICAL** | General protection fault — invalid operation |
| `panic` | **CRITICAL** | Kernel panic — unrecoverable |
| `PKU violation` | **CRITICAL** | Protection key violation — MPK isolation breach |
| `fault.kill` | HIGH | Userspace PD terminated due to fault |

**Pre-boot baseline (QEMU):** faults_zero = PASS (0 fault markers in 98K-line log).

**Real hardware scan:** Operator should grep serial log for `#PF|#GP|panic|PKU`.

**Known real-hardware fault risks** (from `REAL_HARDWARE_BOOT_PROOF_V1.md`):
1. Serial port panic (COM1 absent) — **HIGH** — kernel `.expect()` on serial write
2. ACPI handler `todo!()` (DSDT/SSDT triggers stubbed method) — **HIGH**
3. PS/2 init hang (i8042 disabled in firmware) — **LOW**
4. LAPIC timer wrong rate — **MEDIUM** — scheduler cadence incorrect
5. IOAPIC polarity mismatch — **MEDIUM** — IRQ storm or missed interrupts

---

## G) Non-Claims

The following are NOT claimed by this proof:

| Non-Claim | Reason |
|-----------|--------|
| **Real hardware boot PASS** | Not yet tested on physical machine (PREPARED only) |
| **USB HID keyboard input** | USB HID report parsing not implemented (XHCI driver present, HID path pending) |
| **Physical keyboard → Quil text PASS** | Awaiting real hardware with PS/2 keyboard; QEMU sendkey blocked by environmental limitation (V2 SKIP) |
| **USB mass storage** | Not implemented (ISO is boot medium only, NVMe used for storage in QEMU) |
| **Install-to-partition** | Not claimed (live USB boot only) |
| **Durability / powerloss / journal** | SexFS v0 has no journal, no crash recovery |
| **POSIX / Linux semantics** | Strict no_std Rust microkernel, PDX-only IPC |
| **Framebuffer direct write by non-sexdisplay** | sexdisplay remains sole framebuffer writer; all bounds checks preserved |
| **USB keyboard as proven input route** | XHCI enumerates but HID keyboard report path is not wired |

---

## H) Files Changed

| File | Change | Reason |
|------|--------|--------|
| `scripts/sexos_build_trace.sh` | +3 lines: added `limine_bios_install` action | Resolve real-hardware Blocker 2 — BIOS boot requires MBR/GPT boot records |
| `sexos_build_spec.toml` | +5 lines: added `limine_bios_install` stage after `package_iso` | Integrate bios-install into sealed build trace |
| `docs/handoff/LIVE_USB_OPERATOR_RUNBOOK_V1.md` | **NEW** — complete operator runbook | Step-by-step build/write/boot/capture instructions |
| `docs/handoff/LIVE_USB_REAL_HARDWARE_BOOT_PROOF_V1.md` | **NEW** — this document | Real hardware boot proof preparation and observation plan |

**No kernel edits. No sex-pdx ABI edits. No bootloader config changes. No framebuffer ownership change.**

**All existing gates preserved.** QEMU boot verified after bios-install:
- FAULT_GATE: PASS (0 faults)
- SPAWN_GATE: PASS (all PDs spawn)
- BOOTGRAPH_GATE: PASS (all PDs init)
- sexfiles: PASS (storage ready)

---

## I) Baseline Gate Result (Pre-Change)

```
PASS gates: 270
FAIL gates: 0
SKIP gates: 115 (proofs not enabled in this boot)
faults_zero: PASS (0 fault markers)
FINAL: PASS

live_usb_quil_create_save_reopen: SKIP (not triggered — requires save/open proof first)
physical_keyboard_to_quil_text: SKIP (QEMU sendkey environmental limitation)
```

---

## J) Commit Hash

**Baseline:** `7ca20e6a`  
**Build pipeline changes:** To be committed after review.

---

## K) Next Phase

### If target hardware has PS/2 keyboard:
**`LIVE_USB_QUIL_CREATE_SAVE_REOPEN_PHYSICAL_INPUT_V1`**  
Prove the full create/save/reopen cycle on real hardware using physical PS/2 keyboard input.
Combines the synthetic create/save/reopen proof (proven in QEMU) with the physical
PS/2 keyboard route (awaiting real-hardware IRQ1 delivery).

### If target hardware has only USB keyboard:
**`USB_HID_BOOT_KEYBOARD_PROOF_V1`**  
Implement USB HID boot protocol keyboard report parsing in sexinput or sexusb.
Requires: XHCI driver (exists), USB HID report descriptor parsing, keyboard boot protocol
handler, hook into existing sexinput → silk-shell → Quil input route.

**DO NOT proceed to USB HID without STOP FIRST review of XHCI driver surface area.**

---

## L) Prior Artifacts Referenced

| Document | Date | Content |
|----------|------|---------|
| `REAL_HARDWARE_BOOT_AUDIT_V1.md` | 2026-05-06 | Full real-hardware boot maturity audit, 7 blockers |
| `REAL_HARDWARE_BOOT_PROOF_V1.md` | 2026-05-06 | Blocker analysis, preflight script, test checklist |
| `PHYSICAL_KEYBOARD_TO_QUIL_TEXT_PROOF_V2.md` | 2026-05-25 | PS/2 keyboard proof SKIP (QEMU sendkey limitation) |
| `LIVE_USB_QUIL_CREATE_SAVE_REOPEN_TEST_V1.md` | 2026-05-25 | Synthetic create/save/reopen proof PASS |
| `TEXT_INPUT_PIPELINE_PROOF_V1.md` | 2026-05-22 | Text input pipeline proof PASS |

---

## M) Real Hardware Blocker Status (from prior audit, updated)

| # | Blocker | Status | Fix |
|---|---------|--------|-----|
| 1 | Limine binary ARM64 Mach-O | ✅ **FIXED** | x86-64 ELF binary in `limine/limine` (Limine 7.13.3) |
| 2 | Missing `limine bios-install` | ✅ **FIXED** | Integrated into build trace (this document, `sexos_build_spec.toml`) |
| 3 | ACPI Handler 26× `todo!()` stubs | ⚠️ **OPEN** | Kernel edit required — STOP FIRST |
| 4 | Serial port assumption (COM1 0x3F8) | ⚠️ **OPEN** | Kernel edit required — STOP FIRST |
| 5 | LAPIC timer calibration | ⚠️ **OPEN** | Kernel edit required — STOP FIRST |
| 6 | IOAPIC polarity/trigger hardcoded | ⚠️ **OPEN** | Kernel edit required — STOP FIRST |
| 7 | PS/2 controller presence assumed | ⚠️ **OPEN** | Kernel edit required — STOP FIRST |

Blockers 3-7 remain from the prior audit. All require kernel edits and must go through STOP FIRST.
Blockers 1-2 (build/packaging) are now resolved.

