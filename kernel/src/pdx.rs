//! Protection Domain Exchange (PDX) Message Passing

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct PdxFramebufferHandover {
    pub phys_addr: u64,
    pub width: u32,
    pub height: u32,
    pub pitch: u32,
    pub pkey: u32,
}

/// Enqueue a message to a domain's ring buffer.
pub fn send_to_domain<T>(domain: &str, payload: T) {
    // Scaffold: Atomic push to the SAS domain ringbuffer.
    // serial_println!("PDX: Handover payload sent to {} domain", domain);
}

/// Receive a message from the current domain's ring buffer.
pub fn receive<T>() -> T {
    // Scaffold: Atomic pop/spin-wait from the domain ringbuffer.
    // Returning a dummy payload to satisfy the compiler until ringbuffer is wired.
    unsafe { core::mem::zeroed() }
}
