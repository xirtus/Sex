use crate::backends::FsBackend;
use crate::messages::PageHandover;
use sex_pdx::{pdx_call, MessageType};

/// DiskFs: Real Disk filesystem via sexdrive PDX.
pub struct DiskFs {
    sexdrive_cap: u32,
}

impl DiskFs {
    pub const fn new() -> Self {
        Self { sexdrive_cap: 1 }
    }
}

impl FsBackend for DiskFs {
    fn open(&self, _path: &str, _flags: u32, _mode: u32) -> Result<u64, i64> {
        Ok(100) // Mock inode
    }

    fn read(&self, _inode: u64, offset: u64, len: u32) -> Result<PageHandover, i64> {
        let msg = MessageType::DmaCall {
            command: 1, // READ
            offset,
            size: len as u64,
            buffer_cap: 2, // Assume slot 2 is pre-granted DMA buffer
            device_cap: 0,
        };

        // IPCtax: arg0 is the message pointer
        let res_ptr = pdx_call(self.sexdrive_cap, 0, &msg as *const _ as u64, 0);
        if res_ptr == 0 { return Err(-1); }

        let reply = unsafe { *(res_ptr as *const MessageType) };
        match reply {
            MessageType::DmaReply { status, .. } => {
                if status == 0 {
                    Ok(PageHandover { pfn: 0x2000, pku_key: 3 })
                } else {
                    Err(status)
                }
            },
            _ => Err(-1),
        }
    }

    fn write(&self, _inode: u64, offset: u64, len: u32, page: PageHandover) -> Result<u32, i64> {
        let msg = MessageType::DmaCall {
            command: 2, // WRITE
            offset,
            size: len as u64,
            buffer_cap: 2,
            device_cap: 0,
        };

        let res_ptr = pdx_call(self.sexdrive_cap, 0, &msg as *const _ as u64, 0);
        if res_ptr == 0 { return Err(-1); }

        let reply = unsafe { *(res_ptr as *const MessageType) };
        match reply {
            MessageType::DmaReply { status, .. } => {
                if status == 0 {
                    Ok(len)
                } else {
                    Err(status as i64)
                }
            },
            _ => Err(-1),
        }
    }

    fn close(&self, _inode: u64) -> Result<(), i64> {
        Ok(())
    }
}
