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
    const XHCI_USBCMD: u64 = 0x00;
    const XHCI_USBSTS: u64 = 0x04;
    const USBCMD_RUN_STOP: u32 = 1 << 0;
    const USBCMD_HCRST: u32 = 1 << 1;
    const USBSTS_HCHALTED: u32 = 1 << 0;
    const USBSTS_CNR: u32 = 1 << 11;
    const POLL_BUDGET: usize = 100_000;

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

    loop { sys_yield(); }
}
