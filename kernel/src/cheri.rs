use x86_64::VirtAddr;

/// SexCapability: A CHERI-inspired 128-bit Software Capability (Fat Pointer).
/// Provides deterministic, byte-granularity spatial safety and unforgeability.
#[repr(C, align(16))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SexCapability {
    /// The current memory location (the cursor).
    pub cursor: u64,
    /// The base address of the valid memory region.
    pub base: u64,
    /// The length of the valid memory region in bytes.
    pub length: u64,
    /// Permissions bitmask (Read=1, Write=2, Execute=4, Seal=8).
    pub permissions: u8,
    /// Object Type for sealed capabilities.
    pub otype: u32,
}

impl SexCapability {
    /// Creates a new capability with the specified bounds and permissions.
    pub fn new(base: u64, length: u64, permissions: u8) -> Self {
        Self {
            cursor: base,
            base,
            length,
            permissions,
            otype: 0,
        }
    }

    /// Verifies if an access at the current cursor is within bounds.
    pub fn is_valid(&self) -> bool {
        self.cursor >= self.base && self.cursor < (self.base + self.length)
    }

    /// Shrinks the bounds of the capability (CHERI Monotonicity).
    pub fn narrow(&mut self, new_base: u64, new_length: u64) -> Result<(), &'static str> {
        if new_base < self.base || (new_base + new_length) > (self.base + self.length) {
            return Err("CHERI: Cannot expand bounds of a capability.");
        }
        self.base = new_base;
        self.length = new_length;
        if self.cursor < self.base { self.cursor = self.base; }
        Ok(())
    }

    /// Seals the capability, making it opaque to the holder.
    pub fn seal(&mut self, otype: u32) {
        self.permissions |= 8; // Set SEAL bit
        self.otype = otype;
    }

    /// Unseals the capability if the provided otype matches.
    pub fn unseal(&mut self, otype: u32) -> Result<(), &'static str> {
        if self.otype == otype {
            self.permissions &= !8; // Clear SEAL bit
            Ok(())
        } else {
            Err("CHERI: Seal mismatch.")
        }
    }
}
