# Bell V1 SilkBar Presence Plan

**Status:** Design only — no implementation.
**Date:** 2026-05-06
**Files consulted:** `servers/sexbell/src/main.rs`, `servers/silkbar/src/main.rs`, `crates/silkbar-model/src/lib.rs`, `crates/sex-pdx/src/lib.rs`, `servers/silk-shell/src/main.rs`

---

## 1. Existing LIST Route: Usability Assessment

| Question | Answer |
|---|---|
| Does OP_BELL_LIST (0xC3) exist? | ✅ Yes, defined in `sex-pdx` and handled in Bell's main loop |
| Can SilkBar reach Bell via PDX? | ✅ SilkBar has slot access (kernel init grants caps) |
| Does Bell currently reply to LIST? | ❌ **No.** Bell uses `pdx_listen_raw` + implicit continue. No `pdx_reply` is called anywhere in Bell's main loop. Any `pdx_call(SLOT_BELL, OP_BELL_LIST, ...)` from SilkBar would **block forever**. |
| Can aggregate counts be returned without ABI change? | ✅ **Yes.** Adding `pdx_reply(caller_pd)` to Bell's LIST handler is a server-side code change, not an ABI change. The opcode number, argument layout, and caller contract remain identical. The return register encoding (RAX=status, RSI=value) is an implementation detail, already used by other PDX servers. |
| Can per-lane counts fit in a 64-bit register? | ✅ Yes. 6 lanes × 4 bits per lane = 24 bits. Plus 8 bits for total count. Fits easily in RSI. |
| Is adding `pdx_reply` backward-compatible? | ✅ **Yes.** No current caller uses LIST with `pdx_call` (because it would hang), so no existing code breaks. |

**Verdict: USABLE, with one required Bell-server change (add `pdx_reply` to LIST handler).**

---

## 2. V1 Presence UI Model

### What SilkBar shows (maximum)

```
[≡] [●] [●] [○] [○] [○]   [📶] [📶] [🔋] [●2] [12:00]
  ws0  ws1  ws2  ws3  ws4    net  wifi  bat  bell  clock
                                              ↑
                                        Bell presence dot
                                        with count badge
```

The Bell presence icon is a **generic dot** in the reserved `ModuleSlot::Bell` position (`CHIP_X_BELL = 1020`). It communicates:

| State | Visual | Meaning |
|---|---|---|
| No events (poll returned 0) | Dim/off dot (muted color) | Quiet — no active Bell events |
| Active events, 0 redacted | Solid dot, count badge | N active events visible to SilkBar |
| Active events, some redacted | Solid dot, count = total - redacted, plus privacy indicator | Some FullHidden events exist but are not counted in badge |
| Bell unavailable (poll timeout/error) | Dim/off dot with warning tint | Bell server not responding |
| Muted senders active | Dot with mute indicator (e.g., dimmer fill or smaller) | At least one muted sender has been rejected recently |

### What SilkBar must NOT show

| Feature | Reason |
|---|---|
| Event body/title | Not available via LIST summary; even if technically reachable, SilkBar has no privacy context |
| Sender identity (PD number) | No user-visible mapping from PD to app name in V1 |
| Action labels | V1 actions are marker-only; no label data exists |
| Object references | Not meaningful without Collar; not safe to display |
| Full per-event details | SilkBar is a compact bar, not a notification center |
| Redacted event content | FullHidden events must not leak even aggregated hints beyond count |

### V1 rendering constraints

- **Dot only** — no text banner, no popup, no notification slide-in
- **Count badge** — decimal digit(s) next to dot, max display value 99 (clamp)
- **Color** — single semantic token, reused from existing palette (use `urgent` or `active` tint)
- **Size** — fits in existing `CHIP_W=18` × `CHIP_H=22` slot (same as status chips)

---

## 3. Privacy Rules

| Rule | Enforcement |
|---|---|
| SilkBar receives only aggregate counts | Bell's LIST handler computes counts; SilkBar never sees individual `BellQueueEntry` fields |
| FullHidden events are excluded from all counts visible to SilkBar | `max_privacy_for_caller(silkbar_pd)` must return ≤2 if SilkBar is not the shell (PD 3). Currently the function only gives max privacy to PD 3. **SilkBar PD must be added to the privacy allowlist** or given its own privacy tier. |
| Redacted event count is a separate private counter | `[bell.list.redact]` marker is budgeted; return register carries separate "visible count" and "redacted count" |
| No sender PD numbers reach SilkBar | Aggregate counts only |
| Mute status is a boolean flag, not per-sender list | Bell can report "has_muted_recently" as a single bit |

---

## 4. Polling Cadence

| Property | Value | Rationale |
|---|---|---|
| **Frequency** | Once per 2 seconds | Matches SilkBar's existing ~1Hz clock loop; every other tick to reduce bus load |
| **Mechanism** | `pdx_call(SLOT_BELL, OP_BELL_LIST, ...)` with lane_filter=0xFF | Returns aggregate counts for all lanes in one call |
| **Yield pattern** | Reuse existing `sys_yield()` × 100 + `get_ticks()` gate | No new timer or interrupt needed — piggyback on SilkBar's existing second-granularity loop |
| **Blocking concern** | `pdx_call` blocks until Bell replies | Bell's LIST handler is O(queue_size) = O(16) — negligible latency. No busy wait. |
| **Error handling** | If `pdx_call_checked` returns Err, skip update this cycle | Bell may be temporarily busy or not started yet. SilkBar continues. |
| **Stagger** | Offset from clock send by ~500ms (every other second) | Avoids clustering PDX calls on the same tick |

### Integration point in SilkBar's main loop

The existing loop (lines 125–185 of `silkbar/src/main.rs`) already has:
```rust
for _ in 0..100 { sex_pdx::sys_yield(); }
let ticks = sex_pdx::get_ticks();
let uptime_seconds = ticks / LAPIC_TICKS_PER_SECOND_APPROX;
if uptime_seconds == last_uptime_seconds { continue; }
last_uptime_seconds = uptime_seconds;
```

The Bell poll would be inserted after the yield gate, gated on `uptime_seconds % 2 == 0`:

```
if uptime_seconds % 2 == 0 {
    let result = pdx_call_checked(SLOT_BELL, OP_BELL_LIST, 0xFF, 0, 0);
    if let Ok(packed_counts) = result {
        // Update bell presence state
        send_update(UpdateKind::SetBellPresence, ...);
    }
}
```

---

## 5. Ownership Boundaries

| Layer | Owns | Does NOT own |
|---|---|---|
| **Bell** (`sexbell`) | Event policy, queue, privacy redaction, mute list, LIST response computation | Rendering, UI state, clock |
| **SilkBar** (`silkbar`) | Polling cadence, update dispatch to sexdisplay, presence state machine | Event data, per-event privacy decisions, mute policy |
| **SilkBar model** (`silkbar-model`) | `UpdateKind` variant definitions, layout constants, `SilkBar` struct | Any event-specific fields |
| **Sexdisplay** (`sexdisplay`) | Framebuffer rendering of the Bell dot + count | Polling, event logic, Bell state |
| **Shell** (`silk-shell`) | Focus/context (future), Bell click-to-open action | Bell presence rendering |

### New ABI surface required

**Minimal addition to `crates/silkbar-model/src/lib.rs`:**

```rust
// In enum UpdateKind:
SetBellPresence = 7,   // a=visible_count, b=redacted_count|mute_flag<<8
```

This is a **model extension, not an opcode ABI change**. The `UpdateKind` enum is internal to the SilkBar→sexdisplay protocol (OP_SILKBAR_UPDATE dispatch), not a Bell opcode. Sexdisplay would need a rendering case for this new kind (sexdisplay change, not sexbell change).

---

## 6. Blocked Items

| Item | Blocked by | Workaround |
|---|---|---|
| Bell LIST must call `pdx_reply` | Bell code change | Required before any SilkBar integration. Not an ABI change. |
| Privacy tier for SilkBar PD | Bell must know SilkBar's PD number | Add SilkBar PD to `max_privacy_for_caller()` or give it tier 1 (may see Public + Sensitive but not FullHidden) |
| New `SetBellPresence` UpdateKind | silkbar-model crate change | Add as `UpdateKind::SetBellPresence = 7` — compatible (ABI_VERSION stays 3 since this is an addition, not a renumbering) |
| Sexdisplay rendering for Bell dot | sexdisplay change | Must render dot + count badge in the Bell layout slot |
| Collar/action integration | Phase E | Not needed for V1 presence dot |
| Per-app policy overrides | OP_BELL_SET_POLICY not implemented | Ignored in V1; all senders get PASSIVE lane |
| Push/subscribe | OP_BELL_SUBSCRIBE not implemented | Polling avoids this dependency entirely |

---

## 7. Implementation Boundary (what does NOT happen)

| Feature | Status |
|---|---|
| New Bell opcode | ❌ Not added |
| Push/subscribe | ❌ Not implemented |
| Collar PD/cap integration | ❌ Not started |
| Storage/audio dependency | ❌ Not introduced |
| Event body in SilkBar | ❌ Not displayed |
| Sender identity in SilkBar | ❌ Not exposed |
| Action callback dispatch | ❌ Marker-only, unchanged |
| Shared memory / backing buffer | ❌ Not used |
| Bell policy in SilkBar | ❌ Not moved |
| FullHidden content leak | ❌ Not possible |
| ABI_VERSION bump | ⚠️ Only if new UpdateKind changes enum discriminant layout. Adding `SetBellPresence = 7` at the end is backward-compatible. |

---

## 8. Smallest Future Implementation Sequence

### Step 1: Bell — make LIST reply with aggregate counts

**File:** `servers/sexbell/src/main.rs`
**Change:** In the `OP_BELL_LIST` handler, after the iteration loop, compute packed counts and call `pdx_reply`.

```rust
// After the existing LIST processing loop, before match break:
let packed_counts = (total_count as u64) << 48
                  | (lane0_count as u64) << 40
                  | (lane1_count as u64) << 32
                  | (lane2_count as u64) << 24
                  | (lane3_count as u64) << 16
                  | (lane4_count as u64) << 8
                  | (lane5_count as u64);
// Set RAX=0 (success), RSI=packed_counts
// The kernel will set these registers before returning to caller
pdx_reply(msg.caller_pd);
```

**No ABI change.** Same opcode, same args. Caller now gets a response.

### Step 2: SilkBar model — add SetBellPresence UpdateKind

**File:** `crates/silkbar-model/src/lib.rs`
**Change:** Add `SetBellPresence = 7` to `UpdateKind` enum.

### Step 3: SilkBar — add polling call in main loop

**File:** `servers/silkbar/src/main.rs`
**Change:** Every ~2 seconds, call `pdx_call_checked(SLOT_BELL, OP_BELL_LIST, ...)`. On success, `send_update(UpdateKind::SetBellPresence, ...)`.

### Step 4: Sexdisplay — render Bell dot

**File:** `servers/sexdisplay/src/main.rs`
**Change:** Handle `SetBellPresence` update. Render a dot + count badge in the Bell layout slot. Use existing color tokens (urgent tint for active, muted tint for quiet/error).

### Not yet (Phase E+):

- Click-to-open Bell panel (requires `Action::OpenBell` handler in sexdisplay/shell)
- Per-event list view (requires shell UI, not SilkBar)
- Collar integration for action dispatch
- Push-based updates via OP_BELL_SUBSCRIBE

---

## 9. Risk Summary

| Risk | Likelihood | Mitigation |
|---|---|---|
| Bell blocks during `pdx_reply` setup | Low | LIST handler is O(16), no allocation, no blocking calls |
| Privacy tier for SilkBar PD wrong | Medium | Use PD-based lookup; shell(3) gets full view, silkbar gets tier 1 (Sensitive max) |
| Count wraps or overflows | Low | 4 bits per lane (max 15), total 8 bits (max 255). Queue is 16 entries max — cannot overflow. |
| Poll timeout/error floods | Low | Single poll per 2s; error → skip, no retry storm |
| `ABI_VERSION` mismatch | Low | Add `SetBellPresence` at end of enum, increment ABI_VERSION to 4 only if sexdisplay requires it for dispatch |

---

## 10. Summary

| Item | Status |
|---|---|
| LIST route usable? | ✅ Yes, with one Bell change (add `pdx_reply`) |
| V1 presence model defined | ✅ Generic dot + count badge, 6 lane aggregates |
| Privacy boundaries explicit | ✅ Aggregate counts only; FullHidden excluded; no sender/body/action exposure |
| Polling cadence defined | ✅ ~0.5 Hz, piggyback on existing SilkBar yield loop |
| Ownership boundaries clear | ✅ Bell→data, SilkBar→poll+dispatch, sexdisplay→render |
| Blocked items listed | ✅ Bell reply, privacy tier, model enum, sexdisplay render |
| Smallest implementation sequence | ✅ 4 steps: Bell reply → model enum → SilkBar poll → sexdisplay render |
| No ABI changes required | ✅ |
| No Collar/storage/audio dependencies | ✅ |
