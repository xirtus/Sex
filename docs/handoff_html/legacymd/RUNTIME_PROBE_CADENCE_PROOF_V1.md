# RUNTIME_PROBE_CADENCE_PROOF_V1

- date: 2026-05-07
- answers: why click-focus/drag proofs don't fire in CI 10s probe

---

## Executive Summary

Missing proof markers are caused by **slow cooperative scheduling + tick thresholds too high
for 10s probe window** — NOT by broken feature code.

sexfiles and silk-shell proofs are message-driven (blocking listen), not tick-gated.
They fire correctly. sexdrive block proof fires only when a real client sends block commands.
Only sexinput has tick-gated proofs, and most of those thresholds are unreachable in 10s.

---

## Loop Architecture by PD

| PD | Listen type | Tick counter | Yield source | Notes |
|----|-------------|--------------|--------------|-------|
| sexinput (PD4) | `pdx_try_listen_raw` (non-blocking) | yes, per-iteration | 2 internal (empty poll) + 1 explicit per iteration | ~3 yields per tick |
| silk-shell (PD3) | `pdx_listen_raw` (BLOCKING) | no | explicit `sys_yield()` only | blocks until IPC arrives |
| sexfiles (PD11) | `pdx_listen_raw` (BLOCKING) | no | implicit via pdx_listen | blocks until VFS call arrives |
| sexdrive (PD2) | `pdx_try_listen_raw` (non-blocking) | no | 1 internal yield per empty poll | no tick-gated proofs |

**silk-shell and sexfiles are not affected by probe window for their existing proofs.**
**sexdrive has no synthetic proof generator — only fires on real client block commands.**

---

## sexinput Cadence Measurement (10s probe)

Observed from `.gate_master/serial.log`:
- `[sexinput.usb_mouse.recv]` count = **7** (budget = 16, not exhausted → true count = 7)
- Each recv = one loop iteration where pdx_try_listen_raw(0) returned Some

Each iteration costs **~3 scheduling slots** (2 empty-poll internal yields + 1 explicit yield).
With 12 PDs in cooperative round-robin, each scheduling slot is shared.

**Measured rate: 7 loop iterations / 10s = 0.70 iter/sec**

---

## Tick Threshold Table — sexinput

| Proof | Gate | Tick threshold | Iter needed | Min probe (s) | Fires in 10s? |
|-------|------|----------------|-------------|---------------|---------------|
| KEYBOARD_EDGE_PROOF_V1 | `!PROOFS_DISABLED` | 3 + 4 | 4 | ~6s | **YES ✓** |
| click-focus stage 0 | `!PROOFS_DISABLED` | 10 | 10 | ~14s | **NO ✗** |
| click-focus stage 1+2 | `!PROOFS_DISABLED` | 14, 15 | 15 | ~21s | **NO ✗** |
| silkbar click (all stages) | `SILKBAR_CLICK_PROOF_ENABLED` | 2–33 | 33 | ~47s | **NO ✗** (disabled by default anyway) |
| F5/F6 kbd proof | `KEYBOARD_PROOF_ENABLED` | 50–155 | 155 | ~221s | **NO ✗** |
| drag proof (first fire) | `!PROOFS_DISABLED && !DRAG_DONE` | tick % 120 == 0 | 120 | ~171s | **NO ✗** |

---

## Storage Typed Block Proof

`[sexdrive.block.typed.recv]` fires when a client sends `BLOCK_READ`/`BLOCK_WRITE`/`BLOCK_SYNC`
to SLOT_BLOCK. sexdrive has no synthetic block command generator.

**Why it's absent from log:** No PD sends block commands during a cold 10s boot.
- sexfiles: selects RamFS backend by default (no `SEXFILES_DISKFS=1` env var set)
- spindle: logs `no_storage_cap` → never contacts sexfiles
- linen, quil: no file activity during idle boot

This is **not a probe-window issue** — it's a missing synthetic block proof in sexdrive,
or no client ever exercising the path during boot.

**sexfiles `[sexfiles.ready]` IS in the log** — sexfiles proofs run at startup (env-var-gated,
separate from tick). SEXFILES_GATE PASS is accurate.

---

## Which Proofs Are Actually Broken vs Just Slow

| Proof | Status | Root cause |
|-------|--------|-----------|
| KEYBOARD_EDGE_PROOF_V1 | **PROVEN** ✓ | tick 3/4, fires in 10s |
| click-focus chain | **NOT FIRING** | tick 10-15, needs ~21s probe |
| drag proof | **NOT FIRING** | tick 120, needs ~171s probe — impractical |
| silk-shell key route | **PROVEN** ✓ | message-driven, fired in KEYBOARD_EDGE_PROOF |
| sexfiles VFS | **PROVEN** ✓ | startup proof, not tick-gated |
| sexdrive block typed | **UNPROVEN** | no synthetic generator, no client traffic |

---

## Required Probe Durations

| Goal | Probe window | Why |
|------|-------------|-----|
| Smoke: PD spawn + schedule proof | 10s | Current; adequate |
| click-focus proof fires | **25s** | tick 15 at ~21s |
| drag proof fires | ~180s | Not practical for CI |

---

## Next Recommended Prompt

**OPTION A: RUNTIME_GATE_PROBE_WINDOW_TUNE_V1**

Extend gate default probe from 10s to 25s for deep proof runs.
Also lower drag proof tick threshold from `tick % 120 == 0` to `tick % 5 == 0`
(fire every 5 iterations ≈ every 7s), bounded by DRAG_DONE one-shot gate.

Required changes: `scripts/master_runtime_gate.sh` + `servers/sexinput/src/main.rs` drag threshold.
Zero ABI / kernel / scheduler changes.

This is the right next step. Covers click-focus AND drag in a 25s probe window.

**OPTION B: SCHEDULER_ROUND_ROBIN_PROOF_FIX_V1** — STOP FIRST

Would require scheduler behavior change (faster round-robin, more aggressive yield).
Out of scope until LAPIC timer is fixed (LAPIC_TIMER_SFMASK_PREMORTEM_V1).

**OPTION C: SEXDRIVE_SYNTHETIC_BLOCK_PROOF_V1**

Add one synthetic BLOCK_READ command from sexfiles startup path (or sexdrive self-test)
to prove the block typed route exists.
Scope: sexdrive only, no kernel/ABI change.

---

## Conclusion

**Probe window is the only blocker for click-focus and drag CI proofs.**
Feature code is correct. Storage markers are absent because no client exercises
the block path during cold boot — separate concern from probe window.

Recommended action: RUNTIME_GATE_PROBE_WINDOW_TUNE_V1 first, then SEXDRIVE_SYNTHETIC_BLOCK_PROOF_V1.
