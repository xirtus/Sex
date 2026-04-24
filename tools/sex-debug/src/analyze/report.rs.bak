use crate::analyze::AnalysisResult;
use crate::model::TraceEvent;
use serde::Serialize;

#[derive(Serialize)]
pub struct AnalysisReport<'a> {
    pub root_cause: &'a str,
    pub confidence: f32,
    pub location: Option<&'a str>,
    pub event: EventSnapshot,
    pub context: Vec<EventSnapshot>,
}

#[derive(Serialize)]
pub struct EventSnapshot {
    pub tsc: u64,
    pub rip: String,
    pub pkru: String,
    pub domain: String,
}

pub fn build_report<'a>(
    result: &'a AnalysisResult,
    events: &[TraceEvent],
) -> AnalysisReport<'a> {
    let bad_idx = result.first_bad_index;
    
    let event = if events.is_empty() {
        EventSnapshot {
            tsc: 0,
            rip: "0x0".to_string(),
            pkru: "0x0".to_string(),
            domain: "Unknown".to_string(),
        }
    } else {
        let ev = &events[bad_idx];
        EventSnapshot {
            tsc: ev.tsc,
            rip: format!("0x{:x}", ev.rip),
            pkru: format!("0x{:x}", ev.pkru),
            domain: format!("{:?}", ev.domain),
        }
    };
    
    let mut context = Vec::new();
    if !events.is_empty() {
        let start = bad_idx.saturating_sub(3);
        let end = (bad_idx + 3).min(events.len().saturating_sub(1));
        
        for i in start..=end {
            let ev = &events[i];
            context.push(EventSnapshot {
                tsc: ev.tsc,
                rip: format!("0x{:x}", ev.rip),
                pkru: format!("0x{:x}", ev.pkru),
                domain: format!("{:?}", ev.domain),
            });
        }
    }
    
    AnalysisReport {
        root_cause: &result.root_cause,
        confidence: result.confidence,
        location: result.function.as_deref(),
        event,
        context,
    }
}

pub fn print_report(result: &AnalysisResult, events: &[TraceEvent]) {
    println!("=== SEX-DEBUG ANALYSIS ===");
    println!("Root Cause: {}", result.root_cause);
    println!("Confidence: {}", result.confidence);
    
    let location = match &result.function {
        Some(f) => f.as_str(),
        None => "unknown",
    };
    println!("Location: {}", location);
    
    if let Some(event) = events.get(result.first_bad_index) {
        println!("Event:");
        println!("  TSC: {}", event.tsc);
        println!("  RIP: 0x{:x}", event.rip);
        println!("  PKRU: 0x{:x}", event.pkru);
        println!("  Domain: {:?}", event.domain);
    }
    
    println!("Context:");
    let bad_idx = result.first_bad_index;
    
    let start = if bad_idx >= 3 { bad_idx - 3 } else { 0 };
    let end = if bad_idx + 3 < events.len() { bad_idx + 3 } else { events.len().saturating_sub(1) };
    
    for i in start..=end {
        if let Some(ev) = events.get(i) {
            if i == bad_idx {
                println!(">>> [{}] rip=0x{:x} pkru=0x{:x} domain={:?}", ev.tsc, ev.rip, ev.pkru, ev.domain);
            } else {
                println!("[{}] rip=0x{:x} pkru=0x{:x} domain={:?}", ev.tsc, ev.rip, ev.pkru, ev.domain);
            }
        }
    }
}
