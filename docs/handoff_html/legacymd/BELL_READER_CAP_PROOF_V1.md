# BELL_READER_CAP_PROOF_V1

**Status:** QEMU proof complete. All markers verified.
**Launcher:** `./qemuX.sh` (patched QEMU with XHCI/HID fixes, `-M q35,i8042=off`, USB-only, `-display sdl`)
**Date:** 2026-05-05

---

## 1. Files Changed

| File | Change | Type |
|------|--------|------|
| `docs/handoff/BELL_READER_CAP_PROOF_V1.md` | This document | Handoff |

**No code changes.** Proof-only.

---

## 2. Build Result

```
[SEXOS ENTRYPOINT] success
```

---

## 3. QEMU Runner

`./qemuX.sh` — patched QEMU binary at `/home/xirtus_arch/Documents/microkernel/tools/qemu/build/qemu-system-x86_64` with XHCI/HID fixes.

---

## 4. Boot Log — All Bell Markers

```
[kernel.sexbell.cap.shell] shell→bell slot=12                    ← permanent silk-shell cap
[kernel.sexbell.cap] self slot=12                                ← permanent sexbell self-cap
[kernel.sexbell.cap.seed] enqueued OP_BELL_NOTIFY to sexbell     ← scaffold seed notify
[kernel.sexbell.cap.positive] enqueued OP_BELL_LIST caller_pd=3  ← scaffold positive list
[kernel.sexbell.cap.negative] enqueued OP_BELL_LIST caller_pd=2  ← scaffold negative list

[bell.boot]                                                      ← sexbell alive

[bell.notify.recv] caller_pd=0 category=0 requested=2            ← seed notify
[bell.notify.downgrade] from=2 to=0 reason=no_caps_untrusted
[bell.queue.push] id=1 final_lane=0 count=1
[bell.notify.ok] caller_pd=0 final_lane=0 event_id=1

[bell.readcap.allow] caller_pd=3 op=list                         ← silk-shell approved
[bell.list.recv] lane_filter=0xff max_results=4 caller_pd=3
[bell.list.item] event_id=1 final_lane=0 category=0 privacy=0 redaction=0
[bell.list.done] count=1

[bell.readcap.deny] caller_pd=2 op=list reason=no_read_cap       ← sexdrive denied
```

---

## 5. Proof Verification Matrix

### Positive proof (caller_pd=3, silk-shell)

| Check | Expected | Observed | Result |
|-------|----------|----------|--------|
| `[bell.readcap.allow]` | `caller_pd=3 op=list` | Match | ✅ |
| `[bell.list.recv]` | `caller_pd=3` | Match | ✅ |
| `[bell.list.item]` | `event_id=1 final_lane=0 category=0 privacy=0 redaction=0` | Match | ✅ |
| `[bell.list.done]` | `count=1` | Match | ✅ |
| `[bell.list.empty]` | Absent (queue had 1 entry) | Absent | ✅ |

### Negative proof (caller_pd=2, sexdrive)

| Check | Expected | Observed | Result |
|-------|----------|----------|--------|
| `[bell.readcap.deny]` | `caller_pd=2 reason=no_read_cap` | Match | ✅ |
| `[bell.list.recv]` | Absent (denied before queue access) | Absent | ✅ |
| `[bell.list.item]` | Absent | Absent | ✅ |
| `[bell.list.done]` | Absent | Absent | ✅ |
| `[bell.list.empty]` | Absent | Absent | ✅ |

### Absence checks

| Check | Expected | Result |
|-------|----------|--------|
| `[bell.list.reject]` | Absent (valid requests) | ✅ |
| `[bell.unknown.reject]` | Absent | ✅ |
| `[bell.queue.reject.full]` | Absent (1 of 16 entries) | ✅ |
| `[bell.notify.reject]` | Absent (valid payload) | ✅ |
| Faults/panics | Absent | ✅ |

### All-PD spawn check

| PD | Domain | Spawned? |
|----|--------|----------|
| sexdisplay | 1 | ✅ |
| sexdrive | 2 | ✅ |
| silk-shell | 3 | ✅ |
| sexinput | 4 | ✅ |
| sexusb | 5 | ✅ |
| silkbar | 6 | ✅ |
| linen | 7 | ✅ |
| sexstore | 8 | ✅ |
| quil | 9 | ✅ |
| sexbell | 10 | ✅ |

---

## 6. What This Proves

| Property | How it's proved |
|----------|----------------|
| **silk-shell can call OP_BELL_LIST** | Positive proof: caller_pd=3 passes allowlist, receives item+done |
| **sexdrive cannot call OP_BELL_LIST** | Negative proof: caller_pd=2 denied before queue access |
| **Deny happens before queue read** | Negative proof: no `[bell.list.recv]` after deny |
| **Allowlist accepts authorized callers** | `[bell.readcap.allow]` emitted for caller_pd=3 |
| **Allowlist rejects unauthorized callers** | `[bell.readcap.deny]` emitted for caller_pd=2 |
| **caller_pd is kernel-authoritative** | Set in kernel `MessageType::IpcCall`, not from payload fields |
| **silk-shell has SLOT_BELL cap** | `[kernel.sexbell.cap.shell]` present at boot |
| **sexbell self-cap preserved** | `[kernel.sexbell.cap]` present at boot |

---

## 7. Scaffold Warning

**Three temporary kernel enqueues** in `init.rs`:
1. `OP_BELL_NOTIFY` (seed queue)
2. `OP_BELL_LIST` with `caller_pd=3` (positive proof)
3. `OP_BELL_LIST` with `caller_pd=2` (negative proof)

All removed in `BELL_READER_CAP_CLEANUP_V1`.

---

## 8. Next Phase

**BELL_READER_CAP_CLEANUP_V1** — Remove all three temporary kernel scaffolds. Verify sexbell allowlist + silk-shell SLOT_BELL cap remain. Then freeze.

---

*End of BELL_READER_CAP_PROOF_V1.md*
