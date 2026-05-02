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

#[no_mangle]
pub extern "C" fn _start() -> ! {
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

    loop { sys_yield(); }
}
