//! linen — Silk Desktop Environment dual-pane file manager.
//!
//! Architecture: cosmic-files ported to SexOS. Surface = SexCompositor PDX
//! zero-copy frames. No iced/winit/Electron. Pure Rust, no_std.
//!
//! Layout:
//!   ┌──────────────────────────────────────────────────┐
//!   │  linen                              [path active] │  ← title (32px)
//!   ├──────────────┬───────────────────────────────────┤
//!   │ /left/path/  │ /right/path/                      │  ← pane headers (24px)
//!   ├──────────────┼───────────────────────────────────┤
//!   │ .. (parent)  │ .. (parent)                       │
//!   │ [D] dir      │ [D] dir                           │  ← file list
//!   │ [F] file 1KB │ [F] file 2KB                      │
//!   ├──────────────┴───────────────────────────────────┤
//!   │ F5:Copy  F6:Move  F8:Del  F9:Rename  Tab:Switch  │  ← hints (24px)
//!   └──────────────────────────────────────────────────┘

#![no_std]
#![no_main]

extern crate alloc;

use alloc::vec::Vec;
use core::sync::atomic::{AtomicUsize, Ordering};
use sex_pdx::{
    pdx_call, pdx_listen,
    PDX_SEX_WINDOW_CREATE, PDX_ALLOCATE_MEMORY, PDX_MAP_MEMORY,
    PDX_WINDOW_COMMIT_FRAME, PDX_GET_DISPLAY_INFO,
    LINEN_READDIR, LINEN_COPY, LINEN_MOVE, LINEN_DELETE, LINEN_MKDIR,
    SEXFILES_PD, LinenDirEntry, SexWindowCreateParams, Rect,
};
use sex_graphics::{WindowBuffer, font};

// ─── Bump allocator ───────────────────────────────────────────────────────────
const HEAP_START: usize = 0x6000_0000;
const HEAP_END:   usize = HEAP_START + 192 * 1024 * 1024;
static HEAP_TOP: AtomicUsize = AtomicUsize::new(HEAP_START);
struct Bump;
unsafe impl core::alloc::GlobalAlloc for Bump {
    unsafe fn alloc(&self, l: core::alloc::Layout) -> *mut u8 {
        let mut c = HEAP_TOP.load(Ordering::Relaxed);
        loop {
            let a = (c + l.align() - 1) & !(l.align() - 1);
            let n = a + l.size();
            if n > HEAP_END { return core::ptr::null_mut(); }
            match HEAP_TOP.compare_exchange_weak(c, n, Ordering::SeqCst, Ordering::Relaxed) {
                Ok(_) => return a as *mut u8,
                Err(x) => c = x,
            }
        }
    }
    unsafe fn dealloc(&self, _: *mut u8, _: core::alloc::Layout) {}
}
#[global_allocator]
static ALLOC: Bump = Bump;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! { loop {} }

// ─── Colours ─────────────────────────────────────────────────────────────────
const BG:       u32 = 0xFF1E1E2E;
const MANTLE:   u32 = 0xFF181825;
const SURFACE0: u32 = 0xFF313244;
const SURFACE1: u32 = 0xFF45475A;
const OVERLAY0: u32 = 0xFF6C7086;
const TEXT:     u32 = 0xFFCDD6F4;
const SUBTEXT:  u32 = 0xFFA6ADC8;
const ACCENT:   u32 = 0xFF89B4FA;
const GREEN:    u32 = 0xFFA6E3A1;
const YELLOW:   u32 = 0xFFF9E2AF;
const RED:      u32 = 0xFFF38BA8;
const SEPARATOR: u32 = 0xFF585B70;

// ─── Layout constants ─────────────────────────────────────────────────────────
const WIN_W:   u32 = 1000;
const WIN_H:   u32 = 700;
const TITLE_H: u32 = 32;
const PHDR_H:  u32 = 24;  // pane path header height
const HINT_H:  u32 = 24;
const ROW_H:   u32 = 16;
const PANE_Y:  u32 = TITLE_H + PHDR_H;
const PANE_H:  u32 = WIN_H - PANE_Y - HINT_H;
const PANE_W:  u32 = WIN_W / 2;
const ROWS:    usize = (PANE_H / ROW_H) as usize;

const SEXDISPLAY_PD: u32 = 1;

// ─── Entry buffer for VFS calls ───────────────────────────────────────────────
const MAX_ENTRIES: usize = 512;

// ─── VFS mock data (fallback when sexfiles not yet running) ──────────────────

fn mock_entries(path: &[u8]) -> Vec<LinenDirEntry> {
    let mut v = Vec::new();

    fn dir(name: &[u8]) -> LinenDirEntry {
        let mut e = LinenDirEntry::default();
        let n = name.len().min(63);
        e.name[..n].copy_from_slice(&name[..n]);
        e.name_len = n as u32;
        e.flags = 1; // is_dir
        e
    }
    fn file(name: &[u8], size: u64) -> LinenDirEntry {
        let mut e = LinenDirEntry::default();
        let n = name.len().min(63);
        e.name[..n].copy_from_slice(&name[..n]);
        e.name_len = n as u32;
        e.flags = 0;
        e.size = size;
        e
    }

    // parent entry
    v.push(dir(b".."));

    // path-dependent mock structure
    if path == b"/home/user/" || path == b"/" {
        v.push(dir(b"documents"));
        v.push(dir(b"downloads"));
        v.push(dir(b"music"));
        v.push(dir(b"pictures"));
        v.push(dir(b"videos"));
        v.push(dir(b".config"));
        v.push(dir(b".local"));
        v.push(file(b"readme.txt",  1024));
        v.push(file(b".bashrc",      256));
        v.push(file(b".profile",     128));
    } else if path_contains(path, b"document") {
        v.push(file(b"report_q4.pdf",   2_097_152));
        v.push(file(b"notes.md",           14_336));
        v.push(file(b"todo.txt",            2_048));
        v.push(dir(b"archive"));
        v.push(dir(b"projects"));
    } else if path_contains(path, b"download") {
        v.push(file(b"sexos-latest.iso",   734_003_200));
        v.push(file(b"rustup.sh",              32_768));
        v.push(dir(b"packages"));
    } else if path_contains(path, b"picture") {
        v.push(file(b"wallpaper.png",   3_145_728));
        v.push(file(b"screenshot.png",    819_200));
        v.push(dir(b"2024"));
        v.push(dir(b"2025"));
    } else {
        v.push(file(b"(empty)", 0));
    }
    v
}

fn path_contains(path: &[u8], needle: &[u8]) -> bool {
    if path.len() < needle.len() { return false; }
    for i in 0..=path.len() - needle.len() {
        if &path[i..i + needle.len()] == needle { return true; }
    }
    false
}

// ─── Pane ─────────────────────────────────────────────────────────────────────

struct Pane {
    path:     [u8; 512],
    path_len: usize,
    entries:  Vec<LinenDirEntry>,
    selected: usize,
    scroll:   usize,
}

impl Pane {
    fn new(initial_path: &[u8]) -> Self {
        let mut p = Pane {
            path: [0; 512],
            path_len: 0,
            entries: Vec::new(),
            selected: 0,
            scroll: 0,
        };
        let n = initial_path.len().min(511);
        p.path[..n].copy_from_slice(&initial_path[..n]);
        p.path_len = n;
        p.refresh();
        p
    }

    fn refresh(&mut self) {
        self.entries.clear();
        self.selected = 0;
        self.scroll = 0;

        // Allocate output buffer for VFS entries
        let buf_size = (MAX_ENTRIES * core::mem::size_of::<LinenDirEntry>()) as u64;
        let buf_pfn = unsafe { pdx_call(0, PDX_ALLOCATE_MEMORY, buf_size, 0) };
        let buf_virt = if buf_pfn != u64::MAX {
            unsafe { pdx_call(0, PDX_MAP_MEMORY, buf_pfn, buf_size) }
        } else { u64::MAX };

        let count = if buf_virt != u64::MAX && buf_virt != 0 {
            unsafe {
                pdx_call(
                    SEXFILES_PD,
                    LINEN_READDIR,
                    self.path.as_ptr() as u64,
                    buf_virt,
                )
            }
        } else { 0 };

        if count > 0 && count <= MAX_ENTRIES as u64 && buf_virt != u64::MAX {
            // Parse entries from VFS buffer
            let entries_ptr = buf_virt as *const LinenDirEntry;
            for i in 0..count as usize {
                let e = unsafe { *entries_ptr.add(i) };
                self.entries.push(e);
            }
        } else {
            // Fallback: mock data for development
            self.entries = mock_entries(&self.path[..self.path_len]);
        }
    }

    fn selected_entry(&self) -> Option<&LinenDirEntry> {
        self.entries.get(self.selected)
    }

    fn selected_path(&self, buf: &mut [u8; 512]) -> usize {
        let base = self.path_len;
        let name_len = self.selected_entry()
            .map(|e| e.name_len as usize)
            .unwrap_or(0);
        let need = base + name_len;
        if need >= 512 { return 0; }
        buf[..base].copy_from_slice(&self.path[..base]);
        if let Some(e) = self.selected_entry() {
            let n = e.name_len as usize;
            buf[base..base + n].copy_from_slice(&e.name[..n]);
            if e.flags & 1 != 0 && buf[base + n - 1] != b'/' {
                // append slash for dirs
                if base + n + 1 < 512 { buf[base + n] = b'/'; return base + n + 1; }
            }
            base + n
        } else { base }
    }

    fn navigate_into(&mut self) {
        if let Some(e) = self.selected_entry() {
            let name = e.name;
            let nlen = e.name_len as usize;
            let is_dir = e.flags & 1 != 0;

            if &name[..nlen.min(2)] == b".." {
                self.navigate_up();
                return;
            }
            if !is_dir { return; }

            // Build new path = current_path + name + "/"
            let new_len = self.path_len + nlen + 1;
            if new_len >= 512 { return; }
            let mut new_path = [0u8; 512];
            new_path[..self.path_len].copy_from_slice(&self.path[..self.path_len]);
            new_path[self.path_len..self.path_len + nlen].copy_from_slice(&name[..nlen]);
            new_path[self.path_len + nlen] = b'/';
            self.path = new_path;
            self.path_len = new_len;
            self.refresh();
        }
    }

    fn navigate_up(&mut self) {
        // Strip last path component
        let p = &self.path[..self.path_len];
        // Find second-to-last '/'
        let trimmed = if p.ends_with(b"/") && p.len() > 1 { &p[..p.len()-1] } else { p };
        if let Some(pos) = trimmed.iter().rposition(|&b| b == b'/') {
            self.path_len = pos + 1;
            self.refresh();
        }
    }

    fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            if self.selected < self.scroll { self.scroll = self.selected; }
        }
    }

    fn move_down(&mut self) {
        if self.selected + 1 < self.entries.len() {
            self.selected += 1;
            if self.selected >= self.scroll + ROWS {
                self.scroll = self.selected - ROWS + 1;
            }
        }
    }

    fn path_str(&self) -> &[u8] { &self.path[..self.path_len] }
}

// ─── Size formatter ───────────────────────────────────────────────────────────
fn fmt_size(size: u64, buf: &mut [u8; 10]) -> usize {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;

    let (val, unit): (u64, &[u8]) = if size >= GB {
        (size / GB, b"GB")
    } else if size >= MB {
        (size / MB, b"MB")
    } else if size >= KB {
        (size / KB, b"KB")
    } else {
        (size, b"B ")
    };

    // write val (up to 4 digits)
    let mut tmp = [0u8; 8];
    let mut n = 0usize;
    let mut v = val;
    if v == 0 { tmp[0] = b'0'; n = 1; } else {
        while v > 0 { tmp[n] = b'0' + (v % 10) as u8; n += 1; v /= 10; }
        tmp[..n].reverse();
    }
    let total = n + unit.len();
    buf[..n].copy_from_slice(&tmp[..n]);
    buf[n..n + unit.len()].copy_from_slice(unit);
    total
}

// ─── Main app ─────────────────────────────────────────────────────────────────

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side { Left, Right }

#[derive(Clone, Copy, PartialEq, Eq)]
enum Modal { None, DeleteConfirm, Rename, Mkdir }

struct Linen {
    buf:       WindowBuffer,
    pfn_base:  u64,
    window_id: u64,
    left:      Pane,
    right:     Pane,
    active:    Side,
    modal:     Modal,
    // text input buffer for rename/mkdir
    input:     [u8; 128],
    input_len: usize,
    dirty:     bool,
}

impl Linen {
    fn new() -> Option<Self> {
        let info = unsafe { pdx_call(SEXDISPLAY_PD, PDX_GET_DISPLAY_INFO, 0, 0) };
        let sw = (info >> 32) as u32;
        let sh = info as u32;

        let sz   = (WIN_W * WIN_H * 4) as u64;
        let pfn  = unsafe { pdx_call(0, PDX_ALLOCATE_MEMORY, sz, 0) };
        if pfn == u64::MAX { return None; }
        let virt = unsafe { pdx_call(0, PDX_MAP_MEMORY, pfn, sz) };
        if virt == u64::MAX { return None; }

        // Centre window on screen (or top-left if screen info unavailable)
        let wx = if sw > WIN_W { (sw - WIN_W) / 2 } else { 0 } as i32;
        let wy = if sh > WIN_H { (sh - WIN_H) / 2 } else { 40 } as i32;

        let params = SexWindowCreateParams { x: wx, y: wy, width: WIN_W, height: WIN_H, pfn_base: pfn };
        let wid = unsafe {
            pdx_call(SEXDISPLAY_PD, PDX_SEX_WINDOW_CREATE, &params as *const _ as u64, 0)
        };
        if wid == 0 { return None; }

        Some(Linen {
            buf: unsafe { WindowBuffer::new(virt, WIN_W, WIN_H, WIN_W) },
            pfn_base: pfn,
            window_id: wid,
            left:  Pane::new(b"/home/user/"),
            right: Pane::new(b"/home/user/documents/"),
            active: Side::Left,
            modal: Modal::None,
            input: [0; 128],
            input_len: 0,
            dirty: true,
        })
    }

    fn active_pane(&self)     -> &Pane     { if self.active == Side::Left { &self.left } else { &self.right } }
    fn active_pane_mut(&mut self) -> &mut Pane { if self.active == Side::Left { &mut self.left } else { &mut self.right } }
    fn inactive_pane(&self)   -> &Pane     { if self.active == Side::Left { &self.right } else { &self.left } }

    // ── Draw ─────────────────────────────────────────────────────────────────

    fn draw(&mut self) {
        if !self.dirty { return; }
        self.dirty = false;

        unsafe { self.buf.clear(BG); }

        self.draw_titlebar();
        self.draw_pane_headers();
        self.draw_pane_separator();
        self.draw_pane(Side::Left);
        self.draw_pane(Side::Right);
        self.draw_hint_bar();
        if self.modal != Modal::None {
            self.draw_modal();
        }
        self.commit();
    }

    fn draw_titlebar(&mut self) {
        unsafe { self.buf.draw_rect(Rect { x: 0, y: 0, w: WIN_W, h: TITLE_H }, MANTLE); }
        font::draw_str(&mut self.buf, 16, 12, b"linen", TEXT, None);
        // Show active path in header
        let active_path = if self.active == Side::Left {
            &self.left.path[..self.left.path_len]
        } else {
            &self.right.path[..self.right.path_len]
        };
        let max = ((WIN_W - 120) / font::CHAR_W) as usize;
        let show = active_path.len().min(max);
        font::draw_str(&mut self.buf, WIN_W / 2 - (show as u32 * font::CHAR_W) / 2, 12, &active_path[..show], SUBTEXT, None);
    }

    fn draw_pane_headers(&mut self) {
        let y = TITLE_H as i32;
        unsafe { self.buf.draw_rect(Rect { x: 0, y, w: WIN_W, h: PHDR_H }, SURFACE0); }

        // Left path
        let lactive = self.active == Side::Left;
        let lpath = &self.left.path[..self.left.path_len];
        let lmax = ((PANE_W - 16) / font::CHAR_W) as usize;
        let lshow = lpath.len().min(lmax);
        let lcol = if lactive { ACCENT } else { SUBTEXT };
        font::draw_str(&mut self.buf, 8, TITLE_H + 8, &lpath[..lshow], lcol, None);

        // Right path
        let rpath = &self.right.path[..self.right.path_len];
        let rmax = ((PANE_W - 16) / font::CHAR_W) as usize;
        let rshow = rpath.len().min(rmax);
        let rcol = if !lactive { ACCENT } else { SUBTEXT };
        font::draw_str(&mut self.buf, PANE_W + 8, TITLE_H + 8, &rpath[..rshow], rcol, None);
    }

    fn draw_pane_separator(&mut self) {
        unsafe {
            self.buf.draw_rect(Rect { x: PANE_W as i32, y: TITLE_H as i32, w: 1, h: WIN_H - TITLE_H - HINT_H }, SEPARATOR);
        }
    }

    fn draw_pane(&mut self, side: Side) {
        let x_off = if side == Side::Left { 0u32 } else { PANE_W + 1 };
        let is_active = side == self.active;

        // pane background
        let bg = if is_active { BG } else { MANTLE };
        unsafe { self.buf.draw_rect(Rect { x: x_off as i32, y: PANE_Y as i32, w: PANE_W - 1, h: PANE_H }, bg); }

        let pane = if side == Side::Left { &self.left } else { &self.right };
        let visible = ROWS.min(pane.entries.len().saturating_sub(pane.scroll));

        for rel in 0..visible {
            let idx = pane.scroll + rel;
            let ey = PANE_Y + rel as u32 * ROW_H;
            let entry = &pane.entries[idx];
            let selected = idx == pane.selected && is_active;

            // row background
            if selected {
                unsafe { self.buf.draw_rect(Rect { x: x_off as i32, y: ey as i32, w: PANE_W - 1, h: ROW_H }, SURFACE1); }
            }

            // type indicator
            let (type_char, type_col) = if entry.flags & 1 != 0 {
                (b'D', ACCENT)  // directory
            } else if entry.flags & 4 != 0 {
                (b'X', GREEN)   // executable
            } else {
                (b'F', SUBTEXT) // file
            };

            // [D] or [F]
            unsafe { self.buf.draw_rect(Rect { x: (x_off + 2) as i32, y: (ey + 2) as i32, w: 12, h: 12 }, SURFACE0); }
            font::draw_char(&mut self.buf, x_off + 4, ey + 4, type_char, type_col, None);

            // name
            let nlen = (entry.name_len as usize).min(64);
            let max_name = ((PANE_W - 80) / font::CHAR_W) as usize;
            let show_name = nlen.min(max_name);
            let name_col = if selected { TEXT } else if entry.flags & 1 != 0 { ACCENT } else { TEXT };
            font::draw_str(&mut self.buf, x_off + 20, ey + 4, &entry.name[..show_name], name_col, None);

            // size (files only, right-aligned in pane)
            if entry.flags & 1 == 0 && entry.size > 0 {
                let mut sz_buf = [0u8; 10];
                let sz_len = fmt_size(entry.size, &mut sz_buf);
                let sx = x_off + PANE_W - 4 - sz_len as u32 * font::CHAR_W;
                font::draw_str(&mut self.buf, sx, ey + 4, &sz_buf[..sz_len], OVERLAY0, None);
            }
        }

        // scroll indicator
        if pane.entries.len() > ROWS {
            let track_h = PANE_H;
            let thumb_h = (track_h * ROWS as u32) / pane.entries.len() as u32;
            let thumb_y = PANE_Y + (track_h * pane.scroll as u32) / pane.entries.len() as u32;
            let sx = x_off + PANE_W - 4;
            unsafe {
                self.buf.draw_rect(Rect { x: sx as i32, y: PANE_Y as i32, w: 3, h: track_h }, SURFACE0);
                self.buf.draw_rect(Rect { x: sx as i32, y: thumb_y as i32, w: 3, h: thumb_h.max(4) }, ACCENT);
            }
        }
    }

    fn draw_hint_bar(&mut self) {
        let y = WIN_H - HINT_H;
        unsafe { self.buf.draw_rect(Rect { x: 0, y: y as i32, w: WIN_W, h: HINT_H }, MANTLE); }

        let hints: &[(&[u8], &[u8])] = &[
            (b"F5", b"Copy"),
            (b"F6", b"Move"),
            (b"F8", b"Del"),
            (b"F9", b"Rename"),
            (b"F10", b"Quit"),
            (b"Tab", b"Switch"),
            (b"Ins", b"Select"),
        ];

        let mut hx = 8u32;
        let hy = y + 6;
        for &(key, label) in hints {
            // key badge
            let kw = (key.len() as u32 + 1) * font::CHAR_W;
            unsafe { self.buf.draw_rect(Rect { x: hx as i32, y: hy as i32, w: kw, h: 12 }, SURFACE0); }
            font::draw_str(&mut self.buf, hx + 2, hy + 2, key, ACCENT, None);
            hx += kw + 2;
            // label
            font::draw_str(&mut self.buf, hx, hy + 2, label, TEXT, None);
            hx += (label.len() as u32 + 2) * font::CHAR_W;
        }
    }

    fn draw_modal(&mut self) {
        let mw = 400u32;
        let mh = 100u32;
        let mx = (WIN_W - mw) / 2;
        let my = (WIN_H - mh) / 2;

        unsafe { self.buf.draw_rect(Rect { x: mx as i32, y: my as i32, w: mw, h: mh }, SURFACE0); }
        unsafe { self.buf.draw_rect(Rect { x: mx as i32, y: my as i32, w: mw, h: 1 }, ACCENT); }
        unsafe { self.buf.draw_rect(Rect { x: mx as i32, y: (my + mh - 1) as i32, w: mw, h: 1 }, ACCENT); }

        // Extract name before any mutable borrow of self.buf
        let del_name: Option<([u8; 64], usize)> = self.active_pane()
            .selected_entry()
            .map(|e| (e.name, e.name_len as usize));

        match self.modal {
            Modal::DeleteConfirm => {
                font::draw_str(&mut self.buf, mx + 12, my + 12, b"Delete selected? [Y/N]", TEXT, None);
                if let Some((name, nlen)) = del_name {
                    font::draw_str(&mut self.buf, mx + 12, my + 32, &name[..nlen], RED, None);
                }
            }
            Modal::Rename | Modal::Mkdir => {
                let label: &[u8] = if self.modal == Modal::Rename { b"Rename:" } else { b"New folder:" };
                font::draw_str(&mut self.buf, mx + 12, my + 12, label, TEXT, None);
                // input box
                unsafe { self.buf.draw_rect(Rect { x: (mx + 12) as i32, y: (my + 32) as i32, w: mw - 24, h: 20 }, BG); }
                font::draw_str(&mut self.buf, mx + 14, my + 36, &self.input[..self.input_len], ACCENT, None);
                // cursor
                let cx = mx + 14 + self.input_len as u32 * font::CHAR_W;
                unsafe { self.buf.draw_rect(Rect { x: cx as i32, y: (my + 36) as i32, w: 2, h: font::CHAR_H }, TEXT); }
                font::draw_str(&mut self.buf, mx + 12, my + 60, b"Enter=confirm  Esc=cancel", SUBTEXT, None);
            }
            Modal::None => {}
        }
    }

    fn commit(&self) {
        unsafe { pdx_call(SEXDISPLAY_PD, PDX_WINDOW_COMMIT_FRAME, self.window_id, self.pfn_base); }
    }

    // ── Input handling ────────────────────────────────────────────────────────

    fn handle_key(&mut self, evdev_code: u16, pressed: bool) {
        if !pressed { return; }

        // Modal input handling
        if self.modal != Modal::None {
            self.handle_modal_key(evdev_code);
            return;
        }

        match evdev_code {
            103 => { self.active_pane_mut().move_up(); self.dirty = true; }         // UP
            108 => { self.active_pane_mut().move_down(); self.dirty = true; }       // DOWN
            28  => { self.active_pane_mut().navigate_into(); self.dirty = true; }   // ENTER
            14  => { self.active_pane_mut().navigate_up(); self.dirty = true; }     // BACKSPACE → up
            15  => { self.active = if self.active == Side::Left { Side::Right } else { Side::Left }; self.dirty = true; } // TAB
            63  => self.op_copy(),    // F5
            64  => self.op_move(),    // F6
            66  => { self.modal = Modal::DeleteConfirm; self.dirty = true; } // F8
            67  => { self.begin_rename(); }  // F9 (F9 = 67 in evdev)
            68  => self.quit(),       // F10
            110 => { self.begin_mkdir(); }   // F7 = 65, INSERT = 110
            _   => {}
        }
    }

    fn handle_modal_key(&mut self, code: u16) {
        match self.modal {
            Modal::DeleteConfirm => {
                match code {
                    21 => { self.op_delete_confirm(); } // Y key (evdev 21)
                    49 => { self.modal = Modal::None; self.dirty = true; } // N key (evdev 49)
                    1  => { self.modal = Modal::None; self.dirty = true; } // ESC
                    _  => {}
                }
            }
            Modal::Rename | Modal::Mkdir => {
                match code {
                    28 => { self.modal_confirm(); } // ENTER
                    1  => { self.modal = Modal::None; self.input_len = 0; self.dirty = true; } // ESC
                    14 => { // BACKSPACE
                        if self.input_len > 0 { self.input_len -= 1; self.dirty = true; }
                    }
                    _ => {} // printable chars handled via HIDEvent value
                }
            }
            Modal::None => {}
        }
    }

    fn handle_char_input(&mut self, c: u8) {
        if self.modal == Modal::None { return; }
        if c < 0x20 || c > 0x7E { return; }
        if self.input_len < 127 {
            self.input[self.input_len] = c;
            self.input_len += 1;
            self.dirty = true;
        }
    }

    // ── File operations ───────────────────────────────────────────────────────

    fn op_copy(&mut self) {
        let mut src = [0u8; 512];
        let slen = self.active_pane().selected_path(&mut src);
        if slen == 0 { return; }

        // destination = inactive pane's current dir + filename
        let dst_base = self.inactive_pane().path_len;
        let fname_start = src[..slen].iter().rposition(|&b| b == b'/').map(|p| p+1).unwrap_or(0);
        let fname_len = slen - fname_start;
        let mut dst = [0u8; 512];
        dst[..dst_base].copy_from_slice(&self.inactive_pane().path[..dst_base]);
        dst[dst_base..dst_base + fname_len].copy_from_slice(&src[fname_start..slen]);

        unsafe {
            pdx_call(SEXFILES_PD, LINEN_COPY, src.as_ptr() as u64, dst.as_ptr() as u64);
        }
        // Refresh both panes
        self.left.refresh();
        self.right.refresh();
        self.dirty = true;
    }

    fn op_move(&mut self) {
        let mut src = [0u8; 512];
        let slen = self.active_pane().selected_path(&mut src);
        if slen == 0 { return; }

        let dst_base = self.inactive_pane().path_len;
        let fname_start = src[..slen].iter().rposition(|&b| b == b'/').map(|p| p+1).unwrap_or(0);
        let fname_len = slen - fname_start;
        let mut dst = [0u8; 512];
        dst[..dst_base].copy_from_slice(&self.inactive_pane().path[..dst_base]);
        dst[dst_base..dst_base + fname_len].copy_from_slice(&src[fname_start..slen]);

        unsafe {
            pdx_call(SEXFILES_PD, LINEN_MOVE, src.as_ptr() as u64, dst.as_ptr() as u64);
        }
        self.left.refresh();
        self.right.refresh();
        self.dirty = true;
    }

    fn op_delete_confirm(&mut self) {
        let mut path = [0u8; 512];
        let plen = self.active_pane().selected_path(&mut path);
        if plen > 0 {
            unsafe { pdx_call(SEXFILES_PD, LINEN_DELETE, path.as_ptr() as u64, 0); }
            self.active_pane_mut().refresh();
        }
        self.modal = Modal::None;
        self.dirty = true;
    }

    fn begin_rename(&mut self) {
        // Extract before mutable borrow
        let entry_name: Option<([u8; 64], usize)> = self.active_pane()
            .selected_entry()
            .map(|e| (e.name, e.name_len as usize));
        if let Some((name, n)) = entry_name {
            self.input[..n].copy_from_slice(&name[..n]);
            self.input_len = n;
        }
        self.modal = Modal::Rename;
        self.dirty = true;
    }

    fn begin_mkdir(&mut self) {
        self.input_len = 0;
        self.modal = Modal::Mkdir;
        self.dirty = true;
    }

    fn modal_confirm(&mut self) {
        let mut path = [0u8; 512];
        let base = self.active_pane().path_len;
        path[..base].copy_from_slice(&self.active_pane().path[..base]);

        if self.modal == Modal::Rename {
            // src = selected path, dst = current dir + new name
            let mut src = [0u8; 512];
            let slen = self.active_pane().selected_path(&mut src);
            if slen > 0 {
                path[base..base + self.input_len].copy_from_slice(&self.input[..self.input_len]);
                unsafe { pdx_call(SEXFILES_PD, LINEN_MOVE, src.as_ptr() as u64, path.as_ptr() as u64); }
            }
        } else if self.modal == Modal::Mkdir {
            path[base..base + self.input_len].copy_from_slice(&self.input[..self.input_len]);
            unsafe { pdx_call(SEXFILES_PD, LINEN_MKDIR, path.as_ptr() as u64, 0); }
        }

        self.active_pane_mut().refresh();
        self.modal = Modal::None;
        self.input_len = 0;
        self.dirty = true;
    }

    fn quit(&self) -> ! {
        // Gracefully exit — signal kernel to terminate this PD
        unsafe { pdx_call(0, 0xFF, 0, 0); }
        loop {}
    }

    fn handle_click(&mut self, px: i32, py: i32) {
        // Determine which pane was clicked
        let side = if px < PANE_W as i32 { Side::Left } else { Side::Right };
        let x_off = if side == Side::Left { 0i32 } else { PANE_W as i32 + 1 };

        // Click within pane list area
        if py >= PANE_Y as i32 && py < (WIN_H - HINT_H) as i32 {
            self.active = side;
            let rel = (py - PANE_Y as i32) / ROW_H as i32;
            let pane = if side == Side::Left { &mut self.left } else { &mut self.right };
            let idx = pane.scroll + rel as usize;
            if idx < pane.entries.len() {
                pane.selected = idx;
            }
            self.dirty = true;
        }
    }
}

// ─── Entry point ─────────────────────────────────────────────────────────────

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let mut app = loop {
        if let Some(a) = Linen::new() { break a; }
    };

    loop {
        app.draw();
        let req = pdx_listen(1);

        match req.num {
            // HID keyboard event (from sexinput → silkbar → linen or direct)
            n if n == 0x11 => {
                let ev_type = (req.arg0 >> 16) as u16;
                let code    = (req.arg0 & 0xFFFF) as u16;
                let value   = req.arg1 as i32;
                if ev_type == 1 {
                    // EV_KEY: value=1 pressed, value=0 released, value=2 repeat
                    app.handle_key(code, value >= 1);
                    // Also handle printable char via key→ASCII mapping
                    if value >= 1 {
                        if let Some(c) = evdev_to_ascii(code) {
                            app.handle_char_input(c);
                        }
                    }
                }
            }
            // HID mouse button click
            n if n == 0x10 => {
                let ev_type = (req.arg0 >> 16) as u16;
                let value   = req.arg1 as i32;
                if ev_type == 1 && value == 1 {
                    let px = (req.arg0 & 0xFFFF) as i32;
                    let py = ((req.arg0 >> 32) & 0xFFFF) as i32;
                    app.handle_click(px, py);
                }
            }
            // Window close request from compositor
            0xFF_FF => { app.quit(); }
            _ => {}
        }
    }
}

// ─── Minimal evdev→ASCII table (lowercase, no modifiers) ─────────────────────
fn evdev_to_ascii(code: u16) -> Option<u8> {
    match code {
        2  => Some(b'1'), 3  => Some(b'2'), 4  => Some(b'3'), 5  => Some(b'4'),
        6  => Some(b'5'), 7  => Some(b'6'), 8  => Some(b'7'), 9  => Some(b'8'),
        10 => Some(b'9'), 11 => Some(b'0'), 12 => Some(b'-'), 13 => Some(b'='),
        16 => Some(b'q'), 17 => Some(b'w'), 18 => Some(b'e'), 19 => Some(b'r'),
        20 => Some(b't'), 21 => Some(b'y'), 22 => Some(b'u'), 23 => Some(b'i'),
        24 => Some(b'o'), 25 => Some(b'p'), 26 => Some(b'['), 27 => Some(b']'),
        30 => Some(b'a'), 31 => Some(b's'), 32 => Some(b'd'), 33 => Some(b'f'),
        34 => Some(b'g'), 35 => Some(b'h'), 36 => Some(b'j'), 37 => Some(b'k'),
        38 => Some(b'l'), 39 => Some(b';'), 40 => Some(b'\''),
        44 => Some(b'z'), 45 => Some(b'x'), 46 => Some(b'c'), 47 => Some(b'v'),
        48 => Some(b'b'), 49 => Some(b'n'), 50 => Some(b'm'), 51 => Some(b','),
        52 => Some(b'.'), 53 => Some(b'/'), 57 => Some(b' '),
        _ => None,
    }
}
