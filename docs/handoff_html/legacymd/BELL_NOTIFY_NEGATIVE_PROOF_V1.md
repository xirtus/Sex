# BELL_NOTIFY_NEGATIVE_PROOF_V1

**Status:** Proof complete. Invalid category=7 correctly rejected.
**Build:** `[SEXOS ENTRYPOINT] success`
**Log:** `/home/xirtus_arch/Documents/microkernel/qemu_debug.log`

---

## Files Changed

| File | Change | Lines |
|------|--------|-------|
| `kernel/src/init.rs` | Temporary one-shot invalid OP_BELL_NOTIFY enqueue (proof scaffolding) | +18 |

**Not touched:**
- `servers/sexbell/src/main.rs` — unchanged, no rewrite needed
- `crates/sex-pdx/src/lib.rs` — unchanged
- `limine.cfg` — unchanged
- `sexos_build_spec.toml` — unchanged

---

## Exact Invalid Payload

```rust
MessageType::IpcCall {
    func_id:   sex_pdx::OP_BELL_NOTIFY,   // 0xC0
    arg0:      (7 << 0) | (2 << 8) | (0 << 16) | (0 << 24),
    arg1:      0,
    arg2:      0,
    caller_pd: 0,
}
```

### arg0 bit layout

| Bits | Field | Value | Validity |
|------|-------|-------|----------|
| 0-7 | `category` | **7** | **INVALID** (0..=5 valid) |
| 8-15 | `urgency_hint` | 2 | Valid (URGENT) |
| 16-23 | `privacy_level` | 0 | Valid (Public) |
| 24-31 | `redaction_class` | 0 | Valid (StructuralMeta) |
| 32-63 | `_reserved` | 0 | — |

---

## Boot Log — Bell Section

```
715:[kernel.sexbell.cap] self slot=12
716:[kernel.sexbell.notify.invalid.test] category=7
933:[bell.boot]
934:[bell.notify.reject] caller_pd=0 reason=invalid_category
```

---

## Marker Table

| Line | Marker | Present? | Expected? |
|------|--------|----------|-----------|
| 715 | `[kernel.sexbell.cap]` self slot=12 | ✅ | Yes |
| 716 | `[kernel.sexbell.notify.invalid.test] category=7` | ✅ | Yes |
| 933 | `[bell.boot]` | ✅ | Yes |
| 934 | `[bell.notify.reject]` caller_pd=0 reason=invalid_category | ✅ | **Yes — proof target** |
| — | `[bell.notify.recv]` | ❌ Absent | ✅ Correct — validation fails before recv marker |
| — | `[bell.notify.downgrade]` | ❌ Absent | ✅ Correct — no lane derivation on invalid input |
| — | `[bell.notify.ok]` | ❌ Absent | ✅ Correct — event was rejected |
| — | `[bell.unknown.reject]` | ❌ Absent | ✅ Correct — OP_BELL_NOTIFY was matched |

### Regarding `[bell.notify.recv]` absence

The current handler emits recv marker **after** validation (line 90 in sexbell/main.rs). Since category=7 fails the first check (`valid_category`), the `continue` at line 86 skips the recv marker. This is correct behavior. No handler rewrite needed.

---

## Reject Reason

```
reason=invalid_category
```

The expected reject string from `valid_category(7)` returning false (7 > 5).

---

## Faults / Panics

| Check | Result |
|-------|--------|
| `fault.kill` | 0 |
| `panic` | 0 |
| `#PF` / `#GP` | 0 |

**Zero faults or panics.**

---

## Regression Check — All 10 PDs

All 10 protection domains spawned successfully (same as previous proof). No regression.

---

## Temporary Scaffold Warning

The 18-line scaffolding block in `kernel/src/init.rs` is temporary.
**Must be removed** in `BELL_NOTIFY_NEGATIVE_CLEANUP_V1`.

Marker: `[kernel.sexbell.notify.invalid.test]` — this must NOT remain in the kernel after cleanup.

---

## Next Phase

**BELL_NOTIFY_NEGATIVE_CLEANUP_V1** — Remove scaffolding, verify handler intact, then **Bell Phase 1 freeze**.

---

*End of BELL_NOTIFY_NEGATIVE_PROOF_V1.md*
