#![no_std]
#![no_main]

use silkbar_model::{
    SilkBarUpdate, UpdateKind, ChipKind, ChipSlot, OP_SILKBAR_UPDATE, validate_silkbar_contract,
    SILKBAR_ABI_VERSION, SILKBAR_WORKSPACE_COUNT, SILKBAR_CHIP_COUNT,
    SILKBAR_DEFAULT_ACTIVE_WORKSPACE_IDX, SILKBAR_WORKSPACE_IDX_MAX, DEFAULT_SILK_BAR,
};
use sex_pdx::{OP_BELL_LIST, OP_BELL_SUBSCRIBE, SLOT_BELL};

const BELL_DELIVERY_PROOF_ENABLED: bool = option_env!("SEXOS_BELL_DELIVERY_PROOF").is_some();

fn send_update_status(update: SilkBarUpdate) -> u64 {
    let result = sex_pdx::pdx_call_checked(
        sex_pdx::SLOT_DISPLAY,
        OP_SILKBAR_UPDATE,
        update.kind as u64,
        (update.index as u64) << 32 | update.a as u64,
        update.b as u64,
    );
    if let Err(err) = result {
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
            // Clock-specific drop: 64 entries so starvation survives startup noise.
            if update.kind == UpdateKind::SetClock as u32 {
                static mut CLOCK_DROP_COUNTER: u64 = 0;
                static mut CLOCK_DROP_LOG_BUDGET: u32 = 64;
                let cn = CLOCK_DROP_COUNTER;
                CLOCK_DROP_COUNTER = CLOCK_DROP_COUNTER.wrapping_add(1);
                let cb = &mut CLOCK_DROP_LOG_BUDGET;
                if *cb > 0 {
                    *cb -= 1;
                    sex_pdx::serial_println!("[silkbar.send_update.drop.clock] count={} err={:#x}",
                        cn, err);
                }
            }
        }
        return err;
    }
    0
}

fn send_update(update: SilkBarUpdate) {
    let _ = send_update_status(update);
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    sex_pdx::serial_println!("[silkbar.boot.begin]");
    sex_pdx::serial_println!("[bootgraph.silkbar.spawned]");
    let contract_err = validate_silkbar_contract();
    let degraded = contract_err != 0;
    if degraded {
        sex_pdx::serial_println!("[silkbar.boot.contract.fail] code={}", contract_err);
        sex_pdx::serial_println!("[bootgraph.silkbar.degraded] reason=contract_fail");
    } else {
        sex_pdx::serial_println!("[silkbar.boot.contract.ok]");
        sex_pdx::serial_println!("[bootgraph.silkbar.contract_ready]");
    }
    sex_pdx::serial_println!("[silk.contract.validate.version] version={}", SILKBAR_ABI_VERSION);

    let mut focus_state: u8 = 0;
    let mut last_focus_state: u8 = 0xFF;
    let mut last_options_mask: u32 = 0;
    let mut chip_phase: u8 = 0;
    let mut chip0_net: bool = true;
    // Clock state: software counter (no get_ticks() — frozen under QEMU TCG).
    // Each 100-yield cadence advances by 1 second.
    let mut hh: u8 = DEFAULT_SILK_BAR.clock_hh;
    let mut mm: u8 = DEFAULT_SILK_BAR.clock_mm;
    let mut ss: u8 = DEFAULT_SILK_BAR.clock_ss;
    // Loop iteration counter — drives clock, bell, and chip cadence.
    let mut loop_iter: u64 = 0;
    // Cached Bell generation counter. 0 forces first LIST poll.
    let mut bell_gen_cached: u64 = 0;
    // True when OP_BELL_LIST is enqueued and reply not yet received.
    let mut bell_pending_list: bool = false;

    // INIT updates are deferred to avoid startup blocking before loop liveness.
    // Clock init is intentionally omitted (no init SetClock ss=0).
    let mut init_deferred: [SilkBarUpdate; SILKBAR_WORKSPACE_COUNT + SILKBAR_CHIP_COUNT] =
        [SilkBarUpdate::new(0, 0, 0, 0); SILKBAR_WORKSPACE_COUNT + SILKBAR_CHIP_COUNT];
    let mut init_deferred_count: usize = 0;
    if !degraded {
        for ws_idx in 0..SILKBAR_WORKSPACE_COUNT as u8 {
            init_deferred[init_deferred_count] = SilkBarUpdate::new(
                UpdateKind::SetWorkspaceActive as u32,
                ws_idx,
                if ws_idx == SILKBAR_DEFAULT_ACTIVE_WORKSPACE_IDX { 1 } else { 0 },
                0,
            );
            init_deferred_count += 1;
        }
        for chip_idx in 0..SILKBAR_CHIP_COUNT as u8 {
            init_deferred[init_deferred_count] =
                SilkBarUpdate::new(UpdateKind::SetChipVisible as u32, chip_idx, 1, 0);
            init_deferred_count += 1;
        }
    }
    let mut init_deferred_next: usize = 0;
    sex_pdx::serial_println!("[silkbar.boot.init.defer] count={}", init_deferred_count);

    loop {
        if loop_iter == 0 {
            sex_pdx::serial_println!("[bootgraph.silkbar.loop_ready]");
        }
        // Process at most one upstream message per loop (non-blocking).
        if let Some(msg) = sex_pdx::pdx_try_listen_raw(0) {
            if msg.type_id == sex_pdx::OP_SILKBAR_WORKSPACE_ACTIVE {
                let ws_raw = msg.arg0 as u8;
                if ws_raw > SILKBAR_WORKSPACE_IDX_MAX {
                    sex_pdx::serial_println!(
                        "[silkbar.update.reject] kind={} idx={} reason=out_of_bounds",
                        msg.type_id,
                        ws_raw
                    );
                    sex_pdx::sys_yield();
                    continue;
                }
                let ws = ws_raw;
                sex_pdx::serial_println!("[silkbar.workspace.recv] index={}", ws);
                sex_pdx::serial_println!("[silkbar.workspace.active.set] index={}", ws);
                sex_pdx::serial_println!("[silkbar.workspace.active.send.start] index={}", ws);
                if !degraded {
                    for i in 0..SILKBAR_WORKSPACE_COUNT as u8 {
                        send_update(SilkBarUpdate::new(
                            UpdateKind::SetWorkspaceActive as u32, i, if i == ws { 1 } else { 0 }, 0,
                        ));
                    }
                }
                sex_pdx::serial_println!("[silkbar.workspace.active.send.ok] index={}", ws);
            } else if msg.type_id == sex_pdx::OP_SILKBAR_FOCUS_STATE {
                let raw = msg.arg0 as u8;
                if raw > 3 {
                    sex_pdx::serial_println!(
                        "[silkbar.update.reject] kind={} idx={} reason=out_of_bounds",
                        msg.type_id,
                        raw
                    );
                    sex_pdx::sys_yield();
                    continue;
                }
                focus_state = raw;
                // Extract selected-window options mask from arg1 (V1 extension).
                // Old senders pass 0 (no options) — backward compatible.
                let options_mask = msg.arg1 as u32;
                sex_pdx::serial_println!("[silkbar.selected.options.recv] mask={:#x}", options_mask);
                if !degraded && options_mask != last_options_mask {
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
                    if !degraded {
                        send_update(SilkBarUpdate::new(
                            UpdateKind::SetBellPresence as u32, 0, a, 0,
                        ));
                    }
                    if BELL_DELIVERY_PROOF_ENABLED {
                        sex_pdx::serial_println!("[bell.poll.ok] total={} redacted={} flags={:#x}", total, redacted, flags);
                        sex_pdx::serial_println!("[silkbar.bell.state] total={} redacted={} flags={:#x}", total, redacted, flags);
                    }
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
                        } else if !degraded {
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
            if !degraded {
                for ws in 0..SILKBAR_WORKSPACE_COUNT as u8 {
                    let urgent = if Some(ws) == urgent_ws { 1 } else { 0 };
                    send_update(SilkBarUpdate::new(
                        UpdateKind::SetWorkspaceUrgent as u32, ws, urgent, 0,
                    ));
                }
            }
            last_focus_state = focus_state;
        }

        if !degraded && init_deferred_next < init_deferred_count {
            let idx = init_deferred_next;
            let update = init_deferred[idx];
            let remaining = init_deferred_count - idx - 1;
            send_update(update);
            init_deferred_next += 1;
            sex_pdx::serial_println!("[silkbar.boot.init.flush] idx={} remaining={}", idx, remaining);
            if init_deferred_next == init_deferred_count {
                sex_pdx::serial_println!("[silkbar.boot.init.flush.done]");
            }
        }

        // ~1s via yield (no rdtsc — freezes under QEMU TCG)
        for _ in 0..100 {
            sex_pdx::sys_yield();
        }

        // Advance software clock by one logical second per cadence.
        // get_ticks() is diagnostic-only — under QEMU TCG it returns 0.
        loop_iter = loop_iter.wrapping_add(1);
        // [silkbar.loop.alive] budgeted marker
        {
            static mut LOOP_ALIVE_BUDGET: u32 = 16;
            let b = unsafe { &mut LOOP_ALIVE_BUDGET };
            if *b > 0 {
                *b -= 1;
                sex_pdx::serial_println!("[silkbar.loop.alive] iter={}", loop_iter);
            }
        }
        let raw_ticks = sex_pdx::get_ticks();
        // Software clock: increment by 1 second
        ss = ss.wrapping_add(1);
        if ss >= 60 {
            ss = 0;
            mm = mm.wrapping_add(1);
            if mm >= 60 {
                mm = 0;
                hh = hh.wrapping_add(1);
                if hh >= 24 {
                    hh = 0;
                }
            }
        }
        // [silkbar.clock.tick] budgeted diagnostic
        {
            static mut CLOCK_TICK_BUDGET: u32 = 12;
            let c = unsafe { &mut CLOCK_TICK_BUDGET };
            if *c > 0 {
                *c -= 1;
                sex_pdx::serial_println!("[silkbar.clock.tick] hh={} mm={} ss={} raw_ticks={}", hh, mm, ss, raw_ticks);
            }
        }

        // ── Bell presence poll (every ~2 seconds) ──────────────────────────
        // V2: Poll OP_BELL_SUBSCRIBE for generation counter.
        // If generation changed since last poll, call OP_BELL_LIST.
        // If SUBSCRIBE fails (no cap or Bell down), fall back to LIST.
        // Existing LIST reply handler (type_id=1, caller_pd=1) covers both paths.
        if !degraded && loop_iter % 2 == 0 && !bell_pending_list {
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
        if !degraded && loop_iter % 120 == 0 {
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

        let clock_status = if degraded {
            u64::MAX
        } else {
            send_update_status(SilkBarUpdate::new(
                UpdateKind::SetClock as u32, 0, hh as u32, ((mm as u32) << 8) | ss as u32,
            ))
        };
        if loop_iter == 1 {
            if degraded {
                sex_pdx::serial_println!("[bootgraph.silkbar.degraded] reason=clock_send_disabled");
            } else {
                sex_pdx::serial_println!("[bootgraph.silkbar.clock_ready]");
            }
        }
        // Budgeted diagnostic: first 12 clock sends
        {
            static mut CLOCK_SEND_BUDGET: u32 = 12;
            let remaining = unsafe { &mut CLOCK_SEND_BUDGET };
            if *remaining > 0 {
                *remaining -= 1;
                sex_pdx::serial_println!(
                    "[silkbar.clock.send] hh={} mm={} ss={} status={:#x}",
                    hh, mm, ss, clock_status
                );
            }
        }
    }
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop { core::hint::spin_loop(); }
}
