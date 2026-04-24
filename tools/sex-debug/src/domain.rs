#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Domain {
    Kernel,
    Linen,
    SexDisplay,
    Unknown(u32),
}

impl Domain {
    pub fn from_pkru(pkru: u32) -> Self {
        match pkru {
            0x0 => Domain::Kernel,
            0x55555554 => Domain::Linen,
            0xAAAAAAAA => Domain::SexDisplay,
            other => Domain::Unknown(other),
        }
    }
}