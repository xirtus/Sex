# LIVE USB OPERATOR RUNBOOK V1

**Date:** 2026-05-25  
**Purpose:** Exact steps to build and boot SexOS from USB on real hardware  
**Prerequisite Handoff:** `LIVE_USB_REAL_HARDWARE_BOOT_PROOF_V1.md`

---

## 1. BUILD THE ISO

```bash
cd /home/xirtus_arch/Documents/microkernel

# Single command — builds kernel + all servers + ISO in one sealed trace
./scripts/entrypoint_build.sh
```

**Expect:** Output ends with `[SEXOS ENTRYPOINT] success`.  
**Artifact:** `sexos-v1.0.0.iso` (~4.3 MB).

### Verify ISO (optional)
```bash
file sexos-v1.0.0.iso
# Expected: ISO 9660 CD-ROM filesystem data (DOS/MBR boot sector) 'ISOIMAGE' (bootable)

ls -la sexos-v1.0.0.iso
# Expected: ~4.3 MB
```

---

## 2. INSTALL LIMINE BOOTLOADER TO ISO (MANDATORY FOR REAL HARDWARE)

> ⚠️ **CRITICAL:** Without this step, the ISO will NOT boot on real hardware BIOS.
> QEMU boots via El Torito CD-ROM emulation and does not need this step.

```bash
./limine/limine bios-install sexos-v1.0.0.iso
```

**Expect:** No error output. Limine writes MBR/GPT boot records into the ISO.  
**Verify:** `strings sexos-v1.0.0.iso | grep LIMINE` should show "LIMINE" in the first sector.

---

## 3. PREFLIGHT CHECK ON TARGET MACHINE (OPTIONAL, RECOMMENDED)

Run the preflight script on the target hardware:

```bash
./scripts/real_hardware_preflight.sh
```

This checks:
- PKU feature flag (MPK/PKEY support)
- Serial port availability (COM1 at 0x3F8)
- RAM (≥512 MB)
- UEFI/BIOS firmware mode
- Secure Boot status
- USB XHCI controller presence
- ISO exists and contains Limine signature
- Limine tool exists and is x86-64

---

## 4. WRITE ISO TO USB STICK — ⚠️ DESTRUCTIVE ⚠️

### ⚠️ VERIFY TARGET DEVICE BEFORE PROCEEDING ⚠️

**You are about to overwrite ALL data on the target block device.**
**Double-check the device name. Triple-check. The wrong device = data loss.**

```bash
# 1) List block devices before inserting USB
lsblk -o NAME,SIZE,TYPE,MOUNTPOINT,MODEL | grep -v loop

# 2) Insert USB stick, then list again
lsblk -o NAME,SIZE,TYPE,MOUNTPOINT,MODEL | grep -v loop

# 3) Identify the new device — example: /dev/sdb (NOT sdb1, the whole device)
#    Verify: SIZE matches your USB stick capacity.
```

### Write command (replace `/dev/sdX` with the identified device):

```bash
sudo dd if=sexos-v1.0.0.iso of=/dev/sdX bs=4M status=progress conv=fsync
sync
```

- `bs=4M` — block size for speed
- `status=progress` — shows transfer progress
- `conv=fsync` — flushes write cache before exit
- `sync` — final filesystem sync

**Example (DO NOT COPY BLINDLY):**
```bash
sudo dd if=sexos-v1.0.0.iso of=/dev/sdb bs=4M status=progress conv=fsync
```

---

## 5. BOOT FROM USB

### BIOS/Legacy Boot (CSM)
1. Enter firmware setup (F2/DEL/F12/ESC at power-on — varies by motherboard)
2. **Disable Secure Boot**
3. **Enable Legacy/CSM boot** (if available)
4. Set USB first in boot order
5. Save & Exit → machine reboots from USB

### UEFI Boot
1. Enter firmware setup
2. **Disable Secure Boot**
3. Boot from "UEFI: <USB device name>"
4. Limine UEFI boot should present `SexOS` menu entry

### Boot Menu
- **TIMEOUT=3** (3 seconds before auto-boot)
- **Menu entry:** `SexOS` (auto-selected)
- Press any key during timeout to see menu
- Expected: Kernel boots, display initializes, purple framebuffer appears

---

## 6. CAPTURE SERIAL OUTPUT

SexOS writes debug output to COM1 (0x3F8, 115200 baud). To capture:

### Option A: Serial port on target machine
```bash
# On target machine (Linux live USB):
screen /dev/ttyS0 115200
```
Or with picocom: `picocom -b 115200 /dev/ttyS0`

### Option B: USB-serial adapter (null-modem cable)
```bash
# On observer machine:
screen /dev/ttyUSB0 115200
```

### Option C: No serial port (fallback)
If the target machine has no serial port:
- SexOS will still boot but serial debug output will fail
- Visual confirmation via framebuffer only (purple screen, UI appears)
- **Known issue:** Serial output uses `.expect()` — if COM1 absent, kernel panics
  (see Blocker 4 in `REAL_HARDWARE_BOOT_PROOF_V1.md`)

---

## 7. EXPECTED OBSERVABLE MARKERS

After successful boot, look for these markers in serial output:

### Minimum boot confirmation
```
[live_usb.real_boot.begin]
[live_usb.real_boot.kernel.start]
```

### PD spawn confirmation
```
[live_usb.real_boot.pd.spawn] name=sexdisplay ok=1
[live_usb.real_boot.pd.spawn] name=sexfiles ok=1
[live_usb.real_boot.pd.spawn] name=quil ok=1
[live_usb.real_boot.display.ready] ok=1
```

### Physical keyboard route check (if PS/2/i8042 keyboard present)
```
[live_usb.real_boot.keyboard.route] ps2=0_or_1 usb=0_or_1 honest=1
[live_usb.real_boot.no_faults] ok=1
[live_usb.real_boot.done] ok=1
```

### If no PS/2 keyboard present (modern laptops)
```
[live_usb.real_keyboard.skip] reason=usb_hid_not_yet_implemented ok=1
```

**Minimum happy-path:** Framebuffer initializes, kernel doesn't panic, PDs spawn.

---

## 8. FAULT SCAN

Watch for these in serial or framebuffer output:

| Marker | Meaning |
|--------|---------|
| `#PF` | Page fault — crash |
| `#GP` | General protection fault — crash |
| `panic` | Kernel panic — crash |
| `PKU violation` | Protection key violation — isolation breach |

If any fault appears: **STOP**. Note exact marker, fill fault section in handoff doc.
Do NOT re-attempt until root cause is understood.

---

## 9. TROUBLESHOOTING

| Symptom | Likely Cause | Check |
|---------|-------------|-------|
| No boot at all (black screen) | `limine bios-install` not run | Re-run step 2, rewrite USB |
| No boot (black screen, UEFI) | Secure Boot enabled | Disable Secure Boot in firmware |
| Limine menu appears, kernel doesn't start | Serial port assumption panicked | Check serial output, see Blocker 4 |
| Boot starts but hangs | PS/2 init on machine with i8042 disabled | See Blocker 7 |
| Boots but no display | VIDEO_MODE=1280x720 not supported by GPU | Try lower resolution in limine.cfg |
| Boots but scheduler starvation | LAPIC timer not calibrated for this CPU | See Blocker 5 |
| IRQ storm | IOAPIC polarity/trigger mismatch | See Blocker 6 |
| ACPI panic | DSDT/SSDT parsed stubbed handler | See Blocker 3 |

---

## 10. POST-BOOT: CLEAN UP USB

After testing, the USB stick can be reformatted for normal use:

```bash
# Wipe partition table (replace /dev/sdX)
sudo wipefs -a /dev/sdX

# Create new partition table and format
sudo parted /dev/sdX mklabel gpt
sudo parted /dev/sdX mkpart primary fat32 0% 100%
sudo mkfs.vfat -F32 /dev/sdX1
```

---

## 11. REFERENCE

- **Build prescription:** `sexos_build_spec.toml` (15 packages, sealed trace)
- **Boot config:** `limine.cfg` (SERIAL=yes, VIDEO_MODE=1280x720,32)
- **Kernel boot flow:** See `REAL_HARDWARE_BOOT_AUDIT_V1.md` Appendix B
- **Known blockers:** `REAL_HARDWARE_BOOT_PROOF_V1.md` Section 3
- **Preflight script:** `scripts/real_hardware_preflight.sh`
- **ISO artifact:** `sexos-v1.0.0.iso` (El Torito + EFI hybrid, needs `limine bios-install`)
