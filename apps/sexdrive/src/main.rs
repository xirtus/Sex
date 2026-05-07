#![no_std]
#![no_main]

use core::alloc::{GlobalAlloc, Layout};
use core::sync::atomic::{compiler_fence, Ordering};
use sex_pdx::{
    pdx_call, pdx_reply, pdx_try_listen_raw, serial_println,
    SLOT_SHELL, SLOT_USB_HOST,
    SLOT_BUF_LEND, sys_map_mem_lend,
    BLOCK_READ, BLOCK_WRITE, BLOCK_SYNC,
    BLOCK_ERR_BAD_CMD, BLOCK_ERR_BAD_LEN, BLOCK_ERR_NO_DEVICE,
    BLOCK_SECTOR_SIZE, BLOCK_MAX_XFER,
};

struct DummyAllocator;
unsafe impl GlobalAlloc for DummyAllocator {
    unsafe fn alloc(&self, _layout: Layout) -> *mut u8 { core::ptr::null_mut() }
    unsafe fn dealloc(&self, _ptr: *mut u8, _layout: Layout) {}
}

#[global_allocator]
static ALLOCATOR: DummyAllocator = DummyAllocator;

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop { core::hint::spin_loop(); }
}

#[no_mangle]
pub unsafe extern "C" fn memcmp(a: *const u8, b: *const u8, n: usize) -> i32 {
    for i in 0..n {
        let diff = *a.add(i) as i32 - *b.add(i) as i32;
        if diff != 0 { return diff; }
    }
    0
}

#[no_mangle]
pub unsafe extern "C" fn memcpy(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    for i in 0..n { *dest.add(i) = *src.add(i); }
    dest
}

#[no_mangle]
pub unsafe extern "C" fn memset(dest: *mut u8, c: i32, n: usize) -> *mut u8 {
    for i in 0..n { *dest.add(i) = c as u8; }
    dest
}

#[no_mangle]
pub unsafe extern "C" fn memmove(dest: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    if dest as usize <= src as usize {
        for i in 0..n { *dest.add(i) = *src.add(i); }
    } else {
        for i in (0..n).rev() { *dest.add(i) = *src.add(i); }
    }
    dest
}

// Local Opcode
pub const OP_SHELL_BIND_BUFFER: u64 = 0x14;

// SLOT_NVME_HOST = 16 — matches devmgr.rs grant. Not in sex-pdx to avoid ABI hash churn.
const SLOT_NVME_HOST: u64 = 16;
const PAGE_SIZE: u64 = 4096;
const NVME_SQ0TDBL: u64 = 0x1000;
const NVME_CQ0HDBL: u64 = 0x1004;
const NVME_LBA_BYTES: u64 = 512;
const WRITE_PROOF_LBA: u64 = 2047; // Final LBA in gate nvme.img (2048 sectors)
const WRITE_PROOF_LEN: u64 = 512;
const WRITE_PROOF_MAGIC: u64 = 0x3156_4554_4952_5753; // "SWRITEV1" LE
const MANIFEST_LBA: u64 = 2046;
const PROOF_OBJECT_START_LBA: u64 = 2038;
const PROOF_OBJECT_END_LBA: u64 = 2045;
// V2 multi-object slots (SEXFILES_DISK_MULTI_OBJECT_MANIFEST_PLAN_V1)
const LINEN_OBJECT_START_LBA: u64 = 2030;
const LINEN_OBJECT_END_LBA:   u64 = 2037;
const QUIL_OBJECT_START_LBA:  u64 = 2022;
const QUIL_OBJECT_END_LBA:    u64 = 2029;

struct NvmeIoState {
    ready: bool,
    map_va: u64,
    io_sq_va: u64,
    io_cq_va: u64,
    sq1tdbl: u64,
    cq1hdbl: u64,
    sq_tail: u32,
    cq_head: u32,
    cq_phase: u32,
    next_cid: u16,
}

static mut NVME_IO_STATE: NvmeIoState = NvmeIoState {
    ready: false,
    map_va: 0,
    io_sq_va: 0,
    io_cq_va: 0,
    sq1tdbl: 0,
    cq1hdbl: 0,
    sq_tail: 0,
    cq_head: 0,
    cq_phase: 1,
    next_cid: 0x0500,
};

fn sys_alloc_phys(size: u64) -> u64 {
    let phys: u64;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 31u64,
            in("rdi") size,
            lateout("rax") phys,
            out("rcx") _,
            out("r11") _,
        );
    }
    phys
}

fn sys_map_phys(phys: u64, size: u64) -> u64 {
    let va: u64;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 30u64,
            in("rdi") phys,
            in("rsi") size,
            lateout("rax") va,
            out("rcx") _,
            out("r11") _,
        );
    }
    va
}

fn nvme_read_into_bounce(offset: u64, size: u64) -> u64 {
    if size == 0
        || size > BLOCK_MAX_XFER
        || size > PAGE_SIZE
        || (offset % NVME_LBA_BYTES) != 0
        || (size % NVME_LBA_BYTES) != 0
    {
        serial_println!(
            "[sexdrive.block.read.api.err] reason=bad_len offset={:#x} size={} lba={}",
            offset, size, NVME_LBA_BYTES
        );
        return BLOCK_ERR_BAD_LEN;
    }

    let map_va: u64;
    let io_sq_va: u64;
    let io_cq_va: u64;
    let sq1tdbl: u64;
    let cq1hdbl: u64;
    let mut sq_tail: u32;
    let mut cq_head: u32;
    let mut cq_phase: u32;
    let cid: u16;
    unsafe {
        if !NVME_IO_STATE.ready {
            serial_println!("[sexdrive.block.read.api.err] reason=no_ioq_ready");
            return BLOCK_ERR_NO_DEVICE;
        }
        map_va = NVME_IO_STATE.map_va;
        io_sq_va = NVME_IO_STATE.io_sq_va;
        io_cq_va = NVME_IO_STATE.io_cq_va;
        sq1tdbl = NVME_IO_STATE.sq1tdbl;
        cq1hdbl = NVME_IO_STATE.cq1hdbl;
        sq_tail = NVME_IO_STATE.sq_tail;
        cq_head = NVME_IO_STATE.cq_head;
        cq_phase = NVME_IO_STATE.cq_phase;
        cid = NVME_IO_STATE.next_cid;
        NVME_IO_STATE.next_cid = NVME_IO_STATE.next_cid.wrapping_add(1);
    }

    let data_phys = sys_alloc_phys(PAGE_SIZE);
    if data_phys == 0 || data_phys == u64::MAX || (data_phys % PAGE_SIZE) != 0 {
        serial_println!(
            "[sexdrive.block.read.api.err] reason=data_alloc_invalid phys={:#x}",
            data_phys
        );
        return BLOCK_ERR_NO_DEVICE;
    }
    let data_va = sys_map_phys(data_phys, PAGE_SIZE);
    if data_va == 0 || data_va == u64::MAX {
        serial_println!(
            "[sexdrive.block.read.api.err] reason=data_map_failed va={:#x}",
            data_va
        );
        return BLOCK_ERR_NO_DEVICE;
    }
    unsafe {
        for i in 0..(PAGE_SIZE / 8) as usize {
            core::ptr::write_volatile((data_va as *mut u64).add(i), 0);
        }
    }

    let slba = offset / NVME_LBA_BYTES;
    let nlb = (size / NVME_LBA_BYTES).saturating_sub(1);

    let sqe_ptr = (io_sq_va as *mut u8).wrapping_add((sq_tail as usize) * 64) as *mut u32;
    unsafe {
        for i in 0..16usize {
            core::ptr::write_volatile(sqe_ptr.add(i), 0);
        }
        core::ptr::write_volatile(sqe_ptr.add(0), 0x02u32 | ((cid as u32) << 16)); // READ + CID
        core::ptr::write_volatile(sqe_ptr.add(1), 1u32); // NSID=1
        core::ptr::write_volatile(sqe_ptr.add(6), (data_phys & 0xFFFF_FFFF) as u32); // PRP1 low
        core::ptr::write_volatile(sqe_ptr.add(7), (data_phys >> 32) as u32); // PRP1 high
        core::ptr::write_volatile(sqe_ptr.add(10), (slba & 0xFFFF_FFFF) as u32); // SLBA low
        core::ptr::write_volatile(sqe_ptr.add(11), (slba >> 32) as u32); // SLBA high
        core::ptr::write_volatile(sqe_ptr.add(12), (nlb & 0xFFFF) as u32); // NLB (0-based)
    }
    serial_println!(
        "[sexdrive.block.read.api.nvme.submit] cid={} nsid={} slba={} nlb={} prp1={:#x} sq_tail={}",
        cid as u64, 1u64, slba, nlb, data_phys, sq_tail as u64
    );

    compiler_fence(Ordering::SeqCst);
    sq_tail = (sq_tail + 1) % 16;
    unsafe {
        core::ptr::write_volatile((map_va + sq1tdbl) as *mut u32, sq_tail);
    }

    let mut done = false;
    let mut dw2 = 0u32;
    let mut dw3 = 0u32;
    for _ in 0..1_000_000u32 {
        let cqe_ptr = (io_cq_va as *const u8).wrapping_add((cq_head as usize) * 16) as *const u32;
        let rd_dw2 = unsafe { core::ptr::read_volatile(cqe_ptr.add(2)) };
        let rd_dw3 = unsafe { core::ptr::read_volatile(cqe_ptr.add(3)) };
        let cid_now = rd_dw3 & 0xFFFF;
        let phase_now = (rd_dw3 >> 16) & 0x1;
        if cid_now == cid as u32 && phase_now == cq_phase {
            done = true;
            dw2 = rd_dw2;
            dw3 = rd_dw3;
            serial_println!(
                "[sexdrive.block.read.api.cqe] cid={} phase={} dw2={:#x} dw3={:#x}",
                cid_now as u64, phase_now as u64, rd_dw2 as u64, rd_dw3 as u64
            );
            break;
        }
    }
    if !done {
        serial_println!(
            "[sexdrive.block.read.api.err] reason=cqe_timeout cid={} head={} phase={}",
            cid as u64, cq_head as u64, cq_phase as u64
        );
        return BLOCK_ERR_NO_DEVICE;
    }

    let sf = (dw3 >> 17) & 0x7FFF;
    let sc = sf & 0xFF;
    let sct = (sf >> 8) & 0x7;
    if sc != 0 || sct != 0 {
        let sqhd = dw2 & 0xFFFF;
        let sqid = (dw2 >> 16) & 0xFFFF;
        serial_println!(
            "[sexdrive.block.read.api.err] reason=status_fail cid={} sc={} sct={} sqhd={} sqid={} dw3={:#x}",
            cid as u64, sc as u64, sct as u64, sqhd as u64, sqid as u64, dw3 as u64
        );
        return BLOCK_ERR_NO_DEVICE;
    }

    cq_head = (cq_head + 1) % 16;
    if cq_head == 0 {
        cq_phase ^= 1;
    }
    unsafe {
        core::ptr::write_volatile((map_va + cq1hdbl) as *mut u32, cq_head);
        NVME_IO_STATE.sq_tail = sq_tail;
        NVME_IO_STATE.cq_head = cq_head;
        NVME_IO_STATE.cq_phase = cq_phase;
    }

    let d0 = unsafe { core::ptr::read_volatile(data_va as *const u64) };
    let d1 = unsafe { core::ptr::read_volatile((data_va + 8) as *const u64) };
    serial_println!(
        "[sexdrive.block.read.api.ok] cid={} slba={} nlb={} d0={:#x} d1={:#x}",
        cid as u64, slba, nlb, d0, d1
    );
    0u64
}

fn nvme_read_into_mapped_va(offset: u64, size: u64, dst_va: u64) -> u64 {
    if size == 0
        || size > BLOCK_MAX_XFER
        || size > PAGE_SIZE
        || (offset % NVME_LBA_BYTES) != 0
        || (size % NVME_LBA_BYTES) != 0
        || dst_va == 0
        || dst_va == u64::MAX
    {
        serial_println!(
            "[sexdrive.block.read.handoff.err] reason=bad_args offset={:#x} size={} dst_va={:#x}",
            offset, size, dst_va
        );
        return BLOCK_ERR_BAD_LEN;
    }

    serial_println!(
        "[sexdrive.block.read.handoff.nvme.begin] offset={:#x} size={} dst_va={:#x}",
        offset, size, dst_va
    );

    let map_va: u64;
    let io_sq_va: u64;
    let io_cq_va: u64;
    let sq1tdbl: u64;
    let cq1hdbl: u64;
    let mut sq_tail: u32;
    let mut cq_head: u32;
    let mut cq_phase: u32;
    let cid: u16;
    unsafe {
        if !NVME_IO_STATE.ready {
            serial_println!("[sexdrive.block.read.handoff.err] reason=no_ioq_ready");
            return BLOCK_ERR_NO_DEVICE;
        }
        map_va = NVME_IO_STATE.map_va;
        io_sq_va = NVME_IO_STATE.io_sq_va;
        io_cq_va = NVME_IO_STATE.io_cq_va;
        sq1tdbl = NVME_IO_STATE.sq1tdbl;
        cq1hdbl = NVME_IO_STATE.cq1hdbl;
        sq_tail = NVME_IO_STATE.sq_tail;
        cq_head = NVME_IO_STATE.cq_head;
        cq_phase = NVME_IO_STATE.cq_phase;
        cid = NVME_IO_STATE.next_cid;
        NVME_IO_STATE.next_cid = NVME_IO_STATE.next_cid.wrapping_add(1);
    }

    let data_phys = sys_alloc_phys(PAGE_SIZE);
    if data_phys == 0 || data_phys == u64::MAX || (data_phys % PAGE_SIZE) != 0 {
        serial_println!(
            "[sexdrive.block.read.handoff.err] reason=data_alloc_invalid phys={:#x}",
            data_phys
        );
        return BLOCK_ERR_NO_DEVICE;
    }
    let data_va = sys_map_phys(data_phys, PAGE_SIZE);
    if data_va == 0 || data_va == u64::MAX {
        serial_println!(
            "[sexdrive.block.read.handoff.err] reason=data_map_failed va={:#x}",
            data_va
        );
        return BLOCK_ERR_NO_DEVICE;
    }
    unsafe {
        for i in 0..(PAGE_SIZE / 8) as usize {
            core::ptr::write_volatile((data_va as *mut u64).add(i), 0);
        }
    }

    let slba = offset / NVME_LBA_BYTES;
    let nlb = (size / NVME_LBA_BYTES).saturating_sub(1);
    let sqe_ptr = (io_sq_va as *mut u8).wrapping_add((sq_tail as usize) * 64) as *mut u32;
    unsafe {
        for i in 0..16usize {
            core::ptr::write_volatile(sqe_ptr.add(i), 0);
        }
        core::ptr::write_volatile(sqe_ptr.add(0), 0x02u32 | ((cid as u32) << 16)); // READ + CID
        core::ptr::write_volatile(sqe_ptr.add(1), 1u32); // NSID=1
        core::ptr::write_volatile(sqe_ptr.add(6), (data_phys & 0xFFFF_FFFF) as u32); // PRP1 low
        core::ptr::write_volatile(sqe_ptr.add(7), (data_phys >> 32) as u32); // PRP1 high
        core::ptr::write_volatile(sqe_ptr.add(10), (slba & 0xFFFF_FFFF) as u32); // SLBA low
        core::ptr::write_volatile(sqe_ptr.add(11), (slba >> 32) as u32); // SLBA high
        core::ptr::write_volatile(sqe_ptr.add(12), (nlb & 0xFFFF) as u32); // NLB (0-based)
    }
    compiler_fence(Ordering::SeqCst);
    sq_tail = (sq_tail + 1) % 16;
    unsafe {
        core::ptr::write_volatile((map_va + sq1tdbl) as *mut u32, sq_tail);
    }

    let mut done = false;
    let mut dw2 = 0u32;
    let mut dw3 = 0u32;
    for _ in 0..1_000_000u32 {
        let cqe_ptr = (io_cq_va as *const u8).wrapping_add((cq_head as usize) * 16) as *const u32;
        let rd_dw2 = unsafe { core::ptr::read_volatile(cqe_ptr.add(2)) };
        let rd_dw3 = unsafe { core::ptr::read_volatile(cqe_ptr.add(3)) };
        let cid_now = rd_dw3 & 0xFFFF;
        let phase_now = (rd_dw3 >> 16) & 0x1;
        if cid_now == cid as u32 && phase_now == cq_phase {
            done = true;
            dw2 = rd_dw2;
            dw3 = rd_dw3;
            serial_println!(
                "[sexdrive.block.read.handoff.nvme.cqe] cid={} phase={} dw2={:#x} dw3={:#x}",
                cid_now as u64, phase_now as u64, rd_dw2 as u64, rd_dw3 as u64
            );
            break;
        }
    }
    if !done {
        serial_println!(
            "[sexdrive.block.read.handoff.err] reason=cqe_timeout cid={} head={} phase={}",
            cid as u64, cq_head as u64, cq_phase as u64
        );
        return BLOCK_ERR_NO_DEVICE;
    }

    let sf = (dw3 >> 17) & 0x7FFF;
    let sc = sf & 0xFF;
    let sct = (sf >> 8) & 0x7;
    if sc != 0 || sct != 0 {
        let sqhd = dw2 & 0xFFFF;
        let sqid = (dw2 >> 16) & 0xFFFF;
        serial_println!(
            "[sexdrive.block.read.handoff.err] reason=status_fail cid={} sc={} sct={} sqhd={} sqid={} dw3={:#x}",
            cid as u64, sc as u64, sct as u64, sqhd as u64, sqid as u64, dw3 as u64
        );
        return BLOCK_ERR_NO_DEVICE;
    }

    cq_head = (cq_head + 1) % 16;
    if cq_head == 0 {
        cq_phase ^= 1;
    }
    unsafe {
        core::ptr::write_volatile((map_va + cq1hdbl) as *mut u32, cq_head);
        NVME_IO_STATE.sq_tail = sq_tail;
        NVME_IO_STATE.cq_head = cq_head;
        NVME_IO_STATE.cq_phase = cq_phase;
    }

    unsafe {
        let src = data_va as *const u8;
        let dst = dst_va as *mut u8;
        let mut i = 0usize;
        while i < size as usize {
            core::ptr::write_volatile(dst.add(i), core::ptr::read_volatile(src.add(i)));
            i += 1;
        }
    }
    serial_println!(
        "[sexdrive.block.read.handoff.copy.ok] phase=B len={}",
        size
    );
    0u64
}

fn write_guard_allows(offset: u64, size: u64, buf_cap: u64) -> bool {
    let expected_offset = WRITE_PROOF_LBA * NVME_LBA_BYTES;
    let manifest_offset = MANIFEST_LBA * NVME_LBA_BYTES;
    let object_start_offset = PROOF_OBJECT_START_LBA * NVME_LBA_BYTES;
    let object_end_offset = PROOF_OBJECT_END_LBA * NVME_LBA_BYTES;
    let proof_mode = buf_cap == SLOT_BUF_LEND;
    let allow_manifest = proof_mode && size == WRITE_PROOF_LEN && offset == manifest_offset;
    let linen_start = LINEN_OBJECT_START_LBA * NVME_LBA_BYTES;
    let linen_end   = LINEN_OBJECT_END_LBA   * NVME_LBA_BYTES;
    let quil_start  = QUIL_OBJECT_START_LBA  * NVME_LBA_BYTES;
    let quil_end    = QUIL_OBJECT_END_LBA    * NVME_LBA_BYTES;
    let allow_object = proof_mode && size == WRITE_PROOF_LEN
        && (offset % NVME_LBA_BYTES) == 0
        && offset >= object_start_offset
        && offset <= object_end_offset;
    let allow_linen = proof_mode && size == WRITE_PROOF_LEN
        && (offset % NVME_LBA_BYTES) == 0
        && offset >= linen_start && offset <= linen_end;
    let allow_quil = proof_mode && size == WRITE_PROOF_LEN
        && (offset % NVME_LBA_BYTES) == 0
        && offset >= quil_start && offset <= quil_end;
    let allow_proof = proof_mode && offset == expected_offset && size == WRITE_PROOF_LEN;
    let allow = allow_proof || allow_manifest || allow_object || allow_linen || allow_quil;
    serial_println!(
        "[sexdrive.write.guard.config] proof_lba={} proof_offset={:#x} manifest_lba={} object_lba_start={} object_lba_end={} proof_len={} magic={:#x}",
        WRITE_PROOF_LBA,
        expected_offset,
        MANIFEST_LBA,
        PROOF_OBJECT_START_LBA,
        PROOF_OBJECT_END_LBA,
        WRITE_PROOF_LEN,
        WRITE_PROOF_MAGIC
    );
    serial_println!(
        "[sexdrive.write.guard.begin] offset={:#x} size={} buf_cap={:#x} proof_mode={}",
        offset,
        size,
        buf_cap,
        if proof_mode { 1u64 } else { 0u64 }
    );
    if allow {
        serial_println!(
            "[sexdrive.write.guard.allow] offset={:#x} size={} buf_cap={:#x}",
            offset,
            size,
            buf_cap
        );
    } else {
        serial_println!(
            "[sexdrive.write.guard.deny] offset={:#x} size={} buf_cap={:#x}",
            offset,
            size,
            buf_cap
        );
    }
    allow
}

fn nvme_write_one_block(offset: u64, size: u64, src_va: u64) -> u64 {
    if size != WRITE_PROOF_LEN || src_va == 0 || src_va == u64::MAX {
        serial_println!(
            "[sexdrive.nvme.write.err] reason=bad_args offset={:#x} size={} src_va={:#x}",
            offset, size, src_va
        );
        return BLOCK_ERR_BAD_LEN;
    }

    let map_va: u64;
    let io_sq_va: u64;
    let io_cq_va: u64;
    let sq1tdbl: u64;
    let cq1hdbl: u64;
    let mut sq_tail: u32;
    let mut cq_head: u32;
    let mut cq_phase: u32;
    let write_cid: u16;
    unsafe {
        if !NVME_IO_STATE.ready {
            serial_println!("[sexdrive.nvme.write.err] reason=no_ioq_ready");
            return BLOCK_ERR_NO_DEVICE;
        }
        map_va = NVME_IO_STATE.map_va;
        io_sq_va = NVME_IO_STATE.io_sq_va;
        io_cq_va = NVME_IO_STATE.io_cq_va;
        sq1tdbl = NVME_IO_STATE.sq1tdbl;
        cq1hdbl = NVME_IO_STATE.cq1hdbl;
        sq_tail = NVME_IO_STATE.sq_tail;
        cq_head = NVME_IO_STATE.cq_head;
        cq_phase = NVME_IO_STATE.cq_phase;
        write_cid = NVME_IO_STATE.next_cid;
        NVME_IO_STATE.next_cid = NVME_IO_STATE.next_cid.wrapping_add(1);
    }

    let write_phys = sys_alloc_phys(PAGE_SIZE);
    let write_va = sys_map_phys(write_phys, PAGE_SIZE);
    if write_phys == 0 || write_phys == u64::MAX || (write_phys % PAGE_SIZE) != 0
        || write_va == 0 || write_va == u64::MAX
    {
        serial_println!(
            "[sexdrive.nvme.write.err] reason=write_buf_invalid phys={:#x} va={:#x}",
            write_phys, write_va
        );
        return BLOCK_ERR_NO_DEVICE;
    }
    unsafe {
        for i in 0..(PAGE_SIZE as usize) {
            core::ptr::write_volatile((write_va as *mut u8).add(i), 0u8);
        }
        let mut i = 0usize;
        while i < WRITE_PROOF_LEN as usize {
            let b = core::ptr::read_volatile((src_va as *const u8).add(i));
            core::ptr::write_volatile((write_va as *mut u8).add(i), b);
            i += 1;
        }
    }

    let slba = offset / NVME_LBA_BYTES;
    let nlb = 0u32;
    let sqe_ptr = (io_sq_va as *mut u8).wrapping_add((sq_tail as usize) * 64) as *mut u32;
    unsafe {
        for i in 0..16usize {
            core::ptr::write_volatile(sqe_ptr.add(i), 0);
        }
        core::ptr::write_volatile(sqe_ptr.add(0), 0x01u32 | ((write_cid as u32) << 16));
        core::ptr::write_volatile(sqe_ptr.add(1), 1u32);
        core::ptr::write_volatile(sqe_ptr.add(6), (write_phys & 0xFFFF_FFFF) as u32);
        core::ptr::write_volatile(sqe_ptr.add(7), (write_phys >> 32) as u32);
        core::ptr::write_volatile(sqe_ptr.add(10), (slba & 0xFFFF_FFFF) as u32);
        core::ptr::write_volatile(sqe_ptr.add(11), (slba >> 32) as u32);
        core::ptr::write_volatile(sqe_ptr.add(12), nlb);
    }
    serial_println!(
        "[sexdrive.nvme.write.submit] cid={} nsid=1 slba={} nlb=0 prp1={:#x} sq_tail={}",
        write_cid as u64, slba, write_phys, sq_tail as u64
    );
    serial_println!(
        "[sexdrive.block.write.api.nvme.submit] cid={} nsid=1 slba={} nlb=0 prp1={:#x}",
        write_cid as u64, slba, write_phys
    );

    compiler_fence(Ordering::SeqCst);
    sq_tail = (sq_tail + 1) % 16;
    unsafe {
        core::ptr::write_volatile((map_va + sq1tdbl) as *mut u32, sq_tail);
    }

    let mut done = false;
    let mut dw3 = 0u32;
    for _ in 0..1_000_000u32 {
        let cqe_ptr = (io_cq_va as *const u8).wrapping_add((cq_head as usize) * 16) as *const u32;
        let rd_dw2 = unsafe { core::ptr::read_volatile(cqe_ptr.add(2)) };
        let rd_dw3 = unsafe { core::ptr::read_volatile(cqe_ptr.add(3)) };
        let cid_now = rd_dw3 & 0xFFFF;
        let phase_now = (rd_dw3 >> 16) & 0x1;
        if cid_now == write_cid as u32 && phase_now == cq_phase {
            done = true;
            dw3 = rd_dw3;
            serial_println!(
                "[sexdrive.nvme.write.cqe] cid={} phase={} dw2={:#x} dw3={:#x}",
                cid_now as u64, phase_now as u64, rd_dw2 as u64, rd_dw3 as u64
            );
            serial_println!(
                "[sexdrive.block.write.api.cqe] cid={} phase={} dw2={:#x} dw3={:#x}",
                cid_now as u64, phase_now as u64, rd_dw2 as u64, rd_dw3 as u64
            );
            break;
        }
    }
    if !done {
        serial_println!(
            "[sexdrive.nvme.write.err] reason=cqe_timeout cid={} head={} phase={}",
            write_cid as u64, cq_head as u64, cq_phase as u64
        );
        return BLOCK_ERR_NO_DEVICE;
    }
    let sf = (dw3 >> 17) & 0x7FFF;
    let sc = sf & 0xFF;
    let sct = (sf >> 8) & 0x7;
    if sc != 0 || sct != 0 {
        serial_println!(
            "[sexdrive.nvme.write.err] reason=status_fail cid={} sc={} sct={} dw3={:#x}",
            write_cid as u64, sc as u64, sct as u64, dw3 as u64
        );
        return BLOCK_ERR_NO_DEVICE;
    }

    cq_head = (cq_head + 1) % 16;
    if cq_head == 0 {
        cq_phase ^= 1;
    }
    unsafe {
        core::ptr::write_volatile((map_va + cq1hdbl) as *mut u32, cq_head);
        NVME_IO_STATE.sq_tail = sq_tail;
        NVME_IO_STATE.cq_head = cq_head;
        NVME_IO_STATE.cq_phase = cq_phase;
    }
    serial_println!("[sexdrive.nvme.write.ok] cid={} slba={}", write_cid as u64, slba);
    serial_println!("[sexdrive.block.write.api.ok] cid={} slba={}", write_cid as u64, slba);
    0u64
}

fn nvme_write_readback_proof(offset: u64, size: u64, src_va: u64) -> u64 {
    if size != WRITE_PROOF_LEN || src_va == 0 || src_va == u64::MAX {
        serial_println!(
            "[sexdrive.nvme.write.err] reason=bad_args offset={:#x} size={} src_va={:#x}",
            offset, size, src_va
        );
        return BLOCK_ERR_BAD_LEN;
    }
    let expected_offset = WRITE_PROOF_LBA * NVME_LBA_BYTES;
    if offset != expected_offset {
        serial_println!(
            "[sexdrive.nvme.write.err] reason=guard_offset_mismatch offset={:#x} expected={:#x}",
            offset, expected_offset
        );
        return BLOCK_ERR_BAD_LEN;
    }

    serial_println!("[sexdrive.nvme.write.begin] offset={:#x} size={}", offset, size);
    serial_println!("[sexdrive.block.write.api.nvme.submit] begin=1 offset={:#x} size={}", offset, size);

    let map_va: u64;
    let io_sq_va: u64;
    let io_cq_va: u64;
    let sq1tdbl: u64;
    let cq1hdbl: u64;
    let mut sq_tail: u32;
    let mut cq_head: u32;
    let mut cq_phase: u32;
    let write_cid: u16;
    let read_cid: u16;
    unsafe {
        if !NVME_IO_STATE.ready {
            serial_println!("[sexdrive.nvme.write.err] reason=no_ioq_ready");
            return BLOCK_ERR_NO_DEVICE;
        }
        map_va = NVME_IO_STATE.map_va;
        io_sq_va = NVME_IO_STATE.io_sq_va;
        io_cq_va = NVME_IO_STATE.io_cq_va;
        sq1tdbl = NVME_IO_STATE.sq1tdbl;
        cq1hdbl = NVME_IO_STATE.cq1hdbl;
        sq_tail = NVME_IO_STATE.sq_tail;
        cq_head = NVME_IO_STATE.cq_head;
        cq_phase = NVME_IO_STATE.cq_phase;
        write_cid = NVME_IO_STATE.next_cid;
        NVME_IO_STATE.next_cid = NVME_IO_STATE.next_cid.wrapping_add(1);
        read_cid = NVME_IO_STATE.next_cid;
        NVME_IO_STATE.next_cid = NVME_IO_STATE.next_cid.wrapping_add(1);
    }

    let write_phys = sys_alloc_phys(PAGE_SIZE);
    let write_va = sys_map_phys(write_phys, PAGE_SIZE);
    if write_phys == 0 || write_phys == u64::MAX || (write_phys % PAGE_SIZE) != 0
        || write_va == 0 || write_va == u64::MAX
    {
        serial_println!(
            "[sexdrive.nvme.write.err] reason=write_buf_invalid phys={:#x} va={:#x}",
            write_phys, write_va
        );
        return BLOCK_ERR_NO_DEVICE;
    }
    unsafe {
        for i in 0..(PAGE_SIZE as usize) {
            core::ptr::write_volatile((write_va as *mut u8).add(i), 0u8);
        }
        let mut i = 0usize;
        while i < WRITE_PROOF_LEN as usize {
            let b = core::ptr::read_volatile((src_va as *const u8).add(i));
            core::ptr::write_volatile((write_va as *mut u8).add(i), b);
            i += 1;
        }
    }
    // Overwrite leading bytes with deterministic write marker.
    unsafe {
        core::ptr::write_volatile(write_va as *mut u64, WRITE_PROOF_MAGIC);
        core::ptr::write_volatile((write_va + 8) as *mut u64, WRITE_PROOF_LBA);
    }

    let slba = offset / NVME_LBA_BYTES;
    let nlb = 0u32;
    let sqe_ptr = (io_sq_va as *mut u8).wrapping_add((sq_tail as usize) * 64) as *mut u32;
    unsafe {
        for i in 0..16usize {
            core::ptr::write_volatile(sqe_ptr.add(i), 0);
        }
        core::ptr::write_volatile(sqe_ptr.add(0), 0x01u32 | ((write_cid as u32) << 16)); // WRITE + CID
        core::ptr::write_volatile(sqe_ptr.add(1), 1u32); // NSID=1
        core::ptr::write_volatile(sqe_ptr.add(6), (write_phys & 0xFFFF_FFFF) as u32); // PRP1 low
        core::ptr::write_volatile(sqe_ptr.add(7), (write_phys >> 32) as u32); // PRP1 high
        core::ptr::write_volatile(sqe_ptr.add(10), (slba & 0xFFFF_FFFF) as u32); // SLBA low
        core::ptr::write_volatile(sqe_ptr.add(11), (slba >> 32) as u32); // SLBA high
        core::ptr::write_volatile(sqe_ptr.add(12), nlb); // NLB=0 (one block)
    }
    serial_println!(
        "[sexdrive.nvme.write.submit] cid={} nsid=1 slba={} nlb=0 prp1={:#x} sq_tail={}",
        write_cid as u64, slba, write_phys, sq_tail as u64
    );
    serial_println!(
        "[sexdrive.block.write.api.nvme.submit] cid={} nsid=1 slba={} nlb=0 prp1={:#x}",
        write_cid as u64, slba, write_phys
    );

    compiler_fence(Ordering::SeqCst);
    sq_tail = (sq_tail + 1) % 16;
    unsafe {
        core::ptr::write_volatile((map_va + sq1tdbl) as *mut u32, sq_tail);
    }

    let mut done = false;
    let mut dw2 = 0u32;
    let mut dw3 = 0u32;
    for _ in 0..1_000_000u32 {
        let cqe_ptr = (io_cq_va as *const u8).wrapping_add((cq_head as usize) * 16) as *const u32;
        let rd_dw2 = unsafe { core::ptr::read_volatile(cqe_ptr.add(2)) };
        let rd_dw3 = unsafe { core::ptr::read_volatile(cqe_ptr.add(3)) };
        let cid_now = rd_dw3 & 0xFFFF;
        let phase_now = (rd_dw3 >> 16) & 0x1;
        if cid_now == write_cid as u32 && phase_now == cq_phase {
            done = true;
            dw2 = rd_dw2;
            dw3 = rd_dw3;
            serial_println!(
                "[sexdrive.nvme.write.cqe] cid={} phase={} dw2={:#x} dw3={:#x}",
                cid_now as u64, phase_now as u64, rd_dw2 as u64, rd_dw3 as u64
            );
            serial_println!(
                "[sexdrive.block.write.api.cqe] cid={} phase={} dw2={:#x} dw3={:#x}",
                cid_now as u64, phase_now as u64, rd_dw2 as u64, rd_dw3 as u64
            );
            break;
        }
    }
    if !done {
        serial_println!(
            "[sexdrive.nvme.write.err] reason=cqe_timeout cid={} head={} phase={}",
            write_cid as u64, cq_head as u64, cq_phase as u64
        );
        return BLOCK_ERR_NO_DEVICE;
    }
    let sf = (dw3 >> 17) & 0x7FFF;
    let sc = sf & 0xFF;
    let sct = (sf >> 8) & 0x7;
    if sc != 0 || sct != 0 {
        serial_println!(
            "[sexdrive.nvme.write.err] reason=status_fail cid={} sc={} sct={} dw3={:#x}",
            write_cid as u64, sc as u64, sct as u64, dw3 as u64
        );
        return BLOCK_ERR_NO_DEVICE;
    }
    cq_head = (cq_head + 1) % 16;
    if cq_head == 0 {
        cq_phase ^= 1;
    }
    unsafe {
        core::ptr::write_volatile((map_va + cq1hdbl) as *mut u32, cq_head);
    }
    serial_println!("[sexdrive.nvme.write.ok] cid={} slba={}", write_cid as u64, slba);
    serial_println!("[sexdrive.block.write.api.ok] cid={} slba={}", write_cid as u64, slba);

    // Readback verify from same LBA
    serial_println!("[sexdrive.nvme.write.readback.begin] slba={}", slba);
    let read_phys = sys_alloc_phys(PAGE_SIZE);
    let read_va = sys_map_phys(read_phys, PAGE_SIZE);
    if read_phys == 0 || read_phys == u64::MAX || (read_phys % PAGE_SIZE) != 0
        || read_va == 0 || read_va == u64::MAX
    {
        serial_println!(
            "[sexdrive.nvme.write.err] reason=readback_buf_invalid phys={:#x} va={:#x}",
            read_phys, read_va
        );
        return BLOCK_ERR_NO_DEVICE;
    }
    unsafe {
        for i in 0..(PAGE_SIZE as usize) {
            core::ptr::write_volatile((read_va as *mut u8).add(i), 0u8);
        }
    }

    let sqe_ptr2 = (io_sq_va as *mut u8).wrapping_add((sq_tail as usize) * 64) as *mut u32;
    unsafe {
        for i in 0..16usize {
            core::ptr::write_volatile(sqe_ptr2.add(i), 0);
        }
        core::ptr::write_volatile(sqe_ptr2.add(0), 0x02u32 | ((read_cid as u32) << 16)); // READ + CID
        core::ptr::write_volatile(sqe_ptr2.add(1), 1u32); // NSID=1
        core::ptr::write_volatile(sqe_ptr2.add(6), (read_phys & 0xFFFF_FFFF) as u32);
        core::ptr::write_volatile(sqe_ptr2.add(7), (read_phys >> 32) as u32);
        core::ptr::write_volatile(sqe_ptr2.add(10), (slba & 0xFFFF_FFFF) as u32);
        core::ptr::write_volatile(sqe_ptr2.add(11), (slba >> 32) as u32);
        core::ptr::write_volatile(sqe_ptr2.add(12), 0u32);
    }
    compiler_fence(Ordering::SeqCst);
    sq_tail = (sq_tail + 1) % 16;
    unsafe {
        core::ptr::write_volatile((map_va + sq1tdbl) as *mut u32, sq_tail);
    }

    let mut rb_done = false;
    let mut rb_dw2 = 0u32;
    let mut rb_dw3 = 0u32;
    for _ in 0..1_000_000u32 {
        let cqe_ptr = (io_cq_va as *const u8).wrapping_add((cq_head as usize) * 16) as *const u32;
        let rd_dw2 = unsafe { core::ptr::read_volatile(cqe_ptr.add(2)) };
        let rd_dw3 = unsafe { core::ptr::read_volatile(cqe_ptr.add(3)) };
        let cid_now = rd_dw3 & 0xFFFF;
        let phase_now = (rd_dw3 >> 16) & 0x1;
        if cid_now == read_cid as u32 && phase_now == cq_phase {
            rb_done = true;
            rb_dw2 = rd_dw2;
            rb_dw3 = rd_dw3;
            serial_println!(
                "[sexdrive.nvme.write.readback.cqe] cid={} phase={} dw2={:#x} dw3={:#x}",
                cid_now as u64, phase_now as u64, rd_dw2 as u64, rd_dw3 as u64
            );
            break;
        }
    }
    if !rb_done {
        serial_println!(
            "[sexdrive.nvme.write.err] reason=readback_timeout cid={} head={} phase={}",
            read_cid as u64, cq_head as u64, cq_phase as u64
        );
        return BLOCK_ERR_NO_DEVICE;
    }
    let rb_sf = (rb_dw3 >> 17) & 0x7FFF;
    let rb_sc = rb_sf & 0xFF;
    let rb_sct = (rb_sf >> 8) & 0x7;
    if rb_sc != 0 || rb_sct != 0 {
        serial_println!(
            "[sexdrive.nvme.write.err] reason=readback_status_fail cid={} sc={} sct={} dw3={:#x}",
            read_cid as u64, rb_sc as u64, rb_sct as u64, rb_dw3 as u64
        );
        return BLOCK_ERR_NO_DEVICE;
    }
    cq_head = (cq_head + 1) % 16;
    if cq_head == 0 {
        cq_phase ^= 1;
    }
    unsafe {
        core::ptr::write_volatile((map_va + cq1hdbl) as *mut u32, cq_head);
        NVME_IO_STATE.sq_tail = sq_tail;
        NVME_IO_STATE.cq_head = cq_head;
        NVME_IO_STATE.cq_phase = cq_phase;
    }

    let rb_magic = unsafe { core::ptr::read_volatile(read_va as *const u64) };
    if rb_magic == WRITE_PROOF_MAGIC {
        serial_println!(
            "[sexdrive.nvme.write.readback.match] magic={:#x} slba={}",
            rb_magic, slba
        );
        0u64
    } else {
        serial_println!(
            "[sexdrive.nvme.write.readback.mismatch] got={:#x} expect={:#x} slba={}",
            rb_magic, WRITE_PROOF_MAGIC, slba
        );
        BLOCK_ERR_NO_DEVICE
    }
}

/// [sexdrive.nvme.flush] — Issue NVMe FLUSH command (opcode 0x00) on IO queue 1.
/// No data transfer. Completes successfully only after all previously written
/// data is committed to non-volatile media (or CQE returns success in QEMU).
/// Returns 0 on success, BLOCK_ERR_NO_DEVICE on failure.
fn nvme_flush() -> u64 {
    let io_sq_va: u64;
    let io_cq_va: u64;
    let map_va: u64;
    let sq1tdbl: u64;
    let cq1hdbl: u64;
    let mut sq_tail: u32;
    let mut cq_head: u32;
    let mut cq_phase: u32;
    let flush_cid: u16;
    unsafe {
        if !NVME_IO_STATE.ready {
            serial_println!("[sexdrive.nvme.flush.err] reason=no_ioq_ready");
            return BLOCK_ERR_NO_DEVICE;
        }
        map_va = NVME_IO_STATE.map_va;
        io_sq_va = NVME_IO_STATE.io_sq_va;
        io_cq_va = NVME_IO_STATE.io_cq_va;
        sq1tdbl = NVME_IO_STATE.sq1tdbl;
        cq1hdbl = NVME_IO_STATE.cq1hdbl;
        sq_tail = NVME_IO_STATE.sq_tail;
        cq_head = NVME_IO_STATE.cq_head;
        cq_phase = NVME_IO_STATE.cq_phase;
        flush_cid = NVME_IO_STATE.next_cid;
        NVME_IO_STATE.next_cid = NVME_IO_STATE.next_cid.wrapping_add(1);
    }

    // Build FLUSH SQ entry: opcode=0x00, NSID=1, no data, no metadata.
    let sqe_ptr = unsafe {
        (io_sq_va as *mut u8).wrapping_add((sq_tail as usize) * 64) as *mut u32
    };
    unsafe {
        // Zero entire 64-byte SQ entry.
        let mut i = 0usize;
        while i < 16 {
            core::ptr::write_volatile(sqe_ptr.add(i), 0u32);
            i += 1;
        }
        // CDW0: opcode=0x00 (FLUSH), CID in bits 31:16.
        core::ptr::write_volatile(sqe_ptr.add(0), (flush_cid as u32) << 16);
        // CDW1: NSID=1.
        core::ptr::write_volatile(sqe_ptr.add(1), 1u32);
    }
    serial_println!(
        "[sexdrive.nvme.flush.submit] cid={} nsid=1 sq_tail={}",
        flush_cid as u64, sq_tail as u64
    );

    compiler_fence(Ordering::SeqCst);
    sq_tail = (sq_tail + 1) % 16;
    unsafe {
        core::ptr::write_volatile((map_va + sq1tdbl) as *mut u32, sq_tail);
    }

    // Poll CQE (short timeout — if not supported, return honest error).
    let mut done = false;
    let mut dw3 = 0u32;
    for _poll in 0..1000u32 {
        let cqe_ptr = unsafe {
            (io_cq_va as *const u8).wrapping_add((cq_head as usize) * 16) as *const u32
        };
        let rd_dw3 = unsafe { core::ptr::read_volatile(cqe_ptr.add(3)) };
        let cid_now = rd_dw3 & 0xFFFF;
        let phase_now = (rd_dw3 >> 16) & 0x1;
        if cid_now == flush_cid as u32 && phase_now == cq_phase {
            done = true;
            dw3 = rd_dw3;
            serial_println!(
                "[sexdrive.nvme.flush.cqe] cid={} phase={} dw3={:#x}",
                cid_now as u64, phase_now as u64, rd_dw3 as u64
            );
            break;
        }
        // Spin hint to avoid burning CPU while keeping it simple.
        core::hint::spin_loop();
    }
    if !done {
        serial_println!(
            "[sexdrive.nvme.flush.err] reason=cqe_timeout cid={} head={} phase={}",
            flush_cid as u64, cq_head as u64, cq_phase as u64
        );
        return BLOCK_ERR_NO_DEVICE;
    }
    let sf = (dw3 >> 17) & 0x7FFF;
    let sc = sf & 0xFF;
    let sct = (sf >> 8) & 0x7;
    if sc != 0 || sct != 0 {
        serial_println!(
            "[sexdrive.nvme.flush.err] reason=status_fail cid={} sc={} sct={} dw3={:#x}",
            flush_cid as u64, sc as u64, sct as u64, dw3 as u64
        );
        return BLOCK_ERR_NO_DEVICE;
    }

    cq_head = (cq_head + 1) % 16;
    if cq_head == 0 {
        cq_phase ^= 1;
    }
    unsafe {
        core::ptr::write_volatile((map_va + cq1hdbl) as *mut u32, cq_head);
        NVME_IO_STATE.sq_tail = sq_tail;
        NVME_IO_STATE.cq_head = cq_head;
        NVME_IO_STATE.cq_phase = cq_phase;
    }
    serial_println!("[sexdrive.nvme.flush.ok] cid={}", flush_cid as u64);
    0u64
}

fn nvme_probe_bar() {
    let map_va: u64;
    unsafe {
        // syscall 43 = MAP_PCI_BAR(cap_slot, bar_index, map_size)
        core::arch::asm!(
            "syscall",
            in("rax") 43u64,
            in("rdi") SLOT_NVME_HOST,
            in("rsi") 0u64,       // BAR0
            in("rdx") 0x4000u64,  // 16 KiB — covers NVMe CAP + property registers
            lateout("rax") map_va,
            out("rcx") _,
            out("r11") _,
        );
    }

    if map_va == u64::MAX || map_va == 0 {
        serial_println!(
            "[sexdrive.nvme.bar.resolve.begin] slot={} bar={} [sexdrive.nvme.bar.resolve.err] map_va={:#x} [sexdrive.device.no_nvme_cap]",
            SLOT_NVME_HOST, 0u64, map_va
        );
        return;
    }

    // BAR0 mapped: read NVMe identity registers via volatile MMIO loads.
    let nvme_cap = unsafe { core::ptr::read_volatile((map_va + 0x0000) as *const u64) };
    let nvme_vs = unsafe { core::ptr::read_volatile((map_va + 0x0008) as *const u32) };
    let nvme_cc = unsafe { core::ptr::read_volatile((map_va + 0x0014) as *const u32) };
    let nvme_csts = unsafe { core::ptr::read_volatile((map_va + 0x001C) as *const u32) };
    let nvme_aqa = unsafe { core::ptr::read_volatile((map_va + 0x0024) as *const u32) };
    let nvme_asq = unsafe { core::ptr::read_volatile((map_va + 0x0028) as *const u64) };
    let nvme_acq = unsafe { core::ptr::read_volatile((map_va + 0x0030) as *const u64) };

    let cap_mqes = (nvme_cap & 0xFFFF) as u16;
    let cap_dstrd = ((nvme_cap >> 32) & 0xF) as u8;
    let vs_major = ((nvme_vs >> 16) & 0xFFFF) as u16;
    let vs_minor = ((nvme_vs >> 8) & 0xFF) as u8;
    let csts_rdy = (nvme_csts & 0x1) != 0;
    let cc_en = (nvme_cc & 0x1) != 0;
    serial_println!(
        "[sexdrive.nvme.bar.resolve.begin] slot={} bar={} [sexdrive.nvme.bar.resolve.ok] map_va={:#x} [sexdrive.device.nvme_cap.present] va={:#x} cap={:#x}",
        SLOT_NVME_HOST, 0u64, map_va, map_va, nvme_cap
    );
    serial_println!(
        "[sexdrive.nvme.reg.cap] cap={:#x} mqes={} dstrd={}",
        nvme_cap, cap_mqes, cap_dstrd
    );
    serial_println!(
        "[sexdrive.nvme.reg.vs] vs={:#x} major={} minor={}",
        nvme_vs, vs_major, vs_minor
    );
    serial_println!("[sexdrive.nvme.reg.cc] cc={:#x}", nvme_cc);
    serial_println!(
        "[sexdrive.nvme.reg.csts] csts={:#x} rdy={}",
        nvme_csts,
        if csts_rdy { 1u64 } else { 0u64 }
    );

    if nvme_cap != 0 && nvme_vs != 0 {
        serial_println!(
            "[sexdrive.nvme.identity.ok] cap={:#x} vs={:#x} mqes={} dstrd={} major={} minor={} csts_rdy={}",
            nvme_cap,
            nvme_vs,
            cap_mqes,
            cap_dstrd,
            vs_major,
            vs_minor,
            if csts_rdy { 1u64 } else { 0u64 }
        );
    } else {
        serial_println!(
            "[sexdrive.nvme.identity.err] cap={:#x} vs={:#x} cc={:#x} csts={:#x}",
            nvme_cap,
            nvme_vs,
            nvme_cc,
            nvme_csts
        );
    }

    serial_println!(
        "[sexdrive.nvme.queue.inspect] aqa={:#x} asq={:#x} acq={:#x} cc={:#x} csts={:#x}",
        nvme_aqa,
        nvme_asq,
        nvme_acq,
        nvme_cc,
        nvme_csts
    );
    serial_println!("[sexdrive.nvme.queue.aqa] aqa={:#x}", nvme_aqa);
    serial_println!("[sexdrive.nvme.queue.asq] asq={:#x}", nvme_asq);
    serial_println!("[sexdrive.nvme.queue.acq] acq={:#x}", nvme_acq);

    let queues_preconfigured = nvme_aqa != 0 && nvme_asq != 0 && nvme_acq != 0;
    if queues_preconfigured {
        serial_println!(
            "[sexdrive.nvme.queue.ready] mode=preconfigured cc_en={} csts_rdy={} aqa={:#x} asq={:#x} acq={:#x}",
            if cc_en { 1u64 } else { 0u64 },
            if csts_rdy { 1u64 } else { 0u64 },
            nvme_aqa,
            nvme_asq,
            nvme_acq
        );
    } else {
        if cc_en || csts_rdy {
            serial_println!(
                "[sexdrive.nvme.queue.err] reason=stop_first_controller_enabled_queue_program_requires_disable cc_en={} csts_rdy={} aqa={:#x} asq={:#x} acq={:#x}",
                if cc_en { 1u64 } else { 0u64 },
                if csts_rdy { 1u64 } else { 0u64 },
                nvme_aqa,
                nvme_asq,
                nvme_acq
            );
            return;
        }

        let asq_phys = sys_alloc_phys(PAGE_SIZE);
        let acq_phys = sys_alloc_phys(PAGE_SIZE);
        if asq_phys == 0 || asq_phys == u64::MAX || acq_phys == 0 || acq_phys == u64::MAX {
            serial_println!(
                "[sexdrive.nvme.queue.err] reason=alloc_failed asq_phys={:#x} acq_phys={:#x}",
                asq_phys,
                acq_phys
            );
            return;
        }
        if (asq_phys % PAGE_SIZE) != 0 || (acq_phys % PAGE_SIZE) != 0 {
            serial_println!(
                "[sexdrive.nvme.queue.err] reason=alloc_unaligned asq_phys={:#x} acq_phys={:#x}",
                asq_phys,
                acq_phys
            );
            return;
        }

        let asq_va = sys_map_phys(asq_phys, PAGE_SIZE);
        let acq_va = sys_map_phys(acq_phys, PAGE_SIZE);
        if asq_va == 0 || asq_va == u64::MAX || acq_va == 0 || acq_va == u64::MAX {
            serial_println!(
                "[sexdrive.nvme.queue.err] reason=map_failed asq_va={:#x} acq_va={:#x}",
                asq_va,
                acq_va
            );
            return;
        }

        let asq_words = (PAGE_SIZE / 8) as usize;
        let acq_words = (PAGE_SIZE / 8) as usize;
        unsafe {
            for i in 0..asq_words {
                core::ptr::write_volatile((asq_va as *mut u64).add(i), 0);
            }
            for i in 0..acq_words {
                core::ptr::write_volatile((acq_va as *mut u64).add(i), 0);
            }
        }

        serial_println!(
            "[sexdrive.nvme.queue.alloc.ok] asq_phys={:#x} asq_va={:#x} acq_phys={:#x} acq_va={:#x}",
            asq_phys,
            asq_va,
            acq_phys,
            acq_va
        );

        let aqa = ((16u32 - 1) << 16) | (16u32 - 1);
        unsafe {
            core::ptr::write_volatile((map_va + 0x0024) as *mut u32, aqa);
            core::ptr::write_volatile((map_va + 0x0028) as *mut u64, asq_phys);
            core::ptr::write_volatile((map_va + 0x0030) as *mut u64, acq_phys);
        }

        let aqa_rb = unsafe { core::ptr::read_volatile((map_va + 0x0024) as *const u32) };
        let asq_rb = unsafe { core::ptr::read_volatile((map_va + 0x0028) as *const u64) };
        let acq_rb = unsafe { core::ptr::read_volatile((map_va + 0x0030) as *const u64) };
        serial_println!(
            "[sexdrive.nvme.queue.program.ok] aqa={:#x} asq={:#x} acq={:#x}",
            aqa_rb,
            asq_rb,
            acq_rb
        );
        serial_println!(
            "[sexdrive.nvme.queue.ready] mode=programmed cc_en=0 csts_rdy=0 aqa={:#x} asq={:#x} acq={:#x}",
            aqa_rb,
            asq_rb,
            acq_rb
        );
    }

    let old_cc = nvme_cc;
    let old_csts = nvme_csts;
    let old_aqa = nvme_aqa;
    let old_asq = nvme_asq;
    let old_acq = nvme_acq;
    let cap_to = ((nvme_cap >> 24) & 0xFF) as u32;

    serial_println!(
        "[sexdrive.nvme.reprovision.begin] old_cc={:#x} old_csts={:#x} old_aqa={:#x} old_asq={:#x} old_acq={:#x} cap_to={} dstrd={}",
        old_cc,
        old_csts,
        old_aqa,
        old_asq,
        old_acq,
        cap_to as u64,
        cap_dstrd as u64
    );

    serial_println!(
        "[sexdrive.nvme.reprovision.disable.begin] cc_old={:#x} csts_old={:#x}",
        old_cc,
        old_csts
    );
    let cc_disabled = old_cc & !1u32;
    unsafe {
        core::ptr::write_volatile((map_va + 0x0014) as *mut u32, cc_disabled);
    }

    let disable_poll_limit = if cap_to == 0 { 1_000_000u32 } else { (cap_to * 20_000).max(200_000) };
    let mut disable_polls = 0u32;
    let mut rdy0 = false;
    while disable_polls < disable_poll_limit {
        let csts_now = unsafe { core::ptr::read_volatile((map_va + 0x001C) as *const u32) };
        if (csts_now & 0x1) == 0 {
            rdy0 = true;
            serial_println!(
                "[sexdrive.nvme.reprovision.rdy0] polls={} csts={:#x} cc={:#x}",
                disable_polls as u64,
                csts_now as u64,
                cc_disabled as u64
            );
            break;
        }
        disable_polls += 1;
    }
    if !rdy0 {
        let csts_now = unsafe { core::ptr::read_volatile((map_va + 0x001C) as *const u32) };
        serial_println!(
            "[sexdrive.nvme.reprovision.err] reason=disable_rdy0_timeout polls={} limit={} csts={:#x} cc={:#x}",
            disable_polls as u64,
            disable_poll_limit as u64,
            csts_now as u64,
            cc_disabled as u64
        );
        return;
    }

    let asq_phys = sys_alloc_phys(PAGE_SIZE);
    let acq_phys = sys_alloc_phys(PAGE_SIZE);
    if asq_phys == 0 || asq_phys == u64::MAX || acq_phys == 0 || acq_phys == u64::MAX {
        serial_println!(
            "[sexdrive.nvme.reprovision.err] reason=alloc_failed asq_phys={:#x} acq_phys={:#x}",
            asq_phys,
            acq_phys
        );
        return;
    }
    if (asq_phys % PAGE_SIZE) != 0 || (acq_phys % PAGE_SIZE) != 0 {
        serial_println!(
            "[sexdrive.nvme.reprovision.err] reason=alloc_unaligned asq_phys={:#x} acq_phys={:#x}",
            asq_phys,
            acq_phys
        );
        return;
    }

    let asq_va = sys_map_phys(asq_phys, PAGE_SIZE);
    let acq_va = sys_map_phys(acq_phys, PAGE_SIZE);
    if asq_va == 0 || asq_va == u64::MAX || acq_va == 0 || acq_va == u64::MAX {
        serial_println!(
            "[sexdrive.nvme.reprovision.err] reason=map_failed asq_va={:#x} acq_va={:#x}",
            asq_va,
            acq_va
        );
        return;
    }

    let words = (PAGE_SIZE / 8) as usize;
    unsafe {
        for i in 0..words {
            core::ptr::write_volatile((asq_va as *mut u64).add(i), 0);
            core::ptr::write_volatile((acq_va as *mut u64).add(i), 0);
        }
    }
    serial_println!(
        "[sexdrive.nvme.reprovision.alloc.ok] asq_phys={:#x} asq_va={:#x} acq_phys={:#x} acq_va={:#x}",
        asq_phys,
        asq_va,
        acq_phys,
        acq_va
    );

    let new_aqa = ((16u32 - 1) << 16) | (16u32 - 1);
    unsafe {
        core::ptr::write_volatile((map_va + 0x0024) as *mut u32, new_aqa);
        core::ptr::write_volatile((map_va + 0x0028) as *mut u64, asq_phys);
        core::ptr::write_volatile((map_va + 0x0030) as *mut u64, acq_phys);
    }
    serial_println!("[sexdrive.nvme.reprovision.program.aqa] aqa={:#x}", new_aqa as u64);
    serial_println!("[sexdrive.nvme.reprovision.program.asq] asq={:#x}", asq_phys);
    serial_println!("[sexdrive.nvme.reprovision.program.acq] acq={:#x}", acq_phys);

    serial_println!("[sexdrive.nvme.reprovision.enable.begin] cc_before={:#x}", cc_disabled as u64);
    let cc_enabled = cc_disabled | 1u32;
    unsafe {
        core::ptr::write_volatile((map_va + 0x0014) as *mut u32, cc_enabled);
    }

    let enable_poll_limit = if cap_to == 0 { 1_000_000u32 } else { (cap_to * 20_000).max(200_000) };
    let mut enable_polls = 0u32;
    let mut rdy1 = false;
    while enable_polls < enable_poll_limit {
        let csts_now = unsafe { core::ptr::read_volatile((map_va + 0x001C) as *const u32) };
        if (csts_now & 0x1) != 0 {
            rdy1 = true;
            serial_println!(
                "[sexdrive.nvme.reprovision.rdy1] polls={} csts={:#x} cc={:#x}",
                enable_polls as u64,
                csts_now as u64,
                cc_enabled as u64
            );
            break;
        }
        enable_polls += 1;
    }
    if !rdy1 {
        let csts_now = unsafe { core::ptr::read_volatile((map_va + 0x001C) as *const u32) };
        serial_println!(
            "[sexdrive.nvme.reprovision.err] reason=enable_rdy1_timeout polls={} limit={} csts={:#x} cc={:#x}",
            enable_polls as u64,
            enable_poll_limit as u64,
            csts_now as u64,
            cc_enabled as u64
        );
        return;
    }

    let rb_cc = unsafe { core::ptr::read_volatile((map_va + 0x0014) as *const u32) };
    let rb_csts = unsafe { core::ptr::read_volatile((map_va + 0x001C) as *const u32) };
    let rb_aqa = unsafe { core::ptr::read_volatile((map_va + 0x0024) as *const u32) };
    let rb_asq = unsafe { core::ptr::read_volatile((map_va + 0x0028) as *const u64) };
    let rb_acq = unsafe { core::ptr::read_volatile((map_va + 0x0030) as *const u64) };
    if rb_aqa != new_aqa || rb_asq != asq_phys || rb_acq != acq_phys || (rb_cc & 1) == 0 || (rb_csts & 1) == 0 {
        serial_println!(
            "[sexdrive.nvme.reprovision.err] reason=readback_mismatch cc={:#x} csts={:#x} aqa={:#x} asq={:#x} acq={:#x}",
            rb_cc as u64,
            rb_csts as u64,
            rb_aqa as u64,
            rb_asq,
            rb_acq
        );
        return;
    }
    serial_println!(
        "[sexdrive.nvme.reprovision.ok] cc={:#x} csts={:#x} aqa={:#x} asq={:#x} acq={:#x} disable_polls={} enable_polls={}",
        rb_cc as u64,
        rb_csts as u64,
        rb_aqa as u64,
        rb_asq,
        rb_acq,
        disable_polls as u64,
        enable_polls as u64
    );

    // Identify retry V2 with corrected CQE decode.
    let asq_owned_phys = rb_asq;
    let acq_owned_phys = rb_acq;
    let asq_owned_va = asq_va;
    let acq_owned_va = acq_va;
    let q_entries: u32 = 16;
    let mut sq_tail: u32 = 0;
    let mut cq_head: u32 = 0;
    let mut cq_phase: u32 = 1;

    let cc_css = (rb_cc >> 4) & 0x7;
    let cc_mps = (rb_cc >> 7) & 0xF;
    let cc_ams = (rb_cc >> 11) & 0x7;
    let cc_iosqes = (rb_cc >> 16) & 0xF;
    let cc_iocqes = (rb_cc >> 20) & 0xF;

    serial_println!(
        "[sexdrive.nvme.admin.identify.v2.begin] asq_phys={:#x} asq_va={:#x} acq_phys={:#x} acq_va={:#x} q_entries={} sq_head={} sq_tail={} cq_head={} cq_phase={}",
        asq_owned_phys,
        asq_owned_va,
        acq_owned_phys,
        acq_owned_va,
        q_entries as u64,
        0u64,
        sq_tail as u64,
        cq_head as u64,
        cq_phase as u64
    );
    serial_println!(
        "[sexdrive.nvme.admin.identify.v2.cqe.decode] cc={:#x} en={} css={} mps={} ams={} iosqes={} iocqes={} dstrd={} sq0tdbl={:#x} cq0hdbl={:#x}",
        rb_cc as u64,
        (rb_cc & 1) as u64,
        cc_css as u64,
        cc_mps as u64,
        cc_ams as u64,
        cc_iosqes as u64,
        cc_iocqes as u64,
        cap_dstrd as u64,
        NVME_SQ0TDBL,
        NVME_CQ0HDBL
    );

    if cc_iosqes != 6 || cc_iocqes != 4 || cc_mps != 0 {
        serial_println!(
            "[sexdrive.nvme.admin.identify.v2.err] reason=cc_layout_invalid cc={:#x} iosqes={} iocqes={} mps={}",
            rb_cc as u64,
            cc_iosqes as u64,
            cc_iocqes as u64,
            cc_mps as u64
        );
        return;
    }

    let acq0_ptr = acq_owned_va as *const u32;
    let acq0_dw0_before = unsafe { core::ptr::read_volatile(acq0_ptr.add(0)) };
    let acq0_dw1_before = unsafe { core::ptr::read_volatile(acq0_ptr.add(1)) };
    let acq0_dw2_before = unsafe { core::ptr::read_volatile(acq0_ptr.add(2)) };
    let acq0_dw3_before = unsafe { core::ptr::read_volatile(acq0_ptr.add(3)) };
    serial_println!(
        "[sexdrive.nvme.admin.identify.v2.cqe.raw] stage=before dw0={:#x} dw1={:#x} dw2={:#x} dw3={:#x} phase={}",
        acq0_dw0_before as u64,
        acq0_dw1_before as u64,
        acq0_dw2_before as u64,
        acq0_dw3_before as u64,
        (acq0_dw3_before & 0x1) as u64
    );

    let id_phys = sys_alloc_phys(PAGE_SIZE);
    if id_phys == 0 || id_phys == u64::MAX || (id_phys % PAGE_SIZE) != 0 {
        serial_println!(
            "[sexdrive.nvme.admin.identify.v2.err] reason=id_alloc_invalid phys={:#x}",
            id_phys
        );
        return;
    }
    let id_va = sys_map_phys(id_phys, PAGE_SIZE);
    if id_va == 0 || id_va == u64::MAX {
        serial_println!(
            "[sexdrive.nvme.admin.identify.v2.err] reason=id_map_failed va={:#x}",
            id_va
        );
        return;
    }

    unsafe {
        for i in 0..(PAGE_SIZE / 8) as usize {
            core::ptr::write_volatile((id_va as *mut u64).add(i), 0);
        }
    }

    let cid: u16 = 0x0042;
    let sqe_ptr = (asq_owned_va as *mut u8).wrapping_add((sq_tail as usize) * 64) as *mut u32;
    unsafe {
        for i in 0..16usize {
            core::ptr::write_volatile(sqe_ptr.add(i), 0);
        }
        // CDW0: OPC=0x06 Identify, CID in bits 31:16
        core::ptr::write_volatile(sqe_ptr.add(0), 0x06u32 | ((cid as u32) << 16));
        // PRP1 in DW6/DW7
        core::ptr::write_volatile(sqe_ptr.add(6), (id_phys & 0xFFFF_FFFF) as u32);
        core::ptr::write_volatile(sqe_ptr.add(7), (id_phys >> 32) as u32);
        // CDW10: CNS=1 Identify Controller
        core::ptr::write_volatile(sqe_ptr.add(10), 1u32);
    }

    serial_println!(
        "[sexdrive.nvme.admin.identify.v2.submit] cid={} opc={:#x} cns={} prp1={:#x} sq_tail={}",
        cid as u64,
        0x06u64,
        1u64,
        id_phys,
        sq_tail as u64
    );

    compiler_fence(Ordering::SeqCst);
    sq_tail = (sq_tail + 1) % q_entries;
    unsafe {
        core::ptr::write_volatile((map_va + NVME_SQ0TDBL) as *mut u32, sq_tail);
    }
    serial_println!(
        "[sexdrive.nvme.admin.identify.v2.submit] sq0tdbl={:#x} new_tail={} dstrd={}",
        NVME_SQ0TDBL,
        sq_tail as u64,
        cap_dstrd as u64
    );

    let mut got_cqe = false;
    let mut cqe_dw0 = 0u32;
    let mut cqe_dw1 = 0u32;
    let mut cqe_dw2 = 0u32;
    let mut cqe_dw3 = 0u32;
    let mut cqe_phase = 0u32;
    let mut cqe_cid = 0u32;
    let mut cqe_polls = 0u32;
    let mut acq_changed = false;
    let mut first_change_poll = u32::MAX;
    let mut acq0_dw0_change = acq0_dw0_before;
    let mut acq0_dw1_change = acq0_dw1_before;
    let mut acq0_dw2_change = acq0_dw2_before;
    let mut acq0_dw3_change = acq0_dw3_before;
    let mut saw_cid_ignore_phase = false;
    let mut ig_dw0 = 0u32;
    let mut ig_dw1 = 0u32;
    let mut ig_dw2 = 0u32;
    let mut ig_dw3 = 0u32;
    let mut ig_phase = 0u32;

    while cqe_polls < 1_000_000 {
        let acq0_dw0 = unsafe { core::ptr::read_volatile(acq0_ptr.add(0)) };
        let acq0_dw1 = unsafe { core::ptr::read_volatile(acq0_ptr.add(1)) };
        let acq0_dw2 = unsafe { core::ptr::read_volatile(acq0_ptr.add(2)) };
        let acq0_dw3 = unsafe { core::ptr::read_volatile(acq0_ptr.add(3)) };
        if !acq_changed
            && (acq0_dw0 != acq0_dw0_before
                || acq0_dw1 != acq0_dw1_before
                || acq0_dw2 != acq0_dw2_before
                || acq0_dw3 != acq0_dw3_before)
        {
            acq_changed = true;
            first_change_poll = cqe_polls;
            acq0_dw0_change = acq0_dw0;
            acq0_dw1_change = acq0_dw1;
            acq0_dw2_change = acq0_dw2;
            acq0_dw3_change = acq0_dw3;
            serial_println!(
                "[sexdrive.nvme.admin.identify.v2.cqe.raw] stage=change poll={} dw0={:#x} dw1={:#x} dw2={:#x} dw3={:#x} phase={}",
                cqe_polls as u64,
                acq0_dw0 as u64,
                acq0_dw1 as u64,
                acq0_dw2 as u64,
                acq0_dw3 as u64,
                (acq0_dw3 & 0x1) as u64
            );
        }

        let cqe_ptr = (acq_owned_va as *const u8).wrapping_add((cq_head as usize) * 16) as *const u32;
        let dw0 = unsafe { core::ptr::read_volatile(cqe_ptr.add(0)) };
        let dw1 = unsafe { core::ptr::read_volatile(cqe_ptr.add(1)) };
        let dw2 = unsafe { core::ptr::read_volatile(cqe_ptr.add(2)) };
        let dw3 = unsafe { core::ptr::read_volatile(cqe_ptr.add(3)) };
        // Corrected decode for this observed QEMU path:
        // DW3[15:0]  = CID
        // DW3[16]    = phase
        // DW3[31:17] = status field
        let cmd_id = dw3 & 0xFFFF;
        let phase = (dw3 >> 16) & 0x1;
        if cmd_id == cid as u32 {
            saw_cid_ignore_phase = true;
            ig_dw0 = dw0;
            ig_dw1 = dw1;
            ig_dw2 = dw2;
            ig_dw3 = dw3;
            ig_phase = phase;
        }
        if phase == cq_phase && cmd_id == cid as u32 {
            got_cqe = true;
            cqe_dw0 = dw0;
            cqe_dw1 = dw1;
            cqe_dw2 = dw2;
            cqe_dw3 = dw3;
            cqe_phase = phase;
            cqe_cid = cmd_id;
            serial_println!(
                "[sexdrive.nvme.admin.identify.v2.cqe.decode] match=phase polls={} head={} phase={} cid={} dw0={:#x} dw1={:#x} dw2={:#x} dw3={:#x}",
                cqe_polls as u64,
                cq_head as u64,
                phase as u64,
                cmd_id as u64,
                dw0 as u64,
                dw1 as u64,
                dw2 as u64,
                dw3 as u64
            );
            break;
        }
        cqe_polls += 1;
    }

    if !got_cqe {
        if saw_cid_ignore_phase {
            serial_println!(
                "[sexdrive.nvme.admin.identify.v2.cqe.raw] stage=ignore_phase cid={} phase={} dw0={:#x} dw1={:#x} dw2={:#x} dw3={:#x}",
                cid as u64,
                ig_phase as u64,
                ig_dw0 as u64,
                ig_dw1 as u64,
                ig_dw2 as u64,
                ig_dw3 as u64
            );
        }
        serial_println!(
            "[sexdrive.nvme.admin.identify.v2.err] reason=cqe_timeout cid={} head={} phase={} polls={} acq_changed={} first_change_poll={} acq0_dw0_before={:#x} acq0_dw1_before={:#x} acq0_dw2_before={:#x} acq0_dw3_before={:#x} acq0_dw0_now={:#x} acq0_dw1_now={:#x} acq0_dw2_now={:#x} acq0_dw3_now={:#x}",
            cid as u64,
            cq_head as u64,
            cq_phase as u64,
            cqe_polls as u64,
            if acq_changed { 1u64 } else { 0u64 },
            if first_change_poll == u32::MAX { u64::MAX } else { first_change_poll as u64 },
            acq0_dw0_before as u64,
            acq0_dw1_before as u64,
            acq0_dw2_before as u64,
            acq0_dw3_before as u64,
            acq0_dw0_change as u64,
            acq0_dw1_change as u64,
            acq0_dw2_change as u64,
            acq0_dw3_change as u64
        );
        return;
    }

    serial_println!(
        "[sexdrive.nvme.admin.identify.v2.cqe.raw] stage=phase_match cid={} phase={} dw0={:#x} dw1={:#x} dw2={:#x} dw3={:#x}",
        cqe_cid as u64,
        cqe_phase as u64,
        cqe_dw0 as u64,
        cqe_dw1 as u64,
        cqe_dw2 as u64,
        cqe_dw3 as u64
    );

    let sf = (cqe_dw3 >> 17) & 0x7FFF;
    let sc = sf & 0xFF;
    let sct = (sf >> 8) & 0x7;
    let sqhd = (cqe_dw2 >> 16) & 0xFFFF;

    cq_head = (cq_head + 1) % q_entries;
    if cq_head == 0 {
        cq_phase ^= 1;
    }
    unsafe {
        core::ptr::write_volatile((map_va + NVME_CQ0HDBL) as *mut u32, cq_head);
    }

    if sc != 0 || sct != 0 {
        serial_println!(
            "[sexdrive.nvme.admin.identify.v2.err] reason=cqe_status_fail cid={} sc={} sct={} sqhd={} head={} dw3={:#x}",
            cid as u64,
            sc as u64,
            sct as u64,
            sqhd as u64,
            cq_head as u64,
            cqe_dw3 as u64
        );
        return;
    }

    let sn0 = unsafe { core::ptr::read_volatile((id_va + 4) as *const u64) };
    let mn0 = unsafe { core::ptr::read_volatile((id_va + 24) as *const u64) };
    let nn = unsafe { core::ptr::read_volatile((id_va + 516) as *const u32) };
    serial_println!(
        "[sexdrive.nvme.admin.identify.v2.ok] cid={} sc={} sct={} sqhd={} cqh={} cqp={} sn0={:#x} mn0={:#x} nn={}",
        cid as u64,
        sc as u64,
        sct as u64,
        sqhd as u64,
        cq_head as u64,
        cq_phase as u64,
        sn0,
        mn0,
        nn as u64
    );

    // IO Queue creation proof (no IO read/write in this mission).
    let io_cq_phys = sys_alloc_phys(PAGE_SIZE);
    let io_sq_phys = sys_alloc_phys(PAGE_SIZE);
    if io_cq_phys == 0 || io_cq_phys == u64::MAX || io_sq_phys == 0 || io_sq_phys == u64::MAX {
        serial_println!(
            "[sexdrive.nvme.ioq.err] reason=alloc_failed io_cq_phys={:#x} io_sq_phys={:#x}",
            io_cq_phys,
            io_sq_phys
        );
        return;
    }
    if (io_cq_phys % PAGE_SIZE) != 0 || (io_sq_phys % PAGE_SIZE) != 0 {
        serial_println!(
            "[sexdrive.nvme.ioq.err] reason=alloc_unaligned io_cq_phys={:#x} io_sq_phys={:#x}",
            io_cq_phys,
            io_sq_phys
        );
        return;
    }

    let io_cq_va = sys_map_phys(io_cq_phys, PAGE_SIZE);
    let io_sq_va = sys_map_phys(io_sq_phys, PAGE_SIZE);
    if io_cq_va == 0 || io_cq_va == u64::MAX || io_sq_va == 0 || io_sq_va == u64::MAX {
        serial_println!(
            "[sexdrive.nvme.ioq.err] reason=map_failed io_cq_va={:#x} io_sq_va={:#x}",
            io_cq_va,
            io_sq_va
        );
        return;
    }

    unsafe {
        for i in 0..(PAGE_SIZE / 8) as usize {
            core::ptr::write_volatile((io_cq_va as *mut u64).add(i), 0);
            core::ptr::write_volatile((io_sq_va as *mut u64).add(i), 0);
        }
    }
    serial_println!(
        "[sexdrive.nvme.ioq.alloc.ok] io_cq_phys={:#x} io_cq_va={:#x} io_sq_phys={:#x} io_sq_va={:#x}",
        io_cq_phys,
        io_cq_va,
        io_sq_phys,
        io_sq_va
    );

    // Reuse admin queue state after Identify completion.
    let mut admin_sq_tail = sq_tail;
    let mut admin_cq_head = cq_head;
    let mut admin_cq_phase = cq_phase;

    // Create IO Completion Queue (opcode 0x05), QID=1, QSIZE=15, PC=1, IEN=0, IV=0.
    let create_cq_cid: u16 = 0x0043;
    let cq_sqe_ptr = (asq_owned_va as *mut u8).wrapping_add((admin_sq_tail as usize) * 64) as *mut u32;
    unsafe {
        for i in 0..16usize {
            core::ptr::write_volatile(cq_sqe_ptr.add(i), 0);
        }
        core::ptr::write_volatile(cq_sqe_ptr.add(0), 0x05u32 | ((create_cq_cid as u32) << 16));
        core::ptr::write_volatile(cq_sqe_ptr.add(6), (io_cq_phys & 0xFFFF_FFFF) as u32);
        core::ptr::write_volatile(cq_sqe_ptr.add(7), (io_cq_phys >> 32) as u32);
        core::ptr::write_volatile(cq_sqe_ptr.add(10), (1u32) | (15u32 << 16));
        core::ptr::write_volatile(cq_sqe_ptr.add(11), 1u32); // PC=1, IEN=0, IV=0
    }
    serial_println!(
        "[sexdrive.nvme.ioq.create_cq.submit] cid={} opc={:#x} qid={} qsize={} prp1={:#x} cdw11={:#x}",
        create_cq_cid as u64,
        0x05u64,
        1u64,
        15u64,
        io_cq_phys,
        1u64
    );

    compiler_fence(Ordering::SeqCst);
    admin_sq_tail = (admin_sq_tail + 1) % q_entries;
    unsafe { core::ptr::write_volatile((map_va + NVME_SQ0TDBL) as *mut u32, admin_sq_tail); }

    let mut cq_done = false;
    let mut cq_cqe_dw2 = 0u32;
    let mut cq_cqe_dw3 = 0u32;
    for _ in 0..1_000_000u32 {
        let cqe_ptr = (acq_owned_va as *const u8).wrapping_add((admin_cq_head as usize) * 16) as *const u32;
        let dw2 = unsafe { core::ptr::read_volatile(cqe_ptr.add(2)) };
        let dw3 = unsafe { core::ptr::read_volatile(cqe_ptr.add(3)) };
        let cid_now = dw3 & 0xFFFF;
        let phase_now = (dw3 >> 16) & 0x1;
        if cid_now == create_cq_cid as u32 && phase_now == admin_cq_phase {
            cq_done = true;
            cq_cqe_dw2 = dw2;
            cq_cqe_dw3 = dw3;
            serial_println!(
                "[sexdrive.nvme.ioq.create_cq.cqe] cid={} dw2={:#x} dw3={:#x} phase={}",
                cid_now as u64,
                dw2 as u64,
                dw3 as u64,
                phase_now as u64
            );
            break;
        }
    }
    if !cq_done {
        serial_println!(
            "[sexdrive.nvme.ioq.err] reason=create_cq_timeout cid={}",
            create_cq_cid as u64
        );
        return;
    }

    let cq_sf = (cq_cqe_dw3 >> 17) & 0x7FFF;
    let cq_sc = cq_sf & 0xFF;
    let cq_sct = (cq_sf >> 8) & 0x7;
    admin_cq_head = (admin_cq_head + 1) % q_entries;
    if admin_cq_head == 0 { admin_cq_phase ^= 1; }
    unsafe { core::ptr::write_volatile((map_va + NVME_CQ0HDBL) as *mut u32, admin_cq_head); }
    if cq_sc != 0 || cq_sct != 0 {
        serial_println!(
            "[sexdrive.nvme.ioq.err] reason=create_cq_status_fail cid={} sc={} sct={} dw3={:#x}",
            create_cq_cid as u64,
            cq_sc as u64,
            cq_sct as u64,
            cq_cqe_dw3 as u64
        );
        return;
    }
    serial_println!(
        "[sexdrive.nvme.ioq.create_cq.ok] cid={} sc={} sct={} cqh={} cqp={}",
        create_cq_cid as u64,
        cq_sc as u64,
        cq_sct as u64,
        admin_cq_head as u64,
        admin_cq_phase as u64
    );

    // Create IO Submission Queue (opcode 0x01), QID=1, QSIZE=15, CQID=1, PC=1, QPRIO=0.
    let create_sq_cid: u16 = 0x0044;
    let sq_sqe_ptr = (asq_owned_va as *mut u8).wrapping_add((admin_sq_tail as usize) * 64) as *mut u32;
    unsafe {
        for i in 0..16usize {
            core::ptr::write_volatile(sq_sqe_ptr.add(i), 0);
        }
        core::ptr::write_volatile(sq_sqe_ptr.add(0), 0x01u32 | ((create_sq_cid as u32) << 16));
        core::ptr::write_volatile(sq_sqe_ptr.add(6), (io_sq_phys & 0xFFFF_FFFF) as u32);
        core::ptr::write_volatile(sq_sqe_ptr.add(7), (io_sq_phys >> 32) as u32);
        core::ptr::write_volatile(sq_sqe_ptr.add(10), (1u32) | (15u32 << 16));
        core::ptr::write_volatile(sq_sqe_ptr.add(11), (1u32 << 16) | 1u32); // CQID=1, PC=1, QPRIO=0
    }
    serial_println!(
        "[sexdrive.nvme.ioq.create_sq.submit] cid={} opc={:#x} qid={} qsize={} cqid={} prp1={:#x}",
        create_sq_cid as u64,
        0x01u64,
        1u64,
        15u64,
        1u64,
        io_sq_phys
    );

    compiler_fence(Ordering::SeqCst);
    admin_sq_tail = (admin_sq_tail + 1) % q_entries;
    unsafe { core::ptr::write_volatile((map_va + NVME_SQ0TDBL) as *mut u32, admin_sq_tail); }

    let mut sq_done = false;
    let mut sq_cqe_dw3 = 0u32;
    for _ in 0..1_000_000u32 {
        let cqe_ptr = (acq_owned_va as *const u8).wrapping_add((admin_cq_head as usize) * 16) as *const u32;
        let dw3 = unsafe { core::ptr::read_volatile(cqe_ptr.add(3)) };
        let cid_now = dw3 & 0xFFFF;
        let phase_now = (dw3 >> 16) & 0x1;
        if cid_now == create_sq_cid as u32 && phase_now == admin_cq_phase {
            sq_done = true;
            sq_cqe_dw3 = dw3;
            serial_println!(
                "[sexdrive.nvme.ioq.create_sq.cqe] cid={} dw3={:#x} phase={}",
                cid_now as u64,
                dw3 as u64,
                phase_now as u64
            );
            break;
        }
    }
    if !sq_done {
        serial_println!(
            "[sexdrive.nvme.ioq.err] reason=create_sq_timeout cid={}",
            create_sq_cid as u64
        );
        return;
    }

    let sq_sf = (sq_cqe_dw3 >> 17) & 0x7FFF;
    let sq_sc = sq_sf & 0xFF;
    let sq_sct = (sq_sf >> 8) & 0x7;
    admin_cq_head = (admin_cq_head + 1) % q_entries;
    if admin_cq_head == 0 { admin_cq_phase ^= 1; }
    unsafe { core::ptr::write_volatile((map_va + NVME_CQ0HDBL) as *mut u32, admin_cq_head); }
    if sq_sc != 0 || sq_sct != 0 {
        serial_println!(
            "[sexdrive.nvme.ioq.err] reason=create_sq_status_fail cid={} sc={} sct={} dw3={:#x}",
            create_sq_cid as u64,
            sq_sc as u64,
            sq_sct as u64,
            sq_cqe_dw3 as u64
        );
        return;
    }
    serial_println!(
        "[sexdrive.nvme.ioq.create_sq.ok] cid={} sc={} sct={} cqh={} cqp={}",
        create_sq_cid as u64,
        sq_sc as u64,
        sq_sct as u64,
        admin_cq_head as u64,
        admin_cq_phase as u64
    );

    let sq1tdbl = 0x1000u64 + ((2u64 * 1u64) << (2 + cap_dstrd));
    let cq1hdbl = 0x1000u64 + ((2u64 * 1u64 + 1u64) << (2 + cap_dstrd));
    serial_println!(
        "[sexdrive.nvme.ioq.ready] qid=1 depth=16 sq_tail={} cq_head={} cq_phase={} sq1tdbl={:#x} cq1hdbl={:#x}",
        0u64,
        0u64,
        1u64,
        sq1tdbl,
        cq1hdbl
    );
    unsafe {
        NVME_IO_STATE.ready = true;
        NVME_IO_STATE.map_va = map_va;
        NVME_IO_STATE.io_sq_va = io_sq_va;
        NVME_IO_STATE.io_cq_va = io_cq_va;
        NVME_IO_STATE.sq1tdbl = sq1tdbl;
        NVME_IO_STATE.cq1hdbl = cq1hdbl;
        NVME_IO_STATE.sq_tail = 0;
        NVME_IO_STATE.cq_head = 0;
        NVME_IO_STATE.cq_phase = 1;
    }

    // One real IO READ proof (no BLOCK API wiring in this mission).
    let io_q_entries: u32 = 16;
    let mut io_sq_tail: u32 = 0;
    let mut io_cq_head: u32 = 0;
    let mut io_cq_phase: u32 = 1;

    let data_phys = sys_alloc_phys(PAGE_SIZE);
    if data_phys == 0 || data_phys == u64::MAX || (data_phys % PAGE_SIZE) != 0 {
        serial_println!(
            "[sexdrive.nvme.io.read.err] reason=data_alloc_invalid phys={:#x}",
            data_phys
        );
        return;
    }
    let data_va = sys_map_phys(data_phys, PAGE_SIZE);
    if data_va == 0 || data_va == u64::MAX {
        serial_println!(
            "[sexdrive.nvme.io.read.err] reason=data_map_failed va={:#x}",
            data_va
        );
        return;
    }
    unsafe {
        for i in 0..(PAGE_SIZE / 8) as usize {
            core::ptr::write_volatile((data_va as *mut u64).add(i), 0);
        }
    }
    serial_println!(
        "[sexdrive.nvme.io.read.begin] data_phys={:#x} data_va={:#x} qid={} depth={} sq_tail={} cq_head={} cq_phase={}",
        data_phys,
        data_va,
        1u64,
        io_q_entries as u64,
        io_sq_tail as u64,
        io_cq_head as u64,
        io_cq_phase as u64
    );

    // NVMe READ opcode=0x02, NSID=1, SLBA=0, NLB=0 (one logical block, zero-based count).
    let read_cid: u16 = 0x0045;
    let io_sqe_ptr = (io_sq_va as *mut u8).wrapping_add((io_sq_tail as usize) * 64) as *mut u32;
    unsafe {
        for i in 0..16usize {
            core::ptr::write_volatile(io_sqe_ptr.add(i), 0);
        }
        core::ptr::write_volatile(io_sqe_ptr.add(0), 0x02u32 | ((read_cid as u32) << 16)); // OPC + CID
        core::ptr::write_volatile(io_sqe_ptr.add(1), 1u32); // NSID=1
        core::ptr::write_volatile(io_sqe_ptr.add(6), (data_phys & 0xFFFF_FFFF) as u32); // PRP1 low
        core::ptr::write_volatile(io_sqe_ptr.add(7), (data_phys >> 32) as u32); // PRP1 high
        core::ptr::write_volatile(io_sqe_ptr.add(10), 0u32); // SLBA low
        core::ptr::write_volatile(io_sqe_ptr.add(11), 0u32); // SLBA high
        core::ptr::write_volatile(io_sqe_ptr.add(12), 0u32); // NLB=0 => one LBA
    }
    serial_println!(
        "[sexdrive.nvme.io.read.submit] cid={} opc={:#x} nsid={} slba={} nlb={} prp1={:#x}",
        read_cid as u64,
        0x02u64,
        1u64,
        0u64,
        0u64,
        data_phys
    );

    compiler_fence(Ordering::SeqCst);
    io_sq_tail = (io_sq_tail + 1) % io_q_entries;
    unsafe {
        core::ptr::write_volatile((map_va + sq1tdbl) as *mut u32, io_sq_tail);
    }
    serial_println!(
        "[sexdrive.nvme.io.read.doorbell] sq1tdbl={:#x} new_tail={} dstrd={}",
        sq1tdbl,
        io_sq_tail as u64,
        cap_dstrd as u64
    );

    let mut read_done = false;
    let mut rd_dw0 = 0u32;
    let mut rd_dw1 = 0u32;
    let mut rd_dw2 = 0u32;
    let mut rd_dw3 = 0u32;
    for _ in 0..1_000_000u32 {
        let cqe_ptr = (io_cq_va as *const u8).wrapping_add((io_cq_head as usize) * 16) as *const u32;
        let dw0 = unsafe { core::ptr::read_volatile(cqe_ptr.add(0)) };
        let dw1 = unsafe { core::ptr::read_volatile(cqe_ptr.add(1)) };
        let dw2 = unsafe { core::ptr::read_volatile(cqe_ptr.add(2)) };
        let dw3 = unsafe { core::ptr::read_volatile(cqe_ptr.add(3)) };
        let cid_now = dw3 & 0xFFFF;
        let phase_now = (dw3 >> 16) & 0x1;
        if cid_now == read_cid as u32 && phase_now == io_cq_phase {
            read_done = true;
            rd_dw0 = dw0;
            rd_dw1 = dw1;
            rd_dw2 = dw2;
            rd_dw3 = dw3;
            serial_println!(
                "[sexdrive.nvme.io.read.cqe] cid={} phase={} dw0={:#x} dw1={:#x} dw2={:#x} dw3={:#x}",
                cid_now as u64,
                phase_now as u64,
                dw0 as u64,
                dw1 as u64,
                dw2 as u64,
                dw3 as u64
            );
            break;
        }
    }
    if !read_done {
        serial_println!(
            "[sexdrive.nvme.io.read.err] reason=cqe_timeout cid={} head={} phase={}",
            read_cid as u64,
            io_cq_head as u64,
            io_cq_phase as u64
        );
        return;
    }

    let rd_sf = (rd_dw3 >> 17) & 0x7FFF;
    let rd_sc = rd_sf & 0xFF;
    let rd_sct = (rd_sf >> 8) & 0x7;
    let rd_sqhd = rd_dw2 & 0xFFFF;
    let rd_sqid = (rd_dw2 >> 16) & 0xFFFF;

    io_cq_head = (io_cq_head + 1) % io_q_entries;
    if io_cq_head == 0 { io_cq_phase ^= 1; }
    unsafe {
        core::ptr::write_volatile((map_va + cq1hdbl) as *mut u32, io_cq_head);
    }

    if rd_sc != 0 || rd_sct != 0 {
        serial_println!(
            "[sexdrive.nvme.io.read.err] reason=status_fail cid={} sc={} sct={} sqhd={} sqid={} dw3={:#x}",
            read_cid as u64,
            rd_sc as u64,
            rd_sct as u64,
            rd_sqhd as u64,
            rd_sqid as u64,
            rd_dw3 as u64
        );
        return;
    }

    let d0 = unsafe { core::ptr::read_volatile(data_va as *const u64) };
    let d1 = unsafe { core::ptr::read_volatile((data_va + 8) as *const u64) };
    let d2 = unsafe { core::ptr::read_volatile((data_va + 16) as *const u64) };
    let d3 = unsafe { core::ptr::read_volatile((data_va + 24) as *const u64) };
    serial_println!(
        "[sexdrive.nvme.io.read.ok] cid={} sc={} sct={} sqhd={} sqid={} cqh={} cqp={} d0={:#x} d1={:#x} d2={:#x} d3={:#x}",
        read_cid as u64,
        rd_sc as u64,
        rd_sct as u64,
        rd_sqhd as u64,
        rd_sqid as u64,
        io_cq_head as u64,
        io_cq_phase as u64,
        d0,
        d1,
        d2,
        d3
    );
    unsafe {
        NVME_IO_STATE.sq_tail = io_sq_tail;
        NVME_IO_STATE.cq_head = io_cq_head;
        NVME_IO_STATE.cq_phase = io_cq_phase;
    }
}

fn xhci_probe_mmio() {
    let map_va: u64;
    unsafe {
        // syscall 43 = MAP_PCI_BAR(cap_slot, bar_index, map_size)
        core::arch::asm!(
            "syscall",
            in("rax") 43u64,
            in("rdi") SLOT_USB_HOST,
            in("rsi") 0u64,      // BAR0
            in("rdx") 0x1000u64, // first page only
            lateout("rax") map_va,
            out("rcx") _,
            out("r11") _,
        );
    }

    if map_va == u64::MAX || map_va == 0 {
        serial_println!("[sexdrive] XHCI probe: no BAR lease/mapping available");
        return;
    }

    let regs = map_va as *const u32;
    let cap0 = unsafe { core::ptr::read_volatile(regs.add(0)) };
    let caplength = (cap0 & 0xFF) as u8;
    let hciversion = ((cap0 >> 16) & 0xFFFF) as u16;
    let hcsp1 = unsafe { core::ptr::read_volatile(regs.add(1)) }; // 0x04
    let hcc1 = unsafe { core::ptr::read_volatile(regs.add(4)) };  // 0x10

    serial_println!(
        "[sexdrive] XHCI MMIO probe ok va={:#x} caplength={:#x} hciversion={:#x} hcsp1={:#x} hcc1={:#x}",
        map_va, caplength, hciversion, hcsp1, hcc1
    );
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    nvme_probe_bar();
    xhci_probe_mmio();

    // Wait for display/shell to be ready
    for _ in 0..10_000_000 {
        core::hint::spin_loop();
    }

    // Allocate shared buffer (1024x768x4 = 3MB)
    let fb_size = 1024 * 768 * 4;
    let shared_addr: u64;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 40u64,
            in("rdi") fb_size as u64,
            in("rsi") 1u64, // Consumer: SexDisplay (Domain 1)
            lateout("rax") shared_addr,
        );
    }

    if shared_addr == 0 {
        loop { core::hint::spin_loop(); }
    }

    // Zero-copy handoff: Notify AUTHORITY (Silk-Shell) about shared buffer
    unsafe {
        pdx_call(SLOT_SHELL, OP_SHELL_BIND_BUFFER, shared_addr, 0, 0);
    }

    let mut frame: u32 = 0;
    loop {
        // [sexdrive.block.typed.recv] [sexdrive.block.typed.reply]
        // Typed block command dispatch — decodes SLOT_BLOCK messages.
        // Honest status: ERR_NO_DEVICE for all commands (no real NVMe/AHCI).
        // No fake read success. Returns ERR_BAD_CMD for unknown commands.
        if let Some(msg) = pdx_try_listen_raw(0) {
            let cmd = msg.type_id;
            let offset = msg.arg0;
            let size = msg.arg1;
            let _buf_cap = msg.arg2;

            serial_println!(
                "[sexdrive.block.typed.recv] cmd={} offset={:#x} size={} buf_cap={:#x} caller={}",
                cmd, offset, size, _buf_cap, msg.caller_pd
            );

            // [sexblock.abi.request.decode] — dispatch on typed command
            let reply_val: u64 = match cmd {
                BLOCK_READ => {
                    serial_println!(
                        "[sexdrive.block.read.api.recv] offset={:#x} size={} buf_cap={:#x}",
                        offset, size, _buf_cap
                    );
                    serial_println!(
                        "[sexdrive.block.read.handoff.begin] offset={:#x} size={} buf_cap={:#x}",
                        offset, size, _buf_cap
                    );
                    if size == 0 || size > BLOCK_MAX_XFER {
                        serial_println!(
                            "[sexdrive.block.read.api.err] reason=bad_len size={} max={}",
                            size, BLOCK_MAX_XFER
                        );
                        serial_println!(
                            "[sexdrive.block.read.handoff.err] reason=bad_len size={}",
                            size
                        );
                        BLOCK_ERR_BAD_LEN
                    } else if (offset % BLOCK_SECTOR_SIZE) != 0 {
                        serial_println!(
                            "[sexdrive.block.read.api.err] reason=unaligned_offset offset={:#x} sector={}",
                            offset, BLOCK_SECTOR_SIZE
                        );
                        serial_println!(
                            "[sexdrive.block.read.handoff.err] reason=unaligned_offset offset={:#x}",
                            offset
                        );
                        BLOCK_ERR_BAD_LEN
                    } else if _buf_cap == SLOT_BUF_LEND {
                        if size != 512 {
                            serial_println!(
                                "[sexdrive.block.read.handoff.err] reason=bad_len_phase_b size={} expect=512",
                                size
                            );
                            BLOCK_ERR_BAD_LEN
                        } else {
                            let fill_va = sys_map_mem_lend(SLOT_BUF_LEND);
                            if fill_va == 0 || fill_va == u64::MAX {
                                serial_println!(
                                    "[sexdrive.block.read.handoff.err] reason=map_failed va={:#x}",
                                    fill_va
                                );
                                BLOCK_ERR_NO_DEVICE
                            } else {
                                serial_println!("[sexdrive.bufcap.map.ok] fill_va={:#x}", fill_va);
                                nvme_read_into_mapped_va(offset, size, fill_va)
                            }
                        }
                    } else {
                        serial_println!(
                            "[sexdrive.block.read.api.validate.ok] offset={:#x} size={} lba_bytes={}",
                            offset, size, NVME_LBA_BYTES
                        );
                        if _buf_cap == 0 {
                            serial_println!(
                                "[sexdrive.block.read.handoff.err] reason=buffer_cap_not_real buf_cap={:#x}",
                                _buf_cap
                            );
                        } else {
                            serial_println!(
                                "[sexdrive.block.read.handoff.err] reason=buffer_cap_unverifiable buf_cap={:#x}",
                                _buf_cap
                            );
                        }
                        // NOTE: current typed ABI path reads into a sexdrive-owned bounce buffer;
                        // caller buffer handoff via buf_cap is not wired in this mission.
                        nvme_read_into_bounce(offset, size)
                    }
                }
                BLOCK_WRITE => {
                    serial_println!(
                        "[sexdrive.block.write.api.recv] offset={:#x} size={} buf_cap={:#x}",
                        offset, size, _buf_cap
                    );
                    if write_guard_allows(offset, size, _buf_cap) {
                        let fill_va = sys_map_mem_lend(SLOT_BUF_LEND);
                        if fill_va == 0 || fill_va == u64::MAX {
                            serial_println!(
                                "[sexdrive.nvme.write.err] reason=map_failed va={:#x}",
                                fill_va
                            );
                            BLOCK_ERR_NO_DEVICE
                        } else {
                            serial_println!("[sexdrive.bufcap.map.ok] fill_va={:#x}", fill_va);
                            let proof_offset = WRITE_PROOF_LBA * NVME_LBA_BYTES;
                            if offset == proof_offset {
                                nvme_write_readback_proof(offset, size, fill_va)
                            } else {
                                nvme_write_one_block(offset, size, fill_va)
                            }
                        }
                    } else {
                        serial_println!(
                            "[sexdrive.block.typed] cmd={} ERR_NO_DEVICE honest=write_not_implemented_guard_only",
                            cmd
                        );
                        BLOCK_ERR_NO_DEVICE
                    }
                }
                BLOCK_SYNC => {
                    serial_println!(
                        "[sexdrive.sync.recv] cmd={} honest=flush_not_emulated_by_qemu_nvme",
                        cmd
                    );
                    // [sexdrive.nvme.flush] — NVMe FLUSH opcode 0x00 is wired in
                    // nvme_flush() below, but the current QEMU NVMe emulation does
                    // not post a CQE for FLUSH.  On a real NVMe controller that
                    // supports FLUSH (ONCS bit 4), uncomment the call to nvme_flush()
                    // below to get real durability guarantees.
                    // let flush_status = nvme_flush();
                    // if flush_status == 0 { ... }
                    BLOCK_ERR_NO_DEVICE
                }
                _ => {
                    serial_println!(
                        "[sexdrive.block.typed] cmd={} ERR_BAD_CMD unknown",
                        cmd
                    );
                    BLOCK_ERR_BAD_CMD
                }
            };

            serial_println!(
                "[sexblock.abi.reply.encode] caller={} status={}",
                msg.caller_pd, reply_val
            );
            pdx_reply(msg.caller_pd, reply_val);
            serial_println!(
                "[sexdrive.block.typed.reply] cmd={} caller={} status={}",
                cmd, msg.caller_pd, reply_val
            );
        }

        frame += 1;
        let ptr = shared_addr as *mut u32;
        for y in 0..768 {
            for x in 0..1024 {
                let color = (x as u32 ^ y as u32).wrapping_add(frame);
                unsafe {
                    *ptr.add(y * 1024 + x) = color;
                }
            }
        }

        // Throttle
        for _ in 0..2_000_000 {
            core::hint::spin_loop();
        }
    }
}
