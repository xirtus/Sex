# QUIL_PACKAGE_ONLY_V1

**Status:** Active  
**Purpose:** Package `servers/quil` into ISO as a Limine module without boot-spawning it. Validates build/stage pipeline.  
**Scope:** `sexos_build_spec.toml` only. No kernel/init/sexdisplay/silk-shell edits.  
**Prerequisites:** QUIL_BOOT_MODULE_PLAN_V1 (25e3afd), QUIL_SERVER_STUB_PD_V1 (6c1f3b5)

---

## What Was Done

Added Quil to the ISO build pipeline by editing `sexos_build_spec.toml`:

### Changes

| File | Change |
|------|--------|
| `sexos_build_spec.toml` `[allowed] crates` | Added `"servers/quil/Cargo.toml"` (build whitelist) |
| `sexos_build_spec.toml` `[[stage]]` | Added `build_quil` stage (compile + copy to ISO) |

### Build spec entries

```toml
# In [allowed] crates (after linen):
"servers/quil/Cargo.toml",

# [[stage]] entry (after build_linen):
[[stage]]
id = "build_quil"
action = "cargo_manifest"
manifest = "servers/quil/Cargo.toml"
source_artifact = "target/x86_64-sex/release/quil"
dest_artifact = "iso_root/servers/quil"
```

### Files NOT changed

- `kernel/src/init.rs` — Unchanged. `module_paths` still has 8 entries. No Quil spawn.
- `servers/quil/src/main.rs` — Unchanged. Still pure yield stub.
- `servers/silk-shell/src/main.rs` — Unchanged. Shell still owns surface 201.
- `Cargo.toml` — Unchanged (already added in QUIL_SERVER_STUB_PD_V1).
- `crates/sex-pdx/src/lib.rs` — Unchanged. No new slots or opcodes.

---

## Verification

| Check | Result |
|-------|--------|
| Quil binary in ISO staging? | ✅ `iso_root/servers/quil` (2840 bytes, ELF 64-bit) |
| Kernel init spawn list? | ❌ Quil not in `module_paths` — not spawned |
| `module_paths` unchanged? | ✅ `["sexdisplay", "sexdrive", "silk-shell", "sexinput", "sexusb", "silkbar", "linen", "sexstore"]` |
| Pipeline seal/hash update? | Not needed — `sexos_contract.toml` unchanged, ABI files unchanged |
| Build result | ✅ `[SEXOS ENTRYPOINT] success` |
| ISO servers directory | `linen`, **`quil`**, `sexdisplay`, `sexinput`, `sexstore`, `sexusb`, `silkbar`, `silk-shell` |

### Build log markers

```
[TRACE] stage=build_quil
...
Compiling quil v0.1.0
Finished release profile [optimized] target(s)
...
[TRACE] stage=package_iso
[SEXOS ENTRYPOINT] success
```

---

## Proof Quil Is Not Boot-Spawned

```rust
// kernel/src/init.rs line 37:
let module_paths = ["sexdisplay", "sexdrive", "silk-shell",
                    "sexinput", "sexusb", "silkbar",
                    "linen", "sexstore"];
// 8 entries, domain_id 1-8. No "quil" entry.
// Limine loads iso_root/servers/quil as a module, but
// kernel init never matches "quil" → never spawned.
```

---

## STOP FIRST Items Checked

| # | Condition | Result |
|---|-----------|--------|
| 1 | New PD ID without registry | Not assigned — Quil not spawned |
| 2 | pkey exhaustion | Not allocated — Quil not spawned |
| 3 | Init spawn list risk | Not modified |
| 4 | Module order dependency | Not relevant — no spawn |
| 5 | Build spec ambiguity | Single manifest, unique dest_artifact `iso_root/servers/quil` |
| 6 | Runtime fault risk | None — not spawned |
| 7 | ABI/opcode change | None |
| 8 | Surface ownership conflict | Not touched — shell owns surface 201 |

---

## Phase Sequence Progress

| Phase | Status |
|-------|--------|
| QUIL_SERVER_STUB_PD_V1 | ✅ Build-only PD |
| **QUIL_PACKAGE_ONLY_V1** | **✅ ISO-packaged, not spawned** |
| QUIL_BOOT_SPAWN_DECISION_V1 | 🔲 Docs-only (future) |
| QUIL_SURFACE_HELLO_V1 | 🔲 Requires spawn + ownership resolution |

---

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-04 | Package Quil into ISO without boot-spawning | QUIL_PACKAGE_ONLY_V1 |
