#![no_std]
#![no_main]

use sex_pdx::{pdx_listen_raw, serial_println, OP_BELL_NOTIFY};

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
    /// Always 0 in V1 (reserved).
    action_count:     u8,
    /// Always 0 in V1 (reserved).
    object_ref_count: u8,
    /// Padding.
    _pad:             [u8; 6],
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
                object_ref_count: 0,
                _pad: [0; 6],
            }; BELL_QUEUE_CAPACITY],
        }
    }

    fn is_full(&self) -> bool {
        self.count >= BELL_QUEUE_CAPACITY as u16
    }

    /// Push a new entry, assigning event_id. Returns Ok(event_id) or Err("queue_full").
    fn push(&mut self, caller_pd: u32, category: u8, requested_lane: u8,
            final_lane: u8, final_urgency: u8, privacy_level: u8,
            redaction_class: u8) -> Result<u64, &'static str> {
        if self.is_full() {
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
            action_count: 0,
            object_ref_count: 0,
            _pad: [0; 6],
        };

        self.entries[self.tail as usize] = entry;
        self.tail = (self.tail + 1) % BELL_QUEUE_CAPACITY as u16;
        self.count += 1;

        Ok(event_id)
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
];

/// Check if a caller PD is authorized to call OP_BELL_LIST.
fn is_list_reader_allowed(caller_pd: u32) -> bool {
    BELL_LIST_ALLOWLIST.contains(&caller_pd)
}

// ── Enum Validation ──────────────────────────────────────────────────

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
                let object_refs     = (msg.arg2 & 0xFF) as u8;
                let caller_pd       = msg.caller_pd;

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
                } else if action_count != 0 {
                    reject_reason = Some("action_count_not_zero");
                } else if object_refs != 0 {
                    reject_reason = Some("object_refs_not_zero");
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

                // ── Push to queue ────────────────────────────────────
                let push_result = unsafe {
                    BELL_QUEUE.push(caller_pd, category, urgency_hint,
                                    final_lane, final_urgency, privacy_level,
                                    redaction_class)
                };

                match push_result {
                    Ok(event_id) => {
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
                    }
                    continue;
                }

                unsafe {
                    static mut BELL_READCAP_ALLOW_BUDGET: u32 = 8;
                    let b = &mut BELL_READCAP_ALLOW_BUDGET;
                    if *b > 0 {
                        *b -= 1;
                        serial_println!("[bell.readcap.allow] caller_pd={} op=list", caller_pd);
                    }
                }

                // ── Emit recv marker ──
                unsafe {
                    static mut BELL_LIST_RECV_BUDGET: u32 = 8;
                    let b = &mut BELL_LIST_RECV_BUDGET;
                    if *b > 0 {
                        *b -= 1;
                        serial_println!("[bell.list.recv] lane_filter={:#x} max_results={} caller_pd={}",
                            lane_filter, max_results, caller_pd);
                    }
                }

                // ── Read queue (newest-first, no mutation) ──
                let mut match_count: u32 = 0;

                unsafe {
                    for i in 0..BELL_QUEUE.count as usize {
                        let idx = (BELL_QUEUE.tail as usize + BELL_QUEUE_CAPACITY - 1 - i)
                                  % BELL_QUEUE_CAPACITY;
                        let entry = &BELL_QUEUE.entries[idx];

                        if lane_filter == 0xFF || entry.final_lane == lane_filter {
                            // ── Emit item marker ──
                            static mut BELL_LIST_ITEM_BUDGET: u32 = 16;
                            let b = &mut BELL_LIST_ITEM_BUDGET;
                            if *b > 0 {
                                *b -= 1;
                                serial_println!("[bell.list.item] event_id={} final_lane={} \
                                    category={} privacy={} redaction={}",
                                    entry.event_id, entry.final_lane, entry.category,
                                    entry.privacy_level, entry.redaction_class);
                            }

                            match_count += 1;
                            if match_count >= max_results as u32 {
                                break;
                            }
                        }
                    }
                }

                // ── Emit empty or done ──
                if match_count == 0 {
                    unsafe {
                        static mut BELL_LIST_EMPTY_BUDGET: u32 = 4;
                        let b = &mut BELL_LIST_EMPTY_BUDGET;
                        if *b > 0 {
                            *b -= 1;
                            serial_println!("[bell.list.empty]");
                        }
                    }
                } else {
                    unsafe {
                        static mut BELL_LIST_DONE_BUDGET: u32 = 8;
                        let b = &mut BELL_LIST_DONE_BUDGET;
                        if *b > 0 {
                            *b -= 1;
                            serial_println!("[bell.list.done] count={}", match_count);
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
