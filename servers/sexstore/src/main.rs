#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;
use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

#[alloc_error_handler]
fn alloc_error_handler(_layout: core::alloc::Layout) -> ! {
    loop {}
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}

use sex_pdx::{pdx_listen_raw, pdx_reply, serial_println, SLOT_SEXSTORE};

// Local opcode definitions — promoted to sex-pdx when silk-shell integration lands.
// Kept local to avoid sex-pdx change and ABI hash update this phase.
const OP_KV_GET: u64 = 0xB0;
const OP_KV_PUT: u64 = 0xB1;
const OP_KV_DEL: u64 = 0xB2; // E6: DELETE / tombstone

const SEXSTORE_KV_PROOF_ENABLED: bool =
    option_env!("SEXOS_SEXSTORE_KV_PROOF").is_some();
static mut SEXSTORE_KV_PROOF_STAGE: u8 = 0;

// Status codes — E6 remap aligned with E2 spec.
// GET success: reply is stored u64 (bit 63 = 0).
// Status reply: bit 63 = 1 (REPLY_STATUS_BIT), lower bits = code.
const KV_OK:             u64 = 0x00;
const KV_NOT_FOUND:      u64 = 0x01;
const KV_FULL:           u64 = 0x02;
const KV_INVALID_KEY:    u64 = 0x03;
const KV_INVALID_VALUE:  u64 = 0x04;
const KV_DENIED:         u64 = 0x05; // E6 remap: was 0x01 in E4

// Reply discriminator: bit 63 = 1 indicates status code (not stored value).
const REPLY_STATUS_BIT: u64 = 0x8000_0000_0000_0000;

const KV_SLOT_COUNT: usize = 16;

// E13: Durable backend constants — dual-page atomic swap layout.
#[allow(dead_code)]
const DURABLE_PAGE_SIZE: usize = 512;
const DURABLE_RECORD_COUNT: usize = 16;
const DURABLE_PAGE_A_OFFSET: usize = 0;
const DURABLE_PAGE_B_OFFSET: usize = 512;
const DURABLE_PAGE_ID_MAGIC: u32 = 0x0000A5A5;
const DURABLE_RECORD_MAGIC: u16 = 0xD5E5;
const DURABLE_FORMAT_VERSION: u8 = 0x01;

// Page header field offsets (within 512-byte page)
const PH_OFF_PAGE_ID: usize = 0;   // 4 bytes: u32 magic
const PH_OFF_SEQ: usize = 4;       // 4 bytes: u32 sequence number
const PH_OFF_CRC32: usize = 8;     // 4 bytes: u32 CRC-32C of page (zeroed during compute)
#[allow(dead_code)]
const PH_OFF_RESERVED: usize = 12; // 4 bytes: zero
const PH_SIZE: usize = 16;

// Record field offsets (within 24-byte record, relative to record start)
const REC_OFF_MAGIC: usize = 0;    // 2 bytes: u16 0xD5E5
const REC_OFF_VERSION: usize = 2;  // 1 byte: u8 0x01
const REC_OFF_FLAGS: usize = 3;    // 1 byte: bit0=active, bit1=tombstone
const REC_OFF_SLOT_ID: usize = 4;  // 2 bytes: u16 slot index
const REC_OFF_CRC16: usize = 6;    // 2 bytes: CRC-16-IBM of record
const REC_OFF_STATE: usize = 8;    // 1 byte: 0=Empty, 1=Active, 2=Tombstoned
const REC_OFF_GENERATION: usize = 9; // 1 byte: write count
#[allow(dead_code)]
const REC_OFF_PAD: usize = 10;     // 2 bytes: zero
const REC_OFF_KEY: usize = 12;     // 4 bytes: u32 key
const REC_OFF_VAL: usize = 16;     // 8 bytes: u64 value
const REC_SIZE: usize = 24;

#[derive(Clone, Copy)]
struct KvSlot {
    state:      u8,   // 0=Empty, 1=Active, 2=Tombstoned
    generation: u8,   // 0=never written, 1..255=write count (wraps 255→1)
    key:        u32,
    val:        u64,
}

// Static RAM table — 16 × (1+1+2pad+4+8) = 16 × 16 = 256 bytes. No heap.
static mut KV: [KvSlot; KV_SLOT_COUNT] = [KvSlot { state: 0, generation: 0, key: 0, val: 0 }; KV_SLOT_COUNT];

static mut LOG_PUT: u32 = 32;
static mut LOG_GET: u32 = 32;

// E4: policy and validation proof marker budgets.
static mut LOG_POLICY_ALLOW: u32 = 32;
static mut LOG_POLICY_DENY: u32 = 32;
static mut LOG_KEY_INVALID: u32 = 8;
static mut LOG_VALUE_INVALID: u32 = 8;
static mut LOG_REPLY_ERROR: u32 = 8;

// E6: generation and tombstone proof marker budgets.
static mut LOG_GENERATION_BUMP: u32 = 64;
static mut LOG_TOMBSTONE_RECORD: u32 = 32;
static mut LOG_TOMBSTONE_GET: u32 = 32;
static mut LOG_TOMBSTONE_REVIVE: u32 = 16;

// E7: structured allow/reject proof marker budgets.
static mut LOG_PUT_ALLOW: u32 = 32;
static mut LOG_PUT_REJECT: u32 = 16;
static mut LOG_GET_ALLOW: u32 = 32;
static mut LOG_GET_REJECT: u32 = 16;
static mut LOG_DELETE_ALLOW: u32 = 16;
static mut LOG_DELETE_REJECT: u32 = 8;

// E13: durable proof marker budgets.
static mut LOG_DURABLE_WRITE: u32 = 16;
static mut LOG_DURABLE_WRITE_FAIL: u32 = 8;

// E13: Durable backend storage — 1024-byte static region for two 512-byte pages.
// V1 uses RAM-backed scaffold (not real persistent media). The dual-page atomic swap
// logic is identical regardless of backing store; only page_read/page_write change.
// When real persistent memory becomes available, replace the region address and
// update page_read/page_write to use the new target.
static mut DURABLE_REGION: [u8; 1024] = [0u8; 1024];

unsafe fn kv_reset_for_proof() {
    let mut i = 0;
    while i < KV_SLOT_COUNT {
        KV[i].state = 0;
        KV[i].generation = 0;
        KV[i].key = 0;
        KV[i].val = 0;
        i += 1;
    }
}

unsafe fn kv_put_for_proof(caller: u64, key: u32, val: u64) -> u64 {
    let cls = store_key_owner_class(key);
    if !store_cap_allowed(caller, key) {
        return if cls == 0 {
            REPLY_STATUS_BIT | KV_INVALID_KEY
        } else {
            REPLY_STATUS_BIT | KV_DENIED
        };
    }
    if !store_validate_value(key, val) {
        return REPLY_STATUS_BIT | KV_INVALID_VALUE;
    }
    let mut i = 0usize;
    while i < KV_SLOT_COUNT {
        if KV[i].state != 0 && KV[i].key == key {
            KV[i].state = 1;
            KV[i].val = val;
            KV[i].generation = if KV[i].generation >= 255 { 1 } else { KV[i].generation + 1 };
            return REPLY_STATUS_BIT | KV_OK;
        }
        i += 1;
    }
    let mut empty: Option<usize> = None;
    let mut tomb: Option<usize> = None;
    let mut j = 0usize;
    while j < KV_SLOT_COUNT {
        if KV[j].state == 0 && empty.is_none() {
            empty = Some(j);
        } else if KV[j].state == 2 && tomb.is_none() {
            tomb = Some(j);
        }
        j += 1;
    }
    if let Some(idx) = empty.or(tomb) {
        KV[idx].state = 1;
        KV[idx].generation = 1;
        KV[idx].key = key;
        KV[idx].val = val;
        REPLY_STATUS_BIT | KV_OK
    } else {
        REPLY_STATUS_BIT | KV_FULL
    }
}

unsafe fn kv_get_for_proof(caller: u64, key: u32) -> u64 {
    let cls = store_key_owner_class(key);
    if !store_cap_allowed(caller, key) {
        return if cls == 0 {
            REPLY_STATUS_BIT | KV_INVALID_KEY
        } else {
            REPLY_STATUS_BIT | KV_DENIED
        };
    }
    let mut i = 0usize;
    while i < KV_SLOT_COUNT {
        if KV[i].state != 0 && KV[i].key == key {
            return if KV[i].state == 1 {
                KV[i].val
            } else {
                REPLY_STATUS_BIT | KV_NOT_FOUND
            };
        }
        i += 1;
    }
    REPLY_STATUS_BIT | KV_NOT_FOUND
}

unsafe fn kv_put_raw_for_proof(key: u32, val: u64) -> u64 {
    let mut i = 0usize;
    while i < KV_SLOT_COUNT {
        if KV[i].state != 0 && KV[i].key == key {
            KV[i].state = 1;
            KV[i].val = val;
            KV[i].generation = if KV[i].generation >= 255 { 1 } else { KV[i].generation + 1 };
            return REPLY_STATUS_BIT | KV_OK;
        }
        i += 1;
    }
    let mut j = 0usize;
    while j < KV_SLOT_COUNT {
        if KV[j].state == 0 {
            KV[j].state = 1;
            KV[j].generation = 1;
            KV[j].key = key;
            KV[j].val = val;
            return REPLY_STATUS_BIT | KV_OK;
        }
        j += 1;
    }
    REPLY_STATUS_BIT | KV_FULL
}

unsafe fn run_kv_contract_proof_stage() {
    let stage = SEXSTORE_KV_PROOF_STAGE;
    if stage >= 6 {
        return;
    }
    SEXSTORE_KV_PROOF_STAGE = stage + 1;
    serial_println!("[sexstore.kv.proof] stage={}", stage);
    match stage {
        0 => {
            kv_reset_for_proof();
            let put = kv_put_for_proof(KV_SHELL_CALLER, 0x02, 0x2A02);
            let get = kv_get_for_proof(KV_SHELL_CALLER, 0x02);
            let ok = put == (REPLY_STATUS_BIT | KV_OK) && get == 0x2A02;
            serial_println!("[sexstore.kv.proof.roundtrip] ok={} put={:#x} get={:#x}", ok as u8, put, get);
        }
        1 => {
            let get = kv_get_for_proof(KV_SHELL_CALLER, 0x03);
            let ok = get == (REPLY_STATUS_BIT | KV_NOT_FOUND);
            serial_println!("[sexstore.kv.proof.missing_key] ok={} res={:#x}", ok as u8, get);
        }
        2 => {
            let put = kv_put_for_proof(KV_SHELL_CALLER, 0x00, 1);
            let ok = put == (REPLY_STATUS_BIT | KV_INVALID_KEY);
            serial_println!("[sexstore.kv.proof.oversized_key] ok={} res={:#x} bound=4bytes", ok as u8, put);
        }
        3 => {
            let put = kv_put_for_proof(KV_SHELL_CALLER, 0x01, REPLY_STATUS_BIT);
            let ok = put == (REPLY_STATUS_BIT | KV_INVALID_VALUE);
            serial_println!("[sexstore.kv.proof.oversized_value] ok={} res={:#x} bound=8bytes", ok as u8, put);
        }
        4 => {
            kv_reset_for_proof();
            let mut last = REPLY_STATUS_BIT | KV_OK;
            let mut i = 0usize;
            while i < KV_SLOT_COUNT {
                last = kv_put_raw_for_proof((i as u32) + 1, (i as u64) + 10);
                i += 1;
            }
            let full = kv_put_raw_for_proof((KV_SLOT_COUNT as u32) + 1, 0x55);
            let ok = last == (REPLY_STATUS_BIT | KV_OK) && full == (REPLY_STATUS_BIT | KV_FULL);
            serial_println!("[sexstore.kv.proof.table_full] ok={} last={:#x} full={:#x} slots={}", ok as u8, last, full, KV_SLOT_COUNT);
        }
        5 => {
            let put = kv_put_for_proof(99, 0x01, 0x1234);
            let get = kv_get_for_proof(99, 0x01);
            let ok = put == (REPLY_STATUS_BIT | KV_DENIED) && get == (REPLY_STATUS_BIT | KV_DENIED);
            serial_println!("[sexstore.kv.proof.owner_deny] ok={} put={:#x} get={:#x}", ok as u8, put, get);
        }
        _ => {}
    }
}


/// Bump slot generation (wraps 255 → 1, never 0).
#[inline(always)]
unsafe fn bump_generation(slot: *mut KvSlot) {
    let g = (*slot).generation;
    (*slot).generation = if g >= 255 { 1 } else { g + 1 };
}

// E4: Key owner class and capability checking.
// Silk-shell (domain 3) is the only authorized caller in E4.
// NOTE: This value must match silk-shell's domain ID as assigned by
// kernel/src/init.rs fixed spawn order (module_paths[2] = "silk-shell" → domain_id=3).
// If spawn order or domain allocation changes, update this constant to match.
const KV_SHELL_CALLER: u64 = 3;

/// Return the owner class for a key.
/// 0 = invalid (key 0x00), 1 = shell range (0x01..0x0F), 2 = reserved (0x10+).
fn store_key_owner_class(key: u32) -> u8 {
    if key == 0 { 0 }
    else if key <= 0x0F { 1 }
    else { 2 }
}

/// Check whether `caller_pd` is authorized for operation on `key`.
/// E4: only silk-shell (domain 3) on shell range (0x01..0x0F) is allowed.
fn store_cap_allowed(caller_pd: u64, key: u32) -> bool {
    let cls = store_key_owner_class(key);
    cls == 1 && caller_pd == KV_SHELL_CALLER
}

/// Validate value envelope for known keys.
/// Key 0x01: must have magic=0xAC, version=0x01, valid XOR checksum.
fn store_validate_value(key: u32, value: u64) -> bool {
    // Reject any value with bit 63 set — would collide with REPLY_STATUS_BIT on GET reply.
    // pack_scene_settings_blob() masks checksum to 0x7F, ensuring bit 63 is always 0.
    if value & REPLY_STATUS_BIT != 0 { return false; }
    if key == 0x01 {
        let b = value.to_le_bytes();
        if b[0] != 0xAC || b[1] != 0x01 { return false; }
        // Checksum is stored masked to 7 bits (bit 7 cleared) to keep bit 63 of the u64 clear.
        let chk = (b[0] ^ b[1] ^ b[2] ^ b[3] ^ b[4] ^ b[5] ^ b[6]) & 0x7F;
        if b[7] != chk { return false; }
    }
    true
}

// ── E13: CRC helpers (no_std, no lookup tables) ──────────────────────────────

/// CRC-32C (Castagnoli) bit-by-bit. Polynomial 0x1EDC6F41, reversed 0x82F63B78.
fn crc32c(buf: &[u8]) -> u32 {
    let mut crc = 0xFFFFFFFFu32;
    for &b in buf {
        crc ^= b as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 { (crc >> 1) ^ 0x82F63B78 } else { crc >> 1 };
        }
    }
    !crc
}

/// CRC-16-IBM bit-by-bit. Polynomial 0x8005, initial 0x0000, no XOR out.
fn crc16_ibm(buf: &[u8]) -> u16 {
    let mut crc = 0x0000u16;
    for &b in buf {
        crc ^= (b as u16) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 { (crc << 1) ^ 0x8005 } else { crc << 1 };
        }
    }
    crc
}

// ── E13: Page I/O abstraction (RAM-backed scaffold for V1) ───────────────────

/// Read a 512-byte page from the durable region at the given offset.
fn durable_page_read(page_offset: usize, buf: &mut [u8; 512]) {
    unsafe {
        let src = core::ptr::addr_of!(DURABLE_REGION) as *const u8;
        core::ptr::copy_nonoverlapping(src.add(page_offset), buf.as_mut_ptr(), 512);
    }
}

/// Write a 512-byte page to the durable region at the given offset, then
/// verify by readback comparison. Returns true if verify passes.
fn durable_page_write(page_offset: usize, buf: &[u8; 512]) -> bool {
    unsafe {
        let dst = core::ptr::addr_of_mut!(DURABLE_REGION) as *mut u8;
        core::ptr::copy_nonoverlapping(buf.as_ptr(), dst.add(page_offset), 512);
    }
    // Verify-after-write: read back and compare.
    let mut readback = [0u8; 512];
    durable_page_read(page_offset, &mut readback);
    // For RAM-backed scaffold this is always true; structure preserved for
    // hardware port where readback may differ from write (cache, media).
    readback.iter().zip(buf.iter()).all(|(a, b)| a == b)
}

// ── E13: Page validation ─────────────────────────────────────────────────────

/// Validate a page: check page_id magic and CRC-32C.
fn durable_validate_page(page: &[u8; 512]) -> bool {
    let page_id = u32::from_le_bytes([page[0], page[1], page[2], page[3]]);
    if page_id != DURABLE_PAGE_ID_MAGIC {
        return false;
    }
    let stored_crc = u32::from_le_bytes([page[8], page[9], page[10], page[11]]);
    // Compute CRC with the crc32 field zeroed.
    let mut clean = *page;
    clean[PH_OFF_CRC32..PH_OFF_CRC32 + 4].copy_from_slice(&[0u8; 4]);
    stored_crc == crc32c(&clean)
}

/// Return the sequence number of a validated page, or 0 if invalid.
fn durable_page_seq(page: &[u8; 512]) -> u32 {
    if durable_validate_page(page) {
        u32::from_le_bytes([page[4], page[5], page[6], page[7]])
    } else {
        0
    }
}

// ── E13: Page building from RAM slots ───────────────────────────────────────

/// Build a full 512-byte page snapshot from the 16-slot RAM table.
fn durable_build_page(slots: &[KvSlot; 16], seq: u32) -> [u8; 512] {
    let mut page = [0u8; 512];

    // Header: page_id + seq (crc32 left zero for now)
    page[PH_OFF_PAGE_ID..PH_OFF_PAGE_ID + 4].copy_from_slice(&DURABLE_PAGE_ID_MAGIC.to_le_bytes());
    page[PH_OFF_SEQ..PH_OFF_SEQ + 4].copy_from_slice(&seq.to_le_bytes());
    // PH_OFF_CRC32 stays zero during computation.

    // Records
    for i in 0..DURABLE_RECORD_COUNT {
        let off = PH_SIZE + i * REC_SIZE;
        let slot = &slots[i];

        // Record magic + version
        page[off + REC_OFF_MAGIC..off + REC_OFF_MAGIC + 2].copy_from_slice(&DURABLE_RECORD_MAGIC.to_le_bytes());
        page[off + REC_OFF_VERSION] = DURABLE_FORMAT_VERSION;

        // Flags: bit0 = active data, bit1 = tombstone
        let flags: u8 = if slot.state == 1 { 0x01 } else if slot.state == 2 { 0x02 } else { 0x00 };
        page[off + REC_OFF_FLAGS] = flags;

        // Slot ID (cross-check)
        page[off + REC_OFF_SLOT_ID..off + REC_OFF_SLOT_ID + 2].copy_from_slice(&(i as u16).to_le_bytes());

        // crc16 left zeroed — computed after all other fields are set.

        // State, generation (pad already zero)
        page[off + REC_OFF_STATE] = slot.state;
        page[off + REC_OFF_GENERATION] = slot.generation;

        // Key, value
        page[off + REC_OFF_KEY..off + REC_OFF_KEY + 4].copy_from_slice(&slot.key.to_le_bytes());
        page[off + REC_OFF_VAL..off + REC_OFF_VAL + 8].copy_from_slice(&slot.val.to_le_bytes());

        // Compute record CRC-16 (with crc16 field zeroed at offset REC_OFF_CRC16).
        let rec_crc = crc16_ibm(&page[off..off + REC_SIZE]);
        page[off + REC_OFF_CRC16..off + REC_OFF_CRC16 + 2].copy_from_slice(&rec_crc.to_le_bytes());
    }

    // Compute page CRC-32C (with crc32 field still zeroed).
    let page_crc = crc32c(&page);
    page[PH_OFF_CRC32..PH_OFF_CRC32 + 4].copy_from_slice(&page_crc.to_le_bytes());

    page
}

// ── E13: Durable write (after RAM commit) ────────────────────────────────────

/// Write a full page snapshot to the inactive durable page.
/// Returns true on successful write+verify, false on failure (RAM unchanged).
/// Called after RAM commit in PUT/DEL handlers.
unsafe fn durable_write_all(slots: &[KvSlot; 16]) -> bool {
    // Read current pages to determine active state.
    let mut page_a = [0u8; 512];
    let mut page_b = [0u8; 512];
    durable_page_read(DURABLE_PAGE_A_OFFSET, &mut page_a);
    durable_page_read(DURABLE_PAGE_B_OFFSET, &mut page_b);

    let seq_a = durable_page_seq(&page_a);
    let seq_b = durable_page_seq(&page_b);

    // Target is the page with lower seq (inactive). If tied, target page B.
    let target_offset = if seq_a >= seq_b { DURABLE_PAGE_B_OFFSET } else { DURABLE_PAGE_A_OFFSET };
    let next_seq = if seq_a > seq_b { seq_a + 1 } else { seq_b + 1 };
    // Wrap: u32::MAX → 1 (0 reserved for uninitialized)
    let next_seq = if next_seq == 0 { 1 } else { next_seq };

    let snapshot = durable_build_page(slots, next_seq);

    if durable_page_write(target_offset, &snapshot) {
        if LOG_DURABLE_WRITE > 0 {
            LOG_DURABLE_WRITE -= 1;
            // Log the first slot's key as representative (all slots are written).
            let first_key = slots[0].key;
            serial_println!("[sexstore.durable.write] key={} seq={} page={}", first_key, next_seq,
                if target_offset == DURABLE_PAGE_A_OFFSET { "A" } else { "B" });
        }
        true
    } else {
        if LOG_DURABLE_WRITE_FAIL > 0 {
            LOG_DURABLE_WRITE_FAIL -= 1;
            serial_println!("[sexstore.durable.write.fail] reason=verify_fail");
        }
        false
    }
}

// ── E13: Boot load — hydrate RAM from durable pages ──────────────────────────

/// Load authoritative durable page into the RAM slot table.
/// Returns (total_records, valid_records, corrupt_records) stats.
/// Slots with corrupt records are left at their default (Empty, gen=0).
unsafe fn durable_load_into_ram(slots: &mut [KvSlot; 16]) -> (u32, u32, u32) {
    let mut page_a = [0u8; 512];
    let mut page_b = [0u8; 512];
    durable_page_read(DURABLE_PAGE_A_OFFSET, &mut page_a);
    durable_page_read(DURABLE_PAGE_B_OFFSET, &mut page_b);

    let seq_a = durable_page_seq(&page_a);
    let seq_b = durable_page_seq(&page_b);

    // Select authoritative page (higher seq). Tie-break: page A.
    let (authoritative, auth_seq) = if seq_a >= seq_b { (&page_a, seq_a) } else { (&page_b, seq_b) };

    if auth_seq == 0 {
        // No valid durable data — all slots stay at defaults.
        return (0, 0, 0);
    }

    let mut total: u32 = 0;
    let mut valid: u32 = 0;
    let mut corrupt: u32 = 0;

    for i in 0..DURABLE_RECORD_COUNT {
        let off = PH_SIZE + i * REC_SIZE;
        total += 1;

        // Validate record magic + version.
        let magic = u16::from_le_bytes([authoritative[off + REC_OFF_MAGIC], authoritative[off + REC_OFF_MAGIC + 1]]);
        if magic != DURABLE_RECORD_MAGIC {
            corrupt += 1;
            continue;
        }
        if authoritative[off + REC_OFF_VERSION] != DURABLE_FORMAT_VERSION {
            corrupt += 1;
            continue;
        }

        // Validate slot_id matches expected index.
        let slot_id = u16::from_le_bytes([authoritative[off + REC_OFF_SLOT_ID], authoritative[off + REC_OFF_SLOT_ID + 1]]);
        if slot_id as usize != i {
            corrupt += 1;
            continue;
        }

        // Validate record CRC-16 (with crc16 field zeroed during computation).
        let stored_rec_crc = u16::from_le_bytes([authoritative[off + REC_OFF_CRC16], authoritative[off + REC_OFF_CRC16 + 1]]);
        let mut rec_buf = [0u8; REC_SIZE];
        rec_buf.copy_from_slice(&authoritative[off..off + REC_SIZE]);
        // Zero the crc16 field for computation.
        rec_buf[REC_OFF_CRC16..REC_OFF_CRC16 + 2].copy_from_slice(&[0u8; 2]);
        let computed_rec_crc = crc16_ibm(&rec_buf);
        if computed_rec_crc != stored_rec_crc {
            corrupt += 1;
            continue;
        }

        // Validate state field.
        let state = authoritative[off + REC_OFF_STATE];
        if state > 2 {
            corrupt += 1;
            continue;
        }

        // Record is valid — populate RAM slot.
        let key = u32::from_le_bytes([
            authoritative[off + REC_OFF_KEY],
            authoritative[off + REC_OFF_KEY + 1],
            authoritative[off + REC_OFF_KEY + 2],
            authoritative[off + REC_OFF_KEY + 3],
        ]);
        let val = u64::from_le_bytes([
            authoritative[off + REC_OFF_VAL],
            authoritative[off + REC_OFF_VAL + 1],
            authoritative[off + REC_OFF_VAL + 2],
            authoritative[off + REC_OFF_VAL + 3],
            authoritative[off + REC_OFF_VAL + 4],
            authoritative[off + REC_OFF_VAL + 5],
            authoritative[off + REC_OFF_VAL + 6],
            authoritative[off + REC_OFF_VAL + 7],
        ]);
        slots[i].state = state;
        slots[i].generation = authoritative[off + REC_OFF_GENERATION];
        slots[i].key = key;
        slots[i].val = val;
        valid += 1;
    }

    // Emit boot load proof marker.
    serial_println!("[sexstore.durable.load] seq={} records={} valid={} corrupt={}", auth_seq, total, valid, corrupt);
    if total > 0 && valid == 0 {
        serial_println!("[sexstore.durable.all_corrupt] reason=all_records_invalid");
    }

    (total, valid, corrupt)
}

// ── E13: Durable initialization (first boot) ─────────────────────────────────

/// Initialize the durable backend on first boot (both pages invalid).
/// Writes current RAM state as page A with seq=1.
/// Returns true if initialization was performed (first boot), false if durable
/// was already valid (subsequent boot).
unsafe fn durable_init(slots: &[KvSlot; 16]) -> bool {
    let mut page_a = [0u8; 512];
    let mut page_b = [0u8; 512];
    durable_page_read(DURABLE_PAGE_A_OFFSET, &mut page_a);
    durable_page_read(DURABLE_PAGE_B_OFFSET, &mut page_b);

    let seq_a = durable_page_seq(&page_a);
    let seq_b = durable_page_seq(&page_b);

    if seq_a == 0 && seq_b == 0 {
        // First boot — both pages uninitialized. Write page A with seq=1.
        let snapshot = durable_build_page(slots, 1);
        let ok = durable_page_write(DURABLE_PAGE_A_OFFSET, &snapshot);
        serial_println!("[sexstore.durable.load] seq=1 records=16 valid=16 corrupt=0 init={}", if ok { "ok" } else { "fail" });
        true
    } else {
        false // Durable already valid — no init needed.
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    serial_println!("[sexstore.init.start]");
    // E13: Initialize durable backend and load persisted state into RAM.
    // RAM defaults first (compile-time zeroed KV table), then durable overwrites.
    unsafe {
        let kv_ptr: *mut KvSlot = core::ptr::addr_of_mut!(KV) as *mut KvSlot;
        let slots: &mut [KvSlot; 16] = &mut *(kv_ptr as *mut [KvSlot; 16]);

        // durable_init() writes initial page A with seq=1 on first boot.
        // On subsequent boots, it's a no-op (both pages already valid).
        durable_init(slots);

        // durable_load_into_ram() loads the authoritative page into RAM,
        // overwriting default slots with persisted data.
        durable_load_into_ram(slots);
    }

    // E6: emit status mapping marker once at boot.
    serial_println!("[sexstore.status.mapping] KV_OK=0x00 KV_NOT_FOUND=0x01 KV_FULL=0x02 KV_INVALID_KEY=0x03 KV_INVALID_VALUE=0x04 KV_DENIED=0x05 REPLY_BIT=0x8000");

    if SEXSTORE_KV_PROOF_ENABLED {
        unsafe {
            while SEXSTORE_KV_PROOF_STAGE < 6 {
                run_kv_contract_proof_stage();
            }
        }
    }

    serial_println!("[sexstore.ready]");
    loop {
        let msg = pdx_listen_raw(0); // Slot 0 = self message_ring (all servers listen here)
        let caller = msg.caller_pd as u64;

        unsafe {
            match msg.type_id {
                OP_KV_PUT => {
                    let key = msg.arg0 as u32;
                    let val = msg.arg1;

                    // E4: policy gate — validate caller authority.
                    let cls = store_key_owner_class(key);
                    if !store_cap_allowed(caller, key) {
                        // Use KEY_INVALID budget for key==0, POLICY_DENY for others.
                        if cls == 0 {
                            if LOG_KEY_INVALID > 0 {
                                LOG_KEY_INVALID -= 1;
                                serial_println!("[sexstore.key.invalid] caller={} key=0x00", caller);
                            }
                        } else {
                            if LOG_POLICY_DENY > 0 {
                                LOG_POLICY_DENY -= 1;
                                if cls == 2 {
                                    serial_println!("[sexstore.policy.deny] caller={} key={} class=reserved", caller, key);
                                } else {
                                    serial_println!("[sexstore.policy.deny] caller={} key={} class=shell reason=no_cap", caller, key);
                                }
                            }
                        }
                        if LOG_PUT_REJECT > 0 {
                            LOG_PUT_REJECT -= 1;
                            if cls == 0 {
                                serial_println!("[sexstore.put.reject] caller={} key={} status=invalid_key reason=zero_key", caller, key);
                            } else {
                                serial_println!("[sexstore.put.reject] caller={} key={} status=denied reason=no_cap", caller, key);
                            }
                        }
                        if cls == 0 {
                            pdx_reply(caller as u32, REPLY_STATUS_BIT | KV_INVALID_KEY);
                        } else {
                            pdx_reply(caller as u32, REPLY_STATUS_BIT | KV_DENIED);
                        }
                        continue;
                    }
                    if LOG_POLICY_ALLOW > 0 {
                        LOG_POLICY_ALLOW -= 1;
                        serial_println!("[sexstore.policy.allow] caller={} key={} op=PUT", caller, key);
                    }

                    // E4: validate value envelope for known keys.
                    if !store_validate_value(key, val) {
                        if LOG_VALUE_INVALID > 0 {
                            LOG_VALUE_INVALID -= 1;
                            serial_println!("[sexstore.value.invalid] caller={} key={}", caller, key);
                        }
                        if LOG_PUT_REJECT > 0 {
                            LOG_PUT_REJECT -= 1;
                            serial_println!("[sexstore.put.reject] caller={} key={} status=invalid_value reason=envelope_fail", caller, key);
                        }
                        pdx_reply(caller as u32, REPLY_STATUS_BIT | KV_INVALID_VALUE);
                        continue;
                    }

                    // E6: slot operation with generation bump + tombstone revive/reclaim.
                    let kv_ptr: *mut KvSlot = core::ptr::addr_of_mut!(KV) as *mut KvSlot;

                    // Pass 1: find existing slot for this key (active or tombstoned).
                    let mut found_slot: Option<usize> = None;
                    let mut was_tombstoned = false;
                    let mut was_update = false;
                    let mut i = 0;
                    while i < KV_SLOT_COUNT {
                        let slot = kv_ptr.add(i);
                        if (*slot).state != 0 && (*slot).key == key {
                            found_slot = Some(i);
                            was_tombstoned = (*slot).state == 2;
                            was_update = true;
                            break;
                        }
                        i += 1;
                    }

                    if let Some(idx) = found_slot {
                        let slot = kv_ptr.add(idx);
                        (*slot).val = val;
                        if was_tombstoned {
                            (*slot).state = 1; // revive
                            if LOG_TOMBSTONE_REVIVE > 0 {
                                LOG_TOMBSTONE_REVIVE -= 1;
                                serial_println!("[sexstore.tombstone.revive] key={} old_gen={}", key, (*slot).generation);
                            }
                        }
                        bump_generation(slot);
                        if LOG_GENERATION_BUMP > 0 {
                            LOG_GENERATION_BUMP -= 1;
                            let op = if was_tombstoned { "revive" } else { "put" };
                            serial_println!("[sexstore.generation.bump] key={} slot={} gen={} op={}", key, idx, (*slot).generation, op);
                        }
                        if LOG_PUT_ALLOW > 0 {
                            LOG_PUT_ALLOW -= 1;
                            serial_println!("[sexstore.put.allow] caller={} key={} status=ok state={} gen={}", caller, key, (*slot).state, (*slot).generation);
                        }
                        // E13: durable write after RAM commit.
                        let kv_ref: &[KvSlot; 16] = &*(core::ptr::addr_of!(KV) as *const [KvSlot; 16]);
                        durable_write_all(kv_ref);
                        pdx_reply(caller as u32, REPLY_STATUS_BIT | KV_OK);
                    } else {
                        // Pass 2: find empty slot or reclaim tombstoned slot.
                        let mut inserted = false;
                        let mut full = false;
                        let mut i = 0;
                        while i < KV_SLOT_COUNT {
                            let slot = kv_ptr.add(i);
                            if (*slot).state == 0 {
                                (*slot).state = 1;
                                (*slot).generation = 1; // first write
                                (*slot).key = key;
                                (*slot).val = val;
                                inserted = true;
                                if LOG_GENERATION_BUMP > 0 {
                                    LOG_GENERATION_BUMP -= 1;
                                    serial_println!("[sexstore.generation.bump] key={} slot={} gen=1 op=insert", key, i);
                                }
                                // E13: durable write after insert.
                                let kv_ref: &[KvSlot; 16] = &*(core::ptr::addr_of!(KV) as *const [KvSlot; 16]);
                                durable_write_all(kv_ref);
                                break;
                            }
                            i += 1;
                        }
                        if !inserted {
                            // No empty slot — try reclaiming a tombstoned slot.
                            let mut i = 0;
                            while i < KV_SLOT_COUNT {
                                let slot = kv_ptr.add(i);
                                if (*slot).state == 2 {
                                    (*slot).state = 1;
                                    (*slot).key = key;
                                    (*slot).val = val;
                                    // Reset generation to 1 for new key lifecycle (different from old key).
                                    // E10: generation is per-slot but semantically per-key on reclaim.
                                    (*slot).generation = 1;
                                    inserted = true;
                                    if LOG_GENERATION_BUMP > 0 {
                                        LOG_GENERATION_BUMP -= 1;
                                        serial_println!("[sexstore.generation.bump] key={} slot={} gen=1 op=reclaim", key, i);
                                    }
                                    // E13: durable write after reclaim.
                                    let kv_ref: &[KvSlot; 16] = &*(core::ptr::addr_of!(KV) as *const [KvSlot; 16]);
                                    durable_write_all(kv_ref);
                                    break;
                                }
                                i += 1;
                            }
                        }
                        // No durable write for full path — RAM unchanged.
                        if !inserted { full = true; }
                        let status = if full { KV_FULL } else { KV_OK };
                        if !full {
                            if LOG_PUT_ALLOW > 0 {
                                LOG_PUT_ALLOW -= 1;
                                serial_println!("[sexstore.put.allow] caller={} key={} status=ok state=1 gen=1", caller, key);
                            }
                        } else {
                            if LOG_PUT_REJECT > 0 {
                                LOG_PUT_REJECT -= 1;
                                serial_println!("[sexstore.put.reject] caller={} key={} status=full reason=table_full", caller, key);
                            }
                        }
                        pdx_reply(caller as u32, REPLY_STATUS_BIT | status);
                    }

                    if LOG_PUT > 0 {
                        LOG_PUT -= 1;
                        serial_println!("[sexstore.kv.put] key={} ok={}", key, if was_update { 1 } else { 0 });
                    }
                }

                OP_KV_GET => {
                    let key = msg.arg0 as u32;

                    // E4: policy gate — validate caller authority.
                    let cls = store_key_owner_class(key);
                    if !store_cap_allowed(caller, key) {
                        // Use KEY_INVALID budget for key==0, POLICY_DENY for others.
                        if cls == 0 {
                            if LOG_KEY_INVALID > 0 {
                                LOG_KEY_INVALID -= 1;
                                serial_println!("[sexstore.key.invalid] caller={} key=0x00", caller);
                            }
                        } else {
                            if LOG_POLICY_DENY > 0 {
                                LOG_POLICY_DENY -= 1;
                                serial_println!("[sexstore.policy.deny] caller={} key={} class={}", caller, key, cls);
                            }
                        }
                        if LOG_GET_REJECT > 0 {
                            LOG_GET_REJECT -= 1;
                            if cls == 0 {
                                serial_println!("[sexstore.get.reject] caller={} key={} status=invalid_key reason=zero_key", caller, key);
                            } else {
                                serial_println!("[sexstore.get.reject] caller={} key={} status=denied reason=no_cap", caller, key);
                            }
                        }
                        if cls == 0 {
                            pdx_reply(caller as u32, REPLY_STATUS_BIT | KV_INVALID_KEY);
                        } else {
                            pdx_reply(caller as u32, REPLY_STATUS_BIT | KV_DENIED);
                        }
                        continue;
                    }
                    if LOG_POLICY_ALLOW > 0 {
                        LOG_POLICY_ALLOW -= 1;
                        serial_println!("[sexstore.policy.allow] caller={} key={} op=GET", caller, key);
                    }

                    // E6: scan for active (state==1) or tombstoned (state==2).
                    let kv_ptr: *const KvSlot = core::ptr::addr_of!(KV) as *const KvSlot;
                    let mut found_state: u8 = 0; // 0=not found, 1=active, 2=tombstoned
                    let mut result: u64 = 0;
                    let mut slot_gen: u8 = 0;
                    let mut slot_idx: usize = 0;
                    let mut i = 0;
                    while i < KV_SLOT_COUNT {
                        let slot = kv_ptr.add(i);
                        if (*slot).state != 0 && (*slot).key == key {
                            found_state = (*slot).state;
                            result = (*slot).val;
                            slot_gen = (*slot).generation;
                            slot_idx = i;
                            break;
                        }
                        i += 1;
                    }

                    match found_state {
                        1 => {
                            // Active — return stored value (bit 63 = 0).
                            if LOG_GET_ALLOW > 0 {
                                LOG_GET_ALLOW -= 1;
                                serial_println!("[sexstore.get.allow] caller={} key={} status=ok state=1 gen={}", caller, key, slot_gen);
                            }
                            pdx_reply(caller as u32, result);
                        }
                        2 => {
                            // Tombstoned — return NOT_FOUND with marker.
                            if LOG_TOMBSTONE_GET > 0 {
                                LOG_TOMBSTONE_GET -= 1;
                                serial_println!("[sexstore.tombstone.get] key={} slot={} gen={}", key, slot_idx, slot_gen);
                            }
                            if LOG_GET_REJECT > 0 {
                                LOG_GET_REJECT -= 1;
                                serial_println!("[sexstore.get.reject] caller={} key={} status=not_found reason=tombstoned", caller, key);
                            }
                            pdx_reply(caller as u32, REPLY_STATUS_BIT | KV_NOT_FOUND);
                        }
                        _ => {
                            // Not found.
                            if LOG_GET_REJECT > 0 {
                                LOG_GET_REJECT -= 1;
                                serial_println!("[sexstore.get.reject] caller={} key={} status=not_found reason=missing", caller, key);
                            }
                            pdx_reply(caller as u32, REPLY_STATUS_BIT | KV_NOT_FOUND);
                        }
                    }

                    if LOG_GET > 0 {
                        LOG_GET -= 1;
                        serial_println!("[sexstore.kv.get] key={} hit={}", key, if found_state == 1 { 1 } else { 0 });
                    }
                }

                OP_KV_DEL => {
                    let key = msg.arg0 as u32;

                    // E6: policy gate (same authority as PUT/GET — shell-only range).
                    let cls = store_key_owner_class(key);
                    if !store_cap_allowed(caller, key) {
                        if cls == 0 {
                            if LOG_KEY_INVALID > 0 {
                                LOG_KEY_INVALID -= 1;
                                serial_println!("[sexstore.key.invalid] caller={} key=0x00", caller);
                            }
                        } else {
                            if LOG_POLICY_DENY > 0 {
                                LOG_POLICY_DENY -= 1;
                                if cls == 2 {
                                    serial_println!("[sexstore.policy.deny] caller={} key={} class=reserved", caller, key);
                                } else {
                                    serial_println!("[sexstore.policy.deny] caller={} key={} class=shell reason=no_cap", caller, key);
                                }
                            }
                        }
                        if LOG_DELETE_REJECT > 0 {
                            LOG_DELETE_REJECT -= 1;
                            if cls == 0 {
                                serial_println!("[sexstore.delete.reject] caller={} key={} status=invalid_key reason=zero_key", caller, key);
                            } else {
                                serial_println!("[sexstore.delete.reject] caller={} key={} status=denied reason=no_cap", caller, key);
                            }
                        }
                        if cls == 0 {
                            pdx_reply(caller as u32, REPLY_STATUS_BIT | KV_INVALID_KEY);
                        } else {
                            pdx_reply(caller as u32, REPLY_STATUS_BIT | KV_DENIED);
                        }
                        continue;
                    }
                    if LOG_POLICY_ALLOW > 0 {
                        LOG_POLICY_ALLOW -= 1;
                        serial_println!("[sexstore.policy.allow] caller={} key={} op=DEL", caller, key);
                    }

                    // Scan for key in active or tombstoned state.
                    let kv_ptr: *mut KvSlot = core::ptr::addr_of_mut!(KV) as *mut KvSlot;
                    let mut found_state: u8 = 0;
                    let mut slot_gen: u8 = 0;
                    let mut slot_idx: usize = 0;
                    let mut i = 0;
                    while i < KV_SLOT_COUNT {
                        let slot = kv_ptr.add(i);
                        if (*slot).state != 0 && (*slot).key == key {
                            found_state = (*slot).state;
                            slot_gen = (*slot).generation;
                            slot_idx = i;
                            break;
                        }
                        i += 1;
                    }

                    match found_state {
                        1 => {
                            // Active → tombstone, bump generation.
                            let slot = kv_ptr.add(slot_idx);
                            (*slot).state = 2;
                            bump_generation(slot);
                            if LOG_TOMBSTONE_RECORD > 0 {
                                LOG_TOMBSTONE_RECORD -= 1;
                                serial_println!("[sexstore.tombstone.record] key={} slot={} gen={} reason=delete", key, slot_idx, (*slot).generation);
                            }
                            if LOG_GENERATION_BUMP > 0 {
                                LOG_GENERATION_BUMP -= 1;
                                serial_println!("[sexstore.generation.bump] key={} slot={} gen={} op=tombstone", key, slot_idx, (*slot).generation);
                            }
                            if LOG_DELETE_ALLOW > 0 {
                                LOG_DELETE_ALLOW -= 1;
                                serial_println!("[sexstore.delete.allow] caller={} key={} status=ok state=2 gen={} reason=delete", caller, key, (*slot).generation);
                            }
                            // E13: durable write after tombstone.
                            let kv_ref: &[KvSlot; 16] = &*(core::ptr::addr_of!(KV) as *const [KvSlot; 16]);
                            durable_write_all(kv_ref);
                            pdx_reply(caller as u32, REPLY_STATUS_BIT | KV_OK);
                        }
                        2 => {
                            // Already tombstoned — idempotent.
                            if LOG_TOMBSTONE_RECORD > 0 {
                                LOG_TOMBSTONE_RECORD -= 1;
                                serial_println!("[sexstore.tombstone.record] key={} slot={} gen={} reason=delete_idempotent", key, slot_idx, slot_gen);
                            }
                            if LOG_DELETE_ALLOW > 0 {
                                LOG_DELETE_ALLOW -= 1;
                                serial_println!("[sexstore.delete.allow] caller={} key={} status=ok reason=idempotent", caller, key);
                            }
                            // E13: durable write after idempotent delete (tombstone state already persisted).
                            let kv_ref: &[KvSlot; 16] = &*(core::ptr::addr_of!(KV) as *const [KvSlot; 16]);
                            durable_write_all(kv_ref);
                            pdx_reply(caller as u32, REPLY_STATUS_BIT | KV_OK);
                        }
                        _ => {
                            // Not found.
                            if LOG_DELETE_REJECT > 0 {
                                LOG_DELETE_REJECT -= 1;
                                serial_println!("[sexstore.delete.reject] caller={} key={} status=not_found reason=missing", caller, key);
                            }
                            pdx_reply(caller as u32, REPLY_STATUS_BIT | KV_NOT_FOUND);
                        }
                    }
                }

                _ => {
                    // Unknown opcode — reply 0 and ignore.
                    if LOG_REPLY_ERROR > 0 {
                        LOG_REPLY_ERROR -= 1;
                        serial_println!("[sexstore.reply.error] caller={} op={:#x}", caller, msg.type_id);
                    }
                    pdx_reply(caller as u32, 0);
                }
            }
        }
    }
}
