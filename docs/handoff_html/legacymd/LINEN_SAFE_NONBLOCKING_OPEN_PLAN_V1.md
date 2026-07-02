# Linen Safe Nonblocking Open Plan V1

## Status: PASS (Diagnostic Audit + Design Plan Only)
Date: 2026-05-14
Attempts: 1
Implementation: **NONE** — diagnostics and design plan only

## Files Inspected

| File | Lines Reviewed | Purpose |
|------|---------------|---------|
| `servers/silk-shell/src/main.rs` | 1112-1241, 1570-1710, 2532-2540, 4959-5012, 7229-7352, 9968-10020, 10320-10370, 14860-15015, 14993-15012 | Full blocking chain audit |
| `servers/linen/src/main.rs` | 175-197, 258-298, 620-676, 796-900 | Server-side handlers + storage sync |
| `apps/spindle/src/main.rs` | 804-876 | AsyncEnqueue reference model |

## Exact Blocking Chains

### Chain 1: Initial Linen Surface Paint (first-open only)
```
linen_paint_surface()                          @ silk-shell:1231
  → LINEN_REMOTE_FETCHED guard (once)
  → linen_fetch_remote_snapshot()              @ silk-shell:1155
    → for slot in 0..16:
        pdx_call(SLOT_LINEN, GET_PUBLIC_SNAPSHOT)
        → linen_sync_reply()                   @ silk-shell:1116  ← BLOCKS
        → for chunk in 0..3:
            pdx_call(SLOT_LINEN, GET_PUBLIC_NAME)
            → linen_sync_reply()               @ silk-shell:1116  ← BLOCKS
```
**Max blocks**: 16 snapshots + 16×3 name chunks = up to 64 sync_reply calls.
**Trigger**: Called once after main loop starts (line 14041), deferred past all synthetic proofs.
**HID drain**: OP_HID_EVENT messages processed in-line during wait (mouse keeps working).

### Chain 2: Palette Open Linen (FocusLinen command)
```
palette_execute_selected()                     @ silk-shell:9968
  → Command::FocusLinen (line 10001)
    → open_linen_in_active_scene()              @ silk-shell:7229
      → [frame management: LOCAL, SAFE]
      → linen_paint_surface()                   @ silk-shell:1231 ← BLOCKS first time
```
**Blocking risk**: Confirmed at line 10338:
```
[shell.palette.exec] idx=3 action=Open Linen ok=0 reason=blocking_risk_confirmed
```
**Guard**: `COMMAND_PALETTE_DAILY_PROOF_ACTIVE` gates it out during proof (line 10002).

### Chain 3: F8 ToggleLinen Keyboard Shortcut
```
handle_hid_event()                             @ silk-shell:4961
  → SurfaceAction::ToggleLinen (line 15269)
    → toggle_linen()                            @ silk-shell:7330
      → focus_or_open_linen()                   @ silk-shell:7307
        → open_linen_in_active_scene()          ← BLOCKS via linen_paint_surface
```

### Chain 4: Mesh Detail Enter → Linen Focus (N11)
```
handle_hid_event() → mesh keyboard handler (line 14863)
  → Enter on selected mesh fact
  → mesh_focus_linen_at_selected_fact()          @ silk-shell:2535
    → open_linen_in_active_scene()               @ silk-shell:2537
    → linen_paint_surface()                      @ silk-shell:2539 ← BLOCKS first time
```

### Chain 5: J4/N14 Linen Object → Quil Buffer via PrintScreen
```
handle_hid_event() → mesh/spindle keyboard
  → pdx_call(SLOT_LINEN, OP_LINEN_OPEN_INTENT)   @ silk-shell:15001
  → linen_sync_reply()                            @ silk-shell:15005 ← BLOCKS
  → open_linen_object_in_quil()                   @ silk-shell:15007 (SAFE, local only)
```

## Blocking Helpers Table

| Helper | File:Line | Wait Type | Bounded? | HID Drain? | Fire-and-Forget Safe? |
|--------|-----------|-----------|----------|------------|----------------------|
| `linen_sync_reply()` | silk-shell:1116 | `loop { pdx_listen_raw }` | **NO** | Yes (OP_HID_EVENT) | No |
| `pdx_storage_sync()` | linen:655 | `loop { pdx_listen_raw }` | **NO** | Yes (OP_HID_EVENT) | No |
| `linen_fetch_remote_snapshot()` | silk-shell:1155 | calls sync_reply ×16-64 | **NO** | Indirect | No |
| `linen_paint_surface()` | silk-shell:1231 | calls fetch on first paint | Once-only per boot | Indirect | No (first call) |
| `open_linen_in_active_scene()` | silk-shell:7229 | calls paint_surface | Once-only per boot | Indirect | No (first call) |
| `open_linen_object_in_quil()` | silk-shell:1578 | **LOCAL ONLY** | **YES** | N/A | **YES** |
| `handle_open_intent()` (linen) | linen:620 | `pdx_reply` immediate | **YES** | N/A | **YES** |
| `pdx_call(SLOT_STORAGE,*)` (spindle) | spindle:821 | AsyncEnqueue edge | **YES** | N/A | **YES** |

## Proposed Nonblocking Design

### Core Insight
The blocking occurs because silk-shell waits synchronously for Linen's reply
to `OP_LINEN_GET_PUBLIC_SNAPSHOT` and `OP_LINEN_GET_PUBLIC_NAME`.
Linen's server-side handlers are non-blocking (`handle_open_intent` replies immediately).
The problem is on the **caller side**: silk-shell's `linen_sync_reply()` spin-waits instead
of returning to the event loop.

### Design: Staged Intent + Event-Loop Poll

#### Stage 0: Request Enqueue
```rust
// Fire OP_LINEN_GET_PUBLIC_SNAPSHOT for each slot as fire-and-forget.
// Store a "pending snapshot" counter.
// Return immediately to event loop.
fn linen_request_snapshot_async(slot: u64) {
    pdx_call(SLOT_LINEN, OP_LINEN_GET_PUBLIC_SNAPSHOT, slot, 0, 0);
    PENDING_LINEN_SNAPSHOTS += 1;
}
```

#### Stage 1: Event Loop Poll
```rust
// In main message loop, when a type_id==0x1 reply arrives:
// Check if it's from SLOT_LINEN and matches a pending request.
// Reconcile the reply into LINEN_OBJECTS table.
// Decrement PENDING_LINEN_SNAPSHOTS.
// When PENDING_LINEN_SNAPSHOTS == 0, mark LINEN_REMOTE_FETCHED = true.
fn linen_reconcile_reply(msg: PdxMessage) {
    if msg.caller_pd == LINEN_PD && msg.type_id == 0x1 {
        // Parse packed reply into LINEN_OBJECTS slot.
        // If name chunks pending, fire next chunk request.
        PENDING_LINEN_SNAPSHOTS -= 1;
    }
}
```

#### Stage 2: Timeout/Fail Marker
```rust
// If LINEN_REMOTE_FETCHED is still false after N main loop iterations,
// emit a timeout marker and render static UI anyway.
const LINEN_FETCH_TIMEOUT: u32 = 10_000; // main loop iterations
if !LINEN_REMOTE_FETCHED && loop_count > LINEN_FETCH_TIMEOUT {
    serial_println!("[linen.remote.fetch.timeout] reason=server_no_reply");
    LINEN_REMOTE_FETCHED = true; // fall through to static UI
}
```

#### Stage 3: No Hot-Path Blocking
- `open_linen_in_active_scene()` renders static UI or existing cached data immediately.
- `linen_paint_surface()` checks `LINEN_REMOTE_FETCHED` — if false, renders static UI
  and spawns async fetch (which completes in background).
- No `linen_sync_reply()` call in any keyboard/mouse/Palette dispatch path.
- Mesh Enter, F8, Palette Open Linen all return to event loop within 1ms.

### STOP FIRST Boundaries

#### Do NOT change:
1. **ABI / opcodes**: OP_LINEN_GET_PUBLIC_SNAPSHOT (0x44), OP_LINEN_GET_PUBLIC_NAME (0x45),
   OP_LINEN_OPEN_INTENT (0x46) remain unchanged. Linen server-side handlers are already
   non-blocking.
2. **PDX helpers**: No changes to `pdx_call`, `pdx_listen_raw`, `pdx_reply` — all remain
   as-is.
3. **Kernel routing**: No changes needed — PDX message delivery already works correctly.
4. **Linen server**: `handle_open_intent`, `handle_get_public_snapshot`,
   `handle_get_public_name` all reply immediately — no changes needed server-side.
5. **SexFiles / RamFS**: Not involved in this path. Linen's `pdx_storage_sync()` is
   server-side only and not in silk-shell's call chain for open/focus.
6. **sexusb / sexinput / sexdisplay**: Not involved.

### Recommended Smallest Future Patch

#### File: `servers/silk-shell/src/main.rs`

**Phase 1 — Remove blocking from paint (safest, smallest):**
1. Split `linen_paint_surface()` (line 1231) into two paths:
   - `linen_paint_surface_fast()`: renders static UI if LINEN_REMOTE_FETCHED is false.
     No PDX calls. Used in all dispatch paths (keyboard, palette, mesh).
   - `linen_paint_surface_full()`: calls `linen_fetch_remote_snapshot()` only if needed.
     Only called from event loop (line 14041), not from dispatch.

2. Remove `linen_paint_surface()` call from `open_linen_in_active_scene()` (line 7296).
   Replace with `linen_paint_surface_fast()`.

3. Remove `linen_paint_surface()` call from `mesh_focus_linen_at_selected_fact()` (line 2539).

4. `linen_sync_reply()` (line 1116) remains in codebase for `linen_fetch_remote_snapshot()`
   which only runs in event-loop context (not hot-path dispatch).

**Phase 2 — True async snapshot (larger, post-Phase 1):**
5. Replace `linen_fetch_remote_snapshot()` with staged async version per design above.
6. Add `PENDING_LINEN_SNAPSHOTS` counter and `linen_reconcile_reply()` in main loop.
7. Add timeout/fail marker.
8. Remove `linen_sync_reply()` entirely after verification.

#### Estimated patch size:
- Phase 1: ~15 lines changed (safe, minimal)
- Phase 2: ~60 lines added, ~30 lines removed

## Future Proof Markers

### Required for Phase 1 verification:
```
[linen.paint.fast] reason=static_ui_fallback  ← emitted when fast path used
[linen.paint.dispatch] path=palette|mesh|keyboard  ← which dispatch triggered it
[linen.open.nonblocking] ok=1  ← confirms no linen_sync_reply in dispatch path
```

### Required for Phase 2 verification:
```
[linen.async.snapshot.begin] slots=N  ← async fetch started
[linen.async.snapshot.reconcile] slot=N object_id=X  ← each reply reconciled
[linen.async.snapshot.done] slots=N ok=1  ← all slots fetched
[linen.async.snapshot.timeout] reason=...  ← timeout fallback
[linen.async.proof] stage=N ok=1 reason=...  ← proof stage markers
[linen.async.proof.done] ok=1  ← final proof gate
```

## Acceptance Criteria

1. `open_linen_in_active_scene()` returns in <1ms (no `pdx_listen_raw` blocking).
2. Palette Open Linen action returns `ok=1` instead of `blocking_risk_confirmed`.
3. Mesh detail Enter → Linen focus returns immediately, Linen surface appears with
   static UI, async fetch completes in background.
4. F8 ToggleLinen no longer blocks the keyboard event loop.
5. No new faults. No regression in keyboard nav, mouse input, or surface management.
6. `[linen.open.nonblocking]` marker appears in serial log after first successful
   nonblocking open.
7. Runtime proof gate `SEXOS_LINEN_NONBLOCKING_OPEN_PROOF=1` passes all stages.

## Faults

0 — diagnostic analysis only, no code changes made.

## Files Changed

`docs/handoff/LINEN_SAFE_NONBLOCKING_OPEN_PLAN_V1.md` — created (this document).
No source files modified.
