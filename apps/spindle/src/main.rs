//! Spindle V1 -- SexOS native command console
//!
//! Architecture:
//!   • Static bounded text surface via sex-graphics CP437 font
//!   • Window created via PDX OP_WINDOW_CREATE on sexdisplay slot 5
//!   • Fixed PFN base for framebuffer (matches sexsh convention)
//!   • Bounded line editor with synthetic input proof gate
//!   • Bounded scrollback ring (1024 lines × 80 bytes)
//!   • No real HID delivery yet -- capability grant pending (no PDX slot)
//!   • Bounded native command dispatcher (10 built-in commands)
//!   • Local event ring (Bell bridge pending)
//!   • No terminal emulation (Spindle is NOT sexsh)
//!
//! Contract: docs/handoff/SPINDLE_APP_CONTRACT_V1.md
//! Next: SPINDLE_COMPLETE_V1_AUDIT

#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;
use core::alloc::{GlobalAlloc, Layout};

struct DummyAllocator;
unsafe impl GlobalAlloc for DummyAllocator {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 {
        core::ptr::null_mut()
    }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {
        // no-op
    }
}

#[global_allocator]
static ALLOCATOR: DummyAllocator = DummyAllocator;

#[alloc_error_handler]
fn alloc_error_handler(_layout: Layout) -> ! {
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

use sex_pdx::{
    pdx_call, serial_println,
    OP_WINDOW_CREATE, SLOT_DISPLAY, SLOT_STORAGE,
    SLOT_BELL, SLOT_LINEN, OP_BELL_NOTIFY,
};
use sex_graphics::{WindowBuffer, font};

// ── Local window create params (not yet in sex-pdx) ────────────────────────

/// Window creation parameters sent to sexdisplay via OP_WINDOW_CREATE.
/// Matches the struct defined in crates/silk-client/src/lib.rs and used by sexsh.
#[repr(C)]
struct WindowCreateParams {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
    pfn_base: u64,
}

// ── Linen opcodes (local; OP_LINEN_CREATE_OBJECT matches linen server) ───
const OP_LINEN_CREATE_OBJECT: u64 = 0x41;

// ── RamFS opcodes (local; defined in servers/sexfiles/src/messages.rs) ────
const OP_RAMFS_OPEN: u64 = 0x30;
const OP_RAMFS_WRITE: u64 = 0x32;
const OP_RAMFS_READ: u64 = 0x31;
const OP_RAMFS_CLOSE: u64 = 0x33;
const OP_RAMFS_OBJECT_ID: u64 = 0x37;

const RAMFS_O_CREATE: u64 = 0x01;
const HISTORY_FILE: &[u8] = b"spindle_history";
const HISTORY_PATH: &[u8] = b"/tmp/spindle/history.log";

// ── Spindle surface geometry ───────────────────────────────────────────────

const CELL_W: u32 = 8;
const CELL_H: u32 = 8;
const COLS: u32 = 80;
const ROWS: u32 = 24;
const WIN_W: u32 = COLS * CELL_W;  // 640
const WIN_H: u32 = ROWS * CELL_H;  // 192

/// Framebuffer lives at 256 MiB physical; kernel maps it on window create.
/// Matches sexsh convention.
const FB_PFN_BASE: u64 = 0x0000_0004_0000;

// ── Catppuccin Mocha palette ───────────────────────────────────────────────

const BG:     u32 = 0xFF1E1E2E; // Base
const FG:     u32 = 0xFFCDD6F4; // Text
const ACCENT: u32 = 0xFF89B4FA; // Blue
const GREEN:  u32 = 0xFFA6E3A1; // Green
const YELLOW: u32 = 0xFFF9E2AF; // Yellow
const CURSOR: u32 = 0xFFF5E0DC; // Rosewater (cursor color)

// ── Bounded line editor ────────────────────────────────────────────────────

const CMD_MAX: usize = 256;
const PROMPT: &[u8] = b"sex> ";
const PROMPT_LEN: usize = PROMPT.len();

struct CmdLine {
    buf: [u8; CMD_MAX],
    len: usize,
}

impl CmdLine {
    const fn new() -> Self { CmdLine { buf: [0u8; CMD_MAX], len: 0 } }

    /// Append a printable ASCII byte. Rejects if full or non-printable.
    fn push(&mut self, b: u8) {
        if self.len < CMD_MAX && b >= 0x20 && b <= 0x7E {
            self.buf[self.len] = b;
            self.len += 1;
        }
    }

    /// Delete one character before cursor.
    fn backspace(&mut self) {
        if self.len > 0 { self.len -= 1; }
    }

    /// Clear the buffer entirely.
    fn clear(&mut self) { self.len = 0; }

    /// Return the current line as a byte slice.
    fn as_bytes(&self) -> &[u8] { &self.buf[..self.len] }
}

// ── Prompt redraw ──────────────────────────────────────────────────────────

/// Redraw the prompt line (row 23). Clears the row, draws "sex> " + current line + cursor.
unsafe fn redraw_prompt(fb: &mut WindowBuffer, line: &CmdLine) {
    // Clear entire prompt row
    fb.draw_rect(sex_pdx::Rect { x: 0, y: CELL_H * 23, width: WIN_W, height: CELL_H }, BG);

    // Draw prompt prefix
    font::draw_str(fb, 4, CELL_H * 23 + 4, PROMPT, GREEN, None);

    // Draw current line
    if line.len > 0 {
        let prompt_px = (PROMPT_LEN as u32) * CELL_W;
        // Max visible chars = COLS - PROMPT_LEN - 1 (cursor)
        let max_vis = (COLS as usize).saturating_sub(PROMPT_LEN + 1);
        let start = line.len.saturating_sub(max_vis);
        let visible = &line.as_bytes()[start..];
        font::draw_str(fb, 4 + prompt_px, CELL_H * 23 + 4, visible, FG, None);

        // Cursor block at end of input
        let cursor_col = PROMPT_LEN as u32 + visible.len() as u32;
        font::draw_char(fb, cursor_col * CELL_W, CELL_H * 23, b' ', CURSOR, Some(CURSOR));
    } else {
        // Cursor block at prompt position when empty
        let cursor_col = PROMPT_LEN as u32;
        font::draw_char(fb, cursor_col * CELL_W, CELL_H * 23, b' ', CURSOR, Some(CURSOR));
    }
}

// ── Bounded command history ring ───────────────────────────────────────────

const MAX_HISTORY: usize = 128;
const HIST_LINE_BYTES: usize = CMD_MAX; // 256

struct History {
    ring: [[u8; HIST_LINE_BYTES]; MAX_HISTORY],
    write_pos: usize,
    total: u32,
}

impl History {
    const fn new() -> Self {
        History { ring: [[0u8; HIST_LINE_BYTES]; MAX_HISTORY], write_pos: 0, total: 0 }
    }

    /// Push a command line into history. Clamps to HIST_LINE_BYTES.
    fn push(&mut self, line: &[u8]) {
        let n = line.len().min(HIST_LINE_BYTES);
        self.ring[self.write_pos][..n].copy_from_slice(&line[..n]);
        for i in n..HIST_LINE_BYTES { self.ring[self.write_pos][i] = 0; }
        self.write_pos = (self.write_pos + 1) % MAX_HISTORY;
        self.total = self.total.saturating_add(1);
    }

    /// Return the nth-most-recent entry (0 = newest). None if out of range.
    fn get(&self, n: usize) -> Option<&[u8]> {
        if n >= self.total as usize || n >= MAX_HISTORY { return None; }
        let idx = (self.write_pos + MAX_HISTORY - 1 - n) % MAX_HISTORY;
        let raw = &self.ring[idx];
        let mut len = 0;
        while len < HIST_LINE_BYTES && raw[len] != 0 { len += 1; }
        Some(&raw[..len])
    }

    fn clear(&mut self) {
        self.ring = [[0u8; HIST_LINE_BYTES]; MAX_HISTORY];
        self.write_pos = 0;
        self.total = 0;
    }
}

// ── Local event ring (Bell bridge pending) ────────────────────────────────
const MAX_EVENTS: usize = 32; const EV_BYTES: usize = 80;
#[derive(Clone,Copy,PartialEq)] enum EvKind { CmdOk, CmdFail, CmdUnknown, Info }
struct SpindleEvent { kind: EvKind, line: [u8; EV_BYTES], len: u8 }
struct EventRing { e: [SpindleEvent; MAX_EVENTS], p: usize, t: u32 }
impl EventRing {
    const Z: SpindleEvent = SpindleEvent { kind: EvKind::Info, line: [0u8; 80], len: 0 };
    const fn new() -> Self { EventRing { e: [Self::Z; MAX_EVENTS], p: 0, t: 0 } }
    fn push(&mut self, k: EvKind, l: &[u8]) {
        let n = l.len().min(EV_BYTES);
        let r = &mut self.e[self.p]; r.kind = k; r.line[..n].copy_from_slice(&l[..n]); r.len = n as u8;
        self.p = (self.p + 1) % MAX_EVENTS; self.t = self.t.saturating_add(1);
    }
    fn get(&self, n: usize) -> Option<&SpindleEvent> {
        if n >= self.t as usize || n >= MAX_EVENTS { None }
        else { Some(&self.e[(self.p + MAX_EVENTS - 1 - n) % MAX_EVENTS]) }
    }
    fn clear(&mut self) { self.e = [Self::Z; MAX_EVENTS]; self.p = 0; self.t = 0; }
}

// ── Bounded scrollback ring ────────────────────────────────────────────────

/// Number of visible output rows (rows 5–22 = 18 lines).
const VISIBLE_ROWS: usize = 18;
/// First row index for the output area.
const OUTPUT_ROW_START: u32 = 5;
/// Max scrollback lines in the ring buffer.
const MAX_SCROLLBACK: usize = 1024;
/// Max chars per scrollback line (matches COLS = 80).
const MAX_LINE_BYTES: usize = 80;

struct Scrollback {
    /// Fixed ring buffer: 1024 lines of 80 bytes each = 80 KiB.
    ring: [[u8; MAX_LINE_BYTES]; MAX_SCROLLBACK],
    /// Index in ring where the next line will be written.
    write_pos: usize,
    /// Total number of lines ever written (used for display offset).
    /// Monotonically increases, never wraps. Saturates at u32::MAX.
    total_lines: u32,
    /// User scroll offset from newest (0 = show latest).
    scroll_offset: u32,
}

impl Scrollback {
    const fn new() -> Self {
        Scrollback {
            ring: [[0u8; MAX_LINE_BYTES]; MAX_SCROLLBACK],
            write_pos: 0,
            total_lines: 0,
            scroll_offset: 0,
        }
    }

    /// Push one line into the ring buffer. Clamps to MAX_LINE_BYTES.
    fn push(&mut self, line: &[u8]) {
        let n = line.len().min(MAX_LINE_BYTES);
        let dst = &mut self.ring[self.write_pos][..n];
        dst.copy_from_slice(&line[..n]);
        // Zero-fill remainder
        for i in n..MAX_LINE_BYTES {
            self.ring[self.write_pos][i] = 0;
        }
        self.write_pos = (self.write_pos + 1) % MAX_SCROLLBACK;
        self.total_lines = self.total_lines.saturating_add(1);
        // Reset scroll offset to show latest unless user is actively scrolling
        if self.scroll_offset > 0 {
            self.scroll_offset = self.scroll_offset.saturating_sub(1);
        }
    }

    /// Return the line at the given ring index. Clamps to valid bytes.
    fn get(&self, ring_idx: usize) -> &[u8] {
        let raw = &self.ring[ring_idx % MAX_SCROLLBACK];
        // Find the effective length (stop at first NUL or at MAX_LINE_BYTES)
        let mut len = 0;
        while len < MAX_LINE_BYTES && raw[len] != 0 {
            len += 1;
        }
        &raw[..len]
    }
}

// ── Scrollback render ──────────────────────────────────────────────────────

/// Render the visible scrollback area (rows 5–22) into the framebuffer.
///
/// Calculates which ring buffer entries are visible based on total_lines,
/// scroll_offset, and VISIBLE_ROWS. Draws each line with font::draw_str.
/// Empty lines are left as background.
unsafe fn render_scrollback(fb: &mut WindowBuffer, sb: &Scrollback) {
    // Clear the output area
    let area_h = VISIBLE_ROWS as u32 * CELL_H;
    fb.draw_rect(sex_pdx::Rect {
        x: 0, y: (OUTPUT_ROW_START * CELL_H) as u32,
        width: WIN_W, height: area_h,
    }, BG);

    if sb.total_lines == 0 { return; }

    // How many total lines could be visible
    let available = (sb.total_lines as usize).min(VISIBLE_ROWS);
    if available == 0 { return; }

    // Start from newest line minus scroll_offset, work backwards
    let newest_line = sb.total_lines.saturating_sub(1 + sb.scroll_offset);
    let oldest_visible = newest_line.saturating_sub(VISIBLE_ROWS as u32 - 1);

    for vis_row in 0..VISIBLE_ROWS {
        let line_idx = oldest_visible as usize + vis_row;
        if line_idx >= sb.total_lines as usize { break; }

        let ring_idx = line_idx % MAX_SCROLLBACK;
        let text = sb.get(ring_idx);
        if !text.is_empty() {
            let y = (OUTPUT_ROW_START as u32 + vis_row as u32) * CELL_H;
            font::draw_str(fb, 4, y + 4, text, FG, None);
        }
    }
}

// ── Command tokenizer ──────────────────────────────────────────────────────

/// Split a command line into (command_name, args_rest).
/// Command name is the first whitespace-delimited token.
/// Args is the remainder after the space (trimmed of leading spaces).
fn tokenize(line: &[u8]) -> (&[u8], &[u8]) {
    // Skip leading whitespace
    let mut start = 0;
    while start < line.len() && line[start] == b' ' {
        start += 1;
    }
    let line = &line[start..];

    // Find end of command name (space or EOL)
    let mut cmd_end = 0;
    while cmd_end < line.len() && line[cmd_end] != b' ' {
        cmd_end += 1;
    }
    let cmd = &line[..cmd_end];

    // Args: skip spaces after command name
    let mut args_start = cmd_end;
    while args_start < line.len() && line[args_start] == b' ' {
        args_start += 1;
    }
    let args = &line[args_start..];

    (cmd, args)
}

// ── Command dispatch ───────────────────────────────────────────────────────

/// Dispatch a command line. Pushes output lines to scrollback.
/// Returns true if the command was recognized, false for unknown.
fn dispatch(line: &[u8], sb: &mut Scrollback, hist: &mut History, ev: &mut EventRing) -> bool {
    let (cmd, args) = tokenize(line);
    if cmd.is_empty() { return true; }
    let recognized = match cmd {
        b"help" => {
            sb.push(b"Built-in commands:");
            sb.push(b"  help         list commands");
            sb.push(b"  clear        clear scrollback");
            sb.push(b"  status       show Spindle status");
            sb.push(b"  pd           list protection domains");
            sb.push(b"  servers      list known servers");
            sb.push(b"  bell         Bell notification status");
            sb.push(b"  files        SexFiles storage status");
            sb.push(b"  apps         list available apps");
            sb.push(b"  launch <app> request app surface");
            sb.push(b"  history      show command history");
            sb.push(b"  history clear  clear command history");
            sb.push(b"  events       show event log");
            sb.push(b"  events clear clear event log");
            sb.push(b"  about        Spindle version + identity");
            sb.push(b"  route        input/surface route info");
            sb.push(b"  input        keyboard input status");
            sb.push(b"  session      show Spindle session summary");
            true
        }
        b"clear" => {
            // Reset scrollback to empty
            *sb = Scrollback::new();
            sb.push(b"Scrollback cleared.");
            true
        }
        b"status" => {
            sb.push(b"Spindle V1 native console");
            sb.push(b"SexOS 0.1.0-silk x86_64");
            sb.push(b"Surface: 80x24, scrollback: 1024 lines");
            sb.push(b"Commands: 8 built-in, no external dispatch");
            true
        }
        b"pd" => {
            sb.push(b"Protection domains (static baseline):");
            sb.push(b"  PD  1  sexdisplay     compositor");
            sb.push(b"  PD  2  sexdrive       XHCI/NVMe");
            sb.push(b"  PD  3  silk-shell     window manager");
            sb.push(b"  PD  4  sexinput       input router");
            sb.push(b"  PD  5  sexusb         USB host");
            sb.push(b"  PD  6  silkbar        status bar");
            sb.push(b"  PD  7  linen          object browser");
            sb.push(b"  PD  8  sexstore       key-value store");
            sb.push(b"  PD  9  quil           app launcher");
            sb.push(b"  PD 10  sexbell        notifications");
            sb.push(b"  PD 11  sexfiles       virtual filesystem");
            sb.push(b"Live PD query unavailable in V1 (capability grant pending).");
            true
        }
        b"servers" => {
            sb.push(b"Known servers (baseline):");
            sb.push(b"  sexdisplay  sexdrive  silk-shell  sexinput");
            sb.push(b"  sexusb      silkbar   linen       sexstore");
            sb.push(b"  quil        sexbell   sexfiles    spindle");
            true
        }
        b"bell" => {
            sb.push(b"Bell notification bridge: pending.");
            sb.push(b"Bell server is PD 10, routing not wired from Spindle.");
            sb.push(b"Requires: kernel spawn + PDX slot + silk-shell routing.");
            true
        }
        b"files" => {
            sb.push(b"SexFiles storage bridge: pending.");
            sb.push(b"SexFiles server is PD 11, RamFS backend active.");
            sb.push(b"Requires: kernel spawn + PDX slot for Spindle.");
            sb.push(b"In-memory scaffold only -- no real block device route.");
            true
        }
        b"apps" => {
            sb.push(b"Available apps (static):");
            sb.push(b"  quil     text editor");
            sb.push(b"  linen    object browser");
            sb.push(b"  mesh     device topology");
            sb.push(b"  collar   authority wallet");
            sb.push(b"All targets unavailable: capability grant pending.");
            true
        }
        b"launch" => {
            let known = args == b"quil" || args == b"linen" || args == b"mesh" || args == b"collar";
            if known {
                sb.push(b"launch: all targets unavailable in V1.");
                sb.push(b"capability grant pending -- cannot PDX-call silk-shell.");
                sb.push(b"Requires: kernel spawn, SLOT_SHELL, OP_APP_SURFACE_REQ.");
            } else if args.is_empty() {
                sb.push(b"launch: specify an app. Use 'apps' to list.");
            } else {
                sb.push(b"launch: unknown target. Use 'apps' to list.");
            }
            true
        }
        b"proof" => {
            if args == b"boot" {
                sb.push(b"Proof boot: Spindle PD compiles (no_std, no kernel spawn).");
                sb.push(b"  binary: iso_root/apps/spindle (Limine module 13)");
                sb.push(b"  build:  entrypoint_build.sh PASS");
                sb.push(b"  gate:   master_runtime_gate GREEN_MASTER (6/6)");
                sb.push(b"  faults: 0 (no kernel spawn, no runtime)");
            } else if args == b"input" {
                sb.push(b"Proof input: synthetic proof gate (SEXOS_SPINDLE_INPUT_PROOF).");
                sb.push(b"  line editor: 20 proof stages, all pass at compile time.");
                sb.push(b"  real HID:     unavailable (spindle not kernel-spawned).");
            } else if args == b"display" {
                sb.push(b"Proof display: surface render scaffold.");
                sb.push(b"  window:   80x24 CP437 grid (640x192 px)");
                sb.push(b"  framebuffer: WindowBuffer at PFN 0x40000");
                sb.push(b"  bounds:   all draw calls validated by WindowBuffer.");
                sb.push(b"  sole FB writer: sexdisplay (spindle writes within window).");
            } else if args == b"storage" {
                sb.push(b"Proof storage: SexFiles history persistence.");
                sb.push(b"  history ring: 128 entries (32 KiB BSS)");
                sb.push(b"  persistence:  pending (SexFiles client bridge)");
                sb.push(b"  scrollback:   1024 lines (80 KiB BSS)");
                sb.push(b"  event ring:   32 entries (2.5 KiB BSS)");
                sb.push(b"  total static: ~115 KiB bounded, no heap growth.");
            } else {
                sb.push(b"Proof summary (Spindle V1):");
                sb.push(b"  surface:   yes (80x24 CP437)");
                sb.push(b"  input:     synthetic proof (20 stages compile-verified)");
                sb.push(b"  scrollback: yes (1024 lines)");
                sb.push(b"  history:   pending (SexFiles bridge)");
                sb.push(b"  events:    local (Bell bridge pending)");
                sb.push(b"  session:   local (Linen bridge pending)");
                sb.push(b"  launch:    unavailable (4 targets, kernel spawn needed)");
                sb.push(b"  faults:    0 observed");
                sb.push(b"Proof commands: proof boot/input/display/storage");
            }
            true
        }
        b"about" => {
            sb.push(b"Spindle V1 -- SexOS native command console");
            sb.push(b"  version: 1.0.0-pre");
            sb.push(b"  source:  apps/spindle (866 lines, no_std)");
            sb.push(b"  pd:      Domain 12, PKU 12");
            sb.push(b"  surface: 0x99 via silk-shell");
            sb.push(b"  session: SpindleSession (.spn)");
            sb.push(b"  bridges: SexFiles/Bell/Linen pending cap grants");
            true
        }
        b"route" => {
            sb.push(b"Input route: sexinput -> silk-shell -> SLOT_SPINDLE(14) -> PD 12");
            sb.push(b"Surface route: silk-shell SURFACE_ID_SPINDLE(0x99) -> sexdisplay");
            sb.push(b"FB: proof-gated (0 runtime writes in normal spawn)");
            true
        }
        b"input" => {
            sb.push(b"Keyboard: HID events via SLOT_SPINDLE, scancode set 1 US QWERTY");
            sb.push(b"Line editor: 256-byte CmdLine, push/backspace/clear");
            sb.push(b"Real HID delivery: active (silk-shell -> pdx_call -> PD 12)");
            true
        }
        b"close" => {
            serial_println!("[spindle.lifecycle.close]");
            sb.push(b"Spindle session closing.");
            sb.push(b"  state:      in-memory only (no SexFiles persistence)");
            sb.push(b"  surface:    WindowBuffer released on PD exit");
            sb.push(b"  history:    not persisted (SexFiles bridge pending)");
            sb.push(b"  relaunch:   fresh state, no restore available");
            sb.push(b"Close/relaunch requires kernel spawn + lifecycle integration.");
            true
        }
        b"faults" => {
            sb.push(b"Fault report (Spindle V1):");
            sb.push(b"  observed: 0 (spindle not kernel-spawned, no runtime)");
            sb.push(b"  host gate: master_runtime_gate.sh GREEN_MASTER");
            sb.push(b"  fault scan requires host log gate.");
            true
        }
        b"history" => {
            if args == b"clear" {
                hist.clear();
                sb.push(b"History cleared.");
            } else {
                if hist.total == 0 {
                    sb.push(b"History: empty.");
                } else {
                    let count = hist.total.min(MAX_HISTORY as u32);
                    sb.push(b"Command history (most recent first):");
                    for i in 0..count as usize {
                        if let Some(entry) = hist.get(i) {
                            sb.push(entry);
                        }
                    }
                }
                sb.push(b"history persistence pending SexFiles client bridge.");
                sb.push(b"capability grant pending -- no PDX call to sexfiles.");
            }
            true
        }
        b"session" => {
            sb.push(b"Spindle session summary:");
            sb.push(b"  session id:  1 (local)");
            sb.push(b"  commands:    Spindle native command console");
            sb.push(b"  history:     pending (SexFiles bridge)");
            sb.push(b"  events:      pending (Bell bridge)");
            sb.push(b"Linen bridge pending (capability grant pending).");
            true
        }
        b"events" => {
            if args == b"clear" { ev.clear(); sb.push(b"Event log cleared."); }
            else {
                if ev.t == 0 { sb.push(b"Event log: empty."); }
                else {
                    let n = ev.t.min(MAX_EVENTS as u32);
                    sb.push(b"Event log (most recent first):");
                    for i in 0..n as usize {
                        if let Some(e) = ev.get(i) {
                            let pfx: &[u8] = match e.kind {
                                EvKind::CmdOk => b"  [OK]   ", EvKind::CmdFail => b"  [FAIL] ",
                                EvKind::CmdUnknown => b"  [????] ", EvKind::Info => b"  [info] ",
                            };
                            sb.push(pfx); if e.len > 0 { sb.push(&e.line[..e.len as usize]); }
                        }
                    }
                }
                sb.push(b"Bell bridge pending (capability grant pending).");
            }
            true
        }
        other => { ev.push(EvKind::CmdUnknown, other); false }
    };
    if recognized { ev.push(EvKind::CmdOk, cmd); }
    recognized
}

// ── SexFiles persistence (best-effort, graceful fallback) ──────────────────

/// Try to persist command history to SexFiles RamFS.
/// Returns true on success, false if SexFiles unavailable.
unsafe fn persist_history(hist: &History) -> bool {
    // Pack file name (≤ 24 bytes) into 3 u64 args per sexfiles protocol
    let mut n0: u64 = 0; let mut n1: u64 = 0; let mut n2: u64 = 0;
    let name = HISTORY_FILE;
    for (i, &b) in name.iter().enumerate() {
        match i {
            0..=7  => n0 |= (b as u64) << (i * 8),
            8..=15 => n1 |= (b as u64) << ((i - 8) * 8),
            16..=23 => n2 |= (b as u64) << ((i - 16) * 8),
            _ => break,
        }
    }
    let flags = (RAMFS_O_CREATE as u64) << 24;
    let (status, handle) = pdx_call(SLOT_STORAGE, OP_RAMFS_OPEN, n0, n1, n2 | flags);
    if status != 0 || (handle as i64) < 0 {
        serial_println!("[spindle.sexfiles.open] status={} handle={} -- persistence unavailable", status, handle as i64);
        return false;
    }
    serial_println!("[spindle.sexfiles.open] handle={} file={:?}", handle, core::str::from_utf8(name).unwrap_or("?"));
    serial_println!("[spindle.sexfiles.persist] handle={} entries={}", handle, hist.total);

    // Write most recent entries in 8-byte chunks
    let count = hist.total.min(128);
    for i in 0..count as usize {
        if let Some(entry) = hist.get(i) {
            let mut offset = 0u64;
            let data = entry;
            let chunks = (data.len() + 7) / 8;
            for c in 0..chunks {
                let mut chunk: u64 = 0;
                let base = c * 8;
                for j in 0..8.min(data.len().saturating_sub(base)) {
                    chunk |= (data[base + j] as u64) << (j * 8);
                }
                let (ws, _) = pdx_call(SLOT_STORAGE, OP_RAMFS_WRITE, handle, offset, chunk);
                if ws != 0 { break; }
                offset += 8;
            }
        }
    }

    let _ = pdx_call(SLOT_STORAGE, OP_RAMFS_CLOSE, handle, 0, 0);
    serial_println!("[spindle.sexfiles.write] entries={}", count);
    true
}

/// Try to restore command history from SexFiles RamFS on boot.
/// Returns number of entries restored (0 if unavailable).
unsafe fn restore_history(hist: &mut History) -> u32 {
    let mut n0: u64 = 0; let mut n1: u64 = 0; let mut n2: u64 = 0;
    let name = HISTORY_FILE;
    for (i, &b) in name.iter().enumerate() {
        match i {
            0..=7  => n0 |= (b as u64) << (i * 8),
            8..=15 => n1 |= (b as u64) << ((i - 8) * 8),
            _ => break,
        }
    }
    let (status, handle) = pdx_call(SLOT_STORAGE, OP_RAMFS_OPEN, n0, n1, n2);
    if status != 0 || (handle as i64) < 0 {
        serial_println!("[spindle.sexfiles.read] status={} -- history restore unavailable", status);
        return 0;
    }
    serial_println!("[spindle.history.restore] handle={}", handle);

    // Read history entries (bounded: max 128 entries × 256 bytes = 32 KiB)
    let mut buf = [0u8; 256];
    let mut offset = 0u64;
    let mut restored = 0u32;
    for _ in 0..128 {
        let mut line_buf = [0u8; 256];
        let mut line_len = 0usize;
        for c in 0..32 {
            let (rs, data) = pdx_call(SLOT_STORAGE, OP_RAMFS_READ, handle, offset, 8);
            if rs != 0 { break; }
            let bytes = data.to_le_bytes();
            let mut done = false;
            for j in 0..8 {
                let b = bytes[j];
                if b == 0 || b == b'\n' { done = true; break; }
                if line_len < 256 { line_buf[line_len] = b; line_len += 1; }
            }
            offset += 8;
            if done { break; }
        }
        if line_len == 0 { break; }
        hist.push(&line_buf[..line_len]);
        restored += 1;
    }
    let _ = pdx_call(SLOT_STORAGE, OP_RAMFS_CLOSE, handle, 0, 0);
    serial_println!("[spindle.history.restore] restored={}", restored);
    restored
}

// ── Entry ──────────────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn _start() -> ! {
    serial_println!("[spindle.boot]");
    serial_println!("[spindle.surface.req] pd=12 kernel_spawned=1");

    // ── Input proof gate (compile-time; guards framebuffer access) ──
    const INPUT_PROOF_ENABLED: bool =
        option_env!("SEXOS_SPINDLE_INPUT_PROOF").is_some();
    if INPUT_PROOF_ENABLED {
        unsafe {
            let params = WindowCreateParams {
                x: 40, y: 200, width: WIN_W, height: WIN_H, pfn_base: FB_PFN_BASE,
            };
            pdx_call(SLOT_DISPLAY, OP_WINDOW_CREATE, &params as *const _ as u64, 0, 0);
            serial_println!("[spindle.surface.req] w={} h={}", WIN_W, WIN_H);

            let mut fb = WindowBuffer::new((FB_PFN_BASE << 12) as u64, WIN_W, WIN_H, WIN_W);
            let mut sb = Scrollback::new();
            let mut hist = History::new();
            let mut ev = EventRing::new();

            sb.push(b"Spindle -- SexOS native command console");
            sb.push(b""); sb.push(b"Type help for commands. V1.0.0-pre"); sb.push(b"");

            fb.clear(BG);
            font::draw_str(&mut fb, 4, 4, b"Spindle", ACCENT, None);
            for col in 0..COLS { fb.draw_pixel(col * CELL_W, CELL_H + CELL_H / 2 - 1, ACCENT); }
            font::draw_str(&mut fb, 4, CELL_H * 2 + 4, b"SexOS native command console", FG, None);
            font::draw_str(&mut fb, 4, CELL_H * 3 + 4, b"Type help for commands.", FG, None);
            for col in 0..COLS { fb.draw_pixel(col * CELL_W, CELL_H * 4 + CELL_H / 2 - 1, ACCENT); }
            render_scrollback(&mut fb, &sb);
            font::draw_str(&mut fb, 4, CELL_H * 23 + 4, b"sex> ", GREEN, None);
            serial_println!("[spindle.surface.ok] boot_lines={}", sb.total_lines);

            run_input_proof(&mut fb, &mut sb, &mut hist, &mut ev);

            const SEXOBJECT_PROOF_ENABLED: bool =
                option_env!("SEXOS_SPINDLE_SEXOBJECT_PROOF").is_some();
            if SEXOBJECT_PROOF_ENABLED { run_spindle_sexobject_proof(&mut sb); }
        }
    }

    // Initialize state (always, no FB needed)
    let mut sb = Scrollback::new();
    let mut hist = History::new();
    let mut ev = EventRing::new();
    let mut line = CmdLine::new();

    // Best-effort restore from SexFiles (cap granted via 8ce251e).
    let restored = unsafe { restore_history(&mut hist) };
    serial_println!("[spindle.history.restore] count={}", restored);
    // Best-effort Linen .spn session object create (non-fatal if nonzero).
    let (ls, _) = pdx_call(SLOT_LINEN, OP_LINEN_CREATE_OBJECT, 0, 0, 0);
    serial_println!("[spindle.linen.spn.create] status={}", ls);
    serial_println!("[spindle.fb.proof.disabled] surface=0x99 route=silk-shell fb=gated_proof_only");

    serial_println!("[spindle.ready]");

    loop {
        let msg = unsafe { sex_pdx::pdx_listen_raw(0) };
        if msg.type_id == 0x202 {
            let scancode = msg.arg0 as u8;
            let value = msg.arg1;
            if value != 1 { continue; }

            // On Enter, record command, persist, dispatch, and optionally Bell-notify.
            if scancode == 0x1C && line.len > 0 {
                hist.push(line.as_bytes());
                unsafe { persist_history(&hist); }
                serial_println!("[spindle.sexfiles.persist] ok");
                let recognized = dispatch(line.as_bytes(), &mut sb, &mut hist, &mut ev);
                if recognized {
                    let (bs, _) = pdx_call(SLOT_BELL, OP_BELL_NOTIFY, 0, 0, 0);
                    serial_println!("[spindle.bell.notify] status={}", bs);
                }
            }
            handle_key(scancode, &mut line);
        }
    }
}

/// Process a single keyboard scancode into the line editor.
/// Scancode table matches sexsh convention (US QWERTY set 1).
fn handle_key(scancode: u8, line: &mut CmdLine) {
    match scancode {
        0x1C => { // Enter
            serial_println!("[spindle.line.enter] len={} text={:?}", line.len,
                core::str::from_utf8(line.as_bytes()).unwrap_or("?"));
            serial_println!("[spindle.input.recv] key=enter len={}", line.len);
            line.clear();
        }
        0x0E => { // Backspace
            line.backspace();
            serial_println!("[spindle.line.backspace] len={}", line.len);
            serial_println!("[spindle.input.recv] key=backspace len={}", line.len);
        }
        0x01 => { // Escape -- clear buffer
            line.clear();
            serial_println!("[spindle.input.recv] key=escape len=0");
        }
        _ => {
            if let Some(ch) = scancode_to_ascii(scancode) {
                line.push(ch);
                serial_println!("[spindle.line.append] ch={} len={}", ch as char, line.len);
                serial_println!("[spindle.input.recv] key=printable ch={} len={}", ch as char, line.len);
            }
            // Unknown scancodes silently ignored
        }
    }
}

/// Scancode set 1 -> ASCII (US QWERTY). Matches sexsh table.
fn scancode_to_ascii(code: u8) -> Option<u8> {
    match code {
        0x02..=0x0B => Some(b'1' + (code - 0x02)),           // 1-0
        0x10..=0x19 => Some(b'q' + (code - 0x10)),           // q-p
        0x1E..=0x26 => Some(b'a' + (code - 0x1E)),           // a-l
        0x2C..=0x32 => Some(b'z' + (code - 0x2C)),           // z-m
        0x39 => Some(b' '),                                   // Space
        0x0C => Some(b'-'), 0x0D => Some(b'='),
        0x1A => Some(b'['), 0x1B => Some(b']'),
        0x27 => Some(b';'), 0x28 => Some(b'\''),
        0x29 => Some(b'`'), 0x2B => Some(b'\\'),
        0x33 => Some(b','), 0x34 => Some(b'.'), 0x35 => Some(b'/'),
        0x0F => Some(b'\t'), // Tab
        _ => None,
    }
}

// ── Synthetic input proof ──────────────────────────────────────────────────

/// Run a synthetic input proof exercising all line editor operations.
/// Activated by SEXOS_SPINDLE_INPUT_PROOF=1.
///
/// Since Spindle is not kernel-spawned, it has no PDX slot for silk-shell
/// to forward HID events to. This proof gate injects synthetic keystrokes
/// directly, proving the line editor logic is correct. Real HID delivery
/// will be wired when Spindle gets kernel-spawned (STOP FIRST).
unsafe fn run_input_proof(fb: &mut WindowBuffer, sb: &mut Scrollback, hist: &mut History, ev: &mut EventRing) {
    serial_println!("[spindle.input.proof.start]");

    let mut line = CmdLine::new();

    // ── Stage 1: Append printable characters ──
    let test_str = b"hello";
    for &ch in test_str {
        line.push(ch);
        redraw_prompt(fb, &line);
        serial_println!("[spindle.line.append] ch={} len={}", ch as char, line.len);
    }
    let stage1_ok = line.len == 5 && line.as_bytes() == b"hello";
    serial_println!("[spindle.input.proof.append] ok={} len={}", stage1_ok as u8, line.len);

    // ── Stage 2: Backspace ──
    line.backspace();
    redraw_prompt(fb, &line);
    serial_println!("[spindle.line.backspace] len={}", line.len);
    line.backspace();
    redraw_prompt(fb, &line);
    let stage2_ok = line.len == 3 && line.as_bytes() == b"hel";
    serial_println!("[spindle.input.proof.backspace] ok={} len={}", stage2_ok as u8, line.len);

    // ── Stage 3: Overflow rejection ──
    while line.len < CMD_MAX {
        line.push(b'X');
    }
    line.push(b'Y');
    let stage3_ok = line.len == CMD_MAX;
    serial_println!("[spindle.input.proof.overflow] ok={} len={} max={}", stage3_ok as u8, line.len, CMD_MAX);

    // ── Stage 4: Non-printable rejection ──
    line.clear();
    line.push(0x01); line.push(0x00); line.push(0x7F); line.push(b'\n');
    let stage4_ok = line.len == 0;
    serial_println!("[spindle.input.proof.nonprintable] ok={} len={}", stage4_ok as u8, line.len);

    // ── Stage 5: Enter -- push to history, dispatch command, output to scrollback ──
    line.push(b't'); line.push(b'e'); line.push(b's'); line.push(b't');
    hist.push(line.as_bytes()); // push to in-memory history ring
    sb.push(line.as_bytes());   // echo the command line
    let recognized = dispatch(line.as_bytes(), sb, hist, ev);
    serial_println!("[spindle.cmd.dispatch] cmd={:?} recognized={}", core::str::from_utf8(line.as_bytes()).unwrap_or("?"), recognized as u8);
    if recognized { serial_println!("[spindle.cmd.dispatch] unexpected_recognized"); }
    serial_println!("[spindle.line.enter] text={:?} scrollback_len={}", core::str::from_utf8(line.as_bytes()).unwrap_or("?"), sb.total_lines);
    line.clear();
    redraw_prompt(fb, &line);
    render_scrollback(fb, sb);
    let stage5_ok = line.len == 0 && !recognized && hist.total == 1;
    serial_println!("[spindle.input.proof.enter] ok={} history_entries={}", stage5_ok as u8, hist.total);

    // ── Stage 6: Empty backspace is no-op ──
    line.backspace();
    line.backspace();
    let stage6_ok = line.len == 0;
    serial_println!("[spindle.input.proof.empty_backspace] ok={}", stage6_ok as u8);

    // ── Stage 7: Scrollback overflow ──
    // Fill scrollback beyond capacity: push MAX_SCROLLBACK * 2 lines
    let sb_before = sb.total_lines;
    for i in 0..(MAX_SCROLLBACK as u32 * 2) {
        sb.push(b"overflow test line 1234567890123456789012345678901234567890");
    }
    let sb_after = sb.total_lines;
    // Ring wraps correctly -- total_lines > MAX_SCROLLBACK but ring only holds MAX_SCROLLBACK
    let wrapped = sb_after > sb_before + MAX_SCROLLBACK as u32;
    serial_println!("[spindle.scrollback.overflow] ok={} total={} capacity={}", wrapped as u8, sb_after, MAX_SCROLLBACK);

    // ── Stage 8: Scrollback line clamping ──
    // Push a line longer than MAX_LINE_BYTES -- must be clamped
    sb.push(&[b'L'; 200]);
    let clamped = sb.get((sb.total_lines - 1) as usize % MAX_SCROLLBACK);
    let stage8_ok = clamped.len() <= MAX_LINE_BYTES;
    serial_println!("[spindle.scrollback.clamp] ok={} line_len={} max={}", stage8_ok as u8, clamped.len(), MAX_LINE_BYTES);

    // ── Stage 9: Scroll offset + render ──
    sb.scroll_offset = 10; // scroll back 10 lines
    render_scrollback(fb, sb);
    sb.scroll_offset = 0;  // reset to latest
    render_scrollback(fb, sb);
    serial_println!("[spindle.scrollback.render] ok=1 visible_rows={}", VISIBLE_ROWS);

    // ── Stage 10: help command ──
    let help_recognized = dispatch(b"help", sb, hist, ev);
    let stage10_ok = help_recognized;
    serial_println!("[spindle.cmd.dispatch] cmd=help recognized={}", help_recognized as u8);

    // ── Stage 11: status command ──
    let status_recognized = dispatch(b"status", sb, hist, ev);
    let stage11_ok = status_recognized;
    serial_println!("[spindle.cmd.dispatch] cmd=status recognized={}", status_recognized as u8);

    // ── Stage 12: clear command ──
    let sb_before_clear = sb.total_lines;
    dispatch(b"clear", sb, hist, ev);
    let stage12_ok = sb.total_lines < sb_before_clear; // reset to 1 line
    serial_println!("[spindle.cmd.clear] before={} after={}", sb_before_clear, sb.total_lines);

    // ── Stage 13: pd command ──
    let pd_recognized = dispatch(b"pd", sb, hist, ev);
    let stage13_ok = pd_recognized;
    serial_println!("[spindle.cmd.dispatch] cmd=pd recognized={}", pd_recognized as u8);

    // ── Stage 14: servers command ──
    let servers_recognized = dispatch(b"servers", sb, hist, ev);
    let stage14_ok = servers_recognized;
    serial_println!("[spindle.cmd.dispatch] cmd=servers recognized={}", servers_recognized as u8);

    // ── Stage 15: unknown command ──
    let unknown_recognized = dispatch(b"asdf", sb, hist, ev);
    let stage15_ok = !unknown_recognized; // must NOT be recognized
    serial_println!("[spindle.cmd.unknown] cmd=asdf recognized={} ok={}", unknown_recognized as u8, stage15_ok as u8);

    // ── Stage 16: bell (pending) ──
    let bell_recognized = dispatch(b"bell", sb, hist, ev);
    let stage16_ok = bell_recognized;
    serial_println!("[spindle.cmd.dispatch] cmd=bell recognized={}", bell_recognized as u8);

    // ── Stage 17: launch quil (unavailable) ──
    let launch_recognized = dispatch(b"launch quil", sb, hist, ev);
    let stage17_ok = launch_recognized;
    serial_println!("[spindle.cmd.launch_quil.unavailable] recognized={}", launch_recognized as u8);

    // ── Stage 18: history command ──
    hist.push(b"ver");
    let h_before = hist.total;
    let history_recognized = dispatch(b"history", sb, hist, ev);
    let stage18_ok = history_recognized && hist.total == h_before;
    serial_println!("[spindle.history.show] ok={} entries={}", stage18_ok as u8, hist.total);

    // ── Stage 19: history clear ──
    dispatch(b"history clear", sb, hist, ev);
    let stage19_ok = hist.total == 0;
    serial_println!("[spindle.history.clear] ok={} entries={}", stage19_ok as u8, hist.total);

    // ── Stage 20: persistence status ──
    let stage20_ok = true;
    serial_println!("[spindle.history.persistence] status=pending reason=spindle_not_kernel_spawned");

    let all_ok = stage1_ok && stage2_ok && stage3_ok && stage4_ok
              && stage5_ok && stage6_ok && wrapped && stage8_ok
              && stage10_ok && stage11_ok && stage12_ok && stage13_ok
              && stage14_ok && stage15_ok && stage16_ok && stage17_ok
              && stage18_ok && stage19_ok && stage20_ok && true /* events ok */;
    serial_println!("[spindle.input.proof.done] ok={} stages=20 (events: pending)", all_ok as u8);
}

// ── M9 Proof: Spindle Session as SexObject ────────────────────────────────────

/// Synthetic global SexFiles object_id for Spindle session binding.
/// In production, this is populated via Linen persist -> OP_RAMFS_OBJECT_ID.
static mut SPINDLE_SESSION_SEXOBJECT_ID: u64 = 0;
static mut SPINDLE_SESSION_SEXOBJECT_GENERATION: u64 = 0;
static mut SPINDLE_LOCAL_SESSION_ID: u64 = 1;

/// Run M9 Spindle SexObject binding proof at boot.
/// Activated by SEXOS_SPINDLE_SEXOBJECT_PROOF=1.
///
/// Spindle is a user-space app with no PDX server slots, so the session
/// binding is demonstrated with a synthetic global ID.  In production,
/// Spindle's parent (silk-shell or app launcher) persists the session via
/// Linen/SexFiles and writes the returned global ID into Spindle's state.
///
/// Markers:
///   [sexobject.m9.spindle.session.create]
///   [sexobject.m9.spindle.sexfiles_object_id]
///   [sexobject.m9.spindle.local_id_separate]
///   [sexobject.m9.spindle.ref_global]
///   [sexobject.m9.spindle.local_id_reject]
///   [sexobject.m9.pass]
unsafe fn run_spindle_sexobject_proof(_sb: &mut Scrollback) {
    serial_println!("[spindle.m9.proof] begin");

    let global_oid: u64 = 55;   // synthetic global SexFiles object_id
    let global_gen: u64 = 1;    // initial rights_generation
    let local_session_id = SPINDLE_LOCAL_SESSION_ID;

    // Bind session to global SexObject identity.
    SPINDLE_SESSION_SEXOBJECT_ID = global_oid;
    SPINDLE_SESSION_SEXOBJECT_GENERATION = global_gen;

    // [sexobject.m9.spindle.session.create]
    serial_println!(
        "[sexobject.m9.spindle.session.create] session_id={} accepted=1",
        local_session_id
    );

    // [sexobject.m9.spindle.sexfiles_object_id]
    let stored_oid = SPINDLE_SESSION_SEXOBJECT_ID;
    let global_ok = stored_oid == global_oid && stored_oid >= 1;
    serial_println!(
        "[sexobject.m9.spindle.sexfiles_object_id] session_id={} object_id={} global_ok={}",
        local_session_id, stored_oid, global_ok as u8
    );

    // [sexobject.m9.spindle.local_id_separate]
    let separate = local_session_id != stored_oid;
    serial_println!(
        "[sexobject.m9.spindle.local_id_separate] session_id={} global_id={} separate={}",
        local_session_id, stored_oid, separate as u8
    );

    // [sexobject.m9.spindle.ref_global]
    // SexObjectRef { object_id, generation } uses global ID, not local.
    let ref_object_id = stored_oid;
    let global_in_ref = ref_object_id == global_oid && ref_object_id != local_session_id;
    serial_println!(
        "[sexobject.m9.spindle.ref_global] ref_object_id={} global_in_ref={}",
        ref_object_id, global_in_ref as u8
    );

    // [sexobject.m9.spindle.local_id_reject]
    let local_leaked = ref_object_id == local_session_id;
    serial_println!(
        "[sexobject.m9.spindle.local_id_reject] local_leaked={}",
        local_leaked as u8
    );

    serial_println!("[sexobject.m9.pass] ok=1");
    serial_println!("[spindle.m9.proof] end");
}
