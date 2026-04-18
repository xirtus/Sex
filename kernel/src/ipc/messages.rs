pub use sex_pdx::MessageType;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Message {
    pub caller_pd: u32,
    pub msg_type: MessageType,
    pub arg0: u64,
    pub arg1: u64,
}

impl Message {
    pub fn new(msg_type: MessageType) -> Self {
        Self {
            caller_pd: 0,
            msg_type,
            arg0: 0,
            arg1: 0,
        }
    }

    pub fn msg_type(&self) -> MessageType {
        self.msg_type
    }
}
