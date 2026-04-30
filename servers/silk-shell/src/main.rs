#![no_std]
#![no_main]

extern crate alloc;
use alloc::vec::Vec;
use core::panic::PanicInfo;
use sex_pdx::{
    pdx_call, pdx_try_listen, pdx_reply, sys_yield, sys_set_state, serial_println, WindowDescriptor,
    SLOT_DISPLAY, SLOT_SILKBAR, OP_SILKBAR_WORKSPACE_ACTIVE, OP_SILKBAR_FOCUS_STATE,
    SVC_STATE_LISTENING, ERR_CAP_INVALID,
};

// Local Opcodes
pub const OP_DISPLAY_SET_SNAPSHOT: u64 = 0x15;
pub const OP_SHELL_BIND_BUFFER: u64 = 0x14;
pub const OP_HID_EVENT: u64 = 0x202;

#[panic_handler]
fn panic(info: &PanicInfo) -> ! {
    serial_println!("{}", info);
    loop { sys_yield(); }
}

struct WindowState {
    desc: WindowDescriptor,
}

static mut WINDOWS: Vec<WindowState> = Vec::new();
static mut FOCUS_ID: u64 = 0;
static mut SNAPSHOT: [WindowDescriptor; 16] = [
    WindowDescriptor { window_id: 0, buffer_handle: 0, x: 0, y: 0, width: 0, height: 0, z_index: 0, focus_state: 0 }; 16
];

fn emit_snapshot() {
    unsafe {
        let mut len = 0;
        // Authorities Z-order sorting here: Focused window always on top (last in array)
        let focus_id = FOCUS_ID;
        
        let mut sorted_windows: Vec<usize> = (0..WINDOWS.iter().len()).collect();
        // Simple sort: focus_id window goes to the end
        sorted_windows.sort_by(|&a, &b| {
            if WINDOWS[a].desc.window_id == focus_id { core::cmp::Ordering::Greater }
            else if WINDOWS[b].desc.window_id == focus_id { core::cmp::Ordering::Less }
            else { core::cmp::Ordering::Equal }
        });

        for (i, &idx) in sorted_windows.iter().enumerate() {
            if i >= 16 { break; }
            let w = &WINDOWS[idx];
            SNAPSHOT[i] = w.desc;
            SNAPSHOT[i].z_index = i as u32;
            SNAPSHOT[i].focus_state = if w.desc.window_id == focus_id { 1 } else { 0 };
            len += 1;
        }

        // Emit to sexdisplay (SLOT 5)
        pdx_call(SLOT_DISPLAY, OP_DISPLAY_SET_SNAPSHOT, SNAPSHOT.as_ptr() as u64, len as u64, 0);
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    sex_rt::heap_init();
    serial_println!("[silk-shell] Authority Starting...");

    unsafe {
        WINDOWS = Vec::with_capacity(16);
        
        // Create background window (id=1)
        WINDOWS.push(WindowState {
            desc: WindowDescriptor {
                window_id: 1,
                buffer_handle: 0, // Placeholder
                x: 0, y: 0, width: 1280, height: 720,
                z_index: 0, focus_state: 0,
            }
        });
        FOCUS_ID = 1;

        sys_set_state(SVC_STATE_LISTENING);
    }
    serial_println!("[silk-shell] AUTHORITATIVE WM LISTENING (PDX SLOT 6)");

    // Stage 2B: advertise workspace 0 active to SilkBar
    pdx_call(SLOT_SILKBAR, OP_SILKBAR_WORKSPACE_ACTIVE, 0, 0, 0);
    // Stage 2C: one-time focus advertisement (shell)
    pdx_call(SLOT_SILKBAR, OP_SILKBAR_FOCUS_STATE, 1, 0, 0);
    serial_println!("[silk-shell] Boot workspace advertisement sent to SilkBar");

    loop {
        let mut mutated = false;

        while let Some(msg) = pdx_try_listen() {
            match msg.type_id {
                OP_SHELL_BIND_BUFFER => {
                    let buffer_handle = msg.arg0;
                    serial_println!("[silk-shell] Binding buffer {:#x} to sexdrive window", buffer_handle);

                    unsafe {
                        let mut found = false;
                        for w in WINDOWS.iter_mut() {
                            if w.desc.window_id == 2 {
                                w.desc.buffer_handle = buffer_handle;
                                found = true;
                                break;
                            }
                        }
                        if !found {
                            WINDOWS.push(WindowState {
                                desc: WindowDescriptor {
                                    window_id: 2,
                                    buffer_handle,
                                    x: 100, y: 100, width: 1024, height: 768,
                                    z_index: 1, focus_state: 1,
                                }
                            });
                            FOCUS_ID = 2;
                        }
                    }
                    mutated = true;
                    pdx_reply(0);
                }
                OP_HID_EVENT => {
                    let scancode = msg.arg0 as u8;
                    // let value = msg.arg1; // 1=pressed
                    // let ev_type = msg.arg2; // 1=EV_KEY
                    
                    unsafe {
                        let focus_id = FOCUS_ID;
                        for w in WINDOWS.iter_mut() {
                            if w.desc.window_id == focus_id && focus_id != 1 { // Don't move background
                                match scancode {
                                    0x4B => { w.desc.x -= 10; mutated = true; } // Left
                                    0x4D => { w.desc.x += 10; mutated = true; } // Right
                                    0x48 => { w.desc.y -= 10; mutated = true; } // Up
                                    0x50 => { w.desc.y += 10; mutated = true; } // Down
                                    0x3B => { // F1: Switch focus
                                        FOCUS_ID = if FOCUS_ID == 2 { 1 } else { 2 };
                                        mutated = true;
                                        serial_println!("[silk-shell] Focus switched to {}", FOCUS_ID);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                _ => {
                    // pdx_reply(ERR_CAP_INVALID); // Only reply if it was a call
                }
            }
        }

        if mutated {
            emit_snapshot();
        }

        sys_yield();
    }
}
