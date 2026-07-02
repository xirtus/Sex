# BELL_READER_CAP_IMPLEMENT_V1

**Status:** Implemented. Code changed. Build passes.
**Date:** 2026-05-05
**Depends on:** `BELL_READER_CAP_PLAN_V1.md`, `BELL_LIST_SUMMARY_FREEZE_V1.md`

---

## 1. Files Changed

| File | Change | Type |
|------|--------|------|
| `servers/sexbell/src/main.rs` | Add BELL_LIST_ALLOWLIST, allowlist check, readcap markers | Server edit |
| `kernel/src/init.rs` | Add SLOT_BELL routing cap to silk-shell + proof scaffolds | Kernel edit |
| `docs/handoff/BELL_READER_CAP_IMPLEMENT_V1.md` | This document | Handoff |

**Not changed:** sex-pdx, silk-shell UI, SilkBar, sexdisplay, storage, limine.cfg, sexos_build_spec.toml

---

## 2. Allowlist Implementation

### Constant

```rust
/// Static allowlist of PDs permitted to call OP_BELL_LIST.
/// Default-deny: any PD not in this list is rejected.
/// V1: only silk-shell (domain 3). Extended in future phases.
const BELL_LIST_ALLOWLIST: &[u32] = &[
    3,  // silk-shell (domain 3, policy owner)
];
```

### Helper

```rust
fn is_list_reader_allowed(caller_pd: u32) -> bool {
    BELL_LIST_ALLOWLIST.contains(&caller_pd)
}
```

### Check placement

Checked AFTER arg validation (protocol errors reported regardless of caller) but BEFORE `[bell.list.recv]` and queue access:

```
OP_BELL_LIST
  ├── Parse + validate lane_filter, max_results
  │     └── Invalid → [bell.list.reject] → continue
  ├── Check caller_pd against BELL_LIST_ALLOWLIST
  │     ├── Not found → [bell.readcap.deny] → continue
  │     └── Found    → [bell.readcap.allow] → continue to recv
  ├── [bell.list.recv]
  ├── Read queue, emit items
  └── [bell.list.done] or [bell.list.empty]
```

---

## 3. Kernel Cap Grant

### Added to silk-shell cap block (init.rs, lines 107-112)

```rust
// Bell read-cap: silk-shell can call OP_BELL_LIST
if sexbell_id != 0 {
    pd.grant_capability(sex_pdx::SLOT_BELL, CapabilityData::Domain(sexbell_id));
    serial_println!("[kernel.sexbell.cap.shell] shell→bell slot=12");
}
```

### Silk-shell's full cap set after grant

| Slot | Target | Purpose |
|------|--------|---------|
| SLOT_DISPLAY (5) | sexdisplay | Framebuffer rendering |
| SLOT_SHELL (6) | self | Shell identity |
| SLOT_SILKBAR (7) | silkbar | Workspace IPC |
| SLOT_SEXSTORE (10) | sexstore | K/V storage |
| SLOT_QUIL (11) | quil | App surface routing |
| **SLOT_BELL (12)** | **sexbell** | **Bell read-cap (NEW)** |

### Sexbell self-cap preserved

```rust
// Line 174 (unchanged)
pd.grant_capability(sex_pdx::SLOT_BELL, CapabilityData::Domain(sexbell_id));
```

---

## 4. Proof Scaffolds (Temporary)

### Three sequential messages enqueued to sexbell's message ring:

| # | Type | caller_pd | Purpose | Marker |
|---|------|-----------|---------|--------|
| 1 | OP_BELL_NOTIFY | 0 (kernel) | Seed queue | `[kernel.sexbell.cap.seed]` |
| 2 | OP_BELL_LIST | 3 (silk-shell) | Positive proof | `[kernel.sexbell.cap.positive]` |
| 3 | OP_BELL_LIST | 2 (sexdrive) | Negative proof | `[kernel.sexbell.cap.negative]` |

### Honest caller_pd

`caller_pd` is kernel-authoritative — the kernel sets it for ALL messages regardless of source (syscall or direct enqueue). Setting `caller_pd=3` and `caller_pd=2` in the IpcCall struct is the same mechanism used when a real userspace PD calls `pdx_call` (the kernel fills in `caller_pd` from the current PD's ID). No payload fields are used to encode authorization state.

### Expected positive proof markers

```
[bell.boot]
[bell.notify.recv] caller_pd=0 category=0 requested=2
[bell.notify.downgrade] from=2 to=0 reason=no_caps_untrusted
[bell.queue.push] id=1 final_lane=0 count=1
[bell.notify.ok] event_id=1

[bell.readcap.allow] caller_pd=3 op=list
[bell.list.recv] lane_filter=0xff max_results=4 caller_pd=3
[bell.list.item] event_id=1 final_lane=0 category=0 privacy=0 redaction=0
[bell.list.done] count=1
```

### Expected negative proof markers

```
[bell.readcap.deny] caller_pd=2 op=list reason=no_read_cap
```

**Absent:** `[bell.list.recv]`, `[bell.list.item]`, `[bell.list.done]`, `[bell.list.empty]`, `[bell.unknown.reject]`

---

## 5. New Markers

| Marker | Budget | Fields | When |
|--------|--------|--------|------|
| `[bell.readcap.allow]` | 8 | `caller_pd`, `op` | Approved caller passes allowlist |
| `[bell.readcap.deny]` | 8 | `caller_pd`, `op`, `reason` | Unapproved caller rejected |

### Existing markers preserved (unchanged)

All 25+ prior Bell markers remain intact.

---

## 6. Build Result

```
[SEXOS ENTRYPOINT] success
```

---

## 7. Forbidden Features — Confirmed Absent

| Feature | Check | Result |
|---------|-------|--------|
| Reply ABI (`pdx_reply`) | `rg` on sexbell/main.rs | ❌ Absent |
| Heap/alloc/Vec/String/Box | `rg` on sexbell/main.rs | ❌ Absent |
| SilkBar integration | `rg` on sexbell/main.rs | ❌ Absent |
| Storage/persistence | `rg` on sexbell/main.rs | ❌ Absent |
| Private content (title/body) | `rg` on sexbell/main.rs | ❌ Absent |
| Queue mutation | `rg "OP_BELL_CLEAR"` on sexbell | ❌ Absent |
| sex-pdx edits | No changes | ❌ Unchanged |
| Multi-reader grant | Only silk-shell in allowlist | ✅ Only one |
| caller_pd spoofing via payload | `caller_pd` is kernel-set field, not arg | ✅ Honest |
| Notify authority for silk-shell | SLOT_BELL gives routing only; no OP_BELL_NOTIFY check change | ✅ Not granted |

---

## 8. Next Phase

**BELL_READER_CAP_PROOF_V1** — QEMU boot proof via `./qemuX.sh` showing both positive (`[bell.readcap.allow]` + normal list flow) and negative (`[bell.readcap.deny]` without list flow) markers.

---

*End of BELL_READER_CAP_IMPLEMENT_V1.md*
