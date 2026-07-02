# QUIL_BOOT_MODULE_PLAN_V1

**Status:** Active  
**Purpose:** Plan how/when `servers/quil` becomes ISO-packaged and/or boot-spawned without changing surface ownership or IPC ABI.  
**Scope:** Docs only. No code changes.  
**Prerequisites:** QUIL_SURFACE_HELLO_PLAN_V1 (fbc1abe)

---

## 1. Current Boot/Module/PD Model

### Component diagram

```
sexos_build_spec.toml          kernel/src/init.rs
┌─────────────────────┐        ┌───────────────────────┐
│ [allowed] crates    │        │ module_paths          │
│   servers/linen     │ ─────> │   "linen" → spawn     │
│   servers/sexstore  │  ISO   │   "sexstore" → spawn  │
│   ...               │  build │   (8 servers total)   │
└─────────────────────┘        └───────────────────────┘
       │                                │
       ▼                                ▼
  Limine modules              PD spawn with domain_id
  at boot                     domain_id = pkey (1-8)
```

### Key numbers

| Resource | Used | Available | Constraint |
|----------|------|-----------|------------|
| **domain_id / pkey** | 1-8 | 9-13, 15 | Must be < 16 (4-bit page table field); pkey 0=kernel, 14=SHARED |
| **init module_paths** | 8 entries | Next: index 8 → domain_id=9 | Fixed array — adding requires new entry + domain_id match arm |
| **ISO server slots** | 7 servers | Unbounded | Add `[[stage]]` entry per server |
| **Workspace members** | ~14 crates | Unbounded | Add `"servers/quil"` to `members = [...]` |

### Current Spawn Flow

1. `sexos_build_spec.toml` defines allowed crates + `[[stage]]` entries for ISO packaging
2. Build produces ELF binaries → `iso_root/servers/<name>`
3. Limine loads all `iso_root/servers/*` as boot modules
4. Kernel init iterates `module_paths` array, matches module by path substring
5. Each match calls `pdx_spawn(path, domain_id)` → `create_protection_domain(name, None, domain_id)`
6. `domain_id` becomes the PD's pkey (via `PkruValue::for_domain(domain_id)`)

---

## 2. Packaging Options

### Option A: Package-only (recommended next)

Add Quil to ISO as a boot module but **do not spawn**.

| Step | File | Change |
|------|------|--------|
| 1 | `sexos_build_spec.toml` `[allowed] crates` | Add `"servers/quil/Cargo.toml"` |
| 2 | `sexos_build_spec.toml` `[[stage]]` | Add `build_quil` stage: manifest, source_artifact, dest_artifact |
| 3 | `kernel/src/init.rs` | **No change** — not added to `module_paths` |

**Result:** Quil ELF sits in ISO. Kernel loads it as a module but never spawns it. Zero runtime impact. Proves packaging works.

### Option B: Boot-spawned (future)

Add Quil to ISO AND spawn at boot.

| Step | File | Change | Risk |
|------|------|--------|------|
| 1 | `sexos_build_spec.toml` | Same as Option A | Low |
| 2 | `kernel/src/init.rs` `module_paths` | Add `"quil"` entry | Medium — spawn order, module resolution |
| 3 | `kernel/src/init.rs` domain_id match | Add `domain_id == 9` arm | Medium — PD ID tracking |
| 4 | `kernel/src/init.rs` pkey allocation | domain_id=9 → pkey=9 | Low — within valid range |
| 5 | `sexos_build_spec.toml` `contract_sha256` | Hash changes | High — requires pipeline re-seal |
| 6 | ABI snapshot hash | Changes | High — requires pipeline re-seal |

**Result:** Quil runs at boot. Must have fail-safe (panic doesn't crash system). Still cannot draw on surface 201 (owner_pd issue).

### Option C: Shell-initiated dynamic spawn (future)

Kernel exports a "spawn PD from module" syscall; shell invokes it for Quil on F9 or config.

| Step | File | Change | Risk |
|------|------|--------|------|
| 1 | Kernel syscall | New `spawn_pd` opcode | ⛔ STOP FIRST — ABI change |
| 2 | sex-pdx | New syscall wrapper | ⛔ STOP FIRST — ABI change |
| 3 | Shell | Call on F9 or boot | Depends on #1-2 |

**Result:** Cleanest — Quil only runs when needed. But requires ABI change. **Not viable now.**

---

## 3. Option A: Package-only — Detailed Plan

### sexos_build_spec.toml changes

```toml
# ── [allowed] crates ── add after "servers/linen/Cargo.toml":
"servers/quil/Cargo.toml",

# ── [[stage]] entry ── add after build_linen:
[[stage]]
id = "build_quil"
action = "cargo_manifest"
manifest = "servers/quil/Cargo.toml"
source_artifact = "target/x86_64-sex/release/quil"
dest_artifact = "iso_root/servers/quil"
```

### Files NOT changed

- `kernel/src/init.rs` — No spawn entry. Quil not in `module_paths`.
- `servers/quil/src/main.rs` — No changes. Still pure yield stub.
- `servers/silk-shell/src/main.rs` — No changes. Shell still owns surface 201.
- `Cargo.toml` — Already done (workspace member added in QUIL_SERVER_STUB_PD_V1).
- `crates/sex-pdx/src/lib.rs` — No new slots or opcodes.
- `scripts/` — No build script changes.

### Proof that Quil is not spawned

```rust
// kernel/src/init.rs module_paths has 8 entries; quil is not among them.
let module_paths = ["sexdisplay", "sexdrive", "silk-shell", "sexinput",
                    "sexusb", "silkbar", "linen", "sexstore"];
// Quil module is loaded by Limine but never matched → never spawned.
```

### Pipeline impact

| Item | Changes |
|------|---------|
| `contract_sha256` | **Changes** — `sexos_build_spec.toml` modified |
| `abi_version_hash` | No change — no ABI/syscall/sex-pdx edits |
| Pipeline re-seal required? | **Yes** — contract hash must be updated |
| Build time | +0.3s (single crate, no deps beyond sex-pdx) |
| ISO size | +~16KB (quil ELF) |

---

## 4. Decision Matrix

| Question | Option A (package-only) | Option B (boot-spawn) | Option C (dynamic spawn) |
|----------|------------------------|----------------------|--------------------------|
| Quil binary in ISO? | ✅ Yes | ✅ Yes | Would be yes |
| Quil runs at boot? | ❌ No | ✅ Yes | ❌ No |
| Surface 201 drawing? | ❌ No (owner_pd) | ❌ No (owner_pd) | ❌ No (owner_pd) |
| Pipeline re-seal? | ✅ Required | ✅ Required | ⛔ ABI change |
| Fault risk? | None (not spawned) | Low (panic won't crash kernel) | Low |
| Blocks surface hello? | No (still blocked) | No (still blocked) | No (still blocked) |
| Runtime resource use? | 0 (not spawned) | ~64KB stack + PD struct | Only when spawned |

### Recommendation

**Option A: Package-only now.** This is the smallest safe step that moves Quil from "build only" to "exists in the boot environment." It proves the full toolchain pipeline works for Quil without any runtime risk.

**Do NOT boot-spawn yet.** Spawning without a surface interaction path (owner_pd) adds runtime complexity for zero user-visible benefit. The only thing Quil could do if spawned is `sys_yield` — same as the stub.

**Do NOT attempt surface hello until ownership transfer or shared-draw protocol exists.** Both require prerequisite decisions beyond boot packaging.

---

## 5. STOP FIRST Table

| # | Condition | Risk | Applies to |
|---|-----------|------|------------|
| 1 | **New PD ID without registry audit** | PD ID collision | B, C |
| 2 | **pkey exhaustion** | domain_id > 15 would overflow 4-bit pkey field | B (quilt would be pkey=9, fine) |
| 3 | **Init spawn list reorder** | Changes domain_id assignment → breaks capability assumptions | B |
| 4 | **Module order dependency** | init assumes module position = spawn order | B |
| 5 | **Build spec ambiguity** | Multiple entries for same artifact path | A (must verify unique dest_artifact) |
| 6 | **Runtime fault in unspawned PD** | Impossible — not running | None for A |
| 7 | **New opcode/ABI** | Pipeline hash mismatch, contract violation | C |
| 8 | **Surface ownership conflict** | Two PDs claiming surface 201 | B (if quil called 0xEC) |
| 9 | **Pipeline seal break** | contract_sha256 must be updated | A, B (required, not a stop) |

**No STOP FIRST conditions triggered for Option A.** All changes are confined to `sexos_build_spec.toml`. The pipeline must be re-sealed (contract hash update) but that's expected, not a stop.

---

## 6. Sequence

```
Current (stub, build-only)
  │
  ▼
QUIL_PACKAGE_ONLY_V1          ← recommended next implementation
  ├── Add to build spec [allowed] crates
  ├── Add [[stage]] entry
  ├── Pipeline re-seal (contract hash)
  └── Verify: quil ELF in ISO, not spawned
  │
  ▼
QUIL_BOOT_SPAWN_DECISION_V1   ← docs-only, revisit later
  └── Decide: spawn or stay package-only
  │
  ▼
(Surface hello or mediated protocol — deferred)
```

---

## Change History

| Date | Change | Author |
|------|--------|--------|
| 2026-05-04 | Boot module packaging analysis — recommend package-only now | QUIL_BOOT_MODULE_PLAN_V1 |
