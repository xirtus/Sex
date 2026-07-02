# KERNEL_BAD_RSP_4444_ACPI_DROP_FIX_V2

## Symbolized RIP

- Fault RIP: `0xffffffff802002f8`
- Symbol: `<alloc::sync::Arc<acpi::registers::FixedRegisters<sex_kernel::apic::SexAcpiHandler>>>::drop_slow`
- Instruction at fault: `lock decq 0x8(%rsi)` with fault addr `0x58` (implies bogus `rsi`, consistent with corrupted context/stack).

## Exact Drop Site

The only in-kernel callsite to this `drop_slow` symbol is in:

- `sex_kernel::apic::init_apic` epilogue (`0xffffffff80202d43`), where ACPI platform/table temporaries are dropped after boot parsing.

This indicates the runtime hit of RIP `0xffffffff802002f8` is not expected normal ACPI runtime behavior; it is consistent with corrupted control-flow/stack state jumping into early `.text`.

## Root Cause

First bootstrap context switch set TSS `RSP0` from `TaskContext.kstack_top` directly.

- `TaskContext.kstack_top` is saved-frame base (`alloc_top - 168`), not empty stack top.
- Using that as `RSP0` lets first user→kernel entries push onto the saved frame region.
- Over time this can corrupt saved context and produce bad kernel-mode execution with user-looking `RSP=0x4444...`, including jumps into unrelated early text like ACPI `drop_slow`.

## Fix

- `kernel/src/lib.rs` bootstrap switch path:
  - `update_tss_rsp0(kstack_top)` -> `update_tss_rsp0(kstack_top + 168)`

This matches the existing timer/page-fault switch paths, where `RSP0` is always set to empty top (`saved_base + 168`).

## Scope Safety

- No kernel ABI changes.
- No `sex-pdx` edits.
- No USB/HID changes.
- No SilkBar/sexdisplay visual tuning.
- No scheduler redesign.
