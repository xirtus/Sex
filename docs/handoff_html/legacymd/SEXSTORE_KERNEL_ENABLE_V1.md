# SEXSTORE_KERNEL_ENABLE_V1

## Status

Complete (2026-05-04). sexstore spawned as domain 8, silk-shell granted SLOT_SEXSTORE capability. `[SEXOS ENTRYPOINT] success`.

---

## Changed Files

| File | Change |
|------|--------|
| `crates/sex-pdx/src/lib.rs` | Added `SLOT_SEXSTORE = 10` |
| `kernel/src/init.rs` | Added `sexstore_id` var; added `"sexstore"` to `module_paths` (domain 8); added `domain_id == 8` branch; added guarded `SLOT_SEXSTORE` cap grant to silk-shell |
| `sexos_build_spec.toml` | Added `build_sexstore` stage; added `servers/sexstore/Cargo.toml` to `allowed.crates`; updated `abi_version_hash` |
| `limine.cfg` | Added `MODULE_PATH=boot:///servers/sexstore` |
| `Cargo.toml` | Added `servers/sexstore` to workspace members (required for `cargo_manifest` build action) |

### NOT modified

- `servers/sexstore/src/main.rs` — still a stub infinite loop; KV logic is SEXSTORE_KV_RAM_V1
- `servers/silk-shell/src/main.rs` — no caller code yet; persistence is SCENE_SETTINGS_PERSIST_V1
- `servers/sexdisplay/src/main.rs` — unrelated
- `kernel/src/interrupts.rs` — forbidden

---

## Slot / Domain Assignment

| Entity | Value |
|--------|-------|
| `SLOT_SEXSTORE` | 10 (sex-pdx public const) |
| `SLOT_USB_SEXINPUT` | 9 (kernel-local const, unchanged) |
| sexstore domain_id | 8 |
| sexstore spawn order | 8th in `module_paths` |

---

## ABI Hash Update

sex-pdx changed (new `SLOT_SEXSTORE` const) → hash recomputed:

```bash
{ sha256sum kernel/src/syscalls/mod.rs; sha256sum crates/sex-pdx/src/lib.rs; } | sha256sum
# → 15123e7ffd04ee5a2b6c81304f05cef1da680d3dd28f800b5111d600eadc5bde
```

Updated in `sexos_build_spec.toml`.

---

## Capability Grant (kernel/src/init.rs)

```rust
if sexstore_id != 0 {
    pd.grant_capability(sex_pdx::SLOT_SEXSTORE, CapabilityData::Domain(sexstore_id));
    serial_println!("[kernel.sexstore.cap] shell={} store={}", silkshell_id, sexstore_id);
}
```

Guard prevents capability → domain 0 if sexstore binary is absent from ISO.

---

## Build / Proof Result

```
[SEXOS ENTRYPOINT] success
```

ISO size: 18 files (was 17 — sexstore binary added).

---

## Markers (visible in serial log at runtime)

| Marker | When |
|--------|------|
| `✓ Spawned PD N: .../sexstore (Domain 8)` | Boot — existing pattern |
| `[kernel.sexstore.spawn] id=N` | Boot — sexstore spawn confirmed |
| `[kernel.sexstore.cap] shell=N store=N` | Boot — SLOT_SEXSTORE granted to silk-shell |

---

## Limitations

- **sexstore is still a stub** — `_start()` is an infinite loop; no `pdx_listen_raw`, no KV table, no response to any IPC
- **silk-shell does not call sexstore yet** — no load/save code in silk-shell
- **No K/V behavior implemented** — calling SLOT_SEXSTORE from silk-shell would send to an idle loop
- **No persistence** — sexstore has no storage; all settings remain in-memory only

---

## Next Recommended Phase: SEXSTORE_KV_RAM_V1

Replace sexstore's infinite loop with:

1. Remove `extern crate alloc` / heap infrastructure (no heap needed for static KV)
2. Add `pdx_listen_raw(SLOT_SEXSTORE)` event loop
3. Add static `KV_TABLE: [KvSlot; 16]` (160 bytes, no heap)
4. Handle `OP_KV_GET = 0xB0` and `OP_KV_PUT = 0xB1`
5. Reply with `(status, value)` per API plan
6. Add `[sexstore.kv.get]` / `[sexstore.kv.put]` markers (budget 32 each)
7. Build + verify
8. Create `docs/handoff/SEXSTORE_KV_RAM_V1.md`

After that: SCENE_SETTINGS_PERSIST_V1 (silk-shell reads/writes scene blob at boot/F5/F6).

---

## References

| Doc | Relevance |
|-----|-----------|
| `docs/handoff/SEXSTORE_KERNEL_ENABLE_PLAN_V1.md` | Plan this phase implemented |
| `docs/handoff/SEXSTORE_KV_API_PLAN_V1.md` | API shape for next phase |
| `docs/handoff/SCENE_SETTINGS_INMEM_V1.md` | `SceneAppearanceState` — what will be persisted |
