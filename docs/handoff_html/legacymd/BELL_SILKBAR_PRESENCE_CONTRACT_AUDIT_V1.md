# Bell SilkBar Presence Contract Audit V1

**Status:** Implemented (partial — SLOT_BELL grant pending).
**Date:** 2026-05-06
**Files changed:** 3 (+166 lines)
**Build:** `./scripts/entrypoint_build.sh` — PASS

---

## 1. Contract Audit Result

| Component | Audit result |
|---|---|
| **UpdateKind enum** | `SetBellPresence = 7` added at end. `_ => false` catch-all in `apply_update` ensures backward compatibility. No ABI_VERSION bump needed. |
| **SilkBarUpdate struct** | Unchanged at 16 bytes. `ABI_VERSION` assertion passes. |
| **SilkBar struct** | Added `bell_state: BellState` field. Struct is local to sexdisplay — not part of PDX ABI. No assertions check its size. |
| **apply_update** | New `case 7` stores `BellState` from packed `a` field. Unknown discriminants still return `false`. |
| **SLOT_BELL grant** | ❌ **Not granted to SilkBar.** Currently only silk-shell (PD 3) and sexbell itself have it. SilkBar's `pdx_call(SLOT_BELL, ...)` returns `ERR_CAP_INVALID`. |
| **Contract validation** | `validate_contract()` only checks `ABI_VERSION == SILK_DE_BAR_ABI_V1`. Adding UpdateKind at end doesn't change the version check. |

**Verdict: Contract-safe with one blocked dependency (SLOT_BELL grant).**

---

## 2. Changes by File

### crates/silkbar-model/src/lib.rs (+32 lines)
- Added `BellState` struct: `{ total_visible: u8, redacted_count: u8, flags: u8, _pad: u8 }`
- Added `bell_state: BellState` field to `SilkBar` struct
- Added `UpdateKind::SetBellPresence = 7`
- Added `case 7` in `apply_update`: unpacks `a` field into `bar.bell_state`
- Updated `DEFAULT_SILK_BAR` with default `BellState { 0, 0, 0, 0 }`

### servers/silkbar/src/main.rs (+60 lines)
- Added `use sex_pdx::{OP_BELL_LIST, SLOT_BELL}`
- Added reply listener in message dispatch: `msg.type_id == 1 && msg.caller_pd == 1` → forward as `SetBellPresence`
- Added Bell poll every ~2 seconds (`uptime_seconds % 2 == 0`):
  - Calls `pdx_call_checked(SLOT_BELL, OP_BELL_LIST, 0xFF, 0, 0)`
  - On `Ok`: poll enqueued, reply will arrive asynchronously
  - On `Err(ERR_CAP_INVALID)`: sends `SetBellPresence` with all zeros (dim dot)
- Budgeted markers: `[silkbar.bell.poll]`, `[silkbar.bell.reject]`, `[silkbar.bell.poll.reply]`

### servers/sexdisplay/src/main.rs (+74/-2 lines)
- Updated `bar_color` Bell section: dynamic color based on `bell_state`:
  - `flags & 1 == 0` (unavailable) → `DEFAULT_THEME.muted` (dim)
  - `total_visible == 0` (no events) → `DEFAULT_THEME.muted` (dim)
  - `redacted_count > 0` (privacy events) → amber `0x00FFAA44`
  - Otherwise → gold `0x00FFD700` (active)
- Added `bell_badge_at()` function: renders 1-2 digit count badge at right side of Bell slot
  - Uses same 5×7 FONT as clock digits
  - Clamped to 99 max
- Wired `bell_badge_at()` into both `render()` and `redraw_top_strip()` render loops
- Added `[sexdisplay.bell.render]` budgeted marker in `handle_silkbar_update`

### Not changed (blocked)
- `kernel/src/init.rs` — SLOT_BELL grant for SilkBar not added
- `docs/handoff/BELL_SILKBAR_PRESENCE_PLAN_V1.md` — superseded by this audit

---

## 3. SLOT_BELL Grant Status

| PD | Has SLOT_BELL? | Notes |
|---|---|---|
| sexbell (domain 10) | ✅ Self-capability | Granted at kernel init line 189-193 |
| silk-shell (PD 3) | ✅ Granted | Granted at kernel init line 108-111 |
| **silkbar** (?) | ❌ **Missing** | Needs `pd.grant_capability(SLOT_BELL, Domain(sexbell_id))` in init.rs |

**Required kernel init change (future, after contract review):**
```rust
// In kernel/src/init.rs, after SilkBar's SLOT_DISPLAY grant block:
if sexbell_id != 0 && silkbar_id != 0 {
    if let Some(pd) = DOMAIN_REGISTRY.get(silkbar_id) {
        pd.grant_capability(sex_pdx::SLOT_BELL, CapabilityData::Domain(sexbell_id));
        serial_println!("[kernel.sexbell.cap.silkbar] silkbar→bell slot=12");
    }
}
```

---

## 4. Polling Behavior (Current)

Without SLOT_BELL:
1. Every ~2s, SilkBar calls `pdx_call_checked(SLOT_BELL, OP_BELL_LIST, ...)` 
2. Kernel returns `Err(0xFFFF_FFFF_FFFF_FFFC)` = ERR_CAP_INVALID
3. SilkBar sends `SetBellPresence` with all zeros → sexdisplay renders dim dot
4. System continues normally — no hang, no crash

With SLOT_BELL (after grant):
1. Same poll call enqueues message in Bell's message ring
2. Bell processes LIST, replies via syscall 29
3. Reply arrives as `type_id=1, caller_pd=1` in SilkBar's listen buffer
4. SilkBar forwards packed counts as `SetBellPresence` to sexdisplay
5. Sexdisplay renders dot + count badge

---

## 5. Remaining Blocked Work

| Item | Blocked by | Action needed |
|---|---|---|
| SLOT_BELL grant for SilkBar | Kernel init change | Add grant in `kernel/src/init.rs` after SilkBar's SLOT_DISPLAY block |
| Actual Bell poll replies | SLOT_BELL grant | Poll will succeed once grant is in place |
| Full production testing | Boot test with grant | QEMU test with `qemuX.sh` |

---

## 6. Build Result

**PASS** — `sexos-v1.0.0.iso` produced. No new warnings. Pre-existing mutable static warnings in sexbell unchanged.
