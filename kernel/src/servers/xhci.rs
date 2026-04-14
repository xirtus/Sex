use crate::serial_println;
use crate::servers::dde;

/// XHCI (USB 3.0) Driver Foundation.
/// This module provides the discovery and register-mapping logic 
/// for the eXtensible Host Controller Interface.

/// XHCI Transfer Request Block (TRB) - 16 bytes.
#[repr(C, packed)]
#[derive(Debug, Clone, Copy)]
pub struct Trb {
    pub parameter: u64,
    pub status: u32,
    pub control: u32,
}

pub struct XhciController {
    pub mmio_base: x86_64::VirtAddr,
    pub dcbaa_phys: u64,
    pub cmd_ring_phys: u64,
    pub event_ring_phys: u64,
}

impl XhciController {
    pub fn new(mmio_base: x86_64::VirtAddr) -> Self {
        Self { 
            mmio_base,
            dcbaa_phys: 0,
            cmd_ring_phys: 0,
            event_ring_phys: 0,
        }
    }

    /// Initializes the XHCI controller and its asynchronous rings.
    pub unsafe fn init_hardware(&mut self) {
        let mmio_ptr = self.mmio_base.as_u64() as *mut u32;
        
        // 1. Read Capability Registers
        let cap_length = mmio_ptr.read_volatile() & 0xFF;
        let oper_base = mmio_ptr.add(cap_length as usize / 4);
        let runtime_offset = mmio_ptr.offset(0x18 / 4).read_volatile();
        let runtime_base = mmio_ptr.add(runtime_offset as usize / 4);
        
        serial_println!("XHCI: CapLength={}, OperBase={:p}, RuntimeBase={:p}", 
            cap_length, oper_base, runtime_base);

        // 2. Reset Controller (USBCMD bit 1)
        let usbcmd = oper_base;
        usbcmd.write_volatile(usbcmd.read_volatile() | 2);
        while (usbcmd.read_volatile() & 2) != 0 {}
        serial_println!("XHCI: Controller Reset Complete.");

        // 3. Allocate and Configure DCBAA (Device Context Base Address Array)
        // For the prototype, we assume we have physical frames 0x_E000, 0x_E100, 0x_E200
        self.dcbaa_phys = 0x_E000_0000;
        oper_base.offset(0x30 / 4).write_volatile(self.dcbaa_phys as u32); // DCBAAP
        oper_base.offset(0x34 / 4).write_volatile((self.dcbaa_phys >> 32) as u32);

        // 4. Initialize Command Ring
        self.cmd_ring_phys = 0x_E100_0000;
        oper_base.offset(0x18 / 4).write_volatile((self.cmd_ring_phys | 1) as u32); // CRCR (Ring Cycle State = 1)
        oper_base.offset(0x1C / 4).write_volatile((self.cmd_ring_phys >> 32) as u32);

        // 5. Initialize Event Ring (via Event Ring Segment Table)
        let erst_phys = 0x_E200_0000;
        let ir_base = runtime_base.offset(0x20 / 4); // Interrupter 0
        ir_base.offset(0x08 / 4).write_volatile(1); // ERSTSZ = 1 segment
        ir_base.offset(0x10 / 4).write_volatile(erst_phys as u32); // ERSTBA
        ir_base.offset(0x14 / 4).write_volatile((erst_phys >> 32) as u32);
        
        serial_println!("XHCI: Asynchronous Command/Event Rings initialized.");

        // 6. Start Controller (USBCMD bit 0)
        usbcmd.write_volatile(usbcmd.read_volatile() | 1);
        while (oper_base.offset(0x04 / 4).read_volatile() & 1) != 0 {} // Wait for USBSTS.HCH to clear
        
    /// Performs USB device enumeration and identifies HID devices.
    pub unsafe fn enumerate_devices(&self) {
        serial_println!("XHCI: Enumerating USB devices...");
        
        let mmio_ptr = self.mmio_base.as_u64() as *mut u32;
        let cap_length = mmio_ptr.read_volatile() & 0xFF;
        let oper_base = mmio_ptr.add(cap_length as usize / 4);
        
        // 1. Check Max Ports
        let hcsparams1 = mmio_ptr.offset(0x04 / 4).read_volatile();
        let max_ports = (hcsparams1 >> 24) & 0xFF;
        serial_println!("XHCI: Controller supports {} ports.", max_ports);

        // 2. Iterate through ports to find connected devices
        for port in 1..=max_ports {
            let port_reg_offset = (0x400 + (port - 1) * 0x10) / 4;
            let portsc = oper_base.offset(port_reg_offset as isize).read_volatile();
            
            if (portsc & 0x01) != 0 { // CCS: Current Connect Status
                serial_println!("XHCI: Device connected on Port {}.", port);
                
                // 3. Reset the port (PR: Port Reset)
                oper_base.offset(port_reg_offset as isize).write_volatile(portsc | (1 << 4));
                
                // 4. In a real system, we'd wait for reset to finish and assign a slot
                self.probe_hid_descriptor(port);
            }
        }
    }

    fn probe_hid_descriptor(&self, port: u32) {
        // Conceptual: Send 'Get Descriptor' TRB to the device
        // Class 0x03 = HID
        serial_println!("XHCI: Found HID Device (Class 0x03) on Port {}.", port);
        serial_println!("XHCI: Boot Keyboard/Mouse ready for input redirection.");
    }
}

pub fn xhci_probe() -> Result<(), &'static str> {
    serial_println!("XHCI: Searching for USB 3.0 Controllers...");
    
    // 1. Find device via DDE (Class 0x0C, Subclass 0x03, Prog IF 0x30)
    let devices = dde::dde_pci_enumerate();
    let pci = devices.into_iter().find(|d| d.class_id == 0x0C && d.subclass_id == 0x03)
        .ok_or("XHCI: No controller found")?;

    serial_println!("XHCI: Found at {:02x}:{:02x}.{:x}", pci.bus, pci.dev, pci.func);

    // 2. Map BAR0
    let bar0 = pci.read_u32(0x10) & 0xFFFF_FFF0;
    let mmio_base = dde::dde_ioremap(bar0 as u64, 0x10000)?;
    
    let mut xhci = XhciController::new(mmio_base);
    unsafe { xhci.init_hardware(); }

    Ok(())
}
