# SexOS Server Reference Manual

**Covers:** All ring-3 server processes in `servers/`
**Architecture:** x86_64 no_std Rust, PDX IPC, PKU isolation
**Cross-reference:** See `docs/sexos-system-manual.md` for kernel internals

---

## Table of Contents

1. [Server Architecture Overview](#1-server-architecture-overview)
2. [sexdisplay — Compositor & Display Server](#2-sexdisplay--compositor--display-server)
3. [silk-shell — Window Manager & Desktop Shell](#3-silk-shell--window-manager--desktop-shell)
4. [sexinput — Input / PS2 Driver](#4-sexinput--input--ps2-driver)
5. [sexfiles — Virtual Filesystem Server](#5-sexfiles--virtual-filesystem-server)
6. [sexdrive — NVMe / AHCI Storage Driver](#6-sexdrive--nvme--ahci-storage-driver)
7. [sexshop — Object & Package Store](#7-sexshop--object--package-store)
8. [sexnet — Network Manager](#8-sexnet--network-manager)
9. [sexnode — Cluster Node & Translator](#9-sexnode--cluster-node--translator)
10. [sex-ld — Dynamic Linker](#10-sex-ld--dynamic-linker)
11. [sexc — POSIX Syscall Server](#11-sexc--posix-syscall-server)
12. [sext — Demand Pager](#12-sext--demand-pager)
13. [sexgemini — Compiler Toolchain](#13-sexgemini--compiler-toolchain)
14. [sexstore / sexstore-gui — Legacy Object Store](#14-sexstore--sexstore-gui--legacy-object-store)
15. [tuxedo — DDE Translation Broker](#15-tuxedo--dde-translation-broker)
16. [PDX Common Patterns](#16-pdx-common-patterns)
17. [Server Dependency Map](#17-server-dependency-map)

---

## 1. Server Architecture Overview

All SexOS servers are **statically-linked no_std ELF binaries** that execute in ring 3, isolated from each other via Intel PKU. They communicate exclusively through the **PDX (Protection Domain eXchange)** IPC mechanism — ring-buffered, lock-free message passing via the kernel.

### Lifecycle

```
Kernel boot
  └→ pdx_spawn("servers/sexdisplay", elf_data)   [kernel/src/init.rs]
  └→ jump_to_userland(entry, pkru)
       └→ server _start()
            └→ pdx_listen(0)   // block waiting for messages
```

Currently only `sexdisplay` is spawned at boot (Phase 21). Other servers will be spawned on-demand or via init sequence in later phases.

### PDX Primitives (ring-3 side, from sex-rt / sex-pdx crate)

```rust
pdx_listen(slot) -> PdxEvent         // block until message arrives
pdx_call(slot, syscall_id, a0, a1)   // synchronous call to another PD
pdx_reply(caller_pd, value)          // reply to a caller
pdx_spawn_pd(path, args)             // spawn new Protection Domain
safe_pdx_register(name) -> RingPtr   // register server by name
```

### Capability Slots (Convention)

| Slot | Typical Binding |
|------|----------------|
| 0 | Self / generic listen |
| 1 | Kernel or primary service (VFS/storage) |
| 2 | Allocator (sext) or secondary service |
| 4 | Network (sexnet) |
| 5 | Display compositor (sexdisplay) |

### Zero-Copy Memory Handover

Many servers use **PKU-based zero-copy handover** (`PageHandover`):

```rust
pub struct PageHandover {
    pub pfn: u64,       // physical frame number
    pub pku_key: u8,    // PKU key granting access
}
```

The 3-step handover: `RDPKRU → WRPKRU (grant) → operation → WRPKRU (restore)`.

---

## 2. sexdisplay — Compositor & Display Server

**Path:** `servers/sexdisplay/`
**Files:** `src/main.rs`, `src/lib.rs`
**PKEY:** 1 (trusted display domain, assigned at boot)
**Phase:** Active (Phase 21+)

### Purpose

Wayland-inspired compositor for SexOS. Manages all display output, window surfaces, workspace tags, and HID event forwarding to client windows. Receives the physical framebuffer from the kernel via IPC.

### Entry Point

```rust
// servers/sexdisplay/src/main.rs
pub extern "C" fn _start() -> ! {
    let compositor = unsafe { SexCompositor::new(width, height, stride) };
    loop {
        let request = pdx_listen(0);
        if request.arg0 != 0 {
            let msg = unsafe { &*(request.arg0 as *const PdxMessage) };
            match msg.msg_type {
                MessageType::RawCall(id)       => compositor.handle_pdx_call(...)
                MessageType::HIDEvent { .. }   => forward to focused window
                _                              => log unknown
            }
            pdx_reply(request.caller_pd, reply_val);
        }
        compositor.evaluate_and_render_frame();
        for output in &compositor.outputs {
            pdx_call(0, PDX_COMPOSITOR_COMMIT, ...);
        }
    }
}
```

### SexCompositor

```rust
// servers/sexdisplay/src/lib.rs
pub struct SexCompositor {
    pub magic: u32,                              // 0x53455843 ('SEXC') — sanity check
    pub global_fb_width: u32,
    pub global_fb_height: u32,
    pub outputs: Vec<OutputState>,               // connected displays
    pub internal_framebuffers: Vec<WindowBuffer>,
    pub internal_framebuffer_pfn_bases: Vec<u64>,
    pub all_windows: [Option<Window>; 64],       // MAX_WINDOWS = 64
    pub current_tag_view_mask: u64,              // active workspace bitmask
    pub views: Vec<u64>,
    pub registered_hotkeys: Vec<Hotkey>,
    pub active_notifications: Vec<Notification>,
}
```

### PDX Syscall IDs (Inbound — handled by sexdisplay)

| Constant | Value | Description |
|----------|-------|-------------|
| `PDX_COMPOSITOR_COMMIT` | 0xDD | Commit surface pages to output |
| `PDX_SEX_WINDOW_CREATE` | 0xDE | Create new window |
| `PDX_SET_WINDOW_ROUNDNESS` | 0xDF | Set corner radius |
| `PDX_SET_WINDOW_BLUR` | 0xE0 | Set blur effect strength |
| `PDX_SET_WINDOW_ANIMATION` | 0xE1 | Toggle animation |
| `PDX_SET_WINDOW_DECORATIONS` | 0xE2 | Set border/title bar |
| `PDX_GET_DISPLAY_INFO` | 0xE3 | Query display dimensions |
| `PDX_FOCUS_WINDOW` | 0xE4 | Set keyboard focus |
| `PDX_SET_WINDOW_TAGS` | — | Assign workspace tags to window |
| `PDX_GET_WINDOW_TAGS` | — | Read window's tag mask |
| `PDX_SET_VIEW_TAGS` | — | Switch visible workspace |
| `PDX_GET_VIEW_TAGS` | — | Read current view tag mask |
| `PDX_MOVE_WINDOW` | 0xEE | Move window to (x, y) |
| `PDX_RESIZE_WINDOW` | 0xEF | Resize window to (w, h) |

### Inbound Message Types

- `MessageType::RawCall(syscall_id)` — routed through `handle_pdx_call()`
- `MessageType::HIDEvent { ev_type, code, value }` — forwarded to focused window

### Key Methods

```rust
SexCompositor::new(w, h, stride) -> Self
SexCompositor::handle_pdx_call(caller_pd, num, arg0, arg1, arg2)
SexCompositor::evaluate_and_render_frame()
```

### Framebuffer Handoff (from kernel)

The kernel calls `ship_to_sexdisplay()` (in `kernel/src/graphics/handoff.rs`) which:
1. Tags framebuffer pages with sexdisplay's PKEY via `update_page_pkey()`
2. Enqueues `MessageType::DisplayPrimaryFramebuffer { virt_addr, width, height, pitch }` into `pd.message_ring`
3. Unparks sexdisplay's trampoline task

---

## 3. silk-shell — Window Manager & Desktop Shell

**Path:** `servers/silk-shell/`
**Files:** `src/main.rs`, `main.rs`
**Phase:** Active (Phase 20+)

### Purpose

Desktop shell providing window management, workspace switching, gesture input handling, application launcher, and panel UI. Communicates with `sexdisplay` as its compositor backend.

### Entry Point

```rust
// servers/silk-shell/src/main.rs
pub extern "C" fn _start() -> ! {
    let mut state = ShellState::new();
    // Create main window, panel, launcher surfaces
    let compositor = PdxCompositorClient::new(slot=5);
    compositor.create_window(SexWindowCreateParams { x, y, w, h, pfn_base });
    compositor.set_roundness(window_id, 12);
    compositor.set_blur(window_id, 8);
    compositor.set_animation(window_id, true);
    loop {
        let event = pdx_listen(0);
        handle_input(&mut state, event);
        draw_frame(&mut state);
        compositor.commit(window_id, pfn_list, num_pages);
    }
}
```

### ShellState

```rust
pub struct ShellState {
    pub active_window_id: u32,
    pub is_dragging: bool,
    pub is_resizing: bool,
    pub drag_start_x: i32,
    pub drag_start_y: i32,
    pub current_mouse_x: i32,
    pub current_mouse_y: i32,
    pub selected_app_index: usize,
    pub current_workspace_id: u32,
    pub max_workspaces: u32,
    // gesture state
    pub current_gesture: GestureType,
    pub gesture_dx: i32,
    pub gesture_dy: i32,
}
```

### Input Handling

#### Keyboard Shortcuts

| Combo | Action |
|-------|--------|
| Alt + 1–9 | Switch to workspace/tag N |
| Shift + Alt + 1–9 | Move active window to tag N |
| Ctrl + Alt + T | Spawn `app/ionshell` terminal |

#### Mouse Gestures

| Input | Gesture | Action |
|-------|---------|--------|
| Left click + drag | `OneFingerDrag` | Move focused window |
| Right click + drag | `OneFingerDrag` | Resize focused window |
| Ctrl + drag | `TwoFingerSwipe` | Switch workspace |
| Alt + drag | `ThreeFingerSwipe` | Move window between workspaces |

### PDX Compositor Calls (Outbound to sexdisplay slot 5)

```rust
pub struct PdxCompositorClient { slot: u32 }

impl PdxCompositorClient {
    fn create_window(&self, params: SexWindowCreateParams) -> u32
    fn commit(&self, window_id: u32, pfn_list: *const u64, pages: u32)
    fn set_roundness(&self, window_id: u32, radius: u32)
    fn set_blur(&self, window_id: u32, strength: u32)
    fn set_animation(&self, window_id: u32, enabled: bool)
    fn move_window(&self, window_id: u32, x: i32, y: i32)
    fn resize_window(&self, window_id: u32, w: u32, h: u32)
    fn set_tags(&self, window_id: u32, mask: u64)
    fn set_view(&self, mask: u64)
}
```

### Launcher Apps (hardcoded)

```
app/hello_world
app/calculator
app/editor
```

### Drawing Primitives

```rust
draw_rect(fb: *mut u32, fb_w, x, y, w, h, color: u32)
draw_str(fb: *mut u32, fb_w, x, y, text: &str, color: u32)  // from sex_graphics
```

---

## 4. sexinput — Input / PS2 Driver

**Path:** `servers/sexinput/src/main.rs`
**Phase:** Active

### Purpose

PS/2 keyboard interrupt handler. Reads scancodes from hardware port `0x60` on IRQ vector `0x21`, translates to HID events, forwards to `sexdisplay` (PD ID hardcoded as 1).

### Architecture

```
IRQ 0x21 fires
  → kernel delivers MessageType::HardwareInterrupt to sexinput PD
  → sexinput reads port 0x60
  → translates scancode → HIDEvent
  → pdx_call(1, HIDEvent { ev_type=1, code=scancode, value=1 })
    → sexdisplay receives HIDEvent
      → sexdisplay forwards to focused window
```

### Event Format

```rust
MessageType::HIDEvent {
    ev_type: u16,   // 1 = EV_KEY (keyboard)
    code: u16,      // raw PS/2 scancode
    value: u16,     // 1 = keydown, 0 = keyup
}
```

### Hardware Access

- Port `0x60` — PS/2 data port (requires IOPL=3 or kernel-granted IO capability)
- Registered via `interrupts::register_irq_route(0x21, sexinput_pd_id)` in `kernel/src/init.rs`

---

## 5. sexfiles — Virtual Filesystem Server

**Path:** `servers/sexfiles/src/`
**Files:** `main.rs`, `lib.rs`, `vfs.rs`, `messages.rs`, `pdx.rs`, `trampoline.rs`, `cache.rs`, `backends/mod.rs`, `backends/ramfs.rs`, `backends/tmpfs.rs`, `backends/diskfs.rs`
**Phase:** Phase 19+ (active)

### Purpose

PDX-based zero-copy VFS with multiple filesystem backends. Provides open/read/write/close/stat/readdir and PKU-based zero-copy handover for large data transfers.

### VFS Protocol

```rust
// servers/sexfiles/src/messages.rs
pub enum VfsProtocol {
    Open     { path: u64, flags: u32 },
    Read     { fd: u32, buf_cap: u32, offset: u64, len: u64 },
    Write    { fd: u32, buf_cap: u32, offset: u64, len: u64 },
    Close    { fd: u32 },
    Stat     { path: u64 },
    Readdir  { path: u64 },
    HandoverRead  { fd: u32, pku_key: u8 },  // zero-copy read
    HandoverWrite { fd: u32, pku_key: u8 },  // zero-copy write
    Stats,
    PreWarmKeys  { keys: u64 },
    Fsync    { fd: u32 },
}
```

### Mount Table

```
/          → RamFS
/tmp       → TmpFS
/dev       → TmpFS
/disk      → DiskFS (→ sexdrive)
```

Implemented as a static 4-entry `MountTable`. Path prefix matching routes to the appropriate `FsBackend` implementation.

### FsBackend Trait

```rust
// servers/sexfiles/src/backends/mod.rs
pub trait FsBackend: Send + Sync {
    fn open(&self, path: &str, flags: u32) -> Result<u32, i64>;     // returns fd
    fn read(&self, fd: u32, buf: &mut [u8], offset: u64) -> i64;    // bytes read
    fn write(&self, fd: u32, buf: &[u8], offset: u64) -> i64;       // bytes written
    fn close(&self, fd: u32);
    fn stat(&self, path: &str) -> Option<FileStat>;
    fn readdir(&self, path: &str) -> Vec<DirEntry>;
    fn fsync(&self, fd: u32);
}
```

### Backends

#### RamFS (`backends/ramfs.rs`)
- `BTreeMap<u64, Inode>` with arena allocation
- Inode contains: `ino`, `size`, `data: Vec<u8>`, `children: Vec<u64>`
- Fully in-memory; lost on reboot

#### TmpFS (`backends/tmpfs.rs`)
- Stub implementation of `FsBackend`
- All operations return errors or empty results
- Placeholder for Phase 22

#### DiskFS (`backends/diskfs.rs`)
- Forwards all I/O to `sexdrive` via PDX
- Uses `MessageType::DmaCall { command, offset, size, buffer_cap, device_cap }`
- Awaits `MessageType::DmaReply { status, size }` from sexdrive

### Lock-Free LRU Cache (`cache.rs`)

```rust
pub struct LruCache<const N: usize = 1024> {
    fifo_q: AtomicRing<(u64, PageHandover)>,   // recently added
    lru_q:  AtomicRing<(u64, PageHandover)>,   // promoted entries
}

impl<const N: usize> LruCache<N> {
    pub fn get(&self, key: u64) -> Option<PageHandover>
    pub fn insert(&self, key: u64, val: PageHandover)
    pub fn invalidate(&self, key: u64)
}
```

### PKU Zero-Copy Handover

```rust
// Lock-free PKU grant (3-cycle RDPKRU/WRPKRU dance)
fn pku_grant_temporary(key: u8) -> u32   // saves and grants access
fn pku_restore(saved: u32)               // restores previous PKRU
```

Used for `HandoverRead`/`HandoverWrite` to give caller zero-copy access to file data pages without copying through kernel.

### Statistics (atomic)

```rust
pub static IPC_OPS_TOTAL: AtomicU64
pub static ZERO_COPY_HANDOVERS: AtomicU64
pub static CACHE_HITS: AtomicU64
pub static PKU_FLIPS: AtomicU64
```

### Trampoline Loop (`trampoline.rs`)

```rust
pub fn trampoline_main() -> ! {
    let ring = safe_pdx_register("vfs");
    loop {
        if let Some(msg) = ring.pop_front() {
            let reply = vfs::handle_vfs_message(msg);
            vfs_pdx_reply(msg.caller_pd, reply);
        }
        core::hint::spin_loop();
    }
}
```

---

## 6. sexdrive — NVMe / AHCI Storage Driver

**Path:** `servers/sexdrive/src/driver.rs`
**Phase:** Phase 18+

### Purpose

Raw NVMe block device driver. Maps PCI BAR0 via kernel PDX capability, submits NVMe commands to submission queue, handles MSI-X completions. Provides DMA read/write to callers (primarily sexfiles DiskFS backend).

### NVMe Queue Layout (in BAR0)

```
BAR0 + 0x0000  — NVMe registers base
BAR0 + 0x1008  — Submission Queue 0 Tail Doorbell
BAR0 + 0x2000  — Submission Queue entries (64 bytes each)
BAR0 + 0x3000  — Completion Queue entries
```

### Inbound Message Types

```rust
MessageType::DmaCall {
    command: u32,        // 1=READ, 2=WRITE, 3=SYNC
    offset: u64,         // LBA offset in bytes
    size: u64,           // transfer size in bytes
    buffer_cap: u32,     // capability ID for data buffer
    device_cap: u32,     // capability ID for NVMe device
}

MessageType::DmaReply {
    status: i64,         // 0 = success, negative = error
    size: u64,           // bytes transferred
}

MessageType::HardwareInterrupt {
    vector: u8,          // MSI-X vector (completion notification)
    ...
}
```

### Kernel PDX Calls (Outbound to kernel slot 0)

| ID | Name | Purpose |
|----|------|---------|
| 12 | `RESOLVE_PHYS` | Resolve physical address from lent memory capability |
| 13 | `RESOLVE_BAR` | Map PCI BAR0 into sexdrive's address space |

### Key State

```rust
static mut SQ_TAIL: u32 = 0;   // NVMe submission queue tail pointer
```

### Flow

```
sexfiles DiskFS → DmaCall{READ, offset, size}
  → sexdrive receives via pdx_listen
  → pdx_call(kernel, RESOLVE_BAR) → BAR0 virtual address
  → build NVMe command at SQ[SQ_TAIL]
  → write SQ_TAIL to doorbell (BAR0 + 0x1008)
  → wait for MSI-X interrupt (HardwareInterrupt message)
  → check CQ for completion status
  → pdx_reply(sexfiles, DmaReply{status, size})
```

---

## 7. sexshop — Object & Package Store

**Path:** `servers/sexshop/src/`
**Files:** `main.rs`, `pdx.rs`, `transactions.rs`, `storage.rs`, `cache.rs`, `trampoline.rs`
**Phase:** Phase 20+ (replaces `sexstore`)

### Purpose

Lock-free PDX object store with package fetching, key-value store, transaction support, object migration, and zero-copy cache. Acts as the system package manager and persistent KV database.

### Store Protocol

```rust
// servers/sexshop/src/pdx.rs
pub enum StoreProtocol {
    FetchPackage    { name: u64 },
    CacheBinary     { name: u64, handover: PageHandover },
    TransactionBegin,
    TransactionCommit { tx_id: u64 },
    TransactionAbort  { tx_id: u64 },
    KVGet     { key: u64 },
    KVSet     { key: u64, value: u64 },
    KVDelete  { key: u64 },
    ObjectPut  { hash: u64, handover: PageHandover },
    ObjectGet  { hash: u64 },
    ObjectExists { hash: u64 },
    ObjectMove   { hash: u64, target_node: u64 },
    SyncFilesystem,
    Stats,
}
```

### Storage Paths

| Operation | VFS Path |
|-----------|----------|
| Package fetch | `/pkg/<name>` |
| KV store root | `/etc/sexshop/kv/<key_hex>` |
| Object store root | `/etc/sexshop/obj/<hash_hex>` |
| WAL root | `/etc/sexshop/wal/<tx_id>` |
| Commit marker | `/etc/sexshop/commit/<tx_id>` |

All VFS calls go through sexfiles (Slot 1).

### Transaction System (`transactions.rs`)

```rust
pub struct TxManager {
    transaction_id: AtomicU64,
    pending: Mutex<Vec<u64>>,
}

impl TxManager {
    pub fn begin(&self) -> u64          // returns tx_id, writes WAL header
    pub fn commit(&self, tx_id: u64)    // writes commit marker, removes from pending
    pub fn abort(&self, tx_id: u64)     // removes WAL, removes from pending
}
```

WAL files persist across crashes. On startup, any uncommitted WAL entries are rolled back.

### Object Cache (`cache.rs`)

```rust
pub struct ObjectCache {
    inner: BTreeMap<String, PageHandover>,   // name/hash → PageHandover
    hits: AtomicU64,
    capacity: usize,                          // max 1024 entries
}

impl ObjectCache {
    pub fn lookup(&self, name: &str) -> Option<PageHandover>
    pub fn insert(&self, name: &str, h: PageHandover)
    pub fn invalidate(&self, name: &str)
}
```

### Outbound PDX Calls

| Target | Slot | Purpose |
|--------|------|---------|
| sexfiles | 1 | All VFS operations (open/read/write/close) |
| sexnet | 4 | `ObjectMove` → cluster distribution via `CLUSTER_DISTRIBUTE` |

### Statistics

```rust
pub static IPC_OPS_TOTAL: AtomicU64
pub static ZERO_COPY_HANDOVERS: AtomicU64
```

### Registration

```rust
// trampoline.rs
let ring = safe_pdx_register("store");
```

---

## 8. sexnet — Network Manager

**Path:** `servers/sexnet/src/main.rs`
**Phase:** Phase 19+

### Purpose

WiFi / VPN state manager. Provides network status queries, AP scanning, connection management, and inter-cluster communication primitives.

### Net State

```rust
pub enum WifiState { Disconnected, Connected }
pub enum VpnState  { Down, Up }

pub struct NetState {
    pub wifi: WifiState,
    pub vpn: VpnState,
    pub link_speed_mbps: u32,
    pub ipv4: u32,               // e.g., 0xC0A80164 = 192.168.1.100
}

pub static STATE: Mutex<NetState> = ...;
```

### PDX Syscall IDs (Inbound)

| ID Constant | Action |
|-------------|--------|
| `SEXNET_GET_STATUS` | Returns `NetState` fields |
| `SEXNET_SCAN_WIFI` | Returns mock AP table |
| `SEXNET_CONNECT` | Transitions to `WifiState::Connected`, assigns `192.168.1.100` |
| `SEXNET_DISCONNECT` | Transitions to `WifiState::Disconnected` |
| `SEXNET_VPN_UP` | Transitions to `VpnState::Up` (requires WiFi connected) |
| `SEXNET_VPN_DOWN` | Transitions to `VpnState::Down` |
| `SEXNET_GET_IP` | Returns current `ipv4` as u32 |
| `CLUSTER_RESOLVE` (0x500) | Resolves cluster object capability (from sexnode) |
| `CLUSTER_SIGNAL_SEND` (0x600) | Forwards signals across cluster nodes (from sexnode) |

### Mock AP Table

```
SSID: "SexOS_Network"  RSSI: -45  Channel: 6   Flags: WPA2
SSID: "Silk_Hotspot"   RSSI: -60  Channel: 11  Flags: WPA3
SSID: "OpenAP"         RSSI: -75  Channel: 1   Flags: Open
```

---

## 9. sexnode — Cluster Node & Translator

**Path:** `servers/sexnode/src/main.rs`
**Phase:** Phase 19+

### Purpose

Multi-role server: cluster membership manager, ELF binary translator (foreign arch → native via sex-gemini), and Linux driver loader via DDE wrapper (tuxedo).

### Node Protocol (Inbound)

```rust
pub enum NodeProtocol {
    CapabilityResolve { name: u64 },           // resolve cap by name/hash
    Heartbeat { load: u64 },                    // cluster health signal
    NodeRegister { id: u64, addr: u64 },        // register new cluster member
    ClusterObjectMigrate { hash: u64, target: u64 },
    ClusterSignalForward { signal: u64, target_node: u64 },
}
```

### Cluster Operations

```
CapabilityResolve:
  → check local capability table
  → if not found: pdx_call(sexnet slot 4, CLUSTER_RESOLVE 0x500, name)
  → return resolved capability

Heartbeat:
  → update local load counter
  → pdx_call(sexnet slot 4, CLUSTER_SIGNAL_SEND 0x600, SIGALRM=14)

ClusterObjectMigrate:
  → pdx_call(sexshop slot 1, ObjectMove { hash, target_node })

ClusterSignalForward:
  → pdx_call(sexnet slot 4, CLUSTER_SIGNAL_SEND 0x600, signal)
```

### ELF Translation

```rust
pub struct TranslatorCall {
    pub command: u32,      // 1 = TRANSLATE_ELF
    pub path_ptr: u64,     // pointer to source ELF path string
    pub code_cap: u32,     // capability for source ELF binary
}
```

**TRANSLATE_ELF (1) flow:**
1. `pdx_call(kernel slot 1, RESOLVE_VADDR 14, path_ptr)` → source code capability
2. `pdx_call(sexc slot 2, EXEC 2, "sex-gemini")` → invoke compiler
3. Returns translated native entry point `0x4000_1000` (currently stubbed)

### Linux Driver Loading (DDE)

```rust
pub struct DriverLoadCall {
    pub command: u32,           // 1 = LOAD_LINUX_DRIVER
    pub driver_name_ptr: u64,   // pointer to driver name string
}
```

**LOAD_LINUX_DRIVER (1) flow:**
1. `pdx_call(sexshop slot 1, FetchPackage, driver_name)` → download driver source
2. `pdx_call(sexc slot 2, EXEC 2, "dde-wrap")` → translate via tuxedo DDE wrapper
3. `pdx_call(sexc slot 2, SPAWN_PD 17, entry)` → run as isolated Protection Domain

### Capability Slots Used

| Slot | Server |
|------|--------|
| 1 | sexshop (package fetching) |
| 2 | sexc (exec / spawn) |
| 4 | sexnet (cluster ops) |

---

## 10. sex-ld — Dynamic Linker

**Path:** `servers/sex-ld/src/`
**Files:** `main.rs`, `pdx.rs`
**Phase:** Phase 19+

### Purpose

SASOS replacement for `ld.so`. Handles dynamic symbol resolution and library mapping in the single address space. Calls `sexshop` to fetch library objects.

### LD Protocol (Inbound)

```rust
pub enum LdProtocol {
    ResolveObject { name: u64 },           // resolve library by name hash
    MapLibrary    { hash: u64, base_addr: u64 }, // map library at base_addr
    GetEntry      { hash: u64 },           // get entry point for resolved library
    Stats,
}
```

### Handlers

| Message | Action |
|---------|--------|
| `ResolveObject { name }` | Returns mock hash `0x1234` (stub) |
| `MapLibrary { hash, base_addr }` | Calls sexshop (slot 4) via `ObjectGet { hash }` to fetch binary |
| `GetEntry { hash }` | Returns mock entry `0x4000_0000` (stub) |
| `Stats` | Returns `LD_OPS_TOTAL` |

### Statistics

```rust
pub static LD_OPS_TOTAL: AtomicU64
```

### Capability Slots Used

| Slot | Server |
|------|--------|
| 4 | sexshop (object fetch) |

---

## 11. sexc — POSIX Syscall Server

**Path:** `servers/sexc/src/`
**Files:** `main.rs`, `pipe.rs`, `trampoline.rs`
**Phase:** Phase 24 (planned)

### Purpose

POSIX compatibility layer. Provides pipe I/O, process forking/exec, and signal trampolines via PDX. Bridges POSIX-style semantics into the SexOS PDX model.

### Inbound Message Types

```rust
MessageType::IpcCall  { func_id: u32, arg0: u64 }
MessageType::PipeCall { command: u32, pipe_cap: u32, buffer_cap: u32, size: u64 }
MessageType::ProcCall { command: u32, path_ptr: u64, arg_ptr: u64, page_handover: PageHandover }
MessageType::Signal   { ... }
```

### Pipe Operations (`pipe.rs`)

| Command | ID | Action |
|---------|----|--------|
| `PIPE_CREATE` | 1 | Allocate 4 KiB frame via sext (Cap 2). Returns pipe capability. |
| `PIPE_WRITE` | 2 | Lend `buffer_cap` memory to pipe owner PD. |
| `PIPE_READ` | 3 | Zero-copy ring buffer read from lent memory. |

```rust
// Pipe state
pub struct PipeState {
    pub read_pos: AtomicU64,
    pub write_pos: AtomicU64,
    pub capacity: u64,      // 4096 bytes
    pub pfn: u64,
}
```

### Process Operations (main.rs)

| Command | ID | PDX Call | Purpose |
|---------|----|----------|---------|
| `FORK` | 1 | kernel slot 1, syscall 18 | Fork current process |
| `EXEC` | 2 | sex-ld slot 2, syscall 1 | Execute ELF at path |
| `EXEC_PAGE` | 3 | sexnode slot 2, syscall 2 | Execute page-based binary |

### Signal Trampoline (`trampoline.rs`)

```rust
pub struct SigAction {
    pub sa_handler: usize,    // ring-3 signal handler address
    pub sa_flags: u64,
}

pub static SIGNAL_STATE: AtomicBool

pub fn start_signal_trampoline()
pub fn sexc_trampoline_entry()   // Phase 25 placeholder
```

Registers background PDX listener on a separate stack to handle async signal delivery.

### Capability Slots Used

| Slot | Server |
|------|--------|
| 1 | Kernel (fork syscall) |
| 2 | sex-ld (exec) or sexnode (page-exec) |

---

## 12. sext — Demand Pager

**Path:** `servers/sext/src/main.rs`
**Phase:** Phase 19 stub

### Purpose

Handles demand paging. Receives `MessageType::PageFault` from the kernel's page fault handler (`SEXT_QUEUE` in `interrupts.rs`) and resolves faults by mapping pages on demand.

### Current Status

**Stub implementation.** Always replies with status `0` (success). Full demand paging not yet implemented.

```rust
pub extern "C" fn _start() -> ! {
    loop {
        let event = pdx_listen(0);
        match event.msg_type {
            MessageType::PageFault { .. } => pdx_reply(event.caller_pd, 0),
            _ => {}
        }
    }
}
```

### Integration

The kernel's page fault handler (`kernel/src/interrupts.rs`) pushes `PageFaultEvent` structs into `FAULT_RING` and `SEXT_QUEUE`. In full implementation, sext would:
1. Receive the faulting address + PD ID
2. Allocate a new frame
3. Map it in `GLOBAL_VAS` with the appropriate PKU key
4. Resume the faulting PD

---

## 13. sexgemini — Compiler Toolchain

**Path:** `servers/sexgemini/src/main.rs`
**Phase:** Phase 19 stub

### Purpose

Native SexOS compiler toolchain (sex-gemini). Used by `sexnode` for ELF translation and JIT compilation of foreign binaries.

### Current Status

**Empty stub.** Entry point loops forever without processing messages.

```rust
pub extern "C" fn _start() -> ! {
    loop { core::hint::spin_loop(); }
}
```

---

## 14. sexstore / sexstore-gui — Legacy Object Store

**Path:** `servers/sexstore/src/main.rs`, `servers/sexstore-gui/src/main.rs`
**Phase:** Deprecated (replaced by sexshop in Phase 20)

### Current Status

Both are **empty stub loops**. Kept in tree for reference. New code should use `sexshop`.

---

## 15. tuxedo — DDE Translation Broker

**Path:** `servers/tuxedo/src/`
**Files:** `main.rs`, `lib.rs`
**Phase:** Phase 19 stub

### Purpose

Device Driver Environment (DDE) translation broker. Provides Linux driver compatibility shim. Called by `sexnode`'s `LOAD_LINUX_DRIVER` path via sexc's `dde-wrap` exec.

### Current Status

**Minimal stub.** `lib.rs` has an `init()` placeholder. `main.rs` loops.

```rust
// tuxedo/src/lib.rs
pub fn init() {
    // DDE translation init — Phase 19 stub
}
```

Full implementation would provide:
- Linux kernel API shims (request_irq, ioremap, dma_alloc_coherent, etc.)
- ELF relocation for Linux driver `.ko` modules
- IRQ vector forwarding via PDX

---

## 16. PDX Common Patterns

### Registration Pattern

All servers that accept incoming calls register by name:

```rust
let ring = safe_pdx_register("vfs");   // sexfiles
let ring = safe_pdx_register("store"); // sexshop
```

Returns an `AtomicRing` pointer. Server polls: `ring.pop_front()`.

### Message Dispatch Pattern

```rust
loop {
    if let Some(msg) = ring.pop_front() {
        let reply = match msg.variant {
            Foo { .. } => handle_foo(msg),
            Bar { .. } => handle_bar(msg),
        };
        pdx_reply(msg.caller_pd, reply);
    }
    core::hint::spin_loop();
}
```

### Zero-Copy Handover Pattern

```rust
// Sender: grant temporary access to your pages
let saved_pkru = pku_grant_temporary(target_pku_key);
let handover = PageHandover { pfn: my_page_pfn, pku_key: target_pku_key };
pdx_call(target_slot, OP_WITH_HANDOVER, &handover as *const _ as u64, 0);
pku_restore(saved_pkru);

// Receiver: access pages via handover
let saved = pku_grant_temporary(handover.pku_key);
let ptr = (handover.pfn * 4096 + HHDM_OFFSET) as *mut u8;
// ... use ptr ...
pku_restore(saved);
```

### Capability Slot Convention

```
Slot 0  → self / generic (pdx_listen target)
Slot 1  → kernel  OR  primary backing service (VFS, storage)
Slot 2  → allocator (sext)  OR  secondary service (sexnode, sex-ld)
Slot 4  → sexnet (cluster/network)
Slot 5  → sexdisplay (compositor)
```

### Statistics Pattern (all servers)

```rust
pub static IPC_OPS_TOTAL: AtomicU64 = AtomicU64::new(0);
// Increment on every incoming message
IPC_OPS_TOTAL.fetch_add(1, Ordering::Relaxed);
```

---

## 17. Server Dependency Map

```
silk-shell ──────────────────────────→ sexdisplay (slot 5)
                                          ↑
sexinput ────────────────────────────────┘ (HIDEvent → slot 1)

sexfiles ──────────────────────────→ sexdrive (DiskFS backend)
   ↑
sexshop ──────┬────────────────────→ sexfiles (VFS, slot 1)
              └────────────────────→ sexnet (ObjectMove, slot 4)
   ↑
sex-ld ──────────────────────────→ sexshop (ObjectGet, slot 4)
   ↑
sexnode ──────┬────────────────────→ sexshop (FetchPackage, slot 1)
              ├────────────────────→ sexc    (EXEC/SPAWN, slot 2)
              └────────────────────→ sexnet  (cluster ops, slot 4)
   ↑
sexc ─────────┬────────────────────→ kernel  (fork, slot 1)
              └────────────────────→ sex-ld  (exec, slot 2)

sext ─────────────────────────────→ (responds to kernel page fault queue)

sexgemini ────────────────────────→ (stub — invoked by sexnode translator)
tuxedo ───────────────────────────→ (stub — invoked by sexnode DDE loader)

sexstore ─────────────────────────→ (deprecated stub)
sexstore-gui ─────────────────────→ (deprecated stub)
```

### Kernel → Server IPC

```
kernel init.rs  ──pdx_spawn──→  sexdisplay  (boot-time, ring 3 handoff)
kernel init.rs  ──irq_route──→  sexinput    (IRQ 0x21)
kernel handoff  ──message────→  sexdisplay  (DisplayPrimaryFramebuffer)
kernel pf handler → SEXT_QUEUE → sext       (PageFault messages)
```

---

*Server manual compiled from Phase 21 codebase audit. Stub servers noted — see phase roadmap for implementation schedule.*
