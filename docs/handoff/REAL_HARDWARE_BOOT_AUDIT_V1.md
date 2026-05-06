# REAL_HARDWARE_BOOT_AUDIT_V1

**Date:** 2026-05-06  
**Scope:** Real-hardware boot maturity audit for SexOS microkernel  
**Method:** Static analysis of kernel init, HAL, bootloader config, ISO creation, and runtime scripts  

---

## 1. Current Real-Hardware Readiness Summary

### 1.1 Boot Path

| Component | Status | Details |
|-----------|--------|---------|
| Bootloader | ✅ | Limine 7.x cloned, binaries present (`limine/`) |
| Bootloader config | ✅ | `limine.cfg` with correct PROTOCOL=limine, KERNEL_PATH, MODULE_PATH entries |
| ISO creation | ⚠️ PARTIAL | xorriso creates El Torito ISO but `limine bios-install` step is **MISSING** |
| Kernel binary | ✅ | Correctly placed at `iso_root/sexos-kernel` |
| Modules | ✅ | All 10 servers/apps placed under `iso_root/servers/` and `iso_root/apps/` |
| Limine install | ❌ **MISSING** | `limine bios-install sexos-v1.0.0.iso` is NOT called in `sexos_build_trace.sh` |

### 1.2 Required CPU Features

| Feature | Status | Details |
|---------|--------|---------|
| x86-64 | ✅ | Required by target `x86_64-unknown-none` |
| PKU (CR4.PKE) | ✅ | Checked at runtime via `raw_cpuid`, graceful degradation |
| PKE (MPK) | ✅ | Enabled in HAL `init()` via `Cr4Flags::PROTECTION_KEY_USER` |
| XSAVE | 🟡 NOT USED | Target disables SSE/MMX (`-mmx,-sse,-sse2,+soft-float`); PKRU saved/restored manually in task context via wrpkru. No XSAVE required. |
| APIC | ✅ | Discovered via ACPI/RSDP from Limine |
| IOAPIC | ✅ | Enumerated from MADT |
| HPET/TSC | 🟡 WEAK | LAPIC timer uses hardcoded 1,000,000 count (~1ms at 100MHz) — needs calibration |

### 1.3 Hardware Assumptions

| Assumption | Risk | Details |
|------------|------|---------|
| Serial port at 0x3F8 (COM1) | HIGH | Hardcoded in `kernel/src/serial.rs`; `.expect()` on write failure will panic on hardware without serial |
| PS/2 controller present | MEDIUM | Keyboard init writes to ports 0x60/0x64; real hardware may have i8042 disabled by firmware |
| PCI legacy I/O (0xCF8/0xCFC) | LOW | Works on all x86-64 hardware |
| ACPI RSDP valid | LOW | Provided by Limine; kernel converts to physical address |
| IOAPIC active-high/edge | MEDIUM | Hardcoded polarity/trigger; real hardware may need level-triggered or active-low |
| Framebuffer at Limine-provided address | LOW | Limine guarantees valid framebuffer per VIDEO_MODE request |
| LAPIC timer at ~100MHz | MEDIUM | Fixed count 1,000,000; varies widely (25-400+ MHz on real CPUs) |

### 1.4 Known Blockers (Critical)

| # | Blocker | Component | Impact |
|---|---------|-----------|--------|
| 1 | **Missing `limine bios-install` after ISO creation** | `sexos_build_trace.sh` | **ISO will not boot on real hardware BIOS. QEMU may work via El Torito emulation.** |
| 2 | **ACPI Handler `todo!()` stubs** | `kernel/src/apic.rs: SexAcpiHandler` | If ACPI parsing calls `read_u8()`, `stall()`, `sleep()`, or `create_mutex()`, the kernel will panic |
| 3 | **No USB mass storage / NVMe driver for rootfs** | userspace | System has no persistence layer for real hardware storage |
| 4 | **No real root filesystem** | userspace | initrd/modules only; no way to load additional binaries from disk |
| 5 | **Serial-only debug output** | `kernel/src/serial.rs` | No VGA/text-mode fallback; if serial is absent, kernel panics |

---

## 2. ISO/USB Creation Path

### Current Flow (`sexos_build_trace.sh`)

```
prep_iso_root  →  copy_limine  →  cargo build (kernel + servers)  →  package_iso
```

**The `package_iso` stage:**
```bash
xorriso -as mkisofs -R -r -J \
  -b boot/limine/limine-bios-cd.bin -no-emul-boot -boot-load-size 4 -boot-info-table \
  --efi-boot boot/limine/limine-uefi-cd.bin -efi-boot-part --efi-boot-image --protective-msdos-label \
  iso_root -o sexos-v1.0.0.iso
```

**MISSING step (must be added):**
```bash
./limine/limine bios-install sexos-v1.0.0.iso
```

### QEMU Boot Command (from `dev.sh`, `qemuX.sh`, `master_runtime_gate.sh`)
```
qemu-system-x86_64 -M q35 -m 512M -cpu max,+pku -cdrom sexos-v1.0.0.iso \
  -device nec-usb-xhci,id=xhci -device usb-tablet,bus=xhci.0 \
  -serial stdio -display none
```

---

## 3. Bootloader Configuration (`limine.cfg`)

```
TIMEOUT=3
SERIAL=yes

:SexOS
    PROTOCOL=limine
    KERNEL_PATH=boot:///sexos-kernel
    MODULE_PATH=boot:///servers/sexdisplay
    MODULE_PATH=boot:///servers/silk-shell
    MODULE_PATH=boot:///apps/sexdrive
    MODULE_PATH=boot:///servers/sexinput
    MODULE_PATH=boot:///servers/sexusb
    MODULE_PATH=boot:///servers/silkbar
    MODULE_PATH=boot:///servers/linen
    MODULE_PATH=boot:///servers/sexstore
    MODULE_PATH=boot:///servers/quil
    MODULE_PATH=boot:///servers/sexbell
    MODULE_PATH=boot:///apps/purple-scanout
    VIDEO_MODE=1280x720,32
```

**Observations:**
- `SERIAL=yes` enables Limine's serial output — good for debugging
- `VIDEO_MODE=1280x720,32` requests 1280x720 resolution at 32bpp — may fail on some hardware
- All 10 modules loaded via `MODULE_PATH` — missing module causes silent skip during spawn
- No `KASLR` or `SMAP`/`SMEP` flags — intentionally minimal for SASOS model

---

## 4. Required CPU Feature Verification

### PKU/PKE Check (in `kernel/src/hal/x86_64.rs`)
```rust
if crate::pku::is_pku_supported() {
    unsafe { crate::pku::enable_pku(); }
    crate::pku::set_pku_enabled(true);
} else {
    serial_println!("PKU: unsupported; PKRU ops gated off");
}
```

This is A CORRECT graceful degradation. PKU is optional.

### PKRU Save/Restore (in scheduler)
- PKRU is stored in `TaskContext.pkru` at offset 0x80
- Restored in context-switch assembly: `wrpkru` with value from `[rsi + 0x80]`
- No XSAVE/XRSTOR used — manually managed (valid since no SSE/AVX state)

### XSAVE Check
**Not required.** Target spec disables SSE/MMX:
```json
"features": "-mmx,-sse,-sse2,+soft-float"
```

---

## 5. Known Real-Hardware Pitfalls

### 5.1 LAPIC Timer Calibration
```rust
// kernel/src/apic.rs:232
lapic_ptr.offset(0x380 / 4).write_volatile(1000000);
```
Hardcoded to 1,000,000 cycles. LAPIC timer frequency is CPU-specific:
- Intel: usually ~1/3 of bus clock (varies by microarchitecture)
- AMD: usually ~1/2 of core clock
- Range: ~25 MHz to 400+ MHz

**Impact:** Timer interrupts fire at wrong frequency — too fast or too slow.

### 5.2 ACPI Handler Stubs
```rust
// kernel/src/apic.rs:31-56
fn read_u8(&self, _: usize) -> u8 { todo!() }
fn stall(&self, _: u64) { todo!() }
fn create_mutex(&self) -> acpi::Handle { todo!() }
```
If any ACPI table parsing code calls these (e.g., during DSDT/SSDT iteration), the kernel panics.

### 5.3 Serial Port Assumption
```rust
// kernel/src/serial.rs:8
static ref SERIAL1: Spinlock<SerialPort> = {
    let mut serial_port = unsafe { SerialPort::new(0x3F8) };
    serial_port.init();
    Spinlock::new(serial_port)
};
```
- Assumes 0x3F8 (COM1) is present
- `init()` writes to I/O ports that may not exist
- All `serial_println!()` calls use `.expect("Printing to serial failed")` — panic if port missing

### 5.4 PCI Enumeration Consistency
Two different PCI enumeration implementations exist:
1. `kernel/src/hal/pci.rs` — scans buses 0..255, slots 0..31, functions 0..7 (FULL)
2. `kernel/src/drivers/pci.rs` — scans buses 0..8 only (RESTRICTED, used by bootstrap_drivers)

**Risk:** `bootstrap_drivers()` (used for early GPU discovery) may miss devices on bus > 7.

### 5.5 PS/2 Keyboard Init Without Check
```rust
// kernel/src/keyboard.rs
pub fn init() {
    unsafe {
        cmd_port.write(0x20); // Read command byte
        // ...
        cmd_port.write(0xAE); // Enable keyboard port
        data_port.write(0xF4); // Enable scanning
    }
}
```
- No check if PS/2 controller exists (`0x64` read returns 0xFF on missing controller)
- On hardware with i8042 disabled in firmware, this may hang

---

## 6. Files Changed and Rationale

This audit is **documentation only** — no code changes made.

| File | Type | Purpose |
|------|------|---------|
| `docs/handoff/REAL_HARDWARE_BOOT_AUDIT_V1.md` | NEW | This document |
| `scripts/real_hardware_preflight.sh` | NEW | Preflight checklist script for hardware boot testing |

---

## 7. Build and Runtime Result

- **Build:** Not retested (no code changes)
- **QEMU runtime gate:** Unchanged, expected to pass (no kernel edits)
- **Proof:** All analysis is static/documentation only

---

## 8. Exact Hardware Test Checklist

### Prerequisites
- [ ] x86-64 machine with PKU support (Intel 10th-gen+ or AMD Zen 3+)
- [ ] Known-good serial port (COM1 at 0x3F8) connected via USB-serial adapter
- [ ] USB flash drive (≥1 GB) or CD-RW
- [ ] QEMU on development machine for pre-test validation

### Step-by-Step

```bash
# 1. Preflight check on test machine
#   - Confirm PKU in /proc/cpuinfo: grep pku /proc/cpuinfo
#   - Confirm serial port available: cat /proc/tty/driver/serial

# 2. Rebuild ISO
./scripts/entrypoint_build.sh

# 3. Install Limine bootloader to ISO (REQUIRED for real hardware)
./limine/limine bios-install sexos-v1.0.0.iso

# 4. Verify ISO boots in QEMU
./scripts/master_runtime_gate.sh --skip-build

# 5. Write ISO to USB (replace /dev/sdX with actual device)
#    sudo dd if=sexos-v1.0.0.iso of=/dev/sdX bs=4M status=progress
#    sync

# 6. Boot from USB on test machine
#    - Enter BIOS, disable Secure Boot
#    - Set boot order: USB first (Legacy/CSM mode, not UEFI)
#    - OR: use UEFI boot (Limine supports both)

# 7. Capture serial output
#    - Connect USB-serial adapter to COM1 (typically DB9 or virtual)
#    - screen /dev/ttyUSB0 115200
#    - OR: set up serial console in firmware

# 8. Expected serial markers
#    - "X86Hal: Initializing foundation (BSP)..."
#    - "PKU: Protection Keys enabled in CR4."
#    - "Spawned PD 1: sexdisplay"
#    - "init: FB handed to sexdisplay..."
#    - "MASTER RUNTIME GATE - GREEN_MASTER"
```

### Fallback Debugging
```bash
# If no serial output:
#   1. Check USB-serial adapter is detected: dmesg | grep ttyUSB
#   2. Verify baud rate: stty -F /dev/ttyUSB0 115200 raw
#   3. Try Limine boot menu: press ESC during TIMEOUT=3 (see limine.cfg)
#   4. Test with known-working Linux: dd linux.iso to same USB to verify hardware
```

---

## 9. Next Safest Hardware Patch

### Priority 1: Add `limine bios-install` to ISO Creation

**File:** `scripts/sexos_build_trace.sh`  
**Change:** Add a `limine_bios_install` action after `package_iso`:

```bash
# New action in run_stage():
limine_bios_install)
  ./limine/limine bios-install sexos-v1.0.0.iso
  ;;
```

And add to build spec:
```toml
[[stage]]
id = "limine_bios_install"
action = "limine_bios_install"
```

**STOP FIRST if:** This change could break QEMU boot (it should not — El Torito boot remains intact).

### Priority 2: Serial Port Existence Check

**File:** `kernel/src/serial.rs`  
**Change:** Before initializing, probe 0x3F8 to verify serial port exists. If absent, skip initialization and provide a fallback no-op logger.

**STOP FIRST:** This requires a kernel edit. Do not proceed without handoff approval.

### Priority 3: Hardware Preflight Script

**File:** `scripts/real_hardware_preflight.sh` (NEW)  
**Content:** Runtime checks for PKU support, serial port, memory map boundaries, and framebuffer sanity — log-only, no structural changes.

**Safe to implement:** Yes — no kernel changes, no bootloader changes.

---

## 10. Appendices

### A. Target Specification (`x86_64-sex.json`)
```json
{
  "llvm-target": "x86_64-unknown-none",
  "features": "-mmx,-sse,-sse2,+soft-float",
  "disable-redzone": true,
  "panic-strategy": "abort",
  "code-model": "kernel"
}
```

### B. Kernel Boot Flow
```
Limine BIOS/EFI
  ↓
_start() [kernel/src/main.rs]
  ↓
kernel_init() [kernel/src/lib.rs]
  ├── hal::init()           → GDT, IDT, PKU
  ├── memory::manager::init() → heap, page tables
  ├── core_local::init()    → BSP CoreLocal
  ├── hal::init_advanced()  → APIC, IOAPIC, timer
  ├── keyboard::init()      → PS/2 init
  ├── init::init()          → spawn PDs, grant caps, hand FB
  │   ├── select_primary_gpu()
  │   ├── Spawn PDs 1-10
  │   ├── Grant capabilities
  │   ├── devmgr::init()    → PCI enumeration, device lease
  │   └── FB remap + handoff to sexdisplay
  ├── scheduler bind + tick
  └── → context switch to PD1 (sexdisplay)
```

### C. Complete Module/Server Inventory (ISO Contents)
```
iso_root/
├── sexos-kernel            (kernel binary)
├── limine.cfg              (bootloader config)
├── servers/
│   ├── sexdisplay          (PD1 - framebuffer compositor)
│   ├── silk-shell          (PD3 - window manager / shell)
│   ├── sexinput            (PD4 - input router)
│   ├── sexusb              (PD5 - USB host driver)
│   ├── silkbar             (PD6 - status bar)
│   ├── linen               (PD7 - file browser)
│   ├── sexstore            (PD8 - key-value store)
│   ├── quil                (PD9 - application launcher)
│   └── sexbell             (PD10 - notification daemon)
├── apps/
│   ├── sexdrive            (PD2 - NVMe driver)
│   └── purple-scanout      (diagnostic overlay)
└── boot/limine/
    ├── limine-bios-cd.bin
    ├── limine-uefi-cd.bin
    └── limine-bios.sys
```

