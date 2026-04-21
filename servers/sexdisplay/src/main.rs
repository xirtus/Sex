#![no_std]
#![no_main]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unreachable_patterns)]

// Import the SexCompositor from the lib.rs module.
// This assumes lib.rs is correctly compiled and linked into the sexdisplay binary.
use sexdisplay::{
    SexCompositor, PDX_COMPOSITOR_COMMIT,
    EV_REL, REL_X, REL_Y,
    EV_KEY, BTN_LEFT, BTN_RIGHT, BTN_MIDDLE,
    MOD_LSHIFT, MOD_RSHIFT, MOD_LCTRL, MOD_RCTRL, MOD_LALT, MOD_RALT, MOD_LSUPER, MOD_RSUPER, MOD_NONE
};
use sex_pdx::{pdx_listen, pdx_reply, pdx_call, PdxRequest, MessageType, PdxMessage};

#[macro_export]
macro_rules! serial_println {
    ($($arg:tt)*) => {
        // For now, do nothing. In a real system, this would output to a serial console.
        // Example: unsafe { $(($arg)*); };
    };
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let global_width = 1920; // Example: total virtual desktop width
    let global_height = 1080; // Example: total virtual desktop height
    let _stride = 0; // Not used directly in compositor.new anymore

    let mut compositor = unsafe { SexCompositor::new(global_width, global_height, _stride) };

    // Main event loop for sexdisplay
    loop {
        // 1. Process any pending PDX events
        let request = pdx_listen(0); // Listen for any PDX event

        // Check if there's a valid message pointer
        if request.arg0 != 0 {
            // Safety: We assume arg0 is a valid pointer to a PdxMessage from the kernel.
            let pdx_message_ptr = request.arg0 as *const PdxMessage;
            let message = unsafe { &*pdx_message_ptr };

            let mut reply_val: u64 = 0; // Default success reply

            match message.msg_type {
                MessageType::RawCall(syscall_id) => {
                    reply_val = unsafe { 
                        compositor.handle_pdx_call(
                            request.caller_pd,
                            request.num,
                            request.arg0,
                            request.arg1,
                            request.arg2
                        )
                    };
                },
                MessageType::HIDEvent { ev_type, code, value } => {
                    // Update current modifiers state if a modifier key is pressed/released
                    match code {
                        // Left Shift
                        42 => if value == 1 { compositor.current_modifiers |= MOD_LSHIFT; } else if value == 0 { compositor.current_modifiers &= !MOD_LSHIFT; },
                        // Right Shift
                        54 => if value == 1 { compositor.current_modifiers |= MOD_RSHIFT; } else if value == 0 { compositor.current_modifiers &= !MOD_RSHIFT; },
                        // Left Ctrl
                        29 => if value == 1 { compositor.current_modifiers |= MOD_LCTRL; } else if value == 0 { compositor.current_modifiers &= !MOD_LCTRL; },
                        // Right Ctrl
                        97 => if value == 1 { compositor.current_modifiers |= MOD_RCTRL; } else if value == 0 { compositor.current_modifiers &= !MOD_RCTRL; },
                        // Left Alt
                        56 => if value == 1 { compositor.current_modifiers |= MOD_LALT; } else if value == 0 { compositor.current_modifiers &= !MOD_LALT; },
                        // Right Alt (AltGr)
                        100 => if value == 1 { compositor.current_modifiers |= MOD_RALT; } else if value == 0 { compositor.current_modifiers &= !MOD_RALT; },
                        // Left Super (Windows Key)
                        125 => if value == 1 { compositor.current_modifiers |= MOD_LSUPER; } else if value == 0 { compositor.current_modifiers &= !MOD_LSUPER; },
                        // Right Super
                        126 => if value == 1 { compositor.current_modifiers |= MOD_RSUPER; } else if value == 0 { compositor.current_modifiers &= !MOD_RSUPER; },
                        _ => {}
                    }

                    // Check for hotkeys first
                    let mut hotkey_matched = false;
                    for hotkey in &compositor.registered_hotkeys {
                        if hotkey.ev_type == ev_type && hotkey.code == code && hotkey.value == value && hotkey.modifiers == compositor.current_modifiers {
                            // Match found, forward to the registered PD
                            let hid_message = MessageType::HIDEvent { ev_type, code, value };
                            let mut forwarded_pdx_msg = PdxMessage {
                                msg_type: hid_message,
                                payload: [0; 64],
                            };
                            pdx_call(
                                    hotkey.pd_id,
                                    0,
                                    &mut forwarded_pdx_msg as *mut _ as u64,
                                    0,
                                );
                            serial_println!("sexdisplay: Hotkey matched and forwarded to PD {}", hotkey.pd_id);
                            hotkey_matched = true;
                            break;
                        }
                    }

                    if hotkey_matched {
                        // Hotkey handled, do not forward to focused window
                    } else {
                        // No hotkey matched, proceed with normal event forwarding
                        match ev_type {
                            sexdisplay::EV_REL => {
                                // Relative mouse movement
                                match code {
                                    sexdisplay::REL_X => {
                                        if value > 0 {
                                            compositor.cursor_x = compositor.cursor_x.saturating_add(value as u32).min(compositor.global_fb_width - 1);
                                        } else {
                                            compositor.cursor_x = compositor.cursor_x.saturating_sub(value.abs() as u32).max(0);
                                        }
                                    },
                                    sexdisplay::REL_Y => {
                                        if value > 0 {
                                            compositor.cursor_y = compositor.cursor_y.saturating_add(value as u32).min(compositor.global_fb_height - 1);
                                        } else {
                                            compositor.cursor_y = compositor.cursor_y.saturating_sub(value.abs() as u32).max(0);
                                        }
                                    },
                                    _ => {} // Ignore other relative events for now
                                }
                            },
                            sexdisplay::EV_KEY => {
                                // Mouse button events or other keys
                                match code {
                                    sexdisplay::BTN_LEFT | sexdisplay::BTN_RIGHT | sexdisplay::BTN_MIDDLE => {
                                        // Find the window under the cursor
                                        if let Some(window) = compositor.get_window_at_coords(compositor.cursor_x, compositor.cursor_y) {
                                            let hid_message = MessageType::HIDEvent { ev_type, code, value };
                                            let mut forwarded_pdx_msg = PdxMessage {
                                                msg_type: hid_message,
                                                payload: [0; 64],
                                            };
                                            pdx_call(
                                                    window.pd_id,
                                                    0,
                                                    &mut forwarded_pdx_msg as *mut _ as u64,
                                                    0,
                                                );
                                            serial_println!("sexdisplay: Forwarded Mouse Button Event to PD {}", window.pd_id);
                                        } else {
                                            serial_println!("sexdisplay: Mouse Button Event, no window at cursor. Global event?");
                                        }
                                    },
                                    _ => {
                                        // Other key events (e.g., keyboard) are still forwarded to focused window
                                        let focused_window_id = compositor.get_focused_window_id();
                                        if let Some(focused_id) = focused_window_id {
                                            if let Some(window) = compositor.find_window(focused_id) {
                                                let hid_message = MessageType::HIDEvent { ev_type, code, value };
                                                let mut forwarded_pdx_msg = PdxMessage {
                                                    msg_type: hid_message,
                                                    payload: [0; 64],
                                                };
                                                pdx_call(
                                                        window.pd_id,
                                                        0,
                                                        &mut forwarded_pdx_msg as *mut _ as u64,
                                                        0,
                                                    );
                                                serial_println!("sexdisplay: Forwarded HIDEvent to PD {}", window.pd_id);
                                            } else {
                                                serial_println!("sexdisplay: HIDEvent received, but focused window not found.");
                                            }
                                        } else {
                                            serial_println!("sexdisplay: HIDEvent received, but no window focused.");
                                        }
                                    }
                                }
                            },
                            _ => {
                                // All other HID events (e.g., keyboard, etc.) are forwarded to focused window
                                let focused_window_id = compositor.get_focused_window_id();
                                if let Some(focused_id) = focused_window_id {
                                    if let Some(window) = compositor.find_window(focused_id) {
                                        let hid_message = MessageType::HIDEvent { ev_type, code, value };
                                        let mut forwarded_pdx_msg = PdxMessage {
                                            msg_type: hid_message,
                                            payload: [0; 64],
                                        };
                                        pdx_call(
                                                window.pd_id,
                                                0,
                                                &mut forwarded_pdx_msg as *mut _ as u64,
                                                0,
                                            );
                                        serial_println!("sexdisplay: Forwarded HIDEvent to PD {}", window.pd_id);
                                    } else {
                                        serial_println!("sexdisplay: HIDEvent received, but focused window not found.");
                                    }
                                } else {
                                    serial_println!("sexdisplay: HIDEvent received, but no window focused.");
                                }
                            }
                        }
                    }
                },
                _ => {
                    serial_println!("sexdisplay: Received unhandled PDX message type.");
                }
            }
            pdx_reply(request.caller_pd, reply_val);
        } else {
            core::hint::spin_loop();
        }

        // 2. Evaluate and render the current frame
        unsafe { compositor.evaluate_and_render_frame(); }

        // 3. Commit each output's internal framebuffer to the actual display hardware
        for output_idx in 0..compositor.outputs.len() {
            let output = &compositor.outputs[output_idx];
            let pfn_base = compositor.internal_framebuffer_pfn_bases[output_idx];

            let pfn_list_ptr = &pfn_base as *const u64; // Pointer to the single PFN
            let num_pages = 1; // Assuming each output framebuffer is contiguous and fits in 1 PFN

            let commit_args_for_syscall = [
                pfn_list_ptr as u64,
                num_pages as u64,
                output.width as u64,
                output.height as u64,
                output.width as u64, // Assume stride = width
            ];

            unsafe {
                core::arch::asm!(
                    "syscall",
                    in("rax") 0u64,
                    in("rdi") PDX_COMPOSITOR_COMMIT,
                    in("rsi") commit_args_for_syscall.as_ptr() as u64,
                    options(nostack, preserves_flags)
                );
            }
        }
    }

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    serial_println!("sexdisplay PANIC: {}", _info); 
    loop { core::hint::spin_loop(); }
}
