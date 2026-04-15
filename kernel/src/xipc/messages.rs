#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MessageType {
    RawCall(u64),
    Signal { signo: u32, sender_capability_id: u64 },
    PageFault { fault_addr: u64, error_code: u32, pd_id: u64 },
    SpawnPD { path_ptr: u64 },
    DmaComplete { cap_id: u32, status: i32 },
    InputEvent { scancode: u8, flags: u8 },
}
