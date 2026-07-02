# SILK_DE_M2_ASSERT_PATCH_V1

**Status:** COMPLETE (2026-05-03)
**Mission:** SILK_DE_M2_ASSERT_PATCH_V1
**Patches:** F3 (sexdisplay apply_update discard) + F4 (ChipSlot discriminant coupling)

---

## F3 LOW: apply_update() return value discarded

**Symptom:**
sexdisplay handle_silkbar_update() calls apply_update(bar, update) and
discards the return value. Invalid updates (bad kind, out-of-bounds index,
malformed payload) are silently ignored by apply_update() (returns false)
but the caller unconditionally redraws the top strip, wasting render time
with no state change and no log of the bad update.

**Root cause:**
handle_silkbar_update() returned only a bool (is_clock), not the
apply_update() result. The caller assumed all updates succeed.

**Fix:**
- Changed handle_silkbar_update() return type from bool to (bool, u32)
  returning (applied_ok, kind).
- Caller now branches: on true, redraw top strip normally; on false,
  emit bounded log marker [silkde.m2.assert.bad] apply_update=false kind=N
  and do NOT redraw.

**Files changed:**
- servers/sexdisplay/src/main.rs: handle_silkbar_update() + call site

**Boundary:** sexdisplay only. No kernel, sex-pdx, silk-shell, or sexinput changes.

---

## F4 LOW: implicit ChipSlot/CHIP_SLOTS coupling

**Symptom:**
ChipSlot enum discriminants (0=Chip0, 1=Chip1, 2=Chip2, 3=Clock) are
implicitly coupled to the CHIP_SLOTS array in sexdisplay chip_color().
No invariant ties the enum values to array indices. If discriminants drift
(compiler repr change or manual reordering), the renderer indexes into
wrong ModuleSlot positions, producing garbled chip colors or worse.

**Root cause:**
Enum discriminants are explicit (ChipSlot::Chip0 = 0, etc.) but no
runtime or compile-time assertion validates they match expected indices.

**Fix:**
Added four additive checks in validate_contract() (silkbar-model):
ChipSlot::Chip0 as usize == 0
ChipSlot::Chip1 as usize == 1
ChipSlot::Chip2 as usize == 2
ChipSlot::Clock as usize == 3
Bell is NOT included -- it is a ModuleSlot (discriminant 10), not a ChipSlot.

**Files changed:**
- crates/silkbar-model/src/lib.rs: validate_contract()

**Boundary:** silkbar-model only. No renderer, kernel, or ABI changes.

---

## Deferred Findings

### F1 DEFERRED: queue/PDX overflow diagnostics
The update queue is not wired live -- silkbar produces updates but the queue
transport between PDs is not yet connected. Queue overflow diagnostics need
a separate boundary decision in the PDX IPC layer. Not patched here.

### F2 DEFERRED: stale clock watchdog/fallback
If silkbar crashes, the clock stops updating (no fallback to local tick
source). The sexdisplay clock fallback (!clock_from_silkbar path) exists
but has no watchdog. Stale clock liveness needs a separate contract. Not
patched here.

---

## Invariants Preserved

- No kernel edits
- No sex-pdx edits
- No PDX ABI changes
- No silk-shell/sexinput edits
- No renderer refactor
- No framebuffer ownership changes
- FB bounds checks preserved
- sexdisplay startup contract validation fail-open preserved
- Top-strip render proof preserved
- validate_silkbar_contract() returns 0 on pass, 1 on F4 mismatch
- [silk.contract.validate.ok] version=2 still emitted at boot

---

## Verification

Build then run:
  ./scripts/entrypoint_build.sh
  SEXUSB_XHCI_TRACE=0 ./dev.sh run-nographic 2>/tmp/m2-assert.trace | tee /tmp/m2-assert.log

Verify markers:
  grep -aE "silkde.m2.assert|silk.contract|silk.render_proof|shell.silkbar.click|shell.bell|shell.launcher|shell.status|shell.clock|silkbar.workspace|fault|panic|GP|PF|PAGE FAULT|GENERAL PROTECTION" /tmp/m2-assert.log | head -500

**Expected:**
- [silk.contract.validate.ok] version=2
- [silk.render_proof.top_strip.ok]
- Bell/launcher/status/clock/workspace all still work
- NO [silkde.m2.assert.bad] during normal valid boot
- NO PF/GP/panic

---

## Patch Scope (2 files, 0 new deps)

| File | Change | Lines |
|------|--------|-------|
| crates/silkbar-model/src/lib.rs | Add ChipSlot checks to validate_contract() | +8 |
| servers/sexdisplay/src/main.rs | Capture apply_update() bool; log on false | +6/-6 |
