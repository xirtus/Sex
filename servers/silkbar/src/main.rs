#![no_std]
#![no_main]

use silkbar_model::{
    SilkBarUpdate, UpdateKind, ChipKind, ChipSlot, OP_SILKBAR_UPDATE, validate_silkbar_contract,
    SILKBAR_ABI_VERSION, SILKBAR_WORKSPACE_COUNT, SILKBAR_CHIP_COUNT,
    SILKBAR_DEFAULT_ACTIVE_WORKSPACE_IDX, SILKBAR_WORKSPACE_IDX_MAX,
};
use sex_pdx::{OP_BELL_LIST, OP_BELL_SUBSCRIBE, SLOT_BELL};

fn send_update(update: SilkBarUpdate) {
    let result = sex_pdx::pdx_call_checked(
        sex_pdx::SLOT_DISPLAY,
        OP_SILKBAR_UPDATE,
        update.kind as u64,
        (update.index as u64) << 32 | update.a as u64,
        update.b as u64,
    );
    if let Err(err) = result {
        // Budgeted error diagnostic: log first 16 drops.
        unsafe {
            static mut DROP_COUNTER: u64 = 0;
            static mut DROP_LOG_BUDGET: u32 = 16;
            let n = DROP_COUNTER;
            DROP_COUNTER = DROP_COUNTER.wrapping_add(1);
            let remaining = &mut DROP_LOG_BUDGET;
            if *remaining > 0 {
                *remaining -= 1;
                sex_pdx::serial_println!("[silkbar.send_update.drop] kind={} idx={} err={:#x} count={}",
                    update.kind, update.index, err, n);
            }
        }
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    sex_pdx::serial_println!("[silk.contract.validate.start]");
    let contract_err = validate_silkbar_contract();
    if contract_err != 0 {
        sex_pdx::serial_println!("[silk.contract.validate.fail] reason={}", contract_err);
        loop { core::hint::spin_loop(); }
    }
    sex_pdx::serial_println!("[silk.contract.validate.ok] version={}", SILKBAR_ABI_VERSION);

    let mut focus_state: u8 = 0;
    let mut last_focus_state: u8 = 0xFF;
    let mut last_options_mask: u32 = 0;
    let mut chip_phase: u8 = 0;
    let mut chip0_net: bool = true;
    // Initialize to 0 so the first loop iteration skips the redundant
    // SetClock(ss=0) and waits until uptime_seconds advances to 1.
    // Sexdisplay's fallback clock handles the first second.
    let mut last_uptime_seconds: u64 = 0;
    /// Cached Bell generation counter. 0 forces first LIST poll.
    let mut bell_gen_cached: u64 = 0;
    /// True when OP_BELL_LIST is enqueued and reply not yet received.
    let mut bell_pending_list: bool = false;

    /// Approximate LAPIC timer ticks per second (divide=16, init_count=1_000_000).
    /// Not calibrated — yields monotonic uptime, not wall-clock accuracy.
    const LAPIC_TICKS_PER_SECOND_APPROX: u64 = 62;

    // INIT: full GLOBAL_BAR state — workspace activation, chip visibility.
    // Clock is deliberately omitted: sexdisplay fallback handles the first
    // second, and the main loop sends SetClock starting at ss=1.
    for ws_idx in 0..SILKBAR_WORKSPACE_COUNT as u8 {
        send_update(SilkBarUpdate::new(
            UpdateKind::SetWorkspaceActive as u32, ws_idx, if ws_idx == SILKBAR_DEFAULT_ACTIVE_WORKSPACE_IDX { 1 } else { 0 }, 0,
        ));
    }
    // All four status chips visible
    for chip_idx in 0..SILKBAR_CHIP_COUNT as u8 {
        send_update(SilkBarUpdate::new(
            UpdateKind::SetChipVisible as u32, chip_idx, 1, 0,
        ));
    }

    loop {
        // Process at most one upstream message per loop (non-blocking).
        if let Some(msg) = sex_pdx::pdx_try_listen_raw(0) {
            if msg.type_id == sex_pdx::OP_SILKBAR_WORKSPACE_ACTIVE {
                let ws = (msg.arg0 as u8).min(SILKBAR_WORKSPACE_IDX_MAX);
                sex_pdx::serial_println!("[silkbar.workspace.recv] index={}", ws);
                sex_pdx::serial_println!("[silkbar.workspace.active.set] index={}", ws);
                sex_pdx::serial_println!("[silkbar.workspace.active.send.start] index={}", ws);
                for i in 0..SILKBAR_WORKSPACE_COUNT as u8 {
                    send_update(SilkBarUpdate::new(
                        UpdateKind::SetWorkspaceActive as u32, i, if i == ws { 1 } else { 0 }, 0,
                    ));
                }
                sex_pdx::serial_println!("[silkbar.workspace.active.send.ok] index={}", ws);
            } else if msg.type_id == sex_pdx::OP_SILKBAR_FOCUS_STATE {
                // Clamp invalid producer values to debug(3) to keep update space bounded.
                focus_state = (msg.arg0 as u8).min(3);
                // Extract selected-window options mask from arg1 (V1 extension).
                // Old senders pass 0 (no options) — backward compatible.
                let options_mask = msg.arg1 as u32;
                sex_pdx::serial_println!("[silkbar.selected.options.recv] mask={:#x}", options_mask);
                if options_mask != last_options_mask {
                    last_options_mask = options_mask;
                    send_update(SilkBarUpdate::new(
                        UpdateKind::SetSelectedOptions as u32, 0, options_mask, 0,
                    ));
                    sex_pdx::serial_println!("[silkbar.selected.options.forward] mask={:#x}", options_mask);
                }
            } else if msg.type_id == 1 && msg.caller_pd == 1 {
                // ── Bell reply (SUBSCRIBE generation or LIST packed counts) ──
                if bell_pending_list {
                    // LIST reply: arg0 = packed lane counts from Bell.
                    // Repack for sexdisplay SetBellPresence format:
                    //   bits 7:0   = total_visible
                    //   bits 15:8  = redacted_count (from Bell's bits 63:56)
                    //   bits 23:16 = flags (bit 0 = bell_available since LIST succeeded)
                    bell_pending_list = false;
                    let packed = msg.arg0;
                    let total = (packed & 0xFF) as u8;
                    let redacted = ((packed >> 56) & 0xFF) as u8;
                    let flags: u8 = 1; // bit 0 = bell_available
                    let a = (total as u32) | ((redacted as u32) << 8) | ((flags as u32) << 16);
                    unsafe {
                        static mut BELL_REPLY_BUDGET: u32 = 8;
                        let b = &mut BELL_REPLY_BUDGET;
                        if *b > 0 {
                            *b -= 1;
                            sex_pdx::serial_println!("[silkbar.bell.poll.reply] total={} redacted={} flags={:#x}",
                                total, redacted, flags);
                        }
                    }
                    send_update(SilkBarUpdate::new(
                        UpdateKind::SetBellPresence as u32, 0, a, 0,
                    ));
                } else {
                    // SUBSCRIBE reply: arg0 = current generation.
                    let gen = msg.arg0;
                    let changed = if gen != bell_gen_cached && gen != u64::MAX { 1 } else { 0 };
                    if changed != 0 {
                        unsafe {
                            static mut BELL_GEN_REPLY_CHANGED_BUDGET: u32 = 8;
                            let b = &mut BELL_GEN_REPLY_CHANGED_BUDGET;
                            if *b > 0 {
                                *b -= 1;
                                sex_pdx::serial_println!("[silkbar.bell.gen.reply] gen={} changed={}", gen, changed);
                            }
                        }
                    } else {
                        unsafe {
                            static mut BELL_GEN_REPLY_STEADY_BUDGET: u32 = 1;
                            let b = &mut BELL_GEN_REPLY_STEADY_BUDGET;
                            if *b > 0 {
                                *b -= 1;
                                sex_pdx::serial_println!("[silkbar.bell.gen.reply] gen={} changed={}", gen, changed);
                            }
                        }
                    }
                    if gen == u64::MAX {
                        // Denied — fall back to LIST.
                        unsafe {
                            static mut BELL_GEN_FALLBACK_BUDGET: u32 = 8;
                            let b = &mut BELL_GEN_FALLBACK_BUDGET;
                            if *b > 0 {
                                *b -= 1;
                                sex_pdx::serial_println!("[silkbar.bell.gen.fallback] reason=denied");
                            }
                        }
                        let list_args = 0xFFu64 | (1u64 << 8);
                        if let Ok(_) = sex_pdx::pdx_call_checked(SLOT_BELL, OP_BELL_LIST, list_args, 0, 0) {
                            bell_pending_list = true;
                        } else {
                            send_update(SilkBarUpdate::new(
                                UpdateKind::SetBellPresence as u32, 0, 0, 0,
                            ));
                        }
                    } else if gen != bell_gen_cached {
                        bell_gen_cached = gen;
                        // Generation changed — call LIST.
                        let list_args = 0xFFu64 | (1u64 << 8);
                        if let Ok(_) = sex_pdx::pdx_call_checked(SLOT_BELL, OP_BELL_LIST, list_args, 0, 0) {
                            bell_pending_list = true;
                        }
                    }
                    // If gen == bell_gen_cached: no change, skip update.
                }
            } else {
                sex_pdx::serial_println!("[pdx.opcode.unknown] silkbar type_id={:#x} caller={}", msg.type_id, msg.caller_pd);
            }
        }

        if focus_state != last_focus_state {
            // Focus state drives workspace urgent highlight.
            // none: clear all; shell/app/debug => ws0/ws1/ws2 urgent respectively.
            let urgent_ws = match focus_state {
                1 => Some(0u8),
                2 => Some(1u8),
                3 => Some(2u8),
                _ => None,
            };
            for ws in 0..SILKBAR_WORKSPACE_COUNT as u8 {
                let urgent = if Some(ws) == urgent_ws { 1 } else { 0 };
                send_update(SilkBarUpdate::new(
                    UpdateKind::SetWorkspaceUrgent as u32, ws, urgent, 0,
                ));
            }
            last_focus_state = focus_state;
        }

        // ~1s via yield (no rdtsc — freezes under QEMU TCG)
        for _ in 0..100 {
            sex_pdx::sys_yield();
        }

        // Read kernel uptime ticks for clock and chip cadence
        let ticks = sex_pdx::get_ticks();
        let uptime_seconds = ticks / LAPIC_TICKS_PER_SECOND_APPROX;
        if uptime_seconds == last_uptime_seconds {
            continue;
        }
        last_uptime_seconds = uptime_seconds;
        let hh = ((uptime_seconds / 3600) % 24) as u8;
        let mm = ((uptime_seconds / 60) % 60) as u8;
        let ss = (uptime_seconds % 60) as u8;

        // ── Bell presence poll (every ~2 seconds) ──────────────────────────
        // V2: Poll OP_BELL_SUBSCRIBE for generation counter.
        // If generation changed since last poll, call OP_BELL_LIST.
        // If SUBSCRIBE fails (no cap or Bell down), fall back to LIST.
        // Existing LIST reply handler (type_id=1, caller_pd=1) covers both paths.
        if uptime_seconds % 2 == 0 && !bell_pending_list {
            let result = sex_pdx::pdx_call_checked(SLOT_BELL, OP_BELL_SUBSCRIBE, 0, 0, 0);
            if let Err(e) = result {
                // SUBSCRIBE failed — fall back to LIST.
                unsafe {
                    static mut BELL_GEN_FALLBACK_BUDGET: u32 = 8;
                    let b = &mut BELL_GEN_FALLBACK_BUDGET;
                    if *b > 0 {
                        *b -= 1;
                        sex_pdx::serial_println!("[silkbar.bell.gen.fallback] reason=cap_err err={:#x}", e);
                    }
                }
                let list_args = 0xFFu64 | (1u64 << 8);
                if let Ok(_) = sex_pdx::pdx_call_checked(SLOT_BELL, OP_BELL_LIST, list_args, 0, 0) {
                    bell_pending_list = true;
                } else {
                    send_update(SilkBarUpdate::new(
                        UpdateKind::SetBellPresence as u32, 0, 0, 0,
                    ));
                }
            }
            // If Ok: SUBSCRIBE enqueued. Reply with generation arrives
            // asynchronously via pdx_try_listen_raw(0). Handled above
            // in the type_id=1, caller_pd=1 branch (else: SUBSCRIBE).
        }

        // Stage 2C: bounded internal status-chip stub (no new ABI, no floods).
        // Slow cadence: every 120 seconds.
        if uptime_seconds % 120 == 0 {
            match chip_phase {
                0 => {
                    let chip0_kind = if chip0_net { ChipKind::Net } else { ChipKind::Wifi };
                    send_update(SilkBarUpdate::new(
                        UpdateKind::SetChipKind as u32, ChipSlot::Chip0 as u8, chip0_kind as u32, 0,
                    ));
                    send_update(SilkBarUpdate::new(
                        UpdateKind::SetChipKind as u32, ChipSlot::Chip1 as u8, ChipKind::Wifi as u32, 0,
                    ));
                    chip0_net = !chip0_net;
                }
                1 => {
                    send_update(SilkBarUpdate::new(
                        UpdateKind::SetChipKind as u32, ChipSlot::Chip2 as u8, ChipKind::Battery as u32, 0,
                    ));
                }
                2 => {
                    send_update(SilkBarUpdate::new(
                        UpdateKind::SetChipKind as u32, ChipSlot::Clock as u8, ChipKind::Net as u32, 0,
                    ));
                }
                _ => {
                    send_update(SilkBarUpdate::new(
                        UpdateKind::SetChipKind as u32, ChipSlot::Clock as u8, ChipKind::Battery as u32, 0,
                    ));
                }
            }
            chip_phase = (chip_phase + 1) & 0x3;
        }

        send_update(SilkBarUpdate::new(
            UpdateKind::SetClock as u32, 0, hh as u32, ((mm as u32) << 8) | ss as u32,
        ));
        // Budgeted diagnostic: first 12 clock sends
        {
            static mut CLOCK_SEND_BUDGET: u32 = 12;
            let remaining = unsafe { &mut CLOCK_SEND_BUDGET };
            if *remaining > 0 {
                *remaining -= 1;
                sex_pdx::serial_println!("[silkbar.clock.send] hh={} mm={} ss={}", hh, mm, ss);
            }
        }
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop { core::hint::spin_loop(); }
}
