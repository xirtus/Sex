# ELF Loader Reference

> Referenced from CLAUDE.md (offloaded reference).

---

## Load Details (`kernel/src/elf.rs::load_elf_for_pd`)

- Loads segments at `load_base + (p_vaddr - min_vaddr)` where min_vaddr is the smallest vaddr across all PT_LOAD segments.
- Returns entry point as `load_base + (header.entry - min_vaddr)`.
- For PIE ELFs (p_vaddr=0, min_vaddr=0): segments at `load_base`, entry at `load_base + elf_entry` — correct.
- For fixed-address ELFs (p_vaddr=0x200000, min_vaddr=0x200000): segments at `load_base`, entry at `load_base + (entry - 0x200000)` — correct.

## Critical Gaps

### No GOT Relocation Processing
**Does NOT process `.rela.dyn` or `.rela.plt`.** Any absolute address reference (GOT entry for cross-crate `pub static`) retains the ELF's original address. Use `const` instead of `static` for shared data to force compile-time inlining.

### No Lower-Half Check
**Does NOT check for lower-half vaddr ranges** — the string "ELF lower half phdrs are not allowed" does NOT exist in the kernel source. Segments with vaddr in the 0x0000-0x3FFF range are loaded at `load_base + delta` without rejection.

### GOT Relocation Gap (BURNDOWN)
When sexdisplay/silkbar references `DEFAULT_SILK_BAR` (a `pub static` from another crate), the compiler generates a GOT entry. At link time (PIE), the GOT entry receives the ELF's pre-relocation address (e.g., 0x2001d8). The kernel loads the segment at a different base (e.g., 0x44000000) but never fixes GOT entries. Result: page fault at the stale lower-half address.

**Fix: use `pub const` instead of `pub static`** — forces compile-time inlining, no GOT entry needed.
