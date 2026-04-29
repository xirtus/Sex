# HANDOFF.md — v8 SilkBar PDX Integration (Current Phase)

## Current Runtime State ✅

- **All 5 PDs spawn and run** (sexdisplay, sexdrive, silk-shell, sexinput, silkbar).
- **Zero page faults, zero panics.** The `static`→`const` fix resolved the GOT relocation crash.
- **sexdisplay receives OP_PRIMARY_FB** (syscall 28 returns status=0x11) **and OP_SILKBAR_UPDATE** (0xF2).
- **Scheduler round-robins all 5 PDs** with no stalls.
- **Known: screen may still appear black** — see diagnosis below.

## Two Bugs Fixed This Session

### Bug 1: Cross-crate `static` → GOT not relocated (resolved ✅)

**Root cause:** Kernel ELF loader (`kernel/src/elf.rs`) copies PT_LOAD segments but does NOT process `.rela.dyn` entries. Cross-crate `pub static` items in PIC/PIE binaries produce GOT entries with unrelocated addresses. When code dereferences through the GOT, it reads a pre-relocation address → page fault in user mode.

**Fix:** Changed cross-crate data to `const`:
- `crates/silkbar-model/src/lib.rs`: `pub static DEFAULT_SILK_BAR` → `pub const`, `pub static DEFAULT_THEME` → `pub const`
- `servers/sexdisplay/src/main.rs`: `static DIGITS` → `const DIGITS`

**Evidence:** 5817 repeated page faults before fix → zero after fix.

### Bug 2: Slot 0 message ring never delivered (fixed this session ✅)

**Root cause:** `pdx_listen_raw(0)` calls syscall 28 with `rdi=0`. The kernel did `find_capability(0)` → `CapabilityTable::find(0)` rejects id=0 (1-indexed guard). Returns `None` → falls to `_ => (0,0,0,0,0)` = EMPTY forever.

The `ProtectionDomain::new()` comment says "Slot 0 is ALWAYS the PD's own message ring" but `grant_capability(0, MessageQueue)` calls `insert_at(0, ...)` which also rejects id=0 → early return. The cap was **never inserted**.

**Fix in `kernel/src/syscalls/mod.rs` (syscall 28 handler):**
After checking the reply buffer for slot 0, fall through to `current_pd.message_ring->dequeue()` directly instead of going through the capability table. Non-zero slots still use capability lookup.

**File changed:** `kernel/src/syscalls/mod.rs`

### Bug 3 (cosmetic): `limine.cfg` missing silkbar module (fixed this session ✅)

Added `MODULE_PATH=boot:///servers/silkbar` to `limine.cfg`. Without this, Limine didn't load the silkbar ELF and PD 5 was never spawned.

**File changed:** `limine.cfg`

## Black Screen Diagnosis (not yet fixed)

Sexdisplay DOES receive OP_PRIMARY_FB and DOES render (inline memory writes to the framebuffer). But the QEMU display remains black. Potential causes (in order of likelihood):

1. **Framebuffer PKEY mismatch** — FB pages are mapped with their original PKEY (likely 0). Sexdisplay's PKRU allows PKEY 0 access, so this should be fine. But verify via page table walk / PTE dump.
2. **Framebuffer address is in kernel higher-half (`0xffff8000...`)** — Kernel remapped with USER_ACCESSIBLE flag, but all 4 page-table levels need the User flag for ring-3 access. If upper-level entries lack User flag, ring-3 write fails silently (no #PF because PKU blocks access differently).
3. **Render function writes wrong colors** — Very dark palette could appear black on some QEMU configurations.
4. **QEMU display refresh** — Serial-only mode doesn't show a graphical window. The framebuffer contents are correct but not visible without `-vga std` or `-display gtk`.

## Files Changed This Session

| File | Change |
|------|--------|
| `kernel/src/syscalls/mod.rs` | Slot 0 message ring bypass in syscall 28 |
| `limine.cfg` | Added silkbar module path |
| `crates/silkbar-model/src/lib.rs` | `static` → `const` (previous session) |
| `servers/sexdisplay/src/main.rs` | `static` → `const` (previous session) |

## Build & Run

```bash
# Full rebuild
./build_payload.sh
cp target/x86_64-sex/release/sex-kernel iso_root/sexos-kernel
rm -f sexos-v1.0.0.iso
xorriso -as mkisofs -R -r -J \
  -b boot/limine/limine-bios-cd.bin -no-emul-boot -boot-load-size 4 -boot-info-table \
  --efi-boot boot/limine/limine-uefi-cd.bin -efi-boot-part --efi-boot-image --protective-msdos-label \
  iso_root -o sexos-v1.0.0.iso

# Boot
qemu-system-x86_64 -machine q35 -cpu max,pku=on -smp 4 -m 2G -serial file:qemu_serial.log -cdrom sexos-v1.0.0.iso -no-reboot

# Check
grep -Ei 'panic|fault|Spawned|OP_PRIMARY|FB handed' qemu_serial.log
```
