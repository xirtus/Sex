use crate::model::TraceEvent;
use crate::domain::Domain;

pub fn parse_line(line: &str) -> Option<TraceEvent> {
    let mut parts = line.split(',');
    
    let tsc_str = parts.next()?;
    let rip_str = parts.next()?;
    let pkru_str = parts.next()?;

    let tsc = tsc_str.trim().parse::<u64>().ok()?;
    
    let rip = u64::from_str_radix(rip_str.trim().trim_start_matches("0x"), 16).ok()?;
    let pkru = u32::from_str_radix(pkru_str.trim().trim_start_matches("0x"), 16).ok()?;
    
    Some(TraceEvent {
        tsc,
        rip,
        pkru,
        domain: Domain::from_pkru(pkru),
        symbol: None,
    })
}