# SCENE_SETTINGS_PERSIST_V1

## Status

Complete (2026-05-04). Silk-shell persists `preset_idx` (and reserved
`chrome_flags`/`accessibility_flags`) to sexstore RAM K/V on F5, and loads
them asynchronously at boot. `[SEXOS ENTRYPOINT] success`.

---

## Changed Files

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | Added local sexstore opcodes (`OP_KV_GET=0xB0`, `OP_KV_PUT=0xB1`); added `SCENE_SETTINGS_KEY_APPEARANCE=0x01`; added `SEXSTORE_LOAD_PENDING` flag; added `pack_scene_settings_blob`, `unpack_scene_settings_blob`, `handle_sexstore_get_reply`, `boot_load_scene_settings`; added `0x1` reply arm in main loop; added persist call after F5 preset cycle; added markers |
| `docs/handoff/SCENE_SETTINGS_PERSIST_V1.md` | New — this document. |

### NOT modified

- `servers/sexstore/src/main.rs` — no change needed
- `crates/sex-pdx/src/lib.rs` — no ABI hash change
- `kernel/` — no kernel changes
- `sexos_build_spec.toml` — unchanged
- `limine.cfg` — unchanged
- `servers/sexdisplay/` — unrelated
- `servers/silkbar/` — unrelated

---

## Packed Blob Layout

8-byte `u64` stored in sexstore under key `0x01`:

```
Byte 0: magic      = 0xAC
Byte 1: version    = 0x01
Byte 2: preset_idx (0..3; clamp on read)
Byte 3: chrome_flags (reserved; 0 in V1)
Byte 4: accessibility_flags (reserved; 0 in V1)
Byte 5: reserved = 0
Byte 6: reserved = 0
Byte 7: checksum  = XOR(byte0 .. byte6)
```

### Constants (local to silk-shell)

```rust
const OP_KV_GET: u64 = 0xB0;
const OP_KV_PUT: u64 = 0xB1;
const SCENE_SETTINGS_KEY_APPEARANCE: u64 = 0x01;
const SCENE_BLOB_MAGIC:   u8 = 0xAC;
const SCENE_BLOB_VERSION: u8 = 0x01;
```

---

## Boot Load Sequence

```
1. Initialize SCENE_APPEARANCE_STATE = DEFAULT_SCENE_APPEARANCE
2. Send resolved tokens to sexdisplay (send_scene_render_tokens())
   → screen shows BottleGlass immediately
3. Fire OP_KV_GET to SLOT_SEXSTORE with key=0x01
   → set SEXSTORE_LOAD_PENDING = true
   → marker: [shell.scene.settings.load.request] ok=1 pending
4. Enter main loop
5. On type_id == 0x1 (reply from sexstore):
   → if SEXSTORE_LOAD_PENDING:
     → clear pending
     → validate magic/version/checksum
     → if valid: apply preset_idx, chrome_flags, accessibility_flags;
                 reset use_custom_colors=0, custom_colors=[0;8], ACTIVE_TINT_IDX=0;
                 re-send tokens
                 marker: [shell.scene.settings.load] ok=1 preset=N
     → if invalid: keep defaults already sent
                 marker: [shell.scene.settings.load] ok=0 corrupt
```

### Reply mechanism

- Sexstore reply uses syscall 29 (`SYSCALL_PDX_REPLY`) → kernel pushes
  `IpcReply { value }` into silk-shell's `incoming_replies`
- Silk-shell's `pdx_listen_raw(0)` dequeues the reply with `type_id == 0x1`,
  `msg.arg0 == stored_u64`
- See `docs/handoff/SEXSTORE_KV_RAM_V1.md` for sexstore reply details.

---

## Save Trigger

| Key | Action | Persists? |
|-----|--------|-----------|
| F5 | `cycle_scene_render_token_preset()` | ✅ Yes — saves `preset_idx` + flags |
| F6 | `cycle_custom_tint()` | ❌ No — tint is ephemeral |

F5 fires `OP_KV_PUT` with packed blob. Fire-and-forget — no reply wait,
no retry on `KV_FULL`.

---

## What Is / Is Not Persisted

| Field | Persisted? | Reason |
|-------|------------|--------|
| `preset_idx` | ✅ Yes | Core setting |
| `chrome_flags` | ✅ Yes | Reserved for future chrome |
| `accessibility_flags` | ✅ Yes | Reserved for future a11y |
| `use_custom_colors` | ❌ No | Always reset to 0; requires settings app |
| `custom_colors[8]` | ❌ No | 32 bytes exceeds u64 |
| `ACTIVE_TINT_IDX` | ❌ No | Ephemeral; always resets to 0 (Clear) |

---

## Markers

| Marker | When | Budget |
|--------|------|--------|
| `[shell.scene.settings.load.request] ok=1 pending` | Boot GET fired | 1 |
| `[shell.scene.settings.load.request] ok=0 status=N` | Boot GET failed (sexstore not ready) | 1 |
| `[shell.scene.settings.load] ok=1 preset=N chrome=N access=N` | Valid GET reply applied | 1 |
| `[shell.scene.settings.load] ok=0 corrupt` | GET reply failed validation | 1 |
| `[shell.scene.settings.save] preset=N` | F5 fired PUT | 16 |

Existing markers from SCENE_SETTINGS_INMEM_V1 and SCENE_CUSTOM_COLOR_KEYS_V1 retained.

---

## Build / Proof Result

```
[SEXOS ENTRYPOINT] success
```

No ABI hash update required (sex-pdx unchanged). No kernel changes.

### Verification (if run)

```bash
env SEXUSB_QEMU_DISPLAY=sdl-grab SEXUSB_QEMU_DEVICE=mouse ./dev.sh run 2>&1 | tee /tmp/scene-settings-persist-v1.log

grep -ac "\[shell.scene.settings.load.request\]" /tmp/scene-settings-persist-v1.log
grep -ac "\[shell.scene.settings.load\]" /tmp/scene-settings-persist-v1.log
grep -ac "\[shell.scene.settings.save\]" /tmp/scene-settings-persist-v1.log
grep -ac "\[sexstore.kv.get\]" /tmp/scene-settings-persist-v1.log
grep -ac "\[sexstore.kv.put\]" /tmp/scene-settings-persist-v1.log
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/scene-settings-persist-v1.log
```

---

## Limitations

- **RAM-only** — sexstore has no disk; settings lost on power-off
- **No custom tint persistence** — `ACTIVE_TINT_IDX` resets to 0 (Clear) on reboot
- **No custom color persistence** — `custom_colors` requires settings app UI
- **No settings app** — only F5/F6 shortcuts for state changes
- **No multi-key support** — single key `0x01` for whole scene appearance blob
- **No timeout** — if sexstore never replies, defaults remain; no retry
- **No backward compatibility** — V1 blob with `version=0x01`; future versions must increment version byte

---

## Next Recommended Phase: SCENE_SETTINGS_ABI_PROMOTE_V1

Promote sexstore opcodes (`OP_KV_GET`, `OP_KV_PUT`, key IDs, error codes)
from local copies in silk-shell and sexstore to `crates/sex-pdx/src/lib.rs`.

Rationale: current duplication (2 files) is acceptable for V1 but becomes
a maintenance burden as more clients use sexstore.

Alternatively: **SCENE_SETTINGS_BOOT_PROOF_V1** — test persistence end-to-end
with QEMU run, verify load/save markers, validate that F6 does NOT persist.

---

## References

| Doc | Relevance |
|-----|-----------|
| `docs/handoff/SCENE_SETTINGS_PERSIST_PLAN_V1.md` | Plan this phase implements |
| `docs/handoff/SEXSTORE_KV_RAM_V1.md` | sexstore K/V reply mechanism (syscall 29) |
| `docs/handoff/SCENE_SETTINGS_INMEM_V1.md` | SceneAppearanceState, resolve_scene_render_tokens |
| `docs/handoff/SCENE_CUSTOM_COLOR_KEYS_V1.md` | ACTIVE_TINT_IDX, tint bundles |
| `docs/handoff/SCENE_SETTINGS_STORAGE_PLAN_V1.md` | Long-term storage model |
| `servers/silk-shell/src/main.rs` | Implementation |
| `servers/sexstore/src/main.rs` | OP_KV_GET=0xB0, OP_KV_PUT=0xB1, kv_reply |
