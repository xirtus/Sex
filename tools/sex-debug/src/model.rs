use crate::domain::Domain;

#[derive(Debug, Clone)]
pub struct TraceEvent {
    pub tsc: u64,
    pub rip: u64,
    pub pkru: u32,
    pub domain: Domain,
    pub symbol: Option<String>,
}