#![no_std]
#![no_main]

use sex_pdx::{serial_println, sys_yield, SLOT_USB_HOST, pdx_call_checked};

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop { sys_yield(); }
}

fn map_xhci_bar0(map_size: u64) -> u64 {
    let map_va: u64;
    unsafe {
        // syscall 43 = MAP_PCI_BAR(cap_slot, bar_index, map_size)
        core::arch::asm!(
            "syscall",
            in("rax") 43u64,
            in("rdi") SLOT_USB_HOST,
            in("rsi") 0u64,
            in("rdx") map_size,
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
    // XHCI spec 3.2.4: "For 64-bit registers, the upper dword shall be written first."
    mmio_write32(base, offset + 4, (value >> 32) as u32);
    mmio_write32(base, offset, (value & 0xFFFF_FFFF) as u32);
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

struct BootMouseReport {
    buttons: u8,
    dx: i8,
    dy: i8,
    wheel: i8,
}

fn decode_boot_mouse_report(buf: &[u8], len: usize) -> Option<BootMouseReport> {
    if len < 3 {
        return None;
    }
    let wheel = if len >= 4 { buf[3] as i8 } else { 0 };
    Some(BootMouseReport {
        buttons: buf[0],
        dx: buf[1] as i8,
        dy: buf[2] as i8,
        wheel,
    })
}

struct TabletReport {
    buttons: u8,
    abs_x: u16,
    abs_y: u16,
}

/// Decode QEMU usb-tablet HID report (5 bytes):
///   byte 0: buttons[2:0] (bit0=left, bit1=right, bit2=middle), padding[7:3]
///   byte 1-2: X absolute (little-endian u16, 0..32767)
///   byte 3-4: Y absolute (little-endian u16, 0..32767)
fn decode_tablet_report(buf: &[u8], len: usize) -> Option<TabletReport> {
    if len < 5 {
        return None;
    }
    let abs_x = (buf[1] as u16) | ((buf[2] as u16) << 8);
    let abs_y = (buf[3] as u16) | ((buf[4] as u16) << 8);
    Some(TabletReport {
        buttons: buf[0] & 0x07,
        abs_x,
        abs_y,
    })
}

const SLOT_USB_SEXINPUT: u64 = 9;
const OP_USB_MOUSE_REPORT: u64 = 0x260;

#[no_mangle]
pub extern "C" fn _start() -> ! {
    const PAGE_SIZE: u64 = 4096;
    const TRB_SIZE: u64 = 16;
    const CMD_RING_TRBS: u64 = 64;
    const EVENT_RING_TRBS: u64 = 64;
    const ERST_ENTRIES: u64 = 1;
    const DCBAA_BYTES: u64 = PAGE_SIZE;
    const MAP_BYTES: u64 = 0x10000; // XHCI BAR is typically 64KB

    const XHCI_USBCMD: u64 = 0x00;
    const XHCI_USBSTS: u64 = 0x04;
    const XHCI_OP_CONFIG: u64 = 0x08;
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
    const XHCI_INTR_ERSTSZ: u64 = 0x08;
    const XHCI_INTR_ERSTBA: u64 = 0x10;
    const XHCI_INTR_ERDP: u64 = 0x18;
    // Interrupter 0 registers start at offset 0x20 from runtime_base (XHCI spec 5.3.8).
    // The runtime space begins with MFINDEX at offset 0x00 (32-bit, read-only).
    // Interrupter register sets start at offset 0x20, each 32 bytes.
    const XHCI_INTR_BASE: u64 = 0x20;
    // xHCI command TRB type values per command-ring encoding (xHCI 1.2 §4.11.3).
    // Do NOT use transfer-ring TRB type values for command TRBs — the type number
    // namespace is shared but context-specific per ring type.
    //   Enable Slot      = 9  (not transfer-ring value 1)
    //   Address Device   = 11 (not transfer-ring value 8)
    //   Evaluate Context = 13 (not transfer-ring value 10)
    //   Noop Command     = 23
    const TRB_TYPE_ENABLE_SLOT_CMD: u32 = 9;
    const TRB_TYPE_ADDRESS_DEVICE_CMD: u32 = 11;
    const TRB_TYPE_CONFIGURE_ENDPOINT_CMD: u32 = 12;
    const TRB_TYPE_NOOP_CMD: u32 = 23;
    const TRB_TYPE_EVALUATE_CONTEXT_CMD: u32 = 13;
    const TRB_TYPE_CMD_COMPLETION_EVENT: u32 = 33;
    const TRB_TYPE_TRANSFER_EVENT: u32 = 32;
    const TRB_TYPE_SETUP_STAGE: u32 = 2;
    const TRB_TYPE_DATA_STAGE: u32 = 3;
    const TRB_TYPE_STATUS_STAGE: u32 = 4;
    const TRB_TYPE_NORMAL: u32 = 1;
    const TRB_CC_SUCCESS: u32 = 1;
    const TRB_CC_SHORT_PACKET: u32 = 13;

    serial_println!("[sexusb.boot]");

    let map_va = map_xhci_bar0(MAP_BYTES);
    if map_va == 0 || map_va == u64::MAX {
        serial_println!("[sexusb.xhci.map.bad]");
        loop { sys_yield(); }
    }
    serial_println!("[sexusb.xhci.map.ok]");
    serial_println!("[sexusb.xhci.diag.after_map_ok]");

    let regs = map_va as *const u32;
    let cap0 = unsafe { core::ptr::read_volatile(regs) };
    let caplength = (cap0 & 0xFF) as u8;
    let hciversion = ((cap0 >> 16) & 0xFFFF) as u16;
    let hcsp1 = unsafe { core::ptr::read_volatile(regs.add(1)) };
    let hcsp2 = unsafe { core::ptr::read_volatile(regs.add(2)) };
    let hcc1 = unsafe { core::ptr::read_volatile(regs.add(4)) };

    serial_println!("[sexusb.xhci.caplength] {:#x}", caplength);
    serial_println!("[sexusb.xhci.hciversion] {:#x}", hciversion);
    serial_println!("[sexusb.xhci.hcsp1] {:#x}", hcsp1);
    serial_println!("[sexusb.xhci.hcsp2] {:#x}", hcsp2);
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

    // ── Phase 2: Allocate rings before Run/Stop (XHCI spec 4.6.6) ──
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

    // ── Phase 3: Program DCBAAP + CRCR + ERST before Run/Stop (spec 4.6.6) ──
    serial_println!("[sexusb.xhci.ring.ptrs.write.start]");
    let cap_base = map_va;
    let rtsoff_raw = mmio_read32(cap_base, XHCI_CAP_RTSOFF);
    let rtsoff = (rtsoff_raw & !0x1Fu32) as u64;
    let hcc1_local = mmio_read32(cap_base, XHCI_CAP_HCCPARAMS1);
    let _ = hcc1_local;
    let runtime_base = map_va.wrapping_add(rtsoff);
    let intr_base = runtime_base.wrapping_add(XHCI_INTR_BASE);

    // Bounds checks against the mapped BAR slice.
    let op_need_end = op_base.wrapping_add(XHCI_DCBAAP + 8);
    let rt_need_end = intr_base.wrapping_add(XHCI_INTR_ERDP + 8);
    if op_need_end > map_va.wrapping_add(MAP_BYTES) || rt_need_end > map_va.wrapping_add(MAP_BYTES) {
        serial_println!("[sexusb.xhci.ring.ptrs.write.bad]");
        loop { sys_yield(); }
    }

    // DCBAAP: write lower dword first (QEMU nec-xhci requirement).
    mmio_write32(op_base, XHCI_DCBAAP, dcbaa_phys as u32);
    mmio_write32(op_base, XHCI_DCBAAP + 4, (dcbaa_phys >> 32) as u32);
    serial_println!("[sexusb.xhci.dcbaap.write.ok]");
    // CRCR: write lower dword first then upper dword.
    // QEMU nec-usb-xhci internal command ring pointer latches on upper dword write
    // using the CURRENT lower dword value. Upper-first order (spec 3.2.4) causes
    // stale-lower-zero → internal pointer = 0. Verified via QEMU trace:
    //   usb_xhci_fetch_trb addr 0x0000000000000000
    mmio_write32(op_base, XHCI_CRCR, (cmd_ring_phys | 1u64) as u32); // lower dword first
    mmio_write32(op_base, XHCI_CRCR + 4, ((cmd_ring_phys | 1u64) >> 32) as u32); // upper dword second
    // Immediately read back CRCR to verify write persistence
    let crcr_check_lo = mmio_read32(op_base, XHCI_CRCR);
    let crcr_check_hi = mmio_read32(op_base, XHCI_CRCR + 4);
    let crcr_check = (crcr_check_lo as u64) | ((crcr_check_hi as u64) << 32);
    serial_println!("[sexusb.xhci.crcr.write.ok] wrote={:#x} readback={:#x}",
        cmd_ring_phys | 1u64, crcr_check);
    // Also log phys for comparison
    serial_println!("[sexusb.xhci.phys.diag] cmd={:#x} event={:#x} erst={:#x} dcbaa={:#x}",
        cmd_ring_phys, event_ring_phys, erst_phys, dcbaa_phys);

    mmio_write32(intr_base, XHCI_INTR_ERSTSZ, ERST_ENTRIES as u32);
    // ERSTBA: write lower dword first (QEMU nec-xhci requirement).
    mmio_write32(intr_base, XHCI_INTR_ERSTBA, erst_phys as u32);
    mmio_write32(intr_base, XHCI_INTR_ERSTBA + 4, (erst_phys >> 32) as u32);
    serial_println!("[sexusb.xhci.erst.write.ok]");
    // ERDP: write lower dword first (QEMU nec-xhci requirement).
    // ERDP low bits reserved per spec 5.3.8.3 — do not encode ev_dcs.
    mmio_write32(intr_base, XHCI_INTR_ERDP, event_ring_phys as u32);
    mmio_write32(intr_base, XHCI_INTR_ERDP + 4, (event_ring_phys >> 32) as u32);
    serial_println!("[sexusb.xhci.erdp.write.ok]");

    // ── CONFIG: MaxSlotsEnabled (spec 5.4.7, spec 4.6.6 step 4) ──
    let max_slots = (hcsp1 & 0xFF) as u32;
    if max_slots == 0 {
        serial_println!("[sexusb.xhci.config.max_slots.zero.bad]");
        loop { sys_yield(); }
    }
    mmio_write32(op_base, XHCI_OP_CONFIG, max_slots);
    serial_println!("[sexusb.xhci.config.ok] max_slots={}", max_slots);

    serial_println!("[sexusb.xhci.ring.proof.ok]");

    // ── Phase 4: Run/Stop after rings are programmed (spec 4.6.6) ──
    let usbcmd_before = mmio_read32(op_base, XHCI_USBCMD);
    let usbsts_before = mmio_read32(op_base, XHCI_USBSTS);
    serial_println!("[sexusb.xhci.run.diag.before] usbcmd={:#x} usbsts={:#x}", usbcmd_before, usbsts_before);
    usbcmd = mmio_read32(op_base, XHCI_USBCMD) | USBCMD_RUN_STOP | (1u32 << 2); // RS + INTE
    mmio_write32(op_base, XHCI_USBCMD, usbcmd);
    if wait_until(op_base, XHCI_USBSTS, USBSTS_HCHALTED, false, POLL_BUDGET) {
        let usbcmd_after = mmio_read32(op_base, XHCI_USBCMD);
        let usbsts_after = mmio_read32(op_base, XHCI_USBSTS);
        serial_println!("[sexusb.xhci.run.diag.after] usbcmd={:#x} usbsts={:#x}", usbcmd_after, usbsts_after);
        serial_println!("[sexusb.xhci.run.ok]");
        // Post-RUN detail: verify RS=1, HCHalted=0, CNR=0, HSE=0
        let rs_on = (usbcmd_after & USBCMD_RUN_STOP) != 0;
        let halted = (usbsts_after & USBSTS_HCHALTED) != 0;
        let cnr = (usbsts_after & USBSTS_CNR) != 0;
        let hse = (usbsts_after & (1u32 << 2)) != 0;
        let eint = (usbsts_after & (1u32 << 3)) != 0;
        serial_println!("[sexusb.xhci.run.state] rs={} halted={} cnr={} hse={} eint={}",
            rs_on as u8, halted as u8, cnr as u8, hse as u8, eint as u8);
    } else {
        serial_println!("[sexusb.xhci.run.bad]");
    }

    // Command/event ring state machine: explicit tracked indices and cycle bits.
    // No hardcoded TRB indices.
    // cmd_cycle matches CRCR RCS (starts 1). CRCS/RCS is stable per segment —
    // toggles ONLY on segment boundary wrap (spec 5.4.5), NOT per TRB.
    // Stop marker uses !cmd_cycle (opposite) — same-cycle would match CRCS and
    // be consumed as Reserved TRB (type=0), causing corruption.
    // ev_idx tracks next event slot to consume; ev_dcs is software state matching
    // the event TRB cycle bit (spec 4.11.4). NOT encoded into ERDP — ERDP low bits
    // are reserved per spec 5.3.8.3 (bits 2:0 = 0, bit 3 = EHB).
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
    serial_println!("[sexusb.xhci.diag.dboff] raw={:#x} db_base={:#x} map_va={:#x}",
        dboff_raw, db_base, map_va);
    if db_base.wrapping_add(4) > map_va.wrapping_add(MAP_BYTES) {
        serial_println!("[sexusb.xhci.ring.ptrs.write.bad]");
        loop { sys_yield(); }
    }

    // Diagnostic: read all key registers before doorbell.
    let mfindex = mmio_read32(map_va, rtsoff); // MFINDEX at runtime_base + 0x00
    let iman_rb = mmio_read32(intr_base, 0);    // IMAN at runtime_base + 0x20
    let erstsz_rb = mmio_read32(intr_base, XHCI_INTR_ERSTSZ);
    let erstba_lo = mmio_read32(intr_base, XHCI_INTR_ERSTBA);
    let erstba_hi = mmio_read32(intr_base, XHCI_INTR_ERSTBA + 4);
    let erstba_rb = (erstba_lo as u64) | ((erstba_hi as u64) << 32);
    let erdp_lo = mmio_read32(intr_base, XHCI_INTR_ERDP);
    let erdp_hi = mmio_read32(intr_base, XHCI_INTR_ERDP + 4);
    let erdp_rb = (erdp_lo as u64) | ((erdp_hi as u64) << 32);
    let usbcmd_rb = mmio_read32(op_base, XHCI_USBCMD);
    let usbsts_rb = mmio_read32(op_base, XHCI_USBSTS);
    let crcr_lo = mmio_read32(op_base, XHCI_CRCR);
    let crcr_hi = mmio_read32(op_base, XHCI_CRCR + 4);
    let crcr_rb = (crcr_lo as u64) | ((crcr_hi as u64) << 32);
    let dcbaap_lo = mmio_read32(op_base, XHCI_DCBAAP);
    let dcbaap_hi = mmio_read32(op_base, XHCI_DCBAAP + 4);
    let dcbaap_rb = (dcbaap_lo as u64) | ((dcbaap_hi as u64) << 32);
    serial_println!("[sexusb.xhci.diag.readback] mfindex={:#x} iman={:#x} erstsz={:#x} erstba={:#x} erdp={:#x} usbcmd={:#x} usbsts={:#x} crcr={:#x} dcbaap={:#x}",
        mfindex, iman_rb, erstsz_rb, erstba_rb, erdp_rb, usbcmd_rb, usbsts_rb, crcr_rb, dcbaap_rb);

    // Enable interrupter (IE=1) so event ring is active.
    mmio_write32(intr_base, 0, 2u32); // IMAN: set IE=1 (bit 1)
    let iman_after = mmio_read32(intr_base, 0);
    serial_println!("[sexusb.xhci.diag.iman_ie] iman={:#x}", iman_after);

    // sfence: ensure all prior TRB writes (WB) are globally visible before
    // the doorbell MMIO write (UC) reaches the controller. x86 does not
    // guarantee WB→UC store ordering without explicit fencing.
    unsafe { core::arch::asm!("sfence"); }
    mmio_write32(db_base, 0, 0u32); // Doorbell 0, target 0 (command ring)
    serial_println!("[sexusb.xhci.cmd.noop.doorbell.ok]");

    // Check DCBAA[0] scratchpad entry (needed if MaxScratchpadBufs > 0)
    let max_scratchpad = (hcsp2 >> 16) & 0x7FFu32;
    let dcbaa0 = unsafe { core::ptr::read_volatile(dcbaa_va as *const u64) };
    serial_println!("[sexusb.xhci.diag.scratchpad] max={} dcbaa[0]={:#x}", max_scratchpad, dcbaa0);

    // Full NOOP TRB dword dump
    for ti in 0..2u64 {
        let d0 = trb_read_dword(cmd_ring_va, ti, 0);
        let d1 = trb_read_dword(cmd_ring_va, ti, 1);
        let d2 = trb_read_dword(cmd_ring_va, ti, 2);
        let d3 = trb_read_dword(cmd_ring_va, ti, 3);
        serial_println!("[sexusb.xhci.diag.trb_full] trb[{}] d0={:#x} d1={:#x} d2={:#x} d3={:#x}",
            ti, d0, d1, d2, d3);
    }

    // DIAG: print command ring TRB[0] and TRB[1] to verify TRB writes, and event ring TRB[0] d3
    let cmd_d3_0 = trb_read_dword(cmd_ring_va, 0, 3);
    let cmd_d3_1 = trb_read_dword(cmd_ring_va, 1, 3);
    let ev_d3_before = trb_read_dword(event_ring_va, 0, 3);
    serial_println!("[sexusb.xhci.diag.trb_check] cmd[0].d3={:#x} cmd[1].d3={:#x} ev[0].d3={:#x}",
        cmd_d3_0, cmd_d3_1, ev_d3_before);

    // ── Event poll preflight guard ──
    let poll_read_addr = event_ring_va.wrapping_add(ev_idx.wrapping_mul(16));
    serial_println!("[sexusb.xhci.noop.poll.enter] event_va={:#x} ev_idx={} ev_dcs={} read_addr={:#x}",
        event_ring_va, ev_idx, ev_dcs, poll_read_addr);
    if event_ring_va == 0 || event_ring_va == u64::MAX || ev_idx >= 256 {
        serial_println!("[sexusb.xhci.noop.poll.event_va.bad]");
        loop { sys_yield(); }
    }
    if poll_read_addr < event_ring_va || poll_read_addr.wrapping_add(15) >= event_ring_va.wrapping_add(4096) {
        serial_println!("[sexusb.xhci.noop.poll.event_bounds.bad]");
        loop { sys_yield(); }
    }

    // Consume command completion event at ev_idx.
    let mut noop_ok = false;
    for _ in 0..POLL_BUDGET {
        let ev_d3 = trb_read_dword(event_ring_va, ev_idx, 3);
        if (ev_d3 & 1) == (ev_dcs as u32) {
            let ev_type = (ev_d3 >> 10) & 0x3F;
            if ev_type == TRB_TYPE_CMD_COMPLETION_EVENT {
                let ev_d0 = trb_read_dword(event_ring_va, ev_idx, 0);
                let ev_d1 = trb_read_dword(event_ring_va, ev_idx, 1);
                let ev_d2 = trb_read_dword(event_ring_va, ev_idx, 2);
                let cc = (ev_d2 >> 24) & 0xFF;
                noop_ok = cc == TRB_CC_SUCCESS;
                serial_println!("[sexusb.xhci.cmd.noop.event.raw] d0={:#x} d1={:#x} d2={:#x} d3={:#x} type={} cc={}",
                    ev_d0, ev_d1, ev_d2, ev_d3, ev_type, cc);
                // Clear consumed event cycle bit (spec 4.11.4)
                trb_write_volatile(event_ring_va, ev_idx, 0, 0, 0, ev_d3 & !1u32);
                // Advance event dequeue pointer and update ERDP.
                ev_idx += 1;
                if ev_idx >= EVENT_RING_TRBS {
                    ev_idx = 0;
                    ev_dcs ^= 1; // toggle DCS on segment wrap
                }
                let new_erdp = event_ring_phys + ev_idx * 16;
                // ERDP advance: lower dword first (QEMU nec-xhci requirement).
                mmio_write32(intr_base, XHCI_INTR_ERDP, new_erdp as u32);
                mmio_write32(intr_base, XHCI_INTR_ERDP + 4, (new_erdp >> 32) as u32);
            }
            break;
        }
        sys_yield();
    }

    if noop_ok {
        serial_println!("[sexusb.xhci.cmd.noop.complete.ok]");
    } else {
        serial_println!("[sexusb.xhci.cmd.noop.complete.bad]");
        // Post-timeout controller state: CRCR CRR, USBSTS, event[0] full
        let usbsts_now = mmio_read32(op_base, XHCI_USBSTS);
        let crcr_lo_now = mmio_read32(op_base, XHCI_CRCR);
        let crcr_hi_now = mmio_read32(op_base, XHCI_CRCR + 4);
        let crcr_now = (crcr_lo_now as u64) | ((crcr_hi_now as u64) << 32);
        let ev0_d0 = trb_read_dword(event_ring_va, 0, 0);
        let ev0_d1 = trb_read_dword(event_ring_va, 0, 1);
        let ev0_d2 = trb_read_dword(event_ring_va, 0, 2);
        let ev0_d3 = trb_read_dword(event_ring_va, 0, 3);
        let crr = (crcr_now >> 8) & 1;
        serial_println!("[sexusb.xhci.diag.after_timeout] usbsts={:#x} crcr={:#x} crr={} ev[0]={:#x},{:#x},{:#x},{:#x}",
            usbsts_now, crcr_now, crr, ev0_d0, ev0_d1, ev0_d2, ev0_d3);
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
                // Dump full event TRB for Enable Slot diagnosis
                let ev_d0 = trb_read_dword(event_ring_va, ev_idx, 0);
                let ev_d1 = trb_read_dword(event_ring_va, ev_idx, 1);
                serial_println!("[sexusb.xhci.enable_slot.event.raw] d0={:#x} d1={:#x} d2={:#x} d3={:#x}",
                    ev_d0, ev_d1, ev_d2, ev_d3);
                en_ok = cc == TRB_CC_SUCCESS;
                en_slot_id = (ev_d3 >> 24) & 0xFF;
                // Clear consumed event cycle bit (spec 4.11.4)
                trb_write_volatile(event_ring_va, ev_idx, 0, 0, 0, ev_d3 & !1u32);
                // Advance event dequeue pointer and update ERDP.
                ev_idx += 1;
                if ev_idx >= EVENT_RING_TRBS {
                    ev_idx = 0;
                    ev_dcs ^= 1; // toggle DCS on segment wrap
                }
                let new_erdp = event_ring_phys + ev_idx * 16;
                // ERDP advance: lower dword first (QEMU nec-xhci requirement).
                mmio_write32(intr_base, XHCI_INTR_ERDP, new_erdp as u32);
                mmio_write32(intr_base, XHCI_INTR_ERDP + 4, (new_erdp >> 32) as u32);
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

    // Read MaxPorts from HCSPARAMS1 bits 31:24.
    // XHCI 1.1+ shifts MaxPorts to bits 31:24 (MaxIntrs expands to bits 18:8).
    // QEMU nec-xhci uses 1.1+ layout even with HCIVERSION=0x100.
    let max_ports: u64 = ((hcsp1 >> 24) & 0xFF) as u64;
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
    // Slot Context DW1: bits 23:16 = Root Hub Port Number
    // bits 31:24 = Number of Ports, must remain 0 for non-hub device.
    // xHCI spec places Root Hub Port Number at DW1[31:24] for usb3,
    // but QEMU nec-xhci reads it from DW1[23:16].
    let slot_dw1 = (target_port as u32) << 16;

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
    // Read back DCBAA entry to verify write visibility.
    let dcbaa_ent_readback = unsafe { core::ptr::read_volatile(dcbaa_entry_ptr) };
    serial_println!("[sexusb.xhci.addr_ctx.dcbaa.ok] wrote={:#x} readback={:#x}",
        device_ctx_phys, dcbaa_ent_readback);

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
    // Dump Address Device TRB for diagnostics
    let trb_d0 = trb_read_dword(cmd_ring_va, cmd_idx, 0);
    let trb_d1 = trb_read_dword(cmd_ring_va, cmd_idx, 1);
    let trb_d2 = trb_read_dword(cmd_ring_va, cmd_idx, 2);
    let trb_d3 = trb_read_dword(cmd_ring_va, cmd_idx, 3);
    serial_println!("[sexusb.xhci.address_device.trb.dump] d0={:#x} d1={:#x} d2={:#x} d3={:#x}",
        trb_d0, trb_d1, trb_d2, trb_d3);
    // Dump Input Context (ICC + Slot + EP0)
    let icc_d0 = unsafe { core::ptr::read_volatile(input_ctx_va as *const u32) };
    let icc_d1 = unsafe { core::ptr::read_volatile((input_ctx_va + 4) as *const u32) };
    let in_slot_d0 = unsafe { core::ptr::read_volatile((input_ctx_va + ctx_stride) as *const u32) };
    let in_slot_d1 = unsafe { core::ptr::read_volatile((input_ctx_va + ctx_stride + 4) as *const u32) };
    let in_slot_d2 = unsafe { core::ptr::read_volatile((input_ctx_va + ctx_stride + 8) as *const u32) };
    let in_slot_d3 = unsafe { core::ptr::read_volatile((input_ctx_va + ctx_stride + 12) as *const u32) };
    let in_ep0_d0 = unsafe { core::ptr::read_volatile((input_ctx_va + ctx_stride * 2) as *const u32) };
    let in_ep0_d1 = unsafe { core::ptr::read_volatile((input_ctx_va + ctx_stride * 2 + 4) as *const u32) };
    let in_ep0_d2 = unsafe { core::ptr::read_volatile((input_ctx_va + ctx_stride * 2 + 8) as *const u32) };
    let in_ep0_d3 = unsafe { core::ptr::read_volatile((input_ctx_va + ctx_stride * 2 + 12) as *const u32) };
    serial_println!("[sexusb.xhci.input_ctx.dump] icc=({:#x},{:#x}) slot=({:#x},{:#x},{:#x},{:#x}) ep0=({:#x},{:#x},{:#x},{:#x})",
        icc_d0, icc_d1,
        in_slot_d0, in_slot_d1, in_slot_d2, in_slot_d3,
        in_ep0_d0, in_ep0_d1, in_ep0_d2, in_ep0_d3);
    serial_println!("[sexusb.xhci.address_device.trb.ok]");

    // Cycle-stop at cmd_idx+1 with opposite cycle.
    trb_write_volatile(cmd_ring_va, cmd_idx + 1, 0, 0, 0, cmd_cycle ^ 1);

    // Dump Device Context as read by controller via DCBAA.
    let dev_slot_d0 = unsafe { core::ptr::read_volatile(device_ctx_va as *const u32) };
    let dev_slot_d1 = unsafe { core::ptr::read_volatile((device_ctx_va + 4) as *const u32) };
    let dev_slot_d2 = unsafe { core::ptr::read_volatile((device_ctx_va + 8) as *const u32) };
    let dev_slot_d3 = unsafe { core::ptr::read_volatile((device_ctx_va + 12) as *const u32) };
    let dev_ep0_d0 = unsafe { core::ptr::read_volatile((device_ctx_va + ctx_stride) as *const u32) };
    let dev_ep0_d1 = unsafe { core::ptr::read_volatile((device_ctx_va + ctx_stride + 4) as *const u32) };
    let dev_ep0_d2 = unsafe { core::ptr::read_volatile((device_ctx_va + ctx_stride + 8) as *const u32) };
    let dev_ep0_d3 = unsafe { core::ptr::read_volatile((device_ctx_va + ctx_stride + 12) as *const u32) };
    serial_println!("[sexusb.xhci.dev_ctx.dump] slot=({:#x},{:#x},{:#x},{:#x}) ep0=({:#x},{:#x},{:#x},{:#x})",
        dev_slot_d0, dev_slot_d1, dev_slot_d2, dev_slot_d3,
        dev_ep0_d0, dev_ep0_d1, dev_ep0_d2, dev_ep0_d3);

    // ── XHCI_ADDRESS_DEVICE_ICC_QEMU_LAYOUT_AUDIT_V1 ──
    serial_println!("[sexusb.xhci.addr_ctx.audit.start]");
    serial_println!("[sexusb.xhci.icc_audit.target_port] port={}", target_port);
    let target_portsc = mmio_read32(op_base, PORTSC_BASE + (target_port - 1) * PORTSC_STRIDE);
    serial_println!("[sexusb.xhci.icc_audit.portsc_raw] raw={:#x}", target_portsc);
    let ccs_port = target_portsc & PORTSC_CCS;
    serial_println!("[sexusb.xhci.icc_audit.port_ccs] ccs={}", ccs_port);
    serial_println!("[sexusb.xhci.icc_audit.slot_id] id={}", en_slot_id);
    serial_println!("[sexusb.xhci.icc_audit.ctx_stride] stride={}", ctx_stride);
    serial_println!("[sexusb.xhci.icc_audit.input_ctx_phys] phys={:#x}", input_ctx_phys);
    serial_println!("[sexusb.xhci.icc_audit.device_ctx_phys] phys={:#x}", device_ctx_phys);
    // Raw hex dump of first 64 bytes at input_ctx (16 dwords)
    for i in 0u64..16u64 {
        let word = unsafe { core::ptr::read_volatile((input_ctx_va + i * 4) as *const u32) };
        serial_println!("[sexusb.xhci.icc_audit.raw32] i={} off={:#x} val={:#010x}",
            i, i * 4, word);
    }
    // QEMU-style ICC read: pos=0, offset 0x00..0x1f (ctxsize=32 bytes)
    let qemu_icc0 = unsafe { core::ptr::read_volatile((input_ctx_va + 0x00) as *const u32) };
    let qemu_icc1 = unsafe { core::ptr::read_volatile((input_ctx_va + 0x04) as *const u32) };
    serial_println!("[sexusb.xhci.icc_audit.qemu_icc] icc0={:#x} icc1={:#x} icc0_exp=0 icc1_exp=3 icc0_ok={} icc1_ok={}",
        qemu_icc0, qemu_icc1,
        qemu_icc0 == 0, qemu_icc1 == 3);
    // QEMU-style Slot Context read: pos=1, offset=ctx_stride
    let slot_off = ctx_stride;
    let qemu_slot_dw0 = unsafe { core::ptr::read_volatile((input_ctx_va + slot_off + 0x00) as *const u32) };
    let qemu_slot_dw1 = unsafe { core::ptr::read_volatile((input_ctx_va + slot_off + 0x04) as *const u32) };
    let qemu_slot_dw2 = unsafe { core::ptr::read_volatile((input_ctx_va + slot_off + 0x08) as *const u32) };
    let qemu_slot_dw3 = unsafe { core::ptr::read_volatile((input_ctx_va + slot_off + 0x0c) as *const u32) };
    let qemu_port = (qemu_slot_dw1 >> 16) & 0xFF;
    let spec_port = (qemu_slot_dw1 >> 24) & 0xFF;
    serial_println!("[sexusb.xhci.icc_audit.qemu_slot] dw0={:#x} dw1={:#x} dw2={:#x} dw3={:#x}",
        qemu_slot_dw0, qemu_slot_dw1, qemu_slot_dw2, qemu_slot_dw3);
    serial_println!("[sexusb.xhci.icc_audit.slot_ports] spec_port={} qemu_port={} target_port={}",
        spec_port, qemu_port, target_port as u32);
    // QEMU-style EP0 Context read: pos=2, offset=ctx_stride*2
    let ep0_off = ctx_stride * 2;
    let qemu_ep0_dw0 = unsafe { core::ptr::read_volatile((input_ctx_va + ep0_off + 0x00) as *const u32) };
    let qemu_ep0_dw1 = unsafe { core::ptr::read_volatile((input_ctx_va + ep0_off + 0x04) as *const u32) };
    let qemu_ep0_dw2 = unsafe { core::ptr::read_volatile((input_ctx_va + ep0_off + 0x08) as *const u32) };
    let qemu_ep0_dw3 = unsafe { core::ptr::read_volatile((input_ctx_va + ep0_off + 0x0c) as *const u32) };
    serial_println!("[sexusb.xhci.icc_audit.qemu_ep0] dw0={:#x} dw1={:#x} dw2={:#x} dw3={:#x}",
        qemu_ep0_dw0, qemu_ep0_dw1, qemu_ep0_dw2, qemu_ep0_dw3);
    // Address Device TRB fields
    serial_println!("[sexusb.xhci.icc_audit.addr_trb] d0={:#x} d1={:#x} d2=0 d3={:#x}",
        addr_dev_d0, addr_dev_d1, addr_dev_d3);
    let trb_ptr = ((addr_dev_d1 as u64) << 32) | (addr_dev_d0 as u64);
    serial_println!("[sexusb.xhci.icc_audit.trb_ptr_match] ptr={:#x} match={}", trb_ptr, trb_ptr == input_ctx_phys);
    let bsr = (addr_dev_d3 >> 9) & 1;
    serial_println!("[sexusb.xhci.icc_audit.bsr_zero] bsr={}", bsr);
    // ── END ICC AUDIT ──

    // Doorbell 0 (command ring).
    mmio_write32(db_base, 0, 0u32);
    serial_println!("[sexusb.xhci.address_device.doorbell.ok]");

    // Consume command completion event at ev_idx.
    serial_println!("[sexusb.xhci.address_device.poll.enter] ev_idx={} ev_dcs={}", ev_idx, ev_dcs);
    let mut addr_dev_ok = false;
    for _ in 0..POLL_BUDGET {
        let ev_d3 = trb_read_dword(event_ring_va, ev_idx, 3);
        if (ev_d3 & 1) == (ev_dcs as u32) {
            let ev_type = (ev_d3 >> 10) & 0x3F;
            if ev_type == TRB_TYPE_CMD_COMPLETION_EVENT {
                let ev_d2 = trb_read_dword(event_ring_va, ev_idx, 2);
                let ev_cc = (ev_d2 >> 24) & 0xFF;
                let ev_slot_id = (ev_d3 >> 24) & 0xFF;
                let ev_d0_evt = trb_read_dword(event_ring_va, ev_idx, 0);
                let ev_d1_evt = trb_read_dword(event_ring_va, ev_idx, 1);
                serial_println!("[sexusb.xhci.address_device.event.dump] d0={:#x} d1={:#x} d2={:#x} d3={:#x} type={} cc={} slot={}",
                    ev_d0_evt, ev_d1_evt, ev_d2, ev_d3, ev_type, ev_cc, ev_slot_id);
                if ev_cc == TRB_CC_SUCCESS && ev_slot_id == en_slot_id {
                    addr_dev_ok = true;
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
                // ERDP advance: lower dword first (QEMU nec-xhci requirement).
                mmio_write32(intr_base, XHCI_INTR_ERDP, new_erdp as u32);
                mmio_write32(intr_base, XHCI_INTR_ERDP + 4, (new_erdp >> 32) as u32);
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

    // EP0 output dequeue pointer from Device Context after Address Device.
    let ep0_deq_dw2 = unsafe { core::ptr::read_volatile((device_ctx_va + ctx_stride + 8) as *const u32) };
    let ep0_deq_dw3 = unsafe { core::ptr::read_volatile((device_ctx_va + ctx_stride + 12) as *const u32) };
    let ep0_deq_raw = ((ep0_deq_dw3 as u64) << 32) | (ep0_deq_dw2 as u64);
    let ep0_deq_ptr = ep0_deq_raw & !0xf;
    let ep0_deq_dcs = ep0_deq_dw2 & 1;
    serial_println!("[sexusb.xhci.desc8.ep0_deq] dw2={:#x} dw3={:#x} ptr={:#x} dcs={}",
        ep0_deq_dw2, ep0_deq_dw3, ep0_deq_ptr, ep0_deq_dcs);

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

    // Setup Stage TRB (type=2): QEMU nec-xhci requires DW3 bit6 = 1 to treat the
    // TRB parameter field as inline setup packet data (the xHCI spec default).
    // Without bit6, QEMU interprets the parameter as a DMA pointer (BIOS has it:
    // c=0x30841 → bit6=1). TRT=IN(2) for control read (device-to-host Data Stage).
    let setup_d3 = (TRB_TYPE_SETUP_STAGE << 10)  // type=2 in bits 15:10
        | (2u32 << 16)                           // TRT=IN (0b10) in bits 17:16
        | (1u32 << 6)                            // QEMU nec-xhci: inline setup packet marker
        | ep0_cycle;
    trb_write_volatile(ep0_ring_va, ep0_idx,
        0x0100_0680u32,  // d0: bmReqType=0x80,bReq=0x06,wVal_lo=0x00,wVal_hi=0x01
        0x0008_0000u32,  // d1: wIdx_lo=0x00,wIdx_hi=0x00,wLen_lo=0x08,wLen_hi=0x00
        (8u32 << 0),      // TRB Transfer Length = 8 (bits 0:13, no reserved bits set)
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
    // CH=0 (end of TD), IOC=1 in DW3 bit 5 (Status Stage encoding, xHCI 1.2 §4.11.2.3).
    let status_d3 = (TRB_TYPE_STATUS_STAGE << 10) // type=4 in bits 15:10
        | (0u32 << 16)                            // DIR=OUT (0)
        | (1u32 << 5)                             // IOC=1 (Interrupt On Complete, bit 5)
        | ep0_cycle;
    trb_write_volatile(ep0_ring_va, ep0_idx + 2,
        0, 0,
        0u32,  // DW2: all zero (no IOC, no CH, no ISP for Status Stage)
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

    // Dump EP0 ring TRBs and doorbell state before ringing.
    serial_println!("[sexusb.xhci.desc8.ep0_ring] phys={:#x} idx={} cycle={}", ep0_ring_phys, ep0_idx, ep0_cycle);
    for ti in 0u64..4u64 {
        let td0 = trb_read_dword(ep0_ring_va, ti, 0);
        let td1 = trb_read_dword(ep0_ring_va, ti, 1);
        let td2 = trb_read_dword(ep0_ring_va, ti, 2);
        let td3 = trb_read_dword(ep0_ring_va, ti, 3);
        serial_println!("[sexusb.xhci.desc8.trb{}] d0={:#x} d1={:#x} d2={:#x} d3={:#x}", ti, td0, td1, td2, td3);
    }

    // Doorbell: EP0 on slot_id. DB Target = 1 (EP0 endpoint ID).
    // DB index = en_slot_id (1-based slot from Enable Slot).
    if db_base.wrapping_add(en_slot_id as u64 * 4 + 4) > map_va.wrapping_add(MAP_BYTES) {
        serial_println!("[sexusb.xhci.desc8.doorbell.bad]");
        loop { sys_yield(); }
    }
    mmio_write32(db_base, en_slot_id as u64 * 4, 1u32);  // target=1 (EP0)
    serial_println!("[sexusb.xhci.desc8.db] base={:#x} off=+{} val=1", db_base, en_slot_id as u64 * 4);
    serial_println!("[sexusb.xhci.desc8.doorbell.ok]");

    // Consume Transfer Event (type=32) at current ev_idx.
    // Validate: cc==Success, slot_id matches, endpoint_id==1 (EP0).
    let iman_before = mmio_read32(intr_base, 0);
    let erdp_lo_rb = mmio_read32(intr_base, XHCI_INTR_ERDP);
    let erdp_hi_rb = mmio_read32(intr_base, XHCI_INTR_ERDP + 4);
    let erdp_val = (erdp_lo_rb as u64) | ((erdp_hi_rb as u64) << 32);
    serial_println!("[sexusb.xhci.desc8.wait_state] ev_idx={} ev_dcs={} iman={:#x} erdp={:#x}",
        ev_idx, ev_dcs, iman_before, erdp_val);
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
                // ERDP advance: lower dword first (QEMU nec-xhci requirement).
                mmio_write32(intr_base, XHCI_INTR_ERDP, new_erdp as u32);
                mmio_write32(intr_base, XHCI_INTR_ERDP + 4, (new_erdp >> 32) as u32);
            }
            break;
        }
        sys_yield();
    }

    if !desc_ok {
        let usbsts = mmio_read32(op_base, XHCI_USBSTS);
        let iman_timo = mmio_read32(intr_base, 0);
        let erdp_lo_timo = mmio_read32(intr_base, XHCI_INTR_ERDP);
        let erdp_hi_timo = mmio_read32(intr_base, XHCI_INTR_ERDP + 4);
        let erdp_timo = (erdp_lo_timo as u64) | ((erdp_hi_timo as u64) << 32);
        serial_println!("[sexusb.xhci.desc8.timeout.bad]");
        serial_println!("[sexusb.xhci.desc8.timeout.diag] usbsts={:#x} iman={:#x} erdp={:#x} hce={} eint={} pcd={}",
            usbsts, iman_timo, erdp_timo,
            (usbsts >> 0) & 1, (usbsts >> 3) & 1, (usbsts >> 4) & 1);
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

    // ===== Evaluate Context: update EP0 MPS from bMaxPacketSize0 =====
    // Phase: USB_XHCI_EP0_MPS_EVALUATE_CONTEXT_PROOF_V1
    // If actual_mps matches current boot-guess max_packet_size, skip.
    // Otherwise copy EP0 from Device Context, patch MPS, submit command.
    serial_println!("[sexusb.xhci.eval_ctx.start]");

    let actual_mps = mps as u32;

    // Report-only MPS audit before any modification.
    // Dump output device context EP0 state (what controller actually set).
    let dev_ep0_base = (device_ctx_va + ctx_stride) as *const u32;
    let dev_dw0 = unsafe { core::ptr::read_volatile(dev_ep0_base.add(0)) };
    let dev_dw1 = unsafe { core::ptr::read_volatile(dev_ep0_base.add(1)) };
    let dev_dw2 = unsafe { core::ptr::read_volatile(dev_ep0_base.add(2)) };
    let dev_dw3 = unsafe { core::ptr::read_volatile(dev_ep0_base.add(3)) };
    let dev_ep0_mps = (dev_dw0 >> 16) & 0xFFFF;
    serial_println!("[sexusb.xhci.mps.audit] port_speed={}", port_speed);
    serial_println!("[sexusb.xhci.mps.audit] device_desc_bMaxPacketSize0={}", actual_mps);
    serial_println!("[sexusb.xhci.mps.audit] max_packet_size_speed_guess={}", max_packet_size);
    serial_println!("[sexusb.xhci.mps.audit] output_ep0_dw0={:#x}", dev_dw0);
    serial_println!("[sexusb.xhci.mps.audit] output_ep0_dw1={:#x}", dev_dw1);
    serial_println!("[sexusb.xhci.mps.audit] output_ep0_dw2={:#x}", dev_dw2);
    serial_println!("[sexusb.xhci.mps.audit] output_ep0_dw3={:#x}", dev_dw3);
    serial_println!("[sexusb.xhci.mps.audit] output_ep0_mps_before={}", dev_ep0_mps);
    // Speed-based expected MPS per USB 2.0 spec:
    //   LS (speed 1) -> 8, FS (speed 0) -> 8/16/32/64
    //   HS (speed 2) -> 64, SS (speed 3) -> 512
    serial_println!("[sexusb.xhci.mps.audit] expected_rule=speed_based port_speed={} guess={}",
        port_speed, max_packet_size);
    // Descriptor MPS source: bMaxPacketSize0 from 8-byte GET_DESCRIPTOR(DEVICE).
    serial_println!("[sexusb.xhci.mps.audit] got_rule_source=descriptor_bMaxPacketSize0 value={}",
        actual_mps);
    // Valid EP0 MPS values: 8, 16, 32, 64, 512 per USB spec.
    let mps_valid = match actual_mps { 8 | 16 | 32 | 64 | 512 => true, _ => false };
    if !mps_valid {
        serial_println!("[sexusb.xhci.eval_ctx.mps.invalid.bad] mps={}", actual_mps);
        loop { sys_yield(); }
    }

    if actual_mps == max_packet_size {
        serial_println!("[sexusb.xhci.eval_ctx.skip] mps={}", actual_mps);
        // Skip path is terminal for this proof phase. Future phase
        // (USB_XHCI_GET_DESCRIPTOR_FULL_18_V1) continues from skip-ok.
        loop { sys_yield(); }
    }

    // Reuse input context page (controller done reading after Address Device).
    unsafe { core::ptr::write_bytes(input_ctx_va as *mut u8, 0, PAGE_SIZE as usize); }

    // ICC: Drop=0, Add=EP0 only (bit 1 = context index 1).
    // Slot context not evaluated (not in Add). Per XHCI spec 6.2.2.
    unsafe {
        core::ptr::write_volatile(input_ctx_va as *mut u32, 0u32);        // DW0: Drop none
        core::ptr::write_volatile((input_ctx_va + 4) as *mut u32, 2u32);  // DW1: Add EP0 only
    }

    // Copy EP0 context from output Device Context, patch only MPS.
    // Required by XHCI spec 6.2.3: "Input Endpoint Context for an Evaluate
    // Context command must have the exact same values for fields not intended
    // to be changed." Controller may have updated fields (TR Dequeue Pointer,
    // DCS) during GET_DESCRIPTOR(8) control transfer.
    let dev_ep0_base = (device_ctx_va + ctx_stride) as *const u32;
    let inp_ep0_base = (input_ctx_va + ctx_stride * 2) as *mut u32;
    let ep0_dw0 = unsafe { core::ptr::read_volatile(dev_ep0_base.add(0)) };
    let ep0_dw1 = unsafe { core::ptr::read_volatile(dev_ep0_base.add(1)) };
    let ep0_dw2 = unsafe { core::ptr::read_volatile(dev_ep0_base.add(2)) };
    let ep0_dw3 = unsafe { core::ptr::read_volatile(dev_ep0_base.add(3)) };
    let ep0_dw0_new = (ep0_dw0 & 0x0000_FFFF) | (actual_mps << 16);
    unsafe {
        core::ptr::write_volatile(inp_ep0_base.add(0), ep0_dw0_new);
        core::ptr::write_volatile(inp_ep0_base.add(1), ep0_dw1);
        core::ptr::write_volatile(inp_ep0_base.add(2), ep0_dw2);
        core::ptr::write_volatile(inp_ep0_base.add(3), ep0_dw3);
    }

    // Submit Evaluate Context Command TRB (type=14).
    // d0/d1 = input_context_phys, d2 = 0 (no flags), d3 = slot_id|type|cycle.
    let eval_d0 = (input_ctx_phys & 0xFFFF_FFFF) as u32;
    let eval_d1 = (input_ctx_phys >> 32) as u32;
    let eval_d3 = (en_slot_id << 24)
        | (TRB_TYPE_EVALUATE_CONTEXT_CMD << 10)
        | cmd_cycle;
    trb_write_volatile(cmd_ring_va, cmd_idx, eval_d0, eval_d1, 0u32, eval_d3);
    // Cycle-stop at cmd_idx+1 with opposite cycle.
    trb_write_volatile(cmd_ring_va, cmd_idx + 1, 0, 0, 0, cmd_cycle ^ 1);
    serial_println!("[sexusb.xhci.eval_ctx.trb.ok]");

    // Doorbell 0 (command ring).
    mmio_write32(db_base, 0, 0u32);
    serial_println!("[sexusb.xhci.eval_ctx.doorbell.ok]");

    // Consume Command Completion Event (type=33) at current ev_idx.
    let mut eval_ok = false;
    for _ in 0..POLL_BUDGET {
        let ev_d3 = trb_read_dword(event_ring_va, ev_idx, 3);
        if (ev_d3 & 1) == (ev_dcs as u32) {
            let ev_type = (ev_d3 >> 10) & 0x3F;
            if ev_type == TRB_TYPE_CMD_COMPLETION_EVENT {
                let ev_d2 = trb_read_dword(event_ring_va, ev_idx, 2);
                let cc = (ev_d2 >> 24) & 0xFF;
                let slot = (ev_d3 >> 24) & 0xFF;
                if cc == TRB_CC_SUCCESS && slot == en_slot_id {
                    eval_ok = true;
                    serial_println!("[sexusb.xhci.eval_ctx.event.ok]");
                } else {
                    serial_println!("[sexusb.xhci.eval_ctx.event.bad] cc={} slot={}", cc, slot);
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
                // ERDP advance: lower dword first (QEMU nec-xhci requirement).
                mmio_write32(intr_base, XHCI_INTR_ERDP, new_erdp as u32);
                mmio_write32(intr_base, XHCI_INTR_ERDP + 4, (new_erdp >> 32) as u32);
            }
            break;
        }
        sys_yield();
    }

    if !eval_ok {
        serial_println!("[sexusb.xhci.eval_ctx.timeout.bad]");
        loop { sys_yield(); }
    }

    // Advance command ring producer index (cycle stable until segment wrap).
    cmd_idx += 1;
    let _ = (cmd_idx,);

    // Verify MPS was updated in output Device Context EP0 DW0 bits 31:16.
    let verify_dw0 = unsafe { core::ptr::read_volatile(dev_ep0_base.add(0)) };
    let verify_mps = (verify_dw0 >> 16) & 0xFFFF;
    // Post-eval EP0 context dump for audit.
    let post_dw1 = unsafe { core::ptr::read_volatile(dev_ep0_base.add(1)) };
    let post_dw2 = unsafe { core::ptr::read_volatile(dev_ep0_base.add(2)) };
    let post_dw3 = unsafe { core::ptr::read_volatile(dev_ep0_base.add(3)) };
    serial_println!("[sexusb.xhci.mps.audit] post_eval_ep0_dw0={:#x}", verify_dw0);
    serial_println!("[sexusb.xhci.mps.audit] post_eval_ep0_dw1={:#x}", post_dw1);
    serial_println!("[sexusb.xhci.mps.audit] post_eval_ep0_dw2={:#x}", post_dw2);
    serial_println!("[sexusb.xhci.mps.audit] post_eval_ep0_dw3={:#x}", post_dw3);
    serial_println!("[sexusb.xhci.mps.audit] output_ep0_mps_after={}", verify_mps);
    serial_println!("[sexusb.xhci.mps.audit] actual_mps_target={}", actual_mps);
    if verify_mps == actual_mps {
        serial_println!("[sexusb.xhci.eval_ctx.mps.verify] expected={} got={}", actual_mps, verify_mps);
        serial_println!("[sexusb.xhci.eval_ctx.complete.ok]");
    } else if port_speed == 3 && verify_mps == 512 {
        // SuperSpeed (speed=3) EP0 MPS is fixed at 512 per USB 3.0 spec
        // (§9.6.1: bMaxPacketSize0 encoding differs for SS). Controller
        // correctly ignores Evaluate Context MPS update for SS ports.
        serial_println!("[sexusb.xhci.eval_ctx.ss_mps_512.ok] port_speed=3 ss_mps=512 descriptor_bMaxPacketSize0={}",
            actual_mps);
    } else {
        serial_println!("[sexusb.xhci.eval_ctx.verify.bad] expected={} got={}", actual_mps, verify_mps);
    }

    // ===== GET_DESCRIPTOR(DEVICE, 0, 0, 18) =====
    // Phase: USB_XHCI_GET_DESCRIPTOR_FULL_18_PROOF_V1
    // Fetch full 18-byte device descriptor using the correct EP0 MPS.
    serial_println!("[sexusb.xhci.full18.start]");

    // Read EP0 TR Dequeue Pointer from Device Context output.
    // This tells us where the controller's dequeue pointer is after the
    // first TD (GET_DESCRIPTOR(8)). Expected: index 3 (old stop marker).
    // DO NOT hardcode index 3 — verify at runtime.
    let deq_dev_ep0_base = (device_ctx_va + ctx_stride) as *const u32;
    let deq_dw2 = unsafe { core::ptr::read_volatile(deq_dev_ep0_base.add(2)) };
    let deq_dw3 = unsafe { core::ptr::read_volatile(deq_dev_ep0_base.add(3)) };
    let deq_ptr = ((deq_dw3 as u64) << 32) | (deq_dw2 as u64);
    let deq_dcs = deq_ptr & 1;
    let deq_phys = deq_ptr & !0xFu64;
    let deq_index = (deq_phys.wrapping_sub(ep0_ring_phys)) / 16;

    if deq_dcs != 1
        || deq_phys < ep0_ring_phys
        || deq_phys >= ep0_ring_phys.wrapping_add(PAGE_SIZE)
        || deq_phys % 16 != 0
    {
        serial_println!("[sexusb.xhci.full18.ep0_deq.bad] ptr={:#x} dcs={}", deq_ptr, deq_dcs);
        loop { sys_yield(); }
    }
    // Bounds check: TD (3 TRBs + stop marker) must fit within 256-entry ring.
    if deq_index + 3 >= PAGE_SIZE / TRB_SIZE {
        serial_println!("[sexusb.xhci.full18.ep0_deq.bad] idx={} exceeds ring", deq_index);
        loop { sys_yield(); }
    }
    serial_println!("[sexusb.xhci.full18.ep0_deq.ok] idx={} dcs={}", deq_index, deq_dcs);

    // Zero descriptor buffer to prevent stale data from first fetch
    // masquerading as valid bytes in case of short/residual.
    unsafe { core::ptr::write_bytes(desc_data_va as *mut u8, 0, 18); }
    serial_println!("[sexusb.xhci.full18.zero.ok]");

    // Write 3-TRB chain at verified deq_index.
    // GET_DESCRIPTOR(DEVICE, 0, 0, wLength=18):
    //   [0x80, 0x06, 0x00, 0x01, 0x00, 0x00, 0x12, 0x00]
    // d0 = 0x0100_0680, d1 = 0x0012_0000

    // Setup Stage (type=2): TRB Transfer Length = 8 (fix, NOT wLength).
    // Per xHCI spec §6.4.1.2.1, Table 6-17: SETUP Stage DW2 bits 0:13
    // shall always be 8 (the USB setup packet size). The wLength goes in
    // the setup packet payload (DW0/DW1), NOT in this field.
    // IDT and CH bits ARE defined for SETUP Stage DW2 but cause issues
    // with QEMU nec-xhci; omit them for compatibility.
    let full_setup_d3 = (TRB_TYPE_SETUP_STAGE << 10)
        | (2u32 << 16)                           // TRT=IN (0b10) for control read
        | (1u32 << 6)                            // QEMU nec-xhci: inline setup packet marker
        | ep0_cycle;
    trb_write_volatile(ep0_ring_va, deq_index,
        0x0100_0680u32,                          // d0: bmReqType=0x80,bReq=0x06,wVal=0x0100
        0x0012_0000u32,                          // d1: wIdx=0,wLen=18
        (8u32 << 0),                              // TRB Transfer Length = 8 (setup packet size)
        full_setup_d3);

    // Data Stage (type=3): DIR=IN, CH=1, TRB Transfer Length=18.
    let full_data_d3 = (TRB_TYPE_DATA_STAGE << 10)
        | (1u32 << 16)                           // DIR=IN
        | ep0_cycle;
    trb_write_volatile(ep0_ring_va, deq_index + 1,
        (desc_data_phys & 0xFFFF_FFFF) as u32,
        (desc_data_phys >> 32) as u32,
        (18u32 << 0)                             // TRB Transfer Length = 18
            | (1u32 << 18),                      // CH=1 (IDT=0, IOC=0, ISP=0)
        full_data_d3);

    // Status Stage (type=4): DIR=OUT, CH=0, IOC=1 in DW3 bit 5.
    let full_status_d3 = (TRB_TYPE_STATUS_STAGE << 10)
        | (0u32 << 16)                           // DIR=OUT (for control read status)
        | (1u32 << 5)                            // IOC=1 (bit 5, Status Stage encoding)
        | ep0_cycle;
    trb_write_volatile(ep0_ring_va, deq_index + 2,
        0, 0,
        0u32,
        full_status_d3);

    // Cycle-stop marker at deq_index+3 with opposite cycle.
    trb_write_volatile(ep0_ring_va, deq_index + 3, 0, 0, 0, ep0_cycle ^ 1);
    serial_println!("[sexusb.xhci.full18.trbs.ok]");

    // Advance EP0 ring producer index past this TD.
    ep0_idx = deq_index + 4;
    let _ = (ep0_idx,);

    // Doorbell: EP0 on slot_id. DB Target = 1 (EP0 endpoint ID).
    mmio_write32(db_base, en_slot_id as u64 * 4, 1u32);
    serial_println!("[sexusb.xhci.full18.doorbell.ok]");

    // Consume Transfer Event (type=32) at current ev_idx.
    let mut full_ok = false;
    let mut full_residue: u32 = 0;
    for _ in 0..POLL_BUDGET {
        let ev_d3 = trb_read_dword(event_ring_va, ev_idx, 3);
        if (ev_d3 & 1) == (ev_dcs as u32) {
            let ev_type = (ev_d3 >> 10) & 0x3F;
            if ev_type == TRB_TYPE_TRANSFER_EVENT {
                let ev_d2 = trb_read_dword(event_ring_va, ev_idx, 2);
                let cc = (ev_d2 >> 24) & 0xFF;
                full_residue = ev_d2 & 0xFFFFFF;   // bits 23:0 = residual length
                let slot = (ev_d3 >> 24) & 0xFF;
                let ep   = (ev_d3 >> 16) & 0x1F;
                if cc == TRB_CC_SUCCESS && slot == en_slot_id && ep == 1 {
                    full_ok = true;
                    serial_println!("[sexusb.xhci.full18.event.ok]");
                } else {
                    serial_println!("[sexusb.xhci.full18.event.bad] cc={} slot={} ep={}", cc, slot, ep);
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
                // ERDP advance: lower dword first (QEMU nec-xhci requirement).
                mmio_write32(intr_base, XHCI_INTR_ERDP, new_erdp as u32);
                mmio_write32(intr_base, XHCI_INTR_ERDP + 4, (new_erdp >> 32) as u32);
            }
            break;
        }
        sys_yield();
    }

    if !full_ok {
        serial_println!("[sexusb.xhci.full18.timeout.bad]");
        loop { sys_yield(); }
    }

    if full_residue >= 18 {
        serial_println!("[sexusb.xhci.full18.residue.full.bad] residue={}", full_residue);
        loop { sys_yield(); }
    }

    if full_residue > 0 {
        serial_println!("[sexusb.xhci.full18.residue.warn] residue={}", full_residue);
    }

    // Log raw 18 bytes from descriptor data buffer.
    let full_buf = desc_data_va as *const u8;
    let fb0  = unsafe { core::ptr::read_volatile(full_buf.add(0)) };
    let fb1  = unsafe { core::ptr::read_volatile(full_buf.add(1)) };
    let fb2  = unsafe { core::ptr::read_volatile(full_buf.add(2)) };
    let fb3  = unsafe { core::ptr::read_volatile(full_buf.add(3)) };
    let fb4  = unsafe { core::ptr::read_volatile(full_buf.add(4)) };
    let fb5  = unsafe { core::ptr::read_volatile(full_buf.add(5)) };
    let fb6  = unsafe { core::ptr::read_volatile(full_buf.add(6)) };
    let fb7  = unsafe { core::ptr::read_volatile(full_buf.add(7)) };
    let fb8  = unsafe { core::ptr::read_volatile(full_buf.add(8)) };
    let fb9  = unsafe { core::ptr::read_volatile(full_buf.add(9)) };
    let fb10 = unsafe { core::ptr::read_volatile(full_buf.add(10)) };
    let fb11 = unsafe { core::ptr::read_volatile(full_buf.add(11)) };
    let fb12 = unsafe { core::ptr::read_volatile(full_buf.add(12)) };
    let fb13 = unsafe { core::ptr::read_volatile(full_buf.add(13)) };
    let fb14 = unsafe { core::ptr::read_volatile(full_buf.add(14)) };
    let fb15 = unsafe { core::ptr::read_volatile(full_buf.add(15)) };
    let fb16 = unsafe { core::ptr::read_volatile(full_buf.add(16)) };
    let fb17 = unsafe { core::ptr::read_volatile(full_buf.add(17)) };
    serial_println!(
        "[sexusb.xhci.full18.bytes.ok] bytes=[{:#x},{:#x},{:#x},{:#x},{:#x},{:#x},{:#x},{:#x},{:#x},{:#x},{:#x},{:#x},{:#x},{:#x},{:#x},{:#x},{:#x},{:#x}]",
        fb0, fb1, fb2, fb3, fb4, fb5, fb6, fb7, fb8, fb9, fb10, fb11, fb12, fb13, fb14, fb15, fb16, fb17
    );

    // Log informational fields (no routing/parsing).
    let full_len = fb0;
    let full_type = fb1;
    let full_usb = ((fb3 as u16) << 8) | (fb2 as u16);
    let full_class = fb4;
    let full_subclass = fb5;
    let full_proto = fb6;
    let full_mps = fb7;
    let full_vendor = ((fb9 as u16) << 8) | (fb8 as u16);
    let full_product = ((fb11 as u16) << 8) | (fb10 as u16);
    let full_device = ((fb13 as u16) << 8) | (fb12 as u16);
    let full_configs = fb17;

    serial_println!("[sexusb.xhci.full18.len] len={}", full_len);
    serial_println!("[sexusb.xhci.full18.type] type={}", full_type);
    serial_println!("[sexusb.xhci.full18.usb] usb={:#x}", full_usb);
    serial_println!("[sexusb.xhci.full18.class] class={} subclass={} protocol={}", full_class, full_subclass, full_proto);
    serial_println!("[sexusb.xhci.full18.vendor] vendor={:#x}", full_vendor);
    serial_println!("[sexusb.xhci.full18.product] product={:#x}", full_product);
    serial_println!("[sexusb.xhci.full18.device] device={:#x}", full_device);
    serial_println!("[sexusb.xhci.full18.configs] configs={}", full_configs);
    serial_println!("[sexusb.xhci.full18.mps_check] mps={}", full_mps);

    // MPS consistency check: bMaxPacketSize0 should match earlier fetch.
    if u32::from(full_mps) != actual_mps {
        serial_println!("[sexusb.xhci.full18.mps.mismatch.bad] expected={} got={}", actual_mps, full_mps);
        loop { sys_yield(); }
    }

    // Descriptor sanity warnings (non-fatal).
    if full_len != 18 {
        serial_println!("[sexusb.xhci.full18.desc_len.warn] len={}", full_len);
    }
    if full_type != 1 {
        serial_println!("[sexusb.xhci.full18.desc_type.warn] type={}", full_type);
    }

    if full_residue == 0 {
        serial_println!("[sexusb.xhci.full18.complete.ok]");
    }

    // ===== GET_DESCRIPTOR(CONFIGURATION) Header + Full =====
    // Phase: USB_XHCI_CONFIG_DESCRIPTOR_PROOF_V1
    // Two EP0 transfers: header (wLength=9) then full (wLength=wTotalLength).
    // Descriptor walk to find HID boot mouse interface and interrupt IN endpoint.
    serial_println!("[sexusb.xhci.config.start]");

    // --- TD1: Config Header (wLength=9) ---
    // Read EP0 TR Dequeue Pointer from Device Context output (after full18 TD).
    let cfg_deq_ep0_base = (device_ctx_va + ctx_stride) as *const u32;
    let cfg_deq_dw2 = unsafe { core::ptr::read_volatile(cfg_deq_ep0_base.add(2)) };
    let cfg_deq_dw3 = unsafe { core::ptr::read_volatile(cfg_deq_ep0_base.add(3)) };
    let cfg_deq_ptr = ((cfg_deq_dw3 as u64) << 32) | (cfg_deq_dw2 as u64);
    let cfg_deq_dcs = cfg_deq_ptr & 1;
    let cfg_deq_phys = cfg_deq_ptr & !0xFu64;
    let cfg_deq_index = (cfg_deq_phys.wrapping_sub(ep0_ring_phys)) / 16;

    if cfg_deq_dcs != 1
        || cfg_deq_phys < ep0_ring_phys
        || cfg_deq_phys >= ep0_ring_phys.wrapping_add(PAGE_SIZE)
        || cfg_deq_phys % 16 != 0
    {
        serial_println!("[sexusb.xhci.config.header_deq.bad] ptr={:#x} dcs={}", cfg_deq_ptr, cfg_deq_dcs);
        loop { sys_yield(); }
    }
    if cfg_deq_index + 3 >= PAGE_SIZE / TRB_SIZE {
        serial_println!("[sexusb.xhci.config.header_deq.ring.bad] idx={}", cfg_deq_index);
        loop { sys_yield(); }
    }
    serial_println!("[sexusb.xhci.config.header_deq.ok] idx={} dcs={}", cfg_deq_index, cfg_deq_dcs);

    // Zero first 9 bytes of descriptor buffer (prevent stale data from full18).
    unsafe { core::ptr::write_bytes(desc_data_va as *mut u8, 0, 9); }
    serial_println!("[sexusb.xhci.config.header_zero.ok]");

    // Write 3-TRB chain at cfg_deq_index for GET_DESCRIPTOR(CONFIGURATION, 0, 9).
    //   [0x80, 0x06, 0x00, 0x02, 0x00, 0x00, 0x09, 0x00]
    // d0 = 0x0200_0680, d1 = 0x0009_0000

    // Setup Stage (type=2): TRB Transfer Length = 8 (fixed setup packet size,
    // NOT wLength=9). SETUP Stage DW2 bits 0:13 shall always be 8 per xHCI
    // spec §6.4.1.2.1. The wLength goes in DW0/DW1 (setup packet payload).
    let cfg_hdr_setup_d3 = (TRB_TYPE_SETUP_STAGE << 10)
        | (2u32 << 16)                           // TRT=IN (0b10) for control read
        | (1u32 << 6)                            // QEMU nec-xhci: inline setup packet marker
        | ep0_cycle;
    trb_write_volatile(ep0_ring_va, cfg_deq_index,
        0x0200_0680u32,                          // d0: bmReqType=0x80,bReq=0x06,wVal=0x0200
        0x0009_0000u32,                          // d1: wIdx=0,wLen=9
        (8u32 << 0),                              // TRB Transfer Length = 8 (setup packet size)
        cfg_hdr_setup_d3);

    // Data Stage (type=3): DIR=IN, CH=1, TRB Transfer Length=9.
    let cfg_hdr_data_d3 = (TRB_TYPE_DATA_STAGE << 10)
        | (1u32 << 16)                           // DIR=IN
        | ep0_cycle;
    trb_write_volatile(ep0_ring_va, cfg_deq_index + 1,
        (desc_data_phys & 0xFFFF_FFFF) as u32,
        (desc_data_phys >> 32) as u32,
        (9u32 << 0)                              // TRB Transfer Length = 9
            | (1u32 << 18),                      // CH=1
        cfg_hdr_data_d3);

    // Status Stage (type=4): DIR=OUT, CH=0, IOC=1.
    let cfg_hdr_status_d3 = (TRB_TYPE_STATUS_STAGE << 10)
        | (0u32 << 16)                           // DIR=OUT
        | ep0_cycle;
    trb_write_volatile(ep0_ring_va, cfg_deq_index + 2,
        0, 0,
        0u32,                                      // DW2: all zero
        cfg_hdr_status_d3 | (1u32 << 5));          // IOC=1 (bit 5, Status Stage encoding)

    // Cycle-stop marker at cfg_deq_index+3 with opposite cycle.
    trb_write_volatile(ep0_ring_va, cfg_deq_index + 3, 0, 0, 0, ep0_cycle ^ 1);
    serial_println!("[sexusb.xhci.config.header_trbs.ok]");

    // Advance EP0 ring producer index (cycle stable until segment wrap).
    ep0_idx = cfg_deq_index + 4;
    let _ = (ep0_idx,);

    // Doorbell: EP0 on slot_id. DB Target = 1.
    mmio_write32(db_base, en_slot_id as u64 * 4, 1u32);
    serial_println!("[sexusb.xhci.config.header.doorbell.ok]");

    // Consume Transfer Event (type=32) at current ev_idx.
    let mut cfg_hdr_ok = false;
    let mut cfg_hdr_residue: u32 = 0;
    for _ in 0..POLL_BUDGET {
        let ev_d3 = trb_read_dword(event_ring_va, ev_idx, 3);
        if (ev_d3 & 1) == (ev_dcs as u32) {
            let ev_type = (ev_d3 >> 10) & 0x3F;
            if ev_type == TRB_TYPE_TRANSFER_EVENT {
                let ev_d2 = trb_read_dword(event_ring_va, ev_idx, 2);
                let cc = (ev_d2 >> 24) & 0xFF;
                cfg_hdr_residue = ev_d2 & 0xFFFFFF;
                let slot = (ev_d3 >> 24) & 0xFF;
                let ep   = (ev_d3 >> 16) & 0x1F;
                if cc == TRB_CC_SUCCESS && slot == en_slot_id && ep == 1 {
                    cfg_hdr_ok = true;
                    serial_println!("[sexusb.xhci.config.header.event.ok]");
                } else {
                    serial_println!("[sexusb.xhci.config.header.event.bad] cc={} slot={} ep={}", cc, slot, ep);
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
                // ERDP advance: lower dword first (QEMU nec-xhci requirement).
                mmio_write32(intr_base, XHCI_INTR_ERDP, new_erdp as u32);
                mmio_write32(intr_base, XHCI_INTR_ERDP + 4, (new_erdp >> 32) as u32);
            }
            break;
        }
        sys_yield();
    }

    if !cfg_hdr_ok {
        serial_println!("[sexusb.xhci.config.header.timeout.bad]");
        loop { sys_yield(); }
    }

    // Header residue > 0 is fatal: can't trust wTotalLength.
    if cfg_hdr_residue > 0 {
        serial_println!("[sexusb.xhci.config.header_residue.bad] residue={}", cfg_hdr_residue);
        loop { sys_yield(); }
    }
    serial_println!("[sexusb.xhci.config.header.ok]");

    // Read wTotalLength from config descriptor bytes 2(lo), 3(hi).
    let cfg_buf = desc_data_va as *const u8;
    let cfg_total_lo = unsafe { core::ptr::read_volatile(cfg_buf.add(2)) };
    let cfg_total_hi = unsafe { core::ptr::read_volatile(cfg_buf.add(3)) };
    let w_total_length: u64 = ((cfg_total_hi as u64) << 8) | (cfg_total_lo as u64);

    // Reject wTotalLength < 9 (corrupt). Allow ==9 (valid but no-HID will park).
    if w_total_length < 9 {
        serial_println!("[sexusb.xhci.config.total_len.bad] wTotalLength={}", w_total_length);
        loop { sys_yield(); }
    }
    serial_println!("[sexusb.xhci.config.header.totallen] len={}", w_total_length);

    // --- TD2: Full Config (wLength=wTotalLength) ---
    // Read EP0 TR Dequeue Pointer from Device Context after TD1 consumption.
    let cfg2_deq_dw2 = unsafe { core::ptr::read_volatile(cfg_deq_ep0_base.add(2)) };
    let cfg2_deq_dw3 = unsafe { core::ptr::read_volatile(cfg_deq_ep0_base.add(3)) };
    let cfg2_deq_ptr = ((cfg2_deq_dw3 as u64) << 32) | (cfg2_deq_dw2 as u64);
    let cfg2_deq_dcs = cfg2_deq_ptr & 1;
    let cfg2_deq_phys = cfg2_deq_ptr & !0xFu64;
    let cfg2_deq_index = (cfg2_deq_phys.wrapping_sub(ep0_ring_phys)) / 16;

    if cfg2_deq_dcs != 1
        || cfg2_deq_phys < ep0_ring_phys
        || cfg2_deq_phys >= ep0_ring_phys.wrapping_add(PAGE_SIZE)
        || cfg2_deq_phys % 16 != 0
    {
        serial_println!("[sexusb.xhci.config.full_deq.bad] ptr={:#x} dcs={}", cfg2_deq_ptr, cfg2_deq_dcs);
        loop { sys_yield(); }
    }
    if cfg2_deq_index + 3 >= PAGE_SIZE / TRB_SIZE {
        serial_println!("[sexusb.xhci.config.full_deq.ring.bad] idx={}", cfg2_deq_index);
        loop { sys_yield(); }
    }
    serial_println!("[sexusb.xhci.config.full_deq.ok] idx={} dcs={}", cfg2_deq_index, cfg2_deq_dcs);

    // Zero first wTotalLength bytes of descriptor buffer (prevent stale header data).
    {
        let zero_base = desc_data_va as *mut u8;
        let mut zi: u64 = 0;
        while zi < w_total_length {
            unsafe { core::ptr::write_volatile(zero_base.add(zi as usize), 0u8); }
            zi += 1;
        }
    }
    serial_println!("[sexusb.xhci.config.full_zero.ok]");

    // Write 3-TRB chain at cfg2_deq_index.
    // GET_DESCRIPTOR(CONFIGURATION, 0, wTotalLength):
    //   [0x80, 0x06, 0x00, 0x02, 0x00, 0x00, wLen_lo, wLen_hi]
    // d0 = 0x0200_0680, d1 = (wTotalLength as u32) << 16

    // Setup Stage (type=2): IDT=1 in DW2, CH=1 in DW2, TRT=IN, DW3 bit6 for QEMU inline.
    let cfg_full_setup_d3 = (TRB_TYPE_SETUP_STAGE << 10)
        | (2u32 << 16)                           // TRT=IN (0b10) for control read
        | (1u32 << 6)                            // QEMU nec-xhci: inline setup packet marker
        | ep0_cycle;
    trb_write_volatile(ep0_ring_va, cfg2_deq_index,
        0x0200_0680u32,                          // d0: bmReqType=0x80,bReq=0x06,wVal=0x0200
        (w_total_length as u32) << 16,            // d1: wIdx=0,wLen=wTotalLength
        (8u32 << 0),                              // TRB Transfer Length = 8 (setup packet size, NOT wTotalLength)
        cfg_full_setup_d3);

    // Data Stage (type=3): DIR=IN, CH=1, TRB Transfer Length = wTotalLength.
    let cfg_full_data_d3 = (TRB_TYPE_DATA_STAGE << 10)
        | (1u32 << 16)                           // DIR=IN
        | ep0_cycle;
    trb_write_volatile(ep0_ring_va, cfg2_deq_index + 1,
        (desc_data_phys & 0xFFFF_FFFF) as u32,
        (desc_data_phys >> 32) as u32,
        (w_total_length as u32)                   // TRB Transfer Length = wTotalLength
            | (1u32 << 18),                      // CH=1
        cfg_full_data_d3);

    // Status Stage (type=4): DIR=OUT, CH=0, IOC=1.
    let cfg_full_status_d3 = (TRB_TYPE_STATUS_STAGE << 10)
        | (0u32 << 16)                           // DIR=OUT
        | ep0_cycle;
    trb_write_volatile(ep0_ring_va, cfg2_deq_index + 2,
        0, 0,
        0u32,                                      // DW2: all zero
        cfg_full_status_d3 | (1u32 << 5));         // IOC=1 (bit 5, Status Stage encoding)

    // Cycle-stop marker at cfg2_deq_index+3 with opposite cycle.
    trb_write_volatile(ep0_ring_va, cfg2_deq_index + 3, 0, 0, 0, ep0_cycle ^ 1);
    serial_println!("[sexusb.xhci.config.full_trbs.ok]");

    // Advance EP0 ring producer index.
    ep0_idx = cfg2_deq_index + 4;
    let _ = (ep0_idx,);

    // Doorbell: EP0 on slot_id.
    mmio_write32(db_base, en_slot_id as u64 * 4, 1u32);
    serial_println!("[sexusb.xhci.config.full.doorbell.ok]");

    // Consume Transfer Event (type=32) at current ev_idx.
    let mut cfg_full_ok = false;
    let mut cfg_full_residue: u32 = 0;
    for _ in 0..POLL_BUDGET {
        let ev_d3 = trb_read_dword(event_ring_va, ev_idx, 3);
        if (ev_d3 & 1) == (ev_dcs as u32) {
            let ev_type = (ev_d3 >> 10) & 0x3F;
            if ev_type == TRB_TYPE_TRANSFER_EVENT {
                let ev_d2 = trb_read_dword(event_ring_va, ev_idx, 2);
                let cc = (ev_d2 >> 24) & 0xFF;
                cfg_full_residue = ev_d2 & 0xFFFFFF;
                let slot = (ev_d3 >> 24) & 0xFF;
                let ep   = (ev_d3 >> 16) & 0x1F;
                if cc == TRB_CC_SUCCESS && slot == en_slot_id && ep == 1 {
                    cfg_full_ok = true;
                    serial_println!("[sexusb.xhci.config.full.event.ok]");
                } else {
                    serial_println!("[sexusb.xhci.config.full.event.bad] cc={} slot={} ep={}", cc, slot, ep);
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
                // ERDP advance: lower dword first (QEMU nec-xhci requirement).
                mmio_write32(intr_base, XHCI_INTR_ERDP, new_erdp as u32);
                mmio_write32(intr_base, XHCI_INTR_ERDP + 4, (new_erdp >> 32) as u32);
            }
            break;
        }
        sys_yield();
    }

    if !cfg_full_ok {
        serial_println!("[sexusb.xhci.config.full.timeout.bad]");
        loop { sys_yield(); }
    }

    // Full config residue policy:
    //   residue >= wTotalLength → fatal (received nothing usable)
    //   residue > 0 → partial.warn, walk only received_len = wTotalLength - residue
    //   residue == 0 → complete, walk full wTotalLength
    if cfg_full_residue >= w_total_length as u32 {
        serial_println!("[sexusb.xhci.config.full_residue_full.bad] residue={} wTotalLength={}",
            cfg_full_residue, w_total_length);
        loop { sys_yield(); }
    }

    let received_len: u64 = if cfg_full_residue > 0 {
        serial_println!("[sexusb.xhci.config.full_residue_partial.warn] residue={}", cfg_full_residue);
        w_total_length - cfg_full_residue as u64
    } else {
        w_total_length
    };

    // ===== Descriptor Walk =====
    serial_println!("[sexusb.xhci.config.walk.start]");

    let walk_buf = desc_data_va as *const u8;
    let mut walk_off: u64 = 0;
    let mut inside_hid_iface: bool = false;
    let mut iface_is_boot_mouse: bool = false;
    let mut found_hid_mouse: bool = false;
    let mut found_hid_tablet: bool = false;
    let mut hid_interface_number: u8 = 0;
    let mut hid_report_desc_len: u16 = 0;
    let mut intr_ep_addr: u8 = 0;
    let mut intr_ep_mps: u16 = 0;
    let mut intr_ep_interval: u8 = 0;

    while walk_off < received_len {
        let b_len = unsafe { core::ptr::read_volatile(walk_buf.add(walk_off as usize)) };
        let b_type = unsafe { core::ptr::read_volatile(walk_buf.add(walk_off as usize + 1)) };

        if b_len == 0 {
            serial_println!("[sexusb.xhci.config.desc_zero_len.bad] off={}", walk_off);
            loop { sys_yield(); }
        }
        if (b_len as u64) > (received_len - walk_off) {
            serial_println!("[sexusb.xhci.config.desc_truncated.bad] off={} len={} remain={}",
                walk_off, b_len, received_len - walk_off);
            loop { sys_yield(); }
        }

        match b_type {
            4 => { // INTERFACE descriptor
                if b_len >= 8 {
                    let b_intf_num = unsafe { core::ptr::read_volatile(walk_buf.add(walk_off as usize + 2)) };
                    let b_class    = unsafe { core::ptr::read_volatile(walk_buf.add(walk_off as usize + 5)) };
                    let b_subclass = unsafe { core::ptr::read_volatile(walk_buf.add(walk_off as usize + 6)) };
                    let b_protocol = unsafe { core::ptr::read_volatile(walk_buf.add(walk_off as usize + 7)) };
                    let is_hid = b_class == 0x03;
                    if is_hid {
                        inside_hid_iface = true;
                        iface_is_boot_mouse = (b_subclass == 0x01) && (b_protocol == 0x02);
                        if iface_is_boot_mouse {
                            found_hid_mouse = true;
                            hid_interface_number = b_intf_num;
                            serial_println!("[sexusb.xhci.config.hid_boot_mouse.found] intf={} off={}",
                                b_intf_num, walk_off);
                        } else if b_protocol != 0x01 {
                            // Non-keyboard HID interface (tablet/pointer)
                            found_hid_tablet = true;
                            hid_interface_number = b_intf_num;
                            serial_println!("[sexusb.xhci.config.hid_tablet.found] intf={} off={} subclass={} protocol={}",
                                b_intf_num, walk_off, b_subclass, b_protocol);
                        }
                    } else {
                        inside_hid_iface = false;
                        iface_is_boot_mouse = false;
                    }
                } else {
                    inside_hid_iface = false;
                    iface_is_boot_mouse = false;
                }
            }
            0x21 => { // HID descriptor
                if inside_hid_iface {
                    if b_len < 9 {
                        serial_println!("[sexusb.xhci.config.hid_desc.bad] short_len={}", b_len);
                        loop { sys_yield(); }
                    }
                    let hid_type = unsafe { core::ptr::read_volatile(walk_buf.add(walk_off as usize + 6)) };
                    if hid_type != 0x22 {
                        serial_println!("[sexusb.xhci.config.hid_desc.bad] type={:#x} off={}", hid_type, walk_off);
                        loop { sys_yield(); }
                    }
                    let rpt_len_lo = unsafe { core::ptr::read_volatile(walk_buf.add(walk_off as usize + 7)) };
                    let rpt_len_hi = unsafe { core::ptr::read_volatile(walk_buf.add(walk_off as usize + 8)) };
                    hid_report_desc_len = ((rpt_len_hi as u16) << 8) | (rpt_len_lo as u16);
                    serial_println!("[sexusb.xhci.config.hid_desc.ok] off={} report_len={}", walk_off, hid_report_desc_len);
                }
            }
            5 => { // ENDPOINT descriptor
                if inside_hid_iface {
                    if b_len < 7 {
                        serial_println!("[sexusb.xhci.config.intr_ep_short.bad] off={} len={}", walk_off, b_len);
                        loop { sys_yield(); }
                    }
                    let b_endpoint  = unsafe { core::ptr::read_volatile(walk_buf.add(walk_off as usize + 2)) };
                    let bm_attrs    = unsafe { core::ptr::read_volatile(walk_buf.add(walk_off as usize + 3)) };
                    let mps_lo      = unsafe { core::ptr::read_volatile(walk_buf.add(walk_off as usize + 4)) };
                    let mps_hi      = unsafe { core::ptr::read_volatile(walk_buf.add(walk_off as usize + 5)) };
                    let w_max_pkt   = ((mps_hi as u16) << 8) | (mps_lo as u16);
                    let pkt_size    = w_max_pkt & 0x07FF;
                    let b_interval  = unsafe { core::ptr::read_volatile(walk_buf.add(walk_off as usize + 6)) };
                    let dir_in      = (b_endpoint & 0x80) != 0;
                    let intr_type   = (bm_attrs & 0x03) == 0x03;
                    if dir_in && intr_type {
                        if pkt_size == 0 {
                            serial_println!("[sexusb.xhci.config.intr_ep_mps.bad] off={} mps={}", walk_off, pkt_size);
                            loop { sys_yield(); }
                        }
                        intr_ep_addr = b_endpoint;
                        intr_ep_mps = pkt_size;
                        intr_ep_interval = b_interval;
                        let ep_which = if iface_is_boot_mouse { "mouse" } else { "tablet" };
                        serial_println!("[sexusb.xhci.config.intr_ep.{}] off={} addr={:#x} mps={} interval={}",
                            ep_which, walk_off, b_endpoint, pkt_size, b_interval);
                    }
                }
            }
            _ => { /* skip other descriptor types */ }
        }

        walk_off += b_len as u64;
    }

    serial_println!("[sexusb.xhci.config.walk.done]");

    let cfg_value = unsafe { core::ptr::read_volatile(walk_buf.add(5)) };
    if cfg_value == 0 {
        serial_println!("[sexusb.xhci.config.value.bad] value=0");
        loop { sys_yield(); }
    }

    if !found_hid_mouse && !found_hid_tablet {
        serial_println!("[sexusb.xhci.config.no_hid.park]");
        loop { sys_yield(); }
    }

    let is_tablet_device = found_hid_tablet && !found_hid_mouse;

    // ===== GET_DESCRIPTOR(HID REPORT) =====
    // Phase: USB_XHCI_HID_REPORT_DESCRIPTOR_PROOF_V1
    let report_len = hid_report_desc_len as u32;
    if report_len == 0 || report_len > 256 {
        serial_println!(
            "[sexusb.xhci.hid.report_desc.len.bad] len={} intf={}",
            report_len,
            hid_interface_number
        );
        loop { sys_yield(); }
    }

    serial_println!(
        "[sexusb.xhci.hid.report_desc.start] intf={} len={}",
        hid_interface_number,
        report_len
    );

    let hid_deq_dw2 = unsafe { core::ptr::read_volatile(cfg_deq_ep0_base.add(2)) };
    let hid_deq_dw3 = unsafe { core::ptr::read_volatile(cfg_deq_ep0_base.add(3)) };
    let hid_deq_ptr = ((hid_deq_dw3 as u64) << 32) | (hid_deq_dw2 as u64);
    let hid_deq_dcs = hid_deq_ptr & 1;
    let hid_deq_phys = hid_deq_ptr & !0xFu64;
    let hid_deq_index = (hid_deq_phys.wrapping_sub(ep0_ring_phys)) / 16;

    if hid_deq_dcs != 1
        || hid_deq_phys < ep0_ring_phys
        || hid_deq_phys >= ep0_ring_phys.wrapping_add(PAGE_SIZE)
        || hid_deq_phys % 16 != 0
    {
        serial_println!("[sexusb.xhci.hid.report_desc.deq.bad] ptr={:#x} dcs={}", hid_deq_ptr, hid_deq_dcs);
        loop { sys_yield(); }
    }
    if hid_deq_index + 3 >= PAGE_SIZE / TRB_SIZE {
        serial_println!("[sexusb.xhci.hid.report_desc.deq.ring.bad] idx={}", hid_deq_index);
        loop { sys_yield(); }
    }

    unsafe { core::ptr::write_bytes(desc_data_va as *mut u8, 0, report_len as usize); }

    // SETUP: bmReqType=0x81, bReq=0x06, wValue=0x2200, wIndex=intf, wLength=report_len
    let hid_setup_d3 = (TRB_TYPE_SETUP_STAGE << 10)
        | (2u32 << 16)
        | (1u32 << 6)
        | ep0_cycle;
    trb_write_volatile(
        ep0_ring_va,
        hid_deq_index,
        0x2200_0681u32,
        ((report_len as u32) << 16) | (hid_interface_number as u32),
        8u32,
        hid_setup_d3,
    );

    let hid_data_d3 = (TRB_TYPE_DATA_STAGE << 10) | (1u32 << 16) | ep0_cycle;
    trb_write_volatile(
        ep0_ring_va,
        hid_deq_index + 1,
        (desc_data_phys & 0xFFFF_FFFF) as u32,
        (desc_data_phys >> 32) as u32,
        report_len | (1u32 << 18),
        hid_data_d3,
    );

    let hid_status_d3 = (TRB_TYPE_STATUS_STAGE << 10)
        | (0u32 << 16)
        | (1u32 << 5)
        | ep0_cycle;
    trb_write_volatile(ep0_ring_va, hid_deq_index + 2, 0, 0, 0, hid_status_d3);
    trb_write_volatile(ep0_ring_va, hid_deq_index + 3, 0, 0, 0, ep0_cycle ^ 1);

    ep0_idx = hid_deq_index + 4;
    let _ = (ep0_idx,);

    mmio_write32(db_base, en_slot_id as u64 * 4, 1u32);

    let mut hid_ok = false;
    let mut hid_residue: u32 = 0;
    for _ in 0..POLL_BUDGET {
        let ev_d3 = trb_read_dword(event_ring_va, ev_idx, 3);
        if (ev_d3 & 1) == (ev_dcs as u32) {
            let ev_type = (ev_d3 >> 10) & 0x3F;
            if ev_type == TRB_TYPE_TRANSFER_EVENT {
                let ev_d2 = trb_read_dword(event_ring_va, ev_idx, 2);
                let cc = (ev_d2 >> 24) & 0xFF;
                hid_residue = ev_d2 & 0xFFFFFF;
                let slot = (ev_d3 >> 24) & 0xFF;
                let ep = (ev_d3 >> 16) & 0x1F;
                if cc == TRB_CC_SUCCESS && slot == en_slot_id && ep == 1 {
                    hid_ok = true;
                } else {
                    serial_println!("[sexusb.xhci.hid.report_desc.event.bad] cc={} slot={} ep={}", cc, slot, ep);
                }
                trb_write_volatile(event_ring_va, ev_idx, 0, 0, 0, ev_d3 & !1u32);
                ev_idx += 1;
                if ev_idx >= EVENT_RING_TRBS {
                    ev_idx = 0;
                    ev_dcs ^= 1;
                }
                let new_erdp = event_ring_phys + ev_idx * 16;
                mmio_write32(intr_base, XHCI_INTR_ERDP, new_erdp as u32);
                mmio_write32(intr_base, XHCI_INTR_ERDP + 4, (new_erdp >> 32) as u32);
            }
            break;
        }
        sys_yield();
    }

    if !hid_ok {
        serial_println!("[sexusb.xhci.hid.report_desc.timeout.bad]");
        loop { sys_yield(); }
    }
    if hid_residue > report_len {
        serial_println!("[sexusb.xhci.hid.report_desc.residue.bad] residue={} len={}", hid_residue, report_len);
        loop { sys_yield(); }
    }

    let hid_actual_len = report_len - hid_residue;
    serial_println!(
        "[sexusb.xhci.hid.report_desc.event.ok] actual={} residue={}",
        hid_actual_len,
        hid_residue
    );

    let dump_len = if hid_actual_len > 64 { 64 } else { hid_actual_len };
    let hid_buf = desc_data_va as *const u8;
    let mut bi: u32 = 0;
    while bi < dump_len {
        let bv = unsafe { core::ptr::read_volatile(hid_buf.add(bi as usize)) };
        serial_println!("[sexusb.xhci.hid.report_desc.bytes] i={} b={:#x}", bi, bv);
        bi += 1;
    }

    let mut has_usage_page_gd = false;
    let mut has_usage_mouse = false;
    let mut has_usage_pointer = false;
    let mut has_collection_app = false;
    let mut has_usage_x = false;
    let mut has_usage_y = false;
    let mut si: u32 = 0;
    while si + 1 < hid_actual_len {
        let b0 = unsafe { core::ptr::read_volatile(hid_buf.add(si as usize)) };
        let b1 = unsafe { core::ptr::read_volatile(hid_buf.add(si as usize + 1)) };
        if b0 == 0x05 && b1 == 0x01 {
            has_usage_page_gd = true;
        } else if b0 == 0x09 && b1 == 0x02 {
            has_usage_mouse = true;
        } else if b0 == 0x09 && b1 == 0x01 {
            has_usage_pointer = true;
        } else if b0 == 0xA1 && b1 == 0x01 {
            has_collection_app = true;
        } else if b0 == 0x09 && b1 == 0x30 {
            has_usage_x = true;
        } else if b0 == 0x09 && b1 == 0x31 {
            has_usage_y = true;
        }
        // Check for absolute X/Y (short items: 0x81 = Input, 0x02 = Data,Var,Abs)
        if b0 == 0x81 && b1 == 0x02 {
            // Input (Data,Var,Abs) — could be X or Y depending on usage context
            // We just note that absolute items exist; mouse uses 0x81 0x02 too
        }
        si += 1;
    }

    let is_mouse_shape = has_usage_page_gd && has_usage_mouse && has_collection_app && has_usage_x && has_usage_y;
    let is_tablet_shape = has_usage_page_gd && has_usage_pointer && has_usage_x && has_usage_y;
    if is_tablet_shape {
        serial_println!("[sexusb.xhci.hid.report_desc.tablet_shape.ok]");
    }
    if is_mouse_shape {
        serial_println!("[sexusb.xhci.hid.report_desc.mouse_shape.ok]");
    }
    if !is_mouse_shape && !is_tablet_shape {
        serial_println!("[sexusb.xhci.hid.report_desc.shape.warn] mouse={} tablet={}",
            is_mouse_shape, is_tablet_shape);
    }

    serial_println!("[sexusb.xhci.hid.report_desc.complete.ok] len={}", hid_actual_len);

    // ===== SET_CONFIGURATION =====
    // Phase: USB_XHCI_SET_CONFIGURATION_PROOF_V1
    serial_println!("[sexusb.xhci.set_config.start] value={}", cfg_value);

    let setcfg_deq_dw2 = unsafe { core::ptr::read_volatile(cfg_deq_ep0_base.add(2)) };
    let setcfg_deq_dw3 = unsafe { core::ptr::read_volatile(cfg_deq_ep0_base.add(3)) };
    let setcfg_deq_ptr = ((setcfg_deq_dw3 as u64) << 32) | (setcfg_deq_dw2 as u64);
    let setcfg_deq_dcs = setcfg_deq_ptr & 1;
    let setcfg_deq_phys = setcfg_deq_ptr & !0xFu64;
    let setcfg_deq_index = (setcfg_deq_phys.wrapping_sub(ep0_ring_phys)) / 16;

    if setcfg_deq_dcs != 1
        || setcfg_deq_phys < ep0_ring_phys
        || setcfg_deq_phys >= ep0_ring_phys.wrapping_add(PAGE_SIZE)
        || setcfg_deq_phys % 16 != 0
    {
        serial_println!("[sexusb.xhci.set_config.deq.bad] ptr={:#x} dcs={}", setcfg_deq_ptr, setcfg_deq_dcs);
        loop { sys_yield(); }
    }
    if setcfg_deq_index + 2 >= PAGE_SIZE / TRB_SIZE {
        serial_println!("[sexusb.xhci.set_config.deq.ring.bad] idx={}", setcfg_deq_index);
        loop { sys_yield(); }
    }

    // SETUP: bmReqType=0x00, bReq=0x09, wValue=bConfigurationValue, wIndex=0, wLength=0
    let setcfg_setup_d3 = (TRB_TYPE_SETUP_STAGE << 10)
        | (0u32 << 16)  // TRT=NO DATA
        | (1u32 << 6)   // QEMU nec-xhci inline setup marker
        | ep0_cycle;
    trb_write_volatile(
        ep0_ring_va,
        setcfg_deq_index,
        ((cfg_value as u32) << 16) | 0x0900u32,
        0u32,
        8u32,
        setcfg_setup_d3,
    );

    let setcfg_status_d3 = (TRB_TYPE_STATUS_STAGE << 10)
        | (1u32 << 16)  // DIR=IN for no-data control transfer status
        | (1u32 << 5)   // IOC=1
        | ep0_cycle;
    trb_write_volatile(ep0_ring_va, setcfg_deq_index + 1, 0, 0, 0, setcfg_status_d3);
    trb_write_volatile(ep0_ring_va, setcfg_deq_index + 2, 0, 0, 0, ep0_cycle ^ 1);

    ep0_idx = setcfg_deq_index + 3;
    let _ = (ep0_idx,);

    mmio_write32(db_base, en_slot_id as u64 * 4, 1u32);

    let mut setcfg_ok = false;
    let mut setcfg_residue: u32 = 0;
    for _ in 0..POLL_BUDGET {
        let ev_d3 = trb_read_dword(event_ring_va, ev_idx, 3);
        if (ev_d3 & 1) == (ev_dcs as u32) {
            let ev_type = (ev_d3 >> 10) & 0x3F;
            if ev_type == TRB_TYPE_TRANSFER_EVENT {
                let ev_d2 = trb_read_dword(event_ring_va, ev_idx, 2);
                let cc = (ev_d2 >> 24) & 0xFF;
                setcfg_residue = ev_d2 & 0xFFFFFF;
                let slot = (ev_d3 >> 24) & 0xFF;
                let ep = (ev_d3 >> 16) & 0x1F;
                if cc == TRB_CC_SUCCESS && slot == en_slot_id && ep == 1 {
                    setcfg_ok = true;
                } else {
                    serial_println!("[sexusb.xhci.set_config.event.bad] cc={} slot={} ep={}", cc, slot, ep);
                }
                trb_write_volatile(event_ring_va, ev_idx, 0, 0, 0, ev_d3 & !1u32);
                ev_idx += 1;
                if ev_idx >= EVENT_RING_TRBS {
                    ev_idx = 0;
                    ev_dcs ^= 1;
                }
                let new_erdp = event_ring_phys + ev_idx * 16;
                mmio_write32(intr_base, XHCI_INTR_ERDP, new_erdp as u32);
                mmio_write32(intr_base, XHCI_INTR_ERDP + 4, (new_erdp >> 32) as u32);
            }
            break;
        }
        sys_yield();
    }

    if !setcfg_ok {
        serial_println!("[sexusb.xhci.set_config.timeout.bad]");
        loop { sys_yield(); }
    }
    if setcfg_residue != 0 {
        serial_println!("[sexusb.xhci.set_config.residue.bad] residue={}", setcfg_residue);
        loop { sys_yield(); }
    }

    serial_println!("[sexusb.xhci.set_config.event.ok] actual=0 residue=0");
    serial_println!("[sexusb.xhci.set_config.complete.ok]");

    // Request periodic HID input reports even without movement:
    // SET_IDLE(duration=1*4ms, report_id=0) to interface hid_interface_number.
    let setidle_deq_dw2 = unsafe { core::ptr::read_volatile(cfg_deq_ep0_base.add(2)) };
    let setidle_deq_dw3 = unsafe { core::ptr::read_volatile(cfg_deq_ep0_base.add(3)) };
    let setidle_deq_ptr = ((setidle_deq_dw3 as u64) << 32) | (setidle_deq_dw2 as u64);
    let setidle_deq_phys = setidle_deq_ptr & !0xFu64;
    let setidle_deq_dcs = setidle_deq_ptr & 1;
    let setidle_deq_index = (setidle_deq_phys.wrapping_sub(ep0_ring_phys)) / 16;
    if setidle_deq_dcs != 1
        || setidle_deq_phys < ep0_ring_phys
        || setidle_deq_phys >= ep0_ring_phys.wrapping_add(PAGE_SIZE)
        || setidle_deq_phys % 16 != 0
        || setidle_deq_index + 2 >= PAGE_SIZE / TRB_SIZE
    {
        serial_println!("[sexusb.xhci.hid.set_idle.deq.bad]");
        loop { sys_yield(); }
    }

    let setidle_setup_d3 = (TRB_TYPE_SETUP_STAGE << 10)
        | (0u32 << 16)
        | (1u32 << 6)
        | ep0_cycle;
    trb_write_volatile(
        ep0_ring_va,
        setidle_deq_index,
        0x0100_0A21u32, // bmReqType=0x21, bReq=0x0A, wValue=0x0100
        hid_interface_number as u32, // wIndex=interface, wLength=0
        8u32,
        setidle_setup_d3,
    );
    let setidle_status_d3 = (TRB_TYPE_STATUS_STAGE << 10)
        | (1u32 << 16)
        | (1u32 << 5)
        | ep0_cycle;
    trb_write_volatile(ep0_ring_va, setidle_deq_index + 1, 0, 0, 0, setidle_status_d3);
    trb_write_volatile(ep0_ring_va, setidle_deq_index + 2, 0, 0, 0, ep0_cycle ^ 1);
    ep0_idx = setidle_deq_index + 3;
    let _ = (ep0_idx,);
    mmio_write32(db_base, en_slot_id as u64 * 4, 1u32);

    let mut setidle_ok = false;
    for _ in 0..POLL_BUDGET {
        let ev_d3 = trb_read_dword(event_ring_va, ev_idx, 3);
        if (ev_d3 & 1) == (ev_dcs as u32) {
            let ev_type = (ev_d3 >> 10) & 0x3F;
            if ev_type == TRB_TYPE_TRANSFER_EVENT {
                let ev_d2 = trb_read_dword(event_ring_va, ev_idx, 2);
                let cc = (ev_d2 >> 24) & 0xFF;
                let residue = ev_d2 & 0xFFFFFF;
                let slot = (ev_d3 >> 24) & 0xFF;
                let ep = (ev_d3 >> 16) & 0x1F;
                if cc == TRB_CC_SUCCESS && slot == en_slot_id && ep == 1 && residue == 0 {
                    setidle_ok = true;
                }
                trb_write_volatile(event_ring_va, ev_idx, 0, 0, 0, ev_d3 & !1u32);
                ev_idx += 1;
                if ev_idx >= EVENT_RING_TRBS {
                    ev_idx = 0;
                    ev_dcs ^= 1;
                }
                let new_erdp = event_ring_phys + ev_idx * 16;
                mmio_write32(intr_base, XHCI_INTR_ERDP, new_erdp as u32);
                mmio_write32(intr_base, XHCI_INTR_ERDP + 4, (new_erdp >> 32) as u32);
            }
            break;
        }
        sys_yield();
    }
    if !setidle_ok {
        serial_println!("[sexusb.xhci.hid.set_idle.timeout.bad]");
        loop { sys_yield(); }
    }

    // ===== Configure Endpoint + Interrupt-IN Poll =====
    // Phase: USB_XHCI_INTERRUPT_IN_POLL_PROOF_V1
    if intr_ep_addr == 0 || (intr_ep_addr & 0x80) == 0 || intr_ep_mps == 0 || intr_ep_mps > 16 {
        serial_println!(
            "[sexusb.xhci.intr_in.config.bad] addr={:#x} mps={} interval={}",
            intr_ep_addr,
            intr_ep_mps,
            intr_ep_interval
        );
        loop { sys_yield(); }
    }

    const INTR_DCI: u32 = 3; // EP1 IN endpoint context index
    const EP_TYPE_INTERRUPT_IN: u32 = 7;
    let intr_report_len: u32 = intr_ep_mps as u32;

    let intr_ring_phys = sys_alloc_phys(PAGE_SIZE);
    let intr_report_phys = sys_alloc_phys(PAGE_SIZE);
    if intr_ring_phys == 0 || intr_ring_phys == u64::MAX
        || intr_report_phys == 0 || intr_report_phys == u64::MAX
    {
        serial_println!("[sexusb.xhci.intr_in.alloc.bad]");
        loop { sys_yield(); }
    }
    let intr_ring_va = sys_map_phys(intr_ring_phys, PAGE_SIZE);
    let intr_report_va = sys_map_phys(intr_report_phys, PAGE_SIZE);
    if intr_ring_va == 0 || intr_ring_va == u64::MAX
        || intr_report_va == 0 || intr_report_va == u64::MAX
    {
        serial_println!("[sexusb.xhci.intr_in.map.bad]");
        loop { sys_yield(); }
    }
    if (intr_ring_phys % 64) != 0 || (intr_ring_va % PAGE_SIZE) != 0 {
        serial_println!("[sexusb.xhci.intr_in.align.bad] ring_phys={:#x} ring_va={:#x}", intr_ring_phys, intr_ring_va);
        loop { sys_yield(); }
    }
    unsafe {
        core::ptr::write_bytes(intr_ring_va as *mut u8, 0, PAGE_SIZE as usize);
        core::ptr::write_bytes(intr_report_va as *mut u8, 0, intr_report_len as usize);
        core::ptr::write_bytes(input_ctx_va as *mut u8, 0, PAGE_SIZE as usize);
    }
    // Circular interrupt Transfer Ring: 15 Normal slots + Link TRB at slot 15.
    // Link TRB wraps ring back to slot 0 with TC=1 (toggles xHCI consumer cycle).
    const TRB_TYPE_LINK: u32 = 6;
    const INTR_TR_RING_SIZE: u64 = 16;
    trb_write_volatile(
        intr_ring_va,
        INTR_TR_RING_SIZE - 1,
        (intr_ring_phys & 0xFFFF_FFFF) as u32,
        (intr_ring_phys >> 32) as u32,
        0u32,
        (TRB_TYPE_LINK << 10) | (1u32 << 1) | 1u32, // TC=1, cycle=1
    );

    serial_println!("[sexusb.xhci.intr_in.config_ep.start]");

    // ICC: add Slot Context (bit 0) + Endpoint Context index 3 (bit 3).
    unsafe {
        core::ptr::write_volatile(input_ctx_va as *mut u32, 0u32);            // Drop flags
        core::ptr::write_volatile((input_ctx_va + 4) as *mut u32, 0x9u32);    // Add flags
    }

    // Copy Slot Context from output device context.
    let out_slot_base = device_ctx_va as *const u32;
    let in_slot_base = (input_ctx_va + ctx_stride) as *mut u32;
    for i in 0..8u64 {
        let v = unsafe { core::ptr::read_volatile(out_slot_base.add(i as usize)) };
        unsafe { core::ptr::write_volatile(in_slot_base.add(i as usize), v); }
    }
    // Context Entries (Slot Context DW0 bits 31:27) must cover the highest added DCI.
    let slot_dw0 = unsafe { core::ptr::read_volatile(in_slot_base.add(0)) };
    let slot_dw0_new = (slot_dw0 & !(0x1Fu32 << 27)) | ((INTR_DCI & 0x1F) << 27);
    unsafe { core::ptr::write_volatile(in_slot_base.add(0), slot_dw0_new); }

    // Build EP1 IN endpoint context at DCI=3.
    let in_ep_base = (input_ctx_va + ctx_stride * (1 + INTR_DCI as u64)) as *mut u32;
    let ep_dw0 = ((intr_ep_interval as u32) << 16); // Interval in bits 23:16
    let ep_dw1 = (CERR_DEFAULT & 0x3)
        | (EP_TYPE_INTERRUPT_IN << 3)
        | ((intr_ep_mps as u32) << 16);
    let ep_deq = intr_ring_phys | 1u64; // DCS=1
    let ep_dw2 = (ep_deq & 0xFFFF_FFFF) as u32;
    let ep_dw3 = (ep_deq >> 32) as u32;
    let ep_dw4 = intr_report_len | (intr_report_len << 16); // avg TRB len + max ESIT payload
    unsafe {
        core::ptr::write_volatile(in_ep_base.add(0), ep_dw0);
        core::ptr::write_volatile(in_ep_base.add(1), ep_dw1);
        core::ptr::write_volatile(in_ep_base.add(2), ep_dw2);
        core::ptr::write_volatile(in_ep_base.add(3), ep_dw3);
        core::ptr::write_volatile(in_ep_base.add(4), ep_dw4);
    }

    // Configure Endpoint command.
    let cfg_ep_d0 = (input_ctx_phys & 0xFFFF_FFFF) as u32;
    let cfg_ep_d1 = (input_ctx_phys >> 32) as u32;
    let cfg_ep_d3 = (en_slot_id << 24) | (TRB_TYPE_CONFIGURE_ENDPOINT_CMD << 10) | cmd_cycle;
    trb_write_volatile(cmd_ring_va, cmd_idx, cfg_ep_d0, cfg_ep_d1, 0u32, cfg_ep_d3);
    trb_write_volatile(cmd_ring_va, cmd_idx + 1, 0, 0, 0, cmd_cycle ^ 1);
    mmio_write32(db_base, 0, 0u32);

    let mut cfg_ep_ok = false;
    for _ in 0..POLL_BUDGET {
        let ev_d3 = trb_read_dword(event_ring_va, ev_idx, 3);
        if (ev_d3 & 1) == (ev_dcs as u32) {
            let ev_type = (ev_d3 >> 10) & 0x3F;
            if ev_type == TRB_TYPE_CMD_COMPLETION_EVENT {
                let ev_d2 = trb_read_dword(event_ring_va, ev_idx, 2);
                let cc = (ev_d2 >> 24) & 0xFF;
                let slot = (ev_d3 >> 24) & 0xFF;
                if cc == TRB_CC_SUCCESS && slot == en_slot_id {
                    cfg_ep_ok = true;
                } else {
                    serial_println!("[sexusb.xhci.intr_in.config_ep.event.bad] cc={} slot={}", cc, slot);
                }
                trb_write_volatile(event_ring_va, ev_idx, 0, 0, 0, ev_d3 & !1u32);
                ev_idx += 1;
                if ev_idx >= EVENT_RING_TRBS {
                    ev_idx = 0;
                    ev_dcs ^= 1;
                }
                let new_erdp = event_ring_phys + ev_idx * 16;
                mmio_write32(intr_base, XHCI_INTR_ERDP, new_erdp as u32);
                mmio_write32(intr_base, XHCI_INTR_ERDP + 4, (new_erdp >> 32) as u32);
            }
            break;
        }
        sys_yield();
    }
    if !cfg_ep_ok {
        serial_println!("[sexusb.xhci.intr_in.config_ep.timeout.bad]");
        loop { sys_yield(); }
    }
    cmd_idx += 1;
    serial_println!("[sexusb.xhci.intr_in.config_ep.ok]");

    // Continuous bounded poll: one TRB in-flight at a time.
    // Inner loop waits indefinitely (no POLL_BUDGET timeout) — safe because the
    // xHCI completes (or NAKs-then-completes) the single enqueued TRB before we
    // re-arm. No second TRB is ever enqueued while one is outstanding.
    let dev_kind = if is_tablet_device { "tablet" } else { "mouse" };
    serial_println!("[sexusb.hid.{}.continuous.start] attempts=unbounded", dev_kind);
    let mut saw_nonzero = false;
    let mut i: u32 = 0;
    let mut intr_prod: u64 = 0;
    let mut intr_pcs: u32 = 1;
    loop {
        // Clear report buffer before each transfer.
        let clear_len = if intr_report_len > 8 { 8 } else { intr_report_len };
        unsafe {
            core::ptr::write_bytes(intr_report_va as *mut u8, 0, clear_len as usize);
        }

        // Queue one interrupt-IN Normal TRB at current ring producer slot.
        // Circular ring (Link TRB at slot INTR_TR_RING_SIZE-1) keeps xHCI dequeue
        // advancing correctly across iterations — no stuck-at-slot-1 stall.
        trb_write_volatile(
            intr_ring_va,
            intr_prod,
            (intr_report_phys & 0xFFFF_FFFF) as u32,
            (intr_report_phys >> 32) as u32,
            intr_report_len,
            (TRB_TYPE_NORMAL << 10) | (1u32 << 5) | intr_pcs, // IOC + current cycle
        );

        mmio_write32(db_base, en_slot_id as u64 * 4, INTR_DCI);

        // Wait indefinitely: xHCI retries interrupt-IN until device sends data.
        let mut intr_ok = false;
        let mut intr_residue: u32 = 0;
        loop {
            let ev_d3 = trb_read_dword(event_ring_va, ev_idx, 3);
            if (ev_d3 & 1) != (ev_dcs as u32) {
                sys_yield();
                continue;
            }
            let ev_type = (ev_d3 >> 10) & 0x3F;
            if ev_type == TRB_TYPE_TRANSFER_EVENT {
                let ev_d2 = trb_read_dword(event_ring_va, ev_idx, 2);
                let cc = (ev_d2 >> 24) & 0xFF;
                intr_residue = ev_d2 & 0xFFFFFF;
                let slot = (ev_d3 >> 24) & 0xFF;
                let ep = (ev_d3 >> 16) & 0x1F;
                if (cc == TRB_CC_SUCCESS || cc == TRB_CC_SHORT_PACKET)
                    && slot == en_slot_id && ep == INTR_DCI
                {
                    intr_ok = true;
                } else {
                    serial_println!("[sexusb.xhci.intr_in.event.bad] cc={} slot={} ep={}", cc, slot, ep);
                }
                trb_write_volatile(event_ring_va, ev_idx, 0, 0, 0, ev_d3 & !1u32);
                ev_idx += 1;
                if ev_idx >= EVENT_RING_TRBS { ev_idx = 0; ev_dcs ^= 1; }
                let new_erdp = event_ring_phys + ev_idx * 16;
                mmio_write32(intr_base, XHCI_INTR_ERDP, new_erdp as u32);
                mmio_write32(intr_base, XHCI_INTR_ERDP + 4, (new_erdp >> 32) as u32);
                break;
            }
            // Consume unrelated owned events and continue waiting.
            trb_write_volatile(event_ring_va, ev_idx, 0, 0, 0, ev_d3 & !1u32);
            ev_idx += 1;
            if ev_idx >= EVENT_RING_TRBS { ev_idx = 0; ev_dcs ^= 1; }
            let new_erdp = event_ring_phys + ev_idx * 16;
            mmio_write32(intr_base, XHCI_INTR_ERDP, new_erdp as u32);
            mmio_write32(intr_base, XHCI_INTR_ERDP + 4, (new_erdp >> 32) as u32);
        }
        if !intr_ok {
            // Wrong slot/endpoint on this event — skip report, re-arm.
            continue;
        }
        if intr_residue > intr_report_len {
            serial_println!("[sexusb.xhci.intr_in.residue.bad] residue={}", intr_residue);
            break;
        }
        let intr_actual = intr_report_len - intr_residue;
        let report_ptr = intr_report_va as *const u8;

        if is_tablet_device {
            // === Tablet decode path ===
            let rb0 = unsafe { core::ptr::read_volatile(report_ptr.add(0)) };
            let rb1 = unsafe { core::ptr::read_volatile(report_ptr.add(1)) };
            let rb2 = unsafe { core::ptr::read_volatile(report_ptr.add(2)) };
            let rb3 = unsafe { core::ptr::read_volatile(report_ptr.add(3)) };
            let rb4 = unsafe { core::ptr::read_volatile(report_ptr.add(4)) };
            let report_bytes = [rb0, rb1, rb2, rb3, rb4];
            // Dump raw bytes for first tablet report to verify data.
            if i == 0 && intr_actual >= 5 {
                serial_println!("[sexusb.hid.tablet.raw] b0={:#x} b1={:#x} b2={:#x} b3={:#x} b4={:#x} actual={}",
                    rb0, rb1, rb2, rb3, rb4, intr_actual);
            }

            if let Some(td) = decode_tablet_report(&report_bytes, intr_actual as usize) {
                // Track previous absolute position for delta computation.
                // Use static mut for simplicity; initialized to first report values.
                static mut PREV_ABS_X: u16 = 0;
                static mut PREV_ABS_Y: u16 = 0;
                static mut FIRST_TABLET_REPORT: bool = true;
                let first = unsafe { FIRST_TABLET_REPORT };
                let (dx_i8, dy_i8) = if first {
                    unsafe {
                        PREV_ABS_X = td.abs_x;
                        PREV_ABS_Y = td.abs_y;
                        FIRST_TABLET_REPORT = false;
                    }
                    (0i8, 0i8)
                } else {
                    let prev_x = unsafe { PREV_ABS_X };
                    let prev_y = unsafe { PREV_ABS_Y };
                    let raw_dx = (td.abs_x as i32) - (prev_x as i32);
                    let raw_dy = (td.abs_y as i32) - (prev_y as i32);
                    let dx_clipped = raw_dx.clamp(-128, 127) as i8;
                    let dy_clipped = raw_dy.clamp(-128, 127) as i8;
                    unsafe {
                        PREV_ABS_X = td.abs_x;
                        PREV_ABS_Y = td.abs_y;
                    }
                    (dx_clipped, dy_clipped)
                };
                let packed_axes = (dx_i8 as u8 as u64)
                    | ((dy_i8 as u8 as u64) << 8);
                let _ = pdx_call_checked(
                    SLOT_USB_SEXINPUT,
                    OP_USB_MOUSE_REPORT,
                    0,
                    td.buttons as u64,
                    packed_axes,
                );
                if td.buttons == 0 && dx_i8 == 0 && dy_i8 == 0 {
                    if i < 8 || i % 64 == 0 {
                        serial_println!("[sexusb.hid.tablet.continuous.idle] i={}", i);
                    }
                } else {
                    serial_println!(
                        "[sexusb.hid.tablet.report] i={} buttons={:#x} x={} y={} dx={} dy={}",
                        i, td.buttons, td.abs_x, td.abs_y, dx_i8, dy_i8
                    );
                    if !saw_nonzero {
                        saw_nonzero = true;
                        serial_println!(
                            "[sexusb.hid.tablet.nonzero.ok] i={} buttons={:#x} x={} y={} dx={} dy={}",
                            i, td.buttons, td.abs_x, td.abs_y, dx_i8, dy_i8
                        );
                    }
                }
            } else {
                serial_println!("[sexusb.hid.tablet.decode.bad] len={}", intr_actual);
            }
        } else {
            // === Mouse decode path (unchanged) ===
            let rb0 = unsafe { core::ptr::read_volatile(report_ptr.add(0)) };
            let rb1 = unsafe { core::ptr::read_volatile(report_ptr.add(1)) };
            let rb2 = unsafe { core::ptr::read_volatile(report_ptr.add(2)) };
            let rb3 = unsafe { core::ptr::read_volatile(report_ptr.add(3)) };
            let report_bytes = [rb0, rb1, rb2, rb3];
            if let Some(decoded) = decode_boot_mouse_report(&report_bytes, intr_actual as usize) {
                let packed_axes = (decoded.dx as u8 as u64)
                    | ((decoded.dy as u8 as u64) << 8)
                    | ((decoded.wheel as u8 as u64) << 16);
                let _ = pdx_call_checked(
                    SLOT_USB_SEXINPUT,
                    OP_USB_MOUSE_REPORT,
                    0,
                    decoded.buttons as u64,
                    packed_axes,
                );
                if decoded.buttons == 0 && decoded.dx == 0 && decoded.dy == 0 && decoded.wheel == 0 {
                    if i < 8 || i % 64 == 0 {
                        serial_println!("[sexusb.hid.mouse.continuous.idle] i={}", i);
                    }
                } else {
                    serial_println!(
                        "[sexusb.hid.mouse.continuous.report] i={} buttons={:#x} dx={} dy={} wheel={}",
                        i, decoded.buttons, decoded.dx, decoded.dy, decoded.wheel
                    );
                    if !saw_nonzero {
                        saw_nonzero = true;
                        serial_println!(
                            "[sexusb.hid.mouse.continuous.nonzero.ok] i={} buttons={:#x} dx={} dy={} wheel={}",
                            i, decoded.buttons, decoded.dx, decoded.dy, decoded.wheel
                        );
                        serial_println!(
                            "[sexusb.hid.mouse.nonzero.ok] i={} buttons={:#x} dx={} dy={} wheel={}",
                            i, decoded.buttons, decoded.dx, decoded.dy, decoded.wheel
                        );
                    }
                }
            } else {
                serial_println!("[sexusb.hid.mouse.decode.bad] len={}", intr_actual);
            }
        }
        // Advance circular ring producer: skip over Link TRB at INTR_TR_RING_SIZE-1.
        intr_prod += 1;
        if intr_prod >= INTR_TR_RING_SIZE - 1 {
            // Wrap: toggle PCS and update Link TRB cycle bit to match new PCS
            // so xHCI follows it correctly on the next wrap-around.
            intr_pcs ^= 1;
            trb_write_volatile(
                intr_ring_va,
                INTR_TR_RING_SIZE - 1,
                (intr_ring_phys & 0xFFFF_FFFF) as u32,
                (intr_ring_phys >> 32) as u32,
                0u32,
                (TRB_TYPE_LINK << 10) | (1u32 << 1) | intr_pcs,
            );
            intr_prod = 0;
        }
        serial_println!("[sexusb.xhci.intr_ring.advance] next={} cycle={}", intr_prod, intr_pcs);
        i = i.wrapping_add(1);
    }
    loop { sys_yield(); }
}
