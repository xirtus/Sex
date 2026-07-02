# SCENE_SETTINGS_INPUT_PROOF_PLAN_V1

## Status

Design (2026-05-04). Proof strategy for F5/F6 Scene settings input through
real existing HID path. Docs-only — no code changed.

---

## Verdict: SCENE_SETTINGS_INPUT_PROOF_SAFE_SYNTHETIC ⚠️ Requires gated sexinput change

| Requirement | Feasible? | Notes |
|-------------|-----------|-------|
| Prove F5 → `[shell.appearance.preset]` | ✅ Via synthetic HID event in sexinput |
| Prove F5 → `[shell.scene.settings.save]` | ✅ Same path — triggers persist after cycle |
| Prove F5 → `[sexstore.kv.put]` | ✅ Same path — sexstore's GET handler confirmed working |
| Prove F6 → `[shell.appearance.custom]` | ✅ Via synthetic HID event |
| Prove F6 does NOT cause save | ✅ Check no save marker after F6 event |
| No faults | ✅ Bounded one-shot, no replay |
| Real keyboard proof possible | ❌ No — see Input Path Audit below |
| No code changes | ❌ Must add gated synthetic keyboard proof |

---

## Input Path Audit

### Why Real Keyboard Proof Is Not Possible

| Path | Blocked? | Root Cause |
|------|----------|------------|
| **PS/2 keyboard → QMP `send-key`** | ❌ | Kernel registers `keyboard_interrupt_handler` at `idt[0x21]` but NEVER calls `map_irq(1, 0x21, ...)` to program the I/O APIC redirection entry. IRQ 1 stays masked at the IOAPIC. `register_irq_route` for vector 0x21 is never called. Verified: zero calls to `map_irq` or `register_irq_route` in entire kernel. |
| **USB HID keyboard → sexusb** | ❌ | `servers/sexusb/src/main.rs` has NO keyboard HID report handling. Only mouse reports (OP_USB_MOUSE_REPORT = 0x260) are forwarded. No code references `OP_USB_KEYBOARD_REPORT` (0x261) or any keyboard HID usage page. |
| **Synthetic proofs in sexinput** | ❌ | `SYNTHETIC_INPUT_PROOFS_DISABLED` hardcoded to `true` at line 34. All synthetic proofs (drag, silkbar click, click focus) are dead code. |

### How Keyboard Events SHOULD Arrive at Silk-Shell

```
Source          → Intermediary         → sexinput handler     → silk-shell
─────────────────────────────────────────────────────────────────────────────
PS/2 keyboard   IOAPIC IRQ1→INPUT_RING  SLOT_INPUT poll       OP_HID_EVENT EV_KEY
USB HID kbd     sexusb (not impl)       OP_USB_KEYBOARD_REPORT (not impl)
Synthetic       sexinput code           pdx_call direct        OP_HID_EVENT EV_KEY
```

The only path that can be enabled synthetically is the **third row**: sexinput
directly calls `pdx_call(SLOT_SHELL, OP_HID_EVENT, scancode, 1/0, EV_KEY)`.

---

## Recommended Proof Route

### Option B: Gated Synthetic Keyboard Proof in sexinput

Add a one-shot synthetic keyboard proof to `servers/sexinput/src/main.rs`,
following the exact pattern of existing synthetic proofs (drag proof, silkbar
click proof, click focus proof), gated behind a new compile-time constant.

**Rationale:**
- Reuses the same `OP_HID_EVENT` → `EV_KEY` path as both real keyboard sources
- No kernel changes needed
- No USB HID keyboard stack implementation needed
- Bounded, one-shot, no replay risk
- Gate prevents interference with interactive use
- Proven pattern: 3 existing synthetic proofs in sexinput use this exact structure

### Why Not The Other Options

| Option | Rejected Because |
|--------|------------------|
| A: Interactive SDL QEMU | Requires human pressing keys at correct moment; no captured serial log with proof markers; not automatable |
| C: Fix IO APIC IRQ1 routing | Kernel change — STOP FIRST, forbidden this phase |
| D: Document gap | Leaves F5/F6 path entirely unproven; code has zero test coverage |

---

## Synthetic Keyboard Proof Design

### Gate

```rust
/// Enables one-shot synthetic keyboard proof for F5/F6 HID event path.
/// Set env var `SEXOS_KEYBOARD_PROOF=1` at build time to enable.
/// Default (unset): no behavior change.
/// Only affects sexinput; no kernel changes.
const KEYBOARD_PROOF_ENABLED: bool = option_env!("SEXOS_KEYBOARD_PROOF").is_some();
```

Following the `KEYBOARD_CURSOR_ENABLED` pattern (line 39 of sexinput).

### Proof Sequence

Added after the existing synthetic click focus proof (after line 551 in sexinput),
or interleaved at a tick that avoids collision with existing proofs.

```rust
// 6. One-shot synthetic keyboard proof for F5/F6 scene settings.
//    Sends F5 press+release, then F6 press+release, via OP_HID_EVENT EV_KEY.
//    Bounded: KBD_PROOF_DONE prevents replay after stage 2.
if KEYBOARD_PROOF_ENABLED {
    match kbd_proof_stage {
        // F5 press: scancode 0x3F = 63, EV_KEY, value=1 (pressed)
        0 if tick == 50 => {
            pdx_call(SLOT_SHELL, OP_HID_EVENT, 63, 1, EV_KEY);
            serial_println!("[sexinput.kbd_proof.f5.down]");
            kbd_proof_stage = 1;
        }
        // F5 release
        1 if tick == 55 => {
            pdx_call(SLOT_SHELL, OP_HID_EVENT, 63, 0, EV_KEY);
            serial_println!("[sexinput.kbd_proof.f5.up]");
            kbd_proof_stage = 2;
        }
        // Second F5 (proves cycle wraps + persist fires again)
        2 if tick == 100 => {
            pdx_call(SLOT_SHELL, OP_HID_EVENT, 63, 1, EV_KEY);
            kbd_proof_stage = 3;
        }
        3 if tick == 105 => {
            pdx_call(SLOT_SHELL, OP_HID_EVENT, 63, 0, EV_KEY);
            kbd_proof_stage = 4;
        }
        // F6: scancode 0x40 = 64, EV_KEY, value=1 (pressed)
        4 if tick == 150 => {
            pdx_call(SLOT_SHELL, OP_HID_EVENT, 64, 1, EV_KEY);
            serial_println!("[sexinput.kbd_proof.f6.down]");
            kbd_proof_stage = 5;
        }
        5 if tick == 155 => {
            pdx_call(SLOT_SHELL, OP_HID_EVENT, 64, 0, EV_KEY);
            kbd_proof_stage = 6;
        }
        _ => {}
    }
}
```

### Tick Placement

Existing synthetic proof timeline (approximate):

| Tick | Proof |
|------|-------|
| 0 | Drag proof start (disabled via SYNTHETIC_INPUT_PROOFS_DISABLED) |
| 2–33 | SilkBar click proof (disabled via SYNTHETIC_INPUT_PROOFS_DISABLED) |
| 10–15 | Click focus proof (disabled via SYNTHETIC_INPUT_PROOFS_DISABLED) |
| 40 | Linen surface render |
| 50–105 | **F5 keyboard proof** (proposed, ~tick 120 cycle) |
| 150–155 | **F6 keyboard proof** (proposed) |

Since all existing synthetic proofs are disabled by default, the keyboard proof
at ticks 50–155 will not collide with anything in normal operation. The tick
counter starts at 0 and increments by 1 each loop iteration (at `tick = tick.wrapping_add(1)`).

### Required sexinput Changes

| File | Change |
|------|--------|
| `servers/sexinput/src/main.rs` | Add `KEYBOARD_PROOF_ENABLED` const; add `kbd_proof_stage: u8` local variable; add synthetic keyboard proof block with F5/F6 sequence; add markers |

### NOT modified

- `kernel/` — no kernel changes
- `servers/sexusb/` — no change
- `servers/silk-shell/` — no change
- `servers/sexstore/` — no change
- `crates/sex-pdx/` — no ABI hash change

---

## Expected Markers

### With keyboard proof enabled (`SEXOS_KEYBOARD_PROOF=1`)

| Marker | When |
|--------|------|
| `[sexinput.kbd_proof.f5.down]` | F5 press at tick 50 |
| `[sexinput.kbd_proof.f5.up]` | F5 release at tick 55 |
| `[shell.appearance.preset] idx=1` | F5 cycle → preset 0→1 |
| `[shell.scene.settings.save] preset=1` | F5 persist → save |
| `[sexstore.kv.put] key=1 ok=1` | sexstore receives PUT |
| `[sexinput.kbd_proof.f5.down]` | Second F5 at tick 100 |
| `[shell.appearance.preset] idx=2` | F5 cycle → preset 1→2 |
| `[shell.scene.settings.save] preset=2` | F5 persist → save |
| `[sexstore.kv.put] key=1 ok=1` | sexstore receives PUT |
| `[sexinput.kbd_proof.f6.down]` | F6 press at tick 150 |
| `[shell.appearance.custom] mode=tint tint=1` | F6 tint cycle |
| (NO `[shell.scene.settings.save]`) | F6 does NOT persist ✅ |
| `[shell.scene.settings.load.request]` | Boot GET (1 occurrence only) |
| `[shell.scene.settings.load] ok=0 not-found` | Boot GET miss (1 occurrence only) |
| panic / #PF / #GP | **0** ✅ |

### Without keyboard proof (default, no env var)

| Marker | Count |
|--------|-------|
| `[shell.scene.settings.load.request]` | 1 (boot GET, proven) |
| `[sexstore.kv.get]` | 1 (boot GET, proven) |
| All keyboard proof markers | 0 (gate closed) |

---

## Build / Proof Commands

```bash
# Build with keyboard proof enabled
SEXOS_KEYBOARD_PROOF=1 ./scripts/entrypoint_build.sh

# Run headless capture
timeout 60 qemu-system-x86_64 \
  -M q35 \
  -m 512M \
  -cpu max,+pku \
  -cdrom sexos-v1.0.0.iso \
  -device nec-usb-xhci,id=xhci \
  -device usb-kbd,bus=xhci.0 \
  -display none \
  -serial file:/tmp/sexos-keyboard-proof.log \
  -qmp unix:/tmp/sexos-qmp.sock,server=on,wait=off &
# ... wait 30 seconds, kill ...

# Verify markers
grep -n "sexinput.kbd_proof" /tmp/sexos-keyboard-proof.log
grep -n "appearance.preset" /tmp/sexos-keyboard-proof.log
grep -n "settings.save" /tmp/sexos-keyboard-proof.log
grep -c "sexstore.kv.put" /tmp/sexos-keyboard-proof.log
grep -n "appearance.custom" /tmp/sexos-keyboard-proof.log

# Confirm NO save after F6
grep -c "settings.save" /tmp/sexos-keyboard-proof.log  # should be 2 (two F5 saves)
grep -c "appearance.custom" /tmp/sexos-keyboard-proof.log  # should be 1 (one F6)

# Verify no faults
grep -cE "panic|#PF|#GP|PAGE FAULT|GENERAL PROTECTION" /tmp/sexos-keyboard-proof.log
```

---

## STOP Conditions

| Condition | Action |
|-----------|--------|
| `map_irq` or `register_irq_route` changes needed in kernel | **STOP** — kernel change, defer to APIC fix phase |
| sexusb keyboard HID report handling needed | **STOP** — USB HID keyboard stack is a large change |
| New sex-pdx opcodes or ABI changes needed | **STOP** — not needed for this proof |
| Tick collision with existing synthetic proofs after re-enabling | Adjust tick offsets; existing proofs are currently disabled |
| SLOT_SHELL `pdx_call` fails during synthetic proof | Check silk-shell liveness; proof runs at tick 50+, silk-shell should be listening |

---

## Next Phase: SCENE_SETTINGS_INPUT_PROOF_V1

1. Add `KEYBOARD_PROOF_ENABLED` const to `servers/sexinput/src/main.rs`
2. Add `kbd_proof_stage` local variable
3. Add synthetic F5/F6 keyboard proof block (ticks 50–155)
4. Add `[sexinput.kbd_proof.*]` markers
5. Build: `SEXOS_KEYBOARD_PROOF=1 ./scripts/entrypoint_build.sh`
6. Run QEMU headless, capture serial log
7. Verify all expected markers present
8. Create `docs/handoff/SCENE_SETTINGS_INPUT_PROOF_V1.md`

---

## References

| Doc | Relevance |
|-----|-----------|
| `docs/handoff/SCENE_SETTINGS_BOOT_PROOF_V1.md` | Previous proof attempt; established GET path works |
| `docs/handoff/SCENE_SETTINGS_PERSIST_V1.md` | F5 persist implementation being tested |
| `servers/sexinput/src/main.rs` | Target for synthetic keyboard proof; existing synthetic proof patterns |
| `servers/silk-shell/src/main.rs` | `scancode_to_action(0x3F)` and `(0x40)` dispatch |
| `servers/sexstore/src/main.rs` | OP_KV_GET=0xB0, OP_KV_PUT=0xB1 handlers |
| `kernel/src/interrupts.rs` | `keyboard_interrupt_handler`, `INPUT_RING`, `VECTOR_OWNERS` |
| `kernel/src/apic.rs` | `map_irq(irq, vector, ...)` — never called for keyboard |
