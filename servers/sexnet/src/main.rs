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

unsafe fn sys_net_diag() -> u64 {
    let result: u64;
    core::arch::asm!(
        "syscall",
        inout("rax") 52u64 => result,
        lateout("rcx") _,
        lateout("r11") _,
        options(nostack)
    );
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
                serial_println!("[sexnet.body_text.len] len={}", BODY_TEXT.len());
                return BODY_TEXT.len() as u64;
            }
            if arg0 == SEXNET_HTTP_BODY_CHUNK {
                let idx = arg1 as usize;
                let start = idx.saturating_mul(8);
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
    let raw = unsafe { sys_net_diag() };
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
    serial_println!("[sexnet.boot] ok=1 reason=passive_spawn");
    serial_println!("[sexnet.passive.ready] network=0 dns=0 tcp=0 http=0 tls=0 ok=1 reason=mock_status_only_no_nic");
    serial_println!("[sexnet.passive.spawn.done] ok=1 spawned=1 browser_network=0");
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
