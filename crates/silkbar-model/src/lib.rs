#![no_std]

// ── Geometry Constants (default positions for layout construction) ──────────

pub const PANEL_X: usize = 80;
pub const PANEL_Y: usize = 10;
pub const PANEL_W: usize = 1120;
pub const PANEL_H: usize = 38;
pub const PANEL_GLOW: usize = 2;
pub const PANEL_RADIUS_STEP: usize = 3;
pub const PANEL_MARGIN_TOP: usize = 10;

pub const WORKSPACE_COUNT: usize = 5;
pub const MAX_CHIPS: usize = 4;

pub const WS_Y: usize = 18;
pub const WS_H: usize = 22;
pub const WS_INACTIVE_W: usize = 20;
pub const WS_ACTIVE_W: usize = 30;
pub const WS_X0: usize = 557;
pub const WS_X1: usize = 589;
pub const WS_X2: usize = 625;
pub const WS_X3: usize = 671;
pub const WS_X4: usize = 707;

pub const LAUNCHER_X: usize = 10;
pub const LAUNCHER_Y: usize = 10;
pub const LAUNCHER_W: usize = 80;
pub const LAUNCHER_H: usize = 30;

pub const CHIP_Y: usize = 12;
pub const CHIP_H: usize = 26;
pub const CHIP_W: usize = 56;
pub const CHIP_X0: usize = 1040;
pub const CHIP_X1: usize = 1116;
pub const CHIP_X2: usize = 1192;
pub const CHIP_X3: usize = 1090;
pub const CLOCK_W: usize = 80;
pub const CLOCK_X: usize = 1192;
pub const CLOCK_Y: usize = 16;

/// Total number of layout boxes: 1 launcher + 5 workspaces + 4 chips
pub const LAYOUT_COUNT: usize = 10;

/// ABI version for SilkBar model shared across PDX boundary.
/// Increment when `SilkBarUpdate` layout or `UpdateKind` discriminants change.
pub const ABI_VERSION: u32 = 1;

// ── PDX Protocol Opcodes (v6: wire names exist, no live transport yet) ──────

/// PDX-facing ABI version (u64 for register-width return).
pub const SILKBAR_ABI_VERSION: u64 = 1;

/// Opcode: ping → returns 0 (connectivity check).
pub const OP_SILKBAR_PING: u64 = 0xF0;
/// Opcode: get ABI version → returns SILKBAR_ABI_VERSION.
pub const OP_SILKBAR_GET_ABI: u64 = 0xF1;
/// Opcode: push SilkBarUpdate (unpacked across arg0/arg1/arg2).
pub const OP_SILKBAR_UPDATE: u64 = 0xF2;

// ── Data Model Types ────────────────────────────────────────────────────────

/// Identity of a module slot in the panel layout.
#[derive(Clone, Copy, PartialEq)]
pub enum Module {
    Launcher,
    Workspaces(usize),
    StatusChip(usize),
    Clock,
}

/// Action triggered when this module slot is clicked.
#[derive(Clone, Copy, PartialEq)]
pub enum Action {
    None,
    OpenLauncher,
    SwitchWorkspace(u8),
    ToggleModule(Module),
    OpenClock,
}

/// Geometry + identity + action for one module slot.
#[derive(Clone, Copy)]
pub struct LayoutBox {
    pub x: usize,
    pub y: usize,
    pub w: usize,
    pub h: usize,
    pub module: Module,
    pub action: Action,
}

#[derive(Clone, Copy)]
pub struct WorkspaceState {
    pub index: u8,
    pub active: bool,
    pub urgent: bool,
}

/// Chip kind (visual variant).
#[derive(Clone, Copy, PartialEq)]
pub enum ChipKind {
    Net,
    Wifi,
    Battery,
    Clock,
}

#[derive(Clone, Copy)]
pub struct ChipState {
    pub kind: ChipKind,
    pub visible: bool,
}

#[derive(Clone, Copy)]
pub struct SilkBar {
    /// Layout boxes define position + identity for every module.
    pub layout: [LayoutBox; LAYOUT_COUNT],
    /// Workspace state data (indexed by Module::Workspaces(i)).
    pub workspaces: [WorkspaceState; WORKSPACE_COUNT],
    /// Chip state data (indexed by Module::StatusChip(i)).
    pub chips: [ChipState; MAX_CHIPS],
    pub clock_hh: u8,
    pub clock_mm: u8,
}

// ── Theme (v2: 10 semantic tokens) ─────────────────────────────────────────

/// 10-token semantic color palette.
/// Bevel / highlight / shadow colors are derived at runtime via Theme::bevels().
pub struct Theme {
    /// Desktop background gradient top
    pub bg_top: u32,
    /// Desktop background gradient bottom
    pub bg_bottom: u32,
    /// Panel body fill
    pub panel_fill: u32,
    /// Panel outer glow
    pub panel_glow: u32,
    /// Foreground / label color
    pub text: u32,
    /// Inactive / secondary elements
    pub muted: u32,
    /// Active / focused elements
    pub active: u32,
    /// Urgent / alert elements
    pub urgent: u32,
    /// Chip background fill
    pub chip_fill: u32,
    /// Chip border / shadow
    pub chip_border: u32,
}

// ── Color Helpers ──────────────────────────────────────────────────────────

/// Lighten each RGB component by 0x22 (saturating).
#[inline]
pub fn lighten(c: u32) -> u32 {
    let r = ((c & 0xFF) + 0x22).min(0xFF);
    let g = (((c >> 8) & 0xFF) + 0x22).min(0xFF);
    let b = (((c >> 16) & 0xFF) + 0x22).min(0xFF);
    (c & 0xFF000000) | (b << 16) | (g << 8) | r
}

/// Darken each RGB component by 0x44 (saturating).
#[inline]
pub fn darken(c: u32) -> u32 {
    let r = (c & 0xFF).saturating_sub(0x44);
    let g = ((c >> 8) & 0xFF).saturating_sub(0x44);
    let b = ((c >> 16) & 0xFF).saturating_sub(0x44);
    (c & 0xFF000000) | (b << 16) | (g << 8) | r
}

// ── Derived Bevel Palette ──────────────────────────────────────────────────

/// Full set of 14 derived bevel / highlight / shadow colors computed from Theme.
#[derive(Clone, Copy)]
pub struct BevelPalette {
    pub ws_active_highlight: u32,
    pub ws_active_shadow: u32,
    pub ws_inactive_highlight: u32,
    pub ws_inactive_shadow: u32,
    pub chip_highlight: u32,
    pub glow_top_inner: u32,
    pub glow_bottom_inner: u32,
    pub glow_bottom_outer: u32,
    pub panel_border_top: u32,
    pub panel_border_bottom: u32,
    pub launcher_highlight: u32,
    pub launcher_shadow: u32,
    pub launcher_fill: u32,
    pub panel_body: u32,
}

impl Theme {
    /// Derive the full bevel palette from the 10 semantic tokens.
    /// Uses simple lighten/darken operations on the base colors.
    pub fn bevels(&self) -> BevelPalette {
        BevelPalette {
            ws_active_highlight:  lighten(self.active),
            ws_active_shadow:     darken(self.active),
            ws_inactive_highlight: lighten(self.muted),
            ws_inactive_shadow:   darken(self.muted),
            chip_highlight:       lighten(self.chip_fill),
            glow_top_inner:       darken(self.panel_glow),
            glow_bottom_inner:    darken(darken(self.panel_glow)),
            glow_bottom_outer:    darken(darken(darken(self.panel_glow))),
            panel_border_top:     lighten(self.panel_fill),
            panel_border_bottom:  darken(self.panel_fill),
            launcher_highlight:   lighten(self.panel_fill),
            launcher_shadow:      darken(self.panel_fill),
            launcher_fill:        self.panel_fill,
            panel_body:           self.panel_fill,
        }
    }
}

// ── Hit Test ────────────────────────────────────────────────────────────────

/// Returns the Action for the module at (x, y) in the panel, or `Action::None`.
pub fn hit_test_action(bar: &SilkBar, x: usize, y: usize) -> Action {
    if y < PANEL_Y || y >= PANEL_Y + PANEL_H || x < PANEL_X || x >= PANEL_X + PANEL_W {
        return Action::None;
    }
    for i in 0..LAYOUT_COUNT {
        let lb = &bar.layout[i];
        if x >= lb.x && x < lb.x + lb.w && y >= lb.y && y < lb.y + lb.h {
            return lb.action;
        }
    }
    Action::None
}

// ── Update ABI (v4: stable PDX message types) ──────────────────────────────

/// Discriminant for `SilkBarUpdate.kind`.
/// `#[repr(u32)]` so it maps directly across the PDX ABI boundary.
#[repr(u32)]
#[derive(Clone, Copy, PartialEq)]
pub enum UpdateKind {
    SetWorkspaceActive = 0,
    SetWorkspaceUrgent = 1,
    SetChipVisible = 2,
    SetChipKind = 3,
    SetClock = 4,
    SetThemeToken = 5,
}

/// ABI-stable update message for mutating a `SilkBar` from a PDX caller.
/// Layout is `#[repr(C)]` with natural alignment padding.
///
/// # ABI Contract
/// - Total size must be 16 bytes (asserted at compile time).
/// - `kind` maps to `UpdateKind` discriminants.
/// - Invalid `kind` or out-of-bounds `index` are silently rejected by `apply_update`.
/// - The queue producer (future PDX server) pushes `SilkBarUpdate` values.
///   The queue consumer (sexdisplay render loop) drains and applies them.
///   There is no overwrite: a full queue returns `false` from `push()`.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct SilkBarUpdate {
    /// `UpdateKind` discriminant (passed as u32 for C-ABI safety).
    pub kind: u32,
    /// Slot index (workspace index, chip index, or theme token index).
    pub index: u8,
    /// Primary value field.
    pub a: u32,
    /// Secondary value field (e.g. clock minutes).
    pub b: u32,
}

/// Compile-time size check for the PDX ABI struct.
pub const SILKBAR_UPDATE_SIZE: usize = core::mem::size_of::<SilkBarUpdate>();

impl SilkBarUpdate {
    /// Construct an update from raw fields.
    pub const fn new(kind: u32, index: u8, a: u32, b: u32) -> Self {
        SilkBarUpdate { kind, index, a, b }
    }
}

/// Convenience wrapper: construct a `SilkBarUpdate` from raw fields and apply it.
pub fn apply_raw_update(bar: &mut SilkBar, kind: u32, index: u8, a: u32, b: u32) -> bool {
    let update = SilkBarUpdate { kind, index, a, b };
    apply_update(bar, update)
}

/// Apply a `SilkBarUpdate` to a mutable `SilkBar`.
///
/// Returns `true` on success, `false` if the update kind is unknown,
/// the index is out of bounds, or the payload is invalid.
/// Invalid updates are silently ignored (no panic, no undefined state).
///
/// # Producer / Consumer Model
/// - **Producer** (future PDX server): pushes `SilkBarUpdate` values via `push()`.
/// - **Consumer** (sexdisplay render loop): drains the queue via `drain_into()`
///   once per frame, which calls `apply_update()` for each entry.
/// - **No overwrite**: a full queue rejects new pushes.
/// - **No crash**: malformed updates are silently dropped.
pub fn apply_update(bar: &mut SilkBar, update: SilkBarUpdate) -> bool {
    match update.kind {
        0 => {
            // SetWorkspaceActive: index=ws_idx, a=0|1
            let idx = update.index as usize;
            if idx >= WORKSPACE_COUNT {
                return false;
            }
            bar.workspaces[idx].active = update.a != 0;
            true
        }
        1 => {
            // SetWorkspaceUrgent: index=ws_idx, a=0|1
            let idx = update.index as usize;
            if idx >= WORKSPACE_COUNT {
                return false;
            }
            bar.workspaces[idx].urgent = update.a != 0;
            true
        }
        2 => {
            // SetChipVisible: index=chip_idx, a=0|1
            let idx = update.index as usize;
            if idx >= MAX_CHIPS {
                return false;
            }
            bar.chips[idx].visible = update.a != 0;
            true
        }
        3 => {
            // SetChipKind: index=chip_idx, a=ChipKind as u32
            let idx = update.index as usize;
            if idx >= MAX_CHIPS {
                return false;
            }
            let kind = match update.a {
                0 => ChipKind::Net,
                1 => ChipKind::Wifi,
                2 => ChipKind::Battery,
                3 => ChipKind::Clock,
                _ => return false,
            };
            bar.chips[idx].kind = kind;
            true
        }
        4 => {
            // SetClock: a=hh (0-23), b=mm (0-59)
            let hh = update.a.min(23) as u8;
            let mm = update.b.min(59) as u8;
            bar.clock_hh = hh;
            bar.clock_mm = mm;
            true
        }
        5 => {
            // SetThemeToken: acknowledged but no-op (Theme is not part of SilkBar).
            // Future: route to mutable theme storage.
            false
        }
        _ => false,
    }
}

// ── Update Queue (v5: fixed ring buffer) ───────────────────────────────────

/// Capacity of the fixed-size update ring buffer.
pub const UPDATE_QUEUE_CAP: usize = 32;

/// Lock-free ring buffer of `SilkBarUpdate` entries.
/// `#[repr(C)]` for stable ABI when shared across PDX boundary.
#[repr(C)]
pub struct SilkBarUpdateQueue {
    head: u32,
    tail: u32,
    updates: [SilkBarUpdate; UPDATE_QUEUE_CAP],
}

impl SilkBarUpdateQueue {
    /// Create an empty queue (all slots available).
    pub fn empty() -> Self {
        SilkBarUpdateQueue {
            head: 0,
            tail: 0,
            updates: [SilkBarUpdate { kind: 0, index: 0, a: 0, b: 0 }; UPDATE_QUEUE_CAP],
        }
    }

    /// Enqueue an update.
    /// Returns `false` if the queue is full (no overwrite).
    pub fn push(&mut self, update: SilkBarUpdate) -> bool {
        let cap = UPDATE_QUEUE_CAP as u32;
        let next_tail = (self.tail + 1) % cap;
        if next_tail == self.head {
            return false; // full
        }
        self.updates[self.tail as usize] = update;
        self.tail = next_tail;
        true
    }

    /// Dequeue the oldest update, or `None` if empty.
    pub fn pop(&mut self) -> Option<SilkBarUpdate> {
        if self.head == self.tail {
            return None; // empty
        }
        let cap = UPDATE_QUEUE_CAP as u32;
        let update = self.updates[self.head as usize];
        self.head = (self.head + 1) % cap;
        Some(update)
    }

    /// Drain all pending updates into `bar`, applying each via `apply_update`.
    /// Returns the number of successfully applied updates.
    pub fn drain_into(&mut self, bar: &mut SilkBar) -> u32 {
        let mut count = 0u32;
        while let Some(update) = self.pop() {
            if apply_update(bar, update) {
                count += 1;
            }
        }
        count
    }
}

// ── ABI Assertions (compile time) ──────────────────────────────────────────

/// `SilkBarUpdate` must be exactly 16 bytes for a stable PDX ABI.
const _: () = assert!(core::mem::size_of::<SilkBarUpdate>() == 16);

/// Ring-buffer capacity must be 32 (power of two simplifies future mask-based indexing).
const _: () = assert!(UPDATE_QUEUE_CAP == 32);

/// `ABI_VERSION` must be non-zero.
const _: () = assert!(ABI_VERSION > 0);

// ── Invariant Tests (runtime) ──────────────────────────────────────────────

/// Run all queue invariant tests.
/// Returns `true` if all pass.
///
/// Tests:
/// 1. Empty queue returns `None` on pop.
/// 2. Push then pop returns the same update.
/// 3. Full queue rejects additional pushes.
/// 4. `drain_into` applies a clock update correctly.
pub fn validate_invariants() -> bool {
    // 1. Empty queue
    let mut q = SilkBarUpdateQueue::empty();
    if q.pop().is_some() {
        return false;
    }

    // 2. Push then pop identity
    let u = SilkBarUpdate { kind: 4, index: 0, a: 10, b: 30 };
    if !q.push(u) {
        return false;
    }
    match q.pop() {
        Some(p) => {
            if p.kind != 4 || p.index != 0 || p.a != 10 || p.b != 30 {
                return false;
            }
        }
        None => return false,
    }

    // 3. Full queue rejects overwrite
    let mut q2 = SilkBarUpdateQueue::empty();
    let dummy = SilkBarUpdate { kind: 0, index: 0, a: 0, b: 0 };
    // Fill to capacity - 1 (one slot always reserved for empty-vs-full distinction)
    for _ in 0..UPDATE_QUEUE_CAP - 1 {
        if !q2.push(dummy) {
            return false;
        }
    }
    // Next push must fail
    if q2.push(dummy) {
        return false;
    }

    // 4. drain_into applies clock update
    let mut q3 = SilkBarUpdateQueue::empty();
    let clock = SilkBarUpdate { kind: 4, index: 0, a: 15, b: 45 };
    if !q3.push(clock) {
        return false;
    }
    let mut bar = DEFAULT_SILK_BAR;
    let count = q3.drain_into(&mut bar);
    if count != 1 {
        return false;
    }
    if bar.clock_hh != 15 || bar.clock_mm != 45 {
        return false;
    }

    true
}

// ── Default Instances ───────────────────────────────────────────────────────

pub const DEFAULT_SILK_BAR: SilkBar = SilkBar {
    layout: [
        LayoutBox { x: LAUNCHER_X, y: LAUNCHER_Y, w: LAUNCHER_W, h: LAUNCHER_H, module: Module::Launcher, action: Action::OpenLauncher },
        LayoutBox { x: WS_X0, y: WS_Y, w: WS_INACTIVE_W, h: WS_H, module: Module::Workspaces(0), action: Action::SwitchWorkspace(1) },
        LayoutBox { x: WS_X1, y: WS_Y, w: WS_INACTIVE_W, h: WS_H, module: Module::Workspaces(1), action: Action::SwitchWorkspace(2) },
        LayoutBox { x: WS_X2, y: WS_Y, w: WS_ACTIVE_W,   h: WS_H, module: Module::Workspaces(2), action: Action::SwitchWorkspace(3) },
        LayoutBox { x: WS_X3, y: WS_Y, w: WS_INACTIVE_W, h: WS_H, module: Module::Workspaces(3), action: Action::SwitchWorkspace(4) },
        LayoutBox { x: WS_X4, y: WS_Y, w: WS_INACTIVE_W, h: WS_H, module: Module::Workspaces(4), action: Action::SwitchWorkspace(5) },
        LayoutBox { x: CHIP_X0, y: CHIP_Y, w: CHIP_W, h: CHIP_H, module: Module::StatusChip(0), action: Action::ToggleModule(Module::StatusChip(0)) },
        LayoutBox { x: CHIP_X1, y: CHIP_Y, w: CHIP_W, h: CHIP_H, module: Module::StatusChip(1), action: Action::ToggleModule(Module::StatusChip(1)) },
        LayoutBox { x: CHIP_X2, y: CHIP_Y, w: CHIP_W, h: CHIP_H, module: Module::StatusChip(2), action: Action::ToggleModule(Module::StatusChip(2)) },
        LayoutBox { x: CHIP_X3, y: CHIP_Y, w: CLOCK_W, h: CHIP_H, module: Module::Clock,        action: Action::OpenClock },
    ],
    workspaces: [
        WorkspaceState { index: 1, active: false, urgent: false },
        WorkspaceState { index: 2, active: false, urgent: false },
        WorkspaceState { index: 3, active: true,  urgent: false },
        WorkspaceState { index: 4, active: false, urgent: false },
        WorkspaceState { index: 5, active: false, urgent: false },
    ],
    chips: [
        ChipState { kind: ChipKind::Net,     visible: true },
        ChipState { kind: ChipKind::Wifi,    visible: true },
        ChipState { kind: ChipKind::Battery, visible: true },
        ChipState { kind: ChipKind::Clock,   visible: true },
    ],
    clock_hh: 10,
    clock_mm: 42,
};

pub const DEFAULT_THEME: Theme = Theme {
    bg_top:      0x000A1C1C,
    bg_bottom:   0x00163434,
    panel_fill:  0x00191433,
    panel_glow:  0x00302855,
    text:        0x00FFFFFF,
    muted:       0x004C3C88,
    active:      0x00BBAAFF,
    urgent:      0x00FF6666,
    chip_fill:   0x009EA8FF,
    chip_border: 0x006670AA,
};
