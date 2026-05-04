# SEXSTORE_KV_RAM_V1

## Status

Complete (2026-05-04). sexstore now listens on SLOT_SEXSTORE,
handles OP_KV_GET (0xB0) and OP_KV_PUT (0xB1) with a static
RAM-only K/V table. `[SEXOS ENTRYPOINT] success`.

---

## Changed Files

| File | Change |
|------|--------|
| `servers/sexstore/src/main.rs` | Replaced infinite loop with `pdx_listen_raw(SLOT_SEXSTORE)` event loop; added 16-slot static K/V table; added OP_KV_GET/PUT handlers; added `kv_reply()` via syscall 29; added rate-limited `[sexstore.kv.*]` markers. |
| `docs/handoff/SEXSTORE_KV_RAM_V1.md` | New — this document. |

### NOT modified

- `crates/sex-pdx/src/lib.rs` — opcodes kept local to avoid ABI hash change
- `kernel/` — no kernel changes
- `sexos_build_spec.toml` — unchanged
- `limine.cfg` — unchanged
- `servers/silk-shell/` — no caller code yet (future SCENE_SETTINGS_PERSIST_V1)
- `servers/sexdisplay/` — unrelated

---

## KV Table

- **Type**: `static mut [KvSlot; 16]` — 16 slots, each 16 bytes → 256 bytes total
- **Key**: `u32`
- **Value**: `u64`
- **Lookup**: linear scan (16 slots, O(n) acceptable)
- **Replacement**: updates in-place if key exists
- **Insertion**: first free slot; returns KV_PUT_FULL (0x02) if full

### KvSlot layout

```rust
struct KvSlot {
    used: u8,    // 0 = free, 1 = occupied
    key:  u32,   // 3 bytes implicit padding after used
    val:  u64,
}
// Size: 1 + 3 pad + 4 + 8 = 16 bytes
```

---

## Protocol (local opcodes, not yet in sex-pdx)

| Direction | Opcode | arg0 | arg1 | Reply value |
|-----------|--------|------|------|-------------|
| GET       | 0xB0   | key (u32) | — | stored u64 (0 = miss) |
| PUT       | 0xB1   | key (u32) | val (u64) | 0x00 = ok, 0x02 = full |

### Reply mechanism

Uses `kv_reply()` — inline syscall 29 (`SYSCALL_PDX_REPLY`):
- `rax = 29`, `rdi = target_pd`, `rsi = value`
- Kernel pushes `IpcReply { value }` into target's `incoming_replies`
- Target reads via `pdx_listen_raw(0)` → `msg.type_id == 0x1`, `msg.arg0 == value`

Not using sex-pdx's `pdx_reply()` (syscall 1 — unhandled in current kernel).

---

## Build / Proof

```
[SEXOS ENTRYPOINT] success
```

Sexstore compiles with zero warnings. No kernel/build/spec changes.

---

## Markers

| Marker | When | Budget |
|--------|------|--------|
| `[sexstore.kv.put] key=N ok=0\|1` | On PUT | 32 |
| `[sexstore.kv.get] key=N hit=0\|1` | On GET | 32 |

Markers decrement from 32 to 0 then stop printing.

---

## Design Decisions

1. **Local opcodes** — OP_KV_GET/PUT defined as `const` in main.rs, not in sex-pdx.
   Avoids ABI hash update. Promoted when silk-shell integration lands.
2. **No heap** — KV table is `static mut`. Allocator infrastructure (`LockedHeap`,
   `#[global_allocator]`) remains in source but unused.
3. **Raw pointer access** — `core::ptr::addr_of_mut!()` / `addr_of!()` with cast
   to element pointer. Index-based `while` loops. Avoids `static_mut_refs` warnings.
4. **No shared memory** — All data passed via PDX arg registers (8-byte value fits).
5. **syscall 29** — Verified in kernel: `rdi as u32 = target_pd_id`, `rsi = value`.
   Caller retrieves via `pdx_listen_raw(0)` → `msg.arg0`.

---

## Next Phase: SCENE_SETTINGS_PERSIST_V1

Wire silk-shell to call sexstore at boot (load scene blob) and on F5/F6 (save).

---

## References

| Doc | Relevance |
|-----|-----------|
| `docs/handoff/SEXSTORE_KV_API_PLAN_V1.md` | API design this phase implements |
| `docs/handoff/SEXSTORE_KERNEL_ENABLE_V1.md` | Kernel spawn + cap grant this phase depends on |
| `docs/handoff/SCENE_SETTINGS_INMEM_V1.md` | SceneAppearanceState — what will be persisted |
