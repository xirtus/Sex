# SCENE_SETTINGS_PERSIST_PLAN_V1

## Status

Design (2026-05-04). Scene Appearance settings persistence through sexstore RAM K/V.
Docs-only — no code changed.

---

## Verdict: SCENE_SETTINGS_PERSIST_SAFE_WITH_RAM_SEXSTORE ✅

| Requirement | Feasible? | Notes |
|-------------|-----------|-------|
| One u64 value fits preset_idx + chrome_flags + accessibility_flags | ✅ | 8 bytes → 3 fields + header + checksum fits easily |
| GET reply received in silk-shell main loop | ✅ | `pdx_call` enqueues async; reply arrives as `type_id=0x1` on `pdx_listen_raw(0)` |
| No blocking / no timeout needed | ✅ | Fire-and-forget PUT; GET handled in main loop when reply arrives |
| sexstore absent never fatal | ✅ | `pdx_call` returns `ERR_SERVICE_NOT_READY` → skip GET, keep defaults |
| Corrupt/missing blob never fatal | ✅ | Magic/version/checksum validate; defaults used on failure |
| No heap | ✅ | Packing/unpacking uses stack u64 |
| No kernel changes | ✅ | sexstore already spawned; silk-shell already has SLOT_SEXSTORE cap |
| No sex-pdx ABI hash change | ✅ | Opcodes copied locally in silk-shell; promoted later |

### Not blocked

- **No kernel/ABI edit needed** — sexstore is already spawned, SLOT_SEXSTORE cap already granted
- **No new IPC pattern** — `pdx_listen_raw(0)` + `pdx_call` is existing infrastructure
- **No heap** — packing/unpacking uses stack-only `u64` and `[u8; 8]`

---

## u64 Packed Layout: SceneSettingsBlob

```rust
/// 8-byte blob persisted as a single u64 in sexstore.
/// Key: KV_KEY_SCENE_APPEARANCE = 0x01
///
/// Byte layout (little-endian: byte 0 = LSB when viewed as u64):
///
/// [7:0]   byte 0: magic      = 0xAC
/// [15:8]  byte 1: version    = 0x01
/// [23:16] byte 2: preset_idx = 0..3 (clamp on read)
/// [31:24] byte 3: chrome_flags  (reserved; 0 in V1)
/// [39:32] byte 4: accessibility_flags (bit 0=high_contrast, bit 1=colorblind_safe)
/// [47:40] byte 5: reserved = 0
/// [55:48] byte 6: reserved = 0
/// [63:56] byte 7: checksum = XOR(byte0 .. byte6)
```

### Pack (for PUT)

```rust
fn pack_scene_blob(preset_idx: u8, chrome: u8, access: u8) -> u64 {
    let b: [u8; 8] = [
        0xAC,          // magic
        0x01,          // version
        preset_idx,
        chrome,
        access,
        0u8,           // reserved
        0u8,           // reserved
        0u8,           // placeholder for checksum
    ];
    let chk: u8 = b[0] ^ b[1] ^ b[2] ^ b[3] ^ b[4] ^ b[5] ^ b[6];
    let mut blob = b;
    blob[7] = chk;
    u64::from_le_bytes(blob)
}
```

### Unpack + validate (for GET reply)

```rust
fn unpack_scene_blob(v: u64) -> Option<(u8, u8, u8)> {
    let b: [u8; 8] = v.to_le_bytes();
    if b[0] != 0xAC || b[1] != 0x01 {
        return None;  // wrong magic or version
    }
    let expected: u8 = b[0] ^ b[1] ^ b[2] ^ b[3] ^ b[4] ^ b[5] ^ b[6];
    if b[7] != expected {
        return None;  // checksum mismatch
    }
    Some((b[2], b[3], b[4]))
    // Caller clamps preset_idx to PRESET_COUNT-1 before using
}
```

Return `None` → use `DEFAULT_SCENE_APPEARANCE`. Never fatal.

### Key ID

```rust
const KV_KEY_SCENE_APPEARANCE: u64 = 0x01;
// 0x00 = reserved/invalid; 0x02+ = future keys
```

---

## Caller Opcodes (local to silk-shell)

Copied from sexstore's own consts. Not added to sex-pdx in this phase.

```rust
// Opcodes (match sexstore/src/main.rs)
const OP_KV_GET: u64 = 0xB0;
const OP_KV_PUT: u64 = 0xB1;

// PUT reply values (fire-and-forget; caller may ignore)
const KV_PUT_OK:   u64 = 0x00;
const KV_PUT_FULL: u64 = 0x02;
```

### Why local, not sex-pdx?

| Factor | Local | sex-pdx |
|--------|-------|---------|
| ABI hash update | None | Required |
| Duplication | Two files (sexstore + silk-shell) | Single source |
| Correctness risk | Low (compile-time consts) | None |
| Migration path | Promote in SCENE_SETTINGS_PERSIST_V1 follow-up | Not needed |

Local consts acceptable for V1. **Promoting to sex-pdx is worth a STOP FIRST review**
if the duplication becomes maintenance burden.

---

## Boot Load Sequence

### Phase 1: Send defaults (before GET reply)

Current behavior preserved. At boot, before entering main loop:

```
1. Initialize SCENE_APPEARANCE_STATE = DEFAULT_SCENE_APPEARANCE
2. Send resolved tokens to sexdisplay (send_scene_render_tokens())
   → screen shows BottleGlass immediately regardless of storage state
```

### Phase 2: Fire GET

Immediately after sending defaults:

```rust
unsafe fn boot_load_scene_settings() {
    let (status, _) = pdx_call(SLOT_SEXSTORE, OP_KV_GET, KV_KEY_SCENE_APPEARANCE, 0, 0);
    if status == 0 {
        SEXSTORE_LOAD_PENDING = true;
        // marker: [shell.scene.load] status=1 (pending)
    } else {
        // sexstore not available — keep defaults
        // marker: [shell.scene.load] status=ERR status
    }
}
```

Added after `send_scene_render_tokens()` call at boot, before entering main loop.

### Phase 3: Receive reply in main loop

In the main loop's match on `msg.type_id`, add a new arm for `0x1`:

```rust
// Inside main loop: match msg.type_id { ... }
0x1 => {
    // Reply from sexstore (GET result or PUT ack)
    unsafe {
        if SEXSTORE_LOAD_PENDING {
            SEXSTORE_LOAD_PENDING = false;
            handle_sexstore_get_reply(msg.arg0);
        }
        // PUT acks are fire-and-forget; ignored
    }
}
```

### Phase 4: Validate and apply

```rust
unsafe fn handle_sexstore_get_reply(value: u64) {
    if let Some((preset, chrome, access)) = unpack_scene_blob(value) {
        let clamped_preset = if (preset as usize) < PRESET_COUNT { preset } else { 0 };
        SCENE_APPEARANCE_STATE.preset_idx = clamped_preset;
        SCENE_APPEARANCE_STATE.chrome_flags = chrome;
        SCENE_APPEARANCE_STATE.accessibility_flags = access;
        // use_custom_colors and ACTIVE_TINT_IDX are NOT persisted — reset to 0
        SCENE_APPEARANCE_STATE.use_custom_colors = 0;
        ACTIVE_TINT_IDX = 0;
        // Re-send tokens with restored settings
        let tokens = resolve_scene_render_tokens();
        push_token_preset(&tokens);
        // [shell.scene.load] preset=N chrome=N access=N (budget 1)
    }
    // If None → corrupted or missing; keep defaults already sent at boot
}
```

### Fail-closed invariant

If the GET reply never arrives (sexstore crashed after receiving the GET, or reply is lost), silk-shell continues running with defaults. The screen remains usable. Settings persistence is best-effort.

---

## Save Sequence

### Trigger: F5 (cycle preset)

In `cycle_scene_render_token_preset()`, after state update and token push:

```rust
unsafe fn cycle_scene_render_token_preset() {
    SCENE_APPEARANCE_STATE.preset_idx =
        (SCENE_APPEARANCE_STATE.preset_idx + 1) % PRESET_COUNT as u8;
    SCENE_APPEARANCE_STATE.use_custom_colors = 0;
    ACTIVE_TINT_IDX = 0;
    let tokens = resolve_scene_render_tokens();
    push_token_preset(&tokens);

    // ── PERSIST ──
    let blob = pack_scene_blob(
        SCENE_APPEARANCE_STATE.preset_idx,
        SCENE_APPEARANCE_STATE.chrome_flags,
        SCENE_APPEARANCE_STATE.accessibility_flags,
    );
    pdx_call(SLOT_SEXSTORE, OP_KV_PUT, KV_KEY_SCENE_APPEARANCE, blob, 0);
    // Fire-and-forget: no reply wait, no retry on KV_FULL
    // [shell.scene.save] preset=N (budget 16)
}
```

### No write on F6 (tint cycle)

F6 changes ephemeral state only (ACTIVE_TINT_IDX, use_custom_colors, custom_colors).
These are NOT persisted in V1. No write.

### Fire-and-forget rationale

| Scenario | Behavior |
|----------|----------|
| sexstore processes PUT → KV_OK | No-op |
| sexstore table full → KV_FULL | Setting lost; non-fatal, next F5 retries |
| sexstore not ready | `pdx_call` returns ERR_SERVICE_NOT_READY; no crash |
| Reply delayed | Ignored — main loop does not block |

---

## Timeout / Nonblocking Policy

| Operation | Policy | Rationale |
|-----------|--------|-----------|
| GET at boot | Fire-and-forget via `pdx_call`; reply arrives later in main loop | No blocking; screen works with defaults until reply arrives |
| PUT on F5 | Fire-and-forget via `pdx_call`; reply ignored | No blocking; next F5 will overwrite |
| GET reply handling | Process immediately in main loop; no retry | Single attempt; next boot retries |

**No timeout mechanism** exists in the kernel or sex-pdx for async calls.
V1 does not need one — the async reply pattern is inherently non-blocking.

---

## State Flag

```rust
/// Set when a GET is in flight. Cleared when the reply is received.
/// Guards against misinterpreting a PUT ack as a GET value.
static mut SEXSTORE_LOAD_PENDING: bool = false;
```

Only one GET is ever in flight (at boot). PUT is fire-and-forget.

---

## What Is / Is Not Persisted V1

| Field | Persisted? | Reason |
|-------|------------|--------|
| `preset_idx` | ✅ Yes | Core setting; restored to last used preset |
| `chrome_flags` | ✅ Yes | Reserved for future chrome controls |
| `accessibility_flags` | ✅ Yes | Reserved for future accessibility toggles |
| `use_custom_colors` | ❌ No | Always reset to 0 (off); custom colors require settings app |
| `custom_colors[8]` | ❌ No | 32 bytes exceeds u64; requires settings app UI |
| `ACTIVE_TINT_IDX` | ❌ No | Ephemeral; always resets to 0 (Clear) |

---

## Implementation File List (SCENE_SETTINGS_PERSIST_V1)

### Modified

| File | Change |
|------|--------|
| `servers/silk-shell/src/main.rs` | Add local opcodes; add `pack_scene_blob`, `unpack_scene_blob`, `handle_sexstore_get_reply`, `boot_load_scene_settings`; add `SEXSTORE_LOAD_PENDING` flag; add `0x1` match arm in main loop; add persist call in `cycle_scene_render_token_preset`; add markers |

### NOT modified

| File | Reason |
|------|--------|
| `servers/sexstore/src/main.rs` | No change needed; GET/PUT already implemented |
| `crates/sex-pdx/src/lib.rs` | Opcodes local to silk-shell; no ABI hash update |
| `kernel/` | Forbidden — no kernel changes |
| `sexos_build_spec.toml` | No ABI hash change (no sex-pdx edit) |
| `limine.cfg` | No new binaries |
| `servers/sexdisplay/` | Unrelated |
| `servers/sexfiles/` | Not used |

### Forbidden files (any phase)

- `kernel/` — no kernel changes at any phase
- `crates/sex-pdx/src/lib.rs` unless promoted opcodes justify it

---

## Markers (for implementation)

| Marker | When | Budget |
|--------|------|--------|
| `[shell.scene.load] status=N` | Boot GET fired: 0=ok, 1=pending, err=ERR | 1 |
| `[shell.scene.load] preset=N chrome=N access=N` | GET reply applied successfully | 1 |
| `[shell.scene.load] corrupt` | GET reply failed validation | 1 |
| `[shell.scene.save] preset=N` | F5 fired PUT | 16 |
| `[shell.scene.save] not-ready` | PUT detected ERR_SERVICE_NOT_READY | 16 |

Existing markers from SCENE_SETTINGS_INMEM_V1 and SCENE_CUSTOM_COLOR_KEYS_V1 retained.

---

## Build / Proof Plan (for SCENE_SETTINGS_PERSIST_V1)

```bash
# Build
./scripts/entrypoint_build.sh

# Verify ISO
[SEXOS ENTRYPOINT] success

# Run (optional)
env SEXUSB_QEMU_DISPLAY=sdl-grab SEXUSB_QEMU_DEVICE=mouse ./dev.sh run 2>&1 | tee /tmp/scene-settings-persist-v1.log

# Check markers
grep -ac "\[shell.scene.load\]" /tmp/scene-settings-persist-v1.log
grep -ac "\[shell.scene.save\]" /tmp/scene-settings-persist-v1.log

# Verify no panics
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/scene-settings-persist-v1.log
```

---

## STOP Conditions

| Condition | Action |
|-----------|--------|
| Kernel reply path (syscall 29 → `incoming_replies`) not working for silk-shell at runtime | **STOP** — verify with `pdx_listen_raw(0)` test in silk-shell |
| `pdx_listen_raw(0)` in silk-shell main loop receives `type_id=0x1` for non-sexstore reason | **STOP** — investigate source; add guard or use distinct range |
| sexstore opcodes promoted to sex-pdx before SCENE_SETTINGS_PERSIST_V1 | Accept ABI hash update; document in plan |
| Packed u64 needs more than 8 bytes (e.g. partial custom_colors) | **STOP** — redesign as multi-key or wait for sexstore value expansion |
| Settings app needed before persistence | Defer persistence; keep in-memory only |
| sexstore KV corrupts or loses data after write | Investigate sexstore implementation; add marker tracking |
| silk-shell heap needed for reply handling | **STOP** — redesign with stack-only unpacking (already designed this way) |

---

## Next Phase: SCENE_SETTINGS_PERSIST_V1

Implement persistence in silk-shell:

1. Add local opcodes and key constant to silk-shell `src/main.rs`
2. Add `pack_scene_blob()` / `unpack_scene_blob()` functions
3. Add `static mut SEXSTORE_LOAD_PENDING: bool`
4. Add `boot_load_scene_settings()` — fire GET, set pending flag
5. Add `handle_sexstore_get_reply()` — validate blob, apply state, re-send tokens
6. Add `0x1` match arm in main loop dispatch
7. Add persist call after F5 preset cycle
8. Add markers (`[shell.scene.load]`, `[shell.scene.save]`)
9. Build: `./scripts/entrypoint_build.sh`
10. Create `docs/handoff/SCENE_SETTINGS_PERSIST_V1.md`

After SCENE_SETTINGS_PERSIST_V1: optional SCENE_SETTINGS_PERSIST_ABI_V1 (promote opcodes to sex-pdx) or next feature.

---

## References

| Doc | Relevance |
|-----|-----------|
| `docs/handoff/SEXSTORE_KV_RAM_V1.md` | sexstore K/V implementation (kv_reply via syscall 29) |
| `docs/handoff/SEXSTORE_KV_API_PLAN_V1.md` | API design for K/V (packed blob, opcodes, error codes) |
| `docs/handoff/SEXSTORE_KERNEL_ENABLE_V1.md` | sexstore spawn, SLOT_SEXSTORE cap grant |
| `docs/handoff/SCENE_SETTINGS_STORAGE_PLAN_V1.md` | Storage model, SceneSettingsBlob design, ownership |
| `docs/handoff/SCENE_SETTINGS_INMEM_V1.md` | SceneAppearanceState, resolve_scene_render_tokens |
| `docs/handoff/SCENE_CUSTOM_COLOR_KEYS_V1.md` | ACTIVE_TINT_IDX, TINT_COUNT, cycle_custom_tint |
| `servers/silk-shell/src/main.rs` | `pdx_listen_raw(0)` main loop, cycle handlers |
| `servers/sexstore/src/main.rs` | OP_KV_GET=0xB0, OP_KV_PUT=0xB1, kv_reply |
| `crates/sex-pdx/src/lib.rs` | SLOT_SEXSTORE=10, pdx_call, pdx_listen_raw |
