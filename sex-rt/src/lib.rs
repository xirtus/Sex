#![no_std]

// Fake sex_pdx crate for illustration
pub mod sex_pdx {
    pub struct PdxMessage {
        id: usize,
    }
    impl PdxMessage {
        pub fn new(id: usize) -> Self { Self { id } }
        pub fn append_arg(&mut self, _arg: usize) {}
        pub fn append_slice(&mut self, _buf: &[u8]) {}
        pub fn append_mut_slice(&mut self, _buf: &mut [u8]) {}
    }
    pub struct PdxChannel;
    impl PdxChannel {
        pub fn open(_name: &str) -> Option<Self> { Some(Self) }
        pub fn send_sync(&self, _msg: &PdxMessage) -> usize { 0 }
    }
}

use sex_pdx::{PdxChannel, PdxMessage};

#[global_allocator]
static ALLOCATOR: SimpleAllocator = SimpleAllocator;

pub struct SimpleAllocator;

unsafe impl core::alloc::GlobalAlloc for SimpleAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        // Mock: In SexOS SAS, we call sexnode/kernel to map pages.
        // For prototype, we use a fixed heap range 0x4000_0000.
        static mut HEAP_PTR: core::sync::atomic::AtomicU64 = core::sync::atomic::AtomicU64::new(0x4000_0000);
        let size = layout.size() as u64;
        let align = layout.align() as u64;
        
        let mut ptr = HEAP_PTR.load(core::sync::atomic::Ordering::Relaxed);
        ptr = (ptr + align - 1) & !(align - 1);
        HEAP_PTR.store(ptr + size, core::sync::atomic::Ordering::Relaxed);
        ptr as *mut u8
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {
        // Leak for prototype
    }
}

#[inline]
pub fn sys_write(fd: usize, buf: &[u8]) -> isize {
    // PKU zero-copy buffer map to sexfiles
    let mut msg = PdxMessage::new(1 /* WRITE */);
    msg.append_arg(fd);
    msg.append_slice(buf);
    
    let channel = PdxChannel::open("sexfiles").unwrap();
    channel.send_sync(&msg) as isize
}

#[inline]
pub fn sys_read(fd: usize, buf: &mut [u8]) -> isize {
    let mut msg = PdxMessage::new(0 /* READ */);
    msg.append_arg(fd);
    msg.append_mut_slice(buf);
    
    let channel = PdxChannel::open("sexfiles").unwrap();
    channel.send_sync(&msg) as isize
}

pub fn sys_exit(status: usize) -> ! {
    let mut msg = PdxMessage::new(60 /* EXIT */);
    msg.append_arg(status);
    let channel = PdxChannel::open("sexproc").unwrap();
    channel.send_sync(&msg);
    loop {}
}
