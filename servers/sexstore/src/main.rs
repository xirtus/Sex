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

// Reply values for PUT. GET replies the stored u64 blob directly (0 = not found).
const KV_PUT_OK:   u64 = 0x00;
const KV_PUT_FULL: u64 = 0x02;

const KV_SLOT_COUNT: usize = 16;

#[derive(Clone, Copy)]
struct KvSlot {
    used: u8,
    key:  u32,
    val:  u64,
}

// Static RAM table — 16 × (1+3pad+4+8) = 16 × 16 = 256 bytes. No heap.
static mut KV: [KvSlot; KV_SLOT_COUNT] = [KvSlot { used: 0, key: 0, val: 0 }; KV_SLOT_COUNT];

static mut LOG_PUT: u32 = 32;
static mut LOG_GET: u32 = 32;

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

#[no_mangle]
pub extern "C" fn _start() -> ! {
    loop {
        let msg = pdx_listen_raw(SLOT_SEXSTORE);
        let caller = msg.caller_pd as u64;

        unsafe {
            match msg.type_id {
                OP_KV_PUT => {
                    let key = msg.arg0 as u32;
                    let val = msg.arg1;

                    // Update in-place if key exists, or insert into first free slot.
                    // Use raw pointer index loops to avoid static_mut_refs violation.
                    let kv_ptr: *mut KvSlot = core::ptr::addr_of_mut!(KV) as *mut KvSlot;
                    let mut found = false;
                    let mut full = false;
                    let mut i = 0;
                    while i < KV_SLOT_COUNT {
                        let slot = kv_ptr.add(i);
                        if (*slot).used != 0 && (*slot).key == key {
                            (*slot).val = val;
                            found = true;
                            break;
                        }
                        i += 1;
                    }
                    if !found {
                        let mut inserted = false;
                        let mut i = 0;
                        while i < KV_SLOT_COUNT {
                            let slot = kv_ptr.add(i);
                            if (*slot).used == 0 {
                                (*slot).used = 1;
                                (*slot).key  = key;
                                (*slot).val  = val;
                                inserted = true;
                                break;
                            }
                            i += 1;
                        }
                        if !inserted { full = true; }
                    }

                    let status = if full { KV_PUT_FULL } else { KV_PUT_OK };
                    kv_reply(caller, status);

                    if LOG_PUT > 0 {
                        LOG_PUT -= 1;
                        serial_println!("[sexstore.kv.put] key={} ok={}", key, if full { 0 } else { 1 });
                    }
                }

                OP_KV_GET => {
                    let key = msg.arg0 as u32;

                    // Raw pointer index loop to avoid static_mut_refs violation.
                    let kv_ptr: *const KvSlot = core::ptr::addr_of!(KV) as *const KvSlot;
                    let mut result: u64 = 0; // 0 = not found; caller validates magic in blob
                    let mut i = 0;
                    while i < KV_SLOT_COUNT {
                        let slot = kv_ptr.add(i);
                        if (*slot).used != 0 && (*slot).key == key {
                            result = (*slot).val;
                            break;
                        }
                        i += 1;
                    }

                    kv_reply(caller, result);

                    if LOG_GET > 0 {
                        LOG_GET -= 1;
                        serial_println!("[sexstore.kv.get] key={} hit={}", key, if result != 0 { 1 } else { 0 });
                    }
                }

                _ => {
                    // Unknown opcode — reply 0 and ignore.
                    kv_reply(caller, 0);
                }
            }
        }
    }
}
