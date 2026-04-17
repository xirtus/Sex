#![no_std]

use sex_pdx::{pdx_call, MessageType, DisplayProtocol, Rect, OrbitalEvent};

pub struct Window {
    id: u32,
    buffer: *mut u32,
    w: u32,
    h: u32,
}

impl Window {
    pub fn new(x: i32, y: i32, w: u32, h: u32, title: &str) -> Option<Self> {
        let mut title_buf = [0u8; 64];
        let len = title.len().min(64);
        title_buf[..len].copy_from_slice(&title.as_bytes()[..len]);

        // PDX Call: CreateWindow
        // We use arg1 as the pointer to the DisplayProtocol variant on stack.
        let msg = MessageType::Display(DisplayProtocol::CreateWindow { x, y, w, h, flags: 0, title: title_buf });
        let id = pdx_call(2 /* DISPLAY_PD */, 100 /* CREATE_WINDOW_CMD */, 
            &msg as *const _ as u64, 
            0) as u32;

        if id == 0 { return None; }

        // PDX Call: RequestBuffer
        // Returns the virtual address mapped by sexdisplay into this PD's address space.
        let msg_req = MessageType::Display(DisplayProtocol::RequestBuffer { window_id: id });
        let buf_ptr = pdx_call(2, 101 /* REQUEST_BUFFER_CMD */, 
            &msg_req as *const _ as u64, 
            0) as *mut u32;

        if buf_ptr.is_null() { return None; }

        Some(Self { id, buffer: buf_ptr, w, h })
    }

    pub fn sync(&self) {
        let msg = MessageType::Display(DisplayProtocol::CommitDamage { 
            window_id: self.id, 
            damage: Rect { x: 0, y: 0, w: self.w, h: self.h } 
        });
        pdx_call(2, 102 /* COMMIT_DAMAGE_CMD */, 
            &msg as *const _ as u64, 
            0);
    }

    pub fn data_mut(&mut self) -> &mut [u32] {
        unsafe { core::slice::from_raw_parts_mut(self.buffer, (self.w * self.h) as usize) }
    }
    
    pub fn events(&self) -> Option<OrbitalEvent> {
        let msg = MessageType::Display(DisplayProtocol::PollEvents { window_id: self.id });
        let res = pdx_call(2, 103 /* POLL_EVENTS_CMD */, 
            &msg as *const _ as u64, 
            0);
        
        if res == u64::MAX { return None; }
        
        // Safety: Assume res is a pointer or serialized event
        // In Sex PDX, small events can be returned in registers directly.
        Some(unsafe { *(res as *const OrbitalEvent) })
    }
}

impl Drop for Window {
    fn drop(&mut self) {
        let msg = MessageType::Display(DisplayProtocol::DestroyWindow { window_id: self.id });
        pdx_call(2, 104 /* DESTROY_WINDOW_CMD */, &msg as *const _ as u64, 0);
    }
}
