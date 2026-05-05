# BELL_NOTIFY_PROOF_V1

**Status:** Proof complete. All markers present. No faults.
**Build:** `[SEXOS ENTRYPOINT] success`
**Log:** `/home/xirtus_arch/Documents/microkernel/qemu_debug.log`

---

## Commands Run

```bash
./scripts/entrypoint_build.sh
./run_qemu.sh                    # boots QEMU, output → qemu_debug.log
rg -n "kernel.sexbell.notify.test|bell\.boot|bell\.notify\." qemu_debug.log
rg -c "fault\.kill|panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" qemu_debug.log
```

---

## Boot Log — Bell Section (Lines 698–924)

```
698:✓ Spawned PD 10: /servers/sexbell (Domain 10)
699:[kernel.spawn.sexbell] id=10 path=/servers/sexbell
715:[kernel.sexbell.cap] self slot=12
716:[kernel.sexbell.notify.test] enqueued OP_BELL_NOTIFY to sexbell
921:[bell.boot]
922:[bell.notify.recv] caller_pd=0 category=0 requested=2
923:[bell.notify.downgrade] from=2 to=0 reason=no_caps_untrusted
924:[bell.notify.ok] caller_pd=0 final_lane=0
```

---

## Marker Table

| Line | Marker | Present? | Expected? |
|------|--------|----------|-----------|
| 698 | PD 10 spawn (sexbell, domain 10) | ✅ | Yes |
| 699 | `[kernel.spawn.sexbell]` | ✅ | Yes |
| 715 | `[kernel.sexbell.cap]` self slot=12 | ✅ | Yes |
| 716 | `[kernel.sexbell.notify.test]` | ✅ | Yes |
| 921 | `[bell.boot]` | ✅ | Yes |
| 922 | `[bell.notify.recv]` caller_pd=0 category=0 requested=2 | ✅ | Yes |
| 923 | `[bell.notify.downgrade]` from=2 to=0 reason=no_caps_untrusted | ✅ | Yes |
| 924 | `[bell.notify.ok]` caller_pd=0 final_lane=0 | ✅ | Yes |
| — | `[bell.notify.reject]` | ❌ Absent | ✅ Correct — no reject for valid payload |
| — | `[bell.unknown.reject]` for OP_BELL_NOTIFY | ❌ Absent | ✅ Correct — type_id was matched |

---

## Exact Payload Observed

| Field | Value | Derivation |
|-------|-------|------------|
| `caller_pd` | 0 | Kernel-originated |
| `category` | 0 | Info |
| `urgency_hint` (requested) | 2 | URGENT |
| `final_lane` | 0 | PASSIVE |

---

## Downgrade Result

```
requested=2 (URGENT) → downgrade reason=no_caps_untrusted → final_lane=0 (PASSIVE)
```

This confirms:
- Enum parsing works (category=0 accepted as Info)
- Urgency hint extracted correctly (2)
- No-cap policy applied (unknown/untrusted → max PASSIVE)
- URGENT downgraded to PASSIVE via `no_caps_untrusted`
- `[bell.notify.downgrade]` marker emitted with correct `from=2 to=0`

---

## Faults / Panics

| Check | Result |
|-------|--------|
| `fault.kill` | 0 |
| `panic` | 0 |
| `#PF` / `PAGE FAULT` | 0 |
| `#GP` / `GENERAL PROTECTION` | 0 |
| `bell.unknown.reject` for OP_BELL_NOTIFY | 0 (correctly matched) |

**No real faults or panics.** Two false positives (`keyboard_cursor.gate`, `shell.store.default`) match common substrings only — confirmed not crash-related.

---

## Regression Check — All 10 PDs

| Domain | Server | Spawn Marker |
|--------|--------|-------------|
| 1 | sexdisplay | `✓ Spawned PD 1` |
| 2 | sexdrive | `✓ Spawned PD 2` |
| 3 | silk-shell | `✓ Spawned PD 3` |
| 4 | sexinput | `✓ Spawned PD 4` |
| 5 | sexusb | `✓ Spawned PD 5` |
| 6 | silkbar | `✓ Spawned PD 6` |
| 7 | linen | `✓ Spawned PD 7` |
| 8 | sexstore | `✓ Spawned PD 8` |
| 9 | quil | `✓ Spawned PD 9` |
| 10 | sexbell | `✓ Spawned PD 10` |

All 10 protection domains spawned successfully. No regression in existing PD boot order.

---

## Private Content Confirmation

| Check | Result |
|-------|--------|
| Title/body/sender name in markers | ❌ Absent |
| File paths in markers | ❌ Absent |
| Raw arg dumps | ❌ Absent |
| Action payloads | ❌ Absent (action_count=0 enforced) |
| Object references | ❌ Absent (object_refs=0 enforced) |
| All marker fields StructuralMeta | ✅ Confirmed |

---

## Next Phase

**BELL_NOTIFY_CLEANUP_V1** — Remove kernel scaffolding (18-line enqueue block in init.rs). sexbell dispatch persists as real protocol handler.

---

*End of BELL_NOTIFY_PROOF_V1.md*
