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

use sex_pdx::{pdx_listen_raw, serial_println, SLOT_SEXSTORE};

// Local opcode definitions — promoted to sex-pdx when silk-shell integration lands.
// Kept local to avoid sex-pdx change and ABI hash update this phase.
const OP_KV_GET: u64 = 0xB0;
const OP_KV_PUT: u64 = 0xB1;
const OP_KV_DEL: u64 = 0xB2; // E6: DELETE / tombstone

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

// Reply to caller via kernel syscall 29 (SYSCALL_PDX_REPLY).
// sex-pdx's pdx_reply() uses syscall 1 — unhandled in current kernel. Use 29 directly.
// Kernel: rdi=target_pd, rsi=value → pushed to target's incoming_replies buffer.
// Caller reads reply via pdx_listen_raw(0) → msg.arg0 = this value.
#[inline(always)]
unsafe fn kv_reply(target_pd: u64, val: u64) {
    core::arch::asm!(
        "syscall",
        in("rax") 29u64,
        in("rdi") target_pd,
        in("rsi") val,
        out("rcx") _,
        out("r11") _,
        options(nostack),
    );
}

// Reply with status code (sets bit 63 to distinguish from stored value).
#[inline(always)]
unsafe fn kv_reply_status(target_pd: u64, status: u64) {
    kv_reply(target_pd, REPLY_STATUS_BIT | status);
}

/// Bump slot generation (wraps 255 → 1, never 0).
#[inline(always)]
unsafe fn bump_generation(slot: *mut KvSlot) {
    let g = (*slot).generation;
    (*slot).generation = if g >= 255 { 1 } else { g + 1 };
}

// E4: Key owner class and capability checking.
// Silk-shell (domain 3) is the only authorized caller in E4.
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

#[no_mangle]
pub extern "C" fn _start() -> ! {
    // E6: emit status mapping marker once at boot.
    serial_println!("[sexstore.status.mapping] KV_OK=0x00 KV_NOT_FOUND=0x01 KV_FULL=0x02 KV_INVALID_KEY=0x03 KV_INVALID_VALUE=0x04 KV_DENIED=0x05 REPLY_BIT=0x8000");

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
                            kv_reply_status(caller, KV_INVALID_KEY);
                        } else {
                            kv_reply_status(caller, KV_DENIED);
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
                        kv_reply_status(caller, KV_INVALID_VALUE);
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
                        kv_reply_status(caller, KV_OK);
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
                                    bump_generation(slot);
                                    inserted = true;
                                    if LOG_GENERATION_BUMP > 0 {
                                        LOG_GENERATION_BUMP -= 1;
                                        serial_println!("[sexstore.generation.bump] key={} slot={} gen={} op=reclaim", key, i, (*slot).generation);
                                    }
                                    break;
                                }
                                i += 1;
                            }
                        }
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
                        kv_reply_status(caller, status);
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
                            kv_reply_status(caller, KV_INVALID_KEY);
                        } else {
                            kv_reply_status(caller, KV_DENIED);
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
                            kv_reply(caller, result);
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
                            kv_reply_status(caller, KV_NOT_FOUND);
                        }
                        _ => {
                            // Not found.
                            if LOG_GET_REJECT > 0 {
                                LOG_GET_REJECT -= 1;
                                serial_println!("[sexstore.get.reject] caller={} key={} status=not_found reason=missing", caller, key);
                            }
                            kv_reply_status(caller, KV_NOT_FOUND);
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
                            kv_reply_status(caller, KV_INVALID_KEY);
                        } else {
                            kv_reply_status(caller, KV_DENIED);
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
                            kv_reply_status(caller, KV_OK);
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
                            kv_reply_status(caller, KV_OK);
                        }
                        _ => {
                            // Not found.
                            if LOG_DELETE_REJECT > 0 {
                                LOG_DELETE_REJECT -= 1;
                                serial_println!("[sexstore.delete.reject] caller={} key={} status=not_found reason=missing", caller, key);
                            }
                            kv_reply_status(caller, KV_NOT_FOUND);
                        }
                    }
                }

                _ => {
                    // Unknown opcode — reply 0 and ignore.
                    if LOG_REPLY_ERROR > 0 {
                        LOG_REPLY_ERROR -= 1;
                        serial_println!("[sexstore.reply.error] caller={} op={:#x}", caller, msg.type_id);
                    }
                    kv_reply(caller, 0);
                }
            }
        }
    }
}
