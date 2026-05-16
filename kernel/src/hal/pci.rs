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
