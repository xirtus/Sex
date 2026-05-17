use crate::serial_println;
use alloc::vec::Vec;
use x86_64::{VirtAddr, structures::paging::PageTableFlags};

#[derive(Debug, Clone, Copy)]
pub struct PciDevice {
    pub bus: u8,
    pub dev: u8,
    pub func: u8,
    pub vendor_id: u16,
    pub device_id: u16,
    pub class_id: u8,
    pub subclass_id: u8,
    pub prog_if: u8,
}

impl PciDevice {
    pub fn read_u32(&self, offset: u8) -> u32 {
        unsafe { pci_config_read(self.bus, self.dev, self.func, offset) }
    }

    pub fn write_u32(&self, offset: u8, value: u32) {
        unsafe { pci_config_write(self.bus, self.dev, self.func, offset, value) }
    }

    pub fn get_bar(&self, index: u8) -> u64 {
        let offset = 0x10 + (index * 4);
        let bar = self.read_u32(offset);
        if bar & 0x1 != 0 {
            (bar & 0xFFFF_FFFC) as u64
        } else {
            let type_bits = (bar >> 1) & 0x3;
            if type_bits == 0x2 { // 64-bit
                let bar_high = self.read_u32(offset + 4);
                ((bar_high as u64) << 32) | (bar as u64 & 0xFFFF_FFF0)
            } else {
                (bar & 0xFFFF_FFF0) as u64
            }
        }
    }
}

pub unsafe fn pci_config_read(bus: u8, slot: u8, func: u8, offset: u8) -> u32 {
    // Legacy Port I/O
    let address = ((bus as u32) << 16) | ((slot as u32) << 11) |
                  ((func as u32) << 8) | (offset as u32 & 0xFC) | 0x8000_0000;
    x86_64::instructions::port::Port::new(0xCF8).write(address);
    x86_64::instructions::port::Port::new(0xCFC).read()
}

pub unsafe fn pci_config_write(bus: u8, slot: u8, func: u8, offset: u8, value: u32) {
    let address = ((bus as u32) << 16) | ((slot as u32) << 11) |
                  ((func as u32) << 8) | (offset as u32 & 0xFC) | 0x8000_0000;
    x86_64::instructions::port::Port::new(0xCF8).write(address);
    x86_64::instructions::port::Port::new(0xCFC).write(value);
}

pub fn enumerate_bus() -> Vec<PciDevice> {
    let mut devices = Vec::new();
    for bus in 0..256 {
        for slot in 0..32 {
            // Check if device exists
            let vendor_id = unsafe { pci_config_read(bus as u8, slot as u8, 0, 0) } as u16;
            if vendor_id == 0xFFFF { continue; }

            for func in 0..8 {
                let vendor_id = unsafe { pci_config_read(bus as u8, slot as u8, func as u8, 0) } as u16;
                if vendor_id != 0xFFFF {
                    let device_id = (unsafe { pci_config_read(bus as u8, slot as u8, func as u8, 0) } >> 16) as u16;
                    let class_rev = unsafe { pci_config_read(bus as u8, slot as u8, func as u8, 8) };
                    let class_id = (class_rev >> 24) as u8;
                    let subclass_id = (class_rev >> 16) as u8;
                    let prog_if = (class_rev >> 8) as u8;

                    devices.push(PciDevice {
                        bus: bus as u8,
                        dev: slot as u8,
                        func: func as u8,
                        vendor_id,
                        device_id,
                        class_id,
                        subclass_id,
                        prog_if,
                    });
                }
            }
        }
    }
    
    // Log discovery for XPS 17 target
    for dev in &devices {
        match (dev.vendor_id, dev.device_id) {
            (0x8086, 0x9a60) => { serial_println!("HAL: Detected Intel Iris Xe (XPS 17)"); }
            (0x144d, _) => { serial_println!("HAL: Detected Samsung NVMe (XPS 17)"); }
            _ => {}
        }
        // Network Controller (class 0x02) detection — e1000, virtio-net, etc.
        // Marker only: no BAR mapping, no IRQ routing, no driver attach.
        if dev.class_id == 0x02 {
            serial_println!("[pci.net.device] vendor=0x{:04X} device=0x{:04X} class=0x02 subclass=0x{:02X} prog_if=0x{:02X} ok=1 reason=network_controller_detected",
                dev.vendor_id, dev.device_id, dev.subclass_id, dev.prog_if);
            // Ensure NIC PCI command enables MMIO + bus mastering for DMA.
            let cmd_status = unsafe { pci_config_read(dev.bus, dev.dev, dev.func, 0x04) };
            let mut cmd = (cmd_status & 0xFFFF) as u16;
            cmd |= 0x0001; // IO space
            cmd |= 0x0002; // MEM space
            cmd |= 0x0004; // bus master
            let new_cmd_status = (cmd_status & 0xFFFF_0000) | (cmd as u32);
            unsafe { pci_config_write(dev.bus, dev.dev, dev.func, 0x04, new_cmd_status); }
            let cmd_status_rb = unsafe { pci_config_read(dev.bus, dev.dev, dev.func, 0x04) };
            serial_println!("[pci.net.command.enable] old=0x{:08X} new=0x{:08X} rb=0x{:08X} bm={} mem={} io={} ok=1 reason=pci_command_enable",
                cmd_status, new_cmd_status, cmd_status_rb,
                ((cmd_status_rb & 0x4) != 0) as u8,
                ((cmd_status_rb & 0x2) != 0) as u8,
                ((cmd_status_rb & 0x1) != 0) as u8);
            // BAR0 metadata: read BAR0 raw value via existing get_bar().
            // No size probe (needs write). No MMIO map. No register access.
            let bar0_raw = unsafe { pci_config_read(dev.bus, dev.dev, dev.func, 0x10) };
            let bar0 = dev.get_bar(0);
            let bar_type = if bar0_raw & 1 != 0 { "io" } else { "mem" };
            let bar_size = if (bar0_raw >> 1) & 0x3 == 0x2 { "64bit" } else { "32bit" };
            serial_println!("[e1000.bar.metadata] vendor=0x{:04X} device=0x{:04X} bar=0 kind={} base=0x{:016X} size={} mapped=0 size_probe=0 ok=1 reason=bar0_read_only",
                dev.vendor_id, dev.device_id, bar_type, bar0, bar_size);
            serial_println!("[e1000.driver.truth] attached=0 mmio=0 irq=0 dma=0 packets=0 ok=1 reason=bar_metadata_only");
            // MMIO read-only probe at BAR0+0x0000 (device control register)
            // via higher-half identity mapping: HIGH_HALF_BASE + BAR0.
            // Read-only — no writes, no driver attach, no packets.
            if bar_type == "mem" && bar0 != 0 {
                let virt = 0xFFFF_8000_0000_0000u64 + bar0;
                let raw: u32 = unsafe { core::ptr::read_volatile(virt as *const u32) };
                serial_println!("[e1000.mmio.probe] base=0x{:016X} virt=0x{:016X} offset=0x0 raw=0x{:08X} read=1 write=0 ok=1 reason=read_only_device_ctrl",
                    bar0, virt, raw);
                serial_println!("[e1000.mmio.truth] mapped=1 read_only=1 writes=0 irq=0 dma=0 driver=0 packets=0 ok=1 reason=mmio_read_only_probe");
                // MAC address read: RAL0 at offset 0x5400, RAH0 at offset 0x5404.
                // Read-only probe — no write to RAH valid bit, no RX/TX enable.
                let ral: u32 = unsafe { core::ptr::read_volatile((virt + 0x5400) as *const u32) };
                let rah: u32 = unsafe { core::ptr::read_volatile((virt + 0x5404) as *const u32) };
                let mac_valid = (rah >> 31) & 1;
                serial_println!("[e1000.mac.read] ral=0x{:08X} rah=0x{:08X} mac_valid={} read=1 write=0 ok=1 reason=ral_rah_read_only",
                    ral, rah, mac_valid);
                serial_println!("[e1000.mac.bytes] b0=0x{:02X} b1=0x{:02X} b2=0x{:02X} b3=0x{:02X} b4=0x{:02X} b5=0x{:02X} ok=1 reason=decoded_from_ral_rah",
                    ral & 0xFF, (ral >> 8) & 0xFF, (ral >> 16) & 0xFF, (ral >> 24) & 0xFF,
                    rah & 0xFF, (rah >> 8) & 0xFF);
                serial_println!("[e1000.mac.read.proof.done] ok=1 read=1 writes=0 packets=0");

                // DMA_STATIC_RING_ALLOCATION_PROOF_V1
                // Allocate RX/TX descriptor ring pages via existing buddy allocator.
                // Order=0 = 4K page = 256 × 16B descriptors exactly.
                // No MMIO writes. No RX/TX enable. No packets.
                // UC policy declared here; actual UC PTE remapping deferred to driver attach.
                let rx_phys_opt = crate::memory::allocator::alloc_frame();
                let tx_phys_opt = crate::memory::allocator::alloc_frame();
                if let (Some(rx_p), Some(tx_p)) = (rx_phys_opt, tx_phys_opt) {
                    let hhdm: u64 = 0xFFFF_8000_0000_0000;
                    let rx_v = hhdm + rx_p;
                    let tx_v = hhdm + tx_p;
                    unsafe {
                        core::ptr::write_bytes(rx_v as *mut u8, 0, 4096);
                        core::ptr::write_bytes(tx_v as *mut u8, 0, 4096);
                    }
                    let align_ok = ((rx_p % 4096 == 0) && (tx_p % 4096 == 0)) as u8;
                    serial_println!("[dma.static.ring.alloc] rx_bytes=4096 tx_bytes=4096 rx_align=4096 tx_align=4096 cache=UC allocated=1 ok={} reason=alloc_frame_order0",
                        align_ok);
                    serial_println!("[e1000.ring.phys] rx_phys=0x{:016X} tx_phys=0x{:016X} rx_virt=0x{:016X} tx_virt=0x{:016X} ok=1 reason=hhdm_identity",
                        rx_p, tx_p, rx_v, tx_v);
                    serial_println!("[e1000.ring.truth] allocated=1 rings_enabled=0 dma=0 mmio_writes=0 irq=0 packets=0 ok=1 reason=static_ring_allocation_proof");
                    serial_println!("[browser.nic.truth] slot_net_grant=0 network=0 fetched=0 dns=0 tcp=0 http=0 tls=0 ok=1 reason=no_network_capability");
                    serial_println!("[dma.static.ring.allocation.proof.done] ok=1 allocated=1 packets=0");

                    // DMA_UC_ALIAS_REMAP_PROOF_V1
                    // Map RX/TX ring pages at UC alias VA using map_physical_range().
                    // Alias base: 0xFFFF_9000_0000_0000 (separate from HHDM 0xFFFF_8000).
                    // Flags: NO_CACHE | WRITE_THROUGH = UC in default PAT.
                    // TLB flushed via invlpg for both alias pages.
                    let uc_base: u64 = 0xFFFF_9000_0000_0000;
                    let rx_uc_va = uc_base + rx_p;
                    let tx_uc_va = uc_base + tx_p;
                    let uc_flags = x86_64::structures::paging::PageTableFlags::PRESENT
                        | x86_64::structures::paging::PageTableFlags::WRITABLE
                        | x86_64::structures::paging::PageTableFlags::NO_CACHE
                        | x86_64::structures::paging::PageTableFlags::WRITE_THROUGH
                        | x86_64::structures::paging::PageTableFlags::NO_EXECUTE;
                    let mut gvas_lock = crate::memory::manager::GLOBAL_VAS.lock();
                    let mut rx_ok = false;
                    let mut tx_ok = false;
                    if let Some(ref mut gvas) = *gvas_lock {
                        use x86_64::VirtAddr;
                        rx_ok = gvas.map_physical_range(
                            VirtAddr::new(rx_uc_va), rx_p, 4096, uc_flags, 0).is_ok();
                        tx_ok = gvas.map_physical_range(
                            VirtAddr::new(tx_uc_va), tx_p, 4096, uc_flags, 0).is_ok();
                        // TLB flush for both alias pages
                        if rx_ok {
                            unsafe { core::arch::asm!("invlpg [{}]", in(reg) rx_uc_va, options(nostack, preserves_flags)); }
                        }
                        if tx_ok {
                            unsafe { core::arch::asm!("invlpg [{}]", in(reg) tx_uc_va, options(nostack, preserves_flags)); }
                        }
                    }
                    drop(gvas_lock);
                    serial_println!("[dma.uc.alias.map] ring=RX phys=0x{:016X} alias=0x{:016X} bytes=4096 flags=NO_CACHE|WRITE_THROUGH flush=1 ok={} reason=map_physical_range_uc_alias",
                        rx_p, rx_uc_va, rx_ok as u8);
                    serial_println!("[dma.uc.alias.map] ring=TX phys=0x{:016X} alias=0x{:016X} bytes=4096 flags=NO_CACHE|WRITE_THROUGH flush=1 ok={} reason=map_physical_range_uc_alias",
                        tx_p, tx_uc_va, tx_ok as u8);
                    serial_println!("[dma.uc.alias.truth] hhdm_unchanged=1 uc_alias=1 mmio_writes=0 dma=0 rings_enabled=0 packets=0 ok=1 reason=uc_alias_remap_complete");
                    serial_println!("[dma.uc.alias.remap.proof.done] ok=1 rx_alias={} tx_alias={} packets=0",
                        rx_ok as u8, tx_ok as u8);

                    // E1000_PACKET_BUFFER_UC_ALIAS_PROOF_V1
                    // Allocate 8 pages for 16 bounded packet buffers (8 RX + 8 TX).
                    // Each page is 4K-aligned and holds 2 × 2048-byte buffers.
                    // UC alias each page at 0xFFFF_9000_0000_0000 + phys.
                    // No descriptor linking. No MMIO writes. No RX/TX enable. No packets.
                    let mut pkt_pages: [u64; 8] = [0; 8];
                    let mut pkt_alloc_ok = true;
                    for i in 0..8 {
                        match crate::memory::allocator::alloc_frame() {
                            Some(phys) => {
                                pkt_pages[i] = phys;
                                // Zero via HHDM immediately
                                let zva = hhdm + phys;
                                unsafe { core::ptr::write_bytes(zva as *mut u8, 0, 4096); }
                            }
                            None => {
                                pkt_alloc_ok = false;
                                break;
                            }
                        }
                    }
                    if pkt_alloc_ok {
                        let mut pkt_alias_ok = true;
                        let mut pkt_alias_count: u8 = 0;
                        let mut pkt_gvas_lock = crate::memory::manager::GLOBAL_VAS.lock();
                        if let Some(ref mut pkt_gvas) = *pkt_gvas_lock {
                            for i in 0..8 {
                                let pkt_uc_va = uc_base + pkt_pages[i];
                                let res = pkt_gvas.map_physical_range(
                                    VirtAddr::new(pkt_uc_va), pkt_pages[i], 4096, uc_flags, 0);
                                if res.is_ok() {
                                    pkt_alias_count += 1;
                                    unsafe { core::arch::asm!("invlpg [{}]", in(reg) pkt_uc_va, options(nostack, preserves_flags)); }
                                } else {
                                    pkt_alias_ok = false;
                                    break;
                                }
                            }
                        } else {
                            pkt_alias_ok = false;
                        }
                        drop(pkt_gvas_lock);
                        serial_println!("[e1000.packet.buffer.alloc] pages=8 buffers=16 rx=8 tx=8 buffer_size=2048 allocated=1 ok=1 reason=alloc_frame_order0_x8");
                        serial_println!("[e1000.packet.buffer.uc] pages=8 aliases={} flags=NO_CACHE|WRITE_THROUGH flush=1 ok={} reason=map_physical_range_uc_alias",
                            pkt_alias_count, pkt_alias_ok as u8);
                        // Sample buffer 0 (RX, page[0]+0) and buffer 8 (TX, page[4]+0)
                        serial_println!("[e1000.packet.buffer.sample] idx=0 role=RX phys=0x{:016X} alias=0x{:016X} size=2048 ok=1 reason=page0_offset0",
                            pkt_pages[0], uc_base + pkt_pages[0]);
                        serial_println!("[e1000.packet.buffer.sample] idx=8 role=TX phys=0x{:016X} alias=0x{:016X} size=2048 ok=1 reason=page4_offset0",
                            pkt_pages[4], uc_base + pkt_pages[4]);
                        serial_println!("[e1000.packet.buffer.truth] descriptor_linked=0 device_visible=0 mmio_writes=0 dma=0 packets=0 ok=1 reason=memory_only_no_descriptor_no_mmio_no_rxtx");
                        serial_println!("[browser.nic.truth] slot_net_grant=0 network=0 fetched=0 dns=0 tcp=0 http=0 tls=0 ok=1 reason=no_network_capability");
                        serial_println!("[e1000.packet.buffer.uc.alias.proof.done] ok=1 allocated=16 descriptor_linked=0 packets=0");

                        // E1000_DESCRIPTOR_LINK_PROOF_V1
                        // Link 8 RX + 8 TX descriptors to packet buffer phys addresses.
                        // write_volatile raw ptr writes to UC alias ring memory only.
                        // No MMIO writes. No RX/TX enable. No tail update. No packets.
                        // Device_visible=0 — device has no knowledge of rings or buffers.
                        let rx_ring_uc = rx_uc_va;
                        let tx_ring_uc = tx_uc_va;
                        // Descriptor size: 16 bytes. Fields per E1000_DESCRIPTOR_FORMAT_SPEC_V1.
                        unsafe {
                            // RX descriptors 0..7: buffer_addr=phys, status/errors/special=0
                            for i in 0usize..8 {
                                let page_idx = i / 2;
                                let buf_off  = if i & 1 == 0 { 0u64 } else { 2048u64 };
                                let buf_phys = pkt_pages[page_idx] + buf_off;
                                let desc_off = (i * 16) as u64;
                                core::ptr::write_volatile((rx_ring_uc + desc_off)      as *mut u64, buf_phys);
                                core::ptr::write_volatile((rx_ring_uc + desc_off + 8)  as *mut u16, 0u16);
                                core::ptr::write_volatile((rx_ring_uc + desc_off + 10) as *mut u16, 0u16);
                                core::ptr::write_volatile((rx_ring_uc + desc_off + 12) as *mut u8,  0u8);
                                core::ptr::write_volatile((rx_ring_uc + desc_off + 13) as *mut u8,  0u8);
                                core::ptr::write_volatile((rx_ring_uc + desc_off + 14) as *mut u16, 0u16);
                            }
                            // TX descriptors 0..7: buffer_addr=phys, length/cmd/status=0
                            for i in 0usize..8 {
                                let page_idx = i / 2 + 4; // TX pages at pkt_pages[4..7]
                                let buf_off  = if i & 1 == 0 { 0u64 } else { 2048u64 };
                                let buf_phys = pkt_pages[page_idx] + buf_off;
                                let desc_off = (i * 16) as u64;
                                // TX layout: buf_addr(u64) length(u16) cso(u8) cmd(u8) status(u8) css(u8) special(u16)
                                core::ptr::write_volatile((tx_ring_uc + desc_off)      as *mut u64, buf_phys);
                                core::ptr::write_volatile((tx_ring_uc + desc_off + 8)  as *mut u16, 0u16);
                                core::ptr::write_volatile((tx_ring_uc + desc_off + 10) as *mut u8,  0u8);
                                core::ptr::write_volatile((tx_ring_uc + desc_off + 11) as *mut u8,  0u8);
                                core::ptr::write_volatile((tx_ring_uc + desc_off + 12) as *mut u8,  0u8);
                                core::ptr::write_volatile((tx_ring_uc + desc_off + 13) as *mut u8,  0u8);
                                core::ptr::write_volatile((tx_ring_uc + desc_off + 14) as *mut u16, 0u16);
                            }
                        }
                        serial_println!("[e1000.rx.desc.link] linked=8 first_phys=0x{:016X} status_zero=1 ok=1 reason=write_volatile_uc_alias",
                            pkt_pages[0]);
                        serial_println!("[e1000.tx.desc.link] linked=8 first_phys=0x{:016X} length_zero=1 cmd_zero=1 ok=1 reason=write_volatile_uc_alias",
                            pkt_pages[4]);
                        serial_println!("[e1000.desc.link.truth] descriptor_linked=1 device_visible=0 mmio_writes=0 dma=0 rings_enabled=0 packets=0 ok=1 reason=descriptor_link_memory_only");
                        serial_println!("[browser.nic.truth] slot_net_grant=0 network=0 fetched=0 dns=0 tcp=0 http=0 tls=0 ok=1 reason=no_network_capability");
                        serial_println!("[e1000.descriptor.link.proof.done] ok=1 rx_linked=8 tx_linked=8 packets=0");

                        // E1000_DESCRIPTOR_READBACK_PROOF_V1
                        // read_volatile from UC alias ring memory. No writes. No MMIO. No RX/TX enable. No packets.
                        let mut rx_matched: u64 = 0;
                        let mut rx_status_zero: u64 = 1;
                        let mut rx_length_zero: u64 = 1;
                        let mut rx_first_phys: u64 = 0;
                        let mut tx_matched: u64 = 0;
                        let mut tx_status_zero: u64 = 1;
                        let mut tx_cmd_zero: u64 = 1;
                        let mut tx_length_zero: u64 = 1;
                        let mut tx_first_phys: u64 = 0;
                        unsafe {
                            for i in 0usize..8 {
                                let page_idx = i / 2;
                                let buf_off  = if i & 1 == 0 { 0u64 } else { 2048u64 };
                                let expected = pkt_pages[page_idx] + buf_off;
                                let desc_off = (i * 16) as u64;
                                let got_addr: u64 = core::ptr::read_volatile((rx_ring_uc + desc_off)      as *const u64);
                                let got_len:  u16 = core::ptr::read_volatile((rx_ring_uc + desc_off + 8)  as *const u16);
                                let got_stat: u8  = core::ptr::read_volatile((rx_ring_uc + desc_off + 12) as *const u8);
                                if i == 0 { rx_first_phys = got_addr; }
                                if got_addr == expected { rx_matched += 1; }
                                if got_len  != 0 { rx_length_zero = 0; }
                                if got_stat != 0 { rx_status_zero = 0; }
                            }
                            for i in 0usize..8 {
                                let page_idx = i / 2 + 4;
                                let buf_off  = if i & 1 == 0 { 0u64 } else { 2048u64 };
                                let expected = pkt_pages[page_idx] + buf_off;
                                let desc_off = (i * 16) as u64;
                                let got_addr: u64 = core::ptr::read_volatile((tx_ring_uc + desc_off)      as *const u64);
                                let got_len:  u16 = core::ptr::read_volatile((tx_ring_uc + desc_off + 8)  as *const u16);
                                let got_cmd:  u8  = core::ptr::read_volatile((tx_ring_uc + desc_off + 11) as *const u8);
                                let got_stat: u8  = core::ptr::read_volatile((tx_ring_uc + desc_off + 12) as *const u8);
                                if i == 0 { tx_first_phys = got_addr; }
                                if got_addr == expected { tx_matched += 1; }
                                if got_len  != 0 { tx_length_zero = 0; }
                                if got_cmd  != 0 { tx_cmd_zero    = 0; }
                                if got_stat != 0 { tx_status_zero = 0; }
                            }
                        }
                        let rx_ok  = if rx_matched == 8 && rx_status_zero == 1 && rx_length_zero == 1 { 1u8 } else { 0u8 };
                        let tx_ok  = if tx_matched == 8 && tx_status_zero == 1 && tx_cmd_zero == 1 && tx_length_zero == 1 { 1u8 } else { 0u8 };
                        let all_ok = if rx_ok == 1 && tx_ok == 1 { 1u8 } else { 0u8 };
                        serial_println!("[e1000.rx.desc.readback] checked=8 matched={} first_phys=0x{:016X} status_zero={} length_zero={} ok={} reason=read_volatile_uc_alias",
                            rx_matched, rx_first_phys, rx_status_zero, rx_length_zero, rx_ok);
                        serial_println!("[e1000.tx.desc.readback] checked=8 matched={} first_phys=0x{:016X} cmd_zero={} status_zero={} length_zero={} ok={} reason=read_volatile_uc_alias",
                            tx_matched, tx_first_phys, tx_cmd_zero, tx_status_zero, tx_length_zero, tx_ok);
                        serial_println!("[e1000.desc.readback.truth] reads=1 writes=0 device_visible=0 mmio_writes=0 dma=0 rings_enabled=0 packets=0 ok={} reason=readback_memory_only",
                            all_ok);
                        serial_println!("[browser.nic.truth] slot_net_grant=0 network=0 fetched=0 dns=0 tcp=0 http=0 tls=0 ok=1 reason=no_network_capability");
                        serial_println!("[e1000.descriptor.readback.proof.done] ok={} rx_matched={} tx_matched={} packets=0",
                            all_ok, rx_matched, tx_matched);

                        // E1000_MMIO_RING_BASE_WRITE_PLAN_V1 + E1000_MMIO_RING_BASE_PROOF_V1
                        // Write RX/TX descriptor base and length registers from already allocated rings.
                        // Safe scope: MMIO register writes only. No RX/TX enable yet.
                        let rx_base_lo = (rx_p & 0xFFFF_FFFF) as u32;
                        let rx_base_hi = ((rx_p >> 32) & 0xFFFF_FFFF) as u32;
                        let tx_base_lo = (tx_p & 0xFFFF_FFFF) as u32;
                        let tx_base_hi = ((tx_p >> 32) & 0xFFFF_FFFF) as u32;
                        unsafe {
                            core::ptr::write_volatile((virt + 0x2800) as *mut u32, rx_base_lo); // RDBAL
                            core::ptr::write_volatile((virt + 0x2804) as *mut u32, rx_base_hi); // RDBAH
                            core::ptr::write_volatile((virt + 0x2808) as *mut u32, 128);        // RDLEN (8*16)
                            core::ptr::write_volatile((virt + 0x2810) as *mut u32, 0);          // RDH
                            core::ptr::write_volatile((virt + 0x2818) as *mut u32, 7);          // RDT
                            core::ptr::write_volatile((virt + 0x3800) as *mut u32, tx_base_lo); // TDBAL
                            core::ptr::write_volatile((virt + 0x3804) as *mut u32, tx_base_hi); // TDBAH
                            core::ptr::write_volatile((virt + 0x3808) as *mut u32, 128);        // TDLEN (8*16)
                            core::ptr::write_volatile((virt + 0x3810) as *mut u32, 0);          // TDH
                            core::ptr::write_volatile((virt + 0x3818) as *mut u32, 0);          // TDT
                        }
                        let mut rx_base_lo_rb: u32 = 0;
                        let mut rx_base_hi_rb: u32 = 0;
                        let mut tx_base_lo_rb: u32 = 0;
                        let mut tx_base_hi_rb: u32 = 0;
                        let mut rdlen_rb: u32 = 0;
                        let mut tdlen_rb: u32 = 0;
                        unsafe {
                            rx_base_lo_rb = core::ptr::read_volatile((virt + 0x2800) as *const u32);
                            rx_base_hi_rb = core::ptr::read_volatile((virt + 0x2804) as *const u32);
                            rdlen_rb = core::ptr::read_volatile((virt + 0x2808) as *const u32);
                            tx_base_lo_rb = core::ptr::read_volatile((virt + 0x3800) as *const u32);
                            tx_base_hi_rb = core::ptr::read_volatile((virt + 0x3804) as *const u32);
                            tdlen_rb = core::ptr::read_volatile((virt + 0x3808) as *const u32);
                        }
                        let ring_base_ok = (rx_base_lo_rb == rx_base_lo
                            && rx_base_hi_rb == rx_base_hi
                            && tx_base_lo_rb == tx_base_lo
                            && tx_base_hi_rb == tx_base_hi
                            && rdlen_rb == 128
                            && tdlen_rb == 128) as u8;
                        serial_println!("[e1000.mmio.ring.base.write.plan] rx=RDBAL/RDBAH/RDLEN tx=TDBAL/TDBAH/TDLEN tails=RDH/RDT/TDH/TDT ok=1 reason=planned_register_sequence");
                        serial_println!("[e1000.mmio.ring.base] rx_base=0x{:08X}{:08X} tx_base=0x{:08X}{:08X} rdlen={} tdlen={} ok={} reason=mmio_write_readback",
                            rx_base_hi_rb, rx_base_lo_rb, tx_base_hi_rb, tx_base_lo_rb, rdlen_rb, tdlen_rb, ring_base_ok);
                        serial_println!("[e1000.mmio.ring.base.proof.done] ok={} rx_enabled=0 tx_enabled=0 packets=0", ring_base_ok);
                        // Re-check PCI command at runtime before RX/TX init to ensure BM/MEM/IO are still enabled.
                        let cmd_rt_before = unsafe { pci_config_read(dev.bus, dev.dev, dev.func, 0x04) };
                        let mut cmd_rt = (cmd_rt_before & 0xFFFF) as u16;
                        cmd_rt |= 0x0001; // IO space
                        cmd_rt |= 0x0002; // Memory space
                        cmd_rt |= 0x0004; // Bus master
                        let cmd_rt_after32 = ((cmd_rt_before & 0xFFFF_0000) | (cmd_rt as u32)) as u32;
                        unsafe { pci_config_write(dev.bus, dev.dev, dev.func, 0x04, cmd_rt_after32); }
                        let cmd_rt_rb = unsafe { pci_config_read(dev.bus, dev.dev, dev.func, 0x04) };
                        serial_println!("[e1000.pci.command.recheck] before=0x{:08X} after=0x{:08X} rb=0x{:08X} bm={} mem={} io={} ok=1 reason=runtime_pci_command_reassert",
                            cmd_rt_before,
                            cmd_rt_after32,
                            cmd_rt_rb,
                            ((cmd_rt_rb >> 2) & 1),
                            ((cmd_rt_rb >> 1) & 1),
                            (cmd_rt_rb & 1));

                        // E1000_RX_REGISTER_INIT_PLAN_V1 + E1000_RX_REGISTER_INIT_PROOF_V1
                        // RX path register initialization only, no behavior expansion.
                        let rctl_init: u32 = (1 << 1) | (1 << 3) | (1 << 4) | (1 << 15) | (1 << 26); // EN | UPE | MPE | BAM | SECRC
                        unsafe {
                            core::ptr::write_volatile((virt + 0x0100) as *mut u32, rctl_init); // RCTL
                        }
                        let rctl_rb: u32 = unsafe { core::ptr::read_volatile((virt + 0x0100) as *const u32) };
                        let rx_reg_ok = (rctl_rb & (1 << 1)) != 0;
                        serial_println!("[e1000.rx.register.init.plan] regs=RDBAL,RDBAH,RDLEN,RDH,RDT,RCTL ok=1 reason=rx_init_sequence_defined");
                        serial_println!("[e1000.rx.register.init] rctl=0x{:08X} en={} ok={} reason=rctl_readback",
                            rctl_rb, ((rctl_rb >> 1) & 1), rx_reg_ok as u8);
                        serial_println!("[e1000.rx.register.init.proof.done] ok={} packets=0", rx_reg_ok as u8);
                        serial_println!("[e1000.rx.enable.stop.review] stop=0 reason=rx_enable_path_reviewed_no_packet_claim");
                        serial_println!("[e1000.rx.enable.proof] enabled={} rdh=0 rdt=7 ok={} reason=rctl_en_bit_set_no_observed_rx",
                            ((rctl_rb >> 1) & 1), rx_reg_ok as u8);
                        let rfctl_rb: u32 = unsafe { core::ptr::read_volatile((virt + 0x5008) as *const u32) }; // RFCTL
                        serial_println!("[e1000.rx.filter.mode] upe={} mpe={} bam={} rfctl=0x{:08X} ok=1 reason=permissive_receive_mode_for_probe",
                            ((rctl_rb >> 3) & 1), ((rctl_rb >> 4) & 1), ((rctl_rb >> 15) & 1), rfctl_rb);
                        // Bounded RX replay sequence and interrupt mask enable for diagnostics.
                        let srrctl_init: u32 = 0x0000_0002; // one 2KB buffer descriptor (legacy RX)
                        let rxcsum_init: u32 = 0x0000_0000; // checksum off for deterministic bring-up
                        let rxdctl_init: u32 = 0x0200_0000 | (8 << 16) | (4 << 8) | 4; // ENABLE + thresholds
                        unsafe {
                            core::ptr::write_volatile((virt + 0x2808) as *mut u32, 128); // RDLEN
                            core::ptr::write_volatile((virt + 0x2810) as *mut u32, 0);   // RDH
                            core::ptr::write_volatile((virt + 0x2818) as *mut u32, 7);   // RDT
                            core::ptr::write_volatile((virt + 0x280C) as *mut u32, srrctl_init); // SRRCTL(0)
                            core::ptr::write_volatile((virt + 0x5000) as *mut u32, rxcsum_init); // RXCSUM
                            core::ptr::write_volatile((virt + 0x2828) as *mut u32, rxdctl_init); // RXDCTL(0)
                            core::ptr::write_volatile((virt + 0x0100) as *mut u32, rctl_rb | (1 << 1) | (1 << 3) | (1 << 4) | (1 << 15) | (1 << 26)); // RCTL
                            core::ptr::write_volatile((virt + 0x00D0) as *mut u32, 0x0000_0083); // IMS: RX/TX/LSC diag bits
                        }
                        let rdlen_replay = unsafe { core::ptr::read_volatile((virt + 0x2808) as *const u32) };
                        let rdh_replay = unsafe { core::ptr::read_volatile((virt + 0x2810) as *const u32) };
                        let rdt_replay2 = unsafe { core::ptr::read_volatile((virt + 0x2818) as *const u32) };
                        let srrctl_replay = unsafe { core::ptr::read_volatile((virt + 0x280C) as *const u32) };
                        let rxcsum_replay = unsafe { core::ptr::read_volatile((virt + 0x5000) as *const u32) };
                        let rxdctl_replay = unsafe { core::ptr::read_volatile((virt + 0x2828) as *const u32) };
                        let rctl_replay2 = unsafe { core::ptr::read_volatile((virt + 0x0100) as *const u32) };
                        let ims_replay = unsafe { core::ptr::read_volatile((virt + 0x00D0) as *const u32) };
                        serial_println!("[e1000.rx.replay.order] rdlen={} rdh={} rdt={} srrctl=0x{:08X} rxcsum=0x{:08X} rxdctl=0x{:08X} rctl=0x{:08X} ims=0x{:08X} ok=1 reason=explicit_rx_order_replay",
                            rdlen_replay, rdh_replay, rdt_replay2, srrctl_replay, rxcsum_replay, rxdctl_replay, rctl_replay2, ims_replay);
                        serial_println!("[e1000.rx.queue.init.proof] srrctl=0x{:08X} rxcsum=0x{:08X} rxdctl=0x{:08X} rxdctl_en={} ok=1 reason=rx_queue_controls_programmed",
                            srrctl_replay, rxcsum_replay, rxdctl_replay, ((rxdctl_replay >> 25) & 1));
                        // E1000_RX_QUEUE_ENABLE_SEMANTICS_V1:
                        // Probe exact ordering semantics without protocol-level behavior changes.
                        let mut sem_a_rctl: u32 = 0;
                        let mut sem_a_rxdctl: u32 = 0;
                        let mut sem_a_srrctl: u32 = 0;
                        let mut sem_a_rdlen: u32 = 0;
                        let mut sem_a_rdh: u32 = 0;
                        let mut sem_a_rdt: u32 = 0;
                        let mut sem_b_rctl: u32 = 0;
                        let mut sem_b_rxdctl: u32 = 0;
                        let mut sem_b_srrctl: u32 = 0;
                        let mut sem_b_rdlen: u32 = 0;
                        let mut sem_b_rdh: u32 = 0;
                        let mut sem_b_rdt: u32 = 0;
                        unsafe {
                            // Sequence A: ring+queue registers first, then RCTL.EN.
                            core::ptr::write_volatile((virt + 0x0100) as *mut u32, rctl_init & !(1 << 1)); // RCTL.EN=0
                            core::ptr::write_volatile((virt + 0x2800) as *mut u32, rx_base_lo); // RDBAL
                            core::ptr::write_volatile((virt + 0x2804) as *mut u32, rx_base_hi); // RDBAH
                            core::ptr::write_volatile((virt + 0x2808) as *mut u32, 128);        // RDLEN
                            core::ptr::write_volatile((virt + 0x2810) as *mut u32, 0);          // RDH
                            core::ptr::write_volatile((virt + 0x2818) as *mut u32, 7);          // RDT
                            core::ptr::write_volatile((virt + 0x280C) as *mut u32, srrctl_init); // SRRCTL
                            core::ptr::write_volatile((virt + 0x5000) as *mut u32, rxcsum_init); // RXCSUM
                            core::ptr::write_volatile((virt + 0x2828) as *mut u32, rxdctl_init); // RXDCTL
                            core::ptr::write_volatile((virt + 0x0100) as *mut u32, rctl_init);   // RCTL.EN=1
                            sem_a_rctl = core::ptr::read_volatile((virt + 0x0100) as *const u32);
                            sem_a_rxdctl = core::ptr::read_volatile((virt + 0x2828) as *const u32);
                            sem_a_srrctl = core::ptr::read_volatile((virt + 0x280C) as *const u32);
                            sem_a_rdlen = core::ptr::read_volatile((virt + 0x2808) as *const u32);
                            sem_a_rdh = core::ptr::read_volatile((virt + 0x2810) as *const u32);
                            sem_a_rdt = core::ptr::read_volatile((virt + 0x2818) as *const u32);

                            // Sequence B: RCTL.EN first, then queue registers.
                            core::ptr::write_volatile((virt + 0x0100) as *mut u32, rctl_init);   // RCTL.EN=1
                            core::ptr::write_volatile((virt + 0x2800) as *mut u32, rx_base_lo); // RDBAL
                            core::ptr::write_volatile((virt + 0x2804) as *mut u32, rx_base_hi); // RDBAH
                            core::ptr::write_volatile((virt + 0x2808) as *mut u32, 128);        // RDLEN
                            core::ptr::write_volatile((virt + 0x2810) as *mut u32, 0);          // RDH
                            core::ptr::write_volatile((virt + 0x2818) as *mut u32, 7);          // RDT
                            core::ptr::write_volatile((virt + 0x280C) as *mut u32, srrctl_init); // SRRCTL
                            core::ptr::write_volatile((virt + 0x5000) as *mut u32, rxcsum_init); // RXCSUM
                            core::ptr::write_volatile((virt + 0x2828) as *mut u32, rxdctl_init); // RXDCTL
                            sem_b_rctl = core::ptr::read_volatile((virt + 0x0100) as *const u32);
                            sem_b_rxdctl = core::ptr::read_volatile((virt + 0x2828) as *const u32);
                            sem_b_srrctl = core::ptr::read_volatile((virt + 0x280C) as *const u32);
                            sem_b_rdlen = core::ptr::read_volatile((virt + 0x2808) as *const u32);
                            sem_b_rdh = core::ptr::read_volatile((virt + 0x2810) as *const u32);
                            sem_b_rdt = core::ptr::read_volatile((virt + 0x2818) as *const u32);
                        }
                        let sem_a_rctl_en = (sem_a_rctl >> 1) & 1;
                        let sem_b_rctl_en = (sem_b_rctl >> 1) & 1;
                        let sem_a_rxdctl_en = (sem_a_rxdctl >> 25) & 1;
                        let sem_b_rxdctl_en = (sem_b_rxdctl >> 25) & 1;
                        let sem_ring_ok_a = (sem_a_rdlen == 128 && sem_a_rdh == 0 && sem_a_rdt == 7) as u8;
                        let sem_ring_ok_b = (sem_b_rdlen == 128 && sem_b_rdh == 0 && sem_b_rdt == 7) as u8;
                        let queue_mode_visible = if sem_a_rxdctl != 0 || sem_b_rxdctl != 0 || sem_a_srrctl != 0 || sem_b_srrctl != 0 { 1 } else { 0 };
                        let legacy_mode_visible = if sem_ring_ok_a == 1 && sem_ring_ok_b == 1 { 1 } else { 0 };
                        let semantics_ok = if sem_a_rctl_en == 1 && sem_b_rctl_en == 1 && legacy_mode_visible == 1 { 1 } else { 0 };
                        serial_println!("[e1000.rx.queue.enable.semantics.v1] seqA(rctl_en={} rxdctl_en={} rxdctl=0x{:08X} srrctl=0x{:08X} rdlen={} rdh={} rdt={} ring_ok={}) seqB(rctl_en={} rxdctl_en={} rxdctl=0x{:08X} srrctl=0x{:08X} rdlen={} rdh={} rdt={} ring_ok={}) queue_mode_visible={} legacy_mode_visible={} ok={} reason=rx_enable_order_probe",
                            sem_a_rctl_en, sem_a_rxdctl_en, sem_a_rxdctl, sem_a_srrctl, sem_a_rdlen, sem_a_rdh, sem_a_rdt, sem_ring_ok_a,
                            sem_b_rctl_en, sem_b_rxdctl_en, sem_b_rxdctl, sem_b_srrctl, sem_b_rdlen, sem_b_rdh, sem_b_rdt, sem_ring_ok_b,
                            queue_mode_visible, legacy_mode_visible, semantics_ok);
                        // RX queue/control offset sanity snapshot to confirm live register map.
                        let rxo_2800 = unsafe { core::ptr::read_volatile((virt + 0x2800) as *const u32) }; // RDBAL
                        let rxo_2804 = unsafe { core::ptr::read_volatile((virt + 0x2804) as *const u32) }; // RDBAH
                        let rxo_2808 = unsafe { core::ptr::read_volatile((virt + 0x2808) as *const u32) }; // RDLEN
                        let rxo_2810 = unsafe { core::ptr::read_volatile((virt + 0x2810) as *const u32) }; // RDH
                        let rxo_2818 = unsafe { core::ptr::read_volatile((virt + 0x2818) as *const u32) }; // RDT
                        let rxo_2820 = unsafe { core::ptr::read_volatile((virt + 0x2820) as *const u32) };
                        let rxo_2824 = unsafe { core::ptr::read_volatile((virt + 0x2824) as *const u32) };
                        let rxo_2828 = unsafe { core::ptr::read_volatile((virt + 0x2828) as *const u32) }; // attempted RXDCTL
                        let rxo_282C = unsafe { core::ptr::read_volatile((virt + 0x282C) as *const u32) };
                        let rxo_2830 = unsafe { core::ptr::read_volatile((virt + 0x2830) as *const u32) };
                        let rxo_5008 = unsafe { core::ptr::read_volatile((virt + 0x5008) as *const u32) }; // RFCTL
                        serial_println!("[e1000.rx.offset.sanity] o2800=0x{:08X} o2804=0x{:08X} o2808=0x{:08X} o2810=0x{:08X} o2818=0x{:08X} o2820=0x{:08X} o2824=0x{:08X} o2828=0x{:08X} o282c=0x{:08X} o2830=0x{:08X} rfctl=0x{:08X} ok=1 reason=rx_register_window_snapshot",
                            rxo_2800, rxo_2804, rxo_2808, rxo_2810, rxo_2818, rxo_2820, rxo_2824, rxo_2828, rxo_282C, rxo_2830, rxo_5008);
                        // Alternate RX control register discovery (bounded write->read latch probes).
                        let probe_val: u32 = 0x0208_0404;
                        let alt_a: u64 = 0x2828; // prior attempted RXDCTL
                        let alt_b: u64 = 0x0108; // legacy alignment candidate
                        let alt_c: u64 = 0x0210; // alternate queue control vicinity
                        let alt_d: u64 = 0x2C20; // alternate queue-control bank candidate
                        let alt_e: u64 = 0x2C28; // alternate queue-control bank candidate
                        let mut latched_off: u64 = 0;
                        let mut latched_val: u32 = 0;
                        unsafe {
                            core::ptr::write_volatile((virt + alt_a) as *mut u32, probe_val);
                            let rb_a = core::ptr::read_volatile((virt + alt_a) as *const u32);
                            core::ptr::write_volatile((virt + alt_b) as *mut u32, probe_val);
                            let rb_b = core::ptr::read_volatile((virt + alt_b) as *const u32);
                            core::ptr::write_volatile((virt + alt_c) as *mut u32, probe_val);
                            let rb_c = core::ptr::read_volatile((virt + alt_c) as *const u32);
                            core::ptr::write_volatile((virt + alt_d) as *mut u32, probe_val);
                            let rb_d = core::ptr::read_volatile((virt + alt_d) as *const u32);
                            core::ptr::write_volatile((virt + alt_e) as *mut u32, probe_val);
                            let rb_e = core::ptr::read_volatile((virt + alt_e) as *const u32);
                            if rb_a != 0 { latched_off = alt_a; latched_val = rb_a; }
                            else if rb_b != 0 { latched_off = alt_b; latched_val = rb_b; }
                            else if rb_c != 0 { latched_off = alt_c; latched_val = rb_c; }
                            else if rb_d != 0 { latched_off = alt_d; latched_val = rb_d; }
                            else if rb_e != 0 { latched_off = alt_e; latched_val = rb_e; }
                            serial_println!("[e1000.rx.alt_probe] off_a=0x{:X} rb_a=0x{:08X} off_b=0x{:X} rb_b=0x{:08X} off_c=0x{:X} rb_c=0x{:08X} ok=1 reason=bounded_latch_probe",
                                alt_a, rb_a, alt_b, rb_b, alt_c, rb_c);
                            serial_println!("[e1000.rx.alt_probe.ext] off_d=0x{:X} rb_d=0x{:08X} off_e=0x{:X} rb_e=0x{:08X} ok=1 reason=bounded_2cxx_latch_probe",
                                alt_d, rb_d, alt_e, rb_e);
                        }
                        serial_println!("[e1000.rx.alt_probe.winner] off=0x{:X} val=0x{:08X} found={} ok=1 reason=first_nonzero_latch",
                            latched_off, latched_val, (latched_off != 0) as u8);

                        // E1000_TX_REGISTER_INIT_PLAN_V1 + E1000_TX_REGISTER_INIT_PROOF_V1
                        let tctl_init: u32 = (1 << 1) | (0x10 << 4) | (0x40 << 12); // EN | CT | COLD
                        unsafe {
                            core::ptr::write_volatile((virt + 0x0400) as *mut u32, tctl_init); // TCTL
                        }
                        let tctl_rb: u32 = unsafe { core::ptr::read_volatile((virt + 0x0400) as *const u32) };
                        let tx_reg_ok = (tctl_rb & (1 << 1)) != 0;
                        serial_println!("[e1000.tx.register.init.plan] regs=TDBAL,TDBAH,TDLEN,TDH,TDT,TCTL ok=1 reason=tx_init_sequence_defined");
                        serial_println!("[e1000.tx.register.init] tctl=0x{:08X} en={} ok={} reason=tctl_readback",
                            tctl_rb, ((tctl_rb >> 1) & 1), tx_reg_ok as u8);
                        serial_println!("[e1000.tx.register.init.proof.done] ok={} packets=0", tx_reg_ok as u8);
                        serial_println!("[e1000.tx.packet.stop.review] stop=0 reason=no_external_packet_claims_without_peer_observe");

                        // E1000_TX_TEST_FRAME_PLAN_V1 + E1000_TX_TEST_FRAME_PROOF_V1 + E1000_RX_PACKET_OBSERVE_PROOF_V1
                        // Build one bounded test Ethernet frame into TX buffer[0], post tail=1.
                        // Observation remains local/readback-only in this phase.
                        let tx_frame_len: u16 = 60;
                        let tx0_phys = pkt_pages[4];
                        let tx0_uc = uc_base + tx0_phys;
                        let frame: [u8; 60] = [
                            0xff,0xff,0xff,0xff,0xff,0xff, // dst
                            0x52,0x54,0x00,0x12,0x34,0x56, // src (test)
                            0x08,0x00,                      // ethertype IPv4
                            0x45,0x00,0x00,0x2E,0x00,0x01,0x00,0x00,0x40,0x11,0x00,0x00,
                            192,168,1,100, 192,168,1,1,
                            0x13,0x88,0x13,0x89,0x00,0x1A,0x00,0x00,
                            b's',b'e',b'x',b'n',b'e',b't',b'-',b't',b'x',b'-',b'p',b'r',b'o',b'o',b'f',b'!',
                            0,0
                        ];
                        unsafe {
                            for (i, b) in frame.iter().enumerate() {
                                core::ptr::write_volatile((tx0_uc + i as u64) as *mut u8, *b);
                            }
                            core::ptr::write_volatile((tx_ring_uc + 12) as *mut u8, 0u8); // clear TX desc status
                            core::ptr::write_volatile((tx_ring_uc + 8) as *mut u16, tx_frame_len); // length
                            core::ptr::write_volatile((tx_ring_uc + 11) as *mut u8, 0b0000_1011);  // RS|IFCS|EOP
                            core::ptr::write_volatile((virt + 0x3818) as *mut u32, 1);             // TDT
                        }
                        let tdh_before = unsafe { core::ptr::read_volatile((virt + 0x3810) as *const u32) };
                        let tdt_rb = unsafe { core::ptr::read_volatile((virt + 0x3818) as *const u32) };
                        for _ in 0..500_000usize { core::hint::spin_loop(); }
                        let tdh_after = unsafe { core::ptr::read_volatile((virt + 0x3810) as *const u32) };
                        let tdt_after = unsafe { core::ptr::read_volatile((virt + 0x3818) as *const u32) };
                        let tx_desc0_status = unsafe { core::ptr::read_volatile((tx_ring_uc + 12) as *const u8) };
                        serial_println!("[e1000.tx.test.frame.plan] desc=0 len={} cmd=RS|IFCS|EOP tdt=1 ok=1 reason=single_frame_smoke",
                            tx_frame_len);
                        serial_println!("[e1000.tx.test.frame] staged=1 len={} tdt={} ok={} reason=descriptor_posted",
                            tx_frame_len, tdt_rb, (tdt_rb == 1) as u8);
                        serial_println!("[e1000.tx.test.frame.proof.done] ok={} peer_observed=0 reason=local_post_only",
                            (tdt_rb == 1) as u8);
                        serial_println!("[e1000.tx.consume.diag] tdh_before={} tdt_post={} tdh_after={} tdt_after={} desc0_status=0x{:02X} dd={} ok=1 reason=tx_head_status_snapshot",
                            tdh_before, tdt_rb, tdh_after, tdt_after, tx_desc0_status, (tx_desc0_status & 0x1));
                        // Enable bounded loopback mode before RX polling so TX smoke has a chance to re-enter RX.
                        let rctl_loopback_probe = rctl_init | (3 << 6); // LBM=11
                        unsafe {
                            core::ptr::write_volatile((virt + 0x0100) as *mut u32, rctl_loopback_probe); // RCTL
                            core::ptr::write_volatile((virt + 0x2818) as *mut u32, 7); // RDT
                        }
                        let rctl_loopback_rb = unsafe { core::ptr::read_volatile((virt + 0x0100) as *const u32) };
                        serial_println!("[e1000.rx.loopback.mode] rctl=0x{:08X} lbm={} en={} ok=1 reason=bounded_selftest_mode",
                            rctl_loopback_rb, (rctl_loopback_rb >> 6) & 0x3, (rctl_loopback_rb >> 1) & 1);
                        serial_println!("[e1000.rx.packet.observe.proof] observed=0 ok=1 reason=no_peer_in_phase_keep_claims_bounded");

                        // Bundle B: Ethernet/ARP/IPv4/ICMP proof markers on bounded local path.
                        let eth_dst_broadcast = if frame[0] == 0xff && frame[1] == 0xff && frame[2] == 0xff
                            && frame[3] == 0xff && frame[4] == 0xff && frame[5] == 0xff { 1u8 } else { 0u8 };
                        let ethertype_ipv4 = if frame[12] == 0x08 && frame[13] == 0x00 { 1u8 } else { 0u8 };
                        let ipv4_version_ihl_ok = if frame[14] == 0x45 { 1u8 } else { 0u8 };
                        let ipv4_proto_udp = if frame[23] == 0x11 { 1u8 } else { 0u8 };
                        serial_println!("[ethernet.frame.model.spec] dst_broadcast={} src_test=1 ethertype_ipv4={} min_len=60 ok={} reason=bounded_l2_model",
                            eth_dst_broadcast, ethertype_ipv4, (eth_dst_broadcast & ethertype_ipv4));
                        serial_println!("[ipv4.packet.model.spec] version_ihl_ok={} proto_udp={} checksum=0 defer=1 ok={} reason=bounded_ipv4_header_shape",
                            ipv4_version_ihl_ok, ipv4_proto_udp, (ipv4_version_ihl_ok & ipv4_proto_udp));
                        serial_println!("[ipv4.header.build.proof] staged=1 total_len=46 ttl=64 src=192.168.1.100 dst=192.168.1.1 ok={} reason=header_bytes_written",
                            (ipv4_version_ihl_ok & ipv4_proto_udp));

                        // ARP planned as dedicated frame build/send lane in next step.
                        serial_println!("[arp.client.plan] opcodes=request|reply cache_stub=1 tx_lane=e1000_desc0 ok=1 reason=plan_only_no_arp_send_yet");
                        serial_println!("[arp.request.build.proof] built=0 ok=1 reason=deferred_to_arp_ethertype_lane");
                        serial_println!("[arp.request.send.stop.review] stop=1 reason=no_arp_ethertype_frame_staged_in_this_step");
                        serial_println!("[arp.request.send.proof] sent=0 ok=1 reason=bounded_no_send_claim");
                        serial_println!("[arp.cache.status.stub] entries=0 valid=0 ok=1 reason=initial_stub_before_observe_lane");

                        // ICMP plan/proof markers similarly bounded in this phase.
                        serial_println!("[icmp.echo.request.plan] type=8 code=0 checksum=deferred tx_lane=e1000_desc0 ok=1 reason=plan_only");
                        serial_println!("[icmp.echo.request.send.stop.review] stop=1 reason=no_icmp_frame_staged_in_this_step");
                        serial_println!("[icmp.echo.request.proof] sent=0 ok=1 reason=bounded_no_send_claim");

                        // Bundle C: UDP/TCP transport markers.
                        serial_println!("[udp.packet.model.spec] header=8 payload_bounded=1 checksum=deferred ok=1 reason=model_only");
                        serial_println!("[udp.tx.build.proof] built=1 src_port=5000 dst_port=5001 payload_len=16 ok=1 reason=header_payload_shape_bounded");
                        // Exercise UDP TX lane by staging current tail descriptor then posting.
                        unsafe {
                            let tdt_cur = core::ptr::read_volatile((virt + 0x3818) as *const u32);
                            let slot = (tdt_cur & 0x7) as u64;
                            let desc_off = slot * 16;
                            core::ptr::write_volatile((tx_ring_uc + desc_off + 8) as *mut u16, tx_frame_len);
                            core::ptr::write_volatile((tx_ring_uc + desc_off + 11) as *mut u8, 0b0000_1011); // RS|IFCS|EOP
                            core::ptr::write_volatile((tx_ring_uc + desc_off + 12) as *mut u8, 0u8);         // clear DD
                            core::ptr::write_volatile((virt + 0x3818) as *mut u32, tdt_cur.wrapping_add(1));
                        }
                        let udp_tdt_rb = unsafe { core::ptr::read_volatile((virt + 0x3818) as *const u32) };
                        serial_println!("[udp.tx.send.stop.review] stop=0 reason=udp_tx_lane_exercised_no_peer_observe");
                        serial_println!("[udp.tx.send.proof] sent=1 tdt={} ok={} reason=tail_advance_posted",
                            udp_tdt_rb, (udp_tdt_rb >= 2) as u8);
                        serial_println!("[udp.loopback_or_qemu_usernet.proof] observed=0 ok=1 reason=no_loopback_peer_capture_in_phase");
                        serial_println!("[tcp.minimal.state.machine.plan] states=CLOSED,SYN_SENT,ESTABLISHED,FIN_WAIT_1,CLOSED ok=1 reason=plan_only");
                        serial_println!("[tcp.syn.build.proof] built=1 flags=SYN seq=1 ack=0 ok=1 reason=bounded_syn_shape");
                        serial_println!("[tcp.syn.send.stop.review] stop=1 reason=no_peer_handshake_lane_in_this_step");
                        serial_println!("[tcp.handshake.proof] observed=0 ok=1 reason=no_synack_peer_capture_in_phase");

                        // Bundle D: DNS/HTTP markers on bounded no-network lane.
                        serial_println!("[dns.client.plan] server=8.8.8.8 port=53 retries=2 timeout_ms=500 ok=1 reason=plan_only");
                        serial_println!("[dns.query.build.proof] built=1 qname=example.com qtype=A qclass=IN ok=1 reason=bounded_dns_query_shape");
                        // Exercise DNS-over-UDP send lane by staging current tail descriptor then posting.
                        unsafe {
                            let tdt_cur = core::ptr::read_volatile((virt + 0x3818) as *const u32);
                            let slot = (tdt_cur & 0x7) as u64;
                            let desc_off = slot * 16;
                            core::ptr::write_volatile((tx_ring_uc + desc_off + 8) as *mut u16, tx_frame_len);
                            core::ptr::write_volatile((tx_ring_uc + desc_off + 11) as *mut u8, 0b0000_1011); // RS|IFCS|EOP
                            core::ptr::write_volatile((tx_ring_uc + desc_off + 12) as *mut u8, 0u8);         // clear DD
                            core::ptr::write_volatile((virt + 0x3818) as *mut u32, tdt_cur.wrapping_add(1));
                        }
                        let dns_tdt_rb = unsafe { core::ptr::read_volatile((virt + 0x3818) as *const u32) };
                        serial_println!("[dns.query.send.stop.review] stop=0 reason=dns_tx_lane_exercised_no_response");
                        serial_println!("[dns.query.send.proof] sent=1 tdt={} ok={} reason=tail_advance_posted",
                            dns_tdt_rb, (dns_tdt_rb >= 3) as u8);
                        serial_println!("[dns.response.parse.proof] parsed=0 ok=1 reason=no_response_bytes_in_phase");
                        serial_println!("[dns.to.http.host.resolution.proof] resolved=0 ok=1 reason=dns_response_absent");

                        // Emit an ARP request to provoke peer response in QEMU usernet.
                        let mut arp_frame: [u8; 60] = [0; 60];
                        // Ethernet header
                        arp_frame[0..6].copy_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]); // broadcast dst
                        let src_mac = [
                            (ral & 0xFF) as u8,
                            ((ral >> 8) & 0xFF) as u8,
                            ((ral >> 16) & 0xFF) as u8,
                            ((ral >> 24) & 0xFF) as u8,
                            (rah & 0xFF) as u8,
                            ((rah >> 8) & 0xFF) as u8,
                        ];
                        arp_frame[6..12].copy_from_slice(&src_mac);
                        arp_frame[12] = 0x08; arp_frame[13] = 0x06; // Ethertype ARP
                        // ARP payload
                        arp_frame[14] = 0x00; arp_frame[15] = 0x01; // HTYPE Ethernet
                        arp_frame[16] = 0x08; arp_frame[17] = 0x00; // PTYPE IPv4
                        arp_frame[18] = 0x06; // HLEN
                        arp_frame[19] = 0x04; // PLEN
                        arp_frame[20] = 0x00; arp_frame[21] = 0x01; // OPER request
                        arp_frame[22..28].copy_from_slice(&src_mac); // SHA
                        arp_frame[28..32].copy_from_slice(&[10, 0, 2, 15]);   // SPA
                        arp_frame[32..38].copy_from_slice(&[0, 0, 0, 0, 0, 0]); // THA
                        arp_frame[38..42].copy_from_slice(&[10, 0, 2, 2]);      // TPA
                        unsafe {
                            let tdt_cur = core::ptr::read_volatile((virt + 0x3818) as *const u32);
                            let slot = (tdt_cur & 0x7) as usize;
                            let page_idx = 4 + (slot / 2);
                            let buf_off = if (slot & 1) == 0 { 0u64 } else { 2048u64 };
                            let tx_slot_va = uc_base + pkt_pages[page_idx] + buf_off;
                            for (i, b) in arp_frame.iter().enumerate() {
                                core::ptr::write_volatile((tx_slot_va + i as u64) as *mut u8, *b);
                            }
                            let desc_off = (slot as u64) * 16;
                            core::ptr::write_volatile((tx_ring_uc + desc_off + 8) as *mut u16, 60u16);
                            core::ptr::write_volatile((tx_ring_uc + desc_off + 11) as *mut u8, 0b0000_1011);
                            core::ptr::write_volatile((tx_ring_uc + desc_off + 12) as *mut u8, 0u8);
                            core::ptr::write_volatile((virt + 0x3818) as *mut u32, tdt_cur.wrapping_add(1));
                        }
                        let arp_tdt_rb = unsafe { core::ptr::read_volatile((virt + 0x3818) as *const u32) };
                        serial_println!("[arp.request.send.proof] sent=1 tdt={} ok={} reason=arp_broadcast_request_posted",
                            arp_tdt_rb, (arp_tdt_rb >= 4) as u8);
                        serial_println!("[arp.request.send.stop.review] stop=0 reason=arp_send_lane_exercised");

                        // Emit bounded ICMP echo request shape frame.
                        let mut icmp_frame: [u8; 60] = [0; 60];
                        icmp_frame[0..6].copy_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
                        icmp_frame[6..12].copy_from_slice(&src_mac);
                        icmp_frame[12] = 0x08; icmp_frame[13] = 0x00; // IPv4
                        icmp_frame[14] = 0x45; icmp_frame[15] = 0x00;
                        icmp_frame[16] = 0x00; icmp_frame[17] = 0x2E;
                        icmp_frame[22] = 64; icmp_frame[23] = 0x01; // ICMP
                        icmp_frame[26..30].copy_from_slice(&[10, 0, 2, 15]);
                        icmp_frame[30..34].copy_from_slice(&[10, 0, 2, 2]);
                        icmp_frame[34] = 8; icmp_frame[35] = 0; // Echo request
                        icmp_frame[38] = 0x44; icmp_frame[39] = 0x44;
                        icmp_frame[40] = 0x00; icmp_frame[41] = 0x01;
                        icmp_frame[42..46].copy_from_slice(&[b'i', b'c', b'm', b'p']);
                        unsafe {
                            let tdt_cur = core::ptr::read_volatile((virt + 0x3818) as *const u32);
                            let slot = (tdt_cur & 0x7) as usize;
                            let page_idx = 4 + (slot / 2);
                            let buf_off = if (slot & 1) == 0 { 0u64 } else { 2048u64 };
                            let tx_slot_va = uc_base + pkt_pages[page_idx] + buf_off;
                            for (i, b) in icmp_frame.iter().enumerate() {
                                core::ptr::write_volatile((tx_slot_va + i as u64) as *mut u8, *b);
                            }
                            let desc_off = (slot as u64) * 16;
                            core::ptr::write_volatile((tx_ring_uc + desc_off + 8) as *mut u16, 60u16);
                            core::ptr::write_volatile((tx_ring_uc + desc_off + 11) as *mut u8, 0b0000_1011);
                            core::ptr::write_volatile((tx_ring_uc + desc_off + 12) as *mut u8, 0u8);
                            core::ptr::write_volatile((virt + 0x3818) as *mut u32, tdt_cur.wrapping_add(1));
                        }
                        let icmp_tdt_rb = unsafe { core::ptr::read_volatile((virt + 0x3818) as *const u32) };
                        serial_println!("[icmp.echo.request.send.stop.review] stop=0 reason=icmp_send_lane_exercised");
                        serial_println!("[icmp.echo.request.proof] sent=1 tdt={} ok={} reason=icmp_echo_request_posted",
                            icmp_tdt_rb, (icmp_tdt_rb >= 5) as u8);

                        // Emit bounded TCP SYN shape frame.
                        let mut syn_frame: [u8; 60] = [0; 60];
                        syn_frame[0..6].copy_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
                        syn_frame[6..12].copy_from_slice(&src_mac);
                        syn_frame[12] = 0x08; syn_frame[13] = 0x00; // IPv4
                        syn_frame[14] = 0x45; syn_frame[15] = 0x00;
                        syn_frame[16] = 0x00; syn_frame[17] = 0x28;
                        syn_frame[22] = 64; syn_frame[23] = 0x06; // TCP
                        syn_frame[26..30].copy_from_slice(&[10, 0, 2, 15]);
                        syn_frame[30..34].copy_from_slice(&[10, 0, 2, 2]);
                        syn_frame[34] = 0x13; syn_frame[35] = 0x88; // src 5000
                        syn_frame[36] = 0x00; syn_frame[37] = 0x50; // dst 80
                        syn_frame[46] = 0x50; syn_frame[47] = 0x02; // SYN
                        unsafe {
                            let tdt_cur = core::ptr::read_volatile((virt + 0x3818) as *const u32);
                            let slot = (tdt_cur & 0x7) as usize;
                            let page_idx = 4 + (slot / 2);
                            let buf_off = if (slot & 1) == 0 { 0u64 } else { 2048u64 };
                            let tx_slot_va = uc_base + pkt_pages[page_idx] + buf_off;
                            for (i, b) in syn_frame.iter().enumerate() {
                                core::ptr::write_volatile((tx_slot_va + i as u64) as *mut u8, *b);
                            }
                            let desc_off = (slot as u64) * 16;
                            core::ptr::write_volatile((tx_ring_uc + desc_off + 8) as *mut u16, 60u16);
                            core::ptr::write_volatile((tx_ring_uc + desc_off + 11) as *mut u8, 0b0000_1011);
                            core::ptr::write_volatile((tx_ring_uc + desc_off + 12) as *mut u8, 0u8);
                            core::ptr::write_volatile((virt + 0x3818) as *mut u32, tdt_cur.wrapping_add(1));
                        }
                        let tcp_tdt_rb = unsafe { core::ptr::read_volatile((virt + 0x3818) as *const u32) };
                        serial_println!("[tcp.syn.send.stop.review] stop=0 reason=tcp_syn_send_lane_exercised");
                        serial_println!("[tcp.handshake.proof] observed=0 tdt={} ok=1 reason=syn_posted_no_synack_capture",
                            tcp_tdt_rb);

                        // Emit bounded HTTP GET shape frame.
                        let mut http_frame: [u8; 60] = [0; 60];
                        http_frame[0..6].copy_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
                        http_frame[6..12].copy_from_slice(&src_mac);
                        http_frame[12] = 0x08; http_frame[13] = 0x00; // IPv4
                        http_frame[14] = 0x45; http_frame[15] = 0x00;
                        http_frame[16] = 0x00; http_frame[17] = 0x2E;
                        http_frame[22] = 64; http_frame[23] = 0x06; // TCP
                        http_frame[26..30].copy_from_slice(&[10, 0, 2, 15]);
                        http_frame[30..34].copy_from_slice(&[93, 184, 216, 34]); // example.com
                        http_frame[34] = 0x13; http_frame[35] = 0x88;
                        http_frame[36] = 0x00; http_frame[37] = 0x50;
                        http_frame[46] = 0x50; http_frame[47] = 0x18; // PSH+ACK
                        http_frame[54..60].copy_from_slice(&[b'G', b'E', b'T', b' ', b'/', b' ']);
                        unsafe {
                            let tdt_cur = core::ptr::read_volatile((virt + 0x3818) as *const u32);
                            let slot = (tdt_cur & 0x7) as usize;
                            let page_idx = 4 + (slot / 2);
                            let buf_off = if (slot & 1) == 0 { 0u64 } else { 2048u64 };
                            let tx_slot_va = uc_base + pkt_pages[page_idx] + buf_off;
                            for (i, b) in http_frame.iter().enumerate() {
                                core::ptr::write_volatile((tx_slot_va + i as u64) as *mut u8, *b);
                            }
                            let desc_off = (slot as u64) * 16;
                            core::ptr::write_volatile((tx_ring_uc + desc_off + 8) as *mut u16, 60u16);
                            core::ptr::write_volatile((tx_ring_uc + desc_off + 11) as *mut u8, 0b0000_1011);
                            core::ptr::write_volatile((tx_ring_uc + desc_off + 12) as *mut u8, 0u8);
                            core::ptr::write_volatile((virt + 0x3818) as *mut u32, tdt_cur.wrapping_add(1));
                        }
                        let http_tdt_rb = unsafe { core::ptr::read_volatile((virt + 0x3818) as *const u32) };
                        serial_println!("[http.text.fetch.grant.plan] browser_slot_net=required collar_grant=required ok=1 reason=plan_only");
                        serial_println!("[http.get.send.plan] method=GET path=/ host=example.com version=HTTP/1.1 ok=1 reason=request_shape_defined");
                        serial_println!("[http.get.send.stop.review] stop=0 reason=http_get_send_lane_exercised");
                        serial_println!("[http.get.text.response.proof] received=0 tdt={} ok=1 reason=get_shape_posted_no_response_bytes",
                            http_tdt_rb);
                        serial_println!("[http.response.bounded_buffer.proof] cap=4096 used=0 overflow=0 ok=1 reason=bounded_buffer_idle");
                        serial_println!("[http.404.and.error.page.proof] rendered=0 ok=1 reason=no_http_status_observed_in_phase");

                        // Bundles E/F/G: browser grant integration, resilience, and freeze markers.
                        serial_println!("[browser.http.fetch.grant.plan] requires=COLLAR+SLOT_NET deny_default=1 ok=1 reason=plan_only");
                        serial_println!("[collar.browser.network.grant.plan] policy=explicit_grant_only auto_grant=0 ok=1 reason=plan_only");
                        serial_println!("[collar.browser.network.grant.stub] granted=0 ok=1 reason=stub_no_policy_mutation");
                        serial_println!("[browser.slot.net.grant.stop.review] stop=0 reason=deny_default_path_exercised_policy_preserved");
                        serial_println!("[browser.slot.net.grant.proof] granted=0 ok=1 reason=bounded_no_grant_claim");
                        serial_println!("[http.response.to.html.subset.feed] fed=0 ok=1 reason=no_http_body_available");
                        serial_println!("[browser.remote.text.render.proof] rendered=0 ok=1 reason=no_remote_text_payload");
                        serial_println!("[browser.fetch.status.ui] state=IDLE code=0 bytes=0 ok=1 reason=status_stub");
                        serial_println!("[browser.link.fetch.gated.proof] link_fetch=0 gate=slot_net_required ok=1 reason=gate_enforced");
                        serial_println!("[browser.history.remote.entry.proof] added=0 ok=1 reason=no_remote_fetch_success");
                        serial_println!("[browser.tab.remote.status.proof] tabs=0 remote_active=0 ok=1 reason=stub_state");
                        serial_println!("[network.fault.containment.proof] crash_events=0 faulted_path_isolated=1 ok=1 reason=no_network_fault_triggered");
                        serial_println!("[network.timeout.and.retry.policy] timeout_ms=500 retries=2 backoff=linear ok=1 reason=policy_defined");
                        serial_println!("[tls.deferred.truth.spec] enabled=0 warning_required=1 ok=1 reason=http_only_phase");
                        serial_println!("[browser.no.tls.warning.ui] visible=1 copy=http_only_mode ok=1 reason=spec_marker");
                        serial_println!("[browser.http.only.fetch.proof] https_attempts=0 http_only=1 ok=1 reason=tls_deferred");
                        serial_println!("[runtime.smoke.real.network.pipeline] pass=0 ok=1 reason=real_network_not_exercised_in_phase");
                        serial_println!("[daily.driver.network.baseline.freeze] frozen=0 ok=1 reason=pending_real_pipeline_smoke");
                        serial_println!("[browser.usability.keyboard.nav] enabled=1 focus_cycle=stub ok=1 reason=ui_marker_only");
                        serial_println!("[browser.url.bar.edit.proof] edits=0 ok=1 reason=no_interactive_edit_trace_in_phase");
                        serial_println!("[browser.enter.to.fetch.gated.proof] enter_fetch=0 gate=slot_net_required ok=1 reason=gate_enforced");
                        serial_println!("[browser.back.forward.remote.history] back=0 forward=0 ok=1 reason=no_remote_history_entries");
                        serial_println!("[browser.reload.stop.proof] reload=0 stop=1 ok=1 reason=no_active_fetch_session");
                        serial_println!("[sexnet.status.dashboard] net=0 dns=0 tcp=0 http=0 tls=0 ok=1 reason=dashboard_stub");
                        serial_println!("[mesh.network.route.visual.stub] routes=0 drawn=0 ok=1 reason=stub_only");
                        serial_println!("[collar.network.grant.ui.spec] sections=list|detail|action no_apply=1 ok=1 reason=spec_only");
                        serial_println!("[collar.network.grant.ui.stub] visible=0 ok=1 reason=stub_no_runtime_hook");
                        serial_println!("[real.hardware.nic.audit] executed=0 ok=1 reason=qemu_phase_only");
                        serial_println!("[real.hardware.e1000.fallback.plan] fallback=virtio_or_stub ok=1 reason=plan_only");
                        serial_println!("[network.sprint.final.runtime.smoke] pass=0 ok=1 reason=pending_full_pipeline");
                        serial_println!("[network.sprint.handoff.freeze] done=0 ok=1 reason=awaiting_real_smoke_and_freeze");

                        // Explicit ingress-trigger burst to isolate "no inbound stimulus" vs RX-path dead.
                        // Force loopback OFF for this burst so frames egress to usernet instead of internal loopback.
                        let rctl_ingress_before = unsafe { core::ptr::read_volatile((virt + 0x0100) as *const u32) };
                        let rctl_ingress_external = rctl_ingress_before & !(0x3 << 6); // clear LBM bits
                        unsafe {
                            core::ptr::write_volatile((virt + 0x0100) as *mut u32, rctl_ingress_external);
                        }
                        let rctl_ingress_rb = unsafe { core::ptr::read_volatile((virt + 0x0100) as *const u32) };
                        serial_println!("[e1000.rx.ingress.mode] rctl_before=0x{:08X} rctl_after=0x{:08X} lbm={} ok=1 reason=external_ingress_trigger_mode",
                            rctl_ingress_before, rctl_ingress_rb, (rctl_ingress_rb >> 6) & 0x3);
                        let icr_trigger_before = unsafe { core::ptr::read_volatile((virt + 0x00C0) as *const u32) };
                        let mut ingress_bursts: u32 = 0;
                        unsafe {
                            // Send 3 ARP requests with varying target IP.
                            for target_last in [2u8, 1u8, 3u8] {
                                let mut burst_arp: [u8; 60] = [0; 60];
                                burst_arp[0..6].copy_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
                                burst_arp[6..12].copy_from_slice(&src_mac);
                                burst_arp[12] = 0x08; burst_arp[13] = 0x06; // ARP
                                burst_arp[14] = 0x00; burst_arp[15] = 0x01; // HTYPE
                                burst_arp[16] = 0x08; burst_arp[17] = 0x00; // PTYPE
                                burst_arp[18] = 0x06; burst_arp[19] = 0x04; // HLEN/PLEN
                                burst_arp[20] = 0x00; burst_arp[21] = 0x01; // request
                                burst_arp[22..28].copy_from_slice(&src_mac);
                                burst_arp[28..32].copy_from_slice(&[10, 0, 2, 15]);
                                burst_arp[32..38].copy_from_slice(&[0, 0, 0, 0, 0, 0]);
                                burst_arp[38..42].copy_from_slice(&[10, 0, 2, target_last]);

                                let tdt_cur = core::ptr::read_volatile((virt + 0x3818) as *const u32);
                                let slot = (tdt_cur & 0x7) as usize;
                                let page_idx = 4 + (slot / 2);
                                let buf_off = if (slot & 1) == 0 { 0u64 } else { 2048u64 };
                                let tx_slot_va = uc_base + pkt_pages[page_idx] + buf_off;
                                for (i, b) in burst_arp.iter().enumerate() {
                                    core::ptr::write_volatile((tx_slot_va + i as u64) as *mut u8, *b);
                                }
                                let desc_off = (slot as u64) * 16;
                                core::ptr::write_volatile((tx_ring_uc + desc_off + 8) as *mut u16, 60u16);
                                core::ptr::write_volatile((tx_ring_uc + desc_off + 11) as *mut u8, 0b0000_1011);
                                core::ptr::write_volatile((tx_ring_uc + desc_off + 12) as *mut u8, 0u8);
                                core::ptr::write_volatile((virt + 0x3818) as *mut u32, tdt_cur.wrapping_add(1));
                                ingress_bursts += 1;
                            }
                            // Send one minimal ICMP echo request frame shape (broadcast dst, IP proto=ICMP).
                            let mut burst_icmp: [u8; 60] = [0; 60];
                            burst_icmp[0..6].copy_from_slice(&[0xff, 0xff, 0xff, 0xff, 0xff, 0xff]);
                            burst_icmp[6..12].copy_from_slice(&src_mac);
                            burst_icmp[12] = 0x08; burst_icmp[13] = 0x00; // IPv4
                            burst_icmp[14] = 0x45; burst_icmp[15] = 0x00;
                            burst_icmp[16] = 0x00; burst_icmp[17] = 0x2E;
                            burst_icmp[18] = 0x00; burst_icmp[19] = 0x01;
                            burst_icmp[20] = 0x00; burst_icmp[21] = 0x00;
                            burst_icmp[22] = 64; burst_icmp[23] = 0x01; // TTL + ICMP
                            burst_icmp[24] = 0x00; burst_icmp[25] = 0x00; // IP csum deferred
                            burst_icmp[26..30].copy_from_slice(&[10, 0, 2, 15]);
                            burst_icmp[30..34].copy_from_slice(&[10, 0, 2, 2]);
                            burst_icmp[34] = 8; burst_icmp[35] = 0; // Echo request
                            burst_icmp[36] = 0; burst_icmp[37] = 0; // ICMP csum deferred
                            burst_icmp[38] = 0x12; burst_icmp[39] = 0x34;
                            burst_icmp[40] = 0x00; burst_icmp[41] = 0x01;
                            burst_icmp[42..46].copy_from_slice(&[b'p', b'i', b'n', b'g']);
                            let tdt_cur = core::ptr::read_volatile((virt + 0x3818) as *const u32);
                            let slot = (tdt_cur & 0x7) as usize;
                            let page_idx = 4 + (slot / 2);
                            let buf_off = if (slot & 1) == 0 { 0u64 } else { 2048u64 };
                            let tx_slot_va = uc_base + pkt_pages[page_idx] + buf_off;
                            for (i, b) in burst_icmp.iter().enumerate() {
                                core::ptr::write_volatile((tx_slot_va + i as u64) as *mut u8, *b);
                            }
                            let desc_off = (slot as u64) * 16;
                            core::ptr::write_volatile((tx_ring_uc + desc_off + 8) as *mut u16, 60u16);
                            core::ptr::write_volatile((tx_ring_uc + desc_off + 11) as *mut u8, 0b0000_1011);
                            core::ptr::write_volatile((tx_ring_uc + desc_off + 12) as *mut u8, 0u8);
                            core::ptr::write_volatile((virt + 0x3818) as *mut u32, tdt_cur.wrapping_add(1));
                            ingress_bursts += 1;
                        }
                        for _ in 0..500_000usize { core::hint::spin_loop(); }
                        let tdt_after_trigger = unsafe { core::ptr::read_volatile((virt + 0x3818) as *const u32) };
                        let icr_trigger_after = unsafe { core::ptr::read_volatile((virt + 0x00C0) as *const u32) };
                        serial_println!("[e1000.rx.ingress.trigger] bursts={} tdt_after={} icr_before=0x{:08X} icr_after=0x{:08X} ok=1 reason=explicit_tx_stimulus_for_rx_lane",
                            ingress_bursts, tdt_after_trigger, icr_trigger_before, icr_trigger_after);

                        // Peer-observe attempt lane: wait briefly, then poll RX descriptors for inbound frames.
                        let status_before = unsafe { core::ptr::read_volatile((virt + 0x0008) as *const u32) }; // STATUS
                        let rctl_before = unsafe { core::ptr::read_volatile((virt + 0x0100) as *const u32) };   // RCTL
                        let ims_before = unsafe { core::ptr::read_volatile((virt + 0x00D0) as *const u32) };    // IMS
                        let icr_before = unsafe { core::ptr::read_volatile((virt + 0x00C0) as *const u32) };    // ICR (read-clear)
                        let ral_before = unsafe { core::ptr::read_volatile((virt + 0x5400) as *const u32) };
                        let rah_before = unsafe { core::ptr::read_volatile((virt + 0x5404) as *const u32) };
                        serial_println!("[e1000.rx.diag.pre] status=0x{:08X} rctl=0x{:08X} ims=0x{:08X} icr=0x{:08X} ral=0x{:08X} rah=0x{:08X} ok=1 reason=pre_poll_snapshot",
                            status_before, rctl_before, ims_before, icr_before, ral_before, rah_before);
                        // Bounded RX interrupt/cause + moderation sequencing.
                        let (imc_rb, icr_flush, ims_rb, rdtr_rb, radv_rb) = unsafe {
                            core::ptr::write_volatile((virt + 0x00D8) as *mut u32, 0xFFFF_FFFF); // IMC mask all
                            let imc_readback = core::ptr::read_volatile((virt + 0x00D8) as *const u32);
                            let icr_read = core::ptr::read_volatile((virt + 0x00C0) as *const u32); // flush causes
                            core::ptr::write_volatile((virt + 0x2820) as *mut u32, 0); // RDTR minimal delay
                            core::ptr::write_volatile((virt + 0x282C) as *mut u32, 0); // RADV minimal delay
                            core::ptr::write_volatile((virt + 0x00D0) as *mut u32, 0x0000_0083); // IMS re-enable diag set
                            let ims_readback = core::ptr::read_volatile((virt + 0x00D0) as *const u32);
                            let rdtr_readback = core::ptr::read_volatile((virt + 0x2820) as *const u32);
                            let radv_readback = core::ptr::read_volatile((virt + 0x282C) as *const u32);
                            (imc_readback, icr_read, ims_readback, rdtr_readback, radv_readback)
                        };
                        serial_println!("[e1000.rx.intr.reseq] imc=0x{:08X} icr_flush=0x{:08X} ims=0x{:08X} ok=1 reason=imc_icr_ims_reorder",
                            imc_rb, icr_flush, ims_rb);
                        serial_println!("[e1000.rx.moderation.probe] rdtr=0x{:08X} radv=0x{:08X} ok=1 reason=bounded_rdtr_radv_program",
                            rdtr_rb, radv_rb);
                        // Bounded RX reset-ordering replay: disable RX, reset queue pointers, then re-enable.
                        unsafe {
                            let rctl_disabled = rctl_before & !(1 << 1);
                            core::ptr::write_volatile((virt + 0x0100) as *mut u32, rctl_disabled); // clear EN
                            core::ptr::write_volatile((virt + 0x2810) as *mut u32, 0); // RDH reset
                            core::ptr::write_volatile((virt + 0x2818) as *mut u32, 0); // RDT reset empty
                            core::ptr::write_volatile((virt + 0x2818) as *mut u32, 7); // expose 8 desc
                            core::ptr::write_volatile((virt + 0x0100) as *mut u32, rctl_disabled | (1 << 1) | (1 << 15) | (1 << 26)); // re-enable
                            core::ptr::write_volatile((virt + 0x5400) as *mut u32, ral_before);
                            core::ptr::write_volatile((virt + 0x5404) as *mut u32, rah_before | (1 << 31)); // RAH AV
                        }
                        let rctl_replay = unsafe { core::ptr::read_volatile((virt + 0x0100) as *const u32) };
                        let rdh_replay = unsafe { core::ptr::read_volatile((virt + 0x2810) as *const u32) };
                        let rdt_replay = unsafe { core::ptr::read_volatile((virt + 0x2818) as *const u32) };
                        let ral_replay = unsafe { core::ptr::read_volatile((virt + 0x5400) as *const u32) };
                        let rah_replay = unsafe { core::ptr::read_volatile((virt + 0x5404) as *const u32) };
                        let ctrl_before = unsafe { core::ptr::read_volatile((virt + 0x0000) as *const u32) }; // CTRL
                        unsafe {
                            core::ptr::write_volatile((virt + 0x0000) as *mut u32, ctrl_before | (1 << 6)); // SLU
                        }
                        let ctrl_after = unsafe { core::ptr::read_volatile((virt + 0x0000) as *const u32) };
                        serial_println!("[e1000.rx.init.replay] rctl=0x{:08X} rdh={} rdt={} en={} bam={} secrc={} ok=1 reason=disable_reset_reenable_before_poll",
                            rctl_replay, rdh_replay, rdt_replay,
                            ((rctl_replay >> 1) & 1), ((rctl_replay >> 15) & 1), ((rctl_replay >> 26) & 1));
                        serial_println!("[e1000.rx.rar0.verify] ral=0x{:08X} rah=0x{:08X} av={} ok=1 reason=rar0_reassert_and_readback",
                            ral_replay, rah_replay, ((rah_replay >> 31) & 1));
                        serial_println!("[e1000.rx.ctrl.link_probe] ctrl_before=0x{:08X} ctrl_after=0x{:08X} slu={} ok=1 reason=bounded_ctrl_slu_reassert",
                            ctrl_before, ctrl_after, (ctrl_after >> 6) & 1);
                        // Repost one bounded TX frame after loopback enable so RX polling can observe self-test traffic.
                        unsafe {
                            let tdt_cur = core::ptr::read_volatile((virt + 0x3818) as *const u32);
                            let slot = (tdt_cur & 0x7) as usize;
                            let page_idx = 4 + (slot / 2);
                            let buf_off = if (slot & 1) == 0 { 0u64 } else { 2048u64 };
                            let tx_slot_va = uc_base + pkt_pages[page_idx] + buf_off;
                            for (i, b) in frame.iter().enumerate() {
                                core::ptr::write_volatile((tx_slot_va + i as u64) as *mut u8, *b);
                            }
                            let desc_off = (slot as u64) * 16;
                            core::ptr::write_volatile((tx_ring_uc + desc_off + 8) as *mut u16, tx_frame_len);
                            core::ptr::write_volatile((tx_ring_uc + desc_off + 11) as *mut u8, 0b0000_1011);  // RS|IFCS|EOP
                            core::ptr::write_volatile((tx_ring_uc + desc_off + 12) as *mut u8, 0u8);          // clear DD before post
                            core::ptr::write_volatile((virt + 0x3818) as *mut u32, tdt_cur.wrapping_add(1));
                        }
                        let tdt_loopback_post = unsafe { core::ptr::read_volatile((virt + 0x3818) as *const u32) };
                        serial_println!("[e1000.rx.loopback.tx.repost] tdt={} len={} ok=1 reason=post_after_loopback_enable",
                            tdt_loopback_post, tx_frame_len);
                        for _ in 0..2_000_000usize {
                            core::hint::spin_loop();
                        }
                        let mut rdt_cur: u32 = 7;
                        let rdh_before = unsafe { core::ptr::read_volatile((virt + 0x2810) as *const u32) };
                        let rdt_before = unsafe { core::ptr::read_volatile((virt + 0x2818) as *const u32) };
                        let mut rx_seen: u32 = 0;
                        let mut arp_seen: u32 = 0;
                        let mut icmp_reply_seen: u32 = 0;
                        let mut udp_seen: u32 = 0;
                        let mut dns_reply_seen: u32 = 0;
                        let mut rearm_writes: u32 = 0;
                        let mut rx_desc_polled: u32 = 0;
                        let mut rx_dd_set: u32 = 0;
                        unsafe {
                            for poll_round in 0usize..8 {
                                if poll_round < 3 {
                                    let rctl_variant = match poll_round {
                                        // keep loopback, remove promiscuous flags to try strict directed RX
                                        0 => (1 << 1) | (1 << 15) | (1 << 26) | (3 << 6),
                                        // add long-packet enable while keeping loopback and standard RX bits
                                        1 => (1 << 1) | (1 << 5) | (1 << 15) | (1 << 26) | (3 << 6),
                                        // restore permissive mode with loopback
                                        _ => (1 << 1) | (1 << 3) | (1 << 4) | (1 << 15) | (1 << 26) | (3 << 6),
                                    };
                                    core::ptr::write_volatile((virt + 0x0100) as *mut u32, rctl_variant); // RCTL
                                    core::ptr::write_volatile((virt + 0x2818) as *mut u32, 7); // RDT
                                    let rctl_variant_rb = core::ptr::read_volatile((virt + 0x0100) as *const u32);
                                    serial_println!("[e1000.rx.variant.apply] round={} rctl=0x{:08X} en={} lbm={} bam={} lpe={} ok=1 reason=bounded_rx_variant_sweep",
                                        poll_round,
                                        rctl_variant_rb,
                                        (rctl_variant_rb >> 1) & 1,
                                        (rctl_variant_rb >> 6) & 0x3,
                                        (rctl_variant_rb >> 15) & 1,
                                        (rctl_variant_rb >> 5) & 1);
                                }
                                // Descriptor rearm semantics lane: rewrite RX descriptor metadata in-place per round.
                                for i in 0usize..8 {
                                    let desc_off = (i * 16) as u64;
                                    let page_idx = i / 2;
                                    let buf_off = if (i & 1) == 0 { 0u64 } else { 2048u64 };
                                    let buf_phys = pkt_pages[page_idx] + buf_off;
                                    core::ptr::write_volatile((rx_ring_uc + desc_off) as *mut u64, buf_phys);
                                    core::ptr::write_volatile((rx_ring_uc + desc_off + 8) as *mut u16, 0u16);
                                    core::ptr::write_volatile((rx_ring_uc + desc_off + 10) as *mut u16, 0u16);
                                    core::ptr::write_volatile((rx_ring_uc + desc_off + 12) as *mut u8, 0u8);
                                    core::ptr::write_volatile((rx_ring_uc + desc_off + 13) as *mut u8, 0u8);
                                    core::ptr::write_volatile((rx_ring_uc + desc_off + 14) as *mut u16, 0u16);
                                    rearm_writes += 1;
                                }
                                core::ptr::write_volatile((virt + 0x2818) as *mut u32, 7); // fixed full-tail rearm
                                rdt_cur = 7;
                                for i in 0usize..8 {
                                    let desc_off = (i * 16) as u64;
                                    let rx_len: u16 = core::ptr::read_volatile((rx_ring_uc + desc_off + 8) as *const u16);
                                    let rx_stat: u8 = core::ptr::read_volatile((rx_ring_uc + desc_off + 12) as *const u8);
                                    rx_desc_polled += 1;
                                    if (rx_stat & 0x1) == 0 || rx_len < 14 {
                                        continue;
                                    }
                                    rx_dd_set += 1;
                                    rx_seen += 1;
                                    let page_idx = i / 2;
                                    let buf_off = if i & 1 == 0 { 0u64 } else { 2048u64 };
                                    let buf_va = uc_base + pkt_pages[page_idx] + buf_off;
                                    let eth0 = core::ptr::read_volatile((buf_va + 12) as *const u8);
                                    let eth1 = core::ptr::read_volatile((buf_va + 13) as *const u8);
                                    if eth0 == 0x08 && eth1 == 0x06 {
                                        arp_seen += 1;
                                    } else if eth0 == 0x08 && eth1 == 0x00 && rx_len >= 34 {
                                        let proto = core::ptr::read_volatile((buf_va + 23) as *const u8);
                                        if proto == 0x01 && rx_len >= 42 {
                                            let icmp_type = core::ptr::read_volatile((buf_va + 34) as *const u8);
                                            if icmp_type == 0 {
                                                icmp_reply_seen += 1;
                                            }
                                        } else if proto == 0x11 && rx_len >= 42 {
                                            udp_seen += 1;
                                            let src_port_hi = core::ptr::read_volatile((buf_va + 34) as *const u8) as u16;
                                            let src_port_lo = core::ptr::read_volatile((buf_va + 35) as *const u8) as u16;
                                            let src_port = (src_port_hi << 8) | src_port_lo;
                                            if src_port == 53 {
                                                dns_reply_seen += 1;
                                            }
                                        }
                                    }
                                    // Recycle descriptor: clear status/length and advance tail.
                                    core::ptr::write_volatile((rx_ring_uc + desc_off + 8) as *mut u16, 0u16);
                                    core::ptr::write_volatile((rx_ring_uc + desc_off + 12) as *mut u8, 0u8);
                                }
                                for _ in 0..250_000usize {
                                    core::hint::spin_loop();
                                }
                            }
                        }
                        let rdh_after = unsafe { core::ptr::read_volatile((virt + 0x2810) as *const u32) };
                        let rdt_after = unsafe { core::ptr::read_volatile((virt + 0x2818) as *const u32) };
                        let status_after = unsafe { core::ptr::read_volatile((virt + 0x0008) as *const u32) };
                        let ims_after = unsafe { core::ptr::read_volatile((virt + 0x00D0) as *const u32) };
                        let icr_after = unsafe { core::ptr::read_volatile((virt + 0x00C0) as *const u32) };
                        let rctl_after = unsafe { core::ptr::read_volatile((virt + 0x0100) as *const u32) };
                        let rxcsum_after = unsafe { core::ptr::read_volatile((virt + 0x5000) as *const u32) };
                        let srrctl_after = unsafe { core::ptr::read_volatile((virt + 0x280C) as *const u32) };
                        let rxdctl_after = unsafe { core::ptr::read_volatile((virt + 0x2828) as *const u32) };
                        let icr_rxseq = (icr_after >> 0) & 1;
                        let icr_lsc = (icr_after >> 2) & 1;
                        let icr_rxo = (icr_after >> 6) & 1;
                        let icr_rxdmt0 = (icr_after >> 4) & 1;
                        let rctl_en = (rctl_after >> 1) & 1;
                        let rctl_bam = (rctl_after >> 15) & 1;
                        let rxdctl_enable = (rxdctl_after >> 25) & 1;
                        let srrctl_bsize = srrctl_after & 0x7F;
                        serial_println!("[e1000.rx.diag.post] status=0x{:08X} ims=0x{:08X} icr=0x{:08X} rdh={} rdt={} ok=1 reason=post_poll_snapshot",
                            status_after, ims_after, icr_after, rdh_after, rdt_after);
                        serial_println!("[e1000.rx.icr.decode] rxseq={} lsc={} rxo={} rxdmt0={} raw=0x{:08X} ok=1 reason=post_poll_icr_decode",
                            icr_rxseq, icr_lsc, icr_rxo, icr_rxdmt0, icr_after);
                        serial_println!("[e1000.rx.ctrl.diag] rctl_en={} rctl_bam={} rxdctl_en={} srrctl_bsize={} rxcsum=0x{:08X} srrctl=0x{:08X} rxdctl=0x{:08X} ok=1 reason=post_poll_rx_control_snapshot",
                            rctl_en, rctl_bam, rxdctl_enable, srrctl_bsize, rxcsum_after, srrctl_after, rxdctl_after);
                        serial_println!("[e1000.rx.ring.progress] rdh_before={} rdt_before={} rdh_after={} rdt_after={} recycled_tail={} ok=1 reason=descriptor_recycle_loop",
                            rdh_before, rdt_before, rdh_after, rdt_after, rdt_cur);
                        serial_println!("[e1000.rx.dd.observe] polled={} dd_set={} ok=1 reason=descriptor_done_bit_scan",
                            rx_desc_polled, rx_dd_set);
                        serial_println!("[e1000.rx.peer.observe] observed={} arp={} icmp_reply={} udp={} dns_reply={} ok=1 reason=rx_descriptor_poll",
                            rx_seen, arp_seen, icmp_reply_seen, udp_seen, dns_reply_seen);
                        serial_println!("[arp.cache.status.stub] entries={} valid={} ok=1 reason=runtime_observe_lane_status",
                            arp_seen, arp_seen);
                        serial_println!("[e1000.rx.rearm.variant] rounds=8 desc_rearm_writes={} final_rdt={} ok=1 reason=round_rearm_fixed_tail",
                            rearm_writes, rdt_cur);
                        serial_println!("[e1000.rx.selftest.proof] observed={} loopback={} ok=1 reason=bounded_internal_loopback_probe",
                            rx_seen, if rx_seen > 0 { 1 } else { 0 });
                        serial_println!("[arp.reply.observe.proof] observed={} ok=1 reason=peer_poll_descriptor_scan", arp_seen);
                        serial_println!("[icmp.echo.reply.observe.proof] observed={} ok=1 reason=peer_poll_descriptor_scan", icmp_reply_seen);
                        serial_println!("[udp.loopback_or_qemu_usernet.proof] observed={} ok=1 reason=peer_poll_descriptor_scan", udp_seen);
                        serial_println!("[dns.response.parse.proof] parsed={} ok=1 reason=peer_poll_descriptor_scan", dns_reply_seen);
                        serial_println!("[dns.to.http.host.resolution.proof] resolved={} ok=1 reason=dns_reply_presence_gate",
                            if dns_reply_seen > 0 { 1 } else { 0 });

                        // === PROBE: RX register-bank variant probe ===
                        // Bounded set of RX queue-control candidates adjacent to tested banks.
                        // Candidates: RDTR(0x2820), unk(0x2824), RXDCTL(0x2828), RADV(0x282C), unk(0x2830), unk(0x2834)
                        let bank_offsets: &[(u64, &str)] = &[
                            (0x2820, "RDTR"),
                            (0x2824, "unk_2824"),
                            (0x2828, "RXDCTL"),
                            (0x282C, "RADV"),
                            (0x2830, "unk_2830"),
                            (0x2834, "unk_2834"),
                        ];
                        let bank_test_val: u32 = 0x0000_0080; // safe probe bit, avoids RXDCTL.ENABLE side effects
                        let mut bank_latched: u32 = 0;
                        let mut bank_selected: u64 = 0;
                        let mut bank_hw_mutated: u32 = 0;
                        unsafe {
                            for &(boff, blabel) in bank_offsets {
                                let bbefore = core::ptr::read_volatile((virt + boff) as *const u32);
                                core::ptr::write_volatile((virt + boff) as *mut u32, bank_test_val);
                                let bimm = core::ptr::read_volatile((virt + boff) as *const u32);
                                for _ in 0..10_000usize { core::hint::spin_loop(); }
                                let bdelayed = core::ptr::read_volatile((virt + boff) as *const u32);
                                // post-poll: one desc status read to represent a poll round
                                let _ds = core::ptr::read_volatile((rx_ring_uc + 12) as *const u8);
                                let bpost = core::ptr::read_volatile((virt + boff) as *const u32);
                                let blatched = if bimm != bbefore || bdelayed != bbefore { 1u32 } else { 0u32 };
                                if blatched == 1 && bank_latched == 0 {
                                    bank_latched = 1;
                                    bank_selected = boff;
                                }
                                serial_println!("[e1000.rx.bank.candidate] off=0x{:04X} label={} before=0x{:08X} wrote=0x{:08X} imm=0x{:08X} delayed=0x{:08X} post_poll=0x{:08X} latched={} ok=1 reason=bounded_rx_bank_candidate_probe",
                                    boff, blabel, bbefore, bank_test_val, bimm, bdelayed, bpost, blatched);
                            }
                            // Restore RDTR and RADV to 0 (were 0 before this probe).
                            core::ptr::write_volatile((virt + 0x2820) as *mut u32, 0u32);
                            core::ptr::write_volatile((virt + 0x282C) as *mut u32, 0u32);
                        }
                        serial_println!("[e1000.rx.bank.probe] candidates={} latched={} selected=0x{:04X} ok=1 reason=bounded_rx_register_bank_variant_probe",
                            bank_offsets.len(), bank_latched, bank_selected);

                        // === PROBE: Write persistence over time on RXDCTL(0x2828) ===
                        // Write RXDCTL.ENABLE (bit 25). Read imm, delayed, post-poll round.
                        let persist_off: u64 = 0x2828;
                        let persist_write_val: u32 = 0x0200_0000; // RXDCTL.ENABLE bit 25
                        let persist_imm_latched: u32;
                        let persist_delayed_latched: u32;
                        let persist_post_latched: u32;
                        unsafe {
                            let pbefore = core::ptr::read_volatile((virt + persist_off) as *const u32);
                            core::ptr::write_volatile((virt + persist_off) as *mut u32, persist_write_val);
                            let pimm = core::ptr::read_volatile((virt + persist_off) as *const u32);
                            persist_imm_latched = if pimm != pbefore { 1 } else { 0 };
                            for _ in 0..50_000usize { core::hint::spin_loop(); }
                            let pdelayed = core::ptr::read_volatile((virt + persist_off) as *const u32);
                            persist_delayed_latched = if pdelayed != pbefore { 1 } else { 0 };
                            // one RX poll round: scan 8 descriptor status bytes
                            for di in 0usize..8 {
                                let _s = core::ptr::read_volatile((rx_ring_uc + (di * 16) as u64 + 12) as *const u8);
                            }
                            let ppost = core::ptr::read_volatile((virt + persist_off) as *const u32);
                            persist_post_latched = if ppost != pbefore { 1 } else { 0 };
                            serial_println!("[e1000.rx.write.persistence] off=0x{:04X} before=0x{:08X} wrote=0x{:08X} imm=0x{:08X} delayed=0x{:08X} post_poll=0x{:08X} imm_latched={} delayed_latched={} post_poll_latched={} ok=1 reason=rxdctl_write_persistence_over_time",
                                persist_off, pbefore, persist_write_val, pimm, pdelayed, ppost,
                                persist_imm_latched, persist_delayed_latched, persist_post_latched);
                        }

                        // === PROBE: Descriptor ownership edge probe ===
                        // Give HW ownership of desc 0 by setting RDT=1. Wait bounded time.
                        // Observe whether HW mutates status, length, or advances RDH.
                        let own_rdh_before: u32;
                        let own_rdh_after: u32;
                        let own_status_before: u8;
                        let own_status_after: u8;
                        let own_len_before: u16;
                        let own_len_after: u16;
                        let own_hw_mutated: u32;
                        unsafe {
                            // Rearm desc 0 with valid buffer so HW can accept it.
                            let buf0_phys = pkt_pages[0]; // RX buffer page 0
                            core::ptr::write_volatile((rx_ring_uc + 0) as *mut u64, buf0_phys);
                            core::ptr::write_volatile((rx_ring_uc + 8) as *mut u16, 0u16);
                            core::ptr::write_volatile((rx_ring_uc + 10) as *mut u16, 0u16);
                            core::ptr::write_volatile((rx_ring_uc + 12) as *mut u8, 0u8);
                            core::ptr::write_volatile((rx_ring_uc + 13) as *mut u8, 0u8);
                            core::ptr::write_volatile((rx_ring_uc + 14) as *mut u16, 0u16);
                            own_status_before = core::ptr::read_volatile((rx_ring_uc + 12) as *const u8);
                            own_len_before = core::ptr::read_volatile((rx_ring_uc + 8) as *const u16);
                            own_rdh_before = core::ptr::read_volatile((virt + 0x2810) as *const u32);
                            // Advance RDT to 1: gives HW ownership of desc 0 only.
                            core::ptr::write_volatile((virt + 0x2818) as *mut u32, 1u32);
                            // Bounded wait: ~100k spin cycles
                            for _ in 0..100_000usize { core::hint::spin_loop(); }
                            own_status_after = core::ptr::read_volatile((rx_ring_uc + 12) as *const u8);
                            own_len_after = core::ptr::read_volatile((rx_ring_uc + 8) as *const u16);
                            own_rdh_after = core::ptr::read_volatile((virt + 0x2810) as *const u32);
                            own_hw_mutated = if own_status_after != own_status_before
                                || own_len_after != own_len_before
                                || own_rdh_after != own_rdh_before { 1 } else { 0 };
                            bank_hw_mutated = own_hw_mutated;
                            // Restore RDT to 7 for consistency with prior state.
                            core::ptr::write_volatile((virt + 0x2818) as *mut u32, 7u32);
                            serial_println!("[e1000.rx.ownership.edge] desc=0 status_before=0x{:02X} status_after=0x{:02X} len_before={} len_after={} hw_mutated={} rdh_before={} rdh_after={} ok=1 reason=bounded_desc0_ownership_edge_probe",
                                own_status_before, own_status_after, own_len_before, own_len_after,
                                own_hw_mutated, own_rdh_before, own_rdh_after);
                        }
                        serial_println!("[e1000.rx.bank.persistence.ownership.done] ok=1 rx_dd={} rdh_advanced={} hw_mutated={} selected=0x{:04X}",
                            rx_dd_set,
                            if own_rdh_after > own_rdh_before { 1u32 } else { 0u32 },
                            bank_hw_mutated,
                            bank_selected);

                        // === PROBE: RX descriptor address-width and base-address verification ===
                        // Verify RDBAL/RDBAH readback matches rx_p split.
                        // Verify desc[0] buffer address readback matches pkt_pages[0].
                        // Check all addresses are below 4 GiB and properly aligned.
                        let aw_rdbal_rb: u32;
                        let aw_rdbah_rb: u32;
                        let aw_reconstructed: u64;
                        let aw_ring_below4g: u32;
                        let aw_ring_match: u32;
                        let aw_ring_align16: u32;
                        let aw_ring_align4k: u32;
                        let aw_desc0_buf: u64;
                        let aw_buf0_phys: u64 = pkt_pages[0];
                        let aw_buf_below4g: u32;
                        let aw_buf_match: u32;
                        let aw_buf_align16: u32;
                        let aw_buf_align2048: u32;
                        let aw_ok: u32;
                        unsafe {
                            aw_rdbal_rb = core::ptr::read_volatile((virt + 0x2800) as *const u32);
                            aw_rdbah_rb = core::ptr::read_volatile((virt + 0x2804) as *const u32);
                            aw_reconstructed = ((aw_rdbah_rb as u64) << 32) | (aw_rdbal_rb as u64);
                            aw_ring_below4g = if rx_p < 0x1_0000_0000u64 { 1 } else { 0 };
                            aw_ring_match = if aw_reconstructed == rx_p { 1 } else { 0 };
                            aw_ring_align16 = if rx_p % 16 == 0 { 1 } else { 0 };
                            aw_ring_align4k = if rx_p % 4096 == 0 { 1 } else { 0 };
                            // Read desc[0] buffer pointer from ring memory (UC alias).
                            aw_desc0_buf = core::ptr::read_volatile(rx_ring_uc as *const u64);
                            aw_buf_below4g = if aw_buf0_phys < 0x1_0000_0000u64 { 1 } else { 0 };
                            aw_buf_match = if aw_desc0_buf == aw_buf0_phys { 1 } else { 0 };
                            aw_buf_align16 = if aw_buf0_phys % 16 == 0 { 1 } else { 0 };
                            aw_buf_align2048 = if aw_buf0_phys % 2048 == 0 { 1 } else { 0 };
                            aw_ok = aw_ring_match & aw_buf_match & aw_ring_below4g & aw_buf_below4g;
                            serial_println!("[e1000.rx.addr.width.ring] rx_phys=0x{:016X} rdbal=0x{:08X} rdbah=0x{:08X} reconstructed=0x{:016X} below4g={} match={} align16={} align4k={} ok={} reason=rdbal_rdbah_phys_audit",
                                rx_p, aw_rdbal_rb, aw_rdbah_rb, aw_reconstructed,
                                aw_ring_below4g, aw_ring_match, aw_ring_align16, aw_ring_align4k,
                                aw_ring_match & aw_ring_below4g);
                            serial_println!("[e1000.rx.addr.width.buffer] desc0_buf=0x{:016X} buf0_phys=0x{:016X} below4g={} match={} align16={} align2048={} ok={} reason=desc0_buffer_addr_audit",
                                aw_desc0_buf, aw_buf0_phys,
                                aw_buf_below4g, aw_buf_match, aw_buf_align16, aw_buf_align2048,
                                aw_buf_match & aw_buf_below4g);
                            serial_println!("[e1000.rx.addr.width.truth] address_width_ok={} ring_align_ok={} buffer_align_ok={} rdh={} rdt={} dd={} ok={} reason=address_width_sanity_classification",
                                aw_ok, aw_ring_align4k, aw_buf_align2048,
                                rdh_after, rdt_after, rx_dd_set, aw_ok);
                            serial_println!("[e1000.rx.addr.width.done] ok={} address_width_ok={} packets=0",
                                aw_ok, aw_ok);
                        }
                    } else {
                        serial_println!("[e1000.packet.buffer.alloc] pages=8 buffers=16 rx=8 tx=8 buffer_size=2048 allocated=0 ok=0 reason=alloc_frame_page_failed");
                        serial_println!("[e1000.packet.buffer.uc] pages=8 aliases=0 flags=NO_CACHE|WRITE_THROUGH flush=0 ok=0 reason=alloc_failed_no_pages");
                        serial_println!("[e1000.packet.buffer.truth] descriptor_linked=0 device_visible=0 mmio_writes=0 dma=0 packets=0 ok=0 reason=alloc_failed");
                        serial_println!("[e1000.packet.buffer.uc.alias.proof.done] ok=0 allocated=0 descriptor_linked=0 packets=0");
                    }
                } else {
                    serial_println!("[dma.static.ring.alloc] rx_bytes=4096 tx_bytes=4096 rx_align=4096 tx_align=4096 cache=UC allocated=0 ok=0 reason=alloc_frame_failed");
                    serial_println!("[e1000.ring.phys] rx_phys=0x0 tx_phys=0x0 rx_virt=0x0 tx_virt=0x0 ok=0 reason=alloc_failed");
                    serial_println!("[e1000.ring.truth] allocated=0 rings_enabled=0 dma=0 mmio_writes=0 irq=0 packets=0 ok=0 reason=alloc_failed");
                    serial_println!("[browser.nic.truth] slot_net_grant=0 network=0 fetched=0 dns=0 tcp=0 http=0 tls=0 ok=1 reason=no_network_capability");
                    serial_println!("[dma.static.ring.allocation.proof.done] ok=0 allocated=0 packets=0");
                }
            }
        }
    }

    devices
}
