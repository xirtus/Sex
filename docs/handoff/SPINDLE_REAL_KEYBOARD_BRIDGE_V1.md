# SPINDLE_REAL_KEYBOARD_BRIDGE_V1

**Date:** 2026-05-06
**Status:** STOP FIRST — requires sex-pdx ABI edit (1 line)
**Previous:** SPINDLE_BOOT_MODULE_WIRING_V1

---

## STOP FIRST: Exact Smallest ABI Request

### Prerequisite (1 line in `crates/sex-pdx/src/lib.rs`)

```rust
pub const SLOT_SPINDLE: u64 = 14;  // Spindle command console (domain 12)
```

This is the ONLY sex-pdx change needed. After approval, 3 more files get ~20 lines total.

---

## Full Wiring Plan (After STOP FIRST Approval)

### 1. sex-pdx (1 line)

```rust
// crates/sex-pdx/src/lib.rs, after SLOT_LINEN = 13
pub const SLOT_SPINDLE: u64 = 14;  // Spindle command console (domain 12)
```

### 2. silk-shell (~10 lines)

```rust
// servers/silk-shell/src/main.rs

// Surface ID (with existing surface IDs)
pub const SURFACE_ID_SPINDLE: u64 = 400;

// HID forwarding (in OP_HID_EVENT handler, after Linen block)
} else if FOCUSED_SURFACE_ID == SURFACE_ID_SPINDLE {
    pdx_call(SLOT_SPINDLE, OP_HID_EVENT, scancode as u64, value, EV_KEY);
    mutated = true;
}

// Register surface in boot init (with other lifecycle registrations)
lifecycle_register(SURFACE_ID_SPINDLE, LifecycleState::Visible);
```

### 3. Spindle (~10 lines)

```rust
// apps/spindle/src/main.rs — idle loop
loop {
    let msg = unsafe { pdx_listen_raw(0) };
    if msg.type_id == 0x202 { // OP_HID_EVENT
        let scancode = msg.arg0 as u8;
        let value = msg.arg1;
        if value == 1 { // key press
            handle_key(scancode, &mut line, &mut sb, &mut hist, &mut ev, &mut fb);
        }
    }
}
```

---

## Input Route

```
sexinput (PD 4) ──HID event──→ silk-shell (PD 3)
                                  │
                    FOCUSED_SURFACE_ID == SURFACE_ID_SPINDLE?
                                  │ YES
                                  ▼
                    pdx_call(SLOT_SPINDLE, OP_HID_EVENT, ...)
                                  │
                                  ▼
                    Spindle (PD 12) ── pdx_listen_raw(0)
                                  │
                                  ▼
                    handle_key() → CmdLine.push/backspace
                                 → dispatch()
                                 → render_scrollback()
                                 → redraw_prompt()
```

### Focus Ownership

| Component | Responsibility |
|-----------|---------------|
| sexinput | Raw HID event normalization |
| silk-shell | Focus policy, surface lifecycle, key routing |
| Spindle | Line editor, scrollback, command dispatch |
| sexdisplay | Final pixel rendering |

---

## Expected Markers

| Marker | Source |
|--------|--------|
| `[silk-shell.keyboard.forward.spindle]` | silk-shell (new) |
| `[spindle.input.recv]` | Spindle (new) |
| `[spindle.line.append]` | Spindle (existing in proof) |
| `[spindle.line.backspace]` | Spindle (existing in proof) |
| `[spindle.line.enter]` | Spindle (existing in proof) |

---

## Negative Proof: Focus Isolation

When Spindle is NOT focused:
- `FOCUSED_SURFACE_ID != SURFACE_ID_SPINDLE`
- Silk-shell routes keys to other focused app (or handles as shell shortcuts)
- Spindle's `pdx_listen_raw(0)` receives no HID events
- No change to global keyboard policy

---

## Files After Approval

| File | Change |
|------|--------|
| `crates/sex-pdx/src/lib.rs` | +1 line (SLOT_SPINDLE = 14) |
| `servers/silk-shell/src/main.rs` | +10 lines (surface ID, HID forward, lifecycle) |
| `apps/spindle/src/main.rs` | +10 lines (HID event loop, handle_key) |
| `docs/handoff/SPINDLE_REAL_KEYBOARD_BRIDGE_V1.md` | This doc |

---

## Pre-Approval Evidence

| Check | Status |
|-------|--------|
| Spindle PD | PD 12, spawned, [spindle.ready] |
| Silk-shell HID routing | Exists for Quil (SLOT_QUIL=11), Linen (SLOT_LINEN=13) |
| Pattern established | Identical to Quil/Linen forwarding |
| Smallest change | 1 line in sex-pdx, ~20 lines total |
| Risk | None — additive, follows existing pattern |

---

## STOP FIRST Request

**Add 1 line to `crates/sex-pdx/src/lib.rs`:**
```rust
pub const SLOT_SPINDLE: u64 = 14;
```

This unblocks the final 1% of Spindle V1: real keyboard input through the existing silk-shell focus/input routing path.
