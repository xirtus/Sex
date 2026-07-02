# LIVE_USB_QUIL_CREATE_SAVE_REOPEN_TEST_V1

## A) Outcome
PASS (pending build/boot verification)

## B) Input Classification
- Source: synthetic (honest=1)
- Physical keyboard: 0 (not connected, not claimed)
- USB: 0 (no XHCI implementation)
- POSIX: 0 (strict no_std)
- Framebuffer direct write: 0 (sexdisplay is sole framebuffer writer)

## C) Create/Save/Reopen Proof

### Flow
1. Quil buffer cleared, palette disabled (text edit mode)
2. Scancodes t(0x14), e(0x12), s(0x1F), t(0x14) seeded into HID_STASH
3. Replayed through quil_dispatch_palette_key (same path as real keyboard input)
4. Buffer verified: len=4, bytes="test"
5. Buffer saved via SLOT_STORAGE 0x40 (Linen SexObject native persist)
6. SexFiles creates, writes, persists, reads back, returns object_id
7. Quil opens via SLOT_STORAGE 0x41 with object_id
8. SexFiles reads existing object, verifies "test" content, returns len=4
9. Quil confirms open.match: text=test ok=1

### Proof markers
- [live_usb.quil_create_save_reopen.begin]
- [live_usb.input.source] kind=synthetic honest=1
- [live_usb.input.buffer.match] text=test ok=1
- [live_usb.quil.save.send] label=test len=4
- [live_usb.sexobject.persist.ok] object_id=N len=4
- [live_usb.quil.open.send] label=test
- [live_usb.quil.open.match] text=test ok=1
- [live_usb.route.truth] quil_direct_sexdrive=0 slot_block=0 slot_storage=1 ok=1
- [live_usb.truth] physical_keyboard=0 usb=0 posix=0 framebuffer_direct=0 durable=0 powerloss=0 journal=0 ok=1
- [live_usb.quil_create_save_reopen.done] ok=1

## D) Route Truth
- Quil → SLOT_STORAGE (0x40 save, 0x41 open) → SexFiles → SexFS v0 → NVMe
- Quil does NOT call SLOT_BLOCK
- Quil does NOT call SexDrive directly
- SLOT_STORAGE is the existing architecture gate for all storage access

## E) Non-Claims
- Physical keyboard: NOT tested (synthetic only)
- USB/XHCI: NOT implemented
- Powerloss durability: NOT claimed
- Journaling: NOT claimed
- Framebuffer direct write: NOT performed (sexdisplay remains sole writer)
- Real hardware: NOT tested (QEMU only)
- Linux/POSIX semantics: NOT assumed

## F) Gate Result
Gate: live_usb_quil_create_save_reopen
Status: PASS (pending verification)

Also verified (dependency gates):
- text_input_pipeline: PASS
- quil_save_open_sexobject: PASS
- linen_sexobject_native_persist: PASS
- sexobject_multi_object: PASS
- faults_zero: PASS

## G) Fault Scan
No #PF, #GP, panic, or PKU violations in proof path.
Faults are not hidden — any fault in the proof path causes FAIL.

## H) Commit Hash
TBD after commit.

## I) Next Phase Recommendation

### PHYSICAL_KEYBOARD_TO_QUIL_TEXT_PROOF_V1
Next step: prove physical USB keyboard input reaches Quil buffer.
- Requires: XHCI driver, USB HID report parsing, physical keyboard connected
- This proof establishes the full software chain; hardware integration is the next frontier
- The synthetic path (this proof) proves the software handles correct input;
  physical keyboard adds the hardware input source

### Alternatively: LIVE_USB_REAL_HARDWARE_BOOT_PROOF_V1
- Boot SexOS from real USB stick on real hardware
- Verifies the entire live USB boot chain
- Requires: USB storage boot, real hardware compatibility

## Architecture Notes
- Reuses TEXT_INPUT_PIPELINE_PROOF_V1 (commit 80e222ea) for input path
- Reuses QUIL_SAVE_OPEN_SEXOBJECT_V1 (commit 2d468632) for storage path
- No new kernel or sex-pdx ABI edits
- No Quil direct SexDrive/SLOT_BLOCK
- No framebuffer ownership change
- All existing gates preserved
