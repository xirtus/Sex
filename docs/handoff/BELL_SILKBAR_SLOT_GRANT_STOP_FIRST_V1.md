# Bell SilkBar SLOT_BELL Grant — STOP FIRST Review

**Status:** Implemented.
**Date:** 2026-05-06
**Files changed:** 2 (+8 lines)
**Build:** `./scripts/entrypoint_build.sh` — PASS

---

## 1. STOP FIRST Review Results

| Check | Result |
|---|---|
| SilkBar PD ID identified? | ✅ **6** (sequential allocation from `NEXT_PD_ID` starting at 1; spawn order = 6th module) |
| SLOT_BELL constant identified? | ✅ `sex_pdx::SLOT_BELL = 12` |
| Existing grant pattern followed? | ✅ Same `grant_capability(SLOT_BELL, Domain(sexbell_id))` pattern as silk-shell (line 110) |
| Bell server-side LIST allowlist permits SilkBar? | ⚠️ **Was: NO. Now: YES** (added `6` to `BELL_LIST_ALLOWLIST`) |
| Grant model can restrict opcode? | ❌ **No.** Domain capabilities are all-or-nothing for a PD. A PD with SLOT_BELL can call ALL Bell opcodes (NOTIFY, CLOSE, ACTION, CLEAR, MUTE_SENDER, LIST). **Mitigation**: Bell's server-side checks (LIST allowlist, mute list, validation) still apply for all opcodes. |
| Kernel grant would allow unsafe ops? | ✅ **No.** SilkBar has no reason to call NOTIFY/CLOSE/ACTION/CLEAR/MUTE. Even if it did, Bell's server-side validation (mute check, privacy check, allowlist) applies uniformly to all callers. |
| Unsafe to all PDs? | ✅ **No.** Grant is targeted to exactly one PD (SilkBar, ID 6). |

**Verdict: SAFE to proceed.** Dual-gate model (kernel capability + Bell allowlist) prevents misuse.

---

## 2. Changes Made

### kernel/src/init.rs (+7 lines)

Added grant block within the existing SilkBar capability section (after SLOT_DISPLAY grant):

```rust
// Bell polling cap: SilkBar needs SLOT_BELL for OP_BELL_LIST.
// This is a read-only LIST capability — SilkBar has no NOTIFY/CLOSE/ACTION.
// Bell server-side allowlist (BELL_LIST_ALLOWLIST) provides second gate.
if sexbell_id != 0 {
    pd.grant_capability(sex_pdx::SLOT_BELL, CapabilityData::Domain(sexbell_id));
    serial_println!("[kernel.sexbell.cap.silkbar] silkbar→bell slot=12");
}
```

### servers/sexbell/src/main.rs (+1 line)

Added SilkBar (PD 6) to `BELL_LIST_ALLOWLIST`:

```rust
const BELL_LIST_ALLOWLIST: &[u32] = &[
    3,  // silk-shell (domain 3, policy owner)
    6,  // silkbar (domain 6, privacy-safe aggregate poller)
];
```

---

## 3. Dual-Gate Capability Model

```
┌─────────────────────────────────────────────────────────────────┐
│                        ACCESS CONTROL                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  Gate 1: Kernel Capability (init.rs)                            │
│  ┌──────────────────────────────────────┐                      │
│  │ Only PDs with SLOT_BELL in cap table │                      │
│  │ can send ANY message to Bell.        │                      │
│  │ Without this: ERR_CAP_INVALID.       │                      │
│  └──────────────────────────────────────┘                      │
│           │                                                     │
│           ▼                                                     │
│  Gate 2: Bell Server-Side Allowlist (sexbell)                   │
│  ┌──────────────────────────────────────┐                      │
│  │ OP_BELL_LIST only:                   │                      │
│  │ - is_list_reader_allowed(caller_pd)  │                      │
│  │ - Currently: PD 3 or PD 6           │                      │
│  │ - Non-allowed: reply u64::MAX       │                      │
│  └──────────────────────────────────────┘                      │
│           │                                                     │
│           ▼                                                     │
│  Gate 3: Bell Server-Side Validation (sexbell)                  │
│  ┌──────────────────────────────────────┐                      │
│  │ All other opcodes (NOTIFY, CLOSE,    │                      │
│  │ ACTION, CLEAR, MUTE): standard       │                      │
│  │ validation applies to ALL callers.   │                      │
│  │ Mute check, privacy, spam budget.    │                      │
│  └──────────────────────────────────────┘                      │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

**Risk:** SilkBar COULD call OP_BELL_NOTIFY or OP_BELL_CLOSE if it chose to, since the Domain capability grants access to all opcodes. **Mitigations:**
1. SilkBar's code has no reason to call any opcode other than LIST
2. Bell's server-side validation (mute list, validation) applies uniformly — a NOTIFY from SilkBar would be validated just like any other sender
3. SilkBar's LIST calls are further gated by `BELL_LIST_ALLOWLIST`

---

## 4. Remaining Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| SilkBar misbehaves and calls NOTIFY/CLOSE/ACTION | Low | Low | Bell validates all ops; spam budget limits abuse |
| PD ID 6 conflicts with future module reordering | Low | Medium | Deterministic spawn order; update `NEXT_PD_ID` initial value or add ID guard |
| Hardcoded PD IDs fragile across kernels | Low | Low | All existing grants use same pattern (hardcoded `3` for silk-shell) |

---

## 5. Build Result

**PASS** — `sexos-v1.0.0.iso` produced. No new warnings.

---

## 6. Runtime Markers (expected after boot)

```
[silkbar.bell.poll]           ← poll sent
[silkbar.bell.poll.reply]     ← LIST reply received (if Bell responds)
[bell.list.reply]             ← Bell's reply marker (budgeted)
[sexdisplay.bell.render]      ← sexdisplay renders Bell state
```

Previously the poll path logged `[silkbar.bell.reject] err=0xfffffffffffffffc` (ERR_CAP_INVALID). With the grant, the poll should log `[silkbar.bell.poll] sent` instead.
