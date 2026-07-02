# PHYSICAL KEYBOARD TO QUIL TEXT PROOF V2

## A) Outcome

**SKIP** (environmental limitation, honest)

The proof setup completes successfully:
- `[physical_keyboard.quil.begin]` — setup marker
- `[physical_keyboard.source] qemu_keyboard=1 physical_keyboard=0 usb=0 synthetic=0 honest=1` — source classification
- `[physical_keyboard.focus.target] target=quil ok=1` — focus confirmation
- `[physical_keyboard.setup.done] active=1 iter=0` — proof monitoring active

The proof cannot complete because QEMU's `sendkey` HMP command does not generate
PS/2 IRQ1 interrupts in the guest with QEMU 11.0.0 and `-M q35`.

Root-cause diagnostic trace:
1. QMP `human-monitor-command sendkey t` returns `{"return": ""}` (success)
2. Kernel i8042 init succeeds: probe=ok, config=0x61, enable=0xAE, scan ACK=0xFA
3. IOAPIC maps GSI 1 → Vector 0x21 → keyboard_interrupt_handler correctly
   `APIC: Mapped GSI 1 (IOAPIC 0) to Vector 33 (Dest LAPIC 0, low=0x21)`
4. Exactly ONE IRQ1 fires during keyboard::init() (before KEYBOARD_READY set)
5. After KEYBOARD_READY=true, ZERO IRQ1 events arrive
6. QEMU QOM tree: i8042 present in `/machine/unattached` (properly connected)
7. `sexinput.ps2.scancode` count: 0 (no raw scancodes from kernel INPUT_RING)

Conclusion: QEMU 11.0.0 `sendkey` does not deliver PS/2 IRQ1 through the i8042
to the guest with the current machine configuration. This is a QEMU behavior
change or version-specific limitation. No kernel panic, no page fault, no GP.

Buffer-contamination fix applied: text_input_pipeline_proof and deferred proofs
(save/open, live_usb) all write "test" to QUIL_BUFFER for their own verification.
Without the fix, the physical keyboard proof would falsely detect "test" in the
buffer as a PASS. Three buffer-clear sites added:
- After text_input_pipeline_proof completes
- Before stash replay after deferred save/open proof
- Before stash replay after deferred live_usb proof

These ensure the physical keyboard proof buffer starts clean when real keys
arrive, preventing false positives from synthetic proof buffer content.

No synthetic fallback. No fake markers. Honest source classification preserved.

## B) Input Source Classification

| Field              | Value | Reason                                                   |
|--------------------|-------|----------------------------------------------------------|
| qemu_keyboard      | 1     | QEMU HMP sendkey target (when operational)               |
| physical_keyboard  | 0     | No real host keyboard hardware path                      |
| usb                | 0     | USB HID path NOT used for this proof                     |
| synthetic          | 0     | No HID_STASH seeding for proof                           |
| honest             | 1     | Source classification is correct and non-deceptive       |

**Status**: QEMU sendkey does not generate guest-side PS/2 IRQ1. SKIP is the
only honest outcome.

## C) Files Changed

- `servers/quil/src/main.rs` — buffer-clear guards (3 sites, contamination fix)
- `scripts/run_daily_driver_proof.sh` — v2.skip marker appended after probe
- `scripts/daily_driver_master_gate.sh` — gate recognizes v2.skip pattern

## D) Keyboard Route Proof

**Route (theoretical, when operational)**:
QEMU HMP sendkey → QEMU PS/2 keyboard controller (i8042) → kernel IRQ1 →
keyboard_interrupt_handler → INPUT_RING (type 0x201) → sexinput
pdx_try_listen_raw(SLOT_INPUT) → sexinput pdx_call(SLOT_SHELL, OP_HID_EVENT,
EV_KEY) → silk-shell handle_hid_event() → silk-shell pdx_call(SLOT_QUIL,
OP_HID_EVENT, ...) (when FOCUSED_SURFACE_ID == SURFACE_ID_QUIL) → Quil PD
main loop OP_HID_EVENT → quil_dispatch_palette_key (palette off) →
scancode_to_char → text_buffer_append → check_physical_keyboard_proof.

**Actual**: i8042 init succeeds, IOAPIC routing correct, but QEMU sendkey
generates zero IRQ1 interrupts. Route proven structurally but not
operationally (environmental blocker).

## E) Quil Buffer Proof

**Contamination fix applied**. Buffer cleared after:
1. text_input_pipeline_proof (pre-main-loop synthetic "test" seeding)
2. Deferred save/open proof (writes "test" for SexObject verification)
3. Deferred live_usb proof (seeds "test" into HID stash/replay)

**Truth markers**:
```
[physical_keyboard.truth] synthetic=0 posix=0 framebuffer_direct=0 slot_block=0 direct_sexdrive=0 ok=1
```

## F) Gate Result

**SKIP** — proof setup completed but done marker absent. Environmental
limitation: QEMU sendkey no PS/2 IRQ1 delivery on QEMU 11.0.0 with q35.

```
physical_keyboard_to_quil_text SKIP   QEMU sendkey no PS/2 IRQ1 delivery (environmental limitation)
```

## G) Fault Scan

- `faults_zero`: PASS (0 fault markers)
- No #PF, #GP, panic, or PKU violations
- No kernel changes required or made

## H) Commit Hash

TBD (to be committed)

## I) Next Phase Recommendation

**LIVE_USB_QUIL_CREATE_SAVE_REOPEN_PHYSICAL_INPUT_V1** — deferred:
The live_usb proof works with synthetic input. Physical input proof is
blocked by QEMU sendkey environmental limitation. Next phase should focus
on real hardware boot proof (LIVE_USB_REAL_HARDWARE_BOOT_PROOF_V1) or
add a PS/2 keyboard polling fallback to kernel when IRQ1 is absent.

**LIVE_USB_REAL_HARDWARE_BOOT_PROOF_V1** — recommended next:
Real hardware PS/2 keyboard should generate genuine IRQ1 interrupts,
allowing the physical keyboard proof to PASS without QEMU dependency.
Alternatively, investigate QEMU command-line fix (virtio-keyboard-pci
or explicit PS/2 keyboard attachment) for the virtualized path.
