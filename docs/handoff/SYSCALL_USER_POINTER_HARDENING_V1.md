# SYSCALL_USER_POINTER_HARDENING_V1

Date: 2026-07-21

## Problem

Four kernel syscall paths directly dereferenced caller-supplied pointers with
only alignment or null checks:

- `snapshot_ingest(src)` — copied `SceneSnapshot` from `src` after only an
  alignment check.
- `snapshot_resolve(handle, out_ptr)` — wrote `SceneSnapshot` to `out_ptr`
  after only a null check.
- `PDX_GET_DISPLAY_INFO` (`arg0`) — wrote `DisplayInfo` through `arg0` with no
  validation.
- `raw_print(arg0, arg1)` — constructed a slice from arbitrary `arg0`/`arg1`.

A canonical, user-range pointer can still be unmapped, read-only, mapped under
an inaccessible PKEY, or cross a page boundary into an invalid page.

## Fix

Added `validate_user_bytes(start, len, writable)` in
`kernel/src/syscalls/mod.rs`. It:

1. Rejects non-canonical `start`.
2. Computes `inclusive_end = start.checked_add((len as u64) - 1)` and rejects
   arithmetic wraparound.
3. Rejects non-canonical `inclusive_end` **before** any page alignment,
   masking, or translation.
4. Derives `first_page` and `last_page` only after both endpoints are proven
   canonical.
5. Walks every 4 KiB page in the inclusive range with
   `memory::manager::read_pte_flags`.
6. Rejects pages that are not `PRESENT` or not `USER_ACCESSIBLE`.
7. Rejects write requests against non-`WRITABLE` pages.
8. Extracts the page PKEY (bits 62:59) and checks the current PD's
   `current_pkru_mask` access-disable / write-disable bits.

Each target path now calls `validate_user_bytes` before dereferencing:

- `snapshot_ingest` validates read access for `size_of::<SceneSnapshot>()`.
- `snapshot_resolve` validates write access for `size_of::<SceneSnapshot>()`.
- `PDX_GET_DISPLAY_INFO` validates write access for `size_of::<DisplayInfo>()`.
- `raw_print` rejects `len == 0` as success, rejects `len > RAW_PRINT_MAX`
  (`4096`), and validates read access for the requested range.

All four paths return `sex_pdx::ERR_CAP_INVALID` on validation failure before
any copy or serial output.

## Files changed

- `kernel/src/syscalls/mod.rs` — added validator and gated the four paths.
- `sexos_build_spec.toml` — updated `abi_version_hash` (syscalls/mod.rs is part
  of the ABI snapshot). The actual syscall ABI (numbers, register layout,
  struct layouts, success semantics) is unchanged.
- `scripts/syscall_user_pointer_hardening_gate.sh` — new static + optional
  runtime gate.
- `docs/handoff/SYSCALL_USER_POINTER_HARDENING_V1.md` — this file.

## Validation

Build gates:
- `./scripts/entrypoint_build.sh` PASS
- `./scripts/rsp0_regression_gate.sh` PASS
- `./scripts/syscall_user_pointer_hardening_gate.sh` PASS (static rows)
- `./scripts/syscall_user_pointer_hardening_gate.sh logs/qemu-latest.log` PASS
  (runtime zero-fault row)

Runtime gates:
- `./scripts/disk_persistence_gate.sh` PASS
- `./scripts/usb_path_gate.sh` PASS
- `./scripts/gate_0_2.sh` — BUILD/BOOT/INPUT_OWNERSHIP/FAULT_REGRESSION PASS;
  POINTER_LIVE/KEYBOARD_LIVE FAIL, matching the pre-existing baseline recorded
  in `docs/handoff/GATE_0_2_LAST_RUN.md` and unrelated to this change.

## Residual limitations

1. PKEY validation uses the current PD's `current_pkru_mask`. A transient
   capability revocation that has already updated page-table PKEY bits but not
   yet updated `current_pkru_mask` could allow a stale access. Closing this
   requires a capability-driven PKRU mask (STOP_FIRST, cross-PD authority).
2. `validate_user_bytes` does not hold any lock while reading page tables. On
   SMP a concurrent unmap could race; the range remains valid only for the
   duration of the immediate copy/output. This matches the existing single-core
   kernel assumption.
3. `read_pte_flags` can read huge-page leaf entries. The validator samples the
   4 KiB-aligned page start within a huge page; because the leaf PTE covers the
   entire huge page, the same flags apply to the whole range.
4. This patch does not add capability checks to `MAP_MEMORY`,
   `SYSCALL_ALLOC_SHARED_BUFFER`, or `SYS_NET_DIAG`. Those are STOP_FIRST
   security-model changes tracked separately.

## Future syscall audit targets

Any syscall that casts a user-supplied u64 to a pointer and dereferences it
should call `validate_user_bytes` (or a successor) first. Candidates for the
next pass:

- `MAP_MEMORY` (syscall 30) — maps arbitrary physical addresses.
- `SYSCALL_ALLOC_SHARED_BUFFER` (syscall 40) — allocates shared buffers for
  arbitrary consumer IDs.
- `SYS_GRANT_MEM_LEND` / `SYS_MAP_MEM_LEND` (syscalls 50/51) — already use
  capability lookups but should still validate the returned VA range before
  kernel use.
- Any future syscall accepting a user pointer.

## Reusable rule

"Before the kernel reads or writes memory through a caller-supplied pointer,
prove every page in the byte range is PRESENT, USER_ACCESSIBLE, and has the
required read/write permission under the caller's user-mode PKRU mask."
