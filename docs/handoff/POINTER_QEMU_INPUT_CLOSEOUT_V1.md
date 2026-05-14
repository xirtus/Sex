# POINTER_QEMU_INPUT_CLOSEOUT_V1

Date: 2026-05-14

## Final closeout
This mission closes pointer tuning loops and records the stable dev baseline.

## 1) Stable baseline
Use usb-mouse REL with V2 transfer cap18 baseline.
- REL decode path is functional.
- Routing and click path are functional.
- Fault isolation remained clean.

## 2) Recommended QEMU dev lanes
- Manual GUI work: usb-mouse (REL lane)
- ABS regression checks: usb-tablet lane

## 3) Known limitation (host/QEMU quality)
QEMU/GTK input on this laptop can emit saturated/repeated REL bursts (for example ±127), causing steppy/less-comfortable pointer feel even after conservative transfer shaping.

This is treated as a host-input/dev-lane quality limitation, not a SexOS routing failure.

## 4) Stop condition for tuning
Do not continue REL cap/easing tuning loops unless testing:
- real hardware input lane, or
- a better host input backend/capture path.

## 5) Preferred proof strategy for next GUI missions
Prioritize deterministic proofs over manual pixel-perfect targeting:
- keyboard-driven actions
- synthetic proof modes
- exact coordinate proofs

## 6) Proof summary
- REL decode: PASS
- `source=rel`: PASS
- button/click path: PASS
- faults: 0
- smoothness ceiling: constrained by host/QEMU capture quality

## 7) Notes for future agents
When pointer feel regresses in QEMU GTK:
1. Re-verify REL decode/route markers first.
2. Avoid immediate gain/cap churn.
3. Confirm whether symptoms match saturated host bursts before code changes.
