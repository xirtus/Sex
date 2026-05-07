# REAL_HARDWARE_BOOT_PROOF_V1

- date: 2026-05-06
- baseline HEAD: `90b202cb49259d7d0cbb08e98c78b07507b03a1a`
- scope: Real-hardware boot readiness (BIOS + UEFI)
- previous audit: `REAL_HARDWARE_BOOT_AUDIT_V1.md`
- preflight script: `scripts/real_hardware_preflight.sh`

## 1. Build & QEMU Runtime

```
./scripts/entrypoint_build.sh    → PASS
./scripts/master_runtime_gate.sh --probe 25 → GREEN_MASTER
```

QEMU boot (El Torito CD-ROM emulation) continues to work flawlessly.
Real hardware boot requires additional steps documented below.

## 2. PASS/WARN/FAIL Table

### Boot Path

| Component | Status | Detail |
|-----------|--------|--------|
| Limine bootloader binaries | ✅ PASS | EFI files (x64, IA32, AA64, RISCV64) present; BIOS .sys present |
| Bootloader config (limine.cfg) | ✅ PASS | Correct PROTOCOL, KERNEL_PATH, 12 MODULE_PATH entries, VIDEO_MODE |
| Kernel binary placement | ✅ PASS | iso_root/sexos-kernel → x86_64 ELF |
| Server module placement | ✅ PASS | All 12 modules under iso_root/servers/ and iso_root/apps/ |
| ISO creation (xorriso) | ✅ PASS | El Torito + EFI hybrid ISO created |
| Limine bios-install in pipeline | ❌ **FAIL** | `limine bios-install sexos-v1.0.0.iso` is MISSING from `sexos_build_trace.sh` and build spec |
| Limine binary architecture | ❌ **FAIL** | `limine/limine` is **ARM64 Mach-O**; cannot run on x86_64 Linux dev machine |

### CPU Features

| Feature | Status | Detail |
|---------|--------|--------|
| x86-64 | ✅ PASS | Required by target spec |
| PKU (CR4.PKE) | ✅ PASS | Detected at runtime; graceful degradation if absent |
| PKE/MPK enable | ✅ PASS | Enabled in HAL init |
| XSAVE | 🟡 N/A | Not used (soft-float, manual wrpkru in context switch) |
| APIC (LAPIC+IOAPIC) | ✅ PASS | Discovered via ACPI MADT from Limine RSDP |
| HPET/TSC calibration | 🟡 **WARN** | LAPIC timer uses hardcoded 1,000,000 count; varies 25-400+ MHz on real CPUs |

### Hardware Assumptions

| Assumption | Risk | Status | Detail |
|------------|------|--------|--------|
| Serial port at 0x3F8 (COM1) | **HIGH** | 🟡 **WARN** | Hardcoded; `.expect()` panics if port absent |
| PS/2 controller present | MEDIUM | 🟡 **WARN** | Init writes to 0x60/0x64 without probe; may hang on i8042-disabled firmware |
| PCI legacy I/O (0xCF8/0xCFC) | LOW | ✅ PASS | Universal on x86-64 |
| ACPI RSDP from Limine | LOW | ✅ PASS | Limine provides valid RSDP |
| IOAPIC active-high/edge | MEDIUM | 🟡 **WARN** | Hardcoded; real hw may need level-triggered or active-low |
| Framebuffer from Limine | LOW | ✅ PASS | Limine guarantees FB per VIDEO_MODE=1280x720,32 |
| LAPIC timer 100MHz | MEDIUM | 🟡 **WARN** | Fixed count; timer fires at wrong rate on real CPUs |

### Critical ACPI Handler Stubs

| Stub | Count | Impact |
|------|-------|--------|
| ACPI Handler `todo!()` methods | **26** | If ACPI AML calls `read_u8()`, `stall()`, `sleep()`, `create_mutex()`, etc., kernel panics. Currently not triggered in QEMU (ACPI parser uses only `map_physical_region`). On real hardware, DSDT/SSDT iteration may trigger these. |

## 3. Exact Hardware Blockers

### Blocker 1: Limine Binary Architecture Mismatch (NEW — not in prior audit)
- **File**: `limine/limine`
- **Issue**: Binary is `Mach-O 64-bit arm64`; dev machine is `x86_64 Linux`
- **Symptom**: `Exec format error` — `limine bios-install` cannot run
- **Fix available**: Copy x86_64 ELF binary from `microkernel_nightly/limine_bin/limine` (Limine 7.13.3, confirmed working)
- **Severity**: **CRITICAL** — blocks BIOS boot on any hardware

### Blocker 2: Missing `limine bios-install` in Build Pipeline
- **File**: `scripts/sexos_build_trace.sh`, `sexos_build_spec.toml`
- **Issue**: ISO created via xorriso but Limine MBR/GPT boot records not installed
- **Impact**: ISO will not boot on real hardware via BIOS. QEMU works via El Torito emulation only.
- **Fix needed**: Add `limine_bios_install` stage after `package_iso` in build spec
- **Severity**: **CRITICAL** — blocks BIOS boot on any hardware

### Blocker 3: ACPI Handler 26× `todo!()` Stubs
- **File**: `kernel/src/apic.rs: SexAcpiHandler` (and `kernel/src/hw/init.rs`)
- **Issue**: If ACPI table parsing triggers any of the stubbed methods, kernel panics
- **Current safety**: QEMU's ACPI tables are simple enough to not trigger these. Real hardware DSDT/SSDT may.
- **Fix needed**: Kernel edit — implement or stub-as-noop the critical methods (`stall`, `sleep`, `read_io_*`, `write_io_*`)
- **Severity**: **HIGH** — potential panic on real hardware

### Blocker 4: Serial Port Assumption
- **File**: `kernel/src/serial.rs`
- **Issue**: Hardcoded COM1 at 0x3F8; `.expect()` on write failure panics
- **Fix needed**: Kernel edit — probe port before init, fallback to no-op logger
- **Severity**: **HIGH** — panic on machines without serial (most modern laptops)

### Blocker 5: LAPIC Timer Calibration
- **File**: `kernel/src/apic.rs:232`
- **Issue**: Hardcoded 1,000,000 count (assumes ~100MHz LAPIC, produces ~1ms ticks)
- **Impact**: Timer fires at wrong rate; scheduler cadence wrong; tasks starve or overload
- **Fix needed**: Kernel edit — calibrate against PIT/HPET/ACPI PM timer
- **Severity**: **MEDIUM** — system boots but scheduler cadence incorrect

### Blocker 6: IOAPIC Polarity/Trigger Hardcoded
- **File**: `kernel/src/apic.rs: map_irq`
- **Issue**: Hardcoded active-high/edge; real hardware often needs level-triggered or active-low
- **Impact**: Interrupts may not fire or may fire continuously (IRQ storm)
- **Fix needed**: Kernel edit — read MADT interrupt source overrides
- **Severity**: **MEDIUM** — affects interrupt delivery for PS/2, PCI devices

### Blocker 7: PS/2 Keyboard Init Without Probe
- **File**: `kernel/src/keyboard.rs`
- **Issue**: Writes to ports 0x60/0x64 without checking if i8042 exists
- **Impact**: May hang on hardware with i8042 disabled in firmware (common on modern UEFI systems)
- **Fix needed**: Kernel edit — probe 0x64 before init
- **Severity**: **LOW** — on systems with i8042 disabled, USB HID is the primary input path

## 4. Exact Safe Next Patch (Blocker 1+2 only — no kernel edits)

### Step 1: Replace Limine Binary

```bash
# Copy x86_64 ELF limine binary to replace ARM64 Mach-O
cp limine/limine limine/limine.arm64.bak
cp ../microkernel_nightly/limine_bin/limine limine/limine
chmod +x limine/limine
```

**No kernel edit. No ABI change. Non-destructive.** The ARM64 backup is preserved.

### Step 2: Add `limine_bios_install` to Build Spec

Add to `sexos_build_spec.toml` (after `package_iso` stage):
```toml
[[stage]]
id = "limine_bios_install"
action = "limine_bios_install"
```

Add to `scripts/sexos_build_trace.sh` (in `run_stage()` switch):
```bash
limine_bios_install)
    ./limine/limine bios-install sexos-v1.0.0.iso
    ;;
```

**No kernel edit. No ABI change. Non-destructive.** This only adds boot records to the ISO; QEMU continues to work via El Torito.

### Step 3: Rebuild and Verify

```bash
./scripts/entrypoint_build.sh
./scripts/master_runtime_gate.sh --probe 25
# Expected: GREEN_MASTER (QEMU boot unchanged)
```

## 5. Manual Real-Machine Test Checklist

### Prerequisites
- [ ] x86-64 machine with PKU (Intel 10th-gen+ or AMD Zen 3+)
- [ ] Serial port (COM1 at 0x3F8) or USB-serial adapter → null-modem cable
- [ ] USB flash drive ≥1 GB or CD-RW
- [ ] QEMU on dev machine for pre-test validation

### Steps

```bash
# 1. Verify PKU on test machine
grep pku /proc/cpuinfo

# 2. Verify serial port
cat /proc/tty/driver/serial  # should show 0x3F8
# OR: dmesg | grep tty

# 3. Build ISO with bios-install
./scripts/entrypoint_build.sh

# 4. Verify QEMU boot
./scripts/master_runtime_gate.sh --skip-build --probe 25
# Must show: GREEN_MASTER

# 5. Write ISO to USB
sudo dd if=sexos-v1.0.0.iso of=/dev/sdX bs=4M status=progress
sync

# 6. Boot from USB
#    - Enter BIOS, disable Secure Boot
#    - Set Legacy/CSM boot (or UEFI — Limine supports both)
#    - Boot from USB

# 7. Capture serial output
screen /dev/ttyUSB0 115200
# Expected markers (minimum):
#   "X86Hal: Initializing foundation (BSP)..."
#   "PKU: Protection Keys enabled in CR4."
#   "✓ Spawned PD 1: ...sexdisplay..."
#   "sexfiles.ready"

# 8. Fallback if no serial output
#    - Check Limine TIMEOUT=3: press key during boot to see menu
#    - Try UEFI boot instead of Legacy
#    - Verify BIOS serial port enabled (SuperIO config)
#    - Test USB with known-good Linux ISO
```

## 6. Preflight Script Status

`scripts/real_hardware_preflight.sh` exists and covers:
- CPU features (PKU, x86-64)
- Serial port (ttyS0, USB-serial)
- Memory (≥512MB)
- Firmware (UEFI/BIOS, Secure Boot)
- USB controller (XHCI)
- ISO existence and Limine signature
- Limine tool presence
- UEFI boot files
- Build target spec

**Update needed**: Add Limine binary architecture check (ELF x86-64 vs Mach-O ARM64).

## 7. Blocker 3-7 Summary (Kernel Edits — STOP FIRST)

All remaining blockers (ACPI stubs, serial port, LAPIC timer, IOAPIC polarity, PS/2 probe) require kernel edits. Each must go through the STOP FIRST gate. These are documented in `REAL_HARDWARE_BOOT_AUDIT_V1.md` and remain unchanged from that audit.

### Priority Order (safest first):

1. **Serial port probe** (kernel/src/serial.rs) — smallest change, highest bang-for-buck
2. **ACPI handler stubs** (kernel/src/apic.rs) — replace `todo!()` with no-ops for methods that are safe to stub
3. **LAPIC timer calibration** (kernel/src/apic.rs) — calibrate against PIT channel 2
4. **IOAPIC polarity override** (kernel/src/apic.rs) — read MADT ISO entries
5. **PS/2 controller probe** (kernel/src/keyboard.rs) — check 0x64 before init

## 8. Files Changed by This Audit

| File | Change |
|------|--------|
| `docs/handoff/REAL_HARDWARE_BOOT_PROOF_V1.md` | **NEW** — this document |
| `limine/limine.x86_64` | **NEW** — x86_64 ELF Limine binary (from nightly repo) |

No kernel edits. No ABI changes. No bootloader config changes.
No destructive disk operations.

## 9. Verdict

**PASS** for QEMU (El Torito CD-ROM boot).  
**FAIL** for real hardware BIOS boot until Blocker 1+2 are resolved.  
**WARN** for real hardware UEFI boot (Limine EFI boot may work without bios-install, but not tested).  
**WARN** for general real-hardware stability (ACPI stubs, serial assumption, timer calibration).

### Immediate Action (no STOP FIRST):
- [ ] Copy x86_64 limine binary to `limine/limine`
- [ ] Add `limine_bios_install` stage to build pipeline
- [ ] Rebuild and verify QEMU gate still GREEN_MASTER
- [ ] Run `scripts/real_hardware_preflight.sh` on test machine

### Next Action (STOP FIRST — kernel edit):
- [ ] Implement serial port existence probe and no-op fallback logger
