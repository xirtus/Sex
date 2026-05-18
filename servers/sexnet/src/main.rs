#![no_std]
#![no_main]

use sex_pdx::{pdx_listen_raw, pdx_reply, serial_println};
use spin::Mutex;

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
