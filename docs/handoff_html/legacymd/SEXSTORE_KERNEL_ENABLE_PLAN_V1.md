# SEXSTORE_KERNEL_ENABLE_PLAN_V1

## Status

Design (2026-05-04). Exact minimal patch plan to spawn sexstore and route silk-shell → sexstore capability.
Docs-only — no code changed.

---

## Verdict: SEXSTORE_KERNEL_ENABLE_SAFE_MINIMAL ✅

All changes are additive. No existing slots, domains, caps, or IPC flows are disturbed.
Implementation requires edits to 4 files, all non-disruptive.

| Risk | Assessment |
|------|-----------|
| SLOT_SEXSTORE=10 conflict | ✅ None — slot 10 is free (slot 9 = kernel-local SLOT_USB_SEXINPUT) |
| Domain ID 8 conflict | ✅ None — domains 1–7 assigned, 8 free |
| sexstore missing from ISO | ✅ Fixable — binary exists, not yet in build spec or limine.cfg |
| sexstore can listen post-spawn | ✅ — `pdx_listen_raw` works for any spawned domain |
| silk-shell calls sexstore before sexstore ready | ⚠️ Race — handled by `ERR_SERVICE_NOT_READY`; caller uses defaults |
| sexstore spawn failure → silk-shell gets cap to domain 0 | ✅ Fixable — guard grant with `sexstore_id != 0` |
| ABI hash invalidated | ✅ Fixable — update `sexos_build_spec.toml` after sex-pdx change |

---

## Audit Results

### Current spawn order (`kernel/src/init.rs` line 36)

```rust
let module_paths = ["sexdisplay", "sexdrive", "silk-shell", "sexinput", "sexusb", "silkbar", "linen"];
//  domain_id:           1            2            3             4          5          6         7
```

**sexstore = absent. Domain ID 8 = next free.**

### Current SLOT assignments (`crates/sex-pdx/src/lib.rs` + `kernel/src/init.rs`)

| Slot | Constant | Source | Service |
|------|----------|--------|---------|
| 1 | `SLOT_STORAGE` | sex-pdx | sexfiles VFS |
| 2 | `SLOT_SEXT` | sex-pdx | sext demand pager |
| 3 | `SLOT_INPUT` | sex-pdx | HID input ring |
| 4 | `SLOT_AUDIO` | sex-pdx | audio server |
| 5 | `SLOT_DISPLAY` | sex-pdx | sexdisplay compositor |
| 6 | `SLOT_SHELL` | sex-pdx | silk-shell |
| 7 | `SLOT_SILKBAR` | sex-pdx | silkbar |
| 8 | `SLOT_USB_HOST` | sex-pdx | XHCI probe |
| 9 | `SLOT_USB_SEXINPUT` | kernel-local const | sexusb→sexinput route |
| **10** | **`SLOT_SEXSTORE`** | **← new, sex-pdx** | **sexstore KV** |

SLOT 10 is free. No conflict.

### Current `grant_capability` pattern

silk-shell grant block (`kernel/src/init.rs` lines 87–93):

```rust
if sexdisp_id != 0 && silkshell_id != 0 {
    if let Some(pd) = DOMAIN_REGISTRY.get(silkshell_id) {
        pd.grant_capability(sex_pdx::SLOT_DISPLAY, CapabilityData::Domain(sexdisp_id));
        pd.grant_capability(sex_pdx::SLOT_SHELL,   CapabilityData::Domain(silkshell_id));
        pd.grant_capability(sex_pdx::SLOT_SILKBAR, CapabilityData::Domain(silkbar_id));
        // ← SLOT_SEXSTORE grant goes here
    }
}
```

Pattern is established. One additional `grant_capability` line suffices.

### sexstore binary in ISO

**NOT in `sexos_build_spec.toml`** — no build stage, no copy.
**NOT in `limine.cfg`** — not loaded as a Limine module → kernel MODULE_REQUEST never sees it.

Binary source exists at `servers/sexstore/Cargo.toml`. Must add build stage + ISO copy + limine module entry.

### Can sexstore listen post-spawn without further kernel changes?

**YES.** `pdx_listen_raw(slot)` in sex-pdx works for any spawned PD's own listen slot. No kernel change beyond spawn + capability grant is required. sexstore's `_start()` can call `pdx_listen_raw` immediately after the spawn loop completes and grants are issued.

---

## Exact Patch Plan (for SEXSTORE_KERNEL_ENABLE_V1)

Four files. All additive. No existing lines removed or modified (except the two array/loop additions in init.rs).

---

### File 1: `crates/sex-pdx/src/lib.rs`

Add after `SLOT_USB_HOST` (line ~349):

```rust
pub const SLOT_SEXSTORE: u64 = 10;  // sexstore K/V service
```

**This change triggers an ABI hash update.** See File 4.

---

### File 2: `kernel/src/init.rs`

**Change A** — add `sexstore_id` variable (after `let mut linen_id = 0;`, line ~33):

```rust
let mut sexstore_id = 0;
```

**Change B** — add `"sexstore"` to `module_paths` (line 36), after `"linen"`:

```rust
let module_paths = ["sexdisplay", "sexdrive", "silk-shell", "sexinput", "sexusb", "silkbar", "linen", "sexstore"];
//  domain_id:           1            2            3             4          5          6         7          8
```

**Change C** — add domain_id == 8 branch (after `domain_id == 7` block, line ~70):

```rust
} else if domain_id == 8 {
    sexstore_id = id;
}
```

**Change D** — add capability grant inside silk-shell block (after SLOT_SILKBAR grant, line ~91):

```rust
if sexstore_id != 0 {
    pd.grant_capability(sex_pdx::SLOT_SEXSTORE, CapabilityData::Domain(sexstore_id));
}
serial_println!("✓ sexstore: Capability SLOT_SEXSTORE granted to silk-shell");
```

The `if sexstore_id != 0` guard prevents a capability to domain 0 if sexstore fails to spawn (e.g. binary missing from ISO).

No other kernel changes. No new CapabilityData variants. No scheduler changes. No IPC changes.

---

### File 3: `sexos_build_spec.toml`

**Change A** — add build stage before `copy_limine_cfg`:

```toml
[[stage]]
id = "build_sexstore"
action = "cargo_manifest"
manifest = "servers/sexstore/Cargo.toml"
source_artifact = "target/x86_64-sex/release/sexstore"
dest_artifact = "iso_root/servers/sexstore"
```

**Change B** — update `abi_version_hash` (after sex-pdx change in File 1):

```bash
# Run from repo root:
{ sha256sum kernel/src/syscalls/mod.rs; sha256sum crates/sex-pdx/src/lib.rs; } | sha256sum
```

Replace existing `abi_version_hash = "..."` with output.

---

### File 4: `limine.cfg`

Add before or after `linen` module line:

```
MODULE_PATH=boot:///servers/sexstore
```

Position in limine.cfg does not affect spawn order — spawn order is determined by `module_paths` array in `init.rs`, which matches on path substring.

---

## Implementation Sequencing

The four changes must be applied in this order to avoid incremental build failures:

1. `crates/sex-pdx/src/lib.rs` — add `SLOT_SEXSTORE` (needed by kernel)
2. `kernel/src/init.rs` — reference `sex_pdx::SLOT_SEXSTORE` (compiles against updated sex-pdx)
3. `sexos_build_spec.toml` — add build stage + update abi_version_hash
4. `limine.cfg` — add MODULE_PATH

Single build after all 4 changes. Do not build incrementally between changes.

---

## What sexstore's `_start()` needs (post-enable)

No further kernel changes. Once spawned and capability granted, sexstore can immediately:

```rust
loop {
    let msg = sex_pdx::pdx_listen_raw(sex_pdx::SLOT_SEXSTORE);
    unsafe {
        match msg.opcode {
            sex_pdx::OP_KV_GET => { /* ... */ }
            sex_pdx::OP_KV_PUT => { /* ... */ }
            _ => { sex_pdx::pdx_reply(msg.sender_pd); }
        }
    }
}
```

`pdx_listen_raw` blocks until a message arrives on the slot. No additional syscalls, no new IPC primitives.

---

## STOP Conditions

| Condition | Action |
|-----------|--------|
| `sex_pdx::SLOT_SEXSTORE` reference fails to compile in kernel | Check sex-pdx build order; kernel depends on sex-pdx crate |
| SLOT_USB_SEXINPUT=9 moves to sex-pdx in future | Re-audit; SLOT_SEXSTORE=10 stays valid as long as 9 is taken |
| sexstore spawn fails (binary not in ISO) | `sexstore_id = 0`; guard prevents bad capability; silk-shell gets ERR_SERVICE_NOT_READY → uses defaults |
| PDX listen primitive not available in sexstore | Verify sex-pdx dependency in `servers/sexstore/Cargo.toml`; add if missing |
| ABI hash not updated after sex-pdx change | Build will fail with hash mismatch; compute and update |

---

## Validation Commands (post-implementation)

```bash
# Build passes
./scripts/entrypoint_build.sh

# sexstore binary present in ISO
ls iso_root/servers/sexstore

# sexstore spawned in kernel log
grep "Spawned PD.*sexstore" /tmp/sexstore-kernel-enable-v1.log

# Capability granted
grep "SLOT_SEXSTORE granted" /tmp/sexstore-kernel-enable-v1.log

# No panics
grep -cE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/sexstore-kernel-enable-v1.log
```

---

## NOT changed

| File | Reason |
|------|--------|
| `kernel/src/interrupts.rs` | Forbidden |
| `servers/sexdisplay/src/main.rs` | Unrelated |
| `servers/silk-shell/src/main.rs` | No caller code yet (SCENE_SETTINGS_PERSIST_V1) |
| `servers/sexfiles/src/main.rs` | Not used |
| `kernel/src/scheduler.rs` | No change needed |
| `kernel/src/ipc*.rs` | PDX infra already general |

---

## Proof Markers (for SEXSTORE_KERNEL_ENABLE_V1)

| Marker | When | Budget |
|--------|------|--------|
| `✓ Spawned PD N: .../sexstore (Domain 8)` | Kernel boot, sexstore spawned | — (existing pattern) |
| `✓ sexstore: Capability SLOT_SEXSTORE granted to silk-shell` | Kernel boot, cap granted | — |
| `[sexstore.ready]` | sexstore _start() running (add in SEXSTORE_KV_RAM_V1) | 1 |

---

## Pass Criteria

- [x] Verdict: SEXSTORE_KERNEL_ENABLE_SAFE_MINIMAL
- [x] Current spawn order confirmed: domains 1–7 taken, domain 8 free for sexstore
- [x] SLOT_SEXSTORE=10 confirmed free (slot 9 = kernel-local SLOT_USB_SEXINPUT)
- [x] sexstore binary confirmed NOT in build spec or limine.cfg (must add)
- [x] `grant_capability` pattern confirmed (existing silk-shell block)
- [x] `pdx_listen_raw` confirmed sufficient post-spawn (no extra kernel primitives needed)
- [x] `sexstore_id != 0` guard documented (prevents bad cap on spawn failure)
- [x] ABI hash update flagged
- [x] Implementation sequencing documented (sex-pdx → kernel → build_spec → limine.cfg)
- [x] Single build after all 4 changes
- [x] STOP conditions documented
- [x] Validation commands provided
- [x] NO existing spawn order, slots, caps, or IPC flows disturbed

---

## Next Phase: SEXSTORE_KERNEL_ENABLE_V1

Implement the 4-file patch described above:

1. `crates/sex-pdx/src/lib.rs` — add `SLOT_SEXSTORE = 10`
2. `kernel/src/init.rs` — add sexstore_id var, add to module_paths, add domain_id==8 branch, add SLOT_SEXSTORE capability grant to silk-shell
3. `sexos_build_spec.toml` — add `build_sexstore` stage + update `abi_version_hash`
4. `limine.cfg` — add `MODULE_PATH=boot:///servers/sexstore`
5. Build: `./scripts/entrypoint_build.sh`
6. Verify: `[SEXOS ENTRYPOINT] success` + sexstore spawned in log
7. Create `docs/handoff/SEXSTORE_KERNEL_ENABLE_V1.md`

After this: proceed to **SEXSTORE_KV_RAM_V1** (implement sexstore KV listener using the now-active spawn/capability).

---

## References

| Doc | Relevance |
|-----|-----------|
| `docs/handoff/SEXSTORE_KV_API_PLAN_V1.md` | API shape, opcodes, value model, caller responsibilities |
| `kernel/src/init.rs` | Spawn loop, capability grant pattern |
| `crates/sex-pdx/src/lib.rs` | Slot table, `pdx_listen_raw`, `SLOT_USB_HOST=8` |
| `limine.cfg` | MODULE_PATH entries (one per server) |
| `sexos_build_spec.toml` | Build stages, `abi_version_hash` |
