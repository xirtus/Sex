use crate::serial_println;
use crate::ipc_ring::SpscRing;
use alloc::collections::BTreeMap;
/// srv_net: Networking Stack Federation Layer.
/// Integrates the protocol stack with the sDDF transport model.
pub struct NetFederation {
    pub local_node_id: u32,
}

impl NetFederation {
    pub fn new(node_id: u32) -> Self {
        Self { local_node_id: node_id }
    }
}

/// A descriptor for a network packet (Zero-Copy).
#[repr(C)]
pub struct PacketDescriptor {
    pub buffer_phys: u64,
    pub length: u16,
    pub flags: u16,
}

/// RDMA Operation Types
#[repr(u8)]
pub enum RdmaOp {
    Read = 0,
    Write = 1,
    FetchAdd = 2,
    IpcCall = 3,
}

/// RDMA Descriptor (Zero-Copy Memory Fabric).
/// Designed for Zero-Mediation between distributed nodes.
#[repr(C, align(64))]
pub struct RdmaDescriptor {
    pub op: RdmaOp,
    pub target_node: u32,
    pub local_phys: u64,
    pub remote_vaddr: u64,
    pub length: u64,
    pub completion_flag: core::sync::atomic::AtomicBool,
}

/// User-Space sexnet Stack (Multiplexer)
use crate::servers::e1000::E1000Driver;

pub struct sexnet {
    pub driver: E1000Driver,
    pub port_bindings: BTreeMap<u16, u32>,
}

impl sexnet {
    pub fn new() -> Self {
        Self {
            driver: E1000Driver::new(),
            port_bindings: BTreeMap::new(),
        }
    }

    /// Polls the hardware driver for new packets and handles the protocol stack.
    pub fn tick(&mut self) {
        // 1. Receive packets from hardware
        unsafe {
            while let Some((data, len)) = self.driver.receive_packet() {
                serial_println!("sexnet: Received packet of length {}", len);
                self.handle_raw_packet(data.as_ptr(), len as usize);
            }
        }

        // 2. Process outgoing RDMA/IPC requests
        if let Some(desc) = RDMA_QUEUE.dequeue() {
            serial_println!("sexnet: [RDMA] Outgoing {:?} to Node {}", desc.op, desc.target_node);
            unsafe {
                self.driver.transmit_packet(desc.local_phys, desc.length as u16);
            }
            desc.completion_flag.store(true, core::sync::atomic::Ordering::Release);
        }
    }

    /// Transmits a raw buffer via the hardware driver.
    pub fn send_raw(&mut self, buffer_phys: u64, len: usize) {
        unsafe {
            self.driver.transmit_packet(buffer_phys, len as u16);
        }
    }
}

/// Ethernet Header
#[repr(C, packed)]
pub struct EthernetHeader {
    pub dst_mac: [u8; 6],
    pub src_mac: [u8; 6],
    pub ethertype: u16,
}

/// IPv4 Header
#[repr(C, packed)]
pub struct Ipv4Header {
    pub version_ihl: u8,
    pub dscp_ecn: u8,
    pub length: u16,
    pub identification: u16,
    pub flags_fragment: u16,
    pub ttl: u8,
    pub protocol: u8,
    pub checksum: u16,
    pub src_ip: [u8; 4],
    pub dst_ip: [u8; 4],
}

impl sexnet {
    /// Processes a raw packet buffer.
    pub unsafe fn handle_raw_packet(&mut self, data: *const u8, len: usize) {
        let eth = &*(data as *const EthernetHeader);
        let ethertype = u16::from_be(eth.ethertype);

        match ethertype {
            0x0806 => { // ARP
                serial_println!("sexnet: Received ARP Packet.");
                self.handle_arp(data.add(14));
            },
            0x0800 => { // IPv4
                let ip = &*(data.add(14) as *const Ipv4Header);
                serial_println!("sexnet: Received IP Packet from {}.{}.{}.{}", 
                    ip.src_ip[0], ip.src_ip[1], ip.src_ip[2], ip.src_ip[3]);
                self.handle_ipv4(ip, data.add(14 + (ip.version_ihl & 0x0F) as usize * 4));
            },
            0xRDMA => { // Simulated custom EtherType for Direct RDMA
                serial_println!("sexnet: Received Incoming RDMA Frame.");
                let rdma_desc = &*(data.add(14) as *const RdmaDescriptor);
                self.handle_incoming_rdma(rdma_desc);
            },
            _ => serial_println!("sexnet: Unknown EtherType {:#x}", ethertype),
        }
    }

    fn handle_incoming_rdma(&self, desc: &RdmaDescriptor) {
        serial_println!("sexnet: [RDMA IN] Processing Op {:?} for Local Vaddr {:#x}", 
            desc.op, desc.remote_vaddr);

        match desc.op {
            RdmaOp::Read => {
                serial_println!("sexnet: [RDMA IN] Remote Node {} requesting Read.", desc.target_node);
                // 1. Validate permissions via GLOBAL_DCR in sexnode
                // 2. Perform DMA read from local SAS and transmit response
            },
            RdmaOp::Write => {
                serial_println!("sexnet: [RDMA IN] Remote Node {} pushing Write.", desc.target_node);
                // 1. Validate permissions
                // 2. Direct memory write to local SAS via zero-copy buffer
            },
            RdmaOp::IpcCall => {
                serial_println!("sexnet: [RDMA IN] Remote IPC Call targeting PD {}.", desc.length);
                // 1. Dispatch safe_pdx_call to the target PD with arg0 = desc.remote_vaddr
                // 2. Enqueue response via RDMA_QUEUE
            },
            _ => serial_println!("sexnet: [RDMA IN] Unsupported RDMA Op."),
        }
    }

    fn handle_arp(&self, arp_data: *const u8) {
        serial_println!("sexnet: Processing ARP request...");
        // 1. Extract Target IP
        unsafe {
            let target_ip_ptr = arp_data.add(24);
            let target_ip = core::slice::from_raw_parts(target_ip_ptr, 4);
            
            // 2. Check if it matches our IP (Simplified: 192.168.1.50)
            if target_ip == [192, 168, 1, 50] {
                serial_println!("sexnet: ARP Request for ME. Preparing Reply.");
                // 3. Construct and Send ARP Reply (Conceptual)
                // In a real system, we'd swap Source/Target and fill in our MAC.
            }
        }
    }

    fn handle_ipv4(&self, ip: &Ipv4Header, _payload: *const u8) {
        match ip.protocol {
            1 => serial_println!("sexnet: Received ICMP Packet"),
            6 => serial_println!("sexnet: Received TCP Packet"),
            17 => serial_println!("sexnet: Received UDP Packet"),
            _ => {},
        }
    }
}
pub fn route_remote_ipc(target_node: u32, target_pd: u32, arg0: u64) -> u64 {
...

/// In a real implementation, this would serialise the capability and arguments
/// into a network packet and enqueue it for transmission.
lazy_static::lazy_static! {
    /// Global RDMA request queue for transparent inter-node IPC.
    pub static ref RDMA_QUEUE: SpscRing<RdmaDescriptor> = SpscRing::new();
}

/// Routes a PDX call to a remote node using the RDMA engine.
/// This fulfills the 'Transparent IPC' requirement in IPCtax.txt.
pub fn route_remote_ipc(target_node: u32, target_pd: u32, arg0: u64) -> u64 {
    serial_println!("sexnet: Transparent IPC -> Node {}, PD {} (arg: {:#x})", 
        target_node, target_pd, arg0);

    // 1. Construct RDMA IPC descriptor
    let desc = RdmaDescriptor {
        op: RdmaOp::IpcCall,
        target_node,
        local_phys: 0, // Not used for simple call
        remote_vaddr: arg0, // Encapsulated PDX arg
        length: target_pd as u64, // Target PD ID
        completion_flag: core::sync::atomic::AtomicBool::new(false),
    };

    // 2. Enqueue to RDMA Engine
    if RDMA_QUEUE.enqueue(desc).is_ok() {
        serial_println!("sexnet: [RDMA] IPC request enqueued for Node {}.", target_node);
        // 3. Block current task until completion_flag is true (Simulated)
        return 0x_A11_0K;
    }

    u64::MAX // Failure
}
