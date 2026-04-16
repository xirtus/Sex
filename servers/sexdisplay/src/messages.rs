use sex_pdx::PageHandover;

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisplayMessageType {
    BufferAlloc { width: u32, height: u32, format: u32 },
    BufferCommit { page: PageHandover },
    Modeset { width: u32, height: u32, refresh: u32 },
    Cursor { x: i32, y: i32, visible: bool, buffer_id: u32 },
    GeminiRepairDisplay,
}

