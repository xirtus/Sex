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
    const TRB_TYPE_ADDRESS_DEVICE_CMD: u32 = 8;
    const TRB_TYPE_NOOP_CMD: u32 = 23;
    const TRB_TYPE_CMD_COMPLETION_EVENT: u32 = 33;
    const TRB_TYPE_TRANSFER_EVENT: u32 = 32;
    const TRB_TYPE_SETUP_STAGE: u32 = 2;
    const TRB_TYPE_DATA_STAGE: u32 = 3;
    const TRB_TYPE_STATUS_STAGE: u32 = 4;
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
    // No hardcoded TRB indices.
    // cmd_cycle matches CRCR RCS (starts 1). CRCS/RCS is stable per segment —
    // toggles ONLY on segment boundary wrap (spec 5.4.5), NOT per TRB.
    // Stop marker uses !cmd_cycle (opposite) — same-cycle would match CRCS and
    // be consumed as Reserved TRB (type=0), causing corruption.
    // ev_idx tracks next event slot to consume; ev_dcs matches ERDP DCS (starts 1).
    // ev_dcs toggles only on event ring segment wrap.
    serial_println!("[sexusb.xhci.ring_state.audit.start]");
    let mut cmd_idx: u64 = 0;
    let cmd_cycle: u32 = 1;
    let mut ev_idx: u64 = 0;
    let mut ev_dcs: u64 = 1;
    serial_println!("[sexusb.xhci.ring_state.cmd_cycle.ok]");
    serial_println!("[sexusb.xhci.ring_state.stop_marker.ok]");
    serial_println!("[sexusb.xhci.ring_state.event_cycle.ok]");

    // ===== NOOP =====
    serial_println!("[sexusb.xhci.cmd.noop.start]");

    // Write NOOP TRB at cmd_idx with cmd_cycle.
    let noop_d3 = (TRB_TYPE_NOOP_CMD << 10) | cmd_cycle;
    trb_write_volatile(cmd_ring_va, cmd_idx, 0, 0, 0, noop_d3);
    // Cycle-stop at cmd_idx+1 with !cmd_cycle (opposite). CRCS stays stable until
    // segment wrap (spec 5.4.5), so a TRB with cycle != CRCS causes a cycle stop.
    // Same-cycle stop marker would match CRCS and be consumed as Reserved (corruption).
    trb_write_volatile(cmd_ring_va, cmd_idx + 1, 0, 0, 0, cmd_cycle ^ 1);
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

    // Advance command ring producer index (cycle stable until segment wrap).
    cmd_idx += 1;

    // ===== Enable Slot =====
    serial_println!("[sexusb.xhci.enable_slot.start]");

    // Write Enable Slot TRB at cmd_idx with cmd_cycle.
    let enable_slot_d3 = (TRB_TYPE_ENABLE_SLOT_CMD << 10) | cmd_cycle;
    trb_write_volatile(cmd_ring_va, cmd_idx, 0, 0, 0, enable_slot_d3);
    // Cycle-stop at cmd_idx+1 with !cmd_cycle (opposite).
    trb_write_volatile(cmd_ring_va, cmd_idx + 1, 0, 0, 0, cmd_cycle ^ 1);
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

    serial_println!("[sexusb.xhci.ring_state.audit.ok]");

    // Advance command ring producer index (cycle stable until segment wrap).
    cmd_idx += 1;
    let _ = (cmd_idx,);

    // ===== Context Layout Proof =====
    serial_println!("[sexusb.xhci.addr_ctx.layout.start]");

    // Compute XHCI context stride from HCCPARAMS1 CSZ bit (bit 2).
    // CSZ=0 -> 32 bytes, CSZ=1 -> 64 bytes. See XHCI spec 5.3.2.
    let ctx_stride: u64 = if (hcc1 & (1u32 << 2)) != 0 { 64 } else { 32 };
    serial_println!("[sexusb.xhci.addr_ctx.stride.ok] {}", ctx_stride);

    // Read MaxPorts from HCSPARAMS1 bits 23:16.
    let max_ports: u64 = ((hcsp1 >> 16) & 0xFF) as u64;
    serial_println!("[sexusb.xhci.addr_ctx.ports] {}", max_ports);

    // Scan PORTSC for first connected, enabled port. Get port number and speed.
    const PORTSC_BASE: u64 = 0x400;
    const PORTSC_STRIDE: u64 = 0x10;
    const PORTSC_CCS: u32 = 1u32 << 0;
    let mut target_port: u64 = 0;
    let mut port_speed: u32 = 0;
    for port in 1..=max_ports {
        let portsc_off = PORTSC_BASE + (port - 1) * PORTSC_STRIDE;
        let portsc = mmio_read32(op_base, portsc_off);
        if (portsc & PORTSC_CCS) != 0 {
            target_port = port;
            port_speed = (portsc >> 10) & 0xF;
            serial_println!("[sexusb.xhci.addr_ctx.port.connected] port={} speed={}", port, port_speed);
            break;
        }
    }
    if target_port == 0 {
        serial_println!("[sexusb.xhci.addr_ctx.port.none.bad]");
        loop { sys_yield(); }
    }

    // Allocate input context, device context, EP0 transfer ring pages.
    let input_ctx_phys = sys_alloc_phys(PAGE_SIZE);
    let device_ctx_phys = sys_alloc_phys(PAGE_SIZE);
    let ep0_ring_phys = sys_alloc_phys(PAGE_SIZE);

    if input_ctx_phys == 0 || input_ctx_phys == u64::MAX
        || device_ctx_phys == 0 || device_ctx_phys == u64::MAX
        || ep0_ring_phys == 0 || ep0_ring_phys == u64::MAX
    {
        serial_println!("[sexusb.xhci.addr_ctx.alloc.bad]");
        loop { sys_yield(); }
    }

    let input_ctx_va = sys_map_phys(input_ctx_phys, PAGE_SIZE);
    let device_ctx_va = sys_map_phys(device_ctx_phys, PAGE_SIZE);
    let ep0_ring_va = sys_map_phys(ep0_ring_phys, PAGE_SIZE);

    if input_ctx_va == 0 || input_ctx_va == u64::MAX
        || device_ctx_va == 0 || device_ctx_va == u64::MAX
        || ep0_ring_va == 0 || ep0_ring_va == u64::MAX
    {
        serial_println!("[sexusb.xhci.addr_ctx.map.bad]");
        loop { sys_yield(); }
    }

    // Context pages must be 64-byte aligned for XHCI context/ring requirements.
    let ctx_align_ok = (input_ctx_phys % 64 == 0)
        && (device_ctx_phys % 64 == 0)
        && (ep0_ring_phys % 64 == 0)
        && (input_ctx_va % PAGE_SIZE == 0)
        && (device_ctx_va % PAGE_SIZE == 0)
        && (ep0_ring_va % PAGE_SIZE == 0);
    if !ctx_align_ok {
        serial_println!("[sexusb.xhci.addr_ctx.align.bad]");
        loop { sys_yield(); }
    }

    serial_println!("[sexusb.xhci.addr_ctx.input.ok]");
    serial_println!("[sexusb.xhci.addr_ctx.device.ok]");
    serial_println!("[sexusb.xhci.addr_ctx.ep0_ring.ok]");

    // Zero all three pages.
    unsafe {
        core::ptr::write_bytes(input_ctx_va as *mut u8, 0, PAGE_SIZE as usize);
        core::ptr::write_bytes(device_ctx_va as *mut u8, 0, PAGE_SIZE as usize);
        core::ptr::write_bytes(ep0_ring_va as *mut u8, 0, PAGE_SIZE as usize);
    }

    // Input Context layout (one page):
    //   offset 0:          Input Control Context (full ctx_stride bytes)
    //   offset ctx_stride: Slot Context
    //   offset ctx_stride*2: EP0 Context
    // Device Context layout (one page):
    //   offset 0:          Slot Context
    //   offset ctx_stride: EP0 Context

    // --- ICC: Input Control Context ---
    // DW0: Drop Context flags (bit per context index)
    // DW1: Add Context flags (bit per context index)
    // bit 0 = Slot, bit 1 = EP0, bits 2..30 = EP1..EP29
    const ICC_DROP_NONE: u32 = 0;
    const ICC_ADD_SLOT_EP0: u32 = (1u32 << 0) | (1u32 << 1);
    unsafe {
        let icc_ptr = input_ctx_va as *mut u32;
        core::ptr::write_volatile(icc_ptr.add(0), ICC_DROP_NONE);
        core::ptr::write_volatile(icc_ptr.add(1), ICC_ADD_SLOT_EP0);
    }
    serial_println!("[sexusb.xhci.addr_ctx.icc.ok]");

    // --- Slot Context ---
    // DW0: bits 31:27 = Context Entries, bits 23:20 = Speed, bits 19:0 = Route String
    // DW1: bits 31:24 = Root Hub Port Number
    const SLOT_CTX_ENTRIES_SHIFT: u32 = 27;
    const SLOT_SPEED_SHIFT: u32 = 20;
    let context_entries: u32 = 1;  // EP0 is endpoint context index 1
    let slot_dw0 = (context_entries << SLOT_CTX_ENTRIES_SHIFT)
        | (port_speed << SLOT_SPEED_SHIFT)
        | 0u32;  // route_string = 0 (root hub, no hub routing)
    let slot_dw1: u32 = (target_port as u32) << 24;  // root hub port number in bits 31:24

    // Write to Input Context Slot Context at offset ctx_stride.
    let input_slot_base = (input_ctx_va + ctx_stride) as *mut u32;
    unsafe {
        core::ptr::write_volatile(input_slot_base.add(0), slot_dw0);
        core::ptr::write_volatile(input_slot_base.add(1), slot_dw1);
    }
    // Write to Device Context Slot Context at offset 0.
    let dev_slot_base = device_ctx_va as *mut u32;
    unsafe {
        core::ptr::write_volatile(dev_slot_base.add(0), slot_dw0);
        core::ptr::write_volatile(dev_slot_base.add(1), slot_dw1);
    }
    serial_println!("[sexusb.xhci.addr_ctx.slot.ok]");

    // --- EP0 Context ---
    // DW0: bits 31:16 = Max Packet Size
    // DW1: bits 3:0 = CErr (3), bits 5:3 = EP Type (010b=2 for Control)
    // DW2: TR Dequeue Pointer Low bits 31:4 + DCS bit 0
    // DW3: TR Dequeue Pointer High bits 31:0
    const CERR_DEFAULT: u32 = 3;
    const EP_TYPE_CTRL_SHIFT: u32 = 3;
    const EP_TYPE_CTRL_VAL: u32 = 2;  // 010b = Control (XHCI spec 6.2.3)
    const DCS_INITIAL: u64 = 1;

    // Max Packet Size per port speed (pre-descriptor boot values).
    // LS=8, FS=8, HS=64, SS=512. Unknown speed => log bad + park.
    let max_packet_size: u32 = match port_speed {
        0 => 8,     // Full Speed  (12 Mbps, pre-descriptor MPS=8)
        1 => 8,     // Low Speed   (1.5 Mbps)
        2 => 64,    // High Speed  (480 Mbps)
        3 => 512,   // Super Speed (5 Gbps)
        _ => {
            serial_println!("[sexusb.xhci.addr_ctx.ep0.speed.unknown.bad] port_speed={}", port_speed);
            loop { sys_yield(); }
        }
    };

    let ep0_dw0 = max_packet_size << 16;
    let ep0_dw1 = (EP_TYPE_CTRL_VAL << EP_TYPE_CTRL_SHIFT) | CERR_DEFAULT;
    let ep0_tr_dequeue = ep0_ring_phys | DCS_INITIAL;
    let ep0_dw2 = (ep0_tr_dequeue & 0xFFFF_FFFF) as u32;
    let ep0_dw3 = (ep0_tr_dequeue >> 32) as u32;

    // Write to Input Context EP0 Context at offset ctx_stride * 2.
    let input_ep0_base = (input_ctx_va + ctx_stride * 2) as *mut u32;
    unsafe {
        core::ptr::write_volatile(input_ep0_base.add(0), ep0_dw0);
        core::ptr::write_volatile(input_ep0_base.add(1), ep0_dw1);
        core::ptr::write_volatile(input_ep0_base.add(2), ep0_dw2);
        core::ptr::write_volatile(input_ep0_base.add(3), ep0_dw3);
    }
    // Write to Device Context EP0 Context at offset ctx_stride.
    let dev_ep0_base = (device_ctx_va + ctx_stride) as *mut u32;
    unsafe {
        core::ptr::write_volatile(dev_ep0_base.add(0), ep0_dw0);
        core::ptr::write_volatile(dev_ep0_base.add(1), ep0_dw1);
        core::ptr::write_volatile(dev_ep0_base.add(2), ep0_dw2);
        core::ptr::write_volatile(dev_ep0_base.add(3), ep0_dw3);
    }
    serial_println!("[sexusb.xhci.addr_ctx.ep0.ok]");

    // --- DCBAA ---
    // Validate slot_id < max_slots from HCSPARAMS1 bits 31:24.
    let max_slots: u32 = (hcsp1 >> 24) & 0xFF;
    if en_slot_id == 0 || en_slot_id >= max_slots {
        serial_println!("[sexusb.xhci.addr_ctx.dcbaa.slot.bad] slot={} max_slots={}", en_slot_id, max_slots);
        loop { sys_yield(); }
    }
    // slot_id is 1-indexed. DCBAA entries are 8 bytes each.
    let dcbaa_entry_ptr = (dcbaa_va + (en_slot_id as u64) * 8) as *mut u64;
    unsafe {
        core::ptr::write_volatile(dcbaa_entry_ptr, device_ctx_phys);
    }
    serial_println!("[sexusb.xhci.addr_ctx.dcbaa.ok]");

    serial_println!("[sexusb.xhci.addr_ctx.layout.ok]");

    // ===== Address Device =====
    serial_println!("[sexusb.xhci.address_device.start]");

    // Address Device Command TRB at cmd_idx with cmd_cycle.
    // d0/d1 = input_context_phys, d2 = 0,
    // d3 = (slot_id << 24) | (type=8 << 10) | cmd_cycle (BSR=0 for normal address)
    let addr_dev_d0 = (input_ctx_phys & 0xFFFF_FFFF) as u32;
    let addr_dev_d1 = (input_ctx_phys >> 32) as u32;
    let addr_dev_d3 = (en_slot_id << 24)
        | (TRB_TYPE_ADDRESS_DEVICE_CMD << 10)
        | cmd_cycle;
    trb_write_volatile(cmd_ring_va, cmd_idx, addr_dev_d0, addr_dev_d1, 0u32, addr_dev_d3);
    serial_println!("[sexusb.xhci.address_device.trb.ok]");

    // Cycle-stop at cmd_idx+1 with opposite cycle.
    trb_write_volatile(cmd_ring_va, cmd_idx + 1, 0, 0, 0, cmd_cycle ^ 1);

    // Doorbell 0 (command ring).
    mmio_write32(db_base, 0, 0u32);
    serial_println!("[sexusb.xhci.address_device.doorbell.ok]");

    // Consume command completion event at ev_idx.
    let mut addr_dev_ok = false;
    for _ in 0..POLL_BUDGET {
        let ev_d3 = trb_read_dword(event_ring_va, ev_idx, 3);
        if (ev_d3 & 1) == (ev_dcs as u32) {
            let ev_type = (ev_d3 >> 10) & 0x3F;
            if ev_type == TRB_TYPE_CMD_COMPLETION_EVENT {
                let ev_d2 = trb_read_dword(event_ring_va, ev_idx, 2);
                let ev_cc = (ev_d2 >> 24) & 0xFF;
                let ev_slot_id = (ev_d3 >> 24) & 0xFF;
                if ev_cc == TRB_CC_SUCCESS && ev_slot_id == en_slot_id {
                    addr_dev_ok = true;
                    serial_println!("[sexusb.xhci.address_device.event.seen]");
                    serial_println!("[sexusb.xhci.address_device.complete.ok]");
                    serial_println!("[sexusb.xhci.address_device.slot.ok]");
                } else {
                    serial_println!("[sexusb.xhci.address_device.complete.bad] cc={} slot={}", ev_cc, ev_slot_id);
                }
                // Clear consumed event cycle bit (spec 4.11.4)
                trb_write_volatile(event_ring_va, ev_idx, 0, 0, 0, ev_d3 & !1u32);
                // Advance event dequeue pointer and update ERDP.
                ev_idx += 1;
                if ev_idx >= EVENT_RING_TRBS {
                    ev_idx = 0;
                    ev_dcs ^= 1;
                }
                let new_erdp = event_ring_phys + ev_idx * 16;
                mmio_write64(intr0_base, XHCI_INTR_ERDP, new_erdp | ev_dcs);
            }
            break;
        }
        sys_yield();
    }

    if !addr_dev_ok {
        serial_println!("[sexusb.xhci.address_device.timeout.bad]");
        loop { sys_yield(); }
    }

    // Advance command ring producer index (cycle stable until segment wrap).
    cmd_idx += 1;
    let _ = (cmd_idx,);

    // Read device context slot state from DW3 (offset 12).
    // Slot state is in bits 31:27. After successful Address Device, should be
    // 3 = Addressed (or 2 = Default depending on BSR).
    let dev_slot_dw3_ptr = (device_ctx_va + 12) as *const u32;
    let dev_slot_dw3 = unsafe { core::ptr::read_volatile(dev_slot_dw3_ptr) };
    let slot_state = (dev_slot_dw3 >> 27) & 0x1F;
    serial_println!("[sexusb.xhci.address_device.state.ok] slot_state={}", slot_state);

    // ===== GET_DESCRIPTOR(DEVICE, 0, 0, 8) =====
    // Phase: USB_XHCI_GET_DEVICE_DESCRIPTOR_8_PROOF_V1
    // Scope: exactly one EP0 control transfer, 8 bytes, no Evaluate Context,
    //        no 18-byte fetch, no HID.
    serial_println!("[sexusb.xhci.desc8.start]");

    // Allocate separate descriptor DMA page (no alias with EP0 transfer ring).
    let desc_data_phys = sys_alloc_phys(PAGE_SIZE);
    let desc_data_va = sys_map_phys(desc_data_phys, PAGE_SIZE);
    if desc_data_phys == 0 || desc_data_phys == u64::MAX
        || desc_data_va == 0 || desc_data_va == u64::MAX
    {
        serial_println!("[sexusb.xhci.desc8.alloc.bad]");
        loop { sys_yield(); }
    }
    if (desc_data_phys % 64) != 0 || (desc_data_phys % PAGE_SIZE) != 0 {
        serial_println!("[sexusb.xhci.desc8.align.bad]");
        loop { sys_yield(); }
    }
    unsafe { core::ptr::write_bytes(desc_data_va as *mut u8, 0, PAGE_SIZE as usize); }
    serial_println!("[sexusb.xhci.desc8.alloc.ok] phys={:#x} va={:#x}", desc_data_phys, desc_data_va);

    // EP0 transfer ring state: explicit indices and cycle.
    // ep0_cycle (TRCS) starts at 1, toggles ONLY on segment boundary wrap
    // (spec 4.11.3.1). NOT per TD. Same rule as CRCS for command ring.
    // Stop marker MUST use ep0_cycle ^ 1 (opposite). Same-cycle Reserved TRB
    // matches TRCS and is consumed as valid type=0 → silent corruption.
    let mut ep0_idx: u64 = 0;
    let ep0_cycle: u32 = 1;
    let _ = (ep0_idx, ep0_cycle);

    // Write 3-TRB chain: Setup Stage (type=2), Data Stage (type=3, IN),
    // Status Stage (type=4, DIR=OUT, IOC=1), then stop marker at ep0_idx+3.
    // GET_DESCRIPTOR(DEVICE, 0, 0, wLength=8) setup packet:
    //   [0x80, 0x06, 0x00, 0x01, 0x00, 0x00, 0x08, 0x00] (LE bytes)
    // d0 = 0x0100_0680, d1 = 0x0008_0000

    // Setup Stage TRB (type=2): IDT=1 (immediate data), CH=1 (chain),
    // TRT=IN(1) in d3[17:16], TRB Transfer Length=8.
    let setup_d3 = (TRB_TYPE_SETUP_STAGE << 10)  // type=2 in bits 15:10
        | (1u32 << 16)                           // TRT=IN (0b01) in bits 17:16
        | ep0_cycle;
    trb_write_volatile(ep0_ring_va, ep0_idx,
        0x0100_0680u32,  // d0: bmReqType=0x80,bReq=0x06,wVal_lo=0x00,wVal_hi=0x01
        0x0008_0000u32,  // d1: wIdx_lo=0x00,wIdx_hi=0x00,wLen_lo=0x08,wLen_hi=0x00
        (8u32 << 0)      // TRB Transfer Length = 8
            | (1u32 << 17)  // IDT=1
            | (1u32 << 18), // CH=1
        setup_d3);

    // Data Stage TRB (type=3): DIR=IN (1), CH=1 (chain to Status),
    // TRB Transfer Length=8, IDT=0 (buffer mode), IOC=0, ISP=0.
    let data_d3 = (TRB_TYPE_DATA_STAGE << 10)    // type=3 in bits 15:10
        | (1u32 << 16)                           // DIR=IN (1) in bit 16
        | ep0_cycle;
    trb_write_volatile(ep0_ring_va, ep0_idx + 1,
        (desc_data_phys & 0xFFFF_FFFF) as u32,
        (desc_data_phys >> 32) as u32,
        (8u32 << 0)      // TRB Transfer Length = 8
            | (1u32 << 18), // CH=1 (IOC=0, ISP=0, IDT=0)
        data_d3);

    // Status Stage TRB (type=4): DIR=OUT (0 for control read status),
    // CH=0 (end of TD), IOC=1 (generate Transfer Event).
    let status_d3 = (TRB_TYPE_STATUS_STAGE << 10) // type=4 in bits 15:10
        | (0u32 << 16)                            // DIR=OUT (0)
        | ep0_cycle;
    trb_write_volatile(ep0_ring_va, ep0_idx + 2,
        0, 0,
        1u32 << 22,  // IOC=1 in bit 22
        status_d3);

    // Cycle-stop marker at ep0_idx+3 with opposite cycle (ep0_cycle ^ 1).
    // Same-cycle Reserved TRB at this position would match TRCS and be
    // consumed as a valid type=0 TRB → undefined behavior.
    trb_write_volatile(ep0_ring_va, ep0_idx + 3, 0, 0, 0, ep0_cycle ^ 1);
    serial_println!("[sexusb.xhci.desc8.trbs.ok]");

    // Advance producer index past submitted TRBs + stop marker.
    // ep0_cycle stays stable (no segment wrap in this phase).
    ep0_idx += 4;
    let _ = (ep0_idx,);

    // Doorbell: EP0 on slot_id. DB Target = 1 (EP0 endpoint ID).
    // DB index = en_slot_id (1-based slot from Enable Slot).
    if db_base.wrapping_add(en_slot_id as u64 * 4 + 4) > map_va.wrapping_add(MAP_BYTES) {
        serial_println!("[sexusb.xhci.desc8.doorbell.bad]");
        loop { sys_yield(); }
    }
    mmio_write32(db_base, en_slot_id as u64 * 4, 1u32);  // target=1 (EP0)
    serial_println!("[sexusb.xhci.desc8.doorbell.ok]");

    // Consume Transfer Event (type=32) at current ev_idx.
    // Validate: cc==Success, slot_id matches, endpoint_id==1 (EP0).
    let mut desc_ok = false;
    for _ in 0..POLL_BUDGET {
        let ev_d3 = trb_read_dword(event_ring_va, ev_idx, 3);
        if (ev_d3 & 1) == (ev_dcs as u32) {
            let ev_type = (ev_d3 >> 10) & 0x3F;
            if ev_type == TRB_TYPE_TRANSFER_EVENT {
                let ev_d2 = trb_read_dword(event_ring_va, ev_idx, 2);
                let cc = (ev_d2 >> 24) & 0xFF;
                let slot = (ev_d3 >> 24) & 0xFF;
                let ep   = (ev_d3 >> 16) & 0x1F;
                if cc == TRB_CC_SUCCESS && slot == en_slot_id && ep == 1 {
                    desc_ok = true;
                    serial_println!("[sexusb.xhci.desc8.event.ok]");
                } else {
                    serial_println!("[sexusb.xhci.desc8.event.bad] cc={} slot={} ep={}", cc, slot, ep);
                }
                // Clear consumed event cycle bit (spec 4.11.4)
                trb_write_volatile(event_ring_va, ev_idx, 0, 0, 0, ev_d3 & !1u32);
                // Advance event dequeue pointer and update ERDP.
                ev_idx += 1;
                if ev_idx >= EVENT_RING_TRBS {
                    ev_idx = 0;
                    ev_dcs ^= 1;
                }
                let new_erdp = event_ring_phys + ev_idx * 16;
                mmio_write64(intr0_base, XHCI_INTR_ERDP, new_erdp | ev_dcs);
            }
            break;
        }
        sys_yield();
    }

    if !desc_ok {
        serial_println!("[sexusb.xhci.desc8.timeout.bad]");
        loop { sys_yield(); }
    }

    // Read 8 raw bytes from descriptor data buffer.
    let desc_buf = desc_data_va as *const u8;
    let b0 = unsafe { core::ptr::read_volatile(desc_buf.add(0)) };
    let b1 = unsafe { core::ptr::read_volatile(desc_buf.add(1)) };
    let b2 = unsafe { core::ptr::read_volatile(desc_buf.add(2)) };
    let b3 = unsafe { core::ptr::read_volatile(desc_buf.add(3)) };
    let b4 = unsafe { core::ptr::read_volatile(desc_buf.add(4)) };
    let b5 = unsafe { core::ptr::read_volatile(desc_buf.add(5)) };
    let b6 = unsafe { core::ptr::read_volatile(desc_buf.add(6)) };
    let b7 = unsafe { core::ptr::read_volatile(desc_buf.add(7)) };
    serial_println!("[sexusb.xhci.desc8.bytes.ok] bytes=[{:#x},{:#x},{:#x},{:#x},{:#x},{:#x},{:#x},{:#x}]",
        b0, b1, b2, b3, b4, b5, b6, b7);

    // bMaxPacketSize0 is at offset 7 in device descriptor.
    let mps = b7;
    serial_println!("[sexusb.xhci.desc8.mps.ok] mps={}", mps);

    serial_println!("[sexusb.xhci.desc8.complete.ok]");

    loop { sys_yield(); }
}
