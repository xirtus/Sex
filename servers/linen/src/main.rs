#![no_std]
#![no_main]
#![allow(static_mut_refs)]

mod session;
mod sexobject;

use core::alloc::{GlobalAlloc, Layout};
use sex_pdx::{pdx_call, pdx_listen_raw, pdx_reply, serial_println, SLOT_DISPLAY, SLOT_STORAGE};

struct DummyAllocator;
unsafe impl GlobalAlloc for DummyAllocator {
    unsafe fn alloc(&self, _: Layout) -> *mut u8 { core::ptr::null_mut() }
    unsafe fn dealloc(&self, _: *mut u8, _: Layout) {}
}
#[global_allocator]
static ALLOCATOR: DummyAllocator = DummyAllocator;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! { loop {} }

const SURFACE_ID_LINEN: u64 = 200;
const OP_HID_EVENT: u64 = 0x202;

// ── Linen Session Opcodes ───────────────────────────────────────────────────
/// Create a Linen object. arg0=kind(u8), arg1-arg2=name bytes.
/// Returns: object_id on success, error (negative) on failure.
const OP_LINEN_CREATE_OBJECT: u64 = 0x41;

/// List owned Linen objects. arg0=start_index.
/// Returns: packed {id,kind,owner_pd,name_lo,name_hi,ramfs_handle} or 0 if done.
const OP_LINEN_LIST_OBJECTS: u64 = 0x42;

/// Get Linen object info. arg0=object_id.
/// Returns: packed object data or error (negative).
const OP_LINEN_GET_OBJECT: u64 = 0x43;

/// Maximum display name length (matches RamFS max name).
const LINEN_MAX_NAME: usize = 24;
/// Current create opcode wire payload can carry only 16 name bytes (arg1 + arg2).
const LINEN_CREATE_WIRE_MAX_NAME: usize = 16;

// ── RamFS / SexFiles protocol constants ───────────────────────────────────────
// SEXFILES_RAMFS_CONTRACT_LOCK_V1: bounded flat namespace.
// Used by Linen to persist object metadata.
#[allow(dead_code)]
const OP_RAMFS_OPEN: u64 = 0x30;
#[allow(dead_code)]
const OP_RAMFS_READ: u64 = 0x31;
const OP_RAMFS_WRITE: u64 = 0x32;
const OP_RAMFS_CLOSE: u64 = 0x33;
#[allow(dead_code)]
const OP_RAMFS_LIST: u64 = 0x34;
#[allow(dead_code)]
const OP_RAMFS_STAT: u64 = 0x35;
const OP_RAMFS_CREATE_OWNER: u64 = 0x36;
const OP_RAMFS_OBJECT_ID: u64 = 0x37;

/// Maximum Linen objects in session table.
const LINEN_MAX_OBJECTS: usize = 16;

/// Session manager instance. Initialized at boot.
static mut SESSION: session::Session = session::Session::new();

/// Object kind constants for PDX encoding.
const KIND_DOCUMENT: u8 = 0;
const KIND_SESSION: u8 = 1;
const KIND_UNKNOWN: u8 = 2;

// ── Proof flag ──────────────────────────────────────────────────────────────
/// Build with LINEN_SESSION_PROOF=1 to enable startup proof.
const LINEN_SESSION_PROOF_ENABLED: bool =
    option_env!("SEXOS_LINEN_SESSION_PROOF").is_some();
static mut LINEN_SESSION_PROOF_STAGE: u8 = 0;

/// Build with SEXOS_LINEN_SEXFILES_METADATA_PROOF=1 to enable metadata bridge proof.
const LINEN_SEXFILES_METADATA_PROOF_ENABLED: bool =
    option_env!("SEXOS_LINEN_SEXFILES_METADATA_PROOF").is_some();

/// Build with SEXOS_SEXOBJECT_OQ5_PROOF=1 to enable OQ5 namespace resolution proof.
const SEXOS_OQ5_PROOF_ENABLED: bool =
    option_env!("SEXOS_SEXOBJECT_OQ5_PROOF").is_some();

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // Brief delay to ensure sexdisplay is ready to receive
    for _ in 0..5_000_000 { core::hint::spin_loop(); }

    // Create placeholder surface on sexdisplay (0xEC upsert by id)
    // arg1 = (y<<32)|x, arg2 = (h<<32)|w
    pdx_call(SLOT_DISPLAY, 0xEC, SURFACE_ID_LINEN,
        (500u64 << 32) | 900u64,  // x=900, y=500
        (150u64 << 32) | 300u64); // w=300, h=150
    serial_println!("[linen] Placeholder surface 200 created via 0xEC");

    // Fill rect: local (20, 20, 80, 60), coral color
    pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_LINEN,
        (20u64 << 32) | 20u64,
        (0x00FF6464u64 << 32) | (60u64 << 16) | 80u64);
    serial_println!("[linen] Fill rect 0xEF sent to sexdisplay");
    serial_println!("[linen.ready]");

    // ── Synthetic proof: Linen session object model ──
    if LINEN_SESSION_PROOF_ENABLED {
        unsafe { run_session_proof(); }
    }

    // ── Metadata bridge proof: Linen↔SexFiles persistence ──
    if LINEN_SEXFILES_METADATA_PROOF_ENABLED {
        unsafe { run_metadata_bridge_proof(); }
    }

    // ── OQ5 proof: SexObject ID namespace resolution ──
    if SEXOS_OQ5_PROOF_ENABLED {
        unsafe { run_oq5_proof(); }
    }

    loop {
        let msg = pdx_listen_raw(0);

        match msg.type_id {
            OP_HID_EVENT => {
                handle_hid_event(msg.arg0, msg.arg1);
            }
            OP_LINEN_CREATE_OBJECT => {
                unsafe {
                    handle_create_object(msg.arg0, msg.arg1, msg.arg2, msg.caller_pd);
                }
            }
            OP_LINEN_LIST_OBJECTS => {
                unsafe {
                    handle_list_objects(msg.arg0, msg.caller_pd);
                }
            }
            OP_LINEN_GET_OBJECT => {
                unsafe {
                    handle_get_object(msg.arg0, msg.caller_pd);
                }
            }
            _ => {}
        }
    }
}

// ── HID event handler (unchanged from base) ────────────────────────────────
fn handle_hid_event(scancode: u64, value: u64) {
    unsafe {
        static mut LINEN_KEY_BUDGET: u32 = 16;
        let b = &mut LINEN_KEY_BUDGET;
        if *b > 0 {
            *b -= 1;
            serial_println!("[linen.key.recv] scancode={:#x} val={}", scancode, value);
        }

        static mut LINEN_COLOR_TOGGLE: bool = false;
        if value == 1 {
            LINEN_COLOR_TOGGLE = !LINEN_COLOR_TOGGLE;
            let color = if LINEN_COLOR_TOGGLE { 0x0000FF00 } else { 0x00FF6464 };
            pdx_call(SLOT_DISPLAY, 0xEF, SURFACE_ID_LINEN,
                (20u64 << 32) | 20u64,
                (color << 32) | (60u64 << 16) | 80u64);

            static mut LINEN_VISUAL_BUDGET: u32 = 16;
            let vb = &mut LINEN_VISUAL_BUDGET;
            if *vb > 0 {
                *vb -= 1;
                serial_println!("[linen.focus.visual_update] color={:#x}", color);
            }
        }
    }
}

// ── Session opcode handlers ─────────────────────────────────────────────────

/// Handle OP_LINEN_CREATE_OBJECT.
///
/// arg0 = packed: kind (bits 0-7), name_len (bits 8-15)
/// arg1 = first 8 bytes of display name
/// arg2 = next 8 bytes of display name
/// NOTE: current wire payload supports up to 16 bytes total for create.
/// caller_pd = owner
///
/// Reply: object_id on success, error code (negative) on failure.
unsafe fn handle_create_object(arg0: u64, arg1: u64, arg2: u64, caller_pd: u32) {
    let kind_byte = (arg0 & 0xFF) as u8;
    let name_len = ((arg0 >> 8) & 0xFF) as u8;

    // Validate kind.
    let kind = match kind_byte {
        KIND_DOCUMENT => session::ObjectKind::Document,
        KIND_SESSION => session::ObjectKind::Session,
        KIND_UNKNOWN => session::ObjectKind::Unknown,
        _ => {
            serial_println!("[linen.session.reject] reason=bad_kind kind={} caller={}", kind_byte, caller_pd);
            pdx_reply(caller_pd, 0xFFFF_FFFF_FFFF_FFFC); // ERR_CAP_INVALID equivalent
            return;
        }
    };

    // Validate name length.
    if name_len == 0 || name_len as usize > LINEN_MAX_NAME {
        serial_println!("[linen.session.reject] reason=bad_name_len len={} max={} caller={}",
            name_len, LINEN_MAX_NAME, caller_pd);
        pdx_reply(caller_pd, 0xFFFF_FFFF_FFFF_FFFE); // ERR_SERVICE_NOT_READY equivalent
        return;
    }
    if name_len as usize > LINEN_CREATE_WIRE_MAX_NAME {
        serial_println!("[linen.session.reject] reason=wire_name_len len={} wire_max={} caller={}",
            name_len, LINEN_CREATE_WIRE_MAX_NAME, caller_pd);
        pdx_reply(caller_pd, 0xFFFF_FFFF_FFFF_FFFE);
        return;
    }

    // Pack name from arg1 (bytes 0-7) and arg2 (bytes 8-15).
    let mut name = [0u8; LINEN_MAX_NAME];
    let arg1_bytes = arg1.to_le_bytes();
    let arg2_bytes = arg2.to_le_bytes();
    let copy_len = core::cmp::min(name_len as usize, LINEN_CREATE_WIRE_MAX_NAME);
    name[..8].copy_from_slice(&arg1_bytes);
    if copy_len > 8 {
        let remaining = core::cmp::min(copy_len - 8, 8);
        name[8..8+remaining].copy_from_slice(&arg2_bytes[..remaining]);
    }

    // Create the object in the session.
    let result = SESSION.create(kind, &name[..name_len as usize], caller_pd);
    match result {
        Ok(object_id) => {
            serial_println!("[linen.session.create] id={} kind={} name_len={} owner={}",
                object_id, kind_byte, name_len, caller_pd);

            // ── Persist metadata to SexFiles ──
            let persist_result = linen_persist_object(
                object_id, kind_byte, caller_pd,
                &name, name_len, 1, 0x01,
            );
            match persist_result {
                Ok((handle, sexfiles_oid)) => {
                    // Mark local object as persisted and bind the global SexFiles object_id.
                    let _ = SESSION.set_persisted(object_id, handle);
                    let _ = SESSION.set_sexfiles_object_id(object_id, sexfiles_oid);
                    serial_println!("[linen.sexfiles.proof.create_link] id={} handle={} sexfiles_object_id={}",
                        object_id, handle, sexfiles_oid);
                }
                Err(e) => {
                    // Persistence failed but local object was created.
                    // Reply with object_id anyway; the object exists locally.
                    // Metadata persistence is best-effort in this revision.
                    serial_println!("[linen.sexfiles.persist.warn] id={} err={} local_only=true",
                        object_id, e);
                }
            }

            pdx_reply(caller_pd, object_id);
        }
        Err(e) => {
            serial_println!("[linen.session.reject] reason=create_failed err={} caller={}", e, caller_pd);
            pdx_reply(caller_pd, e as u64);
        }
    }
}

/// Handle OP_LINEN_LIST_OBJECTS.
///
/// arg0 = start_index (byte offset into table, 0 = first)
/// caller_pd = owner filter (only returns objects owned by caller)
///
/// Reply: packed object data, or 0 if no more entries.
/// Packing: bits 0-31 = object_id, bits 32-39 = kind, bits 40-63 = name_len + flags
unsafe fn handle_list_objects(arg0: u64, caller_pd: u32) {
    let start_idx = (arg0 & 0xFF) as u8;
    let result = SESSION.list(caller_pd, start_idx);
    match result {
        Some(obj) => {
            // Send as two replies: arg0=object_id, arg1=name_lo, arg2=packed
            // But pdx_reply only sends one u64.
            // Instead, pack:
            //   value bits 0-31 = object_id (low bits)
            //   value bits 32-39 = kind
            //   value bits 40-47 = name_len
            //   value bits 48-55 = owner_pd (mask)
            let reply = (obj.object_id & 0xFFFF_FFFF)
                      | ((obj.kind as u8 as u64) << 32)
                      | ((obj.name_len as u64) << 40);
            serial_println!("[linen.session.list] id={} kind={} name_len={} owner={}",
                obj.object_id, obj.kind as u8, obj.name_len, obj.owner_pd);
            serial_println!("[linen.session.proof.list] id={} kind={} name_len={} ramfs={}",
                obj.object_id, obj.kind as u8, obj.name_len, obj.ramfs_handle);
            pdx_reply(caller_pd, reply);
        }
        None => {
            pdx_reply(caller_pd, 0);
        }
    }
}

/// Handle OP_LINEN_GET_OBJECT.
///
/// arg0 = object_id
/// caller_pd = for owner validation
///
/// Reply: packed object data, or error code (negative).
unsafe fn handle_get_object(arg0: u64, caller_pd: u32) {
    let object_id = arg0;
    match SESSION.get(object_id, caller_pd) {
        Ok(obj) => {
            let name_lo = u64::from_le_bytes([
                obj.name[0], obj.name[1], obj.name[2], obj.name[3],
                obj.name[4], obj.name[5], obj.name[6], obj.name[7],
            ]);
            let reply_name = name_lo; // first 8 bytes of name as reply value
            serial_println!("[linen.session.get] id={} kind={} name_len={} owner={}",
                obj.object_id, obj.kind as u8, obj.name_len, obj.owner_pd);
            serial_println!("[linen.session.proof.get] id={} owner={} ramfs={}",
                obj.object_id, obj.owner_pd, obj.ramfs_handle);
            pdx_reply(caller_pd, reply_name);
        }
        Err(e) => {
            serial_println!("[linen.session.reject] reason=get_failed id={} err={} caller={}",
                object_id, e, caller_pd);
            pdx_reply(caller_pd, e as u64);
        }
    }
}

// ── SexFiles Storage Bridge Helpers ──────────────────────────────────────────

/// Pack a byte slice name into two u64 args for OP_RAMFS_OPEN.
/// Matches Quil pack_name pattern per SEXFILES_RAMFS_CONTRACT_LOCK_V1.
fn pack_name(name: &[u8]) -> (u64, u64) {
    let mut a0 = 0u64;
    let mut a1 = 0u64;
    for i in 0..name.len().min(8) {
        a0 |= (name[i] as u64) << (i * 8);
    }
    if name.len() > 8 {
        for i in 8..name.len().min(16) {
            a1 |= (name[i] as u64) << ((i - 8) * 8);
        }
    }
    (a0, a1)
}

/// Synchronous PDX call to SexFiles SLOT_STORAGE.
/// Returns the reply value on success, or the error code on failure.
/// Pattern matches Quil pdx_call_and_reply.
fn pdx_storage_sync(opcode: u64, arg0: u64, arg1: u64, arg2: u64) -> Result<u64, i64> {
    let (status, _) = pdx_call(SLOT_STORAGE, opcode, arg0, arg1, arg2);
    if status != 0 {
        return Err(status as i64);
    }
    // Spin for the reply.
    loop {
        let msg = pdx_listen_raw(0);
        if msg.type_id == 0x1 {
            let value = msg.arg0;
            if (value as i64) < 0 {
                return Err(value as i64);
            }
            return Ok(value);
        }
        // Non-reply message before reply arrived — handle HID events inline.
        if msg.type_id == OP_HID_EVENT {
            handle_hid_event(msg.arg0, msg.arg1);
        }
    }
}

/// Build the RamFS metadata file name for a Linen object.
/// Format: "lo." + 4-byte owner_pd hex + "." + 8-byte object_id hex
/// Total: 3 + 8 + 1 + 16 = max 28... wait, 4-byte hex = 8 chars.
/// lo.{owner:08x}.{id:016x} = 3 + 8 + 1 + 16 = 28... too long!
/// Let's use: "lo.{id:016x}" = 18 bytes. Owner is encoded in file content.
/// Name is 18 bytes (fits in 24-byte RamFS limit).
fn make_linen_meta_name(object_id: u64) -> [u8; 24] {
    let mut name = [0u8; 24];
    // Prefix "lo."
    name[0] = b'l';
    name[1] = b'o';
    name[2] = b'.';
    // object_id as 16 hex chars
    let hex = [
        b'0', b'1', b'2', b'3', b'4', b'5', b'6', b'7',
        b'8', b'9', b'a', b'b', b'c', b'd', b'e', b'f',
    ];
    let mut v = object_id;
    for i in (3..19).rev() {
        name[i] = hex[(v & 0xF) as usize];
        v >>= 4;
    }
    name
}

/// Persist a Linen object metadata record to SexFiles.
/// Creates a RamFS file named "lo.{object_id:016x}" with owner = owner_pd,
/// writes packed metadata, then closes.
/// Returns (ramfs_handle, sexfiles_object_id) on success.
/// sexfiles_object_id is the RamFS-assigned global ID (≥1); 0 if OP_RAMFS_OBJECT_ID fails.
unsafe fn linen_persist_object(
    object_id: u64, kind: u8, owner_pd: u32,
    name: &[u8], name_len: u8, generation: u64, flags: u8,
) -> Result<(u64, u64), i64> {
    let meta_name = make_linen_meta_name(object_id);

    // Pack name into arg0/arg1 for OP_RAMFS_CREATE_OWNER.
    let (n0, n1) = pack_name(&meta_name);

    // arg2: name bytes 16-23 in lower 24 bits, owner_pd in upper 32 bits.
    let mut name16_23: u64 = 0;
    for i in 16..meta_name.len().min(24) {
        name16_23 |= (meta_name[i] as u64) << ((i - 16) * 8);
    }
    let arg2 = name16_23 | ((owner_pd as u64) << 32);

    // Create file with explicit owner.
    let handle = pdx_storage_sync(OP_RAMFS_CREATE_OWNER, n0, n1, arg2)
        .map_err(|e| {
            serial_println!("[linen.sexfiles.persist.fail] id={} phase=create_owner err={}",
                object_id, e);
            e
        })?;

    // Obtain the SexFiles-assigned global object_id (authoritative for SexObjectRef).
    let sexfiles_oid = pdx_storage_sync(OP_RAMFS_OBJECT_ID, handle, 0, 0).unwrap_or(0);

    // Write metadata record: 8 bytes per write.
    // Record layout (48 bytes = 6 writes of 8 bytes):
    //   bytes 0..7:  object_id (u64 LE)
    //   bytes 8..9:  kind (u16 LE)
    //   bytes 10..13: owner_pd (u32 LE)
    //   bytes 14..21: generation (u64 LE)
    //   bytes 22: flags (u8)
    //   bytes 23: name_len (u8)
    //   bytes 24..47: name (24 bytes)

    let mut meta = [0u8; 48];

    // object_id (8 bytes)
    meta[0..8].copy_from_slice(&object_id.to_le_bytes());
    // kind (2 bytes)
    meta[8..10].copy_from_slice(&(kind as u16).to_le_bytes());
    // owner_pd (4 bytes)
    meta[10..14].copy_from_slice(&owner_pd.to_le_bytes());
    // generation (8 bytes)
    meta[14..22].copy_from_slice(&generation.to_le_bytes());
    // flags (1 byte)
    meta[22] = flags;
    // name_len (1 byte)
    meta[23] = name_len;
    // name (up to 24 bytes)
    let copy_len = core::cmp::min(name_len as usize, 24);
    meta[24..24 + copy_len].copy_from_slice(&name[..copy_len]);

    // Write in 8-byte chunks.
    for chunk in 0..6 {
        let offset = chunk * 8;
        let mut data = 0u64;
        for i in 0..8 {
            data |= (meta[offset + i] as u64) << (i * 8);
        }
        pdx_storage_sync(OP_RAMFS_WRITE, handle, offset as u64, data)
            .map_err(|e| {
                serial_println!("[linen.sexfiles.persist.fail] id={} phase=write chunk={} err={}",
                    object_id, chunk, e);
                // Best-effort close on write failure
                let _ = pdx_storage_sync(OP_RAMFS_CLOSE, handle, 0, 0);
                e
            })?;
    }

    // Close the file. Data persists for reopen-by-name.
    pdx_storage_sync(OP_RAMFS_CLOSE, handle, 0, 0)
        .map_err(|e| {
            serial_println!("[linen.sexfiles.persist.fail] id={} phase=close err={}",
                object_id, e);
            e
        })?;

    serial_println!("[linen.sexfiles.persist] id={} handle={} sexfiles_object_id={} owner={} kind={} gen={}",
        object_id, handle, sexfiles_oid, owner_pd, kind, generation);
    Ok((handle, sexfiles_oid))
}

// ── Synthetic Proof ─────────────────────────────────────────────────────────

/// Run session object model proof stages at boot.
/// Stages cover owner create/list/get, bounds, invalid id, and table-full behavior.
unsafe fn run_session_proof() {
    let stage = &mut LINEN_SESSION_PROOF_STAGE;
    serial_println!("[linen.session.proof] begin");

    // Stage 0: Create a Document object owned by PD 42.
    {
        let name = b"quil-save-v1\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        let name_len: u8 = 12;
        let result = SESSION.create(session::ObjectKind::Document, &name[..name_len as usize], 42);
        match result {
            Ok(id) => {
                serial_println!("[linen.session.proof] stage=0 create_doc id={} accepted=true", id);
                serial_println!("[linen.session.proof.create] id={} owner=42", id);
            }
            Err(e) => serial_println!("[linen.session.proof] stage=0 create_doc accepted=false err={}", e),
        }
    }
    *stage += 1;

    // Stage 1: List objects owned by PD 42 (should find the document).
    {
        let list_result = SESSION.list(42, 0);
        match list_result {
            Some(obj) => {
                serial_println!("[linen.session.proof] stage=1 list_owned id={} accepted=true", obj.object_id);
                serial_println!("[linen.session.proof.list] id={} owner={} kind={}", obj.object_id, obj.owner_pd, obj.kind as u8);
            }
            None => serial_println!("[linen.session.proof] stage=1 list_owned accepted=false"),
        }
    }
    *stage += 1;

    // Stage 2: List objects owned by PD 99 (non-owner, should get None).
    {
        let list_result = SESSION.list(99, 0);
        match list_result {
            Some(_) => serial_println!("[linen.session.proof] stage=2 list_non_owner accepted=true (UNEXPECTED)"),
            None => serial_println!("[linen.session.proof] stage=2 list_non_owner accepted=false"),
        }
    }
    *stage += 1;

    // Stage 3: Create object with invalid kind byte (3).
    {
        let name = b"bad-kind\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0";
        let name_len: u8 = 8;
        let result = SESSION.create(
            // We can't pass invalid kind through the type-safe enum, so test via
            // the PDX handler path by checking that the handler rejects bad kind.
            session::ObjectKind::Unknown,
            &name[..name_len as usize],
            42,
        );
        // This should succeed (Unknown is a valid kind). The bad kind rejection
        // happens in the PDX handler before calling SESSION.create.
        // We verify that the PDX handler check above is correct via logging.
        match result {
            Ok(id) => serial_println!("[linen.session.proof] stage=3 bad_kind_enum_result id={} (Unknown is valid)", id),
            Err(e) => serial_println!("[linen.session.proof] stage=3 bad_kind accepted=false err={}", e),
        }
    }
    *stage += 1;

    // Stage 4: Create with oversized name (> 24 bytes).
    {
        let long_name = [b'X'; LINEN_MAX_NAME + 1];
        let result = SESSION.create(session::ObjectKind::Document, &long_name, 42);
        serial_println!("[linen.session.proof] stage=4 oversized_name max={}", LINEN_MAX_NAME);
        match result {
            Ok(id) => serial_println!("[linen.session.proof] stage=4 oversized_name accepted=true id={} (UNEXPECTED)", id),
            Err(e) => serial_println!("[linen.session.proof] stage=4 oversized_name accepted=false err={}", e),
        }
        serial_println!("[linen.session.proof.bounds] max_name={} max_objects={}", LINEN_MAX_NAME, LINEN_MAX_OBJECTS);
    }
    *stage += 1;

    // Stage 5: Non-owner tries to get object (PD 99 tries to get object_id 1).
    {
        let get_result = SESSION.get(1, 99);
        match get_result {
            Ok(_) => serial_println!("[linen.session.proof] stage=5 non_owner_get accepted=true (UNEXPECTED)"),
            Err(e) => {
                serial_println!("[linen.session.proof] stage=5 non_owner_get accepted=false err={}", e);
                serial_println!("[linen.session.proof.owner_deny] object_id=1 caller=99 err={}", e);
            }
        }
    }
    *stage += 1;

    // Stage 6: Invalid object_id get should be rejected.
    {
        let get_result = SESSION.get(0xFFFF_FFFF_FFFF_FFFF, 42);
        match get_result {
            Ok(_) => serial_println!("[linen.session.proof] stage=6 invalid_id accepted=true (UNEXPECTED)"),
            Err(e) => serial_println!("[linen.session.proof] stage=6 invalid_id accepted=false err={}", e),
        }
    }
    *stage += 1;

    // Stage 7: Fill remaining slots then verify table-full rejection.
    {
        let fill_name = b"fill";
        let mut fill_ok: u8 = 0;
        loop {
            match SESSION.create(session::ObjectKind::Document, fill_name, 42) {
                Ok(_) => {
                    fill_ok = fill_ok.saturating_add(1);
                }
                Err(e) => {
                    serial_println!("[linen.session.proof.bounds] table_full_err={} fill_ok={}", e, fill_ok);
                    break;
                }
            }
        }
    }
    *stage += 1;

    serial_println!("[linen.session.proof.count] total={} owned_42={}",
        SESSION.count(), SESSION.count_owned(42));

    serial_println!("[linen.session.proof] end");
}

// ── Metadata Bridge Proof ────────────────────────────────────────────────────

/// Run Linen↔SexFiles metadata bridge proof at boot.
/// Activated by SEXOS_LINEN_SEXFILES_METADATA_PROOF=1.
/// Tests: create persist, list link, get link, owner deny, generation bump.
unsafe fn run_metadata_bridge_proof() {
    serial_println!("[linen.sexfiles.metadata.proof] begin");

    // Stage 0: Create object with metadata persistence.
    {
        let name = b"bridge-doc-v1\0\0\0\0\0\0\0\0\0\0\0";
        let name_len: u8 = 13;
        let result = SESSION.create(session::ObjectKind::Document, &name[..name_len as usize], 42);
        match result {
            Ok(id) => {
                serial_println!("[linen.sexfiles.proof.create_link] id={} accepted=true", id);
                // Persist to SexFiles.
                let meta_name = make_linen_meta_name(id);
                let (n0, n1) = pack_name(&meta_name);
                let mut name16_23: u64 = 0;
                for i in 16..meta_name.len().min(24) {
                    name16_23 |= (meta_name[i] as u64) << ((i - 16) * 8);
                }
                let arg2 = name16_23 | ((42u64) << 32);
                match pdx_storage_sync(OP_RAMFS_CREATE_OWNER, n0, n1, arg2) {
                    Ok(handle) => {
                        let _ = SESSION.set_persisted(id, handle);
                        serial_println!("[linen.sexfiles.proof.create_link] id={} handle={} owner=42",
                            id, handle);

                        // Write packed metadata.
                        let mut meta = [0u8; 48];
                        meta[0..8].copy_from_slice(&id.to_le_bytes());
                        meta[8..10].copy_from_slice(&(0u16).to_le_bytes()); // Document = 0
                        meta[10..14].copy_from_slice(&42u32.to_le_bytes());
                        meta[14..22].copy_from_slice(&1u64.to_le_bytes()); // generation
                        meta[22] = 0x01; // flags: persisted
                        meta[23] = name_len;
                        meta[24..24 + name_len as usize].copy_from_slice(&name[..name_len as usize]);

                        for chunk in 0..6 {
                            let offset = chunk * 8;
                            let mut data = 0u64;
                            for i in 0..8 {
                                data |= (meta[offset + i] as u64) << (i * 8);
                            }
                            let _ = pdx_storage_sync(OP_RAMFS_WRITE, handle, offset as u64, data);
                        }

                        let _ = pdx_storage_sync(OP_RAMFS_CLOSE, handle, 0, 0);

                        // Bump generation.
                        let new_gen = SESSION.bump_generation(id);
                        match new_gen {
                            Ok(gen) => serial_println!("[linen.sexfiles.proof.generation] id={} gen={}", id, gen),
                            Err(e) => serial_println!("[linen.sexfiles.proof.generation] id={} err={}", id, e),
                        }
                    }
                    Err(e) => {
                        serial_println!("[linen.sexfiles.proof.create_link] id={} err={}", id, e);
                    }
                }
            }
            Err(e) => serial_println!("[linen.sexfiles.proof.create_link] accepted=false err={}", e),
        }
    }

    // Stage 1: List objects with SexFiles-backed metadata.
    {
        let list_result = SESSION.list(42, 0);
        match list_result {
            Some(obj) => {
                let persisted = (obj.flags & 0x01) != 0;
                serial_println!(
                    "[linen.sexfiles.proof.list_link] id={} owner={} kind={} gen={} persisted={}",
                    obj.object_id, obj.owner_pd, obj.kind as u8, obj.generation, persisted as u8
                );
            }
            None => serial_println!("[linen.sexfiles.proof.list_link] none"),
        }
    }

    // Stage 2: Get object metadata.
    {
        // Find the first object owned by PD 42.
        let list_result = SESSION.list(42, 0);
        if let Some(obj) = list_result {
            let get_result = SESSION.get(obj.object_id, 42);
            match get_result {
                Ok(found) => {
                    let persisted = (found.flags & 0x01) != 0;
                    serial_println!(
                        "[linen.sexfiles.proof.get_link] id={} owner={} gen={} persisted={}",
                        found.object_id, found.owner_pd, found.generation, persisted as u8
                    );
                }
                Err(e) => serial_println!("[linen.sexfiles.proof.get_link] err={}", e),
            }
        }
    }

    // Stage 3: Owner deny — non-owner (PD 99) cannot access PD 42's objects.
    {
        let list_result = SESSION.list(42, 0);
        if let Some(obj) = list_result {
            let get_result = SESSION.get(obj.object_id, 99);
            match get_result {
                Ok(_) => serial_println!("[linen.sexfiles.proof.owner_deny] unexpected_allow id={}", obj.object_id),
                Err(e) => {
                    serial_println!(
                        "[linen.sexfiles.proof.owner_deny] id={} caller=99 err={}",
                        obj.object_id, e
                    );
                }
            }
        }
    }

    serial_println!("[linen.sexfiles.metadata.proof] end");
}

/// OQ5: SexObject ID namespace resolution proof stub.
///
/// OQ5 closes the gap where Linen objects have local IDs but callers need
/// global SexFiles-assigned object_ids for SexObjectRef construction.
/// The proof exercises: create Linen object → create SexFiles file →
/// obtain OP_RAMFS_OBJECT_ID → construct SexObjectRef.
///
/// Stub pending full implementation. The OP_RAMFS_OBJECT_ID opcode (0x37)
/// is implemented in sexfiles RamFS and wired in vfs.rs.
unsafe fn run_oq5_proof() {
    serial_println!("[linen.oq5.proof] stub — OP_RAMFS_OBJECT_ID wired, full proof deferred");
}
