#![no_std]
#![no_main]

use core::alloc::{GlobalAlloc, Layout};
use sex_pdx::{pdx_call, pdx_listen_raw, serial_println, OP_QUIL_PING, SLOT_DISPLAY};

struct DummyAllocator;
unsafe impl GlobalAlloc for DummyAllocator {
    unsafe fn alloc(&self, _: Layout) -> *mut u8 {
        core::ptr::null_mut()
    }
    unsafe fn dealloc(&self, _: *mut u8, _: Layout) {}
}
#[global_allocator]
static ALLOCATOR: DummyAllocator = DummyAllocator;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

const OP_HID_EVENT: u64 = 0x202;
const SURFACE_ID_QUIL: u64 = 201;

const QUIL_ROWS: u8 = 5;
const QUIL_BG_COLOR: u64 = 0x00151D31;
const QUIL_PANEL_COLOR: u64 = 0x001D2842;
const QUIL_ROW_INACTIVE: u64 = 0x00253556;
const QUIL_ROW_SELECTED: u64 = 0x004B6FD3;
const QUIL_ACCENT_COLOR: u64 = 0x00E9D36A;

const QUIL_PANEL_X: u64 = 24;
const QUIL_PANEL_Y: u64 = 24;
const QUIL_PANEL_W: u64 = 760;
const QUIL_PANEL_H: u64 = 520;
const QUIL_PANEL_PAD_X: u64 = 20;
const QUIL_PANEL_PAD_Y: u64 = 30;

const QUIL_ROW_X: u64 = QUIL_PANEL_X + QUIL_PANEL_PAD_X;
const QUIL_ROW_Y0: u64 = QUIL_PANEL_Y + QUIL_PANEL_PAD_Y;
const QUIL_ROW_W: u64 = QUIL_PANEL_W - (QUIL_PANEL_PAD_X * 2);
const QUIL_ROW_H: u64 = 46;
const QUIL_ROW_GAP: u64 = 14;
const QUIL_ACCENT_W: u64 = 8;

fn draw_palette(selected: u8) {
    // Fill bounded base to avoid a giant flat fullscreen look inside tiled boot layout.
    pdx_call(
        SLOT_DISPLAY,
        0xEF,
        SURFACE_ID_QUIL,
        0u64,
        (QUIL_BG_COLOR << 32) | (620u64 << 16) | 960u64,
    );
    // Inner panel area where palette rows live.
    pdx_call(
        SLOT_DISPLAY,
        0xEF,
        SURFACE_ID_QUIL,
        (QUIL_PANEL_Y << 32) | QUIL_PANEL_X,
        (QUIL_PANEL_COLOR << 32) | (QUIL_PANEL_H << 16) | QUIL_PANEL_W,
    );

    unsafe {
        static mut PALETTE_DRAW_BUDGET: u32 = 12;
        let b = &mut PALETTE_DRAW_BUDGET;
        if *b > 0 {
            *b -= 1;
            serial_println!("[quil.palette.draw] rows={} selected={}", QUIL_ROWS, selected);
        }
        static mut PALETTE_PANEL_BUDGET: u32 = 8;
        let pb = &mut PALETTE_PANEL_BUDGET;
        if *pb > 0 {
            *pb -= 1;
            serial_println!(
                "[quil.palette.panel] x={} y={} w={} h={}",
                QUIL_PANEL_X,
                QUIL_PANEL_Y,
                QUIL_PANEL_W,
                QUIL_PANEL_H
            );
        }
    }

    // Guard against row overflow in the bounded panel.
    let rows_bottom = QUIL_ROW_Y0 + QUIL_ROWS as u64 * QUIL_ROW_H + (QUIL_ROWS as u64 - 1) * QUIL_ROW_GAP;
    if rows_bottom > (QUIL_PANEL_Y + QUIL_PANEL_H) {
        serial_println!("[quil.palette.reject] action=draw reason=row_overflow");
    }

    let mut row = 0u8;
    while row < QUIL_ROWS {
        let y = QUIL_ROW_Y0 + (QUIL_ROW_H + QUIL_ROW_GAP) * row as u64;
        let is_selected = row == selected;
        let color = if is_selected {
            QUIL_ROW_SELECTED
        } else {
            QUIL_ROW_INACTIVE
        };

        pdx_call(
            SLOT_DISPLAY,
            0xEF,
            SURFACE_ID_QUIL,
            (y << 32) | QUIL_ROW_X,
            (color << 32) | (QUIL_ROW_H << 16) | QUIL_ROW_W,
        );

        if is_selected {
            pdx_call(
                SLOT_DISPLAY,
                0xEF,
                SURFACE_ID_QUIL,
                (y << 32) | QUIL_ROW_X,
                (QUIL_ACCENT_COLOR << 32) | (QUIL_ROW_H << 16) | QUIL_ACCENT_W,
            );
            unsafe {
                static mut PALETTE_SELECTED_BUDGET: u32 = 16;
                let b = &mut PALETTE_SELECTED_BUDGET;
                if *b > 0 {
                    *b -= 1;
                    serial_println!("[quil.palette.selected] row={}", row);
                }
            }
        }

        unsafe {
            static mut PALETTE_ROW_BUDGET: u32 = 20;
            let b = &mut PALETTE_ROW_BUDGET;
            if *b > 0 {
                *b -= 1;
                serial_println!(
                    "[quil.palette.row] row={} selected={}",
                    row,
                    if is_selected { 1 } else { 0 }
                );
            }
        }

        row += 1;
    }
}

fn decode_palette_key(scancode: u64) -> u8 {
    // 0=none, 1=up, 2=down, 3=enter, 4=esc.
    // Supports common keyboard scancode variants seen in this codebase.
    match scancode as u32 {
        0x48 | 0x67 | 103 => 1,
        0x50 | 0x6c | 108 => 2,
        0x1c | 0x0d | 28 => 3,
        0x01 | 0x1b | 1 => 4,
        _ => 0,
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    serial_println!("[quil.boot]");
    serial_println!("[quil.no_fb_write]");

    let mut selected_row: u8 = 0;
    let mut palette_active = true;

    // One-shot boot draw before listen loop.
    draw_palette(selected_row);
    serial_println!("[quil.boot.draw.ok]");

    serial_println!("[quil.ready]");

    loop {
        let msg = pdx_listen_raw(0);

        unsafe {
            static mut QUIL_LISTEN_BUDGET: u32 = 8;
            let b = &mut QUIL_LISTEN_BUDGET;
            if *b > 0 {
                *b -= 1;
                serial_println!("[quil.pdx.listen] type_id={:#x}", msg.type_id);
            }
        }

        match msg.type_id {
            OP_QUIL_PING => {
                unsafe {
                    static mut QUIL_ROUTE_BUDGET: u32 = 8;
                    let b = &mut QUIL_ROUTE_BUDGET;
                    if *b > 0 {
                        *b -= 1;
                        serial_println!("[quil.route.recv]");
                    }
                }
            }
            OP_HID_EVENT => {
                let scancode = msg.arg0;
                let value = msg.arg1;
                unsafe {
                    static mut QUIL_KEY_BUDGET: u32 = 16;
                    let b = &mut QUIL_KEY_BUDGET;
                    if *b > 0 {
                        *b -= 1;
                        serial_println!("[quil.key.recv] scancode={:#x} val={}", scancode, value);
                    }
                }

                if value == 1 {
                    let action = decode_palette_key(scancode);
                    unsafe {
                        static mut QUIL_PALETTE_KEY_BUDGET: u32 = 24;
                        let kb = &mut QUIL_PALETTE_KEY_BUDGET;
                        if *kb > 0 {
                            *kb -= 1;
                            serial_println!("[quil.palette.key] scancode={:#x} action={}", scancode, action);
                        }
                    }

                    match action {
                        1 => {
                            if palette_active {
                                selected_row = if selected_row == 0 {
                                    QUIL_ROWS - 1
                                } else {
                                    selected_row - 1
                                };
                                draw_palette(selected_row);
                            } else {
                                serial_println!("[quil.palette.reject] action=up reason=inactive");
                            }
                        }
                        2 => {
                            if palette_active {
                                selected_row = (selected_row + 1) % QUIL_ROWS;
                                draw_palette(selected_row);
                            } else {
                                serial_println!("[quil.palette.reject] action=down reason=inactive");
                            }
                        }
                        3 => {
                            if palette_active {
                                serial_println!("[quil.palette.action] kind=enter row={}", selected_row);
                            } else {
                                serial_println!("[quil.palette.reject] action=enter reason=inactive");
                            }
                        }
                        4 => {
                            if palette_active {
                                palette_active = false;
                                pdx_call(
                                    SLOT_DISPLAY,
                                    0xEF,
                                    SURFACE_ID_QUIL,
                                    0u64,
                                    (QUIL_BG_COLOR << 32) | (620u64 << 16) | 960u64,
                                );
                                serial_println!("[quil.palette.action] kind=esc clear=1");
                            } else {
                                serial_println!("[quil.palette.reject] action=esc reason=inactive");
                            }
                        }
                        _ => {
                            // Existing liveness fallback for non-palette keys.
                            static mut QUIL_COLOR_TOGGLE: bool = false;
                            unsafe {
                                QUIL_COLOR_TOGGLE = !QUIL_COLOR_TOGGLE;
                                let color = if QUIL_COLOR_TOGGLE {
                                    0x00FF00FFu64
                                } else {
                                    0x0000FFFFu64
                                };
                                pdx_call(
                                    SLOT_DISPLAY,
                                    0xEF,
                                    SURFACE_ID_QUIL,
                                    0,
                                    (color << 32) | (2000u64 << 16) | 2000u64,
                                );
                            }
                            serial_println!("[quil.palette.reject] action=key reason=unmapped");
                        }
                    }
                }
            }
            _ => {
                unsafe {
                    static mut QUIL_UNKNOWN_BUDGET: u32 = 8;
                    let b = &mut QUIL_UNKNOWN_BUDGET;
                    if *b > 0 {
                        *b -= 1;
                        serial_println!("[quil.unknown.yield] type_id={:#x}", msg.type_id);
                    }
                }
            }
        }
    }
}
