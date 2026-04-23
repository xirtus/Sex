#![no_std]
#![no_main]
#![feature(alloc_error_handler)]

extern crate alloc;
use linked_list_allocator::LockedHeap;

#[global_allocator]
static ALLOCATOR: LockedHeap = LockedHeap::empty();

#[alloc_error_handler]
fn alloc_error_handler(_layout: core::alloc::Layout) -> ! {
    loop {}
}

use sex_pdx::{
    pdx_listen_raw, pdx_reply,
    SEXNET_GET_STATUS, SEXNET_SCAN_WIFI, SEXNET_CONNECT,
    SEXNET_DISCONNECT, SEXNET_VPN_UP, SEXNET_VPN_DOWN, SEXNET_GET_IP,
    SexnetApEntry,
};
use spin::Mutex;

// --------------------------------------------------------------------------
// State
// --------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq)]
enum WifiState { Disconnected, Connected }

#[derive(Clone, Copy, PartialEq)]
enum VpnState { Down, Up }

struct NetState {
    wifi: WifiState,
    vpn: VpnState,
    link_speed_mbps: u16,
    ipv4: u32,
}

static STATE: Mutex<NetState> = Mutex::new(NetState {
    wifi: WifiState::Disconnected,
    vpn: VpnState::Down,
    link_speed_mbps: 0,
    ipv4: 0,
});

// Mock AP scan table — replaced by NIC ring in a real driver.
static MOCK_APS: [SexnetApEntry; 3] = [
    SexnetApEntry { ssid: *b"SexOS_Network\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0", rssi: -45, channel: 6,  flags: 0b0010 },
    SexnetApEntry { ssid: *b"Silk_Hotspot\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0", rssi: -68, channel: 11, flags: 0b0010 },
    SexnetApEntry { ssid: *b"OpenAP\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0\0", rssi: -80, channel: 1,  flags: 0b0001 },
];

// --------------------------------------------------------------------------
// PDX dispatch
// --------------------------------------------------------------------------

fn handle_call(syscall_id: u64, arg0: u64, arg1: u64) -> u64 {
    match syscall_id {
        SEXNET_GET_STATUS => {
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
                return 2; // no carrier
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
    loop {
        let req = unsafe { pdx_listen_raw(0) };
        let result = handle_call(req.opcode(), req.msg_type.arg0(), req.msg_type.arg1());
        pdx_reply(req.caller_pd, result);
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    loop {}
}
