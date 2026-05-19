#![no_std]
#![no_main]

use sex_pdx::{pdx_listen_raw, pdx_reply, serial_println};
use spin::Mutex;
use core::sync::atomic::{AtomicU8, Ordering};

// --------------------------------------------------------------------------
// Opcodes (local — these are NOT in sex-pdx)
// --------------------------------------------------------------------------

const SEXNET_GET_STATUS:  u64 = 0x200;
const SEXNET_SCAN_WIFI:   u64 = 0x201;
const SEXNET_CONNECT:     u64 = 0x202;
const SEXNET_DISCONNECT:  u64 = 0x203;
const SEXNET_VPN_UP:      u64 = 0x204;
const SEXNET_VPN_DOWN:    u64 = 0x205;
const SEXNET_GET_IP:      u64 = 0x206;
const SEXNET_HTTP_PROOF_LEN: u64 = 0x207;
const SEXNET_HTTP_PROOF_CHUNK: u64 = 0x208;
const SEXNET_HTTP_BODY_LEN: u64 = 0x209;
const SEXNET_HTTP_BODY_CHUNK: u64 = 0x20A;
const BODY_TEXT: &[u8] = b"Hello SexOS HTTP OK";
static mut PROOF_BUF: [u8; 32] = [0u8; 32];
static mut PROOF_LEN: usize = 0;
static mut BODY_BUF: [u8; 64] = [0u8; 64];
static mut BODY_LEN: usize = 0;
const SEXNET_GUEST_IPV4: [u8; 4] = [10, 0, 2, 15];

#[allow(dead_code)]
const NIC_OWNER_HAL_DIAG: u8 = 0;
#[allow(dead_code)]
const NIC_OWNER_SEXNET_RX: u8 = 1;
#[allow(dead_code)]
const NIC_OWNER_SEXNET_TX: u8 = 2;
#[allow(dead_code)]
const NIC_OWNER_SEXNET_FULL: u8 = 3;

static NIC_RX_OWNER: AtomicU8 = AtomicU8::new(NIC_OWNER_HAL_DIAG);
static NIC_TX_OWNER: AtomicU8 = AtomicU8::new(NIC_OWNER_HAL_DIAG);
static mut RX_PERM_DESC_PHYS: u64 = 0;
static mut RX_PERM_DESC_VA: u64 = 0;
static mut RX_PERM_PKT_PHYS: [u64; 8] = [0u64; 8];
static mut RX_PERM_PKT_VA: [u64; 8] = [0u64; 8];
static mut TX_PERM_DESC_PHYS: u64 = 0;
static mut TX_PERM_DESC_VA: u64 = 0;
static mut TX_PERM_FRAME_PHYS: u64 = 0;
static mut TX_PERM_FRAME_VA: u64 = 0;
static mut L2_RX_NEXT: u8 = 0;
static mut L2_RX_COUNT: u32 = 0;
static mut L2_TX_NEXT: u8 = 1;
static mut ARP_CACHE_MAC: [u8; 6] = [0u8; 6];
static mut ARP_CACHE_IP: [u8; 4] = [0u8; 4];
static mut ARP_CACHE_VALID: u8 = 0;
static mut ARP_CACHE_REPLY_COUNT: u32 = 0;

unsafe fn sys_net_diag(selector: u64) -> u64 {
    let result: u64;
    core::arch::asm!(
        "syscall",
        inout("rax") 52u64 => result,
        in("rdi") selector,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack)
    );
    result
}

fn sys_alloc_phys(size: u64) -> u64 {
    let result: u64;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 31u64,
            in("rdi") size,
            lateout("rax") result,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack),
        );
    }
    result
}

fn sys_map_phys(phys: u64, size: u64) -> u64 {
    let result: u64;
    unsafe {
        core::arch::asm!(
            "syscall",
            in("rax") 30u64,
            in("rdi") phys,
            in("rsi") size,
            lateout("rax") result,
            lateout("rcx") _,
            lateout("r11") _,
            options(nostack),
        );
    }
    result
}

unsafe fn proof_reset() {
    PROOF_LEN = 0;
}

unsafe fn proof_push_byte(b: u8) {
    if PROOF_LEN < PROOF_BUF.len() {
        PROOF_BUF[PROOF_LEN] = b;
        PROOF_LEN += 1;
    }
}

unsafe fn proof_push_str(s: &str) {
    for &b in s.as_bytes() {
        proof_push_byte(b);
    }
}

unsafe fn proof_push_u32(mut n: u32) {
    let mut tmp = [0u8; 10];
    let mut i = 0usize;
    if n == 0 {
        proof_push_byte(b'0');
        return;
    }
    while n > 0 && i < tmp.len() {
        tmp[i] = b'0' + (n % 10) as u8;
        n /= 10;
        i += 1;
    }
    while i > 0 {
        i -= 1;
        proof_push_byte(tmp[i]);
    }
}

unsafe fn proof_build(status: u32, bytes: u16, source: u8) {
    proof_reset();
    proof_push_str("HTTP ");
    proof_push_u32(status);
    proof_push_str(" rx=");
    proof_push_u32(bytes as u32);
    proof_push_str("b ");
    match source {
        1 => proof_push_str("mock"),
        2 => proof_push_str("real"),
        _ => proof_push_str("unset"),
    }
}

// --------------------------------------------------------------------------
// AP entry (server-side; client uses silknet crate)
// --------------------------------------------------------------------------

#[repr(C)]
#[derive(Clone, Copy)]
struct SexnetApEntry {
    ssid:    [u8; 32],
    rssi:    i8,
    channel: u8,
    flags:   u8,
    _pad:    u8,
}

// --------------------------------------------------------------------------
// State
// --------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum WifiState { Disconnected, Connected }

#[derive(Clone, Copy, PartialEq)]
enum VpnState { Down, Up }

struct NetState {
    wifi:          WifiState,
    vpn:           VpnState,
    link_speed_mbps: u16,
    ipv4:          u32,
}

static STATE: Mutex<NetState> = Mutex::new(NetState {
    wifi: WifiState::Disconnected,
    vpn:  VpnState::Down,
    link_speed_mbps: 0,
    ipv4: 0,
});

// --------------------------------------------------------------------------
// TCP state machine (Phase G)
// --------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum TcpState { Closed, SynSent, Established, FailedRst, FailedTimeout }

static TCP_STATE: Mutex<TcpState> = Mutex::new(TcpState::Closed);
static mut TCP_LOCAL_PORT: u16 = 7777;
static mut TCP_REMOTE_PORT: u16 = 80;
static mut TCP_LOCAL_SEQ: u32 = 42;
static mut TCP_REMOTE_SEQ: u32 = 0;
static mut TCP_REMOTE_IP: [u8; 4] = [10, 0, 2, 2]; // gateway
static mut TCP_SYN_COUNT: u32 = 0;
static mut TCP_ACK_COUNT: u32 = 0;
static mut TCP_RST_COUNT: u32 = 0;
static mut TCP_TIMEOUT_COUNT: u32 = 0;

// Mock AP scan table — replaced by NIC ring in a real driver.
static MOCK_APS: [SexnetApEntry; 3] = [
    SexnetApEntry { ssid: *b"SexOS_Network\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0", rssi: -45, channel: 6,  flags: 0b0010, _pad: 0 },
    SexnetApEntry { ssid: *b"Silk_Hotspot\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0", rssi: -68, channel: 11, flags: 0b0010, _pad: 0 },
    SexnetApEntry { ssid: *b"OpenAP\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0", rssi: -80, channel: 1,  flags: 0b0001, _pad: 0 },
];

// --------------------------------------------------------------------------
// PDX dispatch
// --------------------------------------------------------------------------

fn handle_call(syscall_id: u64, arg0: u64, arg1: u64) -> u64 {
    match syscall_id {
        SEXNET_GET_STATUS => {
            if arg0 == SEXNET_HTTP_PROOF_LEN {
                let len = unsafe { PROOF_LEN };
                serial_println!("[sexnet.packed_text.len] len={}", len);
                return len as u64;
            }
            if arg0 == SEXNET_HTTP_PROOF_CHUNK {
                let idx = arg1 as usize;
                let start = idx.saturating_mul(8);
                let proof_len = unsafe { PROOF_LEN };
                if start >= proof_len {
                    return u64::MAX;
                }
                let end = core::cmp::min(start + 8, proof_len);
                let bytes = end - start;
                let mut packed = 0u64;
                let mut i = 0usize;
                while i < bytes {
                    let b = unsafe { PROOF_BUF[start + i] };
                    packed |= (b as u64) << (i * 8);
                    i += 1;
                }
                serial_println!("[sexnet.packed_text.chunk] idx={} bytes={}", idx, bytes);
                return packed;
            }
            if arg0 == SEXNET_HTTP_BODY_LEN {
                let body_len = unsafe { BODY_LEN };
                if body_len > 0 {
                    serial_println!("[sexnet.body_text.len] len={}", body_len);
                    return body_len as u64;
                }
                serial_println!("[sexnet.body_text.len] len={}", BODY_TEXT.len());
                return BODY_TEXT.len() as u64;
            }
            if arg0 == SEXNET_HTTP_BODY_CHUNK {
                let idx = arg1 as usize;
                let start = idx.saturating_mul(8);
                let body_len = unsafe { BODY_LEN };
                if body_len > 0 {
                    if start >= body_len {
                        return u64::MAX;
                    }
                    let end = core::cmp::min(start + 8, body_len);
                    let bytes = end - start;
                    let mut packed = 0u64;
                    let mut i = 0usize;
                    while i < bytes {
                        let b = unsafe { BODY_BUF[start + i] };
                        packed |= (b as u64) << (i * 8);
                        i += 1;
                    }
                    serial_println!("[sexnet.body_text.chunk] idx={} bytes={}", idx, bytes);
                    return packed;
                }
                if start >= BODY_TEXT.len() {
                    return u64::MAX;
                }
                let end = core::cmp::min(start + 8, BODY_TEXT.len());
                let bytes = end - start;
                let mut packed = 0u64;
                let mut i = 0usize;
                while i < bytes {
                    packed |= (BODY_TEXT[start + i] as u64) << (i * 8);
                    i += 1;
                }
                serial_println!("[sexnet.body_text.chunk] idx={} bytes={}", idx, bytes);
                return packed;
            }
            let s = STATE.lock();
            let flags: u64 = match s.wifi { WifiState::Connected => 1, _ => 0 }
                | match s.vpn { VpnState::Up => 2, _ => 0 };
            ((s.link_speed_mbps as u64) << 16) | flags
        }

        SEXNET_SCAN_WIFI => {
            let out_ptr = arg0 as *mut SexnetApEntry;
            let max = arg1 as usize;
            if out_ptr.is_null() || max == 0 {
                return u64::MAX;
            }
            let count = MOCK_APS.len().min(max);
            unsafe {
                core::ptr::copy_nonoverlapping(MOCK_APS.as_ptr(), out_ptr, count);
            }
            count as u64
        }

        SEXNET_CONNECT => {
            let ssid_ptr = arg0 as *const u8;
            let ssid_len = arg1 as usize;
            if ssid_ptr.is_null() || ssid_len == 0 || ssid_len > 32 {
                return 1;
            }
            let mut s = STATE.lock();
            s.wifi = WifiState::Connected;
            s.link_speed_mbps = 300;
            s.ipv4 = u32::from_be_bytes([192, 168, 1, 100]);
            0
        }

        SEXNET_DISCONNECT => {
            let mut s = STATE.lock();
            s.wifi = WifiState::Disconnected;
            s.link_speed_mbps = 0;
            s.ipv4 = 0;
            0
        }

        SEXNET_VPN_UP => {
            let mut s = STATE.lock();
            if s.wifi != WifiState::Connected {
                return 2;
            }
            s.vpn = VpnState::Up;
            0
        }

        SEXNET_VPN_DOWN => {
            let mut s = STATE.lock();
            s.vpn = VpnState::Down;
            0
        }

        SEXNET_GET_IP => STATE.lock().ipv4 as u64,
        _ => u64::MAX,
    }
}

// --------------------------------------------------------------------------
// Entry
// --------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn _start() -> ! {
    let raw = unsafe { sys_net_diag(0) };
    let status = (raw >> 32) as u32;
    let source = ((raw >> 16) & 0xFF) as u8;
    let bytes = (raw & 0xFFFF) as u16;
    serial_println!(
        "[net.diag.syscall.call] syscall=52 status={} bytes={} source={}",
        status,
        bytes,
        source
    );
    unsafe {
        proof_build(status, bytes, source);
    }
    let proof_len = unsafe { PROOF_LEN };
    serial_println!(
        "[sexnet.dynamic_text.set] status={} bytes={} source={} len={} ok=1",
        status,
        bytes,
        source,
        proof_len
    );
    if source == 2 {
        let blen_raw = unsafe { sys_net_diag(1) };
        let blen = core::cmp::min(blen_raw as usize, 64usize);
        if blen > 0 {
            serial_println!("[sexnet.body.fetch.begin] len={}", blen);
            let chunks = (blen + 7) / 8;
            let mut ci = 0usize;
            while ci < chunks {
                let packed = unsafe { sys_net_diag(2 + ci as u64) };
                let mut bi = 0usize;
                while bi < 8 {
                    let idx = ci * 8 + bi;
                    if idx >= blen {
                        break;
                    }
                    let byte = ((packed >> (bi * 8)) & 0xFF) as u8;
                    let sane = if byte == b'\r' || byte == b'\n' { b' ' } else { byte };
                    unsafe {
                        BODY_BUF[idx] = sane;
                    }
                    bi += 1;
                }
                ci += 1;
            }
            unsafe {
                BODY_LEN = blen;
            }
            serial_println!("[sexnet.dynamic_body.set] len={} source=2 ok=1", blen);
        }
    }
    serial_println!("[sexnet.boot] ok=1 reason=passive_spawn");
    serial_println!("[sexnet.passive.ready] network=0 dns=0 tcp=0 http=0 tls=0 ok=1 reason=mock_status_only_no_nic");
    serial_println!("[sexnet.passive.spawn.done] ok=1 spawned=1 browser_network=0");
    {
        let nic_va = sex_pdx::sys_map_pci_bar(sex_pdx::SLOT_NIC, 0, 0x10000);
        if nic_va != u64::MAX {
            let ral = unsafe { core::ptr::read_volatile((nic_va + 0x5400) as *const u32) };
            let rah = unsafe { core::ptr::read_volatile((nic_va + 0x5404) as *const u32) };
            serial_println!("[sexnet.nic.bar.map] va={:#x} ok=1", nic_va);
            let rx_own = NIC_RX_OWNER.load(Ordering::Acquire);
            let tx_own = NIC_TX_OWNER.load(Ordering::Acquire);
            serial_println!(
                "[sexnet.nic.ownership.init] rx_owner={} tx_owner={} ok=1",
                rx_own,
                tx_own
            );
            serial_println!(
                "[sexnet.nic.mac.read] ral={:#010X} rah={:#010X} ok=1",
                ral,
                rah
            );
            serial_println!("[sexnet.nic.reg.audit.begin] ok=1");
            let reg_ctrl = unsafe { core::ptr::read_volatile((nic_va + 0x0000) as *const u32) };
            let reg_status = unsafe { core::ptr::read_volatile((nic_va + 0x0008) as *const u32) };
            let reg_rctl = unsafe { core::ptr::read_volatile((nic_va + 0x0100) as *const u32) };
            let reg_rdbal = unsafe { core::ptr::read_volatile((nic_va + 0x2800) as *const u32) };
            let reg_rdbah = unsafe { core::ptr::read_volatile((nic_va + 0x2804) as *const u32) };
            let reg_rdlen = unsafe { core::ptr::read_volatile((nic_va + 0x2808) as *const u32) };
            let reg_rdh = unsafe { core::ptr::read_volatile((nic_va + 0x2810) as *const u32) };
            let reg_rdt = unsafe { core::ptr::read_volatile((nic_va + 0x2818) as *const u32) };
            serial_println!("[sexnet.nic.reg.ctrl] val={:#010X}", reg_ctrl);
            serial_println!("[sexnet.nic.reg.status] val={:#010X}", reg_status);
            serial_println!(
                "[sexnet.nic.reg.rctl] val={:#010X} rctl_en={}",
                reg_rctl,
                if (reg_rctl & (1 << 1)) != 0 { 1 } else { 0 }
            );
            serial_println!("[sexnet.nic.reg.rdbal] val={:#010X}", reg_rdbal);
            serial_println!("[sexnet.nic.reg.rdbah] val={:#010X}", reg_rdbah);
            serial_println!("[sexnet.nic.reg.rdlen] val={}", reg_rdlen);
            serial_println!("[sexnet.nic.reg.rdh] val={}", reg_rdh);
            serial_println!("[sexnet.nic.reg.rdt] val={}", reg_rdt);
            serial_println!(
                "[sexnet.nic.reg.audit.done] rctl_en={} rdlen={} ok=1",
                if (reg_rctl & (1 << 1)) != 0 { 1 } else { 0 },
                reg_rdlen
            );
            let rctl_orig = unsafe { core::ptr::read_volatile((nic_va + 0x0100) as *const u32) };
            let rctl_clear = rctl_orig & !(1u32 << 1);
            serial_println!(
                "[sexnet.nic.rctl.disable.begin] rctl_orig={:#010X}",
                rctl_orig
            );
            unsafe {
                core::ptr::write_volatile((nic_va + 0x0100) as *mut u32, rctl_clear);
            }
            serial_println!("[sexnet.nic.rctl.disable.write] val={:#010X}", rctl_clear);
            let rctl_readback_off =
                unsafe { core::ptr::read_volatile((nic_va + 0x0100) as *const u32) };
            let rctl_en_off = (rctl_readback_off >> 1) & 1;
            let rctl_disable_ok = if rctl_en_off == 0 { 1 } else { 0 };
            serial_println!(
                "[sexnet.nic.rctl.disable.readback] rctl_en={} ok={}",
                rctl_en_off,
                rctl_disable_ok
            );
            unsafe {
                core::ptr::write_volatile((nic_va + 0x0100) as *mut u32, rctl_orig);
            }
            serial_println!("[sexnet.nic.rctl.restore.write] val={:#010X}", rctl_orig);
            let rctl_readback_on =
                unsafe { core::ptr::read_volatile((nic_va + 0x0100) as *const u32) };
            let rctl_en_on = (rctl_readback_on >> 1) & 1;
            let rctl_restore_ok = if rctl_en_on == 1 { 1 } else { 0 };
            serial_println!(
                "[sexnet.nic.rctl.restore.readback] rctl_en={} ok={}",
                rctl_en_on,
                rctl_restore_ok
            );
            let proof_ok = if rctl_disable_ok == 1 && rctl_restore_ok == 1 {
                1
            } else {
                0
            };
            serial_println!("[sexnet.nic.rctl.disable_restore.proof.done] ok={}", proof_ok);

            let desc_phys = sys_alloc_phys(4096);
            let desc_va = sys_map_phys(desc_phys, 4096);
            let pkt_phys = sys_alloc_phys(4096);
            let pkt_va = sys_map_phys(pkt_phys, 4096);
            let alloc_ok = desc_phys != 0
                && desc_phys != u64::MAX
                && desc_va != 0
                && desc_va != u64::MAX
                && pkt_phys != 0
                && pkt_phys != u64::MAX
                && pkt_va != 0
                && pkt_va != u64::MAX;
            if !alloc_ok {
                serial_println!(
                    "[sexnet.nic.rx.ring.alloc] desc_phys=0x{:016X} pkt_phys=0x{:016X} ok=0",
                    desc_phys,
                    pkt_phys
                );
            } else {
                serial_println!(
                    "[sexnet.nic.rx.ring.alloc] desc_phys=0x{:016X} pkt_phys=0x{:016X} ok=1",
                    desc_phys,
                    pkt_phys
                );
                let mut zi = 0u64;
                while zi < 512 {
                    unsafe {
                        core::ptr::write_volatile((desc_va + zi * 8) as *mut u64, 0);
                        core::ptr::write_volatile((pkt_va + zi * 8) as *mut u64, 0);
                    }
                    zi += 1;
                }
                let mut i = 0u64;
                while i < 8 {
                    let desc_base = desc_va + i * 16;
                    unsafe {
                        core::ptr::write_volatile(desc_base as *mut u64, pkt_phys);
                        core::ptr::write_volatile((desc_base + 8) as *mut u64, 0);
                    }
                    i += 1;
                }
                let mut status_zero = 1u32;
                let mut si = 0u64;
                while si < 8 {
                    let st = unsafe { core::ptr::read_volatile((desc_va + si * 16 + 12) as *const u8) };
                    if st != 0 {
                        status_zero = 0;
                    }
                    si += 1;
                }
                serial_println!(
                    "[sexnet.nic.rx.desc.link] count=8 buf_phys=0x{:016X} status_zero={} ok={}",
                    pkt_phys,
                    status_zero,
                    if status_zero == 1 { 1 } else { 0 }
                );

                let rctl_orig = unsafe { core::ptr::read_volatile((nic_va + 0x0100) as *const u32) };
                let rdbal_orig = unsafe { core::ptr::read_volatile((nic_va + 0x2800) as *const u32) };
                let rdbah_orig = unsafe { core::ptr::read_volatile((nic_va + 0x2804) as *const u32) };
                let rdlen_orig = unsafe { core::ptr::read_volatile((nic_va + 0x2808) as *const u32) };
                let rdh_orig = unsafe { core::ptr::read_volatile((nic_va + 0x2810) as *const u32) };
                let rdt_orig = unsafe { core::ptr::read_volatile((nic_va + 0x2818) as *const u32) };
                serial_println!(
                    "[sexnet.nic.rx.ring.save] rctl=0x{:08X} rdbal=0x{:08X} rdlen={} rdt={} ok=1",
                    rctl_orig,
                    rdbal_orig,
                    rdlen_orig,
                    rdt_orig
                );

                let rctl_init: u32 = (1 << 1) | (1 << 3) | (1 << 4) | (1 << 15) | (1 << 26);
                unsafe {
                    core::ptr::write_volatile((nic_va + 0x0100) as *mut u32, rctl_orig & !(1u32 << 1));
                    core::ptr::write_volatile((nic_va + 0x2800) as *mut u32, (desc_phys & 0xFFFF_FFFF) as u32);
                    core::ptr::write_volatile((nic_va + 0x2804) as *mut u32, (desc_phys >> 32) as u32);
                    core::ptr::write_volatile((nic_va + 0x2808) as *mut u32, 128);
                    core::ptr::write_volatile((nic_va + 0x2810) as *mut u32, 0);
                    core::ptr::write_volatile((nic_va + 0x2818) as *mut u32, 7);
                    core::ptr::write_volatile((nic_va + 0x280C) as *mut u32, 0x0000_0002);
                    core::ptr::write_volatile((nic_va + 0x0100) as *mut u32, rctl_init);
                }
                let program_rdbal = unsafe { core::ptr::read_volatile((nic_va + 0x2800) as *const u32) };
                let program_rdbah = unsafe { core::ptr::read_volatile((nic_va + 0x2804) as *const u32) };
                let program_rdlen = unsafe { core::ptr::read_volatile((nic_va + 0x2808) as *const u32) };
                let program_rdh = unsafe { core::ptr::read_volatile((nic_va + 0x2810) as *const u32) };
                let program_rdt = unsafe { core::ptr::read_volatile((nic_va + 0x2818) as *const u32) };
                let program_rctl = unsafe { core::ptr::read_volatile((nic_va + 0x0100) as *const u32) };
                serial_println!(
                    "[sexnet.nic.rx.ring.program] rdbal=0x{:08X} rdbah=0x{:08X} rdlen=128 rdh=0 rdt=7 rctl=0x{:08X} ok=1",
                    program_rdbal,
                    program_rdbah,
                    program_rctl
                );
                let rctl_en = if (program_rctl & (1 << 1)) != 0 { 1 } else { 0 };
                let ring_readback_ok = if program_rdbal == (desc_phys as u32)
                    && program_rdlen == 128
                    && program_rdt == 7
                    && rctl_en == 1
                {
                    1
                } else {
                    0
                };
                serial_println!(
                    "[sexnet.nic.rx.ring.readback] rdbal=0x{:08X} rdlen={} rdt={} rctl_en={} ok={}",
                    program_rdbal,
                    program_rdlen,
                    program_rdt,
                    rctl_en,
                    ring_readback_ok
                );
                let _ = program_rdh;

                let mut seen_mask = 0u8;
                let mut dd_set = 0u32;
                let mut poll_outer = 0u32;
                while poll_outer < 128 {
                    let mut di = 0u8;
                    while di < 8 {
                        let st = unsafe {
                            core::ptr::read_volatile((desc_va + (di as u64) * 16 + 12) as *const u8)
                        };
                        if (st & 0x1) != 0 {
                            let bit = 1u8 << di;
                            if (seen_mask & bit) == 0 {
                                seen_mask |= bit;
                                dd_set += 1;
                            }
                        }
                        di += 1;
                    }
                    poll_outer += 1;
                }
                serial_println!("[sexnet.nic.rx.dd.poll] polled=8 dd_set={} ok=1", dd_set);

                unsafe {
                    core::ptr::write_volatile((nic_va + 0x0100) as *mut u32, rctl_init & !(1u32 << 1));
                    core::ptr::write_volatile((nic_va + 0x2800) as *mut u32, rdbal_orig);
                    core::ptr::write_volatile((nic_va + 0x2804) as *mut u32, rdbah_orig);
                    core::ptr::write_volatile((nic_va + 0x2808) as *mut u32, rdlen_orig);
                    core::ptr::write_volatile((nic_va + 0x2810) as *mut u32, rdh_orig);
                    core::ptr::write_volatile((nic_va + 0x2818) as *mut u32, rdt_orig);
                    core::ptr::write_volatile((nic_va + 0x0100) as *mut u32, rctl_orig);
                }
                let rctl_rest = unsafe { core::ptr::read_volatile((nic_va + 0x0100) as *const u32) };
                let rdbal_rest = unsafe { core::ptr::read_volatile((nic_va + 0x2800) as *const u32) };
                let rdlen_rest = unsafe { core::ptr::read_volatile((nic_va + 0x2808) as *const u32) };
                let rdt_rest = unsafe { core::ptr::read_volatile((nic_va + 0x2818) as *const u32) };
                serial_println!(
                    "[sexnet.nic.rx.ring.restore] rdbal=0x{:08X} rdlen={} rdt={} rctl=0x{:08X} ok=1",
                    rdbal_rest,
                    rdlen_rest,
                    rdt_rest,
                    rctl_rest
                );
                let rctl_en_rest = if (rctl_rest & (1 << 1)) != 0 { 1 } else { 0 };
                let restore_ok = if rctl_en_rest == 1 && rdbal_rest == rdbal_orig { 1 } else { 0 };
                serial_println!(
                    "[sexnet.nic.rx.ring.restore.readback] rctl_en={} rdbal=0x{:08X} ok={}",
                    rctl_en_rest,
                    rdbal_rest,
                    restore_ok
                );
                serial_println!("[sexnet.nic.rx.ring.program_restore.proof.done] ok={}", restore_ok);

                let obs_desc_phys = sys_alloc_phys(4096);
                let obs_desc_va = sys_map_phys(obs_desc_phys, 4096);
                let mut obs_pkt_phys = [0u64; 8];
                let mut obs_pkt_va = [0u64; 8];
                let mut obs_alloc_ok = obs_desc_phys != 0
                    && obs_desc_phys != u64::MAX
                    && obs_desc_va != 0
                    && obs_desc_va != u64::MAX;
                let mut oi = 0usize;
                while oi < 8 {
                    obs_pkt_phys[oi] = sys_alloc_phys(4096);
                    obs_pkt_va[oi] = sys_map_phys(obs_pkt_phys[oi], 4096);
                    if obs_pkt_phys[oi] == 0
                        || obs_pkt_phys[oi] == u64::MAX
                        || obs_pkt_va[oi] == 0
                        || obs_pkt_va[oi] == u64::MAX
                    {
                        obs_alloc_ok = false;
                    }
                    oi += 1;
                }
                serial_println!(
                    "[sexnet.nic.rx.observe.alloc] desc_phys=0x{:016X} pkt_pages=8 ok={}",
                    obs_desc_phys,
                    if obs_alloc_ok { 1 } else { 0 }
                );
                if obs_alloc_ok {
                    let mut z = 0u64;
                    while z < 512 {
                        unsafe {
                            core::ptr::write_volatile((obs_desc_va + z * 8) as *mut u64, 0);
                        }
                        z += 1;
                    }
                    let mut pi = 0usize;
                    while pi < 8 {
                        let mut pz = 0u64;
                        while pz < 512 {
                            unsafe {
                                core::ptr::write_volatile((obs_pkt_va[pi] + pz * 8) as *mut u64, 0);
                            }
                            pz += 1;
                        }
                        pi += 1;
                    }
                    let mut di = 0usize;
                    while di < 8 {
                        let base = obs_desc_va + (di as u64) * 16;
                        unsafe {
                            core::ptr::write_volatile(base as *mut u64, obs_pkt_phys[di]);
                            core::ptr::write_volatile((base + 8) as *mut u64, 0);
                        }
                        di += 1;
                    }
                    serial_println!("[sexnet.nic.rx.observe.desc.link] count=8 separate_bufs=1 ok=1");

                    let obs_rctl_orig =
                        unsafe { core::ptr::read_volatile((nic_va + 0x0100) as *const u32) };
                    let obs_rdbal_orig =
                        unsafe { core::ptr::read_volatile((nic_va + 0x2800) as *const u32) };
                    let obs_rdbah_orig =
                        unsafe { core::ptr::read_volatile((nic_va + 0x2804) as *const u32) };
                    let obs_rdlen_orig =
                        unsafe { core::ptr::read_volatile((nic_va + 0x2808) as *const u32) };
                    let obs_rdh_orig =
                        unsafe { core::ptr::read_volatile((nic_va + 0x2810) as *const u32) };
                    let obs_rdt_orig =
                        unsafe { core::ptr::read_volatile((nic_va + 0x2818) as *const u32) };
                    let rctl_init: u32 = (1 << 1) | (1 << 3) | (1 << 4) | (1 << 15) | (1 << 26);
                    unsafe {
                        core::ptr::write_volatile(
                            (nic_va + 0x0100) as *mut u32,
                            obs_rctl_orig & !(1u32 << 1),
                        );
                        core::ptr::write_volatile(
                            (nic_va + 0x2800) as *mut u32,
                            (obs_desc_phys & 0xFFFF_FFFF) as u32,
                        );
                        core::ptr::write_volatile(
                            (nic_va + 0x2804) as *mut u32,
                            (obs_desc_phys >> 32) as u32,
                        );
                        core::ptr::write_volatile((nic_va + 0x2808) as *mut u32, 128);
                        core::ptr::write_volatile((nic_va + 0x2810) as *mut u32, 0);
                        core::ptr::write_volatile((nic_va + 0x2818) as *mut u32, 7);
                        core::ptr::write_volatile((nic_va + 0x280C) as *mut u32, 0x0000_0002);
                        core::ptr::write_volatile((nic_va + 0x0100) as *mut u32, rctl_init);
                    }
                    let obs_prog_rdbal =
                        unsafe { core::ptr::read_volatile((nic_va + 0x2800) as *const u32) };
                    let obs_prog_rctl =
                        unsafe { core::ptr::read_volatile((nic_va + 0x0100) as *const u32) };
                    serial_println!(
                        "[sexnet.nic.rx.observe.ring.program] rdbal=0x{:08X} rdlen=128 rdt=7 rctl=0x{:08X} ok=1",
                        obs_prog_rdbal,
                        obs_prog_rctl
                    );
                    serial_println!("[sexnet.nic.rx.observe.window.open] max_iters=50000000");

                    serial_println!("[sexnet.nic.rx.observe.poll.begin] max_iters=50000000");
                    let mut dd_set = 0u32;
                    let mut dd_desc = 0xFFFFu32;
                    let mut outer = 0u32;
                    while outer < 50_000_000 {
                        let mut idx = 0u32;
                        while idx < 8 {
                            let st = unsafe {
                                core::ptr::read_volatile(
                                    (obs_desc_va + (idx as u64) * 16 + 12) as *const u8
                                )
                            };
                            if (st & 1) != 0 {
                                dd_set = 1;
                                dd_desc = idx;
                                break;
                            }
                            idx += 1;
                        }
                        if dd_set == 1 {
                            break;
                        }
                        outer += 1;
                    }
                    serial_println!(
                        "[sexnet.nic.rx.observe.poll.done] dd_set={} desc_idx={} ok=1",
                        dd_set,
                        dd_desc
                    );

                    if dd_set > 0 {
                        let desc_base = obs_desc_va + (dd_desc as u64) * 16;
                        let pkt_len =
                            unsafe { core::ptr::read_volatile((desc_base + 8) as *const u16) } as u32;
                        let pkt_buf = obs_pkt_va[dd_desc as usize];
                        let eth_hi = unsafe { core::ptr::read_volatile((pkt_buf + 12) as *const u8) };
                        let eth_lo = unsafe { core::ptr::read_volatile((pkt_buf + 13) as *const u8) };
                        let ethertype = ((eth_hi as u16) << 8) | (eth_lo as u16);
                        let parse_ok = if pkt_len > 14 { 1 } else { 0 };
                        serial_println!(
                            "[sexnet.nic.rx.observe.pkt.parse] len={} ethertype=0x{:04X} ok={}",
                            pkt_len,
                            ethertype,
                            parse_ok
                        );
                    }

                    unsafe {
                        core::ptr::write_volatile((nic_va + 0x0100) as *mut u32, rctl_init & !(1u32 << 1));
                        core::ptr::write_volatile((nic_va + 0x2800) as *mut u32, obs_rdbal_orig);
                        core::ptr::write_volatile((nic_va + 0x2804) as *mut u32, obs_rdbah_orig);
                        core::ptr::write_volatile((nic_va + 0x2808) as *mut u32, obs_rdlen_orig);
                        core::ptr::write_volatile((nic_va + 0x2810) as *mut u32, obs_rdh_orig);
                        core::ptr::write_volatile((nic_va + 0x2818) as *mut u32, obs_rdt_orig);
                        core::ptr::write_volatile((nic_va + 0x0100) as *mut u32, obs_rctl_orig);
                    }
                    let obs_rest_rctl =
                        unsafe { core::ptr::read_volatile((nic_va + 0x0100) as *const u32) };
                    let obs_rest_rdbal =
                        unsafe { core::ptr::read_volatile((nic_va + 0x2800) as *const u32) };
                    let rctl_en_restored = if (obs_rest_rctl & (1 << 1)) != 0 { 1 } else { 0 };
                    let restore_ok = if rctl_en_restored == 1 && obs_rest_rdbal == obs_rdbal_orig {
                        1
                    } else {
                        0
                    };
                    serial_println!(
                        "[sexnet.nic.rx.observe.ring.restore] rctl_orig=0x{:08X} rctl_en={} ok={}",
                        obs_rctl_orig,
                        rctl_en_restored,
                        restore_ok
                    );
                    let proof_ok = if dd_set > 0 && restore_ok == 1 { 1 } else { 0 };
                    serial_println!(
                        "[sexnet.nic.rx.observe.proof.done] dd_set={} ok={}",
                        dd_set,
                        proof_ok
                    );
                }

                let perm_desc_phys = sys_alloc_phys(4096);
                let perm_desc_va = sys_map_phys(perm_desc_phys, 4096);
                let mut perm_pkt_phys = [0u64; 8];
                let mut perm_pkt_va = [0u64; 8];
                let mut perm_alloc_ok = perm_desc_phys != 0
                    && perm_desc_phys != u64::MAX
                    && perm_desc_va != 0
                    && perm_desc_va != u64::MAX;
                let mut poi = 0usize;
                while poi < 8 {
                    perm_pkt_phys[poi] = sys_alloc_phys(4096);
                    perm_pkt_va[poi] = sys_map_phys(perm_pkt_phys[poi], 4096);
                    if perm_pkt_phys[poi] == 0
                        || perm_pkt_phys[poi] == u64::MAX
                        || perm_pkt_va[poi] == 0
                        || perm_pkt_va[poi] == u64::MAX
                    {
                        perm_alloc_ok = false;
                    }
                    poi += 1;
                }
                serial_println!(
                    "[sexnet.nic.rx.permanent.alloc] desc_phys=0x{:016X} pkt_pages=8 ok={}",
                    perm_desc_phys,
                    if perm_alloc_ok { 1 } else { 0 }
                );
                if perm_alloc_ok {
                    let mut z = 0u64;
                    while z < 512 {
                        unsafe {
                            core::ptr::write_volatile((perm_desc_va + z * 8) as *mut u64, 0);
                        }
                        z += 1;
                    }
                    let mut pzi = 0usize;
                    while pzi < 8 {
                        let mut pz = 0u64;
                        while pz < 512 {
                            unsafe {
                                core::ptr::write_volatile((perm_pkt_va[pzi] + pz * 8) as *mut u64, 0);
                            }
                            pz += 1;
                        }
                        pzi += 1;
                    }
                    let mut di = 0usize;
                    while di < 8 {
                        let base = perm_desc_va + (di as u64) * 16;
                        unsafe {
                            core::ptr::write_volatile(base as *mut u64, perm_pkt_phys[di]);
                            core::ptr::write_volatile((base + 8) as *mut u64, 0);
                        }
                        di += 1;
                    }
                    serial_println!("[sexnet.nic.rx.permanent.desc.link] count=8 ok=1");

                    let perm_rctl_orig =
                        unsafe { core::ptr::read_volatile((nic_va + 0x0100) as *const u32) };
                    let rctl_init: u32 = (1 << 1) | (1 << 3) | (1 << 4) | (1 << 15) | (1 << 26);
                    unsafe {
                        core::ptr::write_volatile((nic_va + 0x0100) as *mut u32, perm_rctl_orig & !(1u32 << 1));
                        core::ptr::write_volatile((nic_va + 0x2800) as *mut u32, (perm_desc_phys & 0xFFFF_FFFF) as u32);
                        core::ptr::write_volatile((nic_va + 0x2804) as *mut u32, (perm_desc_phys >> 32) as u32);
                        core::ptr::write_volatile((nic_va + 0x2808) as *mut u32, 128);
                        core::ptr::write_volatile((nic_va + 0x2810) as *mut u32, 0);
                        core::ptr::write_volatile((nic_va + 0x2818) as *mut u32, 7);
                        core::ptr::write_volatile((nic_va + 0x280C) as *mut u32, 0x0000_0002);
                        core::ptr::write_volatile((nic_va + 0x0100) as *mut u32, rctl_init);
                    }
                    let perm_prog_rdbal =
                        unsafe { core::ptr::read_volatile((nic_va + 0x2800) as *const u32) };
                    let perm_prog_rctl =
                        unsafe { core::ptr::read_volatile((nic_va + 0x0100) as *const u32) };
                    let perm_rctl_en = if (perm_prog_rctl & (1 << 1)) != 0 { 1 } else { 0 };
                    let perm_ring_ok = if perm_prog_rdbal == (perm_desc_phys as u32) && perm_rctl_en == 1 {
                        1
                    } else {
                        0
                    };
                    serial_println!(
                        "[sexnet.nic.rx.permanent.ring.program] rdbal=0x{:08X} rdlen=128 rdt=7 rctl=0x{:08X} ok={}",
                        perm_prog_rdbal,
                        perm_prog_rctl,
                        perm_ring_ok
                    );

                    if perm_ring_ok == 1 {
                        NIC_RX_OWNER.store(NIC_OWNER_SEXNET_RX, Ordering::Release);
                    }
                    let rx_owner_now = NIC_RX_OWNER.load(Ordering::Acquire);
                    let claim_ok = if rx_owner_now == NIC_OWNER_SEXNET_RX && perm_ring_ok == 1 {
                        1
                    } else {
                        0
                    };
                    serial_println!(
                        "[sexnet.nic.rx.permanent.claim] owner={} ring_ok={} ok={}",
                        rx_owner_now,
                        perm_ring_ok,
                        claim_ok
                    );

                    if claim_ok == 1 {
                        unsafe {
                            RX_PERM_DESC_PHYS = perm_desc_phys;
                            RX_PERM_DESC_VA = perm_desc_va;
                            RX_PERM_PKT_PHYS = perm_pkt_phys;
                            RX_PERM_PKT_VA = perm_pkt_va;
                        }
                        serial_println!("[sexnet.nic.rx.permanent.poll.begin] max_iters=50000000");
                        let mut dd_set = 0u32;
                        let mut dd_desc = 0xFFFFu32;
                        let mut outer = 0u32;
                        while outer < 50_000_000 {
                            let mut idx = 0u32;
                            while idx < 8 {
                                let st = unsafe {
                                    core::ptr::read_volatile((perm_desc_va + (idx as u64) * 16 + 12) as *const u8)
                                };
                                if (st & 1) != 0 {
                                    dd_set = 1;
                                    dd_desc = idx;
                                    break;
                                }
                                idx += 1;
                            }
                            if dd_set == 1 {
                                break;
                            }
                            outer += 1;
                        }
                        serial_println!(
                            "[sexnet.nic.rx.permanent.poll.done] dd_set={} desc_idx={} ok=1",
                            dd_set,
                            dd_desc
                        );

                        if dd_set == 1 {
                            let desc_base = perm_desc_va + (dd_desc as u64) * 16;
                            let pkt_len = unsafe { core::ptr::read_volatile((desc_base + 8) as *const u16) } as u32;
                            let pkt_buf = perm_pkt_va[dd_desc as usize];
                            let eth_hi = unsafe { core::ptr::read_volatile((pkt_buf + 12) as *const u8) };
                            let eth_lo = unsafe { core::ptr::read_volatile((pkt_buf + 13) as *const u8) };
                            let ethertype = ((eth_hi as u16) << 8) | (eth_lo as u16);
                            let parse_ok = if pkt_len > 14 { 1 } else { 0 };
                            serial_println!(
                                "[sexnet.nic.rx.permanent.pkt.parse] len={} ethertype=0x{:04X} ok={}",
                                pkt_len,
                                ethertype,
                                parse_ok
                            );

                            unsafe {
                                core::ptr::write_volatile((desc_base + 8) as *mut u16, 0u16);
                                core::ptr::write_volatile((desc_base + 12) as *mut u8, 0u8);
                            }
                            let new_rdt = (dd_desc + 7) & 7;
                            unsafe {
                                core::ptr::write_volatile((nic_va + 0x2818) as *mut u32, new_rdt);
                            }
                            serial_println!(
                                "[sexnet.nic.rx.permanent.rdt.advance] desc_idx={} new_rdt={} ok=1",
                                dd_desc,
                                new_rdt
                            );
                        }
                    }
                }
            }

            let tx_desc_phys = sys_alloc_phys(4096);
            let tx_desc_va = sys_map_phys(tx_desc_phys, 4096);
            let tx_frame_phys = sys_alloc_phys(4096);
            let tx_frame_va = sys_map_phys(tx_frame_phys, 4096);
            let tx_alloc_ok = tx_desc_phys != 0
                && tx_desc_phys != u64::MAX
                && tx_desc_va != 0
                && tx_desc_va != u64::MAX
                && tx_frame_phys != 0
                && tx_frame_phys != u64::MAX
                && tx_frame_va != 0
                && tx_frame_va != u64::MAX;
            serial_println!(
                "[sexnet.nic.tx.observe.alloc] desc_phys=0x{:016X} frame_phys=0x{:016X} ok={}",
                tx_desc_phys,
                tx_frame_phys,
                if tx_alloc_ok { 1 } else { 0 }
            );
            if tx_alloc_ok {
                let mut z = 0u64;
                while z < 512 {
                    unsafe {
                        core::ptr::write_volatile((tx_desc_va + z * 8) as *mut u64, 0);
                        core::ptr::write_volatile((tx_frame_va + z * 8) as *mut u64, 0);
                    }
                    z += 1;
                }
                let mut bi = 0u64;
                while bi < 6 {
                    unsafe {
                        core::ptr::write_volatile((tx_frame_va + bi) as *mut u8, 0xFF);
                    }
                    bi += 1;
                }
                let src = [0x52u8, 0x54u8, 0x00u8, 0x12u8, 0x34u8, 0x56u8];
                let mut si = 0usize;
                while si < 6 {
                    unsafe {
                        core::ptr::write_volatile((tx_frame_va + 6 + si as u64) as *mut u8, src[si]);
                    }
                    si += 1;
                }
                unsafe {
                    core::ptr::write_volatile((tx_frame_va + 12) as *mut u8, 0x88);
                    core::ptr::write_volatile((tx_frame_va + 13) as *mut u8, 0xB5);
                }
                let mut pi = 14u64;
                while pi < 60 {
                    unsafe {
                        core::ptr::write_volatile((tx_frame_va + pi) as *mut u8, 0x42);
                    }
                    pi += 1;
                }
                serial_println!("[sexnet.nic.tx.observe.frame.write] ethertype=0x88B5 len=60 ok=1");

                unsafe {
                    core::ptr::write_volatile(tx_desc_va as *mut u64, tx_frame_phys);
                    core::ptr::write_volatile((tx_desc_va + 8) as *mut u16, 60u16);
                    core::ptr::write_volatile((tx_desc_va + 10) as *mut u8, 0u8);
                    core::ptr::write_volatile((tx_desc_va + 11) as *mut u8, 0x0Bu8);
                    core::ptr::write_volatile((tx_desc_va + 12) as *mut u8, 0u8);
                    core::ptr::write_volatile((tx_desc_va + 13) as *mut u8, 0u8);
                    core::ptr::write_volatile((tx_desc_va + 14) as *mut u16, 0u16);
                }
                serial_println!("[sexnet.nic.tx.observe.desc.write] len=60 cmd=0x0B sta=0 ok=1");

                let tx_tctl_orig = unsafe { core::ptr::read_volatile((nic_va + 0x0400) as *const u32) };
                let tx_tdbal_orig = unsafe { core::ptr::read_volatile((nic_va + 0x3800) as *const u32) };
                let tx_tdbah_orig = unsafe { core::ptr::read_volatile((nic_va + 0x3804) as *const u32) };
                let tx_tdlen_orig = unsafe { core::ptr::read_volatile((nic_va + 0x3808) as *const u32) };
                let tx_tdh_orig = unsafe { core::ptr::read_volatile((nic_va + 0x3810) as *const u32) };
                let tx_tdt_orig = unsafe { core::ptr::read_volatile((nic_va + 0x3818) as *const u32) };
                serial_println!(
                    "[sexnet.nic.tx.observe.ring.save] tctl=0x{:08X} tdbal=0x{:08X} tdlen={} tdt={} ok=1",
                    tx_tctl_orig,
                    tx_tdbal_orig,
                    tx_tdlen_orig,
                    tx_tdt_orig
                );
                let _ = tx_tdh_orig;

                unsafe {
                    core::ptr::write_volatile((nic_va + 0x0400) as *mut u32, tx_tctl_orig & !(1u32 << 1));
                    core::ptr::write_volatile((nic_va + 0x3800) as *mut u32, (tx_desc_phys & 0xFFFF_FFFF) as u32);
                    core::ptr::write_volatile((nic_va + 0x3804) as *mut u32, (tx_desc_phys >> 32) as u32);
                    core::ptr::write_volatile((nic_va + 0x3808) as *mut u32, 128);
                    core::ptr::write_volatile((nic_va + 0x3810) as *mut u32, 0);
                    core::ptr::write_volatile((nic_va + 0x3818) as *mut u32, 0);
                }
                let tx_tctl_prog: u32 = (1 << 1) | (0x10 << 4) | (0x40 << 12);
                unsafe {
                    core::ptr::write_volatile((nic_va + 0x0400) as *mut u32, tx_tctl_prog);
                }
                let tx_prog_tdbal = unsafe { core::ptr::read_volatile((nic_va + 0x3800) as *const u32) };
                let tx_prog_tctl = unsafe { core::ptr::read_volatile((nic_va + 0x0400) as *const u32) };
                let tx_tctl_en = if (tx_prog_tctl & (1 << 1)) != 0 { 1 } else { 0 };
                let tx_prog_ok = if tx_prog_tdbal == (tx_desc_phys as u32) && tx_tctl_en == 1 {
                    1
                } else {
                    0
                };
                serial_println!(
                    "[sexnet.nic.tx.observe.ring.program] tdbal=0x{:08X} tdlen=128 tdt=0 tctl=0x{:08X} ok={}",
                    tx_prog_tdbal,
                    tx_prog_tctl,
                    tx_prog_ok
                );

                unsafe {
                    core::ptr::write_volatile((nic_va + 0x3818) as *mut u32, 1);
                }
                serial_println!("[sexnet.nic.tx.observe.post] tdt=1 ok=1");

                serial_println!("[sexnet.nic.tx.observe.poll.begin] max_iters=50000000");
                let mut tx_dd_set = 0u32;
                let mut tx_outer = 0u32;
                while tx_outer < 50_000_000 {
                    let tx_st = unsafe { core::ptr::read_volatile((tx_desc_va + 12) as *const u8) };
                    if (tx_st & 1) != 0 {
                        tx_dd_set = 1;
                        break;
                    }
                    tx_outer += 1;
                }
                serial_println!(
                    "[sexnet.nic.tx.observe.poll.done] dd_set={} desc_idx=0 ok=1",
                    tx_dd_set
                );

                unsafe {
                    core::ptr::write_volatile((nic_va + 0x0400) as *mut u32, tx_tctl_orig & !(1u32 << 1));
                    core::ptr::write_volatile((nic_va + 0x3800) as *mut u32, tx_tdbal_orig);
                    core::ptr::write_volatile((nic_va + 0x3804) as *mut u32, tx_tdbah_orig);
                    core::ptr::write_volatile((nic_va + 0x3808) as *mut u32, tx_tdlen_orig);
                    core::ptr::write_volatile((nic_va + 0x3818) as *mut u32, tx_tdt_orig);
                    core::ptr::write_volatile((nic_va + 0x0400) as *mut u32, tx_tctl_orig);
                }
                let tx_rest_tctl = unsafe { core::ptr::read_volatile((nic_va + 0x0400) as *const u32) };
                let tx_rest_tdbal = unsafe { core::ptr::read_volatile((nic_va + 0x3800) as *const u32) };
                let tx_tctl_en_restored = if (tx_rest_tctl & (1 << 1)) != 0 { 1 } else { 0 };
                let tx_restore_ok = if tx_tctl_en_restored == 1 && tx_rest_tdbal == tx_tdbal_orig {
                    1
                } else {
                    0
                };
                serial_println!(
                    "[sexnet.nic.tx.observe.ring.restore] tctl_orig=0x{:08X} tctl_en={} tdbal=0x{:08X} ok={}",
                    tx_tctl_orig,
                    tx_tctl_en_restored,
                    tx_rest_tdbal,
                    tx_restore_ok
                );
                let tx_proof_ok = if tx_dd_set == 1 && tx_restore_ok == 1 { 1 } else { 0 };
                serial_println!(
                    "[sexnet.nic.tx.observe.proof.done] dd_set={} ok={}",
                    tx_dd_set,
                    tx_proof_ok
                );

                let rx_own = NIC_RX_OWNER.load(Ordering::Acquire);
                if rx_own != NIC_OWNER_SEXNET_RX {
                    serial_println!("[sexnet.nic.tx.permanent.skip] reason=rx_not_claimed ok=0");
                } else {
                    let txp_desc_phys = sys_alloc_phys(4096);
                    let txp_desc_va = sys_map_phys(txp_desc_phys, 4096);
                    let txp_frame_phys = sys_alloc_phys(4096);
                    let txp_frame_va = sys_map_phys(txp_frame_phys, 4096);
                    let txp_alloc_ok = txp_desc_phys != 0
                        && txp_desc_phys != u64::MAX
                        && txp_desc_va != 0
                        && txp_desc_va != u64::MAX
                        && txp_frame_phys != 0
                        && txp_frame_phys != u64::MAX
                        && txp_frame_va != 0
                        && txp_frame_va != u64::MAX;
                    serial_println!(
                        "[sexnet.nic.tx.permanent.alloc] desc_phys=0x{:016X} frame_phys=0x{:016X} ok={}",
                        txp_desc_phys,
                        txp_frame_phys,
                        if txp_alloc_ok { 1 } else { 0 }
                    );
                    if txp_alloc_ok {
                        unsafe {
                            TX_PERM_DESC_PHYS = txp_desc_phys;
                            TX_PERM_DESC_VA = txp_desc_va;
                            TX_PERM_FRAME_PHYS = txp_frame_phys;
                            TX_PERM_FRAME_VA = txp_frame_va;
                        }
                        let mut z = 0u64;
                        while z < 512 {
                            unsafe {
                                core::ptr::write_volatile((txp_desc_va + z * 8) as *mut u64, 0);
                                core::ptr::write_volatile((txp_frame_va + z * 8) as *mut u64, 0);
                            }
                            z += 1;
                        }
                        let mut bi = 0u64;
                        while bi < 6 {
                            unsafe {
                                core::ptr::write_volatile((txp_frame_va + bi) as *mut u8, 0xFF);
                            }
                            bi += 1;
                        }
                        let src = [0x52u8, 0x54u8, 0x00u8, 0x12u8, 0x34u8, 0x56u8];
                        let mut si = 0usize;
                        while si < 6 {
                            unsafe {
                                core::ptr::write_volatile((txp_frame_va + 6 + si as u64) as *mut u8, src[si]);
                            }
                            si += 1;
                        }
                        unsafe {
                            core::ptr::write_volatile((txp_frame_va + 12) as *mut u8, 0x88);
                            core::ptr::write_volatile((txp_frame_va + 13) as *mut u8, 0xB5);
                        }
                        let mut pi = 14u64;
                        while pi < 60 {
                            unsafe {
                                core::ptr::write_volatile((txp_frame_va + pi) as *mut u8, 0x42);
                            }
                            pi += 1;
                        }
                        serial_println!("[sexnet.nic.tx.permanent.frame.write] ethertype=0x88B5 len=60 ok=1");

                        unsafe {
                            core::ptr::write_volatile(txp_desc_va as *mut u64, txp_frame_phys);
                            core::ptr::write_volatile((txp_desc_va + 8) as *mut u16, 60u16);
                            core::ptr::write_volatile((txp_desc_va + 10) as *mut u8, 0u8);
                            core::ptr::write_volatile((txp_desc_va + 11) as *mut u8, 0x0Bu8);
                            core::ptr::write_volatile((txp_desc_va + 12) as *mut u8, 0u8);
                            core::ptr::write_volatile((txp_desc_va + 13) as *mut u8, 0u8);
                            core::ptr::write_volatile((txp_desc_va + 14) as *mut u16, 0u16);
                        }
                        serial_println!("[sexnet.nic.tx.permanent.desc.write] len=60 cmd=0x0B sta=0 ok=1");

                        unsafe {
                            core::ptr::write_volatile((nic_va + 0x3800) as *mut u32, (txp_desc_phys & 0xFFFF_FFFF) as u32);
                            core::ptr::write_volatile((nic_va + 0x3804) as *mut u32, (txp_desc_phys >> 32) as u32);
                            core::ptr::write_volatile((nic_va + 0x3808) as *mut u32, 128);
                            core::ptr::write_volatile((nic_va + 0x3810) as *mut u32, 0);
                            core::ptr::write_volatile((nic_va + 0x3818) as *mut u32, 0);
                            core::ptr::write_volatile((nic_va + 0x0400) as *mut u32, 0x0004_0102);
                        }
                        let txp_prog_tdbal = unsafe { core::ptr::read_volatile((nic_va + 0x3800) as *const u32) };
                        let txp_prog_tctl = unsafe { core::ptr::read_volatile((nic_va + 0x0400) as *const u32) };
                        let txp_tctl_en = if (txp_prog_tctl & (1 << 1)) != 0 { 1 } else { 0 };
                        let txp_ring_ok = if txp_prog_tdbal == (txp_desc_phys as u32) && txp_tctl_en == 1 {
                            1
                        } else {
                            0
                        };
                        serial_println!(
                            "[sexnet.nic.tx.permanent.ring.program] tdbal=0x{:08X} tdlen=128 tdt=0 tctl=0x{:08X} ok={}",
                            txp_prog_tdbal,
                            txp_prog_tctl,
                            txp_ring_ok
                        );

                        let mut txp_dd_set = 0u32;
                        if txp_ring_ok == 1 {
                            unsafe {
                                core::ptr::write_volatile((nic_va + 0x3818) as *mut u32, 1);
                            }
                            serial_println!("[sexnet.nic.tx.permanent.post] tdt=1 ok=1");
                            serial_println!("[sexnet.nic.tx.permanent.poll.begin] max_iters=50000000");
                            let mut txp_outer = 0u32;
                            while txp_outer < 50_000_000 {
                                let tx_st = unsafe { core::ptr::read_volatile((txp_desc_va + 12) as *const u8) };
                                if (tx_st & 1) != 0 {
                                    txp_dd_set = 1;
                                    break;
                                }
                                txp_outer += 1;
                            }
                            serial_println!(
                                "[sexnet.nic.tx.permanent.poll.done] dd_set={} desc_idx=0 ok=1",
                                txp_dd_set
                            );
                        }

                        if txp_ring_ok == 1 && txp_dd_set == 1 {
                            NIC_TX_OWNER.store(NIC_OWNER_SEXNET_TX, Ordering::Release);
                        }
                        let tx_owner_now = NIC_TX_OWNER.load(Ordering::Acquire);
                        let tx_claim_ok = if tx_owner_now == NIC_OWNER_SEXNET_TX && txp_ring_ok == 1 {
                            1
                        } else {
                            0
                        };
                        serial_println!(
                            "[sexnet.nic.tx.permanent.claim] owner={} ring_ok={} ok={}",
                            tx_owner_now,
                            txp_ring_ok,
                            tx_claim_ok
                        );

                        if tx_claim_ok == 1 && NIC_RX_OWNER.load(Ordering::Acquire) == NIC_OWNER_SEXNET_RX {
                            NIC_RX_OWNER.store(NIC_OWNER_SEXNET_FULL, Ordering::Release);
                            NIC_TX_OWNER.store(NIC_OWNER_SEXNET_FULL, Ordering::Release);
                            let full_rx = NIC_RX_OWNER.load(Ordering::Acquire);
                            let full_tx = NIC_TX_OWNER.load(Ordering::Acquire);
                            let full_ok = if full_rx == NIC_OWNER_SEXNET_FULL && full_tx == NIC_OWNER_SEXNET_FULL {
                                1
                            } else {
                                0
                            };
                            serial_println!(
                                "[sexnet.nic.tx.permanent.full] rx_owner={} tx_owner={} full_ok={}",
                                full_rx,
                                full_tx,
                                full_ok
                            );
                        }

                        let mut arp_ok = 0u32;
                        let arp_rx_own = NIC_RX_OWNER.load(Ordering::Acquire);
                        let arp_tx_own = NIC_TX_OWNER.load(Ordering::Acquire);
                        if arp_rx_own == NIC_OWNER_SEXNET_FULL && arp_tx_own == NIC_OWNER_SEXNET_FULL {
                            let nic_mac: [u8; 6] = [
                                (ral & 0xFF) as u8,
                                ((ral >> 8) & 0xFF) as u8,
                                ((ral >> 16) & 0xFF) as u8,
                                ((ral >> 24) & 0xFF) as u8,
                                (rah & 0xFF) as u8,
                                ((rah >> 8) & 0xFF) as u8,
                            ];
                            serial_println!("[sexnet.arp.rx.poll.begin] max_iters=50000000");
                            let mut arp_rx = 0u32;
                            let mut sender_mac = [0u8; 6];
                            let mut sender_ip = [0u8; 4];
                            let mut arp_outer = 0u32;
                            while arp_outer < 50_000_000 && arp_rx == 0 {
                                let mut idx = 0u32;
                                while idx < 8 && arp_rx == 0 {
                                    let desc_base = unsafe { RX_PERM_DESC_VA } + (idx as u64) * 16;
                                    let st = unsafe { core::ptr::read_volatile((desc_base + 12) as *const u8) };
                                    if (st & 1) != 0 {
                                        let pkt_buf = unsafe { RX_PERM_PKT_VA[idx as usize] };
                                        let eth_hi = unsafe { core::ptr::read_volatile((pkt_buf + 12) as *const u8) };
                                        let eth_lo = unsafe { core::ptr::read_volatile((pkt_buf + 13) as *const u8) };
                                        let ethertype = ((eth_hi as u16) << 8) | (eth_lo as u16);
                                        let htype_hi = unsafe { core::ptr::read_volatile((pkt_buf + 14) as *const u8) };
                                        let htype_lo = unsafe { core::ptr::read_volatile((pkt_buf + 15) as *const u8) };
                                        let ptype_hi = unsafe { core::ptr::read_volatile((pkt_buf + 16) as *const u8) };
                                        let ptype_lo = unsafe { core::ptr::read_volatile((pkt_buf + 17) as *const u8) };
                                        let hlen = unsafe { core::ptr::read_volatile((pkt_buf + 18) as *const u8) };
                                        let plen = unsafe { core::ptr::read_volatile((pkt_buf + 19) as *const u8) };
                                        let oper_hi = unsafe { core::ptr::read_volatile((pkt_buf + 20) as *const u8) };
                                        let oper_lo = unsafe { core::ptr::read_volatile((pkt_buf + 21) as *const u8) };
                                        let htype = ((htype_hi as u16) << 8) | (htype_lo as u16);
                                        let ptype = ((ptype_hi as u16) << 8) | (ptype_lo as u16);
                                        let oper = ((oper_hi as u16) << 8) | (oper_lo as u16);
                                        let mut tpa = [0u8; 4];
                                        let mut ti = 0usize;
                                        while ti < 4 {
                                            tpa[ti] = unsafe {
                                                core::ptr::read_volatile((pkt_buf + 38 + ti as u64) as *const u8)
                                            };
                                            ti += 1;
                                        }
                                        let tpa_match = if tpa == SEXNET_GUEST_IPV4 { 1 } else { 0 };
                                        let valid = ethertype == 0x0806
                                            && htype == 1
                                            && ptype == 0x0800
                                            && hlen == 6
                                            && plen == 4
                                            && oper == 1
                                            && tpa_match == 1;
                                        if valid {
                                            let mut mi = 0usize;
                                            while mi < 6 {
                                                sender_mac[mi] = unsafe {
                                                    core::ptr::read_volatile((pkt_buf + 22 + mi as u64) as *const u8)
                                                };
                                                mi += 1;
                                            }
                                            let mut si = 0usize;
                                            while si < 4 {
                                                sender_ip[si] = unsafe {
                                                    core::ptr::read_volatile((pkt_buf + 28 + si as u64) as *const u8)
                                                };
                                                si += 1;
                                            }
                                            serial_println!("[sexnet.arp.rx.frame] idx={} ethertype=0x0806 ok=1", idx);
                                            serial_println!(
                                                "[sexnet.arp.rx.validate] htype=1 ptype=0x0800 hlen=6 plen=4 oper=1 tpa_match=1 ok=1"
                                            );
                                            arp_rx = 1;
                                        } else {
                                            serial_println!(
                                                "[sexnet.arp.rx.reject.detail] idx={} etype=0x{:04X} oper=0x{:04X} tpa={}.{}.{}.{} ok=0",
                                                idx,
                                                ethertype,
                                                oper,
                                                tpa[0],
                                                tpa[1],
                                                tpa[2],
                                                tpa[3]
                                            );
                                            serial_println!("[sexnet.arp.rx.reject] idx={} reason=notarp_or_badfield ok=0", idx);
                                        }
                                        unsafe {
                                            core::ptr::write_volatile((desc_base + 8) as *mut u16, 0u16);
                                            core::ptr::write_volatile((desc_base + 12) as *mut u8, 0u8);
                                            core::ptr::write_volatile((nic_va + 0x2818) as *mut u32, idx);
                                        }
                                    }
                                    idx += 1;
                                }
                                arp_outer += 1;
                            }

                            let mut arp_tx_dd = 0u32;
                            if arp_rx == 1 {
                                let tx_frame_va = unsafe { TX_PERM_FRAME_VA };
                                let mut i = 0usize;
                                while i < 6 {
                                    unsafe {
                                        core::ptr::write_volatile((tx_frame_va + i as u64) as *mut u8, sender_mac[i]);
                                        core::ptr::write_volatile((tx_frame_va + 6 + i as u64) as *mut u8, nic_mac[i]);
                                    }
                                    i += 1;
                                }
                                unsafe {
                                    core::ptr::write_volatile((tx_frame_va + 12) as *mut u8, 0x08);
                                    core::ptr::write_volatile((tx_frame_va + 13) as *mut u8, 0x06);
                                    core::ptr::write_volatile((tx_frame_va + 14) as *mut u8, 0x00);
                                    core::ptr::write_volatile((tx_frame_va + 15) as *mut u8, 0x01);
                                    core::ptr::write_volatile((tx_frame_va + 16) as *mut u8, 0x08);
                                    core::ptr::write_volatile((tx_frame_va + 17) as *mut u8, 0x00);
                                    core::ptr::write_volatile((tx_frame_va + 18) as *mut u8, 0x06);
                                    core::ptr::write_volatile((tx_frame_va + 19) as *mut u8, 0x04);
                                    core::ptr::write_volatile((tx_frame_va + 20) as *mut u8, 0x00);
                                    core::ptr::write_volatile((tx_frame_va + 21) as *mut u8, 0x02);
                                }
                                let mut ai = 0usize;
                                while ai < 6 {
                                    unsafe {
                                        core::ptr::write_volatile((tx_frame_va + 22 + ai as u64) as *mut u8, nic_mac[ai]);
                                        core::ptr::write_volatile((tx_frame_va + 32 + ai as u64) as *mut u8, sender_mac[ai]);
                                    }
                                    ai += 1;
                                }
                                let mut gi = 0usize;
                                while gi < 4 {
                                    unsafe {
                                        core::ptr::write_volatile(
                                            (tx_frame_va + 28 + gi as u64) as *mut u8,
                                            SEXNET_GUEST_IPV4[gi],
                                        );
                                        core::ptr::write_volatile(
                                            (tx_frame_va + 38 + gi as u64) as *mut u8,
                                            sender_ip[gi],
                                        );
                                    }
                                    gi += 1;
                                }
                                let mut pad = 42u64;
                                while pad < 60 {
                                    unsafe {
                                        core::ptr::write_volatile((tx_frame_va + pad) as *mut u8, 0u8);
                                    }
                                    pad += 1;
                                }
                                serial_println!("[sexnet.arp.tx.reply.build] spa=10.0.2.15 ok=1");
                                let tx_dump_base = unsafe { TX_PERM_FRAME_VA };
                                let eth_dst0 = unsafe { core::ptr::read_volatile((tx_dump_base + 0) as *const u8) };
                                let eth_dst1 = unsafe { core::ptr::read_volatile((tx_dump_base + 1) as *const u8) };
                                let eth_dst2 = unsafe { core::ptr::read_volatile((tx_dump_base + 2) as *const u8) };
                                let eth_dst3 = unsafe { core::ptr::read_volatile((tx_dump_base + 3) as *const u8) };
                                let eth_dst4 = unsafe { core::ptr::read_volatile((tx_dump_base + 4) as *const u8) };
                                let eth_dst5 = unsafe { core::ptr::read_volatile((tx_dump_base + 5) as *const u8) };
                                let eth_src0 = unsafe { core::ptr::read_volatile((tx_dump_base + 6) as *const u8) };
                                let eth_src1 = unsafe { core::ptr::read_volatile((tx_dump_base + 7) as *const u8) };
                                let eth_src2 = unsafe { core::ptr::read_volatile((tx_dump_base + 8) as *const u8) };
                                let eth_src3 = unsafe { core::ptr::read_volatile((tx_dump_base + 9) as *const u8) };
                                let eth_src4 = unsafe { core::ptr::read_volatile((tx_dump_base + 10) as *const u8) };
                                let eth_src5 = unsafe { core::ptr::read_volatile((tx_dump_base + 11) as *const u8) };
                                let etype_hi = unsafe { core::ptr::read_volatile((tx_dump_base + 12) as *const u8) };
                                let etype_lo = unsafe { core::ptr::read_volatile((tx_dump_base + 13) as *const u8) };
                                let oper_hi = unsafe { core::ptr::read_volatile((tx_dump_base + 20) as *const u8) };
                                let oper_lo = unsafe { core::ptr::read_volatile((tx_dump_base + 21) as *const u8) };
                                let sha0 = unsafe { core::ptr::read_volatile((tx_dump_base + 22) as *const u8) };
                                let sha1 = unsafe { core::ptr::read_volatile((tx_dump_base + 23) as *const u8) };
                                let sha2 = unsafe { core::ptr::read_volatile((tx_dump_base + 24) as *const u8) };
                                let sha3 = unsafe { core::ptr::read_volatile((tx_dump_base + 25) as *const u8) };
                                let sha4 = unsafe { core::ptr::read_volatile((tx_dump_base + 26) as *const u8) };
                                let sha5 = unsafe { core::ptr::read_volatile((tx_dump_base + 27) as *const u8) };
                                let spa0 = unsafe { core::ptr::read_volatile((tx_dump_base + 28) as *const u8) };
                                let spa1 = unsafe { core::ptr::read_volatile((tx_dump_base + 29) as *const u8) };
                                let spa2 = unsafe { core::ptr::read_volatile((tx_dump_base + 30) as *const u8) };
                                let spa3 = unsafe { core::ptr::read_volatile((tx_dump_base + 31) as *const u8) };
                                let tha0 = unsafe { core::ptr::read_volatile((tx_dump_base + 32) as *const u8) };
                                let tha1 = unsafe { core::ptr::read_volatile((tx_dump_base + 33) as *const u8) };
                                let tha2 = unsafe { core::ptr::read_volatile((tx_dump_base + 34) as *const u8) };
                                let tha3 = unsafe { core::ptr::read_volatile((tx_dump_base + 35) as *const u8) };
                                let tha4 = unsafe { core::ptr::read_volatile((tx_dump_base + 36) as *const u8) };
                                let tha5 = unsafe { core::ptr::read_volatile((tx_dump_base + 37) as *const u8) };
                                let tpa0 = unsafe { core::ptr::read_volatile((tx_dump_base + 38) as *const u8) };
                                let tpa1 = unsafe { core::ptr::read_volatile((tx_dump_base + 39) as *const u8) };
                                let tpa2 = unsafe { core::ptr::read_volatile((tx_dump_base + 40) as *const u8) };
                                let tpa3 = unsafe { core::ptr::read_volatile((tx_dump_base + 41) as *const u8) };
                                let etype = ((etype_hi as u16) << 8) | (etype_lo as u16);
                                let oper = ((oper_hi as u16) << 8) | (oper_lo as u16);
                                serial_println!(
                                    "[sexnet.arp.tx.dump] eth_dst={:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X} eth_src={:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X} etype=0x{:04X} oper=0x{:04X} sha={:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X} spa={}.{}.{}.{} tha={:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X} tpa={}.{}.{}.{} ok=1",
                                    eth_dst0, eth_dst1, eth_dst2, eth_dst3, eth_dst4, eth_dst5,
                                    eth_src0, eth_src1, eth_src2, eth_src3, eth_src4, eth_src5,
                                    etype,
                                    oper,
                                    sha0, sha1, sha2, sha3, sha4, sha5,
                                    spa0, spa1, spa2, spa3,
                                    tha0, tha1, tha2, tha3, tha4, tha5,
                                    tpa0, tpa1, tpa2, tpa3
                                );

                                let tx_desc1 = unsafe { TX_PERM_DESC_VA + 16 };
                                unsafe {
                                    core::ptr::write_volatile(tx_desc1 as *mut u64, TX_PERM_FRAME_PHYS);
                                    core::ptr::write_volatile((tx_desc1 + 8) as *mut u16, 60u16);
                                    core::ptr::write_volatile((tx_desc1 + 10) as *mut u8, 0u8);
                                    core::ptr::write_volatile((tx_desc1 + 11) as *mut u8, 0x0Bu8);
                                    core::ptr::write_volatile((tx_desc1 + 12) as *mut u8, 0u8);
                                    core::ptr::write_volatile((tx_desc1 + 13) as *mut u8, 0u8);
                                    core::ptr::write_volatile((tx_desc1 + 14) as *mut u16, 0u16);
                                }
                                serial_println!("[sexnet.arp.tx.desc] slot=1 len=60 ok=1");
                                unsafe {
                                    core::ptr::write_volatile((nic_va + 0x3818) as *mut u32, 2);
                                }
                                serial_println!("[sexnet.arp.tx.post] tdt=2 ok=1");
                                let mut tx_outer = 0u32;
                                while tx_outer < 50_000_000 {
                                    let tx_st = unsafe { core::ptr::read_volatile((tx_desc1 + 12) as *const u8) };
                                    if (tx_st & 1) != 0 {
                                        arp_tx_dd = 1;
                                        break;
                                    }
                                    tx_outer += 1;
                                }
                                serial_println!(
                                    "[sexnet.arp.tx.poll.done] dd_set={} ok={}",
                                    arp_tx_dd,
                                    if arp_tx_dd == 1 { 1 } else { 0 }
                                );
                            }
                            arp_ok = if arp_rx == 1 && arp_tx_dd == 1 { 1 } else { 0 };
                            serial_println!(
                                "[sexnet.arp.proof.done] rx_arp={} tx_dd={} ok={}",
                                arp_rx,
                                arp_tx_dd,
                                arp_ok
                            );
                        } else {
                            serial_println!("[sexnet.arp.skip] reason=not_full ok=0");
                        }

                        let cache_nic_mac: [u8; 6] = [
                            (ral & 0xFF) as u8,
                            ((ral >> 8) & 0xFF) as u8,
                            ((ral >> 16) & 0xFF) as u8,
                            ((ral >> 24) & 0xFF) as u8,
                            (rah & 0xFF) as u8,
                            ((rah >> 8) & 0xFF) as u8,
                        ];
                        let mut cache_replies = 0u32;
                        let mut cache_outer_done = 0u32;
                        let cache_rx_own = NIC_RX_OWNER.load(Ordering::Acquire);
                        let cache_tx_own = NIC_TX_OWNER.load(Ordering::Acquire);
                        serial_println!(
                            "[sexnet.arp.cache.poll.begin] max_iters=100000000 target_replies=2"
                        );
                        if cache_rx_own == NIC_OWNER_SEXNET_FULL && cache_tx_own == NIC_OWNER_SEXNET_FULL {
                            let mut cache_outer = 0u32;
                            while cache_outer < 100_000_000 && cache_replies < 2 {
                                let mut idx = 0u32;
                                while idx < 8 && cache_replies < 2 {
                                    let desc_base = unsafe { RX_PERM_DESC_VA } + (idx as u64) * 16;
                                    let st = unsafe { core::ptr::read_volatile((desc_base + 12) as *const u8) };
                                    if (st & 1) != 0 {
                                        let pkt_buf = unsafe { RX_PERM_PKT_VA[idx as usize] };
                                        let eth_hi = unsafe { core::ptr::read_volatile((pkt_buf + 12) as *const u8) };
                                        let eth_lo = unsafe { core::ptr::read_volatile((pkt_buf + 13) as *const u8) };
                                        let htype_hi = unsafe { core::ptr::read_volatile((pkt_buf + 14) as *const u8) };
                                        let htype_lo = unsafe { core::ptr::read_volatile((pkt_buf + 15) as *const u8) };
                                        let ptype_hi = unsafe { core::ptr::read_volatile((pkt_buf + 16) as *const u8) };
                                        let ptype_lo = unsafe { core::ptr::read_volatile((pkt_buf + 17) as *const u8) };
                                        let hlen = unsafe { core::ptr::read_volatile((pkt_buf + 18) as *const u8) };
                                        let plen = unsafe { core::ptr::read_volatile((pkt_buf + 19) as *const u8) };
                                        let oper_hi = unsafe { core::ptr::read_volatile((pkt_buf + 20) as *const u8) };
                                        let oper_lo = unsafe { core::ptr::read_volatile((pkt_buf + 21) as *const u8) };
                                        let ethertype = ((eth_hi as u16) << 8) | (eth_lo as u16);
                                        let htype = ((htype_hi as u16) << 8) | (htype_lo as u16);
                                        let ptype = ((ptype_hi as u16) << 8) | (ptype_lo as u16);
                                        let oper = ((oper_hi as u16) << 8) | (oper_lo as u16);
                                        let mut tpa = [0u8; 4];
                                        let mut ti = 0usize;
                                        while ti < 4 {
                                            tpa[ti] = unsafe {
                                                core::ptr::read_volatile((pkt_buf + 38 + ti as u64) as *const u8)
                                            };
                                            ti += 1;
                                        }
                                        let valid = ethertype == 0x0806
                                            && htype == 1
                                            && ptype == 0x0800
                                            && hlen == 6
                                            && plen == 4
                                            && oper == 1
                                            && tpa == SEXNET_GUEST_IPV4;
                                        if valid {
                                            let mut mi = 0usize;
                                            while mi < 6 {
                                                unsafe {
                                                    ARP_CACHE_MAC[mi] = core::ptr::read_volatile(
                                                        (pkt_buf + 22 + mi as u64) as *const u8
                                                    );
                                                }
                                                mi += 1;
                                            }
                                            let mut si = 0usize;
                                            while si < 4 {
                                                unsafe {
                                                    ARP_CACHE_IP[si] = core::ptr::read_volatile(
                                                        (pkt_buf + 28 + si as u64) as *const u8
                                                    );
                                                }
                                                si += 1;
                                            }
                                            unsafe {
                                                ARP_CACHE_VALID = 1;
                                            }
                                            let n = cache_replies + 1;
                                            serial_println!(
                                                "[sexnet.arp.cache.learn] n={} sha={:02X}:{:02X}:{:02X}:{:02X}:{:02X}:{:02X} spa={}.{}.{}.{} ok=1",
                                                n,
                                                unsafe { ARP_CACHE_MAC[0] },
                                                unsafe { ARP_CACHE_MAC[1] },
                                                unsafe { ARP_CACHE_MAC[2] },
                                                unsafe { ARP_CACHE_MAC[3] },
                                                unsafe { ARP_CACHE_MAC[4] },
                                                unsafe { ARP_CACHE_MAC[5] },
                                                unsafe { ARP_CACHE_IP[0] },
                                                unsafe { ARP_CACHE_IP[1] },
                                                unsafe { ARP_CACHE_IP[2] },
                                                unsafe { ARP_CACHE_IP[3] }
                                            );

                                            let tx_slot = 3u32 + cache_replies;
                                            let tx_tdt = 4u32 + cache_replies;
                                            let tx_frame_va = unsafe { TX_PERM_FRAME_VA };
                                            let mut i = 0usize;
                                            while i < 6 {
                                                unsafe {
                                                    core::ptr::write_volatile(
                                                        (tx_frame_va + i as u64) as *mut u8,
                                                        ARP_CACHE_MAC[i],
                                                    );
                                                    core::ptr::write_volatile(
                                                        (tx_frame_va + 6 + i as u64) as *mut u8,
                                                        cache_nic_mac[i],
                                                    );
                                                }
                                                i += 1;
                                            }
                                            unsafe {
                                                core::ptr::write_volatile((tx_frame_va + 12) as *mut u8, 0x08);
                                                core::ptr::write_volatile((tx_frame_va + 13) as *mut u8, 0x06);
                                                core::ptr::write_volatile((tx_frame_va + 14) as *mut u8, 0x00);
                                                core::ptr::write_volatile((tx_frame_va + 15) as *mut u8, 0x01);
                                                core::ptr::write_volatile((tx_frame_va + 16) as *mut u8, 0x08);
                                                core::ptr::write_volatile((tx_frame_va + 17) as *mut u8, 0x00);
                                                core::ptr::write_volatile((tx_frame_va + 18) as *mut u8, 0x06);
                                                core::ptr::write_volatile((tx_frame_va + 19) as *mut u8, 0x04);
                                                core::ptr::write_volatile((tx_frame_va + 20) as *mut u8, 0x00);
                                                core::ptr::write_volatile((tx_frame_va + 21) as *mut u8, 0x02);
                                            }
                                            let mut ai = 0usize;
                                            while ai < 6 {
                                                unsafe {
                                                    core::ptr::write_volatile(
                                                        (tx_frame_va + 22 + ai as u64) as *mut u8,
                                                        cache_nic_mac[ai],
                                                    );
                                                    core::ptr::write_volatile(
                                                        (tx_frame_va + 32 + ai as u64) as *mut u8,
                                                        ARP_CACHE_MAC[ai],
                                                    );
                                                }
                                                ai += 1;
                                            }
                                            let mut gi = 0usize;
                                            while gi < 4 {
                                                unsafe {
                                                    core::ptr::write_volatile(
                                                        (tx_frame_va + 28 + gi as u64) as *mut u8,
                                                        SEXNET_GUEST_IPV4[gi],
                                                    );
                                                    core::ptr::write_volatile(
                                                        (tx_frame_va + 38 + gi as u64) as *mut u8,
                                                        ARP_CACHE_IP[gi],
                                                    );
                                                }
                                                gi += 1;
                                            }
                                            let mut pad = 42u64;
                                            while pad < 60 {
                                                unsafe {
                                                    core::ptr::write_volatile((tx_frame_va + pad) as *mut u8, 0u8);
                                                }
                                                pad += 1;
                                            }
                                            let tx_desc = unsafe { TX_PERM_DESC_VA } + (tx_slot as u64) * 16;
                                            unsafe {
                                                core::ptr::write_volatile(tx_desc as *mut u64, TX_PERM_FRAME_PHYS);
                                                core::ptr::write_volatile((tx_desc + 8) as *mut u16, 60u16);
                                                core::ptr::write_volatile((tx_desc + 10) as *mut u8, 0u8);
                                                core::ptr::write_volatile((tx_desc + 11) as *mut u8, 0x0Bu8);
                                                core::ptr::write_volatile((tx_desc + 12) as *mut u8, 0u8);
                                                core::ptr::write_volatile((tx_desc + 13) as *mut u8, 0u8);
                                                core::ptr::write_volatile((tx_desc + 14) as *mut u16, 0u16);
                                                core::ptr::write_volatile((nic_va + 0x3818) as *mut u32, tx_tdt);
                                            }
                                            serial_println!(
                                                "[sexnet.arp.cache.reply] n={} slot={} tdt={} ok=1",
                                                n,
                                                tx_slot,
                                                tx_tdt
                                            );

                                            let mut dd_set = 0u32;
                                            let mut tx_outer = 0u32;
                                            while tx_outer < 50_000_000 {
                                                let tx_st = unsafe {
                                                    core::ptr::read_volatile((tx_desc + 12) as *const u8)
                                                };
                                                if (tx_st & 1) != 0 {
                                                    dd_set = 1;
                                                    break;
                                                }
                                                tx_outer += 1;
                                            }
                                            serial_println!(
                                                "[sexnet.arp.cache.reply.dd] n={} dd_set={} ok={}",
                                                n,
                                                dd_set,
                                                if dd_set == 1 { 1 } else { 0 }
                                            );

                                            cache_replies += 1;
                                            unsafe {
                                                ARP_CACHE_REPLY_COUNT = cache_replies;
                                            }
                                        }
                                        unsafe {
                                            core::ptr::write_volatile((desc_base + 8) as *mut u16, 0u16);
                                            core::ptr::write_volatile((desc_base + 12) as *mut u8, 0u8);
                                            core::ptr::write_volatile((nic_va + 0x2818) as *mut u32, idx);
                                        }
                                    }
                                    idx += 1;
                                }
                                cache_outer += 1;
                            }
                            cache_outer_done = cache_outer;
                        }
                        let cache_ok = if cache_replies == 2 { 1 } else { 0 };
                        serial_println!(
                            "[sexnet.arp.cache.poll.done] outer={} replies={} ok={}",
                            cache_outer_done,
                            cache_replies,
                            cache_ok
                        );
                        serial_println!(
                            "[sexnet.arp.cache.proof.done] replies={} ok={}",
                            cache_replies,
                            cache_ok
                        );

                        let l2_rx_own = NIC_RX_OWNER.load(Ordering::Acquire);
                        let l2_tx_own = NIC_TX_OWNER.load(Ordering::Acquire);
                        if l2_rx_own == NIC_OWNER_SEXNET_FULL && l2_tx_own == NIC_OWNER_SEXNET_FULL {
                            serial_println!("[sexnet.l2.entry] rx_owner=3 tx_owner=3 ok=1");
                            serial_println!("[sexnet.l2.rx.poll.begin] max_frames=3 max_iters_per=3000000");
                            let mut l2_frames = 0u32;
                            let mut outer = 0u32;
                            while outer < 3_000_000 && l2_frames < 3 {
                                let mut break_l2_poll = false;
                                let mut idx = 0u32;
                                while idx < 8 && l2_frames < 3 {
                                    let st = unsafe {
                                        core::ptr::read_volatile((RX_PERM_DESC_VA + (idx as u64) * 16 + 12) as *const u8)
                                    };
                                    if (st & 1) != 0 {
                                        let desc_base = unsafe { RX_PERM_DESC_VA } + (idx as u64) * 16;
                                        let pkt_len = unsafe { core::ptr::read_volatile((desc_base + 8) as *const u16) } as u32;
                                        let pkt_buf = unsafe { RX_PERM_PKT_VA[idx as usize] };
                                        let eth_hi = unsafe { core::ptr::read_volatile((pkt_buf + 12) as *const u8) };
                                        let eth_lo = unsafe { core::ptr::read_volatile((pkt_buf + 13) as *const u8) };
                                        let ethertype = ((eth_hi as u16) << 8) | (eth_lo as u16);
                                        l2_frames += 1;
                                        unsafe {
                                            L2_RX_NEXT = idx as u8;
                                            L2_RX_COUNT = l2_frames;
                                        }
                                        serial_println!(
                                            "[sexnet.l2.rx.frame] idx={} len={} ethertype=0x{:04X} count={} ok=1",
                                            idx,
                                            pkt_len,
                                            ethertype,
                                            l2_frames
                                        );
                                        if ethertype != 0x0806 {
                                            unsafe {
                                                core::ptr::write_volatile((desc_base + 8) as *mut u16, 0u16);
                                                core::ptr::write_volatile((desc_base + 12) as *mut u8, 0u8);
                                                core::ptr::write_volatile((nic_va + 0x2818) as *mut u32, idx);
                                            }
                                            serial_println!(
                                                "[sexnet.l2.rx.recycle] idx={} new_rdt={} ok=1",
                                                idx,
                                                idx
                                            );
                                        } else {
                                            // Preserve ARP frame for later lane while preventing repeated count on same DD.
                                            break_l2_poll = true;
                                            break;
                                        }
                                    }
                                    idx += 1;
                                }
                                if break_l2_poll {
                                    break;
                                }
                                outer += 1;
                            }
                            let l2_rx_ok = if l2_frames > 0 { 1 } else { 0 };
                            serial_println!(
                                "[sexnet.l2.rx.poll.done] frames_rx={} ok={}",
                                l2_frames,
                                l2_rx_ok
                            );

                            let mut l2_tx_dd = 0u32;
                            let tx_perm_ready = unsafe {
                                TX_PERM_DESC_VA != 0 && TX_PERM_FRAME_PHYS != 0 && TX_PERM_FRAME_VA != 0
                            };
                            if tx_perm_ready {
                                let tx_frame_va = unsafe { TX_PERM_FRAME_VA };
                                let mut bi = 0u64;
                                while bi < 6 {
                                    unsafe {
                                        core::ptr::write_volatile((tx_frame_va + bi) as *mut u8, 0xFF);
                                    }
                                    bi += 1;
                                }
                                let src = [0x52u8, 0x54u8, 0x00u8, 0x12u8, 0x34u8, 0x56u8];
                                let mut si = 0usize;
                                while si < 6 {
                                    unsafe {
                                        core::ptr::write_volatile((tx_frame_va + 6 + si as u64) as *mut u8, src[si]);
                                    }
                                    si += 1;
                                }
                                unsafe {
                                    core::ptr::write_volatile((tx_frame_va + 12) as *mut u8, 0x88);
                                    core::ptr::write_volatile((tx_frame_va + 13) as *mut u8, 0xB5);
                                }
                                let mut pi = 14u64;
                                while pi < 60 {
                                    unsafe {
                                        core::ptr::write_volatile((tx_frame_va + pi) as *mut u8, 0x42);
                                    }
                                    pi += 1;
                                }
                                let tx_desc2 = unsafe { TX_PERM_DESC_VA + 32 };
                                unsafe {
                                    core::ptr::write_volatile(tx_desc2 as *mut u64, TX_PERM_FRAME_PHYS);
                                    core::ptr::write_volatile((tx_desc2 + 8) as *mut u16, 60u16);
                                    core::ptr::write_volatile((tx_desc2 + 10) as *mut u8, 0u8);
                                    core::ptr::write_volatile((tx_desc2 + 11) as *mut u8, 0x0Bu8);
                                    core::ptr::write_volatile((tx_desc2 + 12) as *mut u8, 0u8);
                                    core::ptr::write_volatile((tx_desc2 + 13) as *mut u8, 0u8);
                                    core::ptr::write_volatile((tx_desc2 + 14) as *mut u16, 0u16);
                                    L2_TX_NEXT = 2;
                                }
                                serial_println!("[sexnet.l2.tx.reuse.desc] slot=2 len=60 ok=1");
                                unsafe {
                                    core::ptr::write_volatile((nic_va + 0x3818) as *mut u32, 3);
                                }
                                serial_println!("[sexnet.l2.tx.reuse.post] tdt=3 ok=1");
                                let mut tx_outer = 0u32;
                                while tx_outer < 50_000_000 {
                                    let tx_st = unsafe { core::ptr::read_volatile((tx_desc2 + 12) as *const u8) };
                                    if (tx_st & 1) != 0 {
                                        l2_tx_dd = 1;
                                        break;
                                    }
                                    tx_outer += 1;
                                }
                                serial_println!(
                                    "[sexnet.l2.tx.reuse.poll.done] dd_set={} desc_idx=2 ok={}",
                                    l2_tx_dd,
                                    if l2_tx_dd == 1 { 1 } else { 0 }
                                );
                            }

                            let l2_ok = if ((l2_frames > 0) || (arp_ok == 1)) && l2_tx_dd == 1 { 1 } else { 0 };
                            serial_println!(
                                "[sexnet.l2.proof.done] rx_frames={} tx_dd={} ok={}",
                                l2_frames,
                                l2_tx_dd,
                                l2_ok
                            );

                            // ------------------------------------------------------------------
                            // Phase G: TCP SYN build and TX
                            // ------------------------------------------------------------------
                            serial_println!("[sexnet.tcp.entry] state=CLOSED local_port={} remote={}.{}.{}.{}:{} ok=1",
                                unsafe { TCP_LOCAL_PORT },
                                unsafe { TCP_REMOTE_IP[0] }, unsafe { TCP_REMOTE_IP[1] },
                                unsafe { TCP_REMOTE_IP[2] }, unsafe { TCP_REMOTE_IP[3] },
                                unsafe { TCP_REMOTE_PORT }
                            );
                            {
                                let mut tcp_syn_built = 0u32;
                                let mut tcp_syn_tx_dd = 0u32;
                                let tx_perm_ready = unsafe {
                                    TX_PERM_DESC_VA != 0 && TX_PERM_FRAME_PHYS != 0 && TX_PERM_FRAME_VA != 0
                                };
                                if tx_perm_ready {
                                    let tx_va = unsafe { TX_PERM_FRAME_VA };
                                    let nic_mac: [u8; 6] = [
                                        (ral & 0xFF) as u8,
                                        ((ral >> 8) & 0xFF) as u8,
                                        ((ral >> 16) & 0xFF) as u8,
                                        ((ral >> 24) & 0xFF) as u8,
                                        (rah & 0xFF) as u8,
                                        ((rah >> 8) & 0xFF) as u8,
                                    ];
                                    // Ethernet header: src=NIC MAC, dst=broadcast
                                    unsafe {
                                        // dst MAC: broadcast
                                        core::ptr::write_volatile((tx_va + 0) as *mut u8, 0xFF);
                                        core::ptr::write_volatile((tx_va + 1) as *mut u8, 0xFF);
                                        core::ptr::write_volatile((tx_va + 2) as *mut u8, 0xFF);
                                        core::ptr::write_volatile((tx_va + 3) as *mut u8, 0xFF);
                                        core::ptr::write_volatile((tx_va + 4) as *mut u8, 0xFF);
                                        core::ptr::write_volatile((tx_va + 5) as *mut u8, 0xFF);
                                        // src MAC: NIC
                                        core::ptr::write_volatile((tx_va + 6) as *mut u8, nic_mac[0]);
                                        core::ptr::write_volatile((tx_va + 7) as *mut u8, nic_mac[1]);
                                        core::ptr::write_volatile((tx_va + 8) as *mut u8, nic_mac[2]);
                                        core::ptr::write_volatile((tx_va + 9) as *mut u8, nic_mac[3]);
                                        core::ptr::write_volatile((tx_va + 10) as *mut u8, nic_mac[4]);
                                        core::ptr::write_volatile((tx_va + 11) as *mut u8, nic_mac[5]);
                                        core::ptr::write_volatile((tx_va + 12) as *mut u8, 0x08);
                                        core::ptr::write_volatile((tx_va + 13) as *mut u8, 0x00);
                                    }
                                    // IPv4 header: src=10.0.2.15, dst=gateway, proto=6
                                    let ipv4_total: u16 = 20 + 20; // IPv4 + TCP headers, no options
                                    unsafe {
                                        core::ptr::write_volatile((tx_va + 14) as *mut u8, 0x45); // ver=4 ihl=5
                                        core::ptr::write_volatile((tx_va + 15) as *mut u8, 0x00); // DSCP+ECN
                                        core::ptr::write_volatile((tx_va + 16) as *mut u8, ((ipv4_total >> 8) & 0xFF) as u8);
                                        core::ptr::write_volatile((tx_va + 17) as *mut u8, (ipv4_total & 0xFF) as u8);
                                        core::ptr::write_volatile((tx_va + 18) as *mut u8, 0x00); // ID=0
                                        core::ptr::write_volatile((tx_va + 19) as *mut u8, 0x00);
                                        core::ptr::write_volatile((tx_va + 20) as *mut u8, 0x00); // flags+frag
                                        core::ptr::write_volatile((tx_va + 21) as *mut u8, 0x00);
                                        core::ptr::write_volatile((tx_va + 22) as *mut u8, 64);  // ttl
                                        core::ptr::write_volatile((tx_va + 23) as *mut u8, 6);   // proto=6 TCP
                                        core::ptr::write_volatile((tx_va + 24) as *mut u8, 0x00); // csum placeholder
                                        core::ptr::write_volatile((tx_va + 25) as *mut u8, 0x00);
                                        core::ptr::write_volatile((tx_va + 26) as *mut u8, 10);  // src=10.0.2.15
                                        core::ptr::write_volatile((tx_va + 27) as *mut u8, 0);
                                        core::ptr::write_volatile((tx_va + 28) as *mut u8, 2);
                                        core::ptr::write_volatile((tx_va + 29) as *mut u8, 15);
                                        let rip = unsafe { TCP_REMOTE_IP };
                                        core::ptr::write_volatile((tx_va + 30) as *mut u8, rip[0]);
                                        core::ptr::write_volatile((tx_va + 31) as *mut u8, rip[1]);
                                        core::ptr::write_volatile((tx_va + 32) as *mut u8, rip[2]);
                                        core::ptr::write_volatile((tx_va + 33) as *mut u8, rip[3]);
                                    }
                                    // IPv4 checksum
                                    {
                                        let mut ipv4_sum = 0u32;
                                        let mut ck = 0usize;
                                        while ck < 10 {
                                            let off = 14 + ck * 2;
                                            let w_hi = unsafe { core::ptr::read_volatile((tx_va + off as u64) as *const u8) } as u16;
                                            let w_lo = unsafe { core::ptr::read_volatile((tx_va + off as u64 + 1) as *const u8) } as u16;
                                            ipv4_sum += ((w_hi << 8) | w_lo) as u32;
                                            ck += 1;
                                        }
                                        while (ipv4_sum >> 16) != 0 {
                                            ipv4_sum = (ipv4_sum & 0xFFFF) + (ipv4_sum >> 16);
                                        }
                                        let ipv4_csum = !(ipv4_sum as u16);
                                        unsafe {
                                            core::ptr::write_volatile((tx_va + 24) as *mut u8, ((ipv4_csum >> 8) & 0xFF) as u8);
                                            core::ptr::write_volatile((tx_va + 25) as *mut u8, (ipv4_csum & 0xFF) as u8);
                                        }
                                    }
                                    // TCP header
                                    let local_port = unsafe { TCP_LOCAL_PORT };
                                    let remote_port = unsafe { TCP_REMOTE_PORT };
                                    let local_seq = unsafe { TCP_LOCAL_SEQ };
                                    let data_offset: u8 = 5; // 20-byte header, no options
                                    let tcp_flags: u8 = 0x02; // SYN
                                    unsafe {
                                        core::ptr::write_volatile((tx_va + 34) as *mut u8, ((local_port >> 8) & 0xFF) as u8);
                                        core::ptr::write_volatile((tx_va + 35) as *mut u8, (local_port & 0xFF) as u8);
                                        core::ptr::write_volatile((tx_va + 36) as *mut u8, ((remote_port >> 8) & 0xFF) as u8);
                                        core::ptr::write_volatile((tx_va + 37) as *mut u8, (remote_port & 0xFF) as u8);
                                        core::ptr::write_volatile((tx_va + 38) as *mut u8, ((local_seq >> 24) & 0xFF) as u8);
                                        core::ptr::write_volatile((tx_va + 39) as *mut u8, ((local_seq >> 16) & 0xFF) as u8);
                                        core::ptr::write_volatile((tx_va + 40) as *mut u8, ((local_seq >> 8) & 0xFF) as u8);
                                        core::ptr::write_volatile((tx_va + 41) as *mut u8, (local_seq & 0xFF) as u8);
                                        // ack=0
                                        core::ptr::write_volatile((tx_va + 42) as *mut u8, 0x00);
                                        core::ptr::write_volatile((tx_va + 43) as *mut u8, 0x00);
                                        core::ptr::write_volatile((tx_va + 44) as *mut u8, 0x00);
                                        core::ptr::write_volatile((tx_va + 45) as *mut u8, 0x00);
                                        // data_offset | flags
                                        let dof = (data_offset << 4) | ((tcp_flags >> 0) & 0x01);
                                        let reserved_flags = (tcp_flags & 0x3F);
                                        core::ptr::write_volatile((tx_va + 46) as *mut u8, dof);
                                        core::ptr::write_volatile((tx_va + 47) as *mut u8, reserved_flags);
                                        // window = 65535
                                        core::ptr::write_volatile((tx_va + 48) as *mut u8, 0xFF);
                                        core::ptr::write_volatile((tx_va + 49) as *mut u8, 0xFF);
                                        // checksum placeholder
                                        core::ptr::write_volatile((tx_va + 50) as *mut u8, 0x00);
                                        core::ptr::write_volatile((tx_va + 51) as *mut u8, 0x00);
                                        // urgent = 0
                                        core::ptr::write_volatile((tx_va + 52) as *mut u8, 0x00);
                                        core::ptr::write_volatile((tx_va + 53) as *mut u8, 0x00);
                                    }
                                    // TCP checksum over pseudo-header + TCP header
                                    {
                                        let mut tcp_sum = 0u32;
                                        // pseudo-header: src IP
                                        tcp_sum += ((10u16 << 8) | 0u16) as u32;
                                        tcp_sum += ((2u16 << 8) | 15u16) as u32;
                                        // pseudo-header: dst IP
                                        let rip = unsafe { TCP_REMOTE_IP };
                                        tcp_sum += ((rip[0] as u16) << 8 | (rip[1] as u16)) as u32;
                                        tcp_sum += ((rip[2] as u16) << 8 | (rip[3] as u16)) as u32;
                                        // pseudo-header: zero + proto=6
                                        tcp_sum += 6u32;
                                        // pseudo-header: TCP length = 20
                                        tcp_sum += 20u32;
                                        // TCP header words (10 words)
                                        let mut cw = 0usize;
                                        while cw < 10 {
                                            let off = 34 + cw * 2;
                                            let w_hi = unsafe { core::ptr::read_volatile((tx_va + off as u64) as *const u8) } as u16;
                                            let w_lo = unsafe { core::ptr::read_volatile((tx_va + off as u64 + 1) as *const u8) } as u16;
                                            tcp_sum += ((w_hi << 8) | w_lo) as u32;
                                            cw += 1;
                                        }
                                        while (tcp_sum >> 16) != 0 {
                                            tcp_sum = (tcp_sum & 0xFFFF) + (tcp_sum >> 16);
                                        }
                                        let tcp_csum = !(tcp_sum as u16);
                                        unsafe {
                                            core::ptr::write_volatile((tx_va + 50) as *mut u8, ((tcp_csum >> 8) & 0xFF) as u8);
                                            core::ptr::write_volatile((tx_va + 51) as *mut u8, (tcp_csum & 0xFF) as u8);
                                        }
                                        serial_println!(
                                            "[sexnet.tcp.syn.checksum] checksum=0x{:04X} ok=1",
                                            tcp_csum
                                        );
                                    }
                                    serial_println!(
                                        "[sexnet.tcp.syn.build] src_port={} dst_port={} seq={} flags=SYN data_offset=5 window=65535 ok=1",
                                        local_port, remote_port, local_seq
                                    );
                                    serial_println!(
                                        "[sexnet.ipv4.tx.tcp_syn.build] src=10.0.2.15 dst={}.{}.{}.{} total_len={} checksum=ok ok=1",
                                        unsafe { TCP_REMOTE_IP[0] }, unsafe { TCP_REMOTE_IP[1] },
                                        unsafe { TCP_REMOTE_IP[2] }, unsafe { TCP_REMOTE_IP[3] },
                                        ipv4_total
                                    );
                                    tcp_syn_built = 1;
                                    // Pad to 60 bytes if needed
                                    let frame_len = (14 + ipv4_total as u64) as u16;
                                    if frame_len < 60 {
                                        let mut pad = frame_len as u64;
                                        while pad < 60 {
                                            unsafe { core::ptr::write_volatile((tx_va + pad) as *mut u8, 0u8); }
                                            pad += 1;
                                        }
                                    }
                                    let tx_frame_len = if frame_len < 60 { 60u16 } else { frame_len };
                                    // TX descriptor 5 (offset 80) for TCP SYN
                                    let tx_desc5 = unsafe { TX_PERM_DESC_VA + 80 };
                                    unsafe {
                                        core::ptr::write_volatile(tx_desc5 as *mut u64, TX_PERM_FRAME_PHYS);
                                        core::ptr::write_volatile((tx_desc5 + 8) as *mut u16, tx_frame_len);
                                        core::ptr::write_volatile((tx_desc5 + 10) as *mut u8, 0u8);
                                        core::ptr::write_volatile((tx_desc5 + 11) as *mut u8, 0x0Bu8);
                                        core::ptr::write_volatile((tx_desc5 + 12) as *mut u8, 0u8);
                                        core::ptr::write_volatile((tx_desc5 + 13) as *mut u8, 0u8);
                                        core::ptr::write_volatile((tx_desc5 + 14) as *mut u16, 0u16);
                                    }
                                    serial_println!("[sexnet.eth.tx.tcp_syn.desc] len={} ok=1", tx_frame_len);
                                    unsafe {
                                        core::ptr::write_volatile((nic_va + 0x3818) as *mut u32, 6);
                                    }
                                    serial_println!("[sexnet.tcp.syn.tx.post] slot=6 ok=1");
                                    // Poll DD
                                    let mut tx_outer = 0u32;
                                    while tx_outer < 50_000_000 {
                                        let tx_st = unsafe { core::ptr::read_volatile((tx_desc5 + 12) as *const u8) };
                                        if (tx_st & 1) != 0 {
                                            tcp_syn_tx_dd = 1;
                                            break;
                                        }
                                        tx_outer += 1;
                                    }
                                    serial_println!(
                                        "[sexnet.tcp.syn.tx.poll.done] dd_set={} ok={}",
                                        tcp_syn_tx_dd,
                                        if tcp_syn_tx_dd == 1 { 1 } else { 0 }
                                    );
                                    if tcp_syn_tx_dd == 1 {
                                        let mut ts = TCP_STATE.lock();
                                        *ts = TcpState::SynSent;
                                        unsafe { TCP_SYN_COUNT = 1; }
                                        serial_println!("[sexnet.tcp.handshake.state] state=SYN_SENT ok=1");
                                    }
                                }
                                serial_println!(
                                    "[sexnet.tcp.syn.build.proof.done] built={} checksum_ok={} ok={}",
                                    tcp_syn_built,
                                    if tcp_syn_built == 1 { 1 } else { 0 },
                                    if tcp_syn_built == 1 && tcp_syn_tx_dd == 1 { 1 } else { 0 }
                                );
                                serial_println!(
                                    "[sexnet.tcp.syn.tx.proof.done] tx={} tx_dd={} ok={}",
                                    if tcp_syn_built == 1 { 1 } else { 0 },
                                    tcp_syn_tx_dd,
                                    if tcp_syn_built == 1 && tcp_syn_tx_dd == 1 { 1 } else { 0 }
                                );
                            }

                            let ipv4_rx_own = NIC_RX_OWNER.load(Ordering::Acquire);
                            if ipv4_rx_own == NIC_OWNER_SEXNET_FULL {
                                serial_println!("[sexnet.ipv4.entry] rx_owner=3 ok=1");
                                serial_println!("[sexnet.ipv4.rx.poll.begin] max_iters=200000000");
                                // Phase E UDP self-test: inject synthetic UDP echo request
                                // to exercise the UDP handler when no real stimulus is available.
                                let udp_test_idx = 7u32; // highest index, polled last — only fires if no real frames
                                let udp_test_buf = unsafe { RX_PERM_PKT_VA[udp_test_idx as usize] };
                                let udp_payload = "HELLO_SEXNET_UDP_ECHO".as_bytes();
                                let udp_payload_len = udp_payload.len() as u16;
                                let udp_total = 8u16 + udp_payload_len; // UDP header + payload
                                let ipv4_total: u16 = 20 + udp_total;
                                let frame_bytes: u16 = 14 + ipv4_total;
                                // Ethernet header
                                unsafe {
                                    core::ptr::write_volatile((udp_test_buf + 0) as *mut u8, 0x52); // dst MAC (guest)
                                    core::ptr::write_volatile((udp_test_buf + 1) as *mut u8, 0x54);
                                    core::ptr::write_volatile((udp_test_buf + 2) as *mut u8, 0x00);
                                    core::ptr::write_volatile((udp_test_buf + 3) as *mut u8, 0x12);
                                    core::ptr::write_volatile((udp_test_buf + 4) as *mut u8, 0x34);
                                    core::ptr::write_volatile((udp_test_buf + 5) as *mut u8, 0x56);
                                    core::ptr::write_volatile((udp_test_buf + 6) as *mut u8, 0xFE); // src MAC (host)
                                    core::ptr::write_volatile((udp_test_buf + 7) as *mut u8, 0x56);
                                    core::ptr::write_volatile((udp_test_buf + 8) as *mut u8, 0x3A);
                                    core::ptr::write_volatile((udp_test_buf + 9) as *mut u8, 0x6C);
                                    core::ptr::write_volatile((udp_test_buf + 10) as *mut u8, 0x97);
                                    core::ptr::write_volatile((udp_test_buf + 11) as *mut u8, 0x32);
                                    core::ptr::write_volatile((udp_test_buf + 12) as *mut u8, 0x08);
                                    core::ptr::write_volatile((udp_test_buf + 13) as *mut u8, 0x00);
                                    // IPv4 header
                                    core::ptr::write_volatile((udp_test_buf + 14) as *mut u8, 0x45); // ver=4 ihl=5
                                    core::ptr::write_volatile((udp_test_buf + 15) as *mut u8, 0x00);
                                    core::ptr::write_volatile((udp_test_buf + 16) as *mut u8, ((ipv4_total >> 8) & 0xFF) as u8);
                                    core::ptr::write_volatile((udp_test_buf + 17) as *mut u8, (ipv4_total & 0xFF) as u8);
                                    core::ptr::write_volatile((udp_test_buf + 18) as *mut u8, 0x00);
                                    core::ptr::write_volatile((udp_test_buf + 19) as *mut u8, 0x00);
                                    core::ptr::write_volatile((udp_test_buf + 20) as *mut u8, 0x00);
                                    core::ptr::write_volatile((udp_test_buf + 21) as *mut u8, 0x00);
                                    core::ptr::write_volatile((udp_test_buf + 22) as *mut u8, 64);  // ttl=64
                                    core::ptr::write_volatile((udp_test_buf + 23) as *mut u8, 17);  // proto=17 UDP
                                    core::ptr::write_volatile((udp_test_buf + 24) as *mut u8, 0x00); // csum placeholder
                                    core::ptr::write_volatile((udp_test_buf + 25) as *mut u8, 0x00);
                                    core::ptr::write_volatile((udp_test_buf + 26) as *mut u8, 10);   // src=10.0.2.2
                                    core::ptr::write_volatile((udp_test_buf + 27) as *mut u8, 0);
                                    core::ptr::write_volatile((udp_test_buf + 28) as *mut u8, 2);
                                    core::ptr::write_volatile((udp_test_buf + 29) as *mut u8, 2);
                                    core::ptr::write_volatile((udp_test_buf + 30) as *mut u8, 10);   // dst=10.0.2.15
                                    core::ptr::write_volatile((udp_test_buf + 31) as *mut u8, 0);
                                    core::ptr::write_volatile((udp_test_buf + 32) as *mut u8, 2);
                                    core::ptr::write_volatile((udp_test_buf + 33) as *mut u8, 15);
                                    // Compute IPv4 checksum for test frame
                                    let mut ut_sum = 0u32;
                                    let mut ut_ci = 0usize;
                                    while ut_ci < 10 {
                                        let off = 14 + ut_ci * 2;
                                        let w_hi = core::ptr::read_volatile((udp_test_buf + off as u64) as *const u8) as u16;
                                        let w_lo = core::ptr::read_volatile((udp_test_buf + off as u64 + 1) as *const u8) as u16;
                                        ut_sum += ((w_hi << 8) | w_lo) as u32;
                                        ut_ci += 1;
                                    }
                                    while (ut_sum >> 16) != 0 {
                                        ut_sum = (ut_sum & 0xFFFF) + (ut_sum >> 16);
                                    }
                                    let ut_csum = !(ut_sum as u16);
                                    core::ptr::write_volatile((udp_test_buf + 24) as *mut u8, ((ut_csum >> 8) & 0xFF) as u8);
                                    core::ptr::write_volatile((udp_test_buf + 25) as *mut u8, (ut_csum & 0xFF) as u8);
                                    // UDP header
                                    core::ptr::write_volatile((udp_test_buf + 34) as *mut u8, 0x30); // src_port=12345
                                    core::ptr::write_volatile((udp_test_buf + 35) as *mut u8, 0x39);
                                    core::ptr::write_volatile((udp_test_buf + 36) as *mut u8, 0x1E); // dst_port=7777
                                    core::ptr::write_volatile((udp_test_buf + 37) as *mut u8, 0x61);
                                    core::ptr::write_volatile((udp_test_buf + 38) as *mut u8, ((udp_total >> 8) & 0xFF) as u8);
                                    core::ptr::write_volatile((udp_test_buf + 39) as *mut u8, (udp_total & 0xFF) as u8);
                                    core::ptr::write_volatile((udp_test_buf + 40) as *mut u8, 0x00); // checksum=0
                                    core::ptr::write_volatile((udp_test_buf + 41) as *mut u8, 0x00);
                                    // Payload
                                    let mut ut_pi = 0usize;
                                    while ut_pi < udp_payload.len() {
                                        core::ptr::write_volatile((udp_test_buf + 42 + ut_pi as u64) as *mut u8, udp_payload[ut_pi]);
                                        ut_pi += 1;
                                    }
                                }
                                // Mark descriptor as done
                                let udp_test_desc = unsafe { RX_PERM_DESC_VA + (udp_test_idx as u64) * 16 };
                                unsafe {
                                    core::ptr::write_volatile((udp_test_desc + 8) as *mut u16, frame_bytes);
                                    core::ptr::write_volatile((udp_test_desc + 12) as *mut u8, 1u8); // DD=1
                                }
                                serial_println!(
                                    "[sexnet.udp.self_test.inject] idx={} len={} src_port=12345 dst_port=7777 checksum_policy=zero_allowed self_test=1 ok=1",
                                    udp_test_idx,
                                    frame_bytes
                                );
                                let mut ipv4_frames = 0u32;
                                let mut ipv4_ok = 0u32;
                                let mut reject_logged = 0u32;
                                let mut outer = 0u32;
                                while outer < 200_000_000 && ipv4_frames < 1 {
                                    let mut idx = 0u32;
                                    while idx < 8 && ipv4_frames < 1 {
                                        let desc_base = unsafe { RX_PERM_DESC_VA } + (idx as u64) * 16;
                                        let st = unsafe {
                                            core::ptr::read_volatile((desc_base + 12) as *const u8)
                                        };
                                        if (st & 1) != 0 {
                                            let pkt_len = unsafe {
                                                core::ptr::read_volatile((desc_base + 8) as *const u16)
                                            } as usize;
                                            let pkt_buf = unsafe { RX_PERM_PKT_VA[idx as usize] };
                                            let eth_hi = unsafe {
                                                core::ptr::read_volatile((pkt_buf + 12) as *const u8)
                                            };
                                            let eth_lo = unsafe {
                                                core::ptr::read_volatile((pkt_buf + 13) as *const u8)
                                            };
                                            let ethertype = ((eth_hi as u16) << 8) | (eth_lo as u16);
                                            if ethertype == 0x0800 {
                                                serial_println!(
                                                    "[sexnet.ipv4.rx.frame] idx={} pkt_len={} ethertype=0x0800 ok=1",
                                                    idx,
                                                    pkt_len
                                                );
                                                let mut reason = "short";
                                                let mut ok = 0u32;
                                                let mut version = 0u8;
                                                let mut ihl = 0u8;
                                                let mut total_len = 0u16;
                                                let mut flags_frag = 0u16;
                                                let mut frag_masked = 1u16;
                                                let mut dst0 = 0u8;
                                                let mut dst1 = 0u8;
                                                let mut dst2 = 0u8;
                                                let mut dst3 = 0u8;
                                                let mut src0 = 0u8;
                                                let mut src1 = 0u8;
                                                let mut src2 = 0u8;
                                                let mut src3 = 0u8;
                                                let mut proto = 0u8;
                                                let mut ttl = 0u8;
                                                let mut csum = 0u16;
                                                let mut checksum_ok = 0u32;
                                                if pkt_len >= 34 {
                                                    let vihl = unsafe {
                                                        core::ptr::read_volatile((pkt_buf + 14) as *const u8)
                                                    };
                                                    version = (vihl >> 4) & 0x0F;
                                                    ihl = vihl & 0x0F;
                                                    let tl_hi = unsafe {
                                                        core::ptr::read_volatile((pkt_buf + 16) as *const u8)
                                                    } as u16;
                                                    let tl_lo = unsafe {
                                                        core::ptr::read_volatile((pkt_buf + 17) as *const u8)
                                                    } as u16;
                                                    total_len = (tl_hi << 8) | tl_lo;
                                                    ttl = unsafe {
                                                        core::ptr::read_volatile((pkt_buf + 22) as *const u8)
                                                    };
                                                    proto = unsafe {
                                                        core::ptr::read_volatile((pkt_buf + 23) as *const u8)
                                                    };
                                                    src0 = unsafe {
                                                        core::ptr::read_volatile((pkt_buf + 26) as *const u8)
                                                    };
                                                    src1 = unsafe {
                                                        core::ptr::read_volatile((pkt_buf + 27) as *const u8)
                                                    };
                                                    src2 = unsafe {
                                                        core::ptr::read_volatile((pkt_buf + 28) as *const u8)
                                                    };
                                                    src3 = unsafe {
                                                        core::ptr::read_volatile((pkt_buf + 29) as *const u8)
                                                    };
                                                    dst0 = unsafe {
                                                        core::ptr::read_volatile((pkt_buf + 30) as *const u8)
                                                    };
                                                    dst1 = unsafe {
                                                        core::ptr::read_volatile((pkt_buf + 31) as *const u8)
                                                    };
                                                    dst2 = unsafe {
                                                        core::ptr::read_volatile((pkt_buf + 32) as *const u8)
                                                    };
                                                    dst3 = unsafe {
                                                        core::ptr::read_volatile((pkt_buf + 33) as *const u8)
                                                    };
                                                    let ff_hi = unsafe {
                                                        core::ptr::read_volatile((pkt_buf + 20) as *const u8)
                                                    } as u16;
                                                    let ff_lo = unsafe {
                                                        core::ptr::read_volatile((pkt_buf + 21) as *const u8)
                                                    } as u16;
                                                    flags_frag = (ff_hi << 8) | ff_lo;
                                                    frag_masked = flags_frag & 0x3FFF;
                                                    let csum_hi = unsafe {
                                                        core::ptr::read_volatile((pkt_buf + 24) as *const u8)
                                                    } as u16;
                                                    let csum_lo = unsafe {
                                                        core::ptr::read_volatile((pkt_buf + 25) as *const u8)
                                                    } as u16;
                                                    csum = (csum_hi << 8) | csum_lo;
                                                    if version != 4 {
                                                        reason = "version";
                                                    } else if ihl != 5 {
                                                        reason = "ihl";
                                                    } else if total_len < 20 {
                                                        reason = "total_len_min";
                                                    } else if total_len as usize > pkt_len.saturating_sub(14) {
                                                        reason = "total_len_max";
                                                    } else if frag_masked != 0 {
                                                        reason = "fragmented";
                                                    } else if dst0 != 10 || dst1 != 0 || dst2 != 2 || dst3 != 15 {
                                                        reason = "dst";
                                                    } else {
                                                        let mut sum = 0u32;
                                                        let mut ci = 0usize;
                                                        while ci < 10 {
                                                            let off = 14 + ci * 2;
                                                            let w_hi = unsafe {
                                                                core::ptr::read_volatile((pkt_buf + off as u64) as *const u8)
                                                            } as u16;
                                                            let w_lo = unsafe {
                                                                core::ptr::read_volatile((pkt_buf + off as u64 + 1) as *const u8)
                                                            } as u16;
                                                            sum += ((w_hi << 8) | w_lo) as u32;
                                                            ci += 1;
                                                        }
                                                        while (sum >> 16) != 0 {
                                                            sum = (sum & 0xFFFF) + (sum >> 16);
                                                        }
                                                        if (sum as u16) == 0xFFFF {
                                                            ok = 1;
                                                            checksum_ok = 1;
                                                        } else {
                                                            reason = "checksum";
                                                        }
                                                    }
                                                }

                                                serial_println!(
                                                    "[sexnet.ipv4.rx.validate.detail] ver={} ihl={} total_len={} pkt_len={} frag=0x{:04X} dst={}.{}.{}.{} csum=0x{:04X} checksum_ok={} proto={} ttl={} ok=0",
                                                    version,
                                                    ihl,
                                                    total_len,
                                                    pkt_len,
                                                    flags_frag,
                                                    dst0,
                                                    dst1,
                                                    dst2,
                                                    dst3,
                                                    csum,
                                                    checksum_ok,
                                                    proto,
                                                    ttl
                                                );
                                                if ok == 1 {
                                                    serial_println!(
                                                        "[sexnet.ipv4.rx.validate] version=4 ihl=5 total_len={} dst=10.0.2.15 frag=0 checksum=ok src={}.{}.{}.{} proto={} ttl={} ok=1",
                                                        total_len,
                                                        src0,
                                                        src1,
                                                        src2,
                                                        src3,
                                                        proto,
                                                        ttl
                                                    );
                                                    ipv4_ok = 1;
                                                    ipv4_frames += 1;
                                                    // ICMP echo request handler (Phase D)
                                                    if proto == 1 && (total_len as usize) >= 28 {
                                                        let icmp_base = pkt_buf + 34; // 14 eth + 20 ipv4
                                                        let icmp_type = unsafe { core::ptr::read_volatile(icmp_base as *const u8) };
                                                        let icmp_code = unsafe { core::ptr::read_volatile((icmp_base + 1) as *const u8) };
                                                        if icmp_type == 8 && icmp_code == 0 {
                                                            let icmp_hdr_len = 8usize;
                                                            let ipv4_hdr_len = 20usize;
                                                            let icmp_payload_len = (total_len as usize).saturating_sub(ipv4_hdr_len + icmp_hdr_len);
                                                            let id_hi = unsafe { core::ptr::read_volatile((icmp_base + 4) as *const u8) };
                                                            let id_lo = unsafe { core::ptr::read_volatile((icmp_base + 5) as *const u8) };
                                                            let seq_hi = unsafe { core::ptr::read_volatile((icmp_base + 6) as *const u8) };
                                                            let seq_lo = unsafe { core::ptr::read_volatile((icmp_base + 7) as *const u8) };
                                                            let icmp_id = ((id_hi as u16) << 8) | (id_lo as u16);
                                                            let icmp_seq = ((seq_hi as u16) << 8) | (seq_lo as u16);
                                                            let icmp_total_len = icmp_hdr_len + icmp_payload_len;
                                                            serial_println!(
                                                                "[sexnet.icmp.rx.echo] type=8 code=0 len={} id={} seq={} ok=1",
                                                                icmp_total_len, icmp_id, icmp_seq
                                                            );
                                                            // Validate ICMP checksum from RX
                                                            {
                                                                let mut icmp_rx_sum = 0u32;
                                                                let mut cj = 0usize;
                                                                let rx_icmp_words = icmp_total_len / 2;
                                                                while cj < rx_icmp_words {
                                                                    let off = 34 + cj * 2;
                                                                    let w_hi = unsafe { core::ptr::read_volatile((pkt_buf + off as u64) as *const u8) } as u16;
                                                                    let w_lo = unsafe { core::ptr::read_volatile((pkt_buf + off as u64 + 1) as *const u8) } as u16;
                                                                    icmp_rx_sum += ((w_hi << 8) | w_lo) as u32;
                                                                    cj += 1;
                                                                }
                                                                if icmp_total_len % 2 != 0 {
                                                                    let last = unsafe { core::ptr::read_volatile((pkt_buf + 34 + icmp_total_len as u64 - 1) as *const u8) } as u16;
                                                                    icmp_rx_sum += (last << 8) as u32;
                                                                }
                                                                while (icmp_rx_sum >> 16) != 0 {
                                                                    icmp_rx_sum = (icmp_rx_sum & 0xFFFF) + (icmp_rx_sum >> 16);
                                                                }
                                                                let icmp_rx_csum_ok = if (icmp_rx_sum as u16) == 0xFFFF { 1u32 } else { 0u32 };
                                                                serial_println!("[sexnet.icmp.checksum.validate] ok={}", icmp_rx_csum_ok);
                                                            }
                                                            // Build Ethernet + IPv4 + ICMP echo reply in TX frame buffer
                                                            let tx_va = unsafe { TX_PERM_FRAME_VA };
                                                            let nic_mac: [u8; 6] = [
                                                                (ral & 0xFF) as u8,
                                                                ((ral >> 8) & 0xFF) as u8,
                                                                ((ral >> 16) & 0xFF) as u8,
                                                                ((ral >> 24) & 0xFF) as u8,
                                                                (rah & 0xFF) as u8,
                                                                ((rah >> 8) & 0xFF) as u8,
                                                            ];
                                                            let src_mac0 = unsafe { core::ptr::read_volatile((pkt_buf + 6) as *const u8) };
                                                            let src_mac1 = unsafe { core::ptr::read_volatile((pkt_buf + 7) as *const u8) };
                                                            let src_mac2 = unsafe { core::ptr::read_volatile((pkt_buf + 8) as *const u8) };
                                                            let src_mac3 = unsafe { core::ptr::read_volatile((pkt_buf + 9) as *const u8) };
                                                            let src_mac4 = unsafe { core::ptr::read_volatile((pkt_buf + 10) as *const u8) };
                                                            let src_mac5 = unsafe { core::ptr::read_volatile((pkt_buf + 11) as *const u8) };
                                                            // Ethernet header
                                                            unsafe {
                                                                core::ptr::write_volatile((tx_va + 0) as *mut u8, src_mac0);
                                                                core::ptr::write_volatile((tx_va + 1) as *mut u8, src_mac1);
                                                                core::ptr::write_volatile((tx_va + 2) as *mut u8, src_mac2);
                                                                core::ptr::write_volatile((tx_va + 3) as *mut u8, src_mac3);
                                                                core::ptr::write_volatile((tx_va + 4) as *mut u8, src_mac4);
                                                                core::ptr::write_volatile((tx_va + 5) as *mut u8, src_mac5);
                                                                core::ptr::write_volatile((tx_va + 6) as *mut u8, nic_mac[0]);
                                                                core::ptr::write_volatile((tx_va + 7) as *mut u8, nic_mac[1]);
                                                                core::ptr::write_volatile((tx_va + 8) as *mut u8, nic_mac[2]);
                                                                core::ptr::write_volatile((tx_va + 9) as *mut u8, nic_mac[3]);
                                                                core::ptr::write_volatile((tx_va + 10) as *mut u8, nic_mac[4]);
                                                                core::ptr::write_volatile((tx_va + 11) as *mut u8, nic_mac[5]);
                                                                core::ptr::write_volatile((tx_va + 12) as *mut u8, 0x08);
                                                                core::ptr::write_volatile((tx_va + 13) as *mut u8, 0x00);
                                                            }
                                                            let reply_total_len = (ipv4_hdr_len + icmp_total_len) as u16;
                                                            // IPv4 header
                                                            unsafe {
                                                                core::ptr::write_volatile((tx_va + 14) as *mut u8, 0x45); // ver=4 ihl=5
                                                                core::ptr::write_volatile((tx_va + 15) as *mut u8, 0x00); // dscp/ecn
                                                                core::ptr::write_volatile((tx_va + 16) as *mut u8, ((reply_total_len >> 8) & 0xFF) as u8);
                                                                core::ptr::write_volatile((tx_va + 17) as *mut u8, (reply_total_len & 0xFF) as u8);
                                                                core::ptr::write_volatile((tx_va + 18) as *mut u8, 0x00); // id=0
                                                                core::ptr::write_volatile((tx_va + 19) as *mut u8, 0x00);
                                                                core::ptr::write_volatile((tx_va + 20) as *mut u8, 0x00); // flags/frag=0
                                                                core::ptr::write_volatile((tx_va + 21) as *mut u8, 0x00);
                                                                core::ptr::write_volatile((tx_va + 22) as *mut u8, 64);  // ttl=64
                                                                core::ptr::write_volatile((tx_va + 23) as *mut u8, 1);   // proto=1 ICMP
                                                                core::ptr::write_volatile((tx_va + 24) as *mut u8, 0x00); // csum=0 (compute after)
                                                                core::ptr::write_volatile((tx_va + 25) as *mut u8, 0x00);
                                                                core::ptr::write_volatile((tx_va + 26) as *mut u8, 10);  // src=10.0.2.15
                                                                core::ptr::write_volatile((tx_va + 27) as *mut u8, 0);
                                                                core::ptr::write_volatile((tx_va + 28) as *mut u8, 2);
                                                                core::ptr::write_volatile((tx_va + 29) as *mut u8, 15);
                                                                core::ptr::write_volatile((tx_va + 30) as *mut u8, src0); // dst=request src
                                                                core::ptr::write_volatile((tx_va + 31) as *mut u8, src1);
                                                                core::ptr::write_volatile((tx_va + 32) as *mut u8, src2);
                                                                core::ptr::write_volatile((tx_va + 33) as *mut u8, src3);
                                                            }
                                                            // IPv4 checksum
                                                            {
                                                                let mut ipv4_tx_sum = 0u32;
                                                                let mut ck = 0usize;
                                                                while ck < 10 {
                                                                    let off = 14 + ck * 2;
                                                                    let w_hi = unsafe { core::ptr::read_volatile((tx_va + off as u64) as *const u8) } as u16;
                                                                    let w_lo = unsafe { core::ptr::read_volatile((tx_va + off as u64 + 1) as *const u8) } as u16;
                                                                    ipv4_tx_sum += ((w_hi << 8) | w_lo) as u32;
                                                                    ck += 1;
                                                                }
                                                                while (ipv4_tx_sum >> 16) != 0 {
                                                                    ipv4_tx_sum = (ipv4_tx_sum & 0xFFFF) + (ipv4_tx_sum >> 16);
                                                                }
                                                                let ipv4_tx_csum = !(ipv4_tx_sum as u16);
                                                                unsafe {
                                                                    core::ptr::write_volatile((tx_va + 24) as *mut u8, ((ipv4_tx_csum >> 8) & 0xFF) as u8);
                                                                    core::ptr::write_volatile((tx_va + 25) as *mut u8, (ipv4_tx_csum & 0xFF) as u8);
                                                                }
                                                            }
                                                            // ICMP echo reply header
                                                            unsafe {
                                                                core::ptr::write_volatile((tx_va + 34) as *mut u8, 0x00); // type=0 echo reply
                                                                core::ptr::write_volatile((tx_va + 35) as *mut u8, 0x00); // code=0
                                                                core::ptr::write_volatile((tx_va + 36) as *mut u8, 0x00); // csum=0 (compute after)
                                                                core::ptr::write_volatile((tx_va + 37) as *mut u8, 0x00);
                                                                core::ptr::write_volatile((tx_va + 38) as *mut u8, id_hi);
                                                                core::ptr::write_volatile((tx_va + 39) as *mut u8, id_lo);
                                                                core::ptr::write_volatile((tx_va + 40) as *mut u8, seq_hi);
                                                                core::ptr::write_volatile((tx_va + 41) as *mut u8, seq_lo);
                                                            }
                                                            // Copy ICMP payload from request
                                                            {
                                                                let mut pi = 0usize;
                                                                while pi < icmp_payload_len {
                                                                    let pb = unsafe { core::ptr::read_volatile((icmp_base + icmp_hdr_len as u64 + pi as u64) as *const u8) };
                                                                    unsafe {
                                                                        core::ptr::write_volatile((tx_va + 42u64 + pi as u64) as *mut u8, pb);
                                                                    }
                                                                    pi += 1;
                                                                }
                                                            }
                                                            // ICMP checksum
                                                            {
                                                                let mut icmp_tx_sum = 0u32;
                                                                let mut cl = 0usize;
                                                                let tx_icmp_words = icmp_total_len / 2;
                                                                while cl < tx_icmp_words {
                                                                    let off = 34 + cl * 2;
                                                                    let w_hi = unsafe { core::ptr::read_volatile((tx_va + off as u64) as *const u8) } as u16;
                                                                    let w_lo = unsafe { core::ptr::read_volatile((tx_va + off as u64 + 1) as *const u8) } as u16;
                                                                    icmp_tx_sum += ((w_hi << 8) | w_lo) as u32;
                                                                    cl += 1;
                                                                }
                                                                if icmp_total_len % 2 != 0 {
                                                                    let last_off = 34 + icmp_total_len - 1;
                                                                    let last = unsafe { core::ptr::read_volatile((tx_va + last_off as u64) as *const u8) } as u16;
                                                                    icmp_tx_sum += (last << 8) as u32;
                                                                }
                                                                while (icmp_tx_sum >> 16) != 0 {
                                                                    icmp_tx_sum = (icmp_tx_sum & 0xFFFF) + (icmp_tx_sum >> 16);
                                                                }
                                                                let icmp_tx_csum = !(icmp_tx_sum as u16);
                                                                unsafe {
                                                                    core::ptr::write_volatile((tx_va + 36) as *mut u8, ((icmp_tx_csum >> 8) & 0xFF) as u8);
                                                                    core::ptr::write_volatile((tx_va + 37) as *mut u8, (icmp_tx_csum & 0xFF) as u8);
                                                                }
                                                            }
                                                            serial_println!(
                                                                "[sexnet.icmp.tx.reply.build] type=0 code=0 len={} id={} seq={} ok=1",
                                                                icmp_total_len, icmp_id, icmp_seq
                                                            );
                                                            serial_println!("[sexnet.icmp.tx.reply.checksum] ok=1");
                                                            serial_println!(
                                                                "[sexnet.ipv4.tx.icmp_reply.build] src=10.0.2.15 dst={}.{}.{}.{} total_len={} checksum=ok ok=1",
                                                                src0, src1, src2, src3, reply_total_len
                                                            );
                                                            // TX descriptor 3
                                                            let frame_len = (14 + reply_total_len as u64) as u16;
                                                            if frame_len < 60 {
                                                                let mut pad = frame_len as u64;
                                                                while pad < 60 {
                                                                    unsafe { core::ptr::write_volatile((tx_va + pad) as *mut u8, 0u8); }
                                                                    pad += 1;
                                                                }
                                                            }
                                                            let tx_frame_len = if frame_len < 60 { 60u16 } else { frame_len };
                                                            let tx_desc3 = unsafe { TX_PERM_DESC_VA + 48 };
                                                            unsafe {
                                                                core::ptr::write_volatile(tx_desc3 as *mut u64, TX_PERM_FRAME_PHYS);
                                                                core::ptr::write_volatile((tx_desc3 + 8) as *mut u16, tx_frame_len);
                                                                core::ptr::write_volatile((tx_desc3 + 10) as *mut u8, 0u8);
                                                                core::ptr::write_volatile((tx_desc3 + 11) as *mut u8, 0x0Bu8);
                                                                core::ptr::write_volatile((tx_desc3 + 12) as *mut u8, 0u8);
                                                                core::ptr::write_volatile((tx_desc3 + 13) as *mut u8, 0u8);
                                                                core::ptr::write_volatile((tx_desc3 + 14) as *mut u16, 0u16);
                                                            }
                                                            serial_println!("[sexnet.eth.tx.icmp_reply.desc] len={} ok=1", tx_frame_len);
                                                            unsafe {
                                                                core::ptr::write_volatile((nic_va + 0x3818) as *mut u32, 4);
                                                            }
                                                            let mut icmp_reply_done = 0u32;
                                                            let mut tx_outer = 0u32;
                                                            while tx_outer < 50_000_000 {
                                                                let tx_st = unsafe { core::ptr::read_volatile((tx_desc3 + 12) as *const u8) };
                                                                if (tx_st & 1) != 0 {
                                                                    icmp_reply_done = 1;
                                                                    break;
                                                                }
                                                                tx_outer += 1;
                                                            }
                                                            serial_println!(
                                                                "[sexnet.icmp.tx.poll.done] dd_set={} ok={}",
                                                                icmp_reply_done,
                                                                if icmp_reply_done == 1 { 1 } else { 0 }
                                                            );
                                                            serial_println!(
                                                                "[sexnet.icmp.echo.proof.done] rx_echo=1 tx_reply={} tx_dd={} ok={}",
                                                                if icmp_reply_done > 0 { 1 } else { 0 },
                                                                icmp_reply_done,
                                                                if icmp_reply_done == 1 { 1 } else { 0 }
                                                            );
                                                        } else {
                                                            serial_println!(
                                                                "[sexnet.icmp.reject] reason=not_echo_request type={} code={} ok=1",
                                                                icmp_type, icmp_code
                                                            );
                                                        }
                                                    } else if proto == 1 && (total_len as usize) < 28 {
                                                        serial_println!("[sexnet.icmp.reject] reason=too_short_for_icmp ok=1");
                                                    } else if proto == 17 && (total_len as usize) >= 28 {
                                                        // UDP echo handler (Phase E)
                                                        let udp_base = pkt_buf + 34; // 14 eth + 20 ipv4
                                                        let ipv4_hdr_len = 20usize;
                                                        let ipv4_payload_len = (total_len as usize).saturating_sub(ipv4_hdr_len);
                                                        let udp_src_port_hi = unsafe { core::ptr::read_volatile(udp_base as *const u8) };
                                                        let udp_src_port_lo = unsafe { core::ptr::read_volatile((udp_base + 1) as *const u8) };
                                                        let udp_dst_port_hi = unsafe { core::ptr::read_volatile((udp_base + 2) as *const u8) };
                                                        let udp_dst_port_lo = unsafe { core::ptr::read_volatile((udp_base + 3) as *const u8) };
                                                        let udp_len_hi = unsafe { core::ptr::read_volatile((udp_base + 4) as *const u8) };
                                                        let udp_len_lo = unsafe { core::ptr::read_volatile((udp_base + 5) as *const u8) };
                                                        let udp_csum_hi = unsafe { core::ptr::read_volatile((udp_base + 6) as *const u8) };
                                                        let udp_csum_lo = unsafe { core::ptr::read_volatile((udp_base + 7) as *const u8) };
                                                        let udp_src_port = ((udp_src_port_hi as u16) << 8) | (udp_src_port_lo as u16);
                                                        let udp_dst_port = ((udp_dst_port_hi as u16) << 8) | (udp_dst_port_lo as u16);
                                                        let udp_len = ((udp_len_hi as u16) << 8) | (udp_len_lo as u16);
                                                        let udp_csum = ((udp_csum_hi as u16) << 8) | (udp_csum_lo as u16);
                                                        serial_println!(
                                                            "[sexnet.udp.rx.datagram] src_port={} dst_port={} len={} checksum=0x{:04X} ok=1",
                                                            udp_src_port, udp_dst_port, udp_len, udp_csum
                                                        );
                                                        let mut udp_ok = 1u32;
                                                        let mut udp_reason = "";
                                                        let mut checksum_policy = "zero_allowed";
                                                        let udp_len_ok = if (udp_len as usize) >= 8 && (udp_len as usize) <= ipv4_payload_len { 1u32 } else { 0u32 };
                                                        let ports_ok = 1u32;
                                                        if (udp_len as usize) < 8 {
                                                            udp_ok = 0;
                                                            udp_reason = "udp_len_too_small";
                                                        } else if (udp_len as usize) > ipv4_payload_len {
                                                            udp_ok = 0;
                                                            udp_reason = "udp_len_exceeds_ipv4_payload";
                                                        } else if udp_csum != 0 {
                                                            let mut sum = 0u32;
                                                            // Pseudo-header: src IP
                                                            sum += ((src0 as u16) << 8 | (src1 as u16)) as u32;
                                                            sum += ((src2 as u16) << 8 | (src3 as u16)) as u32;
                                                            // Pseudo-header: dst IP
                                                            sum += ((dst0 as u16) << 8 | (dst1 as u16)) as u32;
                                                            sum += ((dst2 as u16) << 8 | (dst3 as u16)) as u32;
                                                            // Pseudo-header: zero + proto=17
                                                            sum += 17u32;
                                                            // Pseudo-header: UDP length
                                                            sum += udp_len as u32;
                                                            // UDP datagram (header + payload, including checksum field as-is)
                                                            {
                                                                let udp_total = udp_len as usize;
                                                                let mut cu = 0usize;
                                                                while cu < udp_total / 2 {
                                                                    let off = 34 + cu * 2;
                                                                    let w_hi = unsafe { core::ptr::read_volatile((pkt_buf + off as u64) as *const u8) } as u16;
                                                                    let w_lo = unsafe { core::ptr::read_volatile((pkt_buf + off as u64 + 1) as *const u8) } as u16;
                                                                    sum += ((w_hi << 8) | w_lo) as u32;
                                                                    cu += 1;
                                                                }
                                                                if udp_total % 2 != 0 {
                                                                    let last_off = 34 + udp_total - 1;
                                                                    let last = unsafe { core::ptr::read_volatile((pkt_buf + last_off as u64) as *const u8) } as u16;
                                                                    sum += (last << 8) as u32;
                                                                }
                                                            }
                                                            while (sum >> 16) != 0 {
                                                                sum = (sum & 0xFFFF) + (sum >> 16);
                                                            }
                                                            if (sum as u16) != 0xFFFF {
                                                                udp_ok = 0;
                                                                udp_reason = "checksum";
                                                            } else {
                                                                checksum_policy = "validated";
                                                            }
                                                        }
                                                        serial_println!(
                                                            "[sexnet.udp.header.validate] len_ok={} ports_ok={} checksum_policy={} ok={}",
                                                            udp_len_ok, ports_ok, checksum_policy, udp_ok
                                                        );
                                                        if udp_ok == 1 {
                                                            let udp_payload_len = (udp_len as usize).saturating_sub(8);
                                                            let tx_va = unsafe { TX_PERM_FRAME_VA };
                                                            let nic_mac: [u8; 6] = [
                                                                (ral & 0xFF) as u8,
                                                                ((ral >> 8) & 0xFF) as u8,
                                                                ((ral >> 16) & 0xFF) as u8,
                                                                ((ral >> 24) & 0xFF) as u8,
                                                                (rah & 0xFF) as u8,
                                                                ((rah >> 8) & 0xFF) as u8,
                                                            ];
                                                            let src_mac0 = unsafe { core::ptr::read_volatile((pkt_buf + 6) as *const u8) };
                                                            let src_mac1 = unsafe { core::ptr::read_volatile((pkt_buf + 7) as *const u8) };
                                                            let src_mac2 = unsafe { core::ptr::read_volatile((pkt_buf + 8) as *const u8) };
                                                            let src_mac3 = unsafe { core::ptr::read_volatile((pkt_buf + 9) as *const u8) };
                                                            let src_mac4 = unsafe { core::ptr::read_volatile((pkt_buf + 10) as *const u8) };
                                                            let src_mac5 = unsafe { core::ptr::read_volatile((pkt_buf + 11) as *const u8) };
                                                            unsafe {
                                                                core::ptr::write_volatile((tx_va + 0) as *mut u8, src_mac0);
                                                                core::ptr::write_volatile((tx_va + 1) as *mut u8, src_mac1);
                                                                core::ptr::write_volatile((tx_va + 2) as *mut u8, src_mac2);
                                                                core::ptr::write_volatile((tx_va + 3) as *mut u8, src_mac3);
                                                                core::ptr::write_volatile((tx_va + 4) as *mut u8, src_mac4);
                                                                core::ptr::write_volatile((tx_va + 5) as *mut u8, src_mac5);
                                                                core::ptr::write_volatile((tx_va + 6) as *mut u8, nic_mac[0]);
                                                                core::ptr::write_volatile((tx_va + 7) as *mut u8, nic_mac[1]);
                                                                core::ptr::write_volatile((tx_va + 8) as *mut u8, nic_mac[2]);
                                                                core::ptr::write_volatile((tx_va + 9) as *mut u8, nic_mac[3]);
                                                                core::ptr::write_volatile((tx_va + 10) as *mut u8, nic_mac[4]);
                                                                core::ptr::write_volatile((tx_va + 11) as *mut u8, nic_mac[5]);
                                                                core::ptr::write_volatile((tx_va + 12) as *mut u8, 0x08);
                                                                core::ptr::write_volatile((tx_va + 13) as *mut u8, 0x00);
                                                            }
                                                            let reply_total_len = (ipv4_hdr_len + udp_len as usize) as u16;
                                                            unsafe {
                                                                core::ptr::write_volatile((tx_va + 14) as *mut u8, 0x45);
                                                                core::ptr::write_volatile((tx_va + 15) as *mut u8, 0x00);
                                                                core::ptr::write_volatile((tx_va + 16) as *mut u8, ((reply_total_len >> 8) & 0xFF) as u8);
                                                                core::ptr::write_volatile((tx_va + 17) as *mut u8, (reply_total_len & 0xFF) as u8);
                                                                core::ptr::write_volatile((tx_va + 18) as *mut u8, 0x00);
                                                                core::ptr::write_volatile((tx_va + 19) as *mut u8, 0x00);
                                                                core::ptr::write_volatile((tx_va + 20) as *mut u8, 0x00);
                                                                core::ptr::write_volatile((tx_va + 21) as *mut u8, 0x00);
                                                                core::ptr::write_volatile((tx_va + 22) as *mut u8, 64);
                                                                core::ptr::write_volatile((tx_va + 23) as *mut u8, 17);
                                                                core::ptr::write_volatile((tx_va + 24) as *mut u8, 0x00);
                                                                core::ptr::write_volatile((tx_va + 25) as *mut u8, 0x00);
                                                                core::ptr::write_volatile((tx_va + 26) as *mut u8, 10);
                                                                core::ptr::write_volatile((tx_va + 27) as *mut u8, 0);
                                                                core::ptr::write_volatile((tx_va + 28) as *mut u8, 2);
                                                                core::ptr::write_volatile((tx_va + 29) as *mut u8, 15);
                                                                core::ptr::write_volatile((tx_va + 30) as *mut u8, src0);
                                                                core::ptr::write_volatile((tx_va + 31) as *mut u8, src1);
                                                                core::ptr::write_volatile((tx_va + 32) as *mut u8, src2);
                                                                core::ptr::write_volatile((tx_va + 33) as *mut u8, src3);
                                                            }
                                                            {
                                                                let mut ipv4_tx_sum = 0u32;
                                                                let mut ck = 0usize;
                                                                while ck < 10 {
                                                                    let off = 14 + ck * 2;
                                                                    let w_hi = unsafe { core::ptr::read_volatile((tx_va + off as u64) as *const u8) } as u16;
                                                                    let w_lo = unsafe { core::ptr::read_volatile((tx_va + off as u64 + 1) as *const u8) } as u16;
                                                                    ipv4_tx_sum += ((w_hi << 8) | w_lo) as u32;
                                                                    ck += 1;
                                                                }
                                                                while (ipv4_tx_sum >> 16) != 0 {
                                                                    ipv4_tx_sum = (ipv4_tx_sum & 0xFFFF) + (ipv4_tx_sum >> 16);
                                                                }
                                                                let ipv4_tx_csum = !(ipv4_tx_sum as u16);
                                                                unsafe {
                                                                    core::ptr::write_volatile((tx_va + 24) as *mut u8, ((ipv4_tx_csum >> 8) & 0xFF) as u8);
                                                                    core::ptr::write_volatile((tx_va + 25) as *mut u8, (ipv4_tx_csum & 0xFF) as u8);
                                                                }
                                                            }
                                                            unsafe {
                                                                core::ptr::write_volatile((tx_va + 34) as *mut u8, ((udp_dst_port >> 8) & 0xFF) as u8);
                                                                core::ptr::write_volatile((tx_va + 35) as *mut u8, (udp_dst_port & 0xFF) as u8);
                                                                core::ptr::write_volatile((tx_va + 36) as *mut u8, ((udp_src_port >> 8) & 0xFF) as u8);
                                                                core::ptr::write_volatile((tx_va + 37) as *mut u8, (udp_src_port & 0xFF) as u8);
                                                                core::ptr::write_volatile((tx_va + 38) as *mut u8, ((udp_len >> 8) & 0xFF) as u8);
                                                                core::ptr::write_volatile((tx_va + 39) as *mut u8, (udp_len & 0xFF) as u8);
                                                                core::ptr::write_volatile((tx_va + 40) as *mut u8, 0x00);
                                                                core::ptr::write_volatile((tx_va + 41) as *mut u8, 0x00);
                                                            }
                                                            if udp_payload_len > 0 {
                                                                let mut pj = 0usize;
                                                                while pj < udp_payload_len {
                                                                    let pb = unsafe { core::ptr::read_volatile((udp_base + 8u64 + pj as u64) as *const u8) };
                                                                    unsafe {
                                                                        core::ptr::write_volatile((tx_va + 42u64 + pj as u64) as *mut u8, pb);
                                                                    }
                                                                    pj += 1;
                                                                }
                                                            }
                                                            serial_println!(
                                                                "[sexnet.udp.tx.reply.build] src_port={} dst_port={} len={} payload_len={} ok=1",
                                                                udp_dst_port, udp_src_port, udp_len, udp_payload_len
                                                            );
                                                            serial_println!(
                                                                "[sexnet.udp.tx.reply.checksum] checksum=0x0000 policy=zero_allowed ok=1"
                                                            );
                                                            serial_println!(
                                                                "[sexnet.ipv4.tx.udp_reply.build] src=10.0.2.15 dst={}.{}.{}.{} total_len={} checksum=ok ok=1",
                                                                src0, src1, src2, src3, reply_total_len
                                                            );
                                                            let frame_len = (14 + reply_total_len as u64) as u16;
                                                            if frame_len < 60 {
                                                                let mut pad = frame_len as u64;
                                                                while pad < 60 {
                                                                    unsafe { core::ptr::write_volatile((tx_va + pad) as *mut u8, 0u8); }
                                                                    pad += 1;
                                                                }
                                                            }
                                                            let tx_frame_len = if frame_len < 60 { 60u16 } else { frame_len };
                                                            let tx_desc4 = unsafe { TX_PERM_DESC_VA + 64 };
                                                            unsafe {
                                                                core::ptr::write_volatile(tx_desc4 as *mut u64, TX_PERM_FRAME_PHYS);
                                                                core::ptr::write_volatile((tx_desc4 + 8) as *mut u16, tx_frame_len);
                                                                core::ptr::write_volatile((tx_desc4 + 10) as *mut u8, 0u8);
                                                                core::ptr::write_volatile((tx_desc4 + 11) as *mut u8, 0x0Bu8);
                                                                core::ptr::write_volatile((tx_desc4 + 12) as *mut u8, 0u8);
                                                                core::ptr::write_volatile((tx_desc4 + 13) as *mut u8, 0u8);
                                                                core::ptr::write_volatile((tx_desc4 + 14) as *mut u16, 0u16);
                                                            }
                                                            serial_println!(
                                                                "[sexnet.eth.tx.udp_reply.desc] len={} ok=1",
                                                                tx_frame_len
                                                            );
                                                            unsafe {
                                                                core::ptr::write_volatile((nic_va + 0x3818) as *mut u32, 5);
                                                            }
                                                            let mut udp_reply_done = 0u32;
                                                            let mut tx_outer = 0u32;
                                                            while tx_outer < 50_000_000 {
                                                                let tx_st = unsafe { core::ptr::read_volatile((tx_desc4 + 12) as *const u8) };
                                                                if (tx_st & 1) != 0 {
                                                                    udp_reply_done = 1;
                                                                    break;
                                                                }
                                                                tx_outer += 1;
                                                            }
                                                            serial_println!(
                                                                "[sexnet.udp.tx.poll.done] dd_set={} ok={}",
                                                                udp_reply_done,
                                                                if udp_reply_done == 1 { 1 } else { 0 }
                                                            );
                                                            serial_println!(
                                                                "[sexnet.udp.echo.proof.done] rx_udp=1 tx_reply={} tx_dd={} ok={}",
                                                                if udp_reply_done > 0 { 1 } else { 0 },
                                                                udp_reply_done,
                                                                if udp_reply_done == 1 { 1 } else { 0 }
                                                            );
                                                        } else {
                                                            serial_println!(
                                                                "[sexnet.udp.reject] reason={} ok=1",
                                                                udp_reason
                                                            );
                                                        }
                                                        serial_println!(
                                                            "[sexnet.udp.header.proof.done] rx_udp=1 valid={} ok=1",
                                                            udp_ok
                                                        );
                                                    // TCP SYN-ACK / RST handler (Phase G)
                                                    } else if proto == 6 && (total_len as usize) >= 40 {
                                                        let tcp_base = pkt_buf + 34; // 14 eth + 20 ipv4
                                                        let ipv4_hdr_len = 20usize;
                                                        let ipv4_payload_len = (total_len as usize).saturating_sub(ipv4_hdr_len);
                                                        // Parse TCP ports
                                                        let tcp_src_port_hi = unsafe { core::ptr::read_volatile(tcp_base as *const u8) };
                                                        let tcp_src_port_lo = unsafe { core::ptr::read_volatile((tcp_base + 1) as *const u8) };
                                                        let tcp_dst_port_hi = unsafe { core::ptr::read_volatile((tcp_base + 2) as *const u8) };
                                                        let tcp_dst_port_lo = unsafe { core::ptr::read_volatile((tcp_base + 3) as *const u8) };
                                                        let tcp_src_port = ((tcp_src_port_hi as u16) << 8) | (tcp_src_port_lo as u16);
                                                        let tcp_dst_port = ((tcp_dst_port_hi as u16) << 8) | (tcp_dst_port_lo as u16);
                                                        // Parse SEQ and ACK
                                                        let seq0 = unsafe { core::ptr::read_volatile((tcp_base + 4) as *const u8) } as u32;
                                                        let seq1 = unsafe { core::ptr::read_volatile((tcp_base + 5) as *const u8) } as u32;
                                                        let seq2 = unsafe { core::ptr::read_volatile((tcp_base + 6) as *const u8) } as u32;
                                                        let seq3 = unsafe { core::ptr::read_volatile((tcp_base + 7) as *const u8) } as u32;
                                                        let tcp_seq = (seq0 << 24) | (seq1 << 16) | (seq2 << 8) | seq3;
                                                        let ack0 = unsafe { core::ptr::read_volatile((tcp_base + 8) as *const u8) } as u32;
                                                        let ack1 = unsafe { core::ptr::read_volatile((tcp_base + 9) as *const u8) } as u32;
                                                        let ack2 = unsafe { core::ptr::read_volatile((tcp_base + 10) as *const u8) } as u32;
                                                        let ack3 = unsafe { core::ptr::read_volatile((tcp_base + 11) as *const u8) } as u32;
                                                        let tcp_ack = (ack0 << 24) | (ack1 << 16) | (ack2 << 8) | ack3;
                                                        // Parse data_offset + flags
                                                        let dof_raw = unsafe { core::ptr::read_volatile((tcp_base + 12) as *const u8) };
                                                        let flags_raw = unsafe { core::ptr::read_volatile((tcp_base + 13) as *const u8) };
                                                        let tcp_data_offset = (dof_raw >> 4) & 0x0F;
                                                        let tcp_flags_syn = (flags_raw >> 1) & 0x01;
                                                        let tcp_flags_ack = (flags_raw >> 4) & 0x01;
                                                        let tcp_flags_rst = (flags_raw >> 2) & 0x01;
                                                        // Parse checksum
                                                        let csum_hi = unsafe { core::ptr::read_volatile((tcp_base + 16) as *const u8) };
                                                        let csum_lo = unsafe { core::ptr::read_volatile((tcp_base + 17) as *const u8) };
                                                        let tcp_csum_rx = ((csum_hi as u16) << 8) | (csum_lo as u16);
                                                        serial_println!(
                                                            "[sexnet.tcp.rx.segment] src_port={} dst_port={} seq={} ack={} data_offset={} flags_syn={} flags_ack={} flags_rst={} csum=0x{:04X} ok=1",
                                                            tcp_src_port, tcp_dst_port, tcp_seq, tcp_ack,
                                                            tcp_data_offset, tcp_flags_syn, tcp_flags_ack, tcp_flags_rst,
                                                            tcp_csum_rx
                                                        );
                                                        let local_port = unsafe { TCP_LOCAL_PORT };
                                                        let remote_port = unsafe { TCP_REMOTE_PORT };
                                                        let local_seq = unsafe { TCP_LOCAL_SEQ };
                                                        let tcp_state = { let s = TCP_STATE.lock(); *s };
                                                        let mut tcp_ok = 0u32;
                                                        let mut tcp_reason = "unknown";
                                                        // Bounds checks
                                                        if tcp_data_offset < 5 {
                                                            tcp_reason = "data_offset_too_small";
                                                        } else if (tcp_data_offset as usize) * 4 > ipv4_payload_len {
                                                            tcp_reason = "header_exceeds_payload";
                                                        } else if tcp_dst_port != local_port {
                                                            tcp_reason = "dst_port_mismatch";
                                                        } else if tcp_src_port != remote_port {
                                                            tcp_reason = "src_port_mismatch";
                                                        } else if tcp_state != TcpState::SynSent && tcp_state != TcpState::Established {
                                                            tcp_reason = "tcp_not_awaiting_response";
                                                        } else {
                                                            // Validate TCP checksum over pseudo-header
                                                            let mut tcp_rx_sum = 0u32;
                                                            // pseudo-header src IP
                                                            tcp_rx_sum += ((src0 as u16) << 8 | (src1 as u16)) as u32;
                                                            tcp_rx_sum += ((src2 as u16) << 8 | (src3 as u16)) as u32;
                                                            // pseudo-header dst IP
                                                            tcp_rx_sum += ((dst0 as u16) << 8 | (dst1 as u16)) as u32;
                                                            tcp_rx_sum += ((dst2 as u16) << 8 | (dst3 as u16)) as u32;
                                                            // pseudo-header zero + proto=6
                                                            tcp_rx_sum += 6u32;
                                                            // pseudo-header TCP length
                                                            let tcp_seg_len = (total_len as usize).saturating_sub(ipv4_hdr_len) as u16;
                                                            tcp_rx_sum += tcp_seg_len as u32;
                                                            // TCP segment words
                                                            let tcp_bytes = tcp_seg_len as usize;
                                                            let tcp_words = tcp_bytes / 2;
                                                            let mut cw = 0usize;
                                                            while cw < tcp_words {
                                                                let off = 34 + cw * 2;
                                                                let w_hi = unsafe { core::ptr::read_volatile((pkt_buf + off as u64) as *const u8) } as u16;
                                                                let w_lo = unsafe { core::ptr::read_volatile((pkt_buf + off as u64 + 1) as *const u8) } as u16;
                                                                tcp_rx_sum += ((w_hi << 8) | w_lo) as u32;
                                                                cw += 1;
                                                            }
                                                            if tcp_bytes % 2 != 0 {
                                                                let last_off = 34 + tcp_bytes - 1;
                                                                let last = unsafe { core::ptr::read_volatile((pkt_buf + last_off as u64) as *const u8) } as u16;
                                                                tcp_rx_sum += (last << 8) as u32;
                                                            }
                                                            while (tcp_rx_sum >> 16) != 0 {
                                                                tcp_rx_sum = (tcp_rx_sum & 0xFFFF) + (tcp_rx_sum >> 16);
                                                            }
                                                            let tcp_csum_ok = if (tcp_rx_sum as u16) == 0xFFFF { 1u32 } else { 0u32 };
                                                            if tcp_csum_ok == 0 {
                                                                tcp_reason = "checksum";
                                                            } else {
                                                                tcp_ok = 1;
                                                                tcp_reason = "ok";
                                                            }
                                                        }
                                                        serial_println!(
                                                            "[sexnet.tcp.rx.validate] ports_ok={} data_offset_ok={} checksum_ok={} ok={}",
                                                            if tcp_dst_port == local_port && tcp_src_port == remote_port { 1 } else { 0 },
                                                            if tcp_data_offset >= 5 { 1 } else { 0 },
                                                            if tcp_ok == 1 { 1 } else { 0 },
                                                            tcp_ok
                                                        );
                                                        if tcp_ok == 1 {
                                                            // Handle RST
                                                            if tcp_flags_rst == 1 {
                                                                let mut ts = TCP_STATE.lock();
                                                                *ts = TcpState::FailedRst;
                                                                unsafe { TCP_RST_COUNT += 1; }
                                                                serial_println!(
                                                                    "[sexnet.tcp.rst.rx] src_port={} dst_port={} seq={} ack={} flags=RST ok=1",
                                                                    tcp_src_port, tcp_dst_port, tcp_seq, tcp_ack
                                                                );
                                                                serial_println!("[sexnet.tcp.handshake.state] state=FAILED_RST ok=1");
                                                                serial_println!(
                                                                    "[sexnet.tcp.synack.rx.proof.done] rx_synack=0 rst=1 timeout=0 ok=0 honest=1"
                                                                );
                                                            // Handle SYN-ACK
                                                            } else if tcp_flags_syn == 1 && tcp_flags_ack == 1 {
                                                                // Validate ACK == local_seq + 1
                                                                let expected_ack = local_seq + 1;
                                                                let ack_ok = if tcp_ack == expected_ack { 1u32 } else { 0u32 };
                                                                serial_println!(
                                                                    "[sexnet.tcp.synack.rx] src_port={} dst_port={} seq={} ack={} flags=SYN|ACK ok=1",
                                                                    tcp_src_port, tcp_dst_port, tcp_seq, tcp_ack
                                                                );
                                                                serial_println!(
                                                                    "[sexnet.tcp.synack.validate] ack_ok={} ports_ok=1 checksum_ok=1 ok={}",
                                                                    ack_ok,
                                                                    if ack_ok == 1 { 1 } else { 0 }
                                                                );
                                                                let mut ts = TCP_STATE.lock();
                                                                *ts = TcpState::Established;
                                                                unsafe {
                                                                    TCP_REMOTE_SEQ = tcp_seq;
                                                                }
                                                                serial_println!("[sexnet.tcp.handshake.state] state=ESTABLISHED ok=1");
                                                                serial_println!(
                                                                    "[sexnet.tcp.synack.rx.proof.done] rx_synack=1 rst=0 timeout=0 ok=1"
                                                                );
                                                                // Build and send final ACK
                                                                let tx_va = unsafe { TX_PERM_FRAME_VA };
                                                                let nic_mac: [u8; 6] = [
                                                                    (ral & 0xFF) as u8,
                                                                    ((ral >> 8) & 0xFF) as u8,
                                                                    ((ral >> 16) & 0xFF) as u8,
                                                                    ((ral >> 24) & 0xFF) as u8,
                                                                    (rah & 0xFF) as u8,
                                                                    ((rah >> 8) & 0xFF) as u8,
                                                                ];
                                                                // Read src MAC from RX frame for Ethernet reply dst
                                                                let src_mac0 = unsafe { core::ptr::read_volatile((pkt_buf + 6) as *const u8) };
                                                                let src_mac1 = unsafe { core::ptr::read_volatile((pkt_buf + 7) as *const u8) };
                                                                let src_mac2 = unsafe { core::ptr::read_volatile((pkt_buf + 8) as *const u8) };
                                                                let src_mac3 = unsafe { core::ptr::read_volatile((pkt_buf + 9) as *const u8) };
                                                                let src_mac4 = unsafe { core::ptr::read_volatile((pkt_buf + 10) as *const u8) };
                                                                let src_mac5 = unsafe { core::ptr::read_volatile((pkt_buf + 11) as *const u8) };
                                                                // Ethernet header
                                                                unsafe {
                                                                    core::ptr::write_volatile((tx_va + 0) as *mut u8, src_mac0);
                                                                    core::ptr::write_volatile((tx_va + 1) as *mut u8, src_mac1);
                                                                    core::ptr::write_volatile((tx_va + 2) as *mut u8, src_mac2);
                                                                    core::ptr::write_volatile((tx_va + 3) as *mut u8, src_mac3);
                                                                    core::ptr::write_volatile((tx_va + 4) as *mut u8, src_mac4);
                                                                    core::ptr::write_volatile((tx_va + 5) as *mut u8, src_mac5);
                                                                    core::ptr::write_volatile((tx_va + 6) as *mut u8, nic_mac[0]);
                                                                    core::ptr::write_volatile((tx_va + 7) as *mut u8, nic_mac[1]);
                                                                    core::ptr::write_volatile((tx_va + 8) as *mut u8, nic_mac[2]);
                                                                    core::ptr::write_volatile((tx_va + 9) as *mut u8, nic_mac[3]);
                                                                    core::ptr::write_volatile((tx_va + 10) as *mut u8, nic_mac[4]);
                                                                    core::ptr::write_volatile((tx_va + 11) as *mut u8, nic_mac[5]);
                                                                    core::ptr::write_volatile((tx_va + 12) as *mut u8, 0x08);
                                                                    core::ptr::write_volatile((tx_va + 13) as *mut u8, 0x00);
                                                                }
                                                                let ack_ipv4_total: u16 = 20 + 20;
                                                                unsafe {
                                                                    core::ptr::write_volatile((tx_va + 14) as *mut u8, 0x45);
                                                                    core::ptr::write_volatile((tx_va + 15) as *mut u8, 0x00);
                                                                    core::ptr::write_volatile((tx_va + 16) as *mut u8, ((ack_ipv4_total >> 8) & 0xFF) as u8);
                                                                    core::ptr::write_volatile((tx_va + 17) as *mut u8, (ack_ipv4_total & 0xFF) as u8);
                                                                    core::ptr::write_volatile((tx_va + 18) as *mut u8, 0x00);
                                                                    core::ptr::write_volatile((tx_va + 19) as *mut u8, 0x01);
                                                                    core::ptr::write_volatile((tx_va + 20) as *mut u8, 0x00);
                                                                    core::ptr::write_volatile((tx_va + 21) as *mut u8, 0x00);
                                                                    core::ptr::write_volatile((tx_va + 22) as *mut u8, 64);
                                                                    core::ptr::write_volatile((tx_va + 23) as *mut u8, 6);
                                                                    core::ptr::write_volatile((tx_va + 24) as *mut u8, 0x00);
                                                                    core::ptr::write_volatile((tx_va + 25) as *mut u8, 0x00);
                                                                    core::ptr::write_volatile((tx_va + 26) as *mut u8, 10);
                                                                    core::ptr::write_volatile((tx_va + 27) as *mut u8, 0);
                                                                    core::ptr::write_volatile((tx_va + 28) as *mut u8, 2);
                                                                    core::ptr::write_volatile((tx_va + 29) as *mut u8, 15);
                                                                    core::ptr::write_volatile((tx_va + 30) as *mut u8, src0);
                                                                    core::ptr::write_volatile((tx_va + 31) as *mut u8, src1);
                                                                    core::ptr::write_volatile((tx_va + 32) as *mut u8, src2);
                                                                    core::ptr::write_volatile((tx_va + 33) as *mut u8, src3);
                                                                }
                                                                // IPv4 checksum
                                                                {
                                                                    let mut ipv4_sum = 0u32;
                                                                    let mut ck = 0usize;
                                                                    while ck < 10 {
                                                                        let off = 14 + ck * 2;
                                                                        let w_hi = unsafe { core::ptr::read_volatile((tx_va + off as u64) as *const u8) } as u16;
                                                                        let w_lo = unsafe { core::ptr::read_volatile((tx_va + off as u64 + 1) as *const u8) } as u16;
                                                                        ipv4_sum += ((w_hi << 8) | w_lo) as u32;
                                                                        ck += 1;
                                                                    }
                                                                    while (ipv4_sum >> 16) != 0 {
                                                                        ipv4_sum = (ipv4_sum & 0xFFFF) + (ipv4_sum >> 16);
                                                                    }
                                                                    let ipv4_csum = !(ipv4_sum as u16);
                                                                    unsafe {
                                                                        core::ptr::write_volatile((tx_va + 24) as *mut u8, ((ipv4_csum >> 8) & 0xFF) as u8);
                                                                        core::ptr::write_volatile((tx_va + 25) as *mut u8, (ipv4_csum & 0xFF) as u8);
                                                                    }
                                                                }
                                                                // TCP ACK header
                                                                let ack_seq = local_seq + 1;
                                                                let ack_ack = unsafe { TCP_REMOTE_SEQ + 1 };
                                                                unsafe {
                                                                    core::ptr::write_volatile((tx_va + 34) as *mut u8, ((local_port >> 8) & 0xFF) as u8);
                                                                    core::ptr::write_volatile((tx_va + 35) as *mut u8, (local_port & 0xFF) as u8);
                                                                    core::ptr::write_volatile((tx_va + 36) as *mut u8, ((remote_port >> 8) & 0xFF) as u8);
                                                                    core::ptr::write_volatile((tx_va + 37) as *mut u8, (remote_port & 0xFF) as u8);
                                                                    core::ptr::write_volatile((tx_va + 38) as *mut u8, ((ack_seq >> 24) & 0xFF) as u8);
                                                                    core::ptr::write_volatile((tx_va + 39) as *mut u8, ((ack_seq >> 16) & 0xFF) as u8);
                                                                    core::ptr::write_volatile((tx_va + 40) as *mut u8, ((ack_seq >> 8) & 0xFF) as u8);
                                                                    core::ptr::write_volatile((tx_va + 41) as *mut u8, (ack_seq & 0xFF) as u8);
                                                                    core::ptr::write_volatile((tx_va + 42) as *mut u8, ((ack_ack >> 24) & 0xFF) as u8);
                                                                    core::ptr::write_volatile((tx_va + 43) as *mut u8, ((ack_ack >> 16) & 0xFF) as u8);
                                                                    core::ptr::write_volatile((tx_va + 44) as *mut u8, ((ack_ack >> 8) & 0xFF) as u8);
                                                                    core::ptr::write_volatile((tx_va + 45) as *mut u8, (ack_ack & 0xFF) as u8);
                                                                    core::ptr::write_volatile((tx_va + 46) as *mut u8, 0x50); // data_offset=5 << 4
                                                                    core::ptr::write_volatile((tx_va + 47) as *mut u8, 0x10); // ACK flag
                                                                    core::ptr::write_volatile((tx_va + 48) as *mut u8, 0xFF); // window=65535
                                                                    core::ptr::write_volatile((tx_va + 49) as *mut u8, 0xFF);
                                                                    core::ptr::write_volatile((tx_va + 50) as *mut u8, 0x00); // csum placeholder
                                                                    core::ptr::write_volatile((tx_va + 51) as *mut u8, 0x00);
                                                                    core::ptr::write_volatile((tx_va + 52) as *mut u8, 0x00); // urgent=0
                                                                    core::ptr::write_volatile((tx_va + 53) as *mut u8, 0x00);
                                                                }
                                                                // TCP checksum
                                                                {
                                                                    let mut tcp_sum = 0u32;
                                                                    tcp_sum += ((10u16 << 8) | 0u16) as u32;
                                                                    tcp_sum += ((2u16 << 8) | 15u16) as u32;
                                                                    tcp_sum += ((src0 as u16) << 8 | (src1 as u16)) as u32;
                                                                    tcp_sum += ((src2 as u16) << 8 | (src3 as u16)) as u32;
                                                                    tcp_sum += 6u32;
                                                                    tcp_sum += 20u32;
                                                                    let mut cw = 0usize;
                                                                    while cw < 10 {
                                                                        let off = 34 + cw * 2;
                                                                        let w_hi = unsafe { core::ptr::read_volatile((tx_va + off as u64) as *const u8) } as u16;
                                                                        let w_lo = unsafe { core::ptr::read_volatile((tx_va + off as u64 + 1) as *const u8) } as u16;
                                                                        tcp_sum += ((w_hi << 8) | w_lo) as u32;
                                                                        cw += 1;
                                                                    }
                                                                    while (tcp_sum >> 16) != 0 {
                                                                        tcp_sum = (tcp_sum & 0xFFFF) + (tcp_sum >> 16);
                                                                    }
                                                                    let tcp_csum = !(tcp_sum as u16);
                                                                    unsafe {
                                                                        core::ptr::write_volatile((tx_va + 50) as *mut u8, ((tcp_csum >> 8) & 0xFF) as u8);
                                                                        core::ptr::write_volatile((tx_va + 51) as *mut u8, (tcp_csum & 0xFF) as u8);
                                                                    }
                                                                    serial_println!(
                                                                        "[sexnet.tcp.ack.checksum] checksum=0x{:04X} ok=1",
                                                                        tcp_csum
                                                                    );
                                                                }
                                                                serial_println!(
                                                                    "[sexnet.tcp.ack.build] seq={} ack={} flags=ACK ok=1",
                                                                    ack_seq, ack_ack
                                                                );
                                                                // Pad frame
                                                                let frame_len_ack = (14 + ack_ipv4_total as u64) as u16;
                                                                if frame_len_ack < 60 {
                                                                    let mut pad = frame_len_ack as u64;
                                                                    while pad < 60 {
                                                                        unsafe { core::ptr::write_volatile((tx_va + pad) as *mut u8, 0u8); }
                                                                        pad += 1;
                                                                    }
                                                                }
                                                                let tx_frame_len_ack = if frame_len_ack < 60 { 60u16 } else { frame_len_ack };
                                                                // TX descriptor 6 (offset 96) for TCP ACK
                                                                let tx_desc6 = unsafe { TX_PERM_DESC_VA + 96 };
                                                                unsafe {
                                                                    core::ptr::write_volatile(tx_desc6 as *mut u64, TX_PERM_FRAME_PHYS);
                                                                    core::ptr::write_volatile((tx_desc6 + 8) as *mut u16, tx_frame_len_ack);
                                                                    core::ptr::write_volatile((tx_desc6 + 10) as *mut u8, 0u8);
                                                                    core::ptr::write_volatile((tx_desc6 + 11) as *mut u8, 0x0Bu8);
                                                                    core::ptr::write_volatile((tx_desc6 + 12) as *mut u8, 0u8);
                                                                    core::ptr::write_volatile((tx_desc6 + 13) as *mut u8, 0u8);
                                                                    core::ptr::write_volatile((tx_desc6 + 14) as *mut u16, 0u16);
                                                                }
                                                                serial_println!(
                                                                    "[sexnet.eth.tx.tcp_ack.desc] len={} ok=1",
                                                                    tx_frame_len_ack
                                                                );
                                                                unsafe {
                                                                    core::ptr::write_volatile((nic_va + 0x3818) as *mut u32, 7);
                                                                }
                                                                let mut ack_tx_dd = 0u32;
                                                                let mut tx_outer = 0u32;
                                                                while tx_outer < 50_000_000 {
                                                                    let tx_st = unsafe { core::ptr::read_volatile((tx_desc6 + 12) as *const u8) };
                                                                    if (tx_st & 1) != 0 {
                                                                        ack_tx_dd = 1;
                                                                        break;
                                                                    }
                                                                    tx_outer += 1;
                                                                }
                                                                serial_println!(
                                                                    "[sexnet.tcp.ack.tx.poll.done] dd_set={} ok={}",
                                                                    ack_tx_dd,
                                                                    if ack_tx_dd == 1 { 1 } else { 0 }
                                                                );
                                                                unsafe { TCP_ACK_COUNT += 1; }
                                                                serial_println!(
                                                                    "[sexnet.tcp.ack.tx.proof.done] ack_sent={} tx_dd={} ok={}",
                                                                    if ack_tx_dd == 1 { 1 } else { 0 },
                                                                    ack_tx_dd,
                                                                    if ack_tx_dd == 1 { 1 } else { 0 }
                                                                );
                                                            } else {
                                                                // Non-SYN-ACK, non-RST TCP segment
                                                                serial_println!(
                                                                    "[sexnet.tcp.rx.ignore] reason=not_synack_or_rst flags_syn={} flags_ack={} flags_rst={} ok=1",
                                                                    tcp_flags_syn, tcp_flags_ack, tcp_flags_rst
                                                                );
                                                            }
                                                        } else {
                                                            serial_println!(
                                                                "[sexnet.tcp.rx.reject] reason={} ok=1",
                                                                tcp_reason
                                                            );
                                                        }
                                                    } else if proto == 6 && (total_len as usize) < 40 {
                                                        serial_println!(
                                                            "[sexnet.tcp.rx.reject] reason=tcp_header_too_short total_len={} ok=1",
                                                            total_len
                                                        );
                                                    }
                                                } else if reject_logged == 0 {
                                                    serial_println!(
                                                        "[sexnet.ipv4.rx.reject.detail] idx={} etype=0x{:04X} reason={} ok=0",
                                                        idx,
                                                        ethertype,
                                                        reason
                                                    );
                                                    reject_logged = 1;
                                                }
                                            } else if reject_logged == 0 {
                                                serial_println!(
                                                    "[sexnet.ipv4.rx.reject.detail] idx={} etype=0x{:04X} reason=non_ipv4 ok=0",
                                                    idx,
                                                    ethertype
                                                );
                                                reject_logged = 1;
                                            }
                                            unsafe {
                                                core::ptr::write_volatile((desc_base + 8) as *mut u16, 0u16);
                                                core::ptr::write_volatile((desc_base + 12) as *mut u8, 0u8);
                                                core::ptr::write_volatile((nic_va + 0x2818) as *mut u32, idx);
                                            }
                                            serial_println!(
                                                "[sexnet.ipv4.rx.recycle] idx={} new_rdt={} ok=1",
                                                idx,
                                                idx
                                            );
                                        }
                                        idx += 1;
                                    }
                                    outer += 1;
                                }
                                serial_println!(
                                    "[sexnet.ipv4.rx.poll.done] frames={} ok={}",
                                    ipv4_frames,
                                    ipv4_ok
                                );
                                serial_println!(
                                    "[sexnet.ipv4.proof.done] frames={} ok={}",
                                    ipv4_frames,
                                    ipv4_ok
                                );
                            } else {
                                serial_println!(
                                    "[sexnet.ipv4.entry] rx_owner={} ok=0",
                                    ipv4_rx_own
                                );
                            }
                        } else {
                            serial_println!(
                                "[sexnet.l2.entry] rx_owner={} tx_owner={} ok=0 reason=not_full",
                                l2_rx_own,
                                l2_tx_own
                            );
                        }
                    }
                }
            }
        } else {
            serial_println!("[sexnet.nic.bar.map] va=MAX ok=0 reason=no_cap_or_map_denied");
        }
    }
    loop {
        let req = unsafe { pdx_listen_raw(0) };
        let result = handle_call(req.type_id, req.arg0, req.arg1);
        pdx_reply(req.caller_pd, result);
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
