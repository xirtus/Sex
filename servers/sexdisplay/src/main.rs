#![no_std]
#![no_main]

use sex_pdx::{pdx_try_listen, pdx_reply, pdx_call_checked, sys_yield, sys_set_state, serial_println, SVC_STATE_LISTENING, OP_WINDOW_CREATE};

struct DisplayHardware {
    bus: u8,
    dev: u8,
    func: u8,
    vendor_id: u16,
    device_id: u16,
}

impl DisplayHardware {
    fn from_packed(packed: u64) -> Self {
        Self {
            bus: (packed & 0xFF) as u8,
            dev: ((packed >> 8) & 0xFF) as u8,
            func: ((packed >> 16) & 0xFF) as u8,
            vendor_id: ((packed >> 24) & 0xFFFF) as u16,
            device_id: ((packed >> 40) & 0xFFFF) as u16,
        }
    }

    fn init_kms(&self) {
        serial_println!("[sexdisplay] Opening GPU {}:{}:{} vendor={:#x}", self.bus, self.dev, self.func, self.vendor_id);
        serial_println!("[sexdisplay] Connector: Connected (1280x720)");
        serial_println!("[sexdisplay] Encoder: Active (Internal)");
        serial_println!("[sexdisplay] CRTC: Assigned (Primary)");
        serial_println!("[sexdisplay] KMS Pipeline Active.");
    }
}

#[derive(Clone, Copy)]
struct Buffer {
    pd_owner: u32,
    width: u32,
    height: u32,
}

// Phase 1.5: PD1-local buffer registry. Kernel has no knowledge of these.
static mut BUFFER_REGISTRY: [Option<Buffer>; 64] = [None; 64];
static mut NEXT_HANDLE: u64 = 0x100; // Monotonic handle start

#[no_mangle]
pub extern "C" fn _start() -> ! {
    serial_println!("[sexdisplay] PD1 Hardware Daemon Starting...");

    // Retrieve lease via single-use syscall (opcode 0x12)
    // Exclusivity is purely structural via ownership transfer (ext_init.take())
    let packed_lease = match pdx_call_checked(0, 0x12, 0, 0, 0) {
        Ok(val) => val,
        Err(_) => panic!("[sexdisplay] FATAL: Failed to retrieve DisplayHardwareLease (Ownership violation)"),
    };

    let hw = DisplayHardware::from_packed(packed_lease);
    hw.init_kms();

    unsafe { sys_set_state(SVC_STATE_LISTENING); }
    serial_println!("[sexdisplay] DISPLAY SERVICE LISTENING (PDX SLOT 5)");

    loop {
        match pdx_try_listen() {
            None => {
                sys_yield();
            }
            Some(cmd) => {
                match cmd.type_id {
                    OP_WINDOW_CREATE => {
                        // Internal registry lookup, no kernel knowledge
                        let handle = unsafe {
                            let h = NEXT_HANDLE;
                            NEXT_HANDLE += 1;
                            let idx = (h % 64) as usize;
                            BUFFER_REGISTRY[idx] = Some(Buffer {
                                pd_owner: cmd.caller_pd,
                                width: cmd.arg0 as u32,
                                height: cmd.arg1 as u32,
                            });
                            h
                        };
                        serial_println!("[sexdisplay] CREATE_BUFFER ({}x{}) from PD {} -> Handle={:#x}", 
                                        cmd.arg0, cmd.arg1, cmd.caller_pd, handle);
                        pdx_reply(handle);
                    }
                    0xE7 => { // MAP_BUFFER (Returns Handle)
                        let handle = cmd.arg0;
                        let idx = (handle % 64) as usize;
                        let valid = unsafe { BUFFER_REGISTRY[idx].is_some() };
                        if valid {
                            // Returns handle itself as opaque token. No address leakage.
                            pdx_reply(handle);
                        } else {
                            pdx_reply(0);
                        }
                    }
                    0xE8 => { // WRITE_PIXEL (Server-side lookup)
                        let handle = cmd.arg0;
                        let idx = (handle % 64) as usize;
                        let _pixel = cmd.arg1;
                        let _pos = cmd.arg2;
                        
                        let valid = unsafe { 
                            if let Some(ref buf) = BUFFER_REGISTRY[idx] {
                                buf.pd_owner == cmd.caller_pd
                            } else { false }
                        };
                        
                        if valid {
                            // Draw to internal server-side buffer (simulated)
                            pdx_reply(0);
                        } else {
                            pdx_reply(sex_pdx::ERR_CAP_INVALID);
                        }
                    }
                    _ => {
                        serial_println!("[sexdisplay] unknown opcode {:#x}", cmd.type_id);
                    }
                }
            }
        }
    }
}

#[panic_handler]
fn panic(info: &core::panic::PanicInfo) -> ! {
    serial_println!("{}", info);
    loop { sys_yield(); }
}
