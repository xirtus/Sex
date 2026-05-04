# SilkBar ABI & Contract Reference

> Referenced from CLAUDE.md (offloaded reference).

---

## SilkBar ABI

- `SilkBarUpdate`: `#[repr(C)]` 16 bytes: kind(u32), index(u8), a(u32), b(u32)
- Update kinds: 0=SetWorkspaceActive, 1=SetWorkspaceUrgent, 2=SetChipVisible, 3=SetChipKind, 4=SetClock, 5=SetThemeToken
- `silkbar-model` crate provides: types, `DEFAULT_SILK_BAR` (const), `DEFAULT_THEME` (const), `apply_update()`, `SilkBarUpdateQueue`
- sexdisplay imports `silkbar-model` for types; renders clock chip at position CHIP_X3=1090, CHIP_Y=18

---

## SilkBar Action Slot Expansion (ABI v1→v2)

Bell module slot (index 10, between Battery and Clock, x=1020).

**Key invariants:**
- `LAYOUT_COUNT = 11` (was 10)
- `MAX_CHIPS = 4` (unchanged — Bell is a ModuleSlot, not a chip)
- Bell hit-test → `Action::OpenBell` (no panel toggle yet)
- Bell rendering: gold 0x00FFD700 at (1020, 18, 18, 22)
- After this expansion is proven, `BELL_PANEL_TOGGLE_V1` wires toggle_os_panel()

---

## Shell Global Interaction Contract (2026-05-03)

**Core insight:** Local phase proofs are not sufficient. Global UI behavior can fail from
event-order bugs, focus conflicts, chrome conflicts, surface ID ambiguity, or dead-PD
dangling state.

7 subcontracts govern interaction integrity:
- **A. SHELL_INTERACTION_STATE_V1** — unified state table (no scattered `*_ACTIVE` booleans)
- **B. HIT_TEST_PRIORITY_V1** — strict z-order
- **C. EVENT_ORDERING_CONTRACT_V1** — deterministic pipeline
- **D. SURFACE_ID_LIFETIME_V1** — monotonic IDs, tombstoning
- **E. CHROME_MODE_ARBITRATION_V1** — exclusive chrome, no focus steal
- **F. DEAD_PD_SURFACE_CLEANUP_V1** — safe teardown
- **G. INTEGRATED_SCENARIO_PROOF_V1** — combined scenario verification

**Every feature must prove:** boundary proof, negative proof, integration proof, handoff proof.

**Stable Baseline Reference:** Read `docs/handoff/STABLE_BASELINE_20260503.md` before any new feature work.

---

## Surface ID Registry

| ID  | Surface    |
|-----|------------|
| 0x90| Cursor     |
| 0x92-0x94| Panels |
| 0x95| Reserved   |
| 100+| Apps       |

**Click-focus guard:** `CLICK_ACTIVE` bool prevents repeat focus on held button. Rising edge only (button down, not held).
