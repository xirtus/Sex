#![no_std]
#![no_main]

use sex_pdx::{serial_println, sys_yield, SLOT_USB_HOST};

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop { sys_yield(); }
}

fn map_xhci_bar0() -> u64 {
    let map_va: u64;
    unsafe {
        // syscall 43 = MAP_PCI_BAR(cap_slot, bar_index, map_size)
        core::arch::asm!(
            "syscall",
            in("rax") 43u64,
            in("rdi") SLOT_USB_HOST,
            in("rsi") 0u64,
            in("rdx") 0x1000u64,
            lateout("rax") map_va,
            out("rcx") _,
            out("r11") _,
        );
    }
    map_va
}

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

#[inline(always)]
fn mmio_read32(base: u64, offset: u64) -> u32 {
    let ptr = (base + offset) as *const u32;
    unsafe { core::ptr::read_volatile(ptr) }
}

#[inline(always)]
fn mmio_write32(base: u64, offset: u64, value: u32) {
    let ptr = (base + offset) as *mut u32;
    unsafe { core::ptr::write_volatile(ptr, value); }
}

#[inline(always)]
fn mmio_write64(base: u64, offset: u64, value: u64) {
    mmio_write32(base, offset, (value & 0xFFFF_FFFF) as u32);
    mmio_write32(base, offset + 4, (value >> 32) as u32);
}

#[inline(always)]
fn trb_write_volatile(base_va: u64, index: u64, d0: u32, d1: u32, d2: u32, d3: u32) {
    let p = (base_va + index * 16) as *mut u32;
    unsafe {
        core::ptr::write_volatile(p.add(0), d0);
        core::ptr::write_volatile(p.add(1), d1);
        core::ptr::write_volatile(p.add(2), d2);
        core::ptr::write_volatile(p.add(3), d3);
    }
}

#[inline(always)]
fn trb_read_dword(base_va: u64, index: u64, word: usize) -> u32 {
    let p = (base_va + index * 16) as *const u32;
    unsafe { core::ptr::read_volatile(p.add(word)) }
}

fn wait_until(base: u64, offset: u64, mask: u32, expect_set: bool, spins: usize) -> bool {
    for _ in 0..spins {
        let v = mmio_read32(base, offset);
        let is_set = (v & mask) != 0;
        if is_set == expect_set {
            return true;
        }
        sys_yield();
    }
    false
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    const PAGE_SIZE: u64 = 4096;
    const TRB_SIZE: u64 = 16;
    const CMD_RING_TRBS: u64 = 64;
    const EVENT_RING_TRBS: u64 = 64;
    const ERST_ENTRIES: u64 = 1;
    const DCBAA_BYTES: u64 = PAGE_SIZE;
    const MAP_BYTES: u64 = 0x1000;

    const XHCI_USBCMD: u64 = 0x00;
    const XHCI_USBSTS: u64 = 0x04;
    const USBCMD_RUN_STOP: u32 = 1 << 0;
    const USBCMD_HCRST: u32 = 1 << 1;
    const USBSTS_HCHALTED: u32 = 1 << 0;
    const USBSTS_CNR: u32 = 1 << 11;
    const POLL_BUDGET: usize = 100_000;
    const XHCI_CRCR: u64 = 0x18;
    const XHCI_DCBAAP: u64 = 0x30;
    const XHCI_CAP_DBOFF: u64 = 0x14;
    const XHCI_CAP_RTSOFF: u64 = 0x18;
    const XHCI_CAP_HCCPARAMS1: u64 = 0x10;
    const XHCI_INTR0_BASE: u64 = 0x20;
    const XHCI_INTR_ERSTSZ: u64 = 0x08;
    const XHCI_INTR_ERSTBA: u64 = 0x10;
    const XHCI_INTR_ERDP: u64 = 0x18;
    const TRB_TYPE_ENABLE_SLOT_CMD: u32 = 9;
    const TRB_TYPE_NOOP_CMD: u32 = 23;
    const TRB_TYPE_CMD_COMPLETION_EVENT: u32 = 33;
    const TRB_CC_SUCCESS: u32 = 1;

    serial_println!("[sexusb.boot]");

    let map_va = map_xhci_bar0();
    if map_va == 0 || map_va == u64::MAX {
        serial_println!("[sexusb.xhci.map.bad]");
        loop { sys_yield(); }
    }
    serial_println!("[sexusb.xhci.map.ok]");

    let regs = map_va as *const u32;
    let cap0 = unsafe { core::ptr::read_volatile(regs) };
    let caplength = (cap0 & 0xFF) as u8;
    let hciversion = ((cap0 >> 16) & 0xFFFF) as u16;
    let hcsp1 = unsafe { core::ptr::read_volatile(regs.add(1)) };
    let hcc1 = unsafe { core::ptr::read_volatile(regs.add(4)) };

    serial_println!("[sexusb.xhci.caplength] {:#x}", caplength);
    serial_println!("[sexusb.xhci.hciversion] {:#x}", hciversion);
    serial_println!("[sexusb.xhci.hcsp1] {:#x}", hcsp1);
    serial_println!("[sexusb.xhci.hcc1] {:#x}", hcc1);
    serial_println!("[sexusb.xhci.probe.ok]");

    let op_base = map_va.wrapping_add(caplength as u64);
    serial_println!("[sexusb.xhci.reset.start]");

    // Stop controller first (clear Run/Stop), then wait for Halted.
    let mut usbcmd = mmio_read32(op_base, XHCI_USBCMD);
    if (usbcmd & USBCMD_RUN_STOP) != 0 {
        usbcmd &= !USBCMD_RUN_STOP;
        mmio_write32(op_base, XHCI_USBCMD, usbcmd);
    }
    if wait_until(op_base, XHCI_USBSTS, USBSTS_HCHALTED, true, POLL_BUDGET) {
        serial_println!("[sexusb.xhci.halted.ok]");
    } else {
        serial_println!("[sexusb.xhci.halted.bad]");
    }

    // Host Controller Reset: set HCRST, wait for hardware to clear.
    usbcmd = mmio_read32(op_base, XHCI_USBCMD) | USBCMD_HCRST;
    mmio_write32(op_base, XHCI_USBCMD, usbcmd);
    let reset_done = wait_until(op_base, XHCI_USBCMD, USBCMD_HCRST, false, POLL_BUDGET);
    let cnr_done = wait_until(op_base, XHCI_USBSTS, USBSTS_CNR, false, POLL_BUDGET);
    if reset_done && cnr_done {
        serial_println!("[sexusb.xhci.reset.ok]");
    } else {
        serial_println!("[sexusb.xhci.reset.bad]");
    }

    // Run proof: set Run/Stop and wait for Halted to clear.
    usbcmd = mmio_read32(op_base, XHCI_USBCMD) | USBCMD_RUN_STOP;
    mmio_write32(op_base, XHCI_USBCMD, usbcmd);
    if wait_until(op_base, XHCI_USBSTS, USBSTS_HCHALTED, false, POLL_BUDGET) {
        serial_println!("[sexusb.xhci.run.ok]");
    } else {
        serial_println!("[sexusb.xhci.run.bad]");
    }

    serial_println!("[sexusb.xhci.ring.alloc.start]");
    let cmd_ring_bytes = CMD_RING_TRBS * TRB_SIZE;
    let event_ring_bytes = EVENT_RING_TRBS * TRB_SIZE;
    let erst_bytes = PAGE_SIZE;

    let cmd_ring_phys = sys_alloc_phys(PAGE_SIZE);
    let event_ring_phys = sys_alloc_phys(PAGE_SIZE);
    let erst_phys = sys_alloc_phys(erst_bytes);
    let dcbaa_phys = sys_alloc_phys(DCBAA_BYTES);

    if cmd_ring_phys == 0 || cmd_ring_phys == u64::MAX
        || event_ring_phys == 0 || event_ring_phys == u64::MAX
        || erst_phys == 0 || erst_phys == u64::MAX
        || dcbaa_phys == 0 || dcbaa_phys == u64::MAX
    {
        serial_println!("[sexusb.xhci.ring.alloc.bad]");
        loop { sys_yield(); }
    }

    let cmd_ring_va = sys_map_phys(cmd_ring_phys, PAGE_SIZE);
    let event_ring_va = sys_map_phys(event_ring_phys, PAGE_SIZE);
    let erst_va = sys_map_phys(erst_phys, erst_bytes);
    let dcbaa_va = sys_map_phys(dcbaa_phys, DCBAA_BYTES);

    if cmd_ring_va == 0 || cmd_ring_va == u64::MAX
        || event_ring_va == 0 || event_ring_va == u64::MAX
        || erst_va == 0 || erst_va == u64::MAX
        || dcbaa_va == 0 || dcbaa_va == u64::MAX
    {
        serial_println!("[sexusb.xhci.ring.alloc.bad]");
        loop { sys_yield(); }
    }

    let aligned_ok = (cmd_ring_phys % PAGE_SIZE == 0)
        && (event_ring_phys % PAGE_SIZE == 0)
        && (erst_phys % PAGE_SIZE == 0)
        && (dcbaa_phys % PAGE_SIZE == 0)
        && (cmd_ring_phys % 64 == 0)
        && (event_ring_phys % 64 == 0)
        && (erst_phys % 64 == 0)
        && (dcbaa_phys % 64 == 0)
        && (cmd_ring_va % PAGE_SIZE == 0)
        && (event_ring_va % PAGE_SIZE == 0)
        && (erst_va % PAGE_SIZE == 0)
        && (dcbaa_va % PAGE_SIZE == 0);
    if !aligned_ok {
        serial_println!("[sexusb.xhci.ring.align.bad]");
        loop { sys_yield(); }
    }

    unsafe {
        core::ptr::write_bytes(cmd_ring_va as *mut u8, 0, PAGE_SIZE as usize);
        core::ptr::write_bytes(event_ring_va as *mut u8, 0, PAGE_SIZE as usize);
        core::ptr::write_bytes(erst_va as *mut u8, 0, erst_bytes as usize);
        core::ptr::write_bytes(dcbaa_va as *mut u8, 0, DCBAA_BYTES as usize);
    }
    serial_println!("[sexusb.xhci.cmd_ring.ok]");
    serial_println!("[sexusb.xhci.event_ring.ok]");
    serial_println!("[sexusb.xhci.dcbaa.ok]");

    // ERST[0] = { ring_segment_base, ring_segment_size, reserved }
    unsafe {
        core::ptr::write_volatile(erst_va as *mut u64, event_ring_phys);
        core::ptr::write_volatile((erst_va + 8) as *mut u32, EVENT_RING_TRBS as u32);
        core::ptr::write_volatile((erst_va + 12) as *mut u32, 0u32);
    }
    let _ = cmd_ring_bytes;
    let _ = event_ring_bytes;
    let _ = ERST_ENTRIES;
    serial_println!("[sexusb.xhci.erst.ok]");

    serial_println!("[sexusb.xhci.ring.ptrs.write.start]");
    let cap_base = map_va;
    let rtsoff_raw = mmio_read32(cap_base, XHCI_CAP_RTSOFF);
    let rtsoff = (rtsoff_raw & !0x1Fu32) as u64;
    let hcc1_local = mmio_read32(cap_base, XHCI_CAP_HCCPARAMS1);
    let _ = hcc1_local;
    let runtime_base = map_va.wrapping_add(rtsoff);
    let intr0_base = runtime_base.wrapping_add(XHCI_INTR0_BASE);

    // Bounds checks against the mapped BAR slice.
    let op_need_end = op_base.wrapping_add(XHCI_DCBAAP + 8);
    let rt_need_end = intr0_base.wrapping_add(XHCI_INTR_ERDP + 8);
    if op_need_end > map_va.wrapping_add(MAP_BYTES) || rt_need_end > map_va.wrapping_add(MAP_BYTES) {
        serial_println!("[sexusb.xhci.ring.ptrs.write.bad]");
        loop { sys_yield(); }
    }

    mmio_write64(op_base, XHCI_DCBAAP, dcbaa_phys);
    serial_println!("[sexusb.xhci.dcbaap.write.ok]");
    mmio_write64(op_base, XHCI_CRCR, cmd_ring_phys | 1u64); // RCS=1
    serial_println!("[sexusb.xhci.crcr.write.ok]");

    mmio_write32(intr0_base, XHCI_INTR_ERSTSZ, ERST_ENTRIES as u32);
    mmio_write64(intr0_base, XHCI_INTR_ERSTBA, erst_phys);
    serial_println!("[sexusb.xhci.erst.write.ok]");
    mmio_write64(intr0_base, XHCI_INTR_ERDP, event_ring_phys | 1u64); // DCS=1
    serial_println!("[sexusb.xhci.erdp.write.ok]");
    serial_println!("[sexusb.xhci.ring.proof.ok]");

    // Command/event ring state machine: explicit tracked indices and cycle bits.
    // No hardcoded TRB indices. cmd_cycle matches CRCR RCS (starts 1).
    // ev_idx tracks next event slot to consume; ev_dcs matches ERDP DCS (starts 1).
    let mut cmd_idx: u64 = 0;
    let mut cmd_cycle: u32 = 1;
    let mut ev_idx: u64 = 0;
    let mut ev_dcs: u64 = 1;

    // ===== NOOP =====
    serial_println!("[sexusb.xhci.cmd.noop.start]");

    // Write NOOP TRB at cmd_idx with cmd_cycle.
    let noop_d3 = (TRB_TYPE_NOOP_CMD << 10) | cmd_cycle;
    trb_write_volatile(cmd_ring_va, cmd_idx, 0, 0, 0, noop_d3);
    // Cycle-stop at cmd_idx+1 with cmd_cycle (same as command). After the controller
    // consumes the command TRB, CRCS toggles to !cmd_cycle, making TRB at cmd_idx+1
    // with cycle=cmd_cycle cause a cycle stop (cycle != CRCS).
    trb_write_volatile(cmd_ring_va, cmd_idx + 1, 0, 0, 0, cmd_cycle);
    serial_println!("[sexusb.xhci.cmd.noop.trb.ok]");

    let dboff_raw = mmio_read32(cap_base, XHCI_CAP_DBOFF);
    let db_base = map_va.wrapping_add((dboff_raw & !0x3u32) as u64);
    if db_base.wrapping_add(4) > map_va.wrapping_add(MAP_BYTES) {
        serial_println!("[sexusb.xhci.ring.ptrs.write.bad]");
        loop { sys_yield(); }
    }
    mmio_write32(db_base, 0, 0u32); // Doorbell 0, target 0 (command ring)
    serial_println!("[sexusb.xhci.cmd.noop.doorbell.ok]");

    // Consume command completion event at ev_idx.
    let mut noop_ok = false;
    for _ in 0..POLL_BUDGET {
        let ev_d3 = trb_read_dword(event_ring_va, ev_idx, 3);
        if (ev_d3 & 1) == (ev_dcs as u32) {
            let ev_type = (ev_d3 >> 10) & 0x3F;
            if ev_type == TRB_TYPE_CMD_COMPLETION_EVENT {
                let ev_d2 = trb_read_dword(event_ring_va, ev_idx, 2);
                let cc = (ev_d2 >> 24) & 0xFF;
                noop_ok = cc == TRB_CC_SUCCESS;
                serial_println!("[sexusb.xhci.cmd.noop.event.seen]");
                // Clear consumed event cycle bit (spec 4.11.4)
                trb_write_volatile(event_ring_va, ev_idx, 0, 0, 0, ev_d3 & !1u32);
                // Advance event dequeue pointer and update ERDP.
                ev_idx += 1;
                if ev_idx >= EVENT_RING_TRBS {
                    ev_idx = 0;
                    ev_dcs ^= 1; // toggle DCS on segment wrap
                }
                let new_erdp = event_ring_phys + ev_idx * 16;
                mmio_write64(intr0_base, XHCI_INTR_ERDP, new_erdp | ev_dcs);
            }
            break;
        }
        sys_yield();
    }

    if noop_ok {
        serial_println!("[sexusb.xhci.cmd.noop.complete.ok]");
    } else {
        serial_println!("[sexusb.xhci.cmd.noop.complete.bad]");
        loop { sys_yield(); }
    }

    // Advance command ring producer state after TRB consumed.
    cmd_idx += 1;
    cmd_cycle ^= 1;

    // ===== Enable Slot =====
    serial_println!("[sexusb.xhci.enable_slot.start]");

    // Write Enable Slot TRB at cmd_idx with cmd_cycle.
    let enable_slot_d3 = (TRB_TYPE_ENABLE_SLOT_CMD << 10) | cmd_cycle;
    trb_write_volatile(cmd_ring_va, cmd_idx, 0, 0, 0, enable_slot_d3);
    // Cycle-stop at cmd_idx+1 with cmd_cycle.
    trb_write_volatile(cmd_ring_va, cmd_idx + 1, 0, 0, 0, cmd_cycle);
    serial_println!("[sexusb.xhci.enable_slot.trb.ok]");

    mmio_write32(db_base, 0, 0u32); // Doorbell 0, target 0 (command ring)
    serial_println!("[sexusb.xhci.enable_slot.doorbell.ok]");

    // Consume command completion event at ev_idx.
    let mut en_ok = false;
    let mut en_slot_id: u32 = 0;
    for _ in 0..POLL_BUDGET {
        let ev_d3 = trb_read_dword(event_ring_va, ev_idx, 3);
        if (ev_d3 & 1) == (ev_dcs as u32) {
            let ev_type = (ev_d3 >> 10) & 0x3F;
            if ev_type == TRB_TYPE_CMD_COMPLETION_EVENT {
                let ev_d2 = trb_read_dword(event_ring_va, ev_idx, 2);
                let cc = (ev_d2 >> 24) & 0xFF;
                en_ok = cc == TRB_CC_SUCCESS;
                en_slot_id = (ev_d3 >> 24) & 0xFF;
                serial_println!("[sexusb.xhci.enable_slot.event.seen]");
                // Clear consumed event cycle bit (spec 4.11.4)
                trb_write_volatile(event_ring_va, ev_idx, 0, 0, 0, ev_d3 & !1u32);
                // Advance event dequeue pointer and update ERDP.
                ev_idx += 1;
                if ev_idx >= EVENT_RING_TRBS {
                    ev_idx = 0;
                    ev_dcs ^= 1; // toggle DCS on segment wrap
                }
                let new_erdp = event_ring_phys + ev_idx * 16;
                mmio_write64(intr0_base, XHCI_INTR_ERDP, new_erdp | ev_dcs);
            }
            break;
        }
        sys_yield();
    }

    if en_ok && en_slot_id != 0 {
        serial_println!("[sexusb.xhci.enable_slot.complete.ok]");
        serial_println!("[sexusb.xhci.enable_slot.slot.ok] {}", en_slot_id);
    } else {
        serial_println!("[sexusb.xhci.enable_slot.complete.bad]");
    }

    // Advance command ring producer state after TRB consumed.
    cmd_idx += 1;
    cmd_cycle ^= 1;

    loop { sys_yield(); }
}
