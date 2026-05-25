# TEXT_INPUT_PIPELINE_PROOF_V1

## Outcome

PASS — typed text "test" reaches Quil buffer through the input pipeline.

## Input Source Classification

**source=synthetic honest=1**

The proof seeds synthetic scancode events (scancode set 1: `t=0x14, e=0x12, s=0x1F, t=0x14`) into the Quil HID_STASH, then replays them through `quil_dispatch_palette_key` with palette OFF (text edit mode). This exercises the **exact same code path** as real keyboard input:

```
HID_STASH seed → quil_dispatch_palette_key (palette=false)
  → scancode_to_char → text_buffer_append → draw_text_lines
```

- No USB HID involved (physical_keyboard=0, usb=0)
- No framebuffer direct write from Quil (framebuffer_direct=0)
- No Linux/POSIX semantics (posix=0)
- This is honest about being synthetic — it proves the QUIL HALF of the pipeline
  (input event → buffer append → render intent). The remaining half
  (physical keyboard → USB HID → sexinput → silk-shell focus route → Quil)
  is the LIVE_USB proof target for the next phase.

## Files Changed

| File | Change |
|------|--------|
| `servers/quil/src/main.rs` | Added `QUIL_TEXT_INPUT_PIPELINE_PROOF_ENABLED` constant, `run_text_input_pipeline_proof()` function, and call site in `_start()` |
| `scripts/run_daily_driver_proof.sh` | Added `export SEXOS_QUIL_TEXT_INPUT_PIPELINE_PROOF=1` |
| `scripts/daily_driver_master_gate.sh` | Added `text_input_pipeline` gate state, detection logic, and reporting array entry |
| `docs/handoff/TEXT_INPUT_PIPELINE_PROOF_V1.md` | This handoff document |

## Buffer Proof

After replaying the stashed scancode events through the dispatch system (palette OFF → text edit mode → scancode_to_char → text_buffer_append), the Quil buffer (`QUIL_BUFFER`) is verified to contain exactly `"test"` (4 bytes) at byte offsets 0-3.

The cursor position (`QUIL_CURSOR_POS`) is verified to be `4` (end of buffer after appending 4 characters).

## Render/Visibility

**Honest limitation**: Quil does not have font rendering (only fill-rect visual representation via `draw_text_lines`). The `draw_text_lines` call was triggered as part of the normal keyboard input path, and fill-rect glyphs were sent to sexdisplay. The `[quil.input.render.intent]` marker documents this, but no pixel-level framebuffer assertion is claimed.

## Gate Result

Gate: `text_input_pipeline`

Required markers:
- `[text_input.pipeline.begin]`
- `[text_input.source] kind=synthetic honest=1`
- `[text_input.focus.target] target=quil ok=1`
- `[text_input.key.recv] ch=t`
- `[text_input.key.recv] ch=e`
- `[text_input.key.recv] ch=s`
- `[text_input.key.recv] ch=t` (4 keys: t, e, s, t)
- `[text_input.char.decode] text=test ok=1`
- `[quil.input.buffer.append] text=test len=4 ok=1`
- `[quil.input.cursor.ok] pos=4`
- `[quil.input.render.intent] text=test ok=1`
- `[text_input.pipeline.truth] physical_keyboard=0 usb=0 posix=0 framebuffer_direct=0 ok=1`
- `[text_input.pipeline.done] ok=1`

## Fault Scan

No #PF, #GP, panic, or PKU violation markers expected in the text input pipeline path.
Faults are caught by the existing `faults_zero` gate.

## Key Gates Preserved

All previous key gates must remain PASS/SKIP:
- `quil_save_open_sexobject`: PASS
- `linen_sexobject_native_persist`: PASS
- `sexobject_multi_object`: PASS
- `linen_diskfs_direct`: SKIP (superseded by SexObject)
- `faults_zero`: PASS

## Commit Hash

To be filled after commit.

## Next Phase Recommendation

**LIVE_USB_QUIL_CREATE_SAVE_REOPEN_TEST_V1**

Now that the Quil buffer append pipeline is proven (synthetic half), the next blocker is the live USB input path. The remaining chain:
1. Physical keyboard → USB HID → sexinput → silk-shell focus route → Quil (live path)
2. Buffer "test" → save/open SexObject → verify "test" survives reopen

The current proof only blocks on steps 3-4 of the full live USB goal:
```
keyboard/input event → shell/focus/editor route → Quil buffer append (THIS PHASE DONE)
→ visible/render marker → buffer text == "test" (THIS PHASE DONE)
→ Quil create "test" → save → reopen → verify (NEXT PHASE)
```
