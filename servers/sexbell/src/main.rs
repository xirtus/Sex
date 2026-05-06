#![no_std]
#![no_main]

use sex_pdx::{pdx_listen_raw, serial_println, OP_BELL_NOTIFY, OP_BELL_CLOSE, OP_BELL_ACTION,
              OP_BELL_CLEAR, OP_BELL_MUTE_SENDER};

/// Reply to caller via kernel syscall 29 (SYSCALL_PDX_REPLY).
/// sex-pdx's pdx_reply() uses syscall 1 — unhandled in current kernel. Use 29 directly.
/// Kernel: rdi=target_pd, rsi=value → pushed to target's incoming_replies buffer.
/// Caller reads reply via pdx_listen_raw(0) → msg.type_id=1, msg.arg0=value.
#[inline(always)]
unsafe fn bell_reply(target_pd: u32, val: u64) {
    core::arch::asm!(
        "syscall",
        in("rax") 29u64,
        in("rdi") target_pd as u64,
        in("rsi") val,
        out("rcx") _,
        out("r11") _,
        options(nostack),
    );
}

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

// ── RAM Queue ─────────────────────────────────────────────────────────
const BELL_QUEUE_CAPACITY: usize = 16;

#[repr(C)]
#[derive(Copy, Clone)]
struct BellQueueEntry {
    /// Monotonic event ID assigned by Bell on accept (0 = invalid sentinel).
    event_id:         u64,
    /// Kernel-authoritative sender PD.
    caller_pd:        u32,
    /// Event category (0=Info .. 5=Error).
    category:         u8,
    /// Urgency hint from sender (0..3).
    requested_lane:   u8,
    /// Final lane after policy derivation (0=PASSIVE .. 5=SECURITY).
    final_lane:       u8,
    /// Final urgency after policy derivation.
    final_urgency:    u8,
    /// Privacy level (0=Public .. 3=FullHidden).
    privacy_level:    u8,
    /// Redaction class (0=StructuralMeta .. 3=SecretContent).
    redaction_class:  u8,
    /// Number of action callbacks attached (V1: 0 or 1).
    action_count:     u8,
    /// First action ID (valid when action_count >= 1). Marker-only in V1.
    action_id:        u8,
    /// Number of object references attached (V1: 0 or 1).
    object_ref_count: u8,
    /// First object reference ID (valid when object_ref_count >= 1). Marker-only in V1.
    object_ref:       u8,
    /// 0 = active, 1 = dismissed by CLOSE or CLEAR. Skipped in LIST.
    dismissed:        u8,
    /// Padding.
    _pad:             [u8; 2],
}

struct BellQueue {
    /// Monotonic event ID counter. 0 reserved as invalid sentinel.
    next_event_id: u64,
    /// Head index (oldest entry).
    head: u16,
    /// Tail index (next write position).
    tail: u16,
    /// Current entry count.
    count: u16,
    /// Fixed-size entry array.
    entries: [BellQueueEntry; BELL_QUEUE_CAPACITY],
}

impl BellQueue {
    const fn new() -> Self {
        BellQueue {
            next_event_id: 1, // start at 1 (0 reserved)
            head: 0,
            tail: 0,
            count: 0,
            entries: [BellQueueEntry {
                event_id: 0,
                caller_pd: 0,
                category: 0,
                requested_lane: 0,
                final_lane: 0,
                final_urgency: 0,
                privacy_level: 0,
                redaction_class: 0,
                action_count: 0,
                action_id: 0,
                object_ref_count: 0,
                object_ref: 0,
                dismissed: 0,
                _pad: [0; 2],
            }; BELL_QUEUE_CAPACITY],
        }
    }

    fn is_full(&self) -> bool {
        self.count >= BELL_QUEUE_CAPACITY as u16
    }

    /// Find the lowest-priority active entry's index in the ring buffer.
    /// Returns None if no active entries exist.
    /// Low priority = lowest final_lane value. Ties broken by oldest (smallest distance from head).
    fn find_lowest_priority_index(&self) -> Option<u16> {
        let mut lowest_idx: Option<u16> = None;
        let mut lowest_lane: u8 = 6; // higher than any valid lane
        let mut lowest_dist: u16 = 0; // distance from head

        for i in 0..self.count as usize {
            let idx = (self.head as usize + i) % BELL_QUEUE_CAPACITY;
            let entry = &self.entries[idx];
            if entry.dismissed != 0 {
                continue;
            }
            if entry.final_lane < lowest_lane
                || (entry.final_lane == lowest_lane && lowest_idx.map_or(true, |_| i as u16 > lowest_dist))
            {
                lowest_lane = entry.final_lane;
                lowest_dist = i as u16;
                lowest_idx = Some(idx as u16);
            }
        }

        lowest_idx
    }

    /// Push a new entry, assigning event_id.
    /// If queue is full, drops the lowest-priority active entry to make room.
    /// Returns Ok with (event_id, Option<dropped_lane>) or Err("queue_full").
    fn push(&mut self, caller_pd: u32, category: u8, requested_lane: u8,
            final_lane: u8, final_urgency: u8, privacy_level: u8,
            redaction_class: u8, action_count: u8, action_id: u8,
            object_ref_count: u8, object_ref: u8) -> Result<(u64, Option<u8>), &'static str> {
        if self.is_full() {
            // Try to drop lowest-priority entry
            if let Some(drop_idx) = self.find_lowest_priority_index() {
                let dropped_lane = self.entries[drop_idx as usize].final_lane;
                self.entries[drop_idx as usize].event_id = 0;
                self.entries[drop_idx as usize].dismissed = 1;
                // Write new entry into the freed slot
                let event_id = self.next_event_id;
                self.next_event_id = if event_id == u64::MAX { 1 } else { event_id + 1 };
                self.entries[drop_idx as usize] = BellQueueEntry {
                    event_id,
                    caller_pd,
                    category,
                    requested_lane,
                    final_lane,
                    final_urgency,
                    privacy_level,
                    redaction_class,
                    action_count,
                    action_id,
                    object_ref_count,
                    object_ref,
                    dismissed: 0,
                    _pad: [0; 2],
                };
                return Ok((event_id, Some(dropped_lane)));
            }
            return Err("queue_full");
        }

        let event_id = self.next_event_id;
        // Wrap safely; 0 is reserved as invalid sentinel.
        self.next_event_id = if event_id == u64::MAX { 1 } else { event_id + 1 };

        let entry = BellQueueEntry {
            event_id,
            caller_pd,
            category,
            requested_lane,
            final_lane,
            final_urgency,
            privacy_level,
            redaction_class,
            action_count,
            action_id,
            object_ref_count,
            object_ref,
            dismissed: 0,
            _pad: [0; 2],
        };

        self.entries[self.tail as usize] = entry;
        self.tail = (self.tail + 1) % BELL_QUEUE_CAPACITY as u16;
        self.count += 1;

        Ok((event_id, None)) // None = no drop occurred
    }
}

/// Static queue instance. Safe for single-threaded use (sexbell has one thread).
static mut BELL_QUEUE: BellQueue = BellQueue::new();

// ── Read-Cap Allowlist ──────────────────────────────────────────────

/// Static allowlist of PDs permitted to call OP_BELL_LIST.
/// Default-deny: any PD not in this list is rejected.
/// V1: only silk-shell (domain 3). Extended in future phases.
const BELL_LIST_ALLOWLIST: &[u32] = &[
    3,  // silk-shell (domain 3, policy owner)
    6,  // silkbar (domain 6, privacy-safe aggregate poller)
];

/// Check if a caller PD is authorized to call OP_BELL_LIST.
fn is_list_reader_allowed(caller_pd: u32) -> bool {
    BELL_LIST_ALLOWLIST.contains(&caller_pd)
}

// ── Mute List ────────────────────────────────────────────────────────

/// Static mute list of sender PDs. Muted senders' NOTIFY is rejected.
const MUTE_LIST_CAPACITY: usize = 16;
static mut MUTE_LIST: [u32; MUTE_LIST_CAPACITY] = [0; MUTE_LIST_CAPACITY];
static mut MUTE_COUNT: usize = 0;

/// Check if a caller PD is muted.
fn is_muted(caller_pd: u32) -> bool {
    unsafe {
        for i in 0..MUTE_COUNT {
            if MUTE_LIST[i] == caller_pd {
                return true;
            }
        }
    }
    false
}

/// Add a PD to the mute list. Returns Ok(()) if added or already present.
fn add_mute(caller_pd: u32) -> Result<(), &'static str> {
    unsafe {
        if is_muted(caller_pd) {
            return Ok(()); // idempotent
        }
        if MUTE_COUNT >= MUTE_LIST_CAPACITY {
            return Err("mute_list_full");
        }
        MUTE_LIST[MUTE_COUNT] = caller_pd;
        MUTE_COUNT += 1;
    }
    Ok(())
}

/// Remove a PD from the mute list. Returns true if found and removed, false if not present.
fn remove_mute(caller_pd: u32) -> bool {
    unsafe {
        for i in 0..MUTE_COUNT {
            if MUTE_LIST[i] == caller_pd {
                // Shift remaining entries left
                for j in i..MUTE_COUNT - 1 {
                    MUTE_LIST[j] = MUTE_LIST[j + 1];
                }
                MUTE_COUNT -= 1;
                return true;
            }
        }
    }
    false
}

// ── Spam Budget ──────────────────────────────────────────────────────

/// Per-PD spam budget: max events per tick window.
const SPAM_WINDOW_TICKS: u64 = 62;      // ~1 second at typical tick rate
const SPAM_MAX_PER_WINDOW: u32 = 8;     // max events per window per PD
const SPAM_BUDGET_SLOTS: usize = 16;    // tracked sender slots

struct SpamBudget {
    slots: [(u32, u32, u64); SPAM_BUDGET_SLOTS], // (caller_pd, count, window_start_tick)
}

static mut SPAM_BUDGET: SpamBudget = SpamBudget {
    slots: [(0, 0, 0); SPAM_BUDGET_SLOTS],
};

/// Check and record a notify event for a caller PD.
/// Returns true if allowed, false if rate-limited (spam_budget_exceeded).
fn check_spam_budget(caller_pd: u32) -> bool {
    let now = sex_pdx::get_ticks();
    unsafe {
        // Find existing slot for this PD
        for i in 0..SPAM_BUDGET_SLOTS {
            let (pd, count, window_start) = &mut SPAM_BUDGET.slots[i];
            if *pd == caller_pd {
                // Check if window has expired
                if now.saturating_sub(*window_start) >= SPAM_WINDOW_TICKS {
                    // New window: reset
                    *window_start = now;
                    *count = 1;
                    return true;
                }
                // Within window: check limit
                if *count >= SPAM_MAX_PER_WINDOW {
                    return false; // rate-limited
                }
                *count += 1;
                return true;
            }
        }
        // No existing slot: find an empty one or reuse oldest
        let mut oldest_idx = 0;
        let mut oldest_tick = u64::MAX;
        for i in 0..SPAM_BUDGET_SLOTS {
            if SPAM_BUDGET.slots[i].0 == 0 {
                // Empty slot: use it
                SPAM_BUDGET.slots[i] = (caller_pd, 1, now);
                return true;
            }
            if SPAM_BUDGET.slots[i].2 < oldest_tick {
                oldest_tick = SPAM_BUDGET.slots[i].2;
                oldest_idx = i;
            }
        }
        // All slots full: evict oldest
        SPAM_BUDGET.slots[oldest_idx] = (caller_pd, 1, now);
        true
    }
}

// ── Enum Validation ──────────────────────────────────────────────────

/// Look up the maximum privacy level a caller PD may view.
/// Default-deny: unknown PDs get 0 (Public only).
/// V1: only silk-shell (domain 3) may view all levels.
fn max_privacy_for_caller(caller_pd: u32) -> u8 {
    match caller_pd {
        3 => 3, // silk-shell: may view FullHidden
        _ => 0, // default: Public only
    }
}

/// Validate a BellCategory enum value (0=Info .. 5=Error).
fn valid_category(v: u8) -> bool {
    v <= 5
}

/// Validate a BellPrivacyLevel enum value (0=Public .. 3=FullHidden).
fn valid_privacy_level(v: u8) -> bool {
    v <= 3
}

/// Validate a BellRedactionClass enum value (0=StructuralMeta .. 3=SecretContent).
fn valid_redaction_class(v: u8) -> bool {
    v <= 3
}

/// First-proof placeholder lane derivation.
///
/// No BellCap table exists yet. Every sender is unknown/untrusted.
/// Unknown/untrusted max lane = PASSIVE (0).
/// Urgency_hint > 0 downgrades to PASSIVE.
fn derive_lane_first_proof(urgency_hint: u8) -> (u8, u8, Option<&'static str>) {
    if urgency_hint == 0 {
        (0, 0, None)
    } else {
        (0, 0, Some("no_caps_untrusted"))
    }
}

#[no_mangle]
pub extern "C" fn _start() -> ! {
    serial_println!("[bell.boot]");

    loop {
        let msg = pdx_listen_raw(0);

        match msg.type_id {
            OP_BELL_NOTIFY => {
                // ── Parse fixed numeric fields from IpcCall args ──────
                let category        = ((msg.arg0 >> 0)  & 0xFF) as u8;
                let urgency_hint    = ((msg.arg0 >> 8)  & 0xFF) as u8;
                let privacy_level   = ((msg.arg0 >> 16) & 0xFF) as u8;
                let redaction_class = ((msg.arg0 >> 24) & 0xFF) as u8;
                let action_count    = (msg.arg1 & 0xFF) as u8;
                let action_id       = ((msg.arg1 >> 8) & 0xFF) as u8;
                let object_ref_count = (msg.arg2 & 0xFF) as u8;
                let object_ref      = ((msg.arg2 >> 8) & 0xFF) as u8;
                let caller_pd       = msg.caller_pd;

                // ── Mute check (reject before any processing) ───────────
                if is_muted(caller_pd) {
                    unsafe {
                        static mut BELL_MUTED_REJECT_BUDGET: u32 = 8;
                        let b = &mut BELL_MUTED_REJECT_BUDGET;
                        if *b > 0 {
                            *b -= 1;
                            serial_println!("[bell.notify.reject] caller_pd={} reason=muted",
                                caller_pd);
                        }
                    }
                    continue;
                }

                // ── Validate enum ranges ─────────────────────────────
                let mut reject_reason: Option<&'static str> = None;

                if !valid_category(category) {
                    reject_reason = Some("invalid_category");
                } else if !valid_privacy_level(privacy_level) {
                    reject_reason = Some("invalid_privacy");
                } else if !valid_redaction_class(redaction_class) {
                    reject_reason = Some("invalid_redaction");
                } else if urgency_hint > 3 {
                    reject_reason = Some("invalid_urgency");
                } else if action_count > 1 {
                    reject_reason = Some("action_count_invalid");
                } else if action_count == 1 && action_id == 0 {
                    reject_reason = Some("action_id_zero");
                } else if object_ref_count > 1 {
                    reject_reason = Some("object_refs_invalid");
                }

                if let Some(reason) = reject_reason {
                    unsafe {
                        static mut BELL_REJECT_BUDGET: u32 = 4;
                        let b = &mut BELL_REJECT_BUDGET;
                        if *b > 0 {
                            *b -= 1;
                            serial_println!("[bell.notify.reject] caller_pd={} reason={}",
                                caller_pd, reason);
                        }
                    }
                    continue;
                }

                // ── Emit recv marker ─────────────────────────────────
                unsafe {
                    static mut BELL_RECV_BUDGET: u32 = 8;
                    let b = &mut BELL_RECV_BUDGET;
                    if *b > 0 {
                        *b -= 1;
                        serial_println!("[bell.notify.recv] caller_pd={} category={} requested={}",
                            caller_pd, category, urgency_hint);
                    }
                }

                // ── Derive lane (placeholder policy: no caps → PASSIVE) ──
                let (final_lane, final_urgency, downgrade) =
                    derive_lane_first_proof(urgency_hint);

                if let Some(reason) = downgrade {
                    unsafe {
                        static mut BELL_DOWNGRADE_BUDGET: u32 = 8;
                        let b = &mut BELL_DOWNGRADE_BUDGET;
                        if *b > 0 {
                            *b -= 1;
                            serial_println!("[bell.notify.downgrade] from={} to={} reason={}",
                                urgency_hint, final_lane, reason);
                        }
                    }
                }

                // ── Spam budget check ───────────────────────────────────
                if !check_spam_budget(caller_pd) {
                    unsafe {
                        static mut BELL_SPAM_REJECT_BUDGET: u32 = 8;
                        let b = &mut BELL_SPAM_REJECT_BUDGET;
                        if *b > 0 {
                            *b -= 1;
                            serial_println!("[bell.notify.reject] caller_pd={} reason=spam_budget_exceeded window=64 max=8",
                                caller_pd);
                        }
                    }
                    continue;
                }

                // ── Push to queue ────────────────────────────────────
                let push_result = unsafe {
                    BELL_QUEUE.push(caller_pd, category, urgency_hint,
                                    final_lane, final_urgency, privacy_level,
                                    redaction_class, action_count, action_id,
                                    object_ref_count, object_ref)
                };

                match push_result {
                    Ok((event_id, drop_info)) => {
                        // ── Emit drop marker if a lowest-priority entry was displaced ──
                        if let Some(dropped_lane) = drop_info {
                            unsafe {
                                static mut BELL_QUEUE_DROP_BUDGET: u32 = 16;
                                let b = &mut BELL_QUEUE_DROP_BUDGET;
                                if *b > 0 {
                                    *b -= 1;
                                    serial_println!("[bell.queue.drop] reason=lowest_priority lane={} dropped_lane={} event_id={}",
                                        final_lane, dropped_lane, event_id);
                                }
                            }
                        }

                        unsafe {
                            static mut BELL_QUEUE_PUSH_BUDGET: u32 = 64;
                            let b = &mut BELL_QUEUE_PUSH_BUDGET;
                            if *b > 0 {
                                *b -= 1;
                                serial_println!("[bell.queue.push] id={} final_lane={} count={}",
                                    event_id, final_lane, BELL_QUEUE.count);
                            }
                        }
                        // ── Emit ok marker ───────────────────────────
                        unsafe {
                            static mut BELL_OK_BUDGET: u32 = 8;
                            let b = &mut BELL_OK_BUDGET;
                            if *b > 0 {
                                *b -= 1;
                                serial_println!("[bell.notify.ok] caller_pd={} final_lane={} event_id={}",
                                    caller_pd, final_lane, event_id);
                            }
                        }
                    }
                    Err(reason) => {
                        unsafe {
                            static mut BELL_QUEUE_FULL_BUDGET: u32 = 16;
                            let b = &mut BELL_QUEUE_FULL_BUDGET;
                            if *b > 0 {
                                *b -= 1;
                                serial_println!("[bell.queue.reject.full] count={}",
                                    BELL_QUEUE_CAPACITY);
                            }
                        }
                        // Emit reject for queue-full
                        unsafe {
                            static mut BELL_REJECT_BUDGET: u32 = 4;
                            let b = &mut BELL_REJECT_BUDGET;
                            if *b > 0 {
                                *b -= 1;
                                serial_println!("[bell.notify.reject] caller_pd={} reason={}",
                                    caller_pd, reason);
                            }
                        }
                    }
                }
            }

            OP_BELL_LIST => {
                // ── Parse ──
                let lane_filter  = ((msg.arg0 >> 0)  & 0xFF) as u8;
                let max_results  = ((msg.arg0 >> 8)  & 0xFF) as u8;
                let caller_pd    = msg.caller_pd;

                // ── Validate lane_filter ──
                // 0xFF = all lanes, 0..=5 = specific lane
                if lane_filter != 0xFF && lane_filter > 5 {
                    unsafe {
                        static mut BELL_LIST_REJECT_BUDGET: u32 = 4;
                        let b = &mut BELL_LIST_REJECT_BUDGET;
                        if *b > 0 {
                            *b -= 1;
                            serial_println!("[bell.list.reject] reason=invalid_lane caller_pd={}",
                                caller_pd);
                        }
                    }
                    continue;
                }

                // ── Validate max_results (1..=4 only, reject out of range) ──
                if max_results == 0 || max_results > 4 {
                    unsafe {
                        static mut BELL_LIST_REJECT_BUDGET: u32 = 4;
                        let b = &mut BELL_LIST_REJECT_BUDGET;
                        if *b > 0 {
                            *b -= 1;
                            serial_println!("[bell.list.reject] reason=invalid_count caller_pd={}",
                                caller_pd);
                        }
                    }
                    continue;
                }

                // ── Check read-cap allowlist ──
                // Default-deny: only allowlisted PDs may call OP_BELL_LIST.
                // Check happens after arg validation (protocol errors are
                // reported regardless of caller) but before queue access.
                if !is_list_reader_allowed(caller_pd) {
                    unsafe {
                        static mut BELL_READCAP_DENY_BUDGET: u32 = 8;
                        let b = &mut BELL_READCAP_DENY_BUDGET;
                        if *b > 0 {
                            *b -= 1;
                            serial_println!("[bell.readcap.deny] caller_pd={} op=list reason=no_read_cap",
                                caller_pd);
                        }
                        // Reply with error so caller does not hang
                        bell_reply(caller_pd, u64::MAX);
                    }
                    continue;
                }

                // ── Emit recv marker ──

                let caller_max_privacy = max_privacy_for_caller(caller_pd);

                // ── Compute aggregate lane counts (full scan, not max_results-limited) ──
                let mut lane_counts: [u32; 6] = [0; 6];
                let mut total_visible: u32 = 0;
                unsafe {
                    for i in 0..BELL_QUEUE.count as usize {
                        let idx = (BELL_QUEUE.tail as usize + BELL_QUEUE_CAPACITY - 1 - i)
                                  % BELL_QUEUE_CAPACITY;
                        let entry = &BELL_QUEUE.entries[idx];
                        if entry.dismissed != 0 { continue; }
                        if entry.privacy_level > caller_max_privacy { continue; }
                        let lane = entry.final_lane as usize;
                        if lane < 6 {
                            lane_counts[lane] += 1;
                            total_visible += 1;
                        }
                    }
                }

                // ── Read queue (newest-first, no mutation) ──
                let mut match_count: u32 = 0;
                let mut redact_count: u32 = 0;

                unsafe {
                    for i in 0..BELL_QUEUE.count as usize {
                        let idx = (BELL_QUEUE.tail as usize + BELL_QUEUE_CAPACITY - 1 - i)
                                  % BELL_QUEUE_CAPACITY;
                        let entry = &BELL_QUEUE.entries[idx];

                        // Skip dismissed entries
                        if entry.dismissed != 0 {
                            continue;
                        }

                        // ── Privacy gate: skip entries above caller's max privacy level ──
                        if entry.privacy_level > caller_max_privacy {
                            if entry.privacy_level == 3 {
                                // FullHidden: count but don't reveal
                                redact_count += 1;
                            }
                            continue;
                        }

                        if lane_filter == 0xFF || entry.final_lane == lane_filter {
                            // ── Emit item marker ──
                            static mut BELL_LIST_ITEM_BUDGET: u32 = 8;
                            let b = &mut BELL_LIST_ITEM_BUDGET;
                            if *b > 0 {
                                *b -= 1;
                                serial_println!("[bell.list.item] event_id={} final_lane={} \
                                    category={} privacy={} redaction={} actions={} refs={}",
                                    entry.event_id, entry.final_lane, entry.category,
                                    entry.privacy_level, entry.redaction_class,
                                    entry.action_count, entry.object_ref_count);
                            }

                            match_count += 1;
                            if match_count >= max_results as u32 {
                                break;
                            }
                        }
                    }
                }

                // ── Emit redact marker if FullHidden entries were filtered ──
                if redact_count > 0 {
                    unsafe {
                        static mut BELL_LIST_REDACT_BUDGET: u32 = 8;
                        let b = &mut BELL_LIST_REDACT_BUDGET;
                        if *b > 0 {
                            *b -= 1;
                            serial_println!("[bell.list.redact] reason=full_hidden count={} caller_pd={}",
                                redact_count, caller_pd);
                        }
                    }
                }

                // ── Reply with packed aggregate counts ──
                // Packing: [63:56]=redacted [55:48]=lane5 [47:40]=lane4 [39:32]=lane3
                //          [31:24]=lane2 [23:16]=lane1 [15:8]=lane0 [7:0]=total_visible
                let packed = (total_visible as u64) & 0xFF
                           | ((lane_counts[0] as u64) & 0xFF) << 8
                           | ((lane_counts[1] as u64) & 0xFF) << 16
                           | ((lane_counts[2] as u64) & 0xFF) << 24
                           | ((lane_counts[3] as u64) & 0xFF) << 32
                           | ((lane_counts[4] as u64) & 0xFF) << 40
                           | ((lane_counts[5] as u64) & 0xFF) << 48
                           | ((redact_count as u64) & 0xFF) << 56;
                unsafe {
                    static mut BELL_LIST_REPLY_BUDGET: u32 = 8;
                    let b = &mut BELL_LIST_REPLY_BUDGET;
                    if *b > 0 {
                        *b -= 1;
                        serial_println!("[bell.list.reply] total={} lanes=[{} {} {} {} {} {}] redacted={}",
                            total_visible,
                            lane_counts[0], lane_counts[1], lane_counts[2],
                            lane_counts[3], lane_counts[4], lane_counts[5],
                            redact_count);
                    }
                    bell_reply(caller_pd, packed);
                }
            }

            OP_BELL_CLOSE => {
                // ── Parse ──
                let event_id   = msg.arg0;
                let caller_pd  = msg.caller_pd;

                // ── Search queue for matching event_id, mark dismissed ──
                let mut found = false;
                unsafe {
                    for i in 0..BELL_QUEUE.count as usize {
                        let idx = (BELL_QUEUE.head as usize + i) % BELL_QUEUE_CAPACITY;
                        if BELL_QUEUE.entries[idx].event_id == event_id
                            && BELL_QUEUE.entries[idx].dismissed == 0
                        {
                            BELL_QUEUE.entries[idx].dismissed = 1;
                            found = true;
                            break;
                        }
                    }
                }

                if found {
                    unsafe {
                        static mut BELL_CLOSE_OK_BUDGET: u32 = 8;
                        let b = &mut BELL_CLOSE_OK_BUDGET;
                        if *b > 0 {
                            *b -= 1;
                            serial_println!("[bell.close.ok] event_id={}", event_id);
                        }
                    }
                } else {
                    unsafe {
                        static mut BELL_CLOSE_REJECT_BUDGET: u32 = 4;
                        let b = &mut BELL_CLOSE_REJECT_BUDGET;
                        if *b > 0 {
                            *b -= 1;
                            serial_println!("[bell.close.reject] reason=not_found event_id={} caller_pd={}",
                                event_id, caller_pd);
                        }
                    }
                }
            }

            OP_BELL_ACTION => {
                // ── Parse ──
                let event_id   = msg.arg0;
                let action_id  = (msg.arg1 & 0xFF) as u8;
                let caller_pd  = msg.caller_pd;

                // ── Search queue for matching event_id with matching action_id ──
                let mut found = false;
                let mut action_final_lane: u8 = 0;
                unsafe {
                    for i in 0..BELL_QUEUE.count as usize {
                        let idx = (BELL_QUEUE.head as usize + i) % BELL_QUEUE_CAPACITY;
                        let entry = &BELL_QUEUE.entries[idx];
                        if entry.event_id == event_id
                            && entry.dismissed == 0
                            && entry.action_count > 0
                            && entry.action_id == action_id
                        {
                            action_final_lane = entry.final_lane;
                            found = true;
                            break;
                        }
                    }
                }

                if found {
                    unsafe {
                        static mut BELL_ACTION_DISPATCH_BUDGET: u32 = 8;
                        let b = &mut BELL_ACTION_DISPATCH_BUDGET;
                        if *b > 0 {
                            *b -= 1;
                            serial_println!("[bell.action.dispatch] event_id={} action_id={} lane={}",
                                event_id, action_id, action_final_lane);
                        }
                    }
                } else {
                    unsafe {
                        static mut BELL_ACTION_REJECT_BUDGET: u32 = 4;
                        let b = &mut BELL_ACTION_REJECT_BUDGET;
                        if *b > 0 {
                            *b -= 1;
                            serial_println!("[bell.action.reject] reason=not_found event_id={} action_id={} caller_pd={}",
                                event_id, action_id, caller_pd);
                        }
                    }
                }
            }

            OP_BELL_CLEAR => {
                // ── Parse ──
                let lane_filter = ((msg.arg0 >> 0) & 0xFF) as u8;
                let caller_pd   = msg.caller_pd;

                if lane_filter == 0xFF {
                    // ── Clear all lanes: reset queue ──
                    unsafe {
                        BELL_QUEUE.head = 0;
                        BELL_QUEUE.tail = 0;
                        BELL_QUEUE.count = 0;
                    }
                    unsafe {
                        static mut BELL_CLEAR_OK_BUDGET: u32 = 4;
                        let b = &mut BELL_CLEAR_OK_BUDGET;
                        if *b > 0 {
                            *b -= 1;
                            serial_println!("[bell.clear.ok] lane=all caller_pd={}", caller_pd);
                        }
                    }
                } else if lane_filter <= 5 {
                    // ── Clear specific lane: mark matching entries as dismissed ──
                    let mut dismiss_count: u32 = 0;
                    unsafe {
                        for i in 0..BELL_QUEUE.count as usize {
                            let idx = (BELL_QUEUE.head as usize + i) % BELL_QUEUE_CAPACITY;
                            if BELL_QUEUE.entries[idx].final_lane == lane_filter
                                && BELL_QUEUE.entries[idx].dismissed == 0
                            {
                                BELL_QUEUE.entries[idx].dismissed = 1;
                                dismiss_count += 1;
                            }
                        }
                    }
                    unsafe {
                        static mut BELL_CLEAR_OK_BUDGET: u32 = 4;
                        let b = &mut BELL_CLEAR_OK_BUDGET;
                        if *b > 0 {
                            *b -= 1;
                            serial_println!("[bell.clear.ok] lane={} count={} caller_pd={}",
                                lane_filter, dismiss_count, caller_pd);
                        }
                    }
                } else {
                    unsafe {
                        static mut BELL_CLEAR_REJECT_BUDGET: u32 = 4;
                        let b = &mut BELL_CLEAR_REJECT_BUDGET;
                        if *b > 0 {
                            *b -= 1;
                            serial_println!("[bell.clear.reject] reason=invalid_lane lane={} caller_pd={}",
                                lane_filter, caller_pd);
                        }
                    }
                }
            }

            OP_BELL_MUTE_SENDER => {
                // ── Parse ──
                let mute_pd   = (msg.arg0 & 0xFFFFFFFF) as u32;
                let action    = ((msg.arg0 >> 32) & 0xFF) as u8;
                let caller_pd = msg.caller_pd;

                match action {
                    0 => {
                        match add_mute(mute_pd) {
                            Ok(()) => {
                                unsafe {
                                    static mut BELL_MUTE_ADD_BUDGET: u32 = 8;
                                    let b = &mut BELL_MUTE_ADD_BUDGET;
                                    if *b > 0 {
                                        *b -= 1;
                                        serial_println!("[bell.mute.add] mute_pd={} caller_pd={}",
                                            mute_pd, caller_pd);
                                    }
                                }
                            }
                            Err(reason) => {
                                unsafe {
                                    static mut BELL_MUTE_REJECT_BUDGET: u32 = 4;
                                    let b = &mut BELL_MUTE_REJECT_BUDGET;
                                    if *b > 0 {
                                        *b -= 1;
                                        serial_println!("[bell.mute.reject] reason={} mute_pd={} caller_pd={}",
                                            reason, mute_pd, caller_pd);
                                    }
                                }
                            }
                        }
                    }
                    1 => {
                        if remove_mute(mute_pd) {
                            unsafe {
                                static mut BELL_MUTE_REMOVE_BUDGET: u32 = 8;
                                let b = &mut BELL_MUTE_REMOVE_BUDGET;
                                if *b > 0 {
                                    *b -= 1;
                                    serial_println!("[bell.mute.remove] mute_pd={} caller_pd={}",
                                        mute_pd, caller_pd);
                                }
                            }
                        } else {
                            unsafe {
                                static mut BELL_MUTE_REJECT_BUDGET: u32 = 4;
                                let b = &mut BELL_MUTE_REJECT_BUDGET;
                                if *b > 0 {
                                    *b -= 1;
                                    serial_println!("[bell.mute.reject] reason=not_found mute_pd={} caller_pd={}",
                                        mute_pd, caller_pd);
                                }
                            }
                        }
                    }
                    _ => {
                        unsafe {
                            static mut BELL_MUTE_REJECT_BUDGET: u32 = 4;
                            let b = &mut BELL_MUTE_REJECT_BUDGET;
                            if *b > 0 {
                                *b -= 1;
                                serial_println!("[bell.mute.reject] reason=invalid_action action={} caller_pd={}",
                                    action, caller_pd);
                            }
                        }
                    }
                }
            }

            _ => {
                unsafe {
                    static mut BELL_UNKNOWN_BUDGET: u32 = 8;
                    let b = &mut BELL_UNKNOWN_BUDGET;
                    if *b > 0 {
                        *b -= 1;
                        serial_println!("[bell.unknown.reject] type_id={:#x}", msg.type_id);
                    }
                }
            }
        }
    }
}
