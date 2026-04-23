#![no_std]

use core::sync::atomic::{AtomicUsize, Ordering};

// Phase 19: Advanced Zero-Copy Runtime
const HEAP_START_VADDR: usize = 0x_4000_0000_0000; // Use a dedicated high virtual address range
static HEAP_TOP: AtomicUsize = AtomicUsize::new(0); // Initially unmapped
static HEAP_LIMIT: AtomicUsize = AtomicUsize::new(0); // Maximum mapped address

#[inline(always)]
fn align_up(addr: usize, align: usize) -> usize {
    (addr + align - 1) & !(align - 1)
}

#[global_allocator]
static ALLOCATOR: SimpleAllocator = SimpleAllocator;

pub struct SimpleAllocator;

/// Requests the kernel to allocate and map more pages to the user heap.
fn expand_heap(needed_size: usize) -> Result<usize, ()> {
    let mut current_limit = HEAP_LIMIT.load(Ordering::Acquire);
    let start_vaddr = if current_limit == 0 {
        HEAP_START_VADDR
    } else {
        current_limit
    };

    let size_aligned = align_up(needed_size, 4096);
    
    // 1. Allocate from kernel
    let pfn: u64;
    unsafe {
        core::arch::asm!("syscall",
            in("rax") 31, // PDX_ALLOCATE_MEMORY
            in("rdi") size_aligned as u64,
            lateout("rax") pfn,
            lateout("rcx") _, lateout("r11") _,
        );
    }
    if pfn == u64::MAX { return Err(()); }

    // 2. Map into user space
    let vaddr: u64;
    unsafe {
        core::arch::asm!("syscall",
            in("rax") 30, // PDX_MAP_MEMORY
            in("rdi") pfn,
            in("rsi") size_aligned as u64,
            lateout("rax") vaddr,
            lateout("rcx") _, lateout("r11") _,
        );
    }
    if vaddr == u64::MAX { return Err(()); }

    HEAP_LIMIT.store(start_vaddr + size_aligned, Ordering::Release);
    if HEAP_TOP.load(Ordering::Acquire) == 0 {
        HEAP_TOP.store(start_vaddr, Ordering::Release);
    }

    Ok(start_vaddr)
}

unsafe impl core::alloc::GlobalAlloc for SimpleAllocator {
    unsafe fn alloc(&self, layout: core::alloc::Layout) -> *mut u8 {
        let size = layout.size();
        let align = layout.align();

        if size == 0 { return core::ptr::null_mut(); }

        let mut current = HEAP_TOP.load(Ordering::Acquire);
        if current == 0 {
            if expand_heap(size + align).is_err() { return core::ptr::null_mut(); }
            current = HEAP_TOP.load(Ordering::Acquire);
        }

        loop {
            let aligned = align_up(current, align);
            let next = aligned.wrapping_add(size);
            let limit = HEAP_LIMIT.load(Ordering::Acquire);

            if next > limit {
                // Request more pages from kernel
                if expand_heap(size + align).is_err() {
                    return core::ptr::null_mut();
                }
                // Update limit and retry
                continue;
            }

            match HEAP_TOP.compare_exchange_weak(
                current,
                next,
                Ordering::AcqRel,
                Ordering::Relaxed,
            ) {
                Ok(_) => return aligned as *mut u8,
                Err(actual) => current = actual,
            }
        }
    }

    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: core::alloc::Layout) {
        // Bump allocator: no reclaim for now.
    }
}

// System calls provided to userland by sex-rt
#[inline]
pub fn sys_write(fd: usize, buf: &[u8]) -> isize {
    // Phase 19: PDX write stub
    let res: i64;
    unsafe {
        core::arch::asm!("syscall",
            in("rax") 27, // pdx_call
            in("rdi") 4,  // Target PID 4 (sexfiles)
            in("rsi") 2,  // PDX_WRITE
            in("rdx") fd as u64,
            in("r10") buf.as_ptr() as u64,
            lateout("rax") res,
            lateout("rcx") _, lateout("r11") _,
        );
    }
    res as isize
}

pub fn sys_exit(status: usize) -> ! {
    unsafe {
        core::arch::asm!("syscall",
            in("rax") 60, // sys_exit
            in("rdi") status as u64,
        );
    }
    loop {}
}

pub fn heap_init() {
    // Initial expansion
    let _ = expand_heap(65536);
}

#[no_mangle]
pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        *dest.add(i) = *src.add(i);
        i += 1;
    }
    dest
}

#[no_mangle]
pub unsafe extern "C" fn memset(s: *mut u8, c: i32, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        *s.add(i) = c as u8;
        i += 1;
    }
    s
}

#[no_mangle]
pub unsafe extern "C" fn memcmp(s1: *const u8, s2: *const u8, n: usize) -> i32 {
    let mut i = 0;
    while i < n {
        let a = *s1.add(i);
        let b = *s2.add(i);
        if a != b {
            return a as i32 - b as i32;
        }
        i += 1;
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if dest < src as *mut u8 {
        memcpy(dest, src, n)
    } else {
        let mut i = n;
        while i > 0 {
            i -= 1;
            *dest.add(i) = *src.add(i);
        }
        dest
    }
}

