
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMessageType {
    BufferAlloc { width: u32, height: u32, format: u32 },
    BufferCommit { buffer_id: u32, damage_x: u32, damage_y: u32, damage_w: u32, damage_h: u32 },
    Modeset { width: u32, height: u32, refresh: u32 },
    Cursor { x: i32, y: i32, visible: bool, buffer_id: u32 },
    GeminiRepairDisplay,
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DisplayBufferReply {
    pub page_count: u32,
    pub pfn_list: [u64; 64],
    pub pku_key: u8,
}
