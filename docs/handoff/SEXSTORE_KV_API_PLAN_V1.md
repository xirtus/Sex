# SEXSTORE_KV_API_PLAN_V1

## Status

Design (2026-05-04). Minimal sexstore K/V API for future Scene settings persistence.
Docs-only — no code changed.

---

## Verdict: SEXSTORE_KV_API_SAFE_TO_DESIGN ✅

Design is safe to document now.
**Implementation requires STOP FIRST** — kernel/src/init.rs must be modified to spawn sexstore and grant capabilities. That is a kernel edit and needs the STOP FIRST review.

| Requirement | Feasible? | Notes |
|-------------|-----------|-------|
| V1 value fits in single PDX call | ✅ | 8-byte value (1 u64) covers all V1 scene config |
| No heap in sexstore KV table | ✅ | Fixed-size `[KvSlot; 16]` static |
| No shared memory / no cross-PD pointer | ✅ | All data passed via PDX arg registers |
| Single-call GET and PUT | ✅ | 8-byte value → arg0=key, arg1=value, arg2=reserved |
| Return value encoding in PDX | ✅ | `pdx_call` returns `(u64, u64)` — status + value |
| No kernel change for API shape | ✅ for design | ❌ for implementation: kernel/init.rs spawn + cap grant needed |
| No ABI hash change now | ✅ for design | ❌ for implementation: sex-pdx gets new consts → hash update |
| Corrupt/absent value never fatal | ✅ | Checksum byte in value; caller always falls back to defaults |
| ERR_SERVICE_NOT_READY handled | ✅ | pdx_call returns sentinel; caller uses defaults |

---

## Current sexstore Audit

### `servers/sexstore/src/main.rs` (25 lines)

```rust
// Has: extern crate alloc, LockedHeap, alloc_error_handler, panic_handler
// Does: infinite loop in _start()
// Missing: pdx_listen_raw, any handler, any storage, any PDX registration
```

- No PDX listener
- No slot assignment
- No opcode handling
- Heap infrastructure present but unused
- **Not spawned** — `module_paths` in `kernel/src/init.rs` does not include `"sexstore"`
- No domain_id assigned

### `kernel/src/init.rs` spawn list (line 36)

```rust
let module_paths = ["sexdisplay", "sexdrive", "silk-shell", "sexinput", "sexusb", "silkbar", "linen"];
//                       1            2            3             4          5          6         7
```

sexstore = absent. Domain IDs 1–7 are taken. sexstore would be domain 8.

### Slot audit (`crates/sex-pdx/src/lib.rs` + `kernel/src/init.rs`)

| Slot | Constant | Service |
|------|----------|---------|
| 1 | `SLOT_STORAGE` | sexfiles VFS |
| 2 | `SLOT_SEXT` | sext demand pager |
| 3 | `SLOT_INPUT` | HID input ring |
| 4 | `SLOT_AUDIO` | audio server |
| 5 | `SLOT_DISPLAY` | sexdisplay compositor |
| 6 | `SLOT_SHELL` | silk-shell |
| 7 | `SLOT_SILKBAR` | silkbar |
| 8 | `SLOT_USB_HOST` | XHCI probe |
| 9 | `SLOT_USB_SEXINPUT` | sexusb→sexinput route (kernel-local const) |
| **10** | **`SLOT_SEXSTORE`** | **← assign here (first free)** |

### `servers/sexfiles/src/main.rs`

Phase 19 trampoline VFS. Multi-module, `extern crate spin`, `extern crate sex_rt`. Too complex for tiny K/V in V1. Not used.

---

## API Design

### Value model

V1 caps value at **8 bytes** (1 u64). Single-call GET and PUT.

This is sufficient for all V1 scene settings. Custom color arrays (32 bytes) are deferred until a settings app exists; until then, tint bundles are code-resident and ephemeral.

### Opcodes

Opcodes are per-slot (dispatched by sexstore's own listener), no collision with sexdisplay/silkbar:

```rust
pub const OP_KV_GET: u64 = 0xB0;  // silk-shell → sexstore: read value
pub const OP_KV_PUT: u64 = 0xB1;  // silk-shell → sexstore: write value
// 0xB2 = OP_KV_DEL: reserved, not V1
```

### Slot constant

```rust
pub const SLOT_SEXSTORE: u64 = 10;  // add to crates/sex-pdx/src/lib.rs
```

### Well-known key IDs

```rust
pub const KV_KEY_SCENE_APPEARANCE: u8 = 0x01;
// 0x00 = reserved/invalid
// 0x02..0xFF = reserved for future keys
```

### Error codes (returned in status position)

```rust
pub const KV_OK:        u64 = 0x00;  // success
pub const KV_NOT_FOUND: u64 = 0x01;  // key has no entry
pub const KV_FULL:      u64 = 0x02;  // PUT failed: no free slots in table
pub const KV_BAD_KEY:   u64 = 0x03;  // key_id = 0 or unknown opcode
```

### GET protocol

```rust
// Caller (silk-shell):
let (status, value) = pdx_call(SLOT_SEXSTORE, OP_KV_GET, key_id as u64, 0, 0);
// Returns:
//   status = KV_OK (0x00) → value is valid u64
//   status = KV_NOT_FOUND → value = 0; use defaults
//   status = KV_BAD_KEY  → key_id invalid; use defaults
// Caller must validate embedded checksum regardless of status.
```

### PUT protocol

```rust
// Caller (silk-shell):
let (status, _) = pdx_call(SLOT_SEXSTORE, OP_KV_PUT, key_id as u64, packed_value, 0);
// Returns:
//   status = KV_OK   → stored successfully
//   status = KV_FULL → table full; drop silently, non-fatal
//   status = KV_BAD_KEY → key invalid; drop silently
// Fire-and-forget: caller does not retry on KV_FULL.
```

### ERR_SERVICE_NOT_READY (sexstore not yet running)

`pdx_call` returns `(ERR_SERVICE_NOT_READY, 0)` when sexstore is not yet ready.
Caller must treat this the same as KV_NOT_FOUND: use defaults.

```rust
// ERR_SERVICE_NOT_READY = 0xFFFF_FFFF_FFFF_FFFE (defined in sex-pdx)
if status == ERR_SERVICE_NOT_READY || status == KV_NOT_FOUND {
    // use DEFAULT_SCENE_APPEARANCE
}
```

---

## Value Encoding: SceneMinimalBlob

8-byte packed u64. No heap. No pointer. Passed directly in pdx_call arg1.

```
Byte 0: magic   = 0xAC (Appearance Config marker)
Byte 1: version = 0x01
Byte 2: preset_idx (0..3; clamp to 0 if out of range on read)
Byte 3: chrome_flags (reserved; 0 in V1)
Byte 4: accessibility_flags (bit 0=high_contrast, bit 1=colorblind_safe)
Byte 5: reserved = 0
Byte 6: reserved = 0
Byte 7: checksum = byte0 ^ byte1 ^ byte2 ^ byte3 ^ byte4 ^ byte5 ^ byte6
```

### Pack (silk-shell PUT)

```rust
fn pack_scene_blob(preset_idx: u8, chrome: u8, access: u8) -> u64 {
    let magic:   u8 = 0xAC;
    let version: u8 = 0x01;
    let b = [magic, version, preset_idx, chrome, access, 0u8, 0u8, 0u8];
    let chk = b[0]^b[1]^b[2]^b[3]^b[4]^b[5]^b[6];
    u64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], chk])
}
```

### Unpack + validate (silk-shell GET)

```rust
fn unpack_scene_blob(v: u64) -> Option<(u8, u8, u8)> {
    let b = v.to_le_bytes();
    if b[0] != 0xAC || b[1] != 0x01 { return None; }  // wrong magic/version
    let expected_chk = b[0]^b[1]^b[2]^b[3]^b[4]^b[5]^b[6];
    if b[7] != expected_chk { return None; }            // checksum mismatch
    Some((b[2], b[3], b[4]))                            // (preset_idx, chrome, access)
}
```

Return `None` → use `DEFAULT_SCENE_APPEARANCE`. Never fatal.

**What is NOT persisted in V1:** custom_colors, ACTIVE_TINT_IDX. Tints are code-resident; custom colors require a settings app. Only preset_idx + flags are worth persisting now.

---

## sexstore Storage Model (for implementation)

Static in-RAM KV table. No heap. No disk in V1.

```rust
const KV_SLOT_COUNT: usize = 16;

struct KvSlot {
    occupied: bool,
    key_id: u8,
    value: u64,
}

static mut KV_TABLE: [KvSlot; KV_SLOT_COUNT] = [KvSlot { occupied: false, key_id: 0, value: 0 }; KV_SLOT_COUNT];
```

### GET handler logic

```rust
// Search occupied slots for key_id
for slot in KV_TABLE.iter() {
    if slot.occupied && slot.key_id == key_id {
        pdx_reply_with(KV_OK, slot.value);
        return;
    }
}
pdx_reply_with(KV_NOT_FOUND, 0);
```

### PUT handler logic

```rust
// Update in-place if key exists
for slot in KV_TABLE.iter_mut() {
    if slot.occupied && slot.key_id == key_id {
        slot.value = new_value;
        pdx_reply_with(KV_OK, 0);
        return;
    }
}
// Allocate free slot
for slot in KV_TABLE.iter_mut() {
    if !slot.occupied {
        slot.occupied = true;
        slot.key_id = key_id;
        slot.value = new_value;
        pdx_reply_with(KV_OK, 0);
        return;
    }
}
pdx_reply_with(KV_FULL, 0);
```

16 slots is 16 × 10 bytes = 160 bytes. No heap.

---

## Caller Responsibilities (silk-shell, future)

| Situation | Action |
|-----------|--------|
| Boot GET → KV_OK + valid checksum | Restore preset_idx + flags |
| Boot GET → KV_NOT_FOUND | Use DEFAULT_SCENE_APPEARANCE |
| Boot GET → KV_OK + bad checksum | Log warning, use defaults |
| Boot GET → ERR_SERVICE_NOT_READY | Use defaults (sexstore not ready) |
| F5/F6 change → PUT success | No action needed |
| F5/F6 change → PUT KV_FULL | Drop silently, non-fatal |
| PUT → ERR_SERVICE_NOT_READY | Drop silently, non-fatal |

Settings failure is **never fatal**. Boot MUST succeed regardless.

---

## Implementation File List (SEXSTORE_KV_RAM_V1)

### Modified

| File | Change | Notes |
|------|--------|-------|
| `servers/sexstore/src/main.rs` | Replace infinite loop with pdx_listen_raw + GET/PUT dispatch + static KV table | Remove unused heap infrastructure |
| `crates/sex-pdx/src/lib.rs` | Add `SLOT_SEXSTORE=10`, `OP_KV_GET=0xB0`, `OP_KV_PUT=0xB1`, `KV_KEY_SCENE_APPEARANCE=0x01`, `KV_OK/NOT_FOUND/FULL/BAD_KEY` | **Triggers ABI hash update** |
| `sexos_build_spec.toml` | Update `abi_version_hash` | Required after sex-pdx change |
| `kernel/src/init.rs` | Add `"sexstore"` to `module_paths`; assign domain_id=8; grant `SLOT_SEXSTORE` capability to silk-shell | **STOP FIRST — kernel edit** |

### NOT modified

| File | Reason |
|------|--------|
| `servers/silk-shell/src/main.rs` | No caller code yet — persistence is SCENE_SETTINGS_PERSIST_V1 |
| `servers/sexdisplay/src/main.rs` | Unrelated |
| `servers/sexfiles/src/main.rs` | Not used for this |
| `kernel/src/interrupts.rs` | Forbidden |

---

## STOP Conditions

| Condition | Action |
|-----------|--------|
| kernel/src/init.rs required for sexstore spawn | **STOP FIRST before implementing** — kernel edit |
| SLOT_SEXSTORE=10 conflicts | Re-audit slot table; use next free |
| Value > 8 bytes required | STOP — design multi-call GET/PUT (separate plan) |
| sexstore KV table needs heap | STOP — redesign as fixed static array |
| sex-pdx change breaks existing ABI | STOP — verify hash update |
| pdx_reply primitive not available | Audit sex-pdx; may need new helper in sexstore |

---

## Proof Markers (for SEXSTORE_KV_RAM_V1 implementation)

| Marker | When | Budget |
|--------|------|--------|
| `[sexstore.kv.put] key=N status=S` | PUT received | 32 |
| `[sexstore.kv.get] key=N status=S` | GET received | 32 |
| `[shell.scene.load] preset=N status=S` | Boot GET result in silk-shell | 1 |
| `[shell.scene.save] preset=N` | PUT fired in silk-shell on change | 16 |

---

## Build / Proof Plan (for SEXSTORE_KV_RAM_V1)

```bash
# Default
./scripts/entrypoint_build.sh

# Verify markers
grep -ac "\[sexstore.kv\]" /tmp/sexstore-kv-ram-v1.log
grep -ac "\[shell.scene.load\]" /tmp/sexstore-kv-ram-v1.log

# No panics
grep -acE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/sexstore-kv-ram-v1.log
```

---

## Future Multi-Call Extension (not V1)

If value > 8 bytes (e.g. full SceneSettingsBlob with custom_colors = 48 bytes), use chunked PUT/GET:

```
OP_KV_PUT_BEGIN = 0xB1: arg0=key, arg1=bytes_0_7, arg2=bytes_8_15 → opens write
OP_KV_PUT_END   = 0xB3: arg0=key, arg1=bytes_16_23, arg2=bytes_24_31 → 24-byte max
(repeat for larger values with a sequence counter in arg0_hi)
```

Design separately. V1 single-call 8-byte model is forward-incompatible with this; future key IDs could use a different internal slot size. Key this on a separate opcode range if needed.

---

## Pass Criteria

- [x] Verdict: SEXSTORE_KV_API_SAFE_TO_DESIGN
- [x] sexstore current state audited (stub, unspawned, no PDX)
- [x] Slot assignment: SLOT_SEXSTORE = 10 (first free after SLOT_USB_SEXINPUT = 9)
- [x] Opcodes: OP_KV_GET = 0xB0, OP_KV_PUT = 0xB1
- [x] Value encoding: 8-byte packed u64 (magic + version + preset + flags + checksum)
- [x] SceneMinimalBlob: pack/unpack pseudocode, no heap
- [x] Error codes: KV_OK / KV_NOT_FOUND / KV_FULL / KV_BAD_KEY
- [x] ERR_SERVICE_NOT_READY handling: use defaults
- [x] Caller responsibilities: all failure paths documented
- [x] Static KV table: 16 slots × 10 bytes = 160 bytes, no heap
- [x] Implementation file list with STOP FIRST flagged
- [x] STOP conditions documented
- [x] Proof markers named
- [x] NOT persisted in V1: custom_colors, ACTIVE_TINT_IDX
- [x] Next phase: SEXSTORE_KV_RAM_V1 (requires STOP FIRST for kernel edit)

---

## Next Phase: SEXSTORE_KV_RAM_V1

**STOP FIRST required before implementation** (kernel/src/init.rs change).

After STOP FIRST review:

1. `kernel/src/init.rs`: add `"sexstore"` to `module_paths` (domain_id=8); grant `SLOT_SEXSTORE` cap to silk-shell
2. `crates/sex-pdx/src/lib.rs`: add `SLOT_SEXSTORE`, `OP_KV_GET/PUT`, key IDs, error codes
3. `sexos_build_spec.toml`: update `abi_version_hash`
4. `servers/sexstore/src/main.rs`: replace loop with pdx_listen_raw handler + static KV table
5. Build: `./scripts/entrypoint_build.sh`
6. Verify: PUT from test harness → stored; GET returns same value; NOT_FOUND for unknown key; defaults used if FULL
7. Create `docs/handoff/SEXSTORE_KV_RAM_V1.md`

After SEXSTORE_KV_RAM_V1: proceed to **SCENE_SETTINGS_PERSIST_V1** (silk-shell reads scene blob at boot, writes on F5/F6).

---

## References

| Doc | Relevance |
|-----|-----------|
| `docs/handoff/SCENE_SETTINGS_STORAGE_PLAN_V1.md` | V1 static model, full 48-byte blob design (future), storage infra audit |
| `docs/handoff/SCENE_SETTINGS_INMEM_V1.md` | `SceneAppearanceState`, `preset_idx`, `chrome_flags`, `accessibility_flags` |
| `docs/handoff/SCENE_CUSTOM_COLOR_KEYS_V1.md` | `ACTIVE_TINT_IDX` — ephemeral, NOT persisted in V1 |
| `servers/sexstore/src/main.rs` | Stub — 25 lines, no PDX |
| `crates/sex-pdx/src/lib.rs` | Slot table, `ERR_SERVICE_NOT_READY`, `pdx_call` return contract |
| `kernel/src/init.rs` | module_paths spawn list, `grant_capability` pattern |
