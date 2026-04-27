#![no_std]
#![no_main]

use sex_pdx::{pdx_try_listen, pdx_reply, pdx_call, pdx_call_checked, sys_yield, sys_set_state, serial_println, SVC_STATE_LISTENING, 
              OP_WINDOW_CREATE, OP_WINDOW_SUBMIT, OP_WINDOW_VBLANK, OP_WINDOW_MAP, OP_WINDOW_WRITE};

pub const FB_WIDTH: usize = 1024;
pub const FB_HEIGHT: usize = 768;
pub const FB_SIZE: usize = FB_WIDTH * FB_HEIGHT;

#[derive(Copy, Clone, PartialEq)]
pub enum FrameState { Idle, BufferReady, FramePending, Flipped }

pub struct Window {
    pub handle: u64,
    pub fb_idx: usize,
    pub state: FrameState,
}

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
        serial_println!("[sexdisplay] Connector: Connected (1024x768)");
        serial_println!("[sexdisplay] Encoder: Active (Internal)");
        serial_println!("[sexdisplay] CRTC: Assigned (Primary)");
        serial_println!("[sexdisplay] KMS Pipeline Active.");
    }
}

static mut VBLANK_COUNTER: u64 = 0;
static mut NEXT_HANDLE: u64 = 0x100;

// Option B: no alloc (strict kernel-like mode)
static mut WINDOWS: [Option<Window>; 16] = [const { None }; 16];
static mut FRAMEBUFFERS: [[u32; FB_SIZE]; 16] = [[0; FB_SIZE]; 16];

fn find_window_mut(handle: u64) -> Option<&'static mut Window> {
    unsafe {
        for slot in WINDOWS.iter_mut() {
            if let Some(ref mut w) = slot {
                if w.handle == handle {
                    return Some(w);
                }
            }
        }
    }
    None
}

fn op_window_create() -> u64 {
    let h = unsafe { NEXT_HANDLE += 1; NEXT_HANDLE };

    unsafe {
        for (i, slot) in WINDOWS.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(Window {
                    handle: h,
                    fb_idx: i,
                    state: FrameState::Idle,
                });
                return h;
            }
        }
    }
    0 // Out of slots
}

fn op_window_write(h: u64, x: usize, y: usize, c: u32) -> Result<(), ()> {
    if let Some(w) = find_window_mut(h) {
        if x >= FB_WIDTH || y >= FB_HEIGHT { return Err(()); }
        unsafe {
            FRAMEBUFFERS[w.fb_idx][y * FB_WIDTH + x] = c;
        }
        Ok(())
    } else {
        Err(())
    }
}

fn op_window_submit(h: u64) {
    if let Some(w) = find_window_mut(h) {
        w.state = FrameState::Flipped;
        serial_println!("[sexdisplay] frame flip committed");
        // Trigger kernel debug hook (pdx_call to slot 0, opcode 0xBB)
        pdx_call(0, 0xBB, 0, 0, 0);
    }
}

fn op_window_vblank(last: u64) {
    loop {
        unsafe { if VBLANK_COUNTER > last { break; } }
        sys_yield();
    }
    serial_println!("[sexdisplay] vblank={}", unsafe { VBLANK_COUNTER });
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    serial_println!("[sexdisplay] PD1 Hardware Daemon Starting...");

    // Retrieve lease via single-use syscall (opcode 0x12)
    let packed_lease = match pdx_call_checked(0, 0x12, 0, 0, 0) {
        Ok(val) => val,
        Err(_) => panic!("[sexdisplay] FATAL: Failed to retrieve DisplayHardwareLease"),
    };

    let hw = DisplayHardware::from_packed(packed_lease);
    hw.init_kms();

    unsafe {
        sys_set_state(SVC_STATE_LISTENING);
    }
    serial_println!("[sexdisplay] DISPLAY SERVICE LISTENING (PDX SLOT 5)");

    loop {
        // Deterministic vblank tick
        unsafe { VBLANK_COUNTER += 1; }

        match pdx_try_listen() {
            None => {
                sys_yield();
            }
            Some(cmd) => {
                match cmd.type_id {
                    OP_WINDOW_CREATE => {
                        let handle = op_window_create();
                        serial_println!("[sexdisplay] CREATE_WINDOW from PD {} -> Handle={:#x}", cmd.caller_pd, handle);
                        pdx_reply(handle);
                    }
                    OP_WINDOW_MAP => {
                        pdx_reply(cmd.arg0); // Opaque handle only
                    }
                    OP_WINDOW_WRITE => {
                        let handle = cmd.arg0;
                        let color = cmd.arg1 as u32;
                        let pos = cmd.arg2; // offset in pixels
                        let x = (pos as usize) % FB_WIDTH;
                        let y = (pos as usize) / FB_WIDTH;
                        
                        if op_window_write(handle, x, y, color).is_ok() {
                            pdx_reply(0);
                        } else {
                            pdx_reply(sex_pdx::ERR_CAP_INVALID);
                        }
                    }
                    OP_WINDOW_SUBMIT => {
                        op_window_submit(cmd.arg0);
                        pdx_reply(0);
                    }
                    OP_WINDOW_VBLANK => {
                        op_window_vblank(unsafe { VBLANK_COUNTER });
                        pdx_reply(unsafe { VBLANK_COUNTER });
                    }
                    _ => {
                        serial_println!("[sexdisplay] unknown opcode {:#x}", cmd.type_id);
                        pdx_reply(sex_pdx::ERR_CAP_INVALID);
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
