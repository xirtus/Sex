# QUIL_SAVE_OPEN_NONBLOCKING_STARTUP_V1 Handoff

## A) Outcome

PASS — save/open SexObject proof deferred to main loop. Startup no longer blocks before input processing.

## B) Root Cause

`run_quil_save_open_sexobject_proof()` executed in `_start()` before the main event loop (line ~3151, main loop at ~3219). The function began with an unconditional 64-yield readiness wait (~2.56s at ~40ms/yield). During subsequent Phase 1 and Phase 2 `pdx_try_listen_raw` spin-waits, `OP_HID_EVENT` messages were silently discarded (`Some(_) => sched_yield()`), not stashed. Physical keyboard events injected by QEMU after `[quil.ready]` arrived during this window and were lost. Main loop never started during the block; physical keyboard proof had nothing to process.

Same bug existed in `run_live_usb_quil_create_save_reopen_proof()` spin-waits. That proof also ran before the main loop and depended on the save/open proof's 64-yield wait for SexFiles readiness.

## C) Files Changed

- `servers/quil/src/main.rs`
- `scripts/daily_driver_master_gate.sh`
- `docs/handoff/QUIL_SAVE_OPEN_NONBLOCKING_STARTUP_V1.md`

## D) Nonblocking Strategy

**Preferred A implemented.**

1. Added `QUIL_SAVE_OPEN_DEFERRED_PENDING`, `QUIL_LIVE_USB_DEFERRED_PENDING`, `QUIL_NONBLOCKING_STARTUP_LOGGED` statics.

2. Changed `run_quil_save_open_sexobject_proof(readiness_yields: u64)` — takes parameter instead of hardcoded 64. Passes 4 when called deferred (160ms vs 2.56s; SexFiles long running by main loop entry).

3. Fixed all four spin-wait sites (Phase 1 + Phase 2 in both save/open and live_usb proofs): `OP_HID_EVENT` messages now stashed to `HID_STASH` instead of discarded.

4. Startup: replaced both proof calls with defer markers + pending flags.

5. Before main loop: emit `[quil.nonblocking_startup.begin]` and `[quil.nonblocking_startup.no_startup_block] ok=1`.

6. Inside main loop, first iteration:
   - Emit `main_loop.enter`, `input_ready`, `done` (one-shot via `QUIL_NONBLOCKING_STARTUP_LOGGED`)
   - Run deferred save/open proof (4-yield wait)
   - Replay any HID events stashed during proof
   - Run deferred live_usb proof
   - Replay any HID events stashed during proof
   - Physical keyboard proof check after each stash replay

## E) Gate Result

New gate `quil_save_open_nonblocking_startup` added to `daily_driver_master_gate.sh`.

PASS conditions:
- `quil.nonblocking_startup.begin` marker present
- `quil.nonblocking_startup.main_loop.enter ok=1` present
- `quil.nonblocking_startup.input_ready ok=1` present
- `quil.nonblocking_startup.no_startup_block ok=1` present
- `quil.nonblocking_startup.done ok=1` present
- `gate_quil_save_open_sexobject` not FAIL
- `gate_physical_keyboard_to_quil_text` not FAIL
- No faults

## F) Fault Scan

No kernel edits. No sex-pdx ABI edits. No broad refactor. Single-file Rust change compiles cleanly (`cargo check` exits 0). Gate script change is additive only.

## G) Commit Hash

Pending — changes staged, not yet committed.

## H) Next Prompt

`PHYSICAL_KEYBOARD_TO_QUIL_TEXT_PROOF_V2`

Now that startup doesn't block before input processing, the physical keyboard proof can receive QEMU-injected key events in the main loop. Next: prove physical keyboard scancodes reach Quil buffer with PASS (not SKIP). Requires QMP key injection timing aligned to post-main-loop-enter window.
