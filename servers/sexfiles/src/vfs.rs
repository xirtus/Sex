extern crate alloc;
use crate::backends::ramfs::RamFs;
use crate::backends::FsBackend;
use crate::backends::diskfs::{DiskFs, DiskManifestEntryV1, DISKFS_MANIFEST_OBJECT_PATH, DISKFS_MANIFEST_MAGIC, DISKFS_MANIFEST_FLAG_READ, DISKFS_MANIFEST_FLAG_WRITE};
use sex_pdx::SLOT_BUF_LEND;
use crate::messages;
use core::sync::atomic::{AtomicU64, Ordering};

/// VFS operation counters (diagnostic only).

// SERIAL_DIET_V1: shared budget for hot-loop diagnostics. Early boot keeps
// the full trace (first 200 lines); steady-state loops go quiet so
// serial VM-exits stop dominating wall clock. Error and gate-required
// markers are NOT routed through this.
static mut HOT_LOG_BUDGET: u32 = 200;
#[inline(always)]
fn hot_log() -> bool {
    unsafe {
        if HOT_LOG_BUDGET > 0 { HOT_LOG_BUDGET -= 1; true } else { false }
    }
}

pub static IPC_OPS_TOTAL: AtomicU64 = AtomicU64::new(0);

/// The single RamFS instance backing all VFS operations.
pub static RAMFS: RamFs = RamFs::new();

// ── DiskFS bridge buffer state ──────────────────────────────────────────────
/// One-time granted MemLend buffer VA for DiskFS bridge ops.
/// Granted via sys_grant_mem_lend(SLOT_BLOCK, 4096, SLOT_BUF_LEND) on first use.
/// SexFiles owns this buffer for its lifetime; Linen never sees it.
static DISKFS_BRIDGE_BUF_VA: AtomicU64 = AtomicU64::new(0);

/// Whether the manifest at LBA 2046 has been validated (or bootstrapped).
/// Set to true after the first successful diskfs_ensure_manifest() call.
/// Avoids redundant NVMe manifest reads on every bridge WRITE/READ.
static DISKFS_MANIFEST_READY: AtomicU64 = AtomicU64::new(0);

/// Currently selected DiskFS object path_id for bridge operations,
/// PER CALLER PD. Default 0 = /disk/sexfiles-proof-v1.
/// SEXFILES_DEFER_V1 made concurrent clients real: a single global
/// selection let interleaved clients clobber each other and do I/O on
/// the wrong object. Indexed by caller_pd % DISKFS_CLIENT_SLOTS.
/// Set via OP_DISKFS_SELECT. Read by bridge WRITE/READ/STAT/HASH handlers.
const DISKFS_CLIENT_SLOTS: usize = 32;
static DISKFS_SELECTED_PATH_ID: [AtomicU64; DISKFS_CLIENT_SLOTS] =
    [const { AtomicU64::new(0) }; DISKFS_CLIENT_SLOTS];

fn diskfs_selected_for(caller_pd: u32) -> u64 {
    DISKFS_SELECTED_PATH_ID[caller_pd as usize % DISKFS_CLIENT_SLOTS].load(Ordering::Relaxed)
}

/// Whether a SELECT has been issued at least once since boot.
static DISKFS_SELECT_USED: AtomicU64 = AtomicU64::new(0);

static DISKFS_BRIDGE_REUSE_PRINTED: AtomicU64 = AtomicU64::new(0);

pub(crate) fn diskfs_bridge_get_buf_va() -> u64 {
    let va = DISKFS_BRIDGE_BUF_VA.load(Ordering::Relaxed);
    if va != 0 && va != u64::MAX {
        if DISKFS_BRIDGE_REUSE_PRINTED.compare_exchange(0, 1, Ordering::Relaxed, Ordering::Relaxed).is_ok() {
            crate::pdx::serial_println!(
                "[sexfiles.bridge.diskfs.buf.reuse] va={:#x}",
                va
            );
        }
        return va;
    }
    let new_va = sex_pdx::sys_grant_mem_lend(
        sex_pdx::SLOT_BLOCK, 4096, sex_pdx::SLOT_BUF_LEND,
    );
    if new_va != 0 && new_va != u64::MAX {
        DISKFS_BRIDGE_BUF_VA.store(new_va, Ordering::Relaxed);
        crate::pdx::serial_println!(
            "[sexfiles.bridge.diskfs.buf.ready] buf_va={:#x}",
            new_va
        );
    }
    new_va
}

// ── DiskFS bridge inline handlers ───────────────────────────────────────────

fn handle_diskfs_select(path_id: u64, caller_pd: u32) -> u64 {
    if hot_log() { crate::pdx::serial_println!(
        "[sexfiles.bridge.diskfs.recv] op=0x3E select path_id={}",
        path_id
    ); }

    let buf_va = diskfs_bridge_get_buf_va();
    if buf_va == 0 || buf_va == u64::MAX {
        crate::pdx::serial_println!(
            "[sexfiles.bridge.diskfs.select.err] reason=grant_failed"
        );
        return messages::ERR_NOT_FOUND as u64;
    }
    if let Err(e) = v3_ensure(buf_va) {
        crate::pdx::serial_println!(
            "[sexfiles.bridge.diskfs.select.err] reason=v3_ensure_failed code={}", e);
        return e;
    }
    match v3_resolve(path_id) {
        Ok(_entry) => {
            DISKFS_SELECTED_PATH_ID[caller_pd as usize % DISKFS_CLIENT_SLOTS]
                .store(path_id, Ordering::Relaxed);
            if DISKFS_SELECT_USED.load(Ordering::Relaxed) == 0 {
                DISKFS_SELECT_USED.store(1, Ordering::Relaxed);
                crate::pdx::serial_println!(
                    "[sexfiles.bridge.diskfs.select.v1_single_client]"
                );
            }
            if hot_log() { crate::pdx::serial_println!(
                "[sexfiles.bridge.diskfs.select.ok] path_id={}", path_id
            ); }
            0
        }
        Err(e) => {
            crate::pdx::serial_println!(
                "[sexfiles.bridge.diskfs.select.err] path_id={} code={}",
                path_id, e
            );
            e
        }
    }
}

// DISKFS_V3: path-string resolution retired — selection resolves through
// the dynamic manifest table (v3_resolve / v3_name_of).

fn handle_diskfs_write(byte_offset: u64, data_lo: u64, data_hi: u64, caller_pd: u32) -> u64 {
    if hot_log() { crate::pdx::serial_println!(
        "[sexfiles.bridge.diskfs.recv] op=0x38 offset={}",
        byte_offset
    ); }

    if byte_offset >= messages::DISKFS_OBJECT_SIZE {
        crate::pdx::serial_println!(
            "[sexfiles.bridge.diskfs.write.err] reason=offset_past_end offset={}",
            byte_offset
        );
        return messages::ERR_OVERFLOW as u64;
    }

    let max_write = (messages::DISKFS_OBJECT_SIZE - byte_offset) as usize;
    if max_write < messages::DISKFS_MAX_WRITE {
        // Boundary: reject writes that would cross the 4096-byte boundary.
        // Preferred for V1 simplicity (option A from plan).
        crate::pdx::serial_println!(
            "[sexfiles.bridge.diskfs.write.err] reason=boundary_write offset={} max={}",
            byte_offset, max_write
        );
        return messages::ERR_OVERFLOW as u64;
    }

    let buf_va = diskfs_bridge_get_buf_va();
    if buf_va == 0 || buf_va == u64::MAX {
        crate::pdx::serial_println!(
            "[sexfiles.bridge.diskfs.write.err] reason=grant_failed buf_va={:#x}",
            buf_va
        );
        return messages::ERR_NOT_FOUND as u64;
    }

    // DISKFS_V3: ensure manifest loaded, resolve selection to an entry.
    if let Err(e) = v3_ensure(buf_va) {
        crate::pdx::serial_println!(
            "[sexfiles.bridge.diskfs.write.err] reason=v3_ensure_failed code={}", e);
        return e;
    }
    let entry = match v3_resolve(diskfs_selected_for(caller_pd)) {
        Ok(en) => en,
        Err(e) => return e,
    };

    // Pack 16 bytes inline from data_lo + data_hi.
    let mut inline_data = [0u8; 16];
    {
        let lo = data_lo.to_le_bytes();
        let hi = data_hi.to_le_bytes();
        let mut i = 0;
        while i < 8 {
            inline_data[i] = lo[i];
            inline_data[i + 8] = hi[i];
            i += 1;
        }
    }

    match DiskFs::diskfs_write_object_entry(
        entry,
        byte_offset,
        &inline_data,
        buf_va,
    ) {
        Ok(n) => {
            crate::pdx::serial_println!(
                "[sexfiles.bridge.diskfs.write.ok] offset={} written={}",
                byte_offset, n
            );
            n
        }
        Err(e) => {
            crate::pdx::serial_println!(
                "[sexfiles.bridge.diskfs.write.err] offset={} code={}",
                byte_offset, e
            );
            e
        }
    }
}

fn handle_diskfs_read(byte_offset: u64, max_len: u64, caller_pd: u32) -> u64 {
    if hot_log() { crate::pdx::serial_println!(
        "[sexfiles.bridge.diskfs.recv] op=0x39 offset={}",
        byte_offset
    ); }

    if max_len == 0 || max_len > messages::DISKFS_MAX_READ as u64 {
        crate::pdx::serial_println!(
            "[sexfiles.bridge.diskfs.read.err] reason=bad_max_len max_len={}",
            max_len
        );
        return messages::ERR_OVERFLOW as u64;
    }

    if byte_offset >= messages::DISKFS_OBJECT_SIZE {
        crate::pdx::serial_println!(
            "[sexfiles.bridge.diskfs.read.err] reason=offset_past_end offset={}",
            byte_offset
        );
        return messages::ERR_OVERFLOW as u64;
    }

    if byte_offset + max_len > messages::DISKFS_OBJECT_SIZE {
        crate::pdx::serial_println!(
            "[sexfiles.bridge.diskfs.read.err] reason=read_past_end offset={} max_len={}",
            byte_offset, max_len
        );
        return messages::ERR_OVERFLOW as u64;
    }

    let buf_va = diskfs_bridge_get_buf_va();
    if buf_va == 0 || buf_va == u64::MAX {
        crate::pdx::serial_println!(
            "[sexfiles.bridge.diskfs.read.err] reason=grant_failed buf_va={:#x}",
            buf_va
        );
        return messages::ERR_NOT_FOUND as u64;
    }

    // DISKFS_V3: ensure manifest loaded, resolve selection to an entry.
    if let Err(e) = v3_ensure(buf_va) {
        crate::pdx::serial_println!(
            "[sexfiles.bridge.diskfs.read.err] reason=v3_ensure_failed code={}", e);
        return e;
    }
    let entry = match v3_resolve(diskfs_selected_for(caller_pd)) {
        Ok(en) => en,
        Err(e) => return e,
    };

    let rlen = max_len as usize;
    let mut rbuf = [0u8; 8];
    match DiskFs::diskfs_read_object_entry(
        entry,
        byte_offset,
        &mut rbuf[..rlen],
        buf_va,
    ) {
        Ok(n) => {
            // Pack up to 8 bytes into reply u64 (LE).
            let mut reply: u64 = 0;
            let mut i = 0;
            while i < n as usize && i < 8 {
                reply |= (rbuf[i] as u64) << (i * 8);
                i += 1;
            }
            crate::pdx::serial_println!(
                "[sexfiles.bridge.diskfs.read.ok] offset={} read={}",
                byte_offset, n
            );
            if hot_log() { crate::pdx::serial_println!(
                "[sexfiles.bridge.diskfs.read.reply] caller={} value={:#x} off={} read={}",
                caller_pd, reply, byte_offset, n
            ); }
            reply
        }
        Err(e) => {
            crate::pdx::serial_println!(
                "[sexfiles.bridge.diskfs.read.err] caller={} err={} off={}",
                caller_pd, e, byte_offset
            );
            e
        }
    }
}

fn handle_diskfs_flush() -> u64 {
    if hot_log() { crate::pdx::serial_println!(
        "[sexfiles.bridge.diskfs.recv] op=0x3A"
    ); }
    let status = DiskFs::diskfs_fsync();
    if status == 0 {
        crate::pdx::serial_println!("[sexfiles.bridge.diskfs.flush.ok]");
    } else {
        crate::pdx::serial_println!(
            "[sexfiles.bridge.diskfs.flush.err] status={} honest=flush_not_emulated_by_qemu_nvme",
            status
        );
    }
    status
}

fn handle_diskfs_stat(caller_pd: u32) -> u64 {
    if hot_log() { crate::pdx::serial_println!(
        "[sexfiles.bridge.diskfs.recv] op=0x3B"
    ); }
    let path_id = diskfs_selected_for(caller_pd);
    let name = match v3_name_of(path_id) {
        Some(n) => n,
        None => return messages::ERR_NOT_FOUND as u64,
    };
    let path = &name[..v3_name_len(&name)];
    // All V3 objects are fixed 4096 bytes, READ|WRITE.
    let flags: u64 = (DISKFS_MANIFEST_FLAG_READ | DISKFS_MANIFEST_FLAG_WRITE) as u64;
    let size: u64 = messages::DISKFS_OBJECT_SIZE;
    let packed = (flags << 32) | (size & 0xFFFF_FFFF);
    if hot_log() { crate::pdx::serial_println!(
        "[sexfiles.bridge.diskfs.stat.ok] path_id={} path={} size={} flags={:#x}",
        path_id,
        core::str::from_utf8(path).unwrap_or("?"),
        size, flags
    ); }
    packed
}

fn handle_diskfs_manifest_hash(caller_pd: u32) -> u64 {
    if hot_log() { crate::pdx::serial_println!(
        "[sexfiles.bridge.diskfs.recv] op=0x3C"
    ); }
    let path_id = diskfs_selected_for(caller_pd);
    let name = match v3_name_of(path_id) {
        Some(n) => n,
        None => return messages::ERR_NOT_FOUND as u64,
    };
    let hash = DiskFs::proof_manifest_name_hash(&name[..v3_name_len(&name)]);
    crate::pdx::serial_println!(
        "[sexfiles.bridge.diskfs.manifest_hash.ok] path_id={} hash={:#x}",
        path_id, hash
    );
    hash
}

/// Route a PDX message to the appropriate backend handler.
/// Called from the trampoline message loop.
/// `caller_pd` is the PD of the requesting process (from PDX message).

// ═══════════════════════════════════════════════════════════════════════════
//  DISKFS_V3 — dynamic object store (replaces the fixed 3-path assumption).
//
//  On-disk: ONE 512-byte manifest sector at LBA 2046.
//    [0..8)   magic  "SDISKMV1" (unchanged — version field disambiguates)
//    [8..10)  version = 3
//    [10..12) entry_count = 15
//    [12..16) generation (u32, bumped on every metadata change)
//    16 + i*32 .. : entry i (15 entries × 32 bytes = 480):
//       name[24]  zero-padded ASCII; name[0]==0 → slot free
//       +24 u16   start_lba (deterministic: 2046 - 8*(i+1))
//       +26 u16   len_sectors (= 8, 4096-byte objects)
//       +28 u16   flags (READ|WRITE)
//       +30 u16   slot generation (bumped on create; stale-id detection)
//
//  Allocation: slot index IS the object id (path_id). LBA mapping is fixed
//  per slot, so no allocator and no fragmentation; deleting clears the name
//  and bumps generation; re-creating in the slot zeroes the object's first
//  sector so stale content headers (e.g. quil "QP01") never resurrect.
//  Crash behavior: metadata changes are a single 512-byte sector write
//  (write + read-back verify); content writes are app-level (header last
//  via reverse-chunk order in quil).
//  Upgrade: a V2 (or missing) manifest is rewritten as V3 with slots 0-2
//  seeded at the legacy LBAs — existing object CONTENT is untouched.
//  Enumeration order: slot index ascending — deterministic.
// ═══════════════════════════════════════════════════════════════════════════
pub const DISKFS_V3_SLOTS: usize = 15;
pub const DISKFS_V3_NAME_MAX: usize = 24;
const DISKFS_V3_VERSION: u16 = 3;
/// System objects (slots 0-2) cannot be deleted or renamed.
const DISKFS_V3_SYSTEM_SLOTS: u64 = 3;

#[derive(Clone, Copy)]
struct V3Entry {
    name: [u8; DISKFS_V3_NAME_MAX],
    start_lba: u16,
    len_sectors: u16,
    flags: u16,
    gen: u16,
}

const V3_EMPTY: V3Entry = V3Entry {
    name: [0u8; DISKFS_V3_NAME_MAX], start_lba: 0, len_sectors: 8,
    flags: (DISKFS_MANIFEST_FLAG_READ | DISKFS_MANIFEST_FLAG_WRITE), gen: 0,
};

static mut V3_TABLE: [V3Entry; DISKFS_V3_SLOTS] = [V3_EMPTY; DISKFS_V3_SLOTS];
static V3_LOADED: AtomicU64 = AtomicU64::new(0);
static V3_GENERATION: AtomicU64 = AtomicU64::new(0);

fn v3_slot_lba(i: usize) -> u16 { (2046 - 8 * (i as u64 + 1)) as u16 }

fn v3_name_len(name: &[u8; DISKFS_V3_NAME_MAX]) -> usize {
    let mut n = 0;
    while n < DISKFS_V3_NAME_MAX && name[n] != 0 { n += 1; }
    n
}

fn v3_in_use(i: usize) -> bool {
    unsafe { V3_TABLE[i].name[0] != 0 }
}

/// Read one 512-byte sector at `lba` into `out` via the lend buffer.
fn v3_sector_read(lba: u64, buf_va: u64, out: &mut [u8; 512]) -> Result<(), u64> {
    let status = DiskFs::diskfs_block_read(lba * 512, 512, SLOT_BUF_LEND);
    if status != 0 { return Err(status); }
    unsafe {
        let p = buf_va as *const u8;
        for i in 0..512 { out[i] = core::ptr::read_volatile(p.add(i)); }
    }
    Ok(())
}

fn v3_sector_write(lba: u64, buf_va: u64, data: &[u8; 512]) -> Result<(), u64> {
    unsafe {
        let p = buf_va as *mut u8;
        for i in 0..512 { core::ptr::write_volatile(p.add(i), data[i]); }
    }
    let status = DiskFs::diskfs_block_write(lba * 512, 512, SLOT_BUF_LEND);
    if status != 0 { return Err(status); }
    Ok(())
}

fn v3_persist(buf_va: u64) -> Result<(), u64> {
    let mut sector = [0u8; 512];
    sector[0..8].copy_from_slice(&DISKFS_MANIFEST_MAGIC.to_le_bytes());
    sector[8..10].copy_from_slice(&DISKFS_V3_VERSION.to_le_bytes());
    sector[10..12].copy_from_slice(&(DISKFS_V3_SLOTS as u16).to_le_bytes());
    let g = V3_GENERATION.fetch_add(1, Ordering::Relaxed) as u32 + 1;
    sector[12..16].copy_from_slice(&g.to_le_bytes());
    for i in 0..DISKFS_V3_SLOTS {
        let e = unsafe { V3_TABLE[i] };
        let off = 16 + i * 32;
        sector[off..off + DISKFS_V3_NAME_MAX].copy_from_slice(&e.name);
        sector[off + 24..off + 26].copy_from_slice(&e.start_lba.to_le_bytes());
        sector[off + 26..off + 28].copy_from_slice(&e.len_sectors.to_le_bytes());
        sector[off + 28..off + 30].copy_from_slice(&e.flags.to_le_bytes());
        sector[off + 30..off + 32].copy_from_slice(&e.gen.to_le_bytes());
    }
    v3_sector_write(2046, buf_va, &sector)?;
    // Read-back verify.
    let mut check = [0u8; 512];
    v3_sector_read(2046, buf_va, &mut check)?;
    if check != sector {
        crate::pdx::serial_println!("[sexfiles.diskfs.v3.persist.err] reason=verify_mismatch");
        return Err(messages::ERR_BAD_CMD as u64);
    }
    crate::pdx::serial_println!("[sexfiles.diskfs.v3.persist.ok] generation={}", g);
    Ok(())
}

fn v3_seed_slot(i: usize, name: &[u8]) {
    unsafe {
        let mut e = V3_EMPTY;
        let n = name.len().min(DISKFS_V3_NAME_MAX);
        e.name[..n].copy_from_slice(&name[..n]);
        e.start_lba = v3_slot_lba(i);
        e.gen = 1;
        V3_TABLE[i] = e;
    }
}

/// Load (or bootstrap/upgrade) the V3 manifest. Idempotent; cached.
fn v3_ensure(buf_va: u64) -> Result<(), u64> {
    if V3_LOADED.load(Ordering::Relaxed) != 0 { return Ok(()); }
    let mut sector = [0u8; 512];
    v3_sector_read(2046, buf_va, &mut sector)?;
    let magic = u64::from_le_bytes(sector[0..8].try_into().unwrap());
    let version = u16::from_le_bytes(sector[8..10].try_into().unwrap());
    if magic == DISKFS_MANIFEST_MAGIC && version == DISKFS_V3_VERSION {
        let g = u32::from_le_bytes(sector[12..16].try_into().unwrap());
        V3_GENERATION.store(g as u64, Ordering::Relaxed);
        let mut live = 0;
        for i in 0..DISKFS_V3_SLOTS {
            let off = 16 + i * 32;
            let mut e = V3_EMPTY;
            e.name.copy_from_slice(&sector[off..off + DISKFS_V3_NAME_MAX]);
            e.start_lba = u16::from_le_bytes(sector[off + 24..off + 26].try_into().unwrap());
            e.len_sectors = u16::from_le_bytes(sector[off + 26..off + 28].try_into().unwrap());
            e.flags = u16::from_le_bytes(sector[off + 28..off + 30].try_into().unwrap());
            e.gen = u16::from_le_bytes(sector[off + 30..off + 32].try_into().unwrap());
            // Corrupt-entry guard: a nonzero name with an out-of-range LBA is
            // dropped (treated as free) rather than trusted.
            if e.name[0] != 0 && (e.start_lba != v3_slot_lba(i) || e.len_sectors != 8) {
                crate::pdx::serial_println!(
                    "[sexfiles.diskfs.v3.load.drop] slot={} reason=bad_geometry lba={}",
                    i, e.start_lba);
                e = V3_EMPTY;
            }
            if e.name[0] != 0 { live += 1; }
            unsafe { V3_TABLE[i] = e; }
        }
        V3_LOADED.store(1, Ordering::Relaxed);
        crate::pdx::serial_println!("[sexfiles.diskfs.v3.load.ok] live={} generation={}", live, g);
        return Ok(());
    }
    // Bootstrap / upgrade from V2 or blank: seed the 3 legacy objects at
    // their legacy LBAs (content untouched), everything else free.
    v3_seed_slot(0, b"sexfiles-proof-v1");
    v3_seed_slot(1, b"linen-object-v1");
    v3_seed_slot(2, b"quil-object-v1");
    for i in 3..DISKFS_V3_SLOTS { unsafe { V3_TABLE[i] = V3_EMPTY; } }
    v3_persist(buf_va)?;
    V3_LOADED.store(1, Ordering::Relaxed);
    crate::pdx::serial_println!(
        "[sexfiles.diskfs.v3.bootstrap.ok] from_version={} seeded=3", version);
    Ok(())
}

/// Resolve a live path_id to a legacy-shape manifest entry for object I/O.
fn v3_resolve(path_id: u64) -> Result<DiskManifestEntryV1, u64> {
    if path_id >= DISKFS_V3_SLOTS as u64 { return Err(messages::ERR_BAD_CMD as u64); }
    let i = path_id as usize;
    if !v3_in_use(i) { return Err(messages::ERR_NOT_FOUND as u64); }
    let e = unsafe { V3_TABLE[i] };
    Ok(DiskManifestEntryV1 {
        name_hash: 0,
        start_lba: e.start_lba as u64,
        len_bytes: (e.len_sectors as u32) * 512,
        flags: e.flags,
    })
}

fn v3_name_of(path_id: u64) -> Option<[u8; DISKFS_V3_NAME_MAX]> {
    if path_id >= DISKFS_V3_SLOTS as u64 { return None; }
    let i = path_id as usize;
    if !v3_in_use(i) { return None; }
    Some(unsafe { V3_TABLE[i].name })
}

fn v3_unpack_name(lo: u64, hi: u64) -> ([u8; DISKFS_V3_NAME_MAX], usize) {
    let mut name = [0u8; DISKFS_V3_NAME_MAX];
    name[..8].copy_from_slice(&lo.to_le_bytes());
    name[8..16].copy_from_slice(&hi.to_le_bytes());
    // Sanitize: printable ASCII only, stop at first NUL.
    let mut n = 0;
    while n < 16 && name[n] != 0 {
        if name[n] < 0x20 || name[n] > 0x7E { name[n] = b'_'; }
        n += 1;
    }
    for b in name[n..].iter_mut() { *b = 0; }
    (name, n)
}

/// OP_DISKFS_CREATE (0x42): arg1|arg2 = up to 16 name bytes packed LE.
/// Returns new path_id. Duplicate name → ERR_EXISTS; no name → ERR_BAD_CMD;
/// table full → ERR_FULL. Zeroes the object's first sector so stale headers
/// from a previously deleted object never resurrect.
fn handle_diskfs_create(name_lo: u64, name_hi: u64, buf_va: u64) -> u64 {
    let (name, n) = v3_unpack_name(name_lo, name_hi);
    if n == 0 { return messages::ERR_BAD_CMD as u64; }
    for i in 0..DISKFS_V3_SLOTS {
        if v3_in_use(i) && unsafe { V3_TABLE[i].name } == name {
            crate::pdx::serial_println!(
                "[sexfiles.diskfs.v3.create.err] reason=exists slot={}", i);
            return messages::ERR_EXISTS as u64;
        }
    }
    let mut slot = None;
    for i in (DISKFS_V3_SYSTEM_SLOTS as usize)..DISKFS_V3_SLOTS {
        if !v3_in_use(i) { slot = Some(i); break; }
    }
    let Some(i) = slot else {
        crate::pdx::serial_println!("[sexfiles.diskfs.v3.create.err] reason=full");
        return messages::ERR_FULL as u64;
    };
    unsafe {
        let mut e = V3_EMPTY;
        e.name = name;
        e.start_lba = v3_slot_lba(i);
        e.gen = V3_TABLE[i].gen.wrapping_add(1).max(1);
        V3_TABLE[i] = e;
    }
    // Zero the object's first sector (kills stale content headers).
    let zero = [0u8; 512];
    if let Err(e) = v3_sector_write(v3_slot_lba(i) as u64, buf_va, &zero) {
        unsafe { V3_TABLE[i] = V3_EMPTY; }
        crate::pdx::serial_println!("[sexfiles.diskfs.v3.create.err] reason=zero_failed code={}", e);
        return e;
    }
    if let Err(e) = v3_persist(buf_va) {
        unsafe { V3_TABLE[i] = V3_EMPTY; }
        return e;
    }
    crate::pdx::serial_println!(
        "[sexfiles.diskfs.v3.create.ok] slot={} gen={}", i, unsafe { V3_TABLE[i].gen });
    i as u64
}

/// OP_DISKFS_LIST (0x43): arg0 = path_id, arg1 = query.
///   query 0/1/2 → 8 name bytes (chunk) packed LE (0 = free slot / past end)
///   query 0xFF  → (in_use<<62) | (gen<<32) | slot_count (bit63 kept clear:
///                 sync clients sign-check replies)
///   query 0xFE  → global manifest generation (change detection for Linen)
fn handle_diskfs_list(path_id: u64, query: u64) -> u64 {
    if query == 0xFE {
        return V3_GENERATION.load(Ordering::Relaxed);
    }
    if query == 0xFF {
        if path_id >= DISKFS_V3_SLOTS as u64 { return 0; }
        let i = path_id as usize;
        let used = v3_in_use(i) as u64;
        let gen = unsafe { V3_TABLE[i].gen } as u64;
        // bit62 (NOT 63): reply values are sign-checked by sync clients.
        return (used << 62) | (gen << 32) | (DISKFS_V3_SLOTS as u64);
    }
    let Some(name) = v3_name_of(path_id) else { return 0; };
    let chunk = (query as usize).min(2);
    let mut v = 0u64;
    for k in 0..8 {
        let idx = chunk * 8 + k;
        if idx < DISKFS_V3_NAME_MAX {
            v |= (name[idx] as u64) << (k * 8);
        }
    }
    v
}

/// OP_DISKFS_DELETE (0x47): arg0 = path_id. System slots protected.
fn handle_diskfs_delete(path_id: u64, buf_va: u64) -> u64 {
    if path_id < DISKFS_V3_SYSTEM_SLOTS { return messages::ERR_PERM_DENIED as u64; }
    if path_id >= DISKFS_V3_SLOTS as u64 { return messages::ERR_BAD_CMD as u64; }
    let i = path_id as usize;
    if !v3_in_use(i) { return messages::ERR_NOT_FOUND as u64; }
    unsafe {
        V3_TABLE[i].name = [0u8; DISKFS_V3_NAME_MAX];
        V3_TABLE[i].gen = V3_TABLE[i].gen.wrapping_add(1);
    }
    if let Err(e) = v3_persist(buf_va) { return e; }
    crate::pdx::serial_println!("[sexfiles.diskfs.v3.delete.ok] slot={}", i);
    0
}

/// OP_DISKFS_RENAME (0x48): arg0 = path_id, arg1|arg2 = new name.
fn handle_diskfs_rename(path_id: u64, name_lo: u64, name_hi: u64, buf_va: u64) -> u64 {
    if path_id < DISKFS_V3_SYSTEM_SLOTS { return messages::ERR_PERM_DENIED as u64; }
    if path_id >= DISKFS_V3_SLOTS as u64 { return messages::ERR_BAD_CMD as u64; }
    let i = path_id as usize;
    if !v3_in_use(i) { return messages::ERR_NOT_FOUND as u64; }
    let (name, n) = v3_unpack_name(name_lo, name_hi);
    if n == 0 { return messages::ERR_BAD_CMD as u64; }
    for j in 0..DISKFS_V3_SLOTS {
        if j != i && v3_in_use(j) && unsafe { V3_TABLE[j].name } == name {
            return messages::ERR_EXISTS as u64;
        }
    }
    unsafe { V3_TABLE[i].name = name; }
    if let Err(e) = v3_persist(buf_va) { return e; }
    crate::pdx::serial_println!("[sexfiles.diskfs.v3.rename.ok] slot={}", i);
    0
}

pub fn handle_vfs_message(type_id: u64, arg0: u64, arg1: u64, arg2: u64, caller_pd: u32) -> u64 {
    crate::pdx::serial_println!(
        "[sexfiles.vfs.enter] type={:#x} caller={} a0={:#x} a1={:#x}",
        type_id, caller_pd, arg0, arg1
    );
    IPC_OPS_TOTAL.fetch_add(1, Ordering::Relaxed);

    // All operations currently route to RamFS.
    let backend: &dyn FsBackend = &RAMFS;

    match type_id {
        // ── OP_RAMFS_OPEN ──
        // arg0 = name[0..7], arg1 = name[8..15], arg2 = name[16..23] | (flags << 24)
        messages::OP_RAMFS_OPEN => {
            // Mask flag byte (bits 24-31) from name portion per protocol spec:
            //   arg2 = name[16..23] | (flags << 24)
            let name_bytes = unpack_name(arg0, arg1, arg2 & !(0xFFu64 << 24));
            let flags = (arg2 >> 24) as u32;
            match backend.open(&name_bytes, flags, 0, caller_pd) {
                Ok(handle) => handle,
                Err(e) => e as u64,
            }
        }

        // ── OP_RAMFS_READ ──
        // arg0 = handle, arg1 = offset, arg2 = max_len
        messages::OP_RAMFS_READ => {
            let handle = arg0;
            let offset = arg1;
            let max_len = (arg2 as usize).min(messages::RAMFS_MAX_FILE_SIZE);
            let mut buf = [0u8; 8]; // Return up to 8 bytes in the reply
            let to_read = max_len.min(buf.len());
            match backend.read(handle, offset, &mut buf[..to_read], caller_pd) {
                Ok(n) => {
                    // Pack read data into reply u64
                    let mut reply = 0u64;
                    for i in 0..n.min(8) as usize {
                        reply |= (buf[i] as u64) << (i * 8);
                    }
                    reply
                }
                Err(e) => e as u64,
            }
        }

        // ── OP_RAMFS_WRITE ──
        // arg0 = handle, arg1 = offset, arg2 = packed data (8 bytes)
        messages::OP_RAMFS_WRITE => {
            let handle = arg0;
            let offset = arg1;
            let data = arg2.to_le_bytes(); // 8 bytes of data
            match backend.write(handle, offset, &data, caller_pd) {
                Ok(n) => n,
                Err(e) => e as u64,
            }
        }

        // ── OP_RAMFS_CLOSE ──
        // arg0 = handle
        messages::OP_RAMFS_CLOSE => {
            let handle = arg0;
            match backend.close(handle, caller_pd) {
                Ok(_) => 0,
                Err(e) => e as u64,
            }
        }

        // ── OP_RAMFS_LIST ──
        // arg0 = index
        // Returns: packed { handle: u64, name_len: u32 } in upper/lower bits,
        // or 0 if no more entries. Only returns entries owned by caller_pd.
        messages::OP_RAMFS_LIST => {
            let index = arg0 as usize;
            match backend.list_at(index, caller_pd) {
                Some((handle, name_len)) => {
                    (handle << 32) | (name_len as u64)
                }
                None => 0,
            }
        }

        // ── OP_RAMFS_CREATE_OWNER ──
        // arg0 = name bytes 0..7
        // arg1 = name bytes 8..15
        // arg2 = name bytes 16..23 (lower 24 bits) | (owner_pd << 32)
        messages::OP_RAMFS_CREATE_OWNER => {
            let name_bytes =
                unpack_name(arg0, arg1, arg2 & !(0xFFFF_FFFFu64 << 32));
            let owner_pd = (arg2 >> 32) as u32;
            match backend.create_with_owner(&name_bytes, owner_pd, caller_pd) {
                Ok(handle) => handle,
                Err(e) => e as u64,
            }
        }

        // ── OP_RAMFS_OBJECT_ID ──
        // arg0 = handle
        // Returns: RamFS-assigned object_id (≥1) for the open handle.
        // Caller must own the file (or caller_pd=0 for server-internal).
        // Use to resolve OQ5: obtain SexFiles-assigned global ID from a handle.
        messages::OP_RAMFS_OBJECT_ID => {
            let handle = arg0;
            match RAMFS.object_id_for_handle(handle, caller_pd) {
                Ok(id) => id,
                Err(e) => e as u64,
            }
        }

        // ── OP_RAMFS_STAT ──
        // arg0 = handle
        messages::OP_RAMFS_STAT => {
            let handle = arg0;
            match backend.stat(handle, caller_pd) {
                Ok((size, name_len)) => {
                    (size << 32) | name_len as u64
                }
                Err(e) => e as u64,
            }
        }

        // ── OP_RAMFS_READNAME ──
        // arg0 = handle, arg1 = byte_offset, arg2 = max_len (clamped to 8)
        // Returns: up to 8 filename bytes LE. 0 = EOF. negative = error.
        messages::OP_RAMFS_READNAME => {
            let handle = arg0;
            let byte_offset = arg1;
            let max_len = (arg2 as usize).min(8);
            let mut buf = [0u8; 8];
            match RAMFS.read(handle, byte_offset, &mut buf[..max_len], caller_pd) {
                Ok(n) => {
                    let mut packed = 0u64;
                    for i in 0..(n as usize).min(8) {
                        packed |= (buf[i] as u64) << (i * 8);
                    }
                    crate::pdx::serial_println!(
                        "[sexfiles.ramfs.readname.ok] handle={} off={} len={}",
                        handle, byte_offset, max_len
                    );
                    packed
                }
                Err(e) => {
                    crate::pdx::serial_println!(
                        "[sexfiles.ramfs.readname.deny] handle={} err={}",
                        handle, e
                    );
                    e as u64
                }
            }
        }

        // ── DiskFS bridge opcodes (0x38-0x3C) ──
        // Route: Linen → SLOT_STORAGE → SexFiles → DiskFS → SLOT_BLOCK → SexDrive → NVMe
        // Fixed-object bridge: /disk/sexfiles-proof-v1
        messages::OP_DISKFS_WRITE => {
            // arg0 = byte_offset, arg1 = data_lo, arg2 = data_hi
            if hot_log() { crate::pdx::serial_println!("[sexfiles.route.dispatch] op=0x38 name=write caller={}", caller_pd); }
            let reply = handle_diskfs_write(arg0, arg1, arg2, caller_pd);
            if hot_log() { crate::pdx::serial_println!("[sexfiles.route.reply] op=0x38 caller={} value={:#x}", caller_pd, reply); }
            reply
        }
        messages::OP_DISKFS_READ => {
            // arg0 = byte_offset, arg1 = max_len, arg2 = 0 (reserved)
            if hot_log() { crate::pdx::serial_println!("[sexfiles.route.dispatch] op=0x39 name=read caller={}", caller_pd); }
            let reply = handle_diskfs_read(arg0, arg1, caller_pd);
            if hot_log() { crate::pdx::serial_println!("[sexfiles.route.reply] op=0x39 caller={} value={:#x}", caller_pd, reply); }
            reply
        }
        messages::OP_DISKFS_FLUSH => {
            if hot_log() { crate::pdx::serial_println!("[sexfiles.route.dispatch] op=0x3A name=flush caller={}", caller_pd); }
            let reply = handle_diskfs_flush();
            if hot_log() { crate::pdx::serial_println!("[sexfiles.route.reply] op=0x3A caller={} value={:#x}", caller_pd, reply); }
            reply
        }
        messages::OP_DISKFS_STAT => {
            if hot_log() { crate::pdx::serial_println!("[sexfiles.route.dispatch] op=0x3B name=stat caller={}", caller_pd); }
            let reply = handle_diskfs_stat(caller_pd);
            if hot_log() { crate::pdx::serial_println!("[sexfiles.route.reply] op=0x3B caller={} value={:#x}", caller_pd, reply); }
            reply
        }
        messages::OP_DISKFS_MANIFEST_HASH => {
            if hot_log() { crate::pdx::serial_println!("[sexfiles.route.dispatch] op=0x3C name=hash caller={}", caller_pd); }
            let reply = handle_diskfs_manifest_hash(caller_pd);
            if hot_log() { crate::pdx::serial_println!("[sexfiles.route.reply] op=0x3C caller={} value={:#x}", caller_pd, reply); }
            reply
        }
        messages::OP_DISKFS_CREATE => {
            crate::pdx::serial_println!("[sexfiles.route.dispatch] op=0x42 name=create caller={}", caller_pd);
            let buf_va = diskfs_bridge_get_buf_va();
            if buf_va == 0 || buf_va == u64::MAX { return messages::ERR_NOT_FOUND as u64; }
            if let Err(e) = v3_ensure(buf_va) { return e; }
            let reply = handle_diskfs_create(arg1, arg2, buf_va);
            crate::pdx::serial_println!("[sexfiles.route.reply] op=0x42 caller={} value={:#x}", caller_pd, reply);
            reply
        }
        messages::OP_DISKFS_LIST => {
            if hot_log() { crate::pdx::serial_println!("[sexfiles.route.dispatch] op=0x43 name=list caller={}", caller_pd); }
            let buf_va = diskfs_bridge_get_buf_va();
            if buf_va == 0 || buf_va == u64::MAX { return messages::ERR_NOT_FOUND as u64; }
            if let Err(e) = v3_ensure(buf_va) { return e; }
            handle_diskfs_list(arg0, arg1)
        }
        messages::OP_DISKFS_DELETE => {
            crate::pdx::serial_println!("[sexfiles.route.dispatch] op=0x47 name=delete caller={}", caller_pd);
            let buf_va = diskfs_bridge_get_buf_va();
            if buf_va == 0 || buf_va == u64::MAX { return messages::ERR_NOT_FOUND as u64; }
            if let Err(e) = v3_ensure(buf_va) { return e; }
            handle_diskfs_delete(arg0, buf_va)
        }
        messages::OP_DISKFS_RENAME => {
            crate::pdx::serial_println!("[sexfiles.route.dispatch] op=0x48 name=rename caller={}", caller_pd);
            let buf_va = diskfs_bridge_get_buf_va();
            if buf_va == 0 || buf_va == u64::MAX { return messages::ERR_NOT_FOUND as u64; }
            if let Err(e) = v3_ensure(buf_va) { return e; }
            handle_diskfs_rename(arg0, arg1, arg2, buf_va)
        }
        messages::OP_DISKFS_SELECT => {
            // arg0 = path_id, arg1/arg2 = 0 (reserved)
            if hot_log() { crate::pdx::serial_println!("[sexfiles.route.dispatch] op=0x3E name=select caller={}", caller_pd); }
            let reply = handle_diskfs_select(arg0, caller_pd);
            if hot_log() { crate::pdx::serial_println!("[sexfiles.route.reply] op=0x3E caller={} value={:#x}", caller_pd, reply); }
            reply
        }

        messages::OP_SEXOBJECT_NATIVE_PERSIST_PROOF => {
            if hot_log() { crate::pdx::serial_println!(
                "[sexfiles.route.dispatch] op=0x40 ok=1 caller={}",
                caller_pd,
            ); }
            match crate::backends::diskfs::sexobject_native_persist_linen_proof() {
                Ok(object_id) => {
                    if hot_log() { crate::pdx::serial_println!(
                        "[sexfiles.route.reply] op=0x40 status=S ok=1 caller={} object_id={}",
                        caller_pd, object_id,
                    ); }
                    object_id
                }
                Err(e) => {
                    if hot_log() { crate::pdx::serial_println!(
                        "[sexfiles.route.reply] op=0x40 status=E ok=0 caller={} err={}",
                        caller_pd, e,
                    ); }
                    e as u64
                }
            }
        }

        messages::OP_SEXOBJECT_READ_BACK => {
            // arg0 = object_id, arg1/arg2 = 0 (reserved)
            if hot_log() { crate::pdx::serial_println!(
                "[sexfiles.route.dispatch] op=0x41 name=sexobject_read_back caller={}",
                caller_pd
            ); }
            match crate::backends::diskfs::sexobject_read_back_for_quil(arg0) {
                Ok(len) => {
                    if hot_log() { crate::pdx::serial_println!(
                        "[sexfiles.route.reply] op=0x41 caller={} len={} ok=1",
                        caller_pd, len
                    ); }
                    // Return len as positive value for the caller to verify
                    len as u64
                }
                Err(e) => {
                    if hot_log() { crate::pdx::serial_println!(
                        "[sexfiles.route.reply] op=0x41 caller={} err={}",
                        caller_pd, e
                    ); }
                    e as u64
                }
            }
        }

        messages::OP_RAMFS_STATUS => {
            // Phase B1: object status query by object_id
            // arg0 = object_id, arg1/arg2 = 0
            crate::pdx::serial_println!("[sexfiles.status.query] object={} ok=1 reason=received", arg0);
            // Query backend for object existence
            // RamFS backend: check if any file has this object_id
            let exists = false; // stub — full lookup deferred
            let size: u64 = 0;
            let generation: u64 = 0;
            crate::pdx::serial_println!("[sexfiles.status.result] object={} exists={} size={} generation={} ok=1 reason=phaseb1_marker",
                arg0, exists as u8, size, generation);
            0 // fire-and-forget, no meaningful reply
        }

        _ => messages::ERR_NOT_FOUND as u64,
    }
}

/// Unpack a name from three u64 args.
/// Name is stored little-endian in arg0..arg2, up to 24 bytes,
/// zero-padded. Returns the actual name slice (strips trailing zeros).
fn unpack_name(arg0: u64, arg1: u64, arg2: u64) -> alloc::vec::Vec<u8> {
    let mut name = alloc::vec::Vec::with_capacity(messages::RAMFS_MAX_NAME);
    let bytes0 = arg0.to_le_bytes();
    let bytes1 = arg1.to_le_bytes();
    let bytes2 = arg2.to_le_bytes();
    name.extend_from_slice(&bytes0);
    name.extend_from_slice(&bytes1);
    // First 24 bytes of arg2 are name; rest may be flags
    name.extend_from_slice(&bytes2[..8]);
    // Strip trailing zeros (but preserve embedded zeros as valid name bytes)
    while name.last() == Some(&0) {
        name.pop();
    }
    name
}
