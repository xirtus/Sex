# REAL_HARDWARE_LIMINE_PIPELINE_FIX_V1

- date: 2026-05-06
- baseline HEAD: `90b202cb49259d7d0cbb08e98c78b07507b03a1a`
- previous proof: `REAL_HARDWARE_BOOT_PROOF_V1.md`
- scope: Fix two critical non-kernel boot-pipeline blockers

## Purpose

Fix the two critical non-kernel blockers identified by `REAL_HARDWARE_BOOT_PROOF_V1`:
1. `limine/limine` was ARM64 Mach-O, not x86_64 ELF — could not run on build host
2. `limine bios-install` step was MISSING from the build pipeline — ISO would not boot on real hardware BIOS

No kernel edits. No ABI changes. No driver changes.

## Changes

### 1. Limine Binary Replacement

| Before | After |
|--------|-------|
| `limine/limine`: Mach-O 64-bit arm64 | `limine/limine`: ELF 64-bit x86-64 |
| Cannot execute on x86_64 Linux | Limine 7.13.3, runs correctly |
| Backup: `limine/limine.arm64.bak` | |

**Source**: `/home/xirtus_arch/Documents/microkernel_nightly/limine_bin/limine` (Limine 7.13.3, x86_64 ELF)
**SHA256**: `a83e64767a37a131a4977976616035f5da0eeb73dbe9d0a4a6548233b3dc89a5`

### 2. Build Spec — New Stage (`sexos_build_spec.toml`)

Added after `package_iso`:
```toml
[[stage]]
id = "limine_bios_install"
action = "limine_bios_install"
```

### 3. Build Trace — New Action Handler (`scripts/sexos_build_trace.sh`)

Added in `run_stage()` case statement:
```bash
limine_bios_install)
    limine/limine bios-install sexos-v1.0.0.iso
    ;;
```

### 4. Preflight Script Updates (`scripts/real_hardware_preflight.sh`)

Section 8 (Limine Tool) now checks:
- Binary architecture: must be x86_64 ELF, fails if ARM64/Mach-O detected
- Build spec: verifies `limine_bios_install` stage exists in `sexos_build_spec.toml`

## Files Changed

| File | Change |
|------|--------|
| `limine/limine` | Replaced ARM64 Mach-O with x86_64 ELF (Limine 7.13.3) |
| `limine/limine.arm64.bak` | **NEW** — backup of original ARM64 binary |
| `sexos_build_spec.toml` | +`limine_bios_install` stage after `package_iso` |
| `scripts/sexos_build_trace.sh` | +`limine_bios_install` action handler |
| `scripts/real_hardware_preflight.sh` | +binary architecture check, +bios-install stage check |
| `docs/handoff/REAL_HARDWARE_LIMINE_PIPELINE_FIX_V1.md` | **NEW** — this document |

## Build/Runtime/Preflight Results

```
./scripts/entrypoint_build.sh
  [TRACE] stage=limine_bios_install
  Limine BIOS stages installed successfully!
  [SEXOS ENTRYPOINT] success

./scripts/master_runtime_gate.sh --probe 25 --keep-log
  FINAL_SCORE: GREEN_MASTER
  All 6 gates PASS, QEMU boot unchanged

./scripts/real_hardware_preflight.sh
  PASS  14
  WARN   0
  FAIL   0
  SKIP   0
  [RESULT] Preflight looks OK.
```

## Before/After Summary

| Check | Before | After |
|-------|--------|-------|
| Limine binary type | ARM64 Mach-O (FAIL) | x86_64 ELF (PASS) |
| bios-install in build spec | MISSING (FAIL) | PRESENT (PASS) |
| bios-install in build trace | MISSING (FAIL) | PRESENT (PASS) |
| Preflight architecture check | N/A | x86_64 ELF confirmed |
| Preflight stage check | N/A | limine_bios_install found |
| QEMU runtime gate | GREEN_MASTER | GREEN_MASTER (unchanged) |

## Non-Goals Preserved
- No kernel edits
- No `sex-pdx` ABI edits
- No server changes
- No driver changes
- No bootloader config changes
- No destructive disk operations on host
- BIOS install targets only the build artifact (`sexos-v1.0.0.iso`)
- ARM64 binary preserved as `limine/limine.arm64.bak`

## Remaining Hardware Blockers

These are documented in `REAL_HARDWARE_BOOT_PROOF_V1.md` and remain **unchanged** by this fix:

| # | Blocker | Severity | Requires |
|---|---------|----------|----------|
| 1 | 26× `todo!()` in ACPI handler | HIGH | Kernel edit (STOP FIRST) |
| 2 | Serial port hardcoded 0x3F8, panics on absent | HIGH | Kernel edit (STOP FIRST) |
| 3 | LAPIC timer hardcoded to 1,000,000 counts | MEDIUM | Kernel edit (STOP FIRST) |
| 4 | IOAPIC polarity/trigger hardcoded edge/active-high | MEDIUM | Kernel edit (STOP FIRST) |
| 5 | PS/2 keyboard init without probe | LOW | Kernel edit (STOP FIRST) |

## Next Steps

The real-hardware boot pipeline is now structurally complete. The ISO produced by
`./scripts/entrypoint_build.sh` has proper BIOS MBR/GPT boot records and will
attempt to boot on real hardware.

The next prompts (STOP FIRST — kernel edits) are:
1. `ACPI_HANDLER_TODO_STUB_AUDIT_STOPFIRST_V1`
2. `SERIAL_DEBUG_NO_PANIC_FALLBACK_STOPFIRST_V1`
3. `LAPIC_TIMER_CALIBRATION_PLAN_STOPFIRST_V1`
4. `IOAPIC_POLARITY_TRIGGER_FROM_MADT_STOPFIRST_V1`

**Before any of those**: a manual real-machine boot test with the current ISO
should be attempted to see how far it gets before hitting a kernel blocker.
